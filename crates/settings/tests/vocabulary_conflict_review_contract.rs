use std::collections::{BTreeMap, BTreeSet};

use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_core::vocabulary_migration::{
    VocabularyMigrationConflict, VocabularyMigrationConflictKind,
    assess_vocabulary_migration_readiness, inspect_vocabulary_migration,
};
use outflow_settings::{
    Settings, SettingsError, SettingsJsonError, VocabularyEntry, VocabularyMigrationReadiness,
    VocabularySourcePolicy, preflight_vocabulary_compatibility,
};
use serde_json::{Value, json};

/// A content-free component of entries that require one manual review decision.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ConflictReviewGroup {
    entry_indices: Vec<usize>,
    conflicts: Vec<VocabularyMigrationConflict>,
    manual_review_required: bool,
}

/// Projects pairwise reports into connected groups for a future review flow.
///
/// This test-only helper uses the conflict report as its only input. It does
/// not inspect vocabulary values, infer source-policy provenance, pick a
/// preferred mapping, mutate settings, or perform persistence.
fn group_conflicts_for_review(
    conflicts: &[VocabularyMigrationConflict],
) -> Vec<ConflictReviewGroup> {
    let mut adjacency = BTreeMap::<usize, BTreeSet<usize>>::new();
    for conflict in conflicts {
        adjacency
            .entry(conflict.original_index)
            .or_default()
            .insert(conflict.conflicting_index);
        adjacency
            .entry(conflict.conflicting_index)
            .or_default()
            .insert(conflict.original_index);
    }

    let mut visited = BTreeSet::new();
    let mut groups = Vec::new();

    // BTreeMap visits vertices in ascending order. Sorting each component and
    // the final groups makes the public test contract stable even if report
    // pairs arrive in a different order.
    for start in adjacency.keys().copied() {
        if !visited.insert(start) {
            continue;
        }

        let mut pending = vec![start];
        let mut entry_indices = Vec::new();
        while let Some(index) = pending.pop() {
            entry_indices.push(index);
            if let Some(neighbors) = adjacency.get(&index) {
                for neighbor in neighbors {
                    if visited.insert(*neighbor) {
                        pending.push(*neighbor);
                    }
                }
            }
        }

        entry_indices.sort_unstable();
        let members = entry_indices.iter().copied().collect::<BTreeSet<_>>();
        let group_conflicts = conflicts
            .iter()
            .filter(|conflict| {
                members.contains(&conflict.original_index)
                    && members.contains(&conflict.conflicting_index)
            })
            .copied()
            .collect();
        groups.push(ConflictReviewGroup {
            entry_indices,
            conflicts: group_conflicts,
            manual_review_required: true,
        });
    }

    groups.sort_by_key(|group| group.entry_indices[0]);
    groups
}

fn conflicts_for(entries: &[(&str, &str)]) -> Vec<VocabularyMigrationConflict> {
    let borrowed = entries
        .iter()
        .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
        .collect::<Vec<_>>();
    inspect_vocabulary_migration(&borrowed).expect("synthetic vocabulary is valid")
}

fn entries_with_phrases<'a>(phrases: &'a [&'a str]) -> Vec<(&'a str, &'a str)> {
    phrases
        .iter()
        .map(|phrase| (*phrase, "synthetic replacement"))
        .collect()
}

fn settings_with_vocabulary(entries: &[(&str, &str)]) -> Settings {
    Settings {
        vocabulary: entries
            .iter()
            .enumerate()
            .map(|(index, (spoken, written))| VocabularyEntry {
                id: format!("synthetic-{index}"),
                spoken: (*spoken).to_owned(),
                written: (*written).to_owned(),
            })
            .collect(),
        ..Settings::default()
    }
}

// These tests intentionally construct a settings DTO containing a Unicode 18
// conflict to exercise the read-only preflight. Bypass the validating codec
// only when comparing the unchanged synthetic in-memory value before/after.
fn raw_settings_json_for_test(settings: &Settings) -> String {
    serde_json::to_string(settings).expect("synthetic settings serialize for read-only comparison")
}

fn group_indices(groups: &[ConflictReviewGroup]) -> Vec<Vec<usize>> {
    groups
        .iter()
        .map(|group| group.entry_indices.clone())
        .collect()
}

#[test]
fn empty_conflict_list_produces_no_groups() {
    assert!(group_conflicts_for_review(&[]).is_empty());
}

#[test]
fn one_conflicting_pair_produces_one_group() {
    let conflicts = conflicts_for(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].entry_indices, vec![0, 1]);
    assert!(groups[0].manual_review_required);
    assert_eq!(groups[0].conflicts, conflicts);
}

#[test]
fn three_equivalent_entries_produce_one_group() {
    let entries = entries_with_phrases(&["straße", "STRASSE", "Strasse"]);
    let conflicts = conflicts_for(&entries);
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].entry_indices, vec![0, 1, 2]);
    assert_eq!(groups[0].conflicts.len(), 3);
}

#[test]
fn two_independent_conflicting_pairs_produce_two_groups() {
    let conflicts = conflicts_for(&[
        ("straße", "Street"),
        ("STRASSE", "Avenue"),
        ("north", "North"),
        ("south", "South"),
        ("é", "Accent A"),
        ("e\u{301}", "Accent B"),
    ]);
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(group_indices(&groups), vec![vec![0, 1], vec![4, 5]]);
}

#[test]
fn connected_conflicts_are_grouped_transitively() {
    let complete_report = conflicts_for(&[("straße", "A"), ("STRASSE", "B"), ("Strasse", "C")]);
    let chain = complete_report
        .iter()
        .copied()
        .filter(|conflict| {
            (conflict.original_index == 0 && conflict.conflicting_index == 1)
                || (conflict.original_index == 1 && conflict.conflicting_index == 2)
        })
        .collect::<Vec<_>>();
    assert_eq!(chain.len(), 2);

    let groups = group_conflicts_for_review(&chain);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].entry_indices, vec![0, 1, 2]);
    assert_eq!(groups[0].conflicts, chain);
}

#[test]
fn duplicate_pair_reports_do_not_duplicate_entry_indices() {
    let pair = conflicts_for(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let conflicts = vec![pair[0], pair[0]];
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].entry_indices, vec![0, 1]);
    assert_eq!(groups[0].conflicts, conflicts);
}

#[test]
fn entry_indices_are_sorted_ascending() {
    let mut conflicts = conflicts_for(&[("straße", "A"), ("STRASSE", "B"), ("Strasse", "C")]);
    conflicts.reverse();

    let groups = group_conflicts_for_review(&conflicts);
    assert_eq!(groups[0].entry_indices, vec![0, 1, 2]);
}

#[test]
fn groups_are_ordered_by_smallest_entry_index() {
    let mut conflicts = conflicts_for(&[
        ("straße", "Street"),
        ("STRASSE", "Avenue"),
        ("north", "North"),
        ("south", "South"),
        ("é", "Accent A"),
        ("e\u{301}", "Accent B"),
    ]);
    conflicts.reverse();

    let groups = group_conflicts_for_review(&conflicts);
    assert_eq!(group_indices(&groups), vec![vec![0, 1], vec![4, 5]]);
}

#[test]
fn legacy_conflict_classification_is_preserved() {
    // The core checker accepts this read-only report input even though the
    // production schema-v1 settings validator rejects duplicate sources.
    let conflicts = conflicts_for(&[("Café", "First"), ("CAFÉ", "Second")]);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].classification,
        VocabularyMigrationConflictKind::ExistingLegacyConflict
    );

    let groups = group_conflicts_for_review(&conflicts);
    assert_eq!(groups[0].conflicts, conflicts);
}

#[test]
fn unicode18_conflict_classification_is_preserved() {
    let conflicts = conflicts_for(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(
        conflicts[0].classification,
        VocabularyMigrationConflictKind::NewUnicode18Conflict
    );

    let groups = group_conflicts_for_review(&conflicts);
    assert_eq!(groups[0].conflicts, conflicts);
}

#[test]
fn mixed_classifications_are_preserved_in_one_group() {
    let conflicts = conflicts_for(&[("straße", "A"), ("STRASSE", "B"), ("strasse", "C")]);
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].entry_indices, vec![0, 1, 2]);
    assert_eq!(groups[0].conflicts, conflicts);
    assert!(groups[0].conflicts.iter().any(|conflict| {
        conflict.classification == VocabularyMigrationConflictKind::ExistingLegacyConflict
    }));
    assert!(groups[0].conflicts.iter().any(|conflict| {
        conflict.classification == VocabularyMigrationConflictKind::NewUnicode18Conflict
    }));
}

#[test]
fn german_sharp_s_collision_is_reported_as_unicode18_conflict() {
    let conflicts = conflicts_for(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(groups[0].entry_indices, vec![0, 1]);
    assert_eq!(
        groups[0].conflicts[0].classification,
        VocabularyMigrationConflictKind::NewUnicode18Conflict
    );
}

#[test]
fn composed_and_decomposed_accent_collision_is_reported() {
    let conflicts = conflicts_for(&[("é", "Accent A"), ("e\u{301}", "Accent B")]);
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(groups[0].entry_indices, vec![0, 1]);
    assert_eq!(
        groups[0].conflicts[0].classification,
        VocabularyMigrationConflictKind::NewUnicode18Conflict
    );
}

#[test]
fn greek_sigma_collision_is_reported() {
    let conflicts = conflicts_for(&[("σ", "Sigma"), ("ς", "Final sigma")]);
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(groups[0].entry_indices, vec![0, 1]);
    assert_eq!(
        groups[0].conflicts[0].classification,
        VocabularyMigrationConflictKind::NewUnicode18Conflict
    );
}

#[test]
fn distinct_nonconflicting_vocabulary_produces_no_groups() {
    let conflicts = conflicts_for(&[("north parkade", "North"), ("southern route", "South")]);
    assert!(conflicts.is_empty());
    assert!(group_conflicts_for_review(&conflicts).is_empty());
}

#[test]
fn repeated_calls_produce_identical_group_order_and_conflicts() {
    let mut conflicts = conflicts_for(&[
        ("straße", "Street"),
        ("STRASSE", "Avenue"),
        ("north", "North"),
        ("south", "South"),
        ("é", "Accent A"),
        ("e\u{301}", "Accent B"),
    ]);
    conflicts.reverse();

    let first = group_conflicts_for_review(&conflicts);
    let second = group_conflicts_for_review(&conflicts);
    assert_eq!(first, second);
}

#[test]
fn original_vocabulary_entries_remain_unchanged() {
    let entries = vec![
        (String::from("straße"), String::from("Street")),
        (String::from("STRASSE"), String::from("Avenue")),
    ];
    let original = entries.clone();
    let borrowed = entries
        .iter()
        .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
        .collect::<Vec<_>>();
    let conflicts = inspect_vocabulary_migration(&borrowed).expect("synthetic vocabulary is valid");
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(groups.len(), 1);
    assert!(
        entries
            .iter()
            .zip(&original)
            .all(|(current, before)| current == before),
        "original synthetic vocabulary changed"
    );
}

#[test]
fn grouping_does_not_infer_source_policy_provenance() {
    let entries = [
        VocabularyEntryRef {
            spoken: "straße",
            written: "Street",
        },
        VocabularyEntryRef {
            spoken: "STRASSE",
            written: "Avenue",
        },
    ];
    let unknown = assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown)
        .expect("unknown-provenance assessment succeeds");
    let verified_legacy = assess_vocabulary_migration_readiness(
        &entries,
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("explicit synthetic provenance assessment succeeds");
    let unknown_conflicts = match unknown {
        VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts } => conflicts,
        _ => panic!("unknown source policy must remain unknown"),
    };
    let verified_conflicts = match verified_legacy {
        VocabularyMigrationReadiness::ManualResolutionRequired { conflicts } => conflicts,
        _ => panic!("conflicts must remain review-required"),
    };

    assert_eq!(unknown_conflicts, verified_conflicts);
    assert_eq!(
        group_conflicts_for_review(&unknown_conflicts),
        group_conflicts_for_review(&verified_conflicts)
    );
}

#[test]
fn report_never_selects_a_preferred_mapping() {
    let conflicts = conflicts_for(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let groups = group_conflicts_for_review(&conflicts);

    assert_eq!(groups[0].entry_indices, vec![0, 1]);
    assert!(groups[0].manual_review_required);
    assert_eq!(groups[0].conflicts, conflicts);
}

#[test]
fn group_debug_report_contains_no_vocabulary_or_comparison_key_text() {
    let entries = [
        ("private spoken phrase", "private written replacement"),
        ("PRIVATE SPOKEN PHRASE", "another private replacement"),
    ];
    let groups = group_conflicts_for_review(&conflicts_for(&entries));
    let report = format!("{groups:?}");

    for private_value in [
        "private spoken phrase",
        "PRIVATE SPOKEN PHRASE",
        "private written replacement",
        "another private replacement",
    ] {
        assert!(!report.contains(private_value));
    }
    let normalized_comparison_key = "private spoken phrase".to_lowercase();
    assert!(!report.contains(&normalized_comparison_key));
}

#[test]
fn production_settings_decoder_continues_rejecting_schema_v2() {
    let mut value: Value = serde_json::from_str(
        &Settings::default()
            .to_json()
            .expect("schema-v1 defaults serialize in memory"),
    )
    .expect("serialized schema-v1 defaults are valid JSON");
    value["schema_version"] = json!(2);

    assert_eq!(
        Settings::from_json(&value.to_string()),
        Err(SettingsJsonError::InvalidSettings(
            SettingsError::UnsupportedSchemaVersion { found: 2 }
        ))
    );
}

#[test]
fn review_projection_has_no_persistence_or_migration_side_effects() {
    let settings = settings_with_vocabulary(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let original = settings.clone();
    let before = raw_settings_json_for_test(&settings);

    let readiness = preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown)
        .expect("in-memory preflight succeeds");
    let conflicts = match readiness {
        VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts } => conflicts,
        _ => panic!("unknown provenance must remain review-required"),
    };
    let groups = group_conflicts_for_review(&conflicts);
    let after = raw_settings_json_for_test(&settings);

    assert_eq!(groups.len(), 1);
    assert!(settings == original, "in-memory settings changed");
    assert!(before == after, "in-memory settings serialization changed");
}

#[test]
fn prints_conflict_review_contract_summary_from_real_reports() {
    let original_entries = vec![
        (String::from("straße"), String::from("Street")),
        (String::from("STRASSE"), String::from("Avenue")),
        (String::from("Strasse"), String::from("Road")),
    ];
    let original_snapshot = original_entries.clone();
    let borrowed = original_entries
        .iter()
        .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
        .collect::<Vec<_>>();
    let connected_conflicts = inspect_vocabulary_migration(&borrowed)
        .expect("three synthetic Unicode-equivalent entries are valid");

    let single_conflicts = conflicts_for(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let single_groups = group_conflicts_for_review(&single_conflicts);
    assert_eq!(single_groups.len(), 1);

    let connected_groups = group_conflicts_for_review(&connected_conflicts);
    assert_eq!(connected_groups.len(), 1);
    assert_eq!(connected_groups[0].entry_indices, vec![0, 1, 2]);

    let independent_conflicts = conflicts_for(&[
        ("straße", "Street"),
        ("STRASSE", "Avenue"),
        ("north", "North"),
        ("south", "South"),
        ("é", "Accent A"),
        ("e\u{301}", "Accent B"),
    ]);
    let independent_groups = group_conflicts_for_review(&independent_conflicts);
    assert_eq!(independent_groups.len(), 2);
    assert_eq!(
        group_indices(&independent_groups),
        vec![vec![0, 1], vec![4, 5]]
    );

    let ordering_verified = independent_groups
        .windows(2)
        .all(|pair| pair[0].entry_indices[0] < pair[1].entry_indices[0]);
    assert!(ordering_verified);

    let flattened_conflicts = independent_groups
        .iter()
        .flat_map(|group| group.conflicts.iter().copied())
        .collect::<Vec<_>>();
    let classifications_preserved = flattened_conflicts == independent_conflicts;
    assert!(classifications_preserved);

    let original_vocabulary_unchanged = original_entries
        .iter()
        .zip(&original_snapshot)
        .all(|(current, before)| current == before);
    assert!(original_vocabulary_unchanged);

    let all_conflicted_entries_remain_in_review = connected_groups
        .iter()
        .all(|group| group.manual_review_required && group.entry_indices.len() > 1);
    assert!(all_conflicted_entries_remain_in_review);

    let resolution_applied = !original_vocabulary_unchanged;
    assert!(!resolution_applied);

    let settings = settings_with_vocabulary(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let settings_snapshot = settings.clone();
    let settings_before = raw_settings_json_for_test(&settings);
    let settings_readiness =
        preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown)
            .expect("read-only settings preflight succeeds");
    let settings_conflicts = match settings_readiness {
        VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts } => conflicts,
        _ => panic!("unknown source policy must remain review-required"),
    };
    let settings_groups = group_conflicts_for_review(&settings_conflicts);
    assert_eq!(settings_groups.len(), 1);
    assert!(settings_groups[0].manual_review_required);
    let settings_after = raw_settings_json_for_test(&settings);
    let settings_state_unchanged =
        settings == settings_snapshot && settings_before == settings_after;
    assert!(settings_state_unchanged);

    let v2_probe = settings_before.replace("\"schema_version\":1", "\"schema_version\":2");
    let production_v2_supported = Settings::from_json(&v2_probe).is_ok();
    assert!(!production_v2_supported);

    println!("Outflow vocabulary conflict review contract");
    println!();
    println!("Single conflict: {} GROUP", single_groups.len());
    println!("Three connected entries: {} GROUP", connected_groups.len());
    println!(
        "Two independent conflicts: {} GROUPS",
        independent_groups.len()
    );
    println!(
        "Group ordering: {}",
        if ordering_verified {
            "VERIFIED"
        } else {
            "FAILED"
        }
    );
    println!(
        "Conflict classifications: {}",
        if classifications_preserved {
            "PRESERVED"
        } else {
            "CHANGED"
        }
    );
    println!(
        "Original vocabulary: {}",
        if original_vocabulary_unchanged {
            "UNCHANGED"
        } else {
            "CHANGED"
        }
    );
    println!();
    println!(
        "Automatic winner selected: {}",
        if all_conflicted_entries_remain_in_review {
            "NO"
        } else {
            "YES"
        }
    );
    println!(
        "Conflict resolution applied: {}",
        if resolution_applied { "YES" } else { "NO" }
    );
    println!(
        "Settings files modified: {}",
        if settings_state_unchanged {
            "NO"
        } else {
            "YES"
        }
    );
    println!(
        "Production schema-v2 support: {}",
        if production_v2_supported {
            "IMPLEMENTED"
        } else {
            "NOT IMPLEMENTED"
        }
    );
}
