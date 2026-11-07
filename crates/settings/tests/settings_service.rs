use outflow_settings::{
    CleanupMode, MAX_VOCABULARY_ENTRIES, Settings, SettingsChanges, SettingsError,
    SettingsSaveOutcome, SettingsService, SettingsServiceError, SettingsServiceOpenStatus,
    SettingsStorageError, VocabularyEntry, VocabularyMigrationConflictKind,
    VocabularyMigrationReadiness, load_settings, save_settings_atomic,
};
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for attempt in 0..128_u32 {
            let path = std::env::temp_dir().join(format!(
                "outflow-settings-service-{label}-{}-{timestamp}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create isolated settings test directory: {error}"),
            }
        }
        panic!("could not allocate an isolated settings test directory");
    }

    fn settings_path(&self) -> PathBuf {
        self.0.join("settings.json")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o700));
        }
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn entry(id: &str, spoken: &str, written: &str) -> VocabularyEntry {
    VocabularyEntry {
        id: id.to_owned(),
        spoken: spoken.to_owned(),
        written: written.to_owned(),
    }
}

fn write_settings(path: &Path, settings: &Settings) {
    save_settings_atomic(path, settings).expect("write valid synthetic settings");
}

fn write_raw_settings(path: &Path, settings: &Settings) -> Vec<u8> {
    let bytes = serde_json::to_vec(settings).expect("serialize synthetic settings fixture");
    fs::write(path, &bytes).expect("write synthetic settings fixture");
    bytes
}

fn loaded(path: &Path) -> Settings {
    load_settings(path)
        .expect("load settings")
        .expect("settings file exists")
}

#[test]
fn first_run_returns_defaults_without_creating_a_file_and_valid_load_is_read_only() {
    let directory = TestDirectory::new("first-run");
    let path = directory.settings_path();

    let service = SettingsService::open(&path).expect("open first-run service");
    assert_eq!(service.open_status(), SettingsServiceOpenStatus::FirstRun);
    assert_eq!(service.current(), &Settings::default());
    assert!(!path.exists());

    service
        .current()
        .validate()
        .expect("first-run defaults are valid");
    write_settings(&path, &Settings::default());
    let before = fs::read(&path).expect("read valid fixture");
    let loaded_service = SettingsService::open(&path).expect("load existing valid settings");
    assert_eq!(
        loaded_service.open_status(),
        SettingsServiceOpenStatus::Loaded
    );
    assert_eq!(loaded_service.current(), &Settings::default());
    assert_eq!(fs::read(path).expect("read settings after load"), before);
}

#[test]
fn preference_changes_are_saved_and_reloaded() {
    let directory = TestDirectory::new("preferences");
    let path = directory.settings_path();
    let mut service = SettingsService::open(&path).expect("open first-run service");

    assert_eq!(
        service
            .apply_changes(SettingsChanges {
                cleanup_mode: Some(CleanupMode::Verbatim),
                remove_fillers: Some(false),
                run_at_login: Some(true),
            })
            .expect("save preference changes"),
        SettingsSaveOutcome::Saved
    );

    let reloaded = SettingsService::open(&path).expect("reload changed preferences");
    assert_eq!(reloaded.current().cleanup.mode, CleanupMode::Verbatim);
    assert!(!reloaded.current().cleanup.remove_fillers);
    assert!(reloaded.current().startup.run_at_login);
    assert_eq!(loaded(&path), *reloaded.current());
}

#[test]
fn add_edit_delete_persist_stable_ids_and_order() {
    let directory = TestDirectory::new("crud");
    let path = directory.settings_path();
    let mut service = SettingsService::open(&path).expect("open first-run service");

    service
        .add_vocabulary(entry("id-first", "synthetic north", "Synthetic North"))
        .expect("persist first vocabulary entry");
    service
        .add_vocabulary(entry("id-second", "synthetic south", "Synthetic South"))
        .expect("persist second vocabulary entry");

    let mut reloaded = SettingsService::open(&path).expect("reload entries");
    assert_eq!(
        reloaded.open_status(),
        SettingsServiceOpenStatus::VocabularyReviewRequired
    );
    assert!(matches!(
        reloaded.vocabulary_readiness(),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts }) if conflicts.is_empty()
    ));
    reloaded
        .confirm_current_vocabulary_review()
        .expect("explicitly review the synthetic vocabulary list");
    reloaded
        .edit_vocabulary(
            "id-first",
            "synthetic north edited".to_owned(),
            "Synthetic North Edited".to_owned(),
        )
        .expect("edit first entry by stable ID");
    reloaded
        .delete_vocabulary("id-second")
        .expect("delete second entry by stable ID");

    let final_service = SettingsService::open(&path).expect("reload final vocabulary");
    assert_eq!(final_service.vocabulary().len(), 1);
    assert_eq!(final_service.vocabulary()[0].id, "id-first");
    assert_eq!(
        final_service.vocabulary()[0].spoken,
        "synthetic north edited"
    );
    assert_eq!(
        final_service.vocabulary()[0].written,
        "Synthetic North Edited"
    );
}

#[test]
fn existing_nonempty_vocabulary_blocks_writes_until_explicit_review() {
    let directory = TestDirectory::new("review");
    let path = directory.settings_path();
    write_settings(
        &path,
        &Settings {
            vocabulary: vec![entry("legacy-id", "legacy synthetic", "Legacy Synthetic")],
            ..Settings::default()
        },
    );
    let before = fs::read(&path).expect("capture original schema-v1 bytes");
    let mut service = SettingsService::open(&path).expect("open legacy settings read-only");

    let error = service
        .add_vocabulary(entry("new-id", "another synthetic", "Another Synthetic"))
        .expect_err("writes require review of existing entries");
    assert!(matches!(
        error,
        SettingsServiceError::VocabularyReviewRequired(
            VocabularyMigrationReadiness::SourcePolicyUnknown { .. }
        )
    ));
    assert!(!error.to_string().contains("legacy synthetic"));
    assert!(!format!("{error:?}").contains("legacy synthetic"));
    assert_eq!(fs::read(&path).expect("read unchanged file"), before);
    assert_eq!(service.vocabulary().len(), 1);

    service
        .confirm_current_vocabulary_review()
        .expect("caller reviewed the current ordered entries");
    service
        .add_vocabulary(entry("new-id", "another synthetic", "Another Synthetic"))
        .expect("reviewed candidate persists");
    assert_eq!(loaded(&path).vocabulary.len(), 2);
}

#[test]
fn unicode18_legacy_conflicts_are_readable_redacted_and_repaired_only_explicitly() {
    let directory = TestDirectory::new("unicode-review");
    let path = directory.settings_path();
    let original = Settings {
        vocabulary: vec![
            entry("first-id", "straße", "Street One"),
            entry("second-id", "STRASSE", "Street Two"),
        ],
        ..Settings::default()
    };
    let original_bytes = write_raw_settings(&path, &original);

    let mut service = SettingsService::open(&path).expect("open conflicting file for review");
    assert_eq!(
        service.open_status(),
        SettingsServiceOpenStatus::VocabularyReviewRequired
    );
    assert_eq!(
        fs::read(&path).expect("read unchanged file"),
        original_bytes
    );
    assert!(matches!(
        service.vocabulary_readiness(),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts })
            if conflicts.len() == 1
                && conflicts[0].classification == VocabularyMigrationConflictKind::NewUnicode18Conflict
    ));

    service
        .confirm_current_vocabulary_review()
        .expect("explicit review acknowledgment");
    service
        .edit_vocabulary(
            "first-id",
            "synthetic street one".to_owned(),
            "Street One".to_owned(),
        )
        .expect("explicit edit resolves the current-policy conflict");
    assert_eq!(loaded(&path).vocabulary.len(), 2);
}

#[test]
fn lower_case_legacy_conflicts_are_classified_without_rewriting_source_text() {
    let directory = TestDirectory::new("legacy-conflict");
    let path = directory.settings_path();
    let original = Settings {
        vocabulary: vec![
            entry("first-id", "Café", "Cafe One"),
            entry("second-id", "CAFÉ", "Cafe Two"),
        ],
        ..Settings::default()
    };
    let original_bytes = write_raw_settings(&path, &original);

    let service = SettingsService::open(&path).expect("open legacy conflict for review");
    assert_eq!(
        fs::read(&path).expect("read unchanged legacy file"),
        original_bytes
    );
    assert!(matches!(
        service.vocabulary_readiness(),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts })
            if conflicts.len() == 1
                && conflicts[0].classification == VocabularyMigrationConflictKind::ExistingLegacyConflict
    ));
    assert_eq!(service.vocabulary()[0].spoken, "Café");
    assert_eq!(service.vocabulary()[1].spoken, "CAFÉ");
}

#[test]
fn five_hundred_entries_survive_reload_and_the_501st_is_rejected() {
    let directory = TestDirectory::new("limit");
    let path = directory.settings_path();
    let initial = Settings {
        vocabulary: (0..MAX_VOCABULARY_ENTRIES - 1)
            .map(|index| {
                entry(
                    &format!("entry-id-{index}"),
                    &format!("synthetic phrase {index}"),
                    &format!("Synthetic written {index}"),
                )
            })
            .collect(),
        ..Settings::default()
    };
    write_settings(&path, &initial);
    let mut service = SettingsService::open(&path).expect("open 499-entry collection");
    service
        .confirm_current_vocabulary_review()
        .expect("review existing ordered list");
    service
        .add_vocabulary(entry(
            "entry-id-499",
            "synthetic phrase 499",
            "Synthetic written 499",
        ))
        .expect("persist the 500th entry");
    let before = fs::read(&path).expect("capture 500-entry file");
    assert_eq!(
        SettingsService::open(&path)
            .expect("reload all entries")
            .vocabulary()
            .len(),
        MAX_VOCABULARY_ENTRIES
    );

    let error = service
        .add_vocabulary(entry(
            "entry-id-500",
            "synthetic phrase 500",
            "Synthetic written 500",
        ))
        .expect_err("the 501st entry is rejected");
    assert!(matches!(
        error,
        SettingsServiceError::Vocabulary(
            outflow_settings::VocabularyManagementError::CollectionFull { maximum: 500 }
        )
    ));
    assert_eq!(service.vocabulary().len(), 500);
    assert_eq!(
        fs::read(&path).expect("read retained 500-entry file"),
        before
    );
}

#[test]
fn invalid_empty_oversized_duplicate_id_and_duplicate_source_changes_are_atomic() {
    let directory = TestDirectory::new("invalid-input");
    let path = directory.settings_path();
    let mut service = SettingsService::open(&path).expect("open service");
    service
        .add_vocabulary(entry(
            "existing-id",
            "private source phrase",
            "Private Written",
        ))
        .expect("persist initial entry");
    let original = service.current().clone();
    let original_bytes = fs::read(&path).expect("capture persisted settings");

    for candidate in [
        entry("empty-spoken", "  \t", "Valid"),
        entry("empty-written", "Valid source", "  \t"),
        entry("oversized", &"😀".repeat(129), "Valid"),
    ] {
        let error = service
            .add_vocabulary(candidate)
            .expect_err("invalid phrase must be rejected");
        assert!(matches!(error, SettingsServiceError::Vocabulary(_)));
        assert_eq!(service.current(), &original);
        assert_eq!(
            fs::read(&path).expect("read unchanged file"),
            original_bytes
        );
    }

    for candidate in [
        entry("existing-id", "different private source", "Changed"),
        entry("second-id", "PRIVATE SOURCE PHRASE", "Changed"),
    ] {
        let error = service
            .add_vocabulary(candidate)
            .expect_err("duplicate ID or source must be rejected");
        assert!(matches!(error, SettingsServiceError::Vocabulary(_)));
        assert_eq!(service.current(), &original);
        assert_eq!(
            fs::read(&path).expect("read unchanged file"),
            original_bytes
        );
        let rendered = format!("{error} {error:?}");
        for private in [
            "private source phrase",
            "different private source",
            "PRIVATE SOURCE PHRASE",
        ] {
            assert!(!rendered.contains(private));
        }
    }
}

#[test]
fn unsupported_schema_and_corrupt_json_are_never_reset_automatically() {
    let directory = TestDirectory::new("bad-documents");
    let path = directory.settings_path();
    for bytes in [br#"{"schema_version":2}"#.as_slice(), b"{ not json"] {
        fs::write(&path, bytes).expect("write invalid fixture");
        let before = fs::read(&path).expect("capture invalid fixture");
        assert!(SettingsService::open(&path).is_err());
        assert_eq!(
            fs::read(&path).expect("read invalid file after open"),
            before
        );
        fs::remove_file(&path).expect("remove isolated invalid fixture");
    }
}

#[test]
fn reset_requires_confirmation_and_uses_existing_file_only() {
    let directory = TestDirectory::new("reset");
    let path = directory.settings_path();
    let mut service = SettingsService::open(&path).expect("open first-run service");
    assert!(matches!(
        service.reset(false),
        Err(SettingsServiceError::Storage(
            SettingsStorageError::ResetNotConfirmed
        ))
    ));
    assert!(!path.exists());
    assert!(matches!(
        service.reset(true),
        Err(SettingsServiceError::Storage(
            SettingsStorageError::DestinationMissing
        ))
    ));
    assert!(!path.exists());

    service
        .apply_changes(SettingsChanges {
            cleanup_mode: Some(CleanupMode::Verbatim),
            remove_fillers: Some(false),
            run_at_login: Some(true),
        })
        .expect("persist nondefault settings");
    let before = fs::read(&path).expect("capture nondefault settings");
    assert!(matches!(
        service.reset(false),
        Err(SettingsServiceError::Storage(
            SettingsStorageError::ResetNotConfirmed
        ))
    ));
    assert_eq!(
        fs::read(&path).expect("read after unconfirmed reset"),
        before
    );
    assert_eq!(service.current().cleanup.mode, CleanupMode::Verbatim);

    assert_eq!(service.reset(true), Ok(SettingsSaveOutcome::Saved));
    assert_eq!(service.current(), &Settings::default());
    assert_eq!(loaded(&path), Settings::default());
}

#[test]
fn persisted_schema_contains_only_settings_and_vocabulary_not_audio_or_transcripts() {
    let directory = TestDirectory::new("privacy");
    let path = directory.settings_path();
    let mut service = SettingsService::open(&path).expect("open service");
    service
        .add_vocabulary(entry(
            "synthetic-id",
            "synthetic private phrase",
            "Synthetic Written",
        ))
        .expect("persist user-entered vocabulary");

    let root: Value = serde_json::from_slice(&fs::read(&path).expect("read persisted JSON"))
        .expect("persisted settings are valid JSON");
    let object = root.as_object().expect("settings JSON root is an object");
    assert_eq!(
        object
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        [
            "cleanup",
            "language",
            "recording",
            "schema_version",
            "startup",
            "updates",
            "vocabulary"
        ]
        .into_iter()
        .collect()
    );
    let serialized = root.to_string().to_ascii_lowercase();
    assert!(!serialized.contains("transcript"));
    assert!(!serialized.contains("audio"));
}

#[test]
fn review_decoder_never_accepts_unsupported_schema_or_invalid_phrases() {
    let directory = TestDirectory::new("review-decoder");
    let path = directory.settings_path();
    let invalid = Settings {
        vocabulary: vec![entry("id", "", "written")],
        ..Settings::default()
    };
    write_raw_settings(&path, &invalid);
    assert!(matches!(
        SettingsService::open(&path),
        Err(SettingsServiceError::Storage(
            SettingsStorageError::InvalidSettings(SettingsError::EmptyVocabularyPhrase { .. })
        ))
    ));
}
