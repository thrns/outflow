use std::error::Error;

use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_core::vocabulary_migration::{
    VocabularyMatchingPolicyId, VocabularyMigrationConflictKind, VocabularyMigrationReadiness,
    VocabularySourcePolicy, assess_vocabulary_migration_readiness, inspect_vocabulary_migration,
};

fn readiness_label(readiness: &VocabularyMigrationReadiness) -> &'static str {
    match readiness {
        VocabularyMigrationReadiness::EmptyCollection => "NO VOCABULARY MIGRATION NEEDED",
        VocabularyMigrationReadiness::SourcePolicyUnknown { .. } => "REVIEW REQUIRED",
        VocabularyMigrationReadiness::ManualResolutionRequired { .. } => "MANUAL RESOLUTION",
        VocabularyMigrationReadiness::CompatibleForExplicitMigration => "COMPATIBLE",
        VocabularyMigrationReadiness::AlreadyCurrent => "ALREADY CURRENT",
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let unchanged = [
        VocabularyEntryRef {
            spoken: "north parkade",
            written: "North Parkade",
        },
        VocabularyEntryRef {
            spoken: "Rust compiler",
            written: "Rust compiler",
        },
    ];
    let legacy_conflict = [
        VocabularyEntryRef {
            spoken: "Alpha",
            written: "first replacement",
        },
        VocabularyEntryRef {
            spoken: "ALPHA",
            written: "second replacement",
        },
    ];
    let new_conflict = [
        VocabularyEntryRef {
            spoken: "straße",
            written: "first replacement",
        },
        VocabularyEntryRef {
            spoken: "STRASSE",
            written: "second replacement",
        },
    ];
    let multiple_new_conflicts = [
        VocabularyEntryRef {
            spoken: "straße",
            written: "replacement one",
        },
        VocabularyEntryRef {
            spoken: "STRASSE",
            written: "replacement two",
        },
        VocabularyEntryRef {
            spoken: "é",
            written: "replacement three",
        },
        VocabularyEntryRef {
            spoken: "e\u{301}",
            written: "replacement four",
        },
        VocabularyEntryRef {
            spoken: "σ",
            written: "replacement five",
        },
        VocabularyEntryRef {
            spoken: "ς",
            written: "replacement six",
        },
    ];

    let unchanged_report = inspect_vocabulary_migration(&unchanged)?;
    let legacy_report = inspect_vocabulary_migration(&legacy_conflict)?;
    let new_report = inspect_vocabulary_migration(&new_conflict)?;
    let multiple_report = inspect_vocabulary_migration(&multiple_new_conflicts)?;

    let legacy_ready = assess_vocabulary_migration_readiness(
        &unchanged,
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )?;
    let legacy_conflict_readiness = assess_vocabulary_migration_readiness(
        &new_conflict,
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )?;
    let current_readiness = assess_vocabulary_migration_readiness(
        &unchanged,
        VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1,
    )?;
    let unknown_readiness =
        assess_vocabulary_migration_readiness(&unchanged, VocabularySourcePolicy::Unknown)?;
    let empty_readiness =
        assess_vocabulary_migration_readiness(&[], VocabularySourcePolicy::Unknown)?;

    let source = VocabularyMatchingPolicyId::LegacyRustLowercaseV1;
    let target = VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1;
    let existing_detected = legacy_report.iter().any(|conflict| {
        conflict.classification == VocabularyMigrationConflictKind::ExistingLegacyConflict
    });
    let new_detected = new_report.iter().any(|conflict| {
        conflict.classification == VocabularyMigrationConflictKind::NewUnicode18Conflict
    });
    let expected_indices = [(0, 1), (2, 3), (4, 5)];
    let indices_verified = multiple_report
        .iter()
        .map(|conflict| (conflict.original_index, conflict.conflicting_index))
        .collect::<Vec<_>>()
        == expected_indices;
    let multiple_are_new = multiple_report.iter().all(|conflict| {
        conflict.classification == VocabularyMigrationConflictKind::NewUnicode18Conflict
    });
    let manual_resolution_required = !multiple_report.is_empty() && multiple_are_new;
    let user_entries_deleted = false;
    let settings_files_modified = false;
    let migration_applied = false;

    println!("Outflow vocabulary migration compatibility check");
    println!("Source policy: {}", source.label());
    println!("Target policy: {}", target.label());
    println!(
        "Unchanged collection: {}",
        if unchanged_report.is_empty() {
            "COMPATIBLE"
        } else {
            "CONFLICTS DETECTED"
        }
    );
    println!(
        "Existing legacy conflict: {}",
        if existing_detected {
            "DETECTED"
        } else {
            "NONE"
        }
    );
    println!(
        "New Unicode 18 conflict: {}",
        if new_detected { "DETECTED" } else { "NONE" }
    );
    println!(
        "Multiple new conflicts: {}",
        if multiple_are_new && multiple_report.len() == expected_indices.len() {
            "DETECTED"
        } else {
            "NOT VERIFIED"
        }
    );
    println!(
        "Conflict indices: {}",
        if indices_verified {
            "VERIFIED"
        } else {
            "NOT VERIFIED"
        }
    );
    println!(
        "Migration applied: {}",
        if migration_applied { "YES" } else { "NO" }
    );
    println!(
        "Settings files modified: {}",
        if settings_files_modified { "YES" } else { "NO" }
    );
    println!(
        "User entries deleted: {}",
        if user_entries_deleted { "YES" } else { "NO" }
    );
    println!(
        "Manual conflict resolution: {}",
        if manual_resolution_required {
            "REQUIRED"
        } else {
            "NOT REQUIRED"
        }
    );

    // The assessment has no persistence input or side effect. These snapshots
    // make the unchanged synthetic values visible in the demonstration output.
    let vocabulary_values_modified = false;
    let migration_applied = false;
    let settings_schema_modified = false;

    println!("\nVocabulary migration readiness");
    println!(
        "Verified legacy, no conflicts: {}",
        readiness_label(&legacy_ready)
    );
    println!(
        "Verified legacy, Unicode 18 conflict: {}",
        readiness_label(&legacy_conflict_readiness)
    );
    println!(
        "Verified current, no conflicts: {}",
        readiness_label(&current_readiness)
    );
    println!(
        "Unknown source policy: {}",
        readiness_label(&unknown_readiness)
    );
    println!("Empty collection: {}", readiness_label(&empty_readiness));
    println!(
        "\nSettings schema modified: {}",
        if settings_schema_modified {
            "YES"
        } else {
            "NO"
        }
    );
    println!(
        "Migration applied: {}",
        if migration_applied { "YES" } else { "NO" }
    );
    println!(
        "Vocabulary values modified: {}",
        if vocabulary_values_modified {
            "YES"
        } else {
            "NO"
        }
    );

    Ok(())
}
