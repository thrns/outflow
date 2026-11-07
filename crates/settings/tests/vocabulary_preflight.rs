use outflow_settings::{
    CleanupMode, CleanupSettings, RecordingSettings, Settings, SettingsError, StartupSettings,
    UpdateSettings, VocabularyEntry, VocabularyField, VocabularyMigrationConflict,
    VocabularyMigrationConflictKind, VocabularyMigrationError, VocabularyMigrationReadiness,
    VocabularySourcePolicy, VocabularyValidationError, preflight_vocabulary_compatibility,
};

fn entry(id: usize, spoken: &str, written: &str) -> VocabularyEntry {
    VocabularyEntry {
        id: id.to_string(),
        spoken: spoken.to_owned(),
        written: written.to_owned(),
    }
}

fn settings_with_vocabulary(vocabulary: Vec<VocabularyEntry>) -> Settings {
    Settings {
        vocabulary,
        ..Settings::default()
    }
}

fn readiness_label(readiness: &VocabularyMigrationReadiness) -> &'static str {
    match readiness {
        VocabularyMigrationReadiness::EmptyCollection => "EMPTY COLLECTION",
        VocabularyMigrationReadiness::SourcePolicyUnknown { .. } => "REVIEW REQUIRED",
        VocabularyMigrationReadiness::ManualResolutionRequired { .. } => "MANUAL RESOLUTION",
        VocabularyMigrationReadiness::CompatibleForExplicitMigration => "COMPATIBLE",
        VocabularyMigrationReadiness::AlreadyCurrent => "ALREADY CURRENT",
    }
}

#[test]
fn empty_vocabulary_needs_no_vocabulary_migration_for_unknown_provenance() {
    let settings = Settings::default();

    assert_eq!(
        preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::EmptyCollection)
    );
}

#[test]
fn unknown_provenance_remains_unknown_for_one_or_multiple_distinct_entries() {
    for vocabulary in [
        vec![entry(0, "hello", "Hello")],
        vec![entry(0, "hello", "Hello"), entry(1, "world", "World")],
    ] {
        let settings = settings_with_vocabulary(vocabulary);
        assert_eq!(settings.schema_version, 1);
        assert_eq!(
            preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown),
            Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
                conflicts: Vec::new(),
            })
        );
    }
}

#[test]
fn verified_legacy_and_current_policies_report_their_distinct_ready_states() {
    let settings = settings_with_vocabulary(vec![
        entry(0, "north parkade", "North Parkade"),
        entry(1, "Rust", "Rust"),
    ]);

    assert_eq!(
        preflight_vocabulary_compatibility(
            &settings,
            VocabularySourcePolicy::VerifiedLegacyLowercase
        ),
        Ok(VocabularyMigrationReadiness::CompatibleForExplicitMigration)
    );
    assert_eq!(
        preflight_vocabulary_compatibility(
            &settings,
            VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1
        ),
        Ok(VocabularyMigrationReadiness::AlreadyCurrent)
    );
}

#[test]
fn unicode_18_collisions_report_classification_indices_and_deterministic_order() {
    let settings = settings_with_vocabulary(vec![
        entry(0, "straße", "Street"),
        entry(1, "café", "Cafe"),
        entry(2, "STRASSE", "Road"),
        entry(3, "cafe\u{301}", "Coffee shop"),
        entry(4, "σ", "Sigma"),
        entry(5, "ς", "Final sigma"),
    ]);
    let expected = vec![
        VocabularyMigrationConflict {
            original_index: 0,
            conflicting_index: 2,
            classification: VocabularyMigrationConflictKind::NewUnicode18Conflict,
            source_policy: outflow_settings::VocabularyMatchingPolicyId::LegacyRustLowercaseV1,
            target_policy:
                outflow_settings::VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1,
        },
        VocabularyMigrationConflict {
            original_index: 1,
            conflicting_index: 3,
            classification: VocabularyMigrationConflictKind::NewUnicode18Conflict,
            source_policy: outflow_settings::VocabularyMatchingPolicyId::LegacyRustLowercaseV1,
            target_policy:
                outflow_settings::VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1,
        },
        VocabularyMigrationConflict {
            original_index: 4,
            conflicting_index: 5,
            classification: VocabularyMigrationConflictKind::NewUnicode18Conflict,
            source_policy: outflow_settings::VocabularyMatchingPolicyId::LegacyRustLowercaseV1,
            target_policy:
                outflow_settings::VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1,
        },
    ];

    assert_eq!(
        preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
            conflicts: expected.clone(),
        })
    );
    assert_eq!(
        preflight_vocabulary_compatibility(
            &settings,
            VocabularySourcePolicy::VerifiedLegacyLowercase
        ),
        Ok(VocabularyMigrationReadiness::ManualResolutionRequired {
            conflicts: expected,
        })
    );
    assert!(matches!(
        preflight_vocabulary_compatibility(
            &settings,
            VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1
        ),
        Ok(VocabularyMigrationReadiness::ManualResolutionRequired { .. })
    ));
}

#[test]
fn existing_lowercase_conflicts_are_classified_without_choosing_a_winner() {
    let settings = settings_with_vocabulary(vec![
        entry(0, "Hello", "First replacement"),
        entry(1, "hello", "Second replacement"),
    ]);

    assert_eq!(
        preflight_vocabulary_compatibility(
            &settings,
            VocabularySourcePolicy::VerifiedLegacyLowercase
        ),
        Ok(VocabularyMigrationReadiness::ManualResolutionRequired {
            conflicts: vec![VocabularyMigrationConflict {
                original_index: 0,
                conflicting_index: 1,
                classification: VocabularyMigrationConflictKind::ExistingLegacyConflict,
                source_policy: outflow_settings::VocabularyMatchingPolicyId::LegacyRustLowercaseV1,
                target_policy:
                    outflow_settings::VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1,
            }],
        })
    );
    assert_eq!(settings.vocabulary[0].written, "First replacement");
    assert_eq!(settings.vocabulary[1].written, "Second replacement");
}

#[test]
fn in_memory_preflight_reports_unicode18_collision_without_inferring_provenance() {
    let settings = settings_with_vocabulary(vec![
        entry(0, "straße", "Street"),
        entry(1, "STRASSE", "Road"),
    ]);
    assert_eq!(
        preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
            conflicts: vec![VocabularyMigrationConflict {
                original_index: 0,
                conflicting_index: 1,
                classification: VocabularyMigrationConflictKind::NewUnicode18Conflict,
                source_policy: outflow_settings::VocabularyMatchingPolicyId::LegacyRustLowercaseV1,
                target_policy:
                    outflow_settings::VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1,
            }],
        })
    );
}

#[test]
fn current_settings_decoder_rejects_unicode18_conflicts_in_schema_v1() {
    let mut document: serde_json::Value = serde_json::from_str(
        &Settings::default()
            .to_json()
            .expect("synthetic defaults serialize"),
    )
    .expect("settings encode as JSON");
    document["vocabulary"] = serde_json::json!([
        { "id": "one", "spoken": "straße", "written": "First" },
        { "id": "two", "spoken": "STRASSE", "written": "Second" }
    ]);

    assert!(matches!(
        Settings::from_json(&document.to_string()),
        Err(outflow_settings::SettingsJsonError::InvalidSettings(
            SettingsError::DuplicateVocabularySource {
                first_index: 0,
                duplicate_index: 1,
            }
        ))
    ));
}

#[test]
fn invalid_spoken_and_written_phrases_use_core_validation_errors() {
    let invalid_spoken = settings_with_vocabulary(vec![entry(0, " \t", "replacement")]);
    assert_eq!(
        preflight_vocabulary_compatibility(&invalid_spoken, VocabularySourcePolicy::Unknown),
        Err(VocabularyMigrationError::InvalidEntry {
            index: 0,
            source: VocabularyValidationError::Empty(VocabularyField::Spoken),
        })
    );

    let invalid_written = settings_with_vocabulary(vec![entry(0, "spoken", " \t")]);
    assert_eq!(
        preflight_vocabulary_compatibility(&invalid_written, VocabularySourcePolicy::Unknown),
        Err(VocabularyMigrationError::InvalidEntry {
            index: 0,
            source: VocabularyValidationError::Empty(VocabularyField::Written),
        })
    );
}

#[test]
fn core_entry_limit_accepts_500_and_rejects_501() {
    let accepted = settings_with_vocabulary(
        (0..500)
            .map(|index| {
                entry(
                    index,
                    &format!("spoken-{index}"),
                    &format!("written-{index}"),
                )
            })
            .collect(),
    );
    assert!(matches!(
        preflight_vocabulary_compatibility(&accepted, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts }) if conflicts.is_empty()
    ));

    let rejected = settings_with_vocabulary(
        (0..501)
            .map(|index| {
                entry(
                    index,
                    &format!("spoken-{index}"),
                    &format!("written-{index}"),
                )
            })
            .collect(),
    );
    assert_eq!(
        preflight_vocabulary_compatibility(&rejected, VocabularySourcePolicy::Unknown),
        Err(VocabularyMigrationError::TooManyEntries {
            count: 501,
            maximum: 500,
        })
    );
}

#[test]
fn preflight_preserves_vocabulary_other_fields_and_existing_json_behavior() {
    let settings = Settings {
        recording: RecordingSettings {
            push_to_talk: Some("Ctrl+Alt+Space".to_owned()),
            toggle: Some("Ctrl+Alt+D".to_owned()),
            input_device: Some("Synthetic microphone".to_owned()),
        },
        cleanup: CleanupSettings {
            mode: CleanupMode::Verbatim,
            remove_fillers: false,
        },
        startup: StartupSettings { run_at_login: true },
        updates: UpdateSettings {
            user_initiated_only: true,
        },
        vocabulary: vec![entry(7, "cafe\u{301}", "Cafe\u{301}")],
        ..Settings::default()
    };
    let original = settings.clone();
    let original_json = settings
        .to_json()
        .expect("synthetic valid settings serialize before preflight");

    assert_eq!(
        preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
            conflicts: Vec::new(),
        })
    );

    assert_eq!(settings, original);
    assert_eq!(settings.vocabulary[0].spoken, "cafe\u{301}");
    assert_eq!(
        settings.to_json().expect("settings still serialize"),
        original_json
    );
    let serialized: serde_json::Value =
        serde_json::from_str(&original_json).expect("settings serialize as JSON");
    assert_eq!(serialized["schema_version"], 1);
    assert!(serialized.get("vocabulary_matching_policy").is_none());
}

#[test]
fn integration_summary_uses_readiness_results_from_real_preflight_calls() {
    let unknown_settings = settings_with_vocabulary(vec![entry(0, "hello", "Hello")]);
    let legacy_settings = settings_with_vocabulary(vec![entry(0, "hello", "Hello")]);
    let conflict_settings = settings_with_vocabulary(vec![
        entry(0, "straße", "Street"),
        entry(1, "STRASSE", "Road"),
    ]);
    let current_settings = settings_with_vocabulary(vec![entry(0, "hello", "Hello")]);
    let original_vocabulary = conflict_settings.vocabulary.clone();

    let unknown =
        preflight_vocabulary_compatibility(&unknown_settings, VocabularySourcePolicy::Unknown)
            .expect("unknown provenance is assessable");
    let legacy = preflight_vocabulary_compatibility(
        &legacy_settings,
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("legacy provenance is assessable");
    let conflict = preflight_vocabulary_compatibility(
        &conflict_settings,
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("legacy collision is assessable");
    let current = preflight_vocabulary_compatibility(
        &current_settings,
        VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1,
    )
    .expect("current provenance is assessable");

    println!("Settings vocabulary compatibility preflight");
    println!("Schema: v{}", unknown_settings.schema_version);
    println!("Source policy: UNKNOWN");
    println!("Nonempty vocabulary: {}", readiness_label(&unknown));
    println!(
        "Verified legacy, no conflicts: {}",
        readiness_label(&legacy)
    );
    println!(
        "Verified legacy, Unicode 18 conflict: {}",
        readiness_label(&conflict)
    );
    println!(
        "Verified current, no conflicts: {}",
        readiness_label(&current)
    );
    println!("Settings loaded automatically: NO");
    println!("Settings saved: NO");
    println!("Migration applied: NO");
    println!(
        "Original vocabulary modified: {}",
        if conflict_settings.vocabulary == original_vocabulary {
            "NO"
        } else {
            "YES"
        }
    );

    assert!(matches!(
        unknown,
        VocabularyMigrationReadiness::SourcePolicyUnknown { .. }
    ));
    assert_eq!(
        legacy,
        VocabularyMigrationReadiness::CompatibleForExplicitMigration
    );
    assert!(matches!(
        conflict,
        VocabularyMigrationReadiness::ManualResolutionRequired { .. }
    ));
    assert_eq!(current, VocabularyMigrationReadiness::AlreadyCurrent);
    assert_eq!(conflict_settings.vocabulary, original_vocabulary);
}
