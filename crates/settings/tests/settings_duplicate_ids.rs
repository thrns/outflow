use outflow_settings::{
    Settings, SettingsError, SettingsService, SettingsServiceError, SettingsStorageError,
    VocabularyEntry, load_settings, save_settings_atomic,
};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "outflow-duplicate-vocabulary-id-{}-{now}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create isolated settings directory");
        Self(path)
    }

    fn settings_path(&self) -> PathBuf {
        self.0.join("settings.json")
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn duplicate_id_settings() -> Settings {
    Settings {
        vocabulary: vec![
            VocabularyEntry {
                id: "reused-stable-id".to_owned(),
                spoken: "synthetic alpha".to_owned(),
                written: "Synthetic Alpha".to_owned(),
            },
            VocabularyEntry {
                id: "reused-stable-id".to_owned(),
                spoken: "synthetic beta".to_owned(),
                written: "Synthetic Beta".to_owned(),
            },
        ],
        ..Settings::default()
    }
}

#[test]
fn strict_load_and_service_open_reject_duplicate_ids_without_rewriting_the_file() {
    let directory = TestDirectory::new();
    let path = directory.settings_path();
    let invalid = duplicate_id_settings();
    let original = serde_json::to_vec(&invalid)
        .expect("synthetic invalid settings serialize for a read-only load test");
    fs::write(&path, &original).expect("write original duplicate-ID schema-v1 fixture");

    let expected = SettingsError::DuplicateVocabularyId {
        first_index: 0,
        duplicate_index: 1,
    };
    assert!(matches!(
        load_settings(&path),
        Err(SettingsStorageError::InvalidSettings(error)) if error == expected
    ));
    assert!(matches!(
        SettingsService::open(&path),
        Err(SettingsServiceError::Storage(SettingsStorageError::InvalidSettings(error)))
            if error == expected
    ));
    assert_eq!(
        fs::read(&path).expect("read preserved original file"),
        original
    );
}

#[test]
fn atomic_writer_rejects_duplicate_ids_before_replacing_an_existing_file() {
    let directory = TestDirectory::new();
    let path = directory.settings_path();
    let valid = Settings::default();
    save_settings_atomic(&path, &valid).expect("write valid baseline");
    let original = fs::read(&path).expect("capture valid baseline bytes");

    assert!(matches!(
        save_settings_atomic(&path, &duplicate_id_settings()),
        Err(SettingsStorageError::InvalidSettings(
            SettingsError::DuplicateVocabularyId {
                first_index: 0,
                duplicate_index: 1,
            }
        ))
    ));
    assert_eq!(fs::read(&path).expect("read unchanged baseline"), original);
}
