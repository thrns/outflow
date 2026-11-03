#![cfg(unix)]

use outflow_settings::{
    CleanupMode, Settings, SettingsError, SettingsStorageError, VocabularyEntry, load_settings,
    save_settings_atomic,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SETTINGS_FILE: &str = "settings.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaleUpdateAssessment {
    MatchesObservedCurrent,
    RejectedStaleSnapshot,
}

/// Compares complete schema-v1 snapshots. Equality only describes the value
/// observed at this comparison; it is not an atomic precondition for a later
/// write and cannot reveal intervening changes that returned to the same value.
fn assess_stale_update(expected: &Settings, observed_current: &Settings) -> StaleUpdateAssessment {
    if expected == observed_current {
        StaleUpdateAssessment::MatchesObservedCurrent
    } else {
        StaleUpdateAssessment::RejectedStaleSnapshot
    }
}

/// Only a successfully loaded existing document can produce an assessment.
/// Missing, malformed, unsupported, or otherwise unreadable settings are not
/// replaced with defaults for comparison.
fn assess_loaded_snapshot(
    expected: &Settings,
    loaded: Result<Option<Settings>, SettingsStorageError>,
) -> Option<StaleUpdateAssessment> {
    match loaded {
        Ok(Some(observed_current)) => Some(assess_stale_update(expected, &observed_current)),
        Ok(None) | Err(_) => None,
    }
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        for attempt in 0..128_u32 {
            let path = std::env::temp_dir().join(format!(
                "outflow-settings-stale-update-{label}-{}-{timestamp}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create isolated settings test directory: {error}"),
            }
        }

        panic!("could not allocate a unique isolated settings test directory");
    }

    fn settings_path(&self) -> PathBuf {
        self.0.join(SETTINGS_FILE)
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn load_required(path: &Path) -> Settings {
    load_settings(path)
        .expect("load synthetic settings")
        .expect("synthetic settings file exists")
}

fn write_valid_settings(path: &Path, settings: &Settings) {
    settings
        .validate()
        .expect("synthetic schema-v1 settings are valid");
    save_settings_atomic(path, settings).expect("write synthetic settings through public writer");
}

fn vocabulary_entry(id: &str, spoken: &str, written: &str) -> VocabularyEntry {
    VocabularyEntry {
        id: id.to_owned(),
        spoken: spoken.to_owned(),
        written: written.to_owned(),
    }
}

fn independent_stale_edit_scenario(label: &str) -> StaleUpdateAssessment {
    let directory = TestDirectory::new(label);
    let path = directory.settings_path();
    let initial = Settings::default();
    assert_eq!(initial.schema_version, 1);
    write_valid_settings(&path, &initial);

    let actor_one_expected = load_required(&path);
    let actor_two_expected = load_required(&path);
    assert_eq!(actor_one_expected, initial);
    assert_eq!(actor_two_expected, initial);

    let mut actor_one_prepared = actor_one_expected.clone();
    actor_one_prepared.cleanup.mode = CleanupMode::Verbatim;
    let mut actor_two_prepared = actor_two_expected.clone();
    actor_two_prepared.startup.run_at_login = true;
    actor_one_prepared
        .validate()
        .expect("actor one's schema-v1 update is valid");
    actor_two_prepared
        .validate()
        .expect("actor two's schema-v1 update is valid");

    write_valid_settings(&path, &actor_one_prepared);
    let observed_after_actor_one = load_required(&path);
    assert_eq!(observed_after_actor_one, actor_one_prepared);

    let assessment = assess_stale_update(&actor_two_expected, &observed_after_actor_one);
    assert_eq!(assessment, StaleUpdateAssessment::RejectedStaleSnapshot);
    assert_eq!(
        load_required(&path),
        actor_one_prepared,
        "actor two's stale write is not submitted and B remains unchanged"
    );
    assert_ne!(actor_two_prepared, observed_after_actor_one);

    assessment
}

fn refreshed_snapshot_scenario(label: &str) -> StaleUpdateAssessment {
    let directory = TestDirectory::new(label);
    let path = directory.settings_path();
    let initial = Settings::default();
    write_valid_settings(&path, &initial);

    let mut actor_one_prepared = load_required(&path);
    actor_one_prepared.cleanup.mode = CleanupMode::Verbatim;
    write_valid_settings(&path, &actor_one_prepared);

    let actor_two_refreshed = load_required(&path);
    let observed_current = load_required(&path);
    assert_eq!(actor_two_refreshed, actor_one_prepared);
    assert_eq!(observed_current, actor_one_prepared);

    let assessment = assess_stale_update(&actor_two_refreshed, &observed_current);
    assert_eq!(assessment, StaleUpdateAssessment::MatchesObservedCurrent);
    assert_eq!(load_required(&path), actor_one_prepared);
    assessment
}

fn changed_vocabulary_assessment() -> StaleUpdateAssessment {
    let expected = Settings {
        vocabulary: vec![vocabulary_entry(
            "task71-vocabulary-original",
            "synthetic original spoken",
            "Synthetic Original Written",
        )],
        ..Settings::default()
    };
    let mut observed = expected.clone();
    observed.vocabulary[0] = vocabulary_entry(
        "task71-vocabulary-edited",
        "synthetic edited spoken",
        "Synthetic Edited Written",
    );

    expected
        .validate()
        .expect("expected schema-v1 vocabulary is valid");
    observed
        .validate()
        .expect("observed schema-v1 vocabulary is valid");
    assert_eq!(expected.schema_version, observed.schema_version);
    assess_stale_update(&expected, &observed)
}

fn vocabulary_reorder_assessment() -> StaleUpdateAssessment {
    let expected = Settings {
        vocabulary: vec![
            vocabulary_entry(
                "task71-order-first",
                "synthetic order first",
                "Synthetic Order First",
            ),
            vocabulary_entry(
                "task71-order-second",
                "synthetic order second",
                "Synthetic Order Second",
            ),
        ],
        ..Settings::default()
    };
    let mut observed = expected.clone();
    observed.vocabulary.reverse();

    expected
        .validate()
        .expect("ordered schema-v1 vocabulary is valid");
    observed
        .validate()
        .expect("reordered schema-v1 vocabulary is valid");
    assess_stale_update(&expected, &observed)
}

fn aba_history_scenario(label: &str) -> StaleUpdateAssessment {
    let directory = TestDirectory::new(label);
    let path = directory.settings_path();
    let initial_a = Settings::default();
    write_valid_settings(&path, &initial_a);
    let expected_a = load_required(&path);
    assert_eq!(expected_a, initial_a);

    let mut different_b = initial_a.clone();
    different_b.startup.run_at_login = true;
    write_valid_settings(&path, &different_b);
    assert_eq!(load_required(&path), different_b);

    write_valid_settings(&path, &initial_a);
    let observed_a_again = load_required(&path);
    assert_eq!(observed_a_again, initial_a);

    let assessment = assess_stale_update(&expected_a, &observed_a_again);
    assert_eq!(assessment, StaleUpdateAssessment::MatchesObservedCurrent);
    assessment
}

#[test]
fn equal_snapshots_match() {
    let settings = Settings::default();
    settings
        .validate()
        .expect("default schema-v1 settings are valid");
    assert_eq!(
        assess_stale_update(&settings, &settings),
        StaleUpdateAssessment::MatchesObservedCurrent
    );
}

#[test]
fn changed_preference_is_stale() {
    let expected = Settings::default();
    let mut observed = expected.clone();
    observed.cleanup.mode = CleanupMode::Verbatim;

    assert_eq!(expected.schema_version, observed.schema_version);
    assert_eq!(
        assess_stale_update(&expected, &observed),
        StaleUpdateAssessment::RejectedStaleSnapshot
    );
}

#[test]
fn independent_stale_edit_is_rejected_without_submitting_the_stale_write() {
    assert_eq!(
        independent_stale_edit_scenario("independent-stale"),
        StaleUpdateAssessment::RejectedStaleSnapshot
    );
}

#[test]
fn refreshed_snapshot_matches_the_current_loaded_value() {
    assert_eq!(
        refreshed_snapshot_scenario("refreshed"),
        StaleUpdateAssessment::MatchesObservedCurrent
    );
}

#[test]
fn vocabulary_change_is_stale_even_when_schema_version_is_unchanged() {
    assert_eq!(
        changed_vocabulary_assessment(),
        StaleUpdateAssessment::RejectedStaleSnapshot
    );
}

#[test]
fn vocabulary_reordering_is_detected() {
    assert_eq!(
        vocabulary_reorder_assessment(),
        StaleUpdateAssessment::RejectedStaleSnapshot
    );
}

#[test]
fn schema_version_is_not_a_concurrency_revision() {
    let expected = Settings::default();
    let mut observed = expected.clone();
    observed.startup.run_at_login = true;

    assert_eq!(expected.schema_version, 1);
    assert_eq!(observed.schema_version, 1);
    assert_eq!(
        assess_stale_update(&expected, &observed),
        StaleUpdateAssessment::RejectedStaleSnapshot
    );
}

#[test]
fn a_to_b_to_a_history_remains_ambiguous() {
    assert_eq!(
        aba_history_scenario("aba"),
        StaleUpdateAssessment::MatchesObservedCurrent
    );
}

#[test]
fn missing_malformed_and_unsupported_settings_never_match() {
    let directory = TestDirectory::new("loader-outcomes");
    let expected = Settings::default();

    let missing = load_settings(&directory.0.join("missing.json"));
    assert_eq!(missing.as_ref().expect("missing path is readable"), &None);
    assert_eq!(assess_loaded_snapshot(&expected, missing), None);

    let malformed_path = directory.0.join("malformed.json");
    fs::write(&malformed_path, b"{not valid json").expect("write synthetic malformed JSON");
    let malformed = load_settings(&malformed_path);
    assert!(matches!(
        &malformed,
        Err(SettingsStorageError::InvalidDocument { .. })
    ));
    assert_eq!(assess_loaded_snapshot(&expected, malformed), None);

    let unsupported_path = directory.0.join("unsupported.json");
    fs::write(&unsupported_path, br#"{"schema_version":2}"#)
        .expect("write synthetic unsupported schema marker");
    let unsupported = load_settings(&unsupported_path);
    assert!(matches!(
        &unsupported,
        Err(SettingsStorageError::InvalidSettings(
            SettingsError::UnsupportedSchemaVersion { found: 2 }
        ))
    ));
    assert_eq!(assess_loaded_snapshot(&expected, unsupported), None);
}

#[test]
fn summary_is_deterministic_private_and_non_mutating() {
    let equal = Settings::default();
    assert_eq!(
        assess_stale_update(&equal, &equal),
        StaleUpdateAssessment::MatchesObservedCurrent
    );

    let mut changed_preference = equal.clone();
    changed_preference.cleanup.mode = CleanupMode::Verbatim;
    let preference_assessment = assess_stale_update(&equal, &changed_preference);
    assert_eq!(
        preference_assessment,
        StaleUpdateAssessment::RejectedStaleSnapshot
    );

    let independent_stale = independent_stale_edit_scenario("summary-independent");
    let refreshed = refreshed_snapshot_scenario("summary-refreshed");
    let changed_vocabulary = changed_vocabulary_assessment();
    let reordered_vocabulary = vocabulary_reorder_assessment();
    let aba_history = aba_history_scenario("summary-aba");

    assert_eq!(
        independent_stale,
        StaleUpdateAssessment::RejectedStaleSnapshot
    );
    assert_eq!(refreshed, StaleUpdateAssessment::MatchesObservedCurrent);
    assert_eq!(
        changed_vocabulary,
        StaleUpdateAssessment::RejectedStaleSnapshot
    );
    assert_eq!(
        reordered_vocabulary,
        StaleUpdateAssessment::RejectedStaleSnapshot
    );
    assert_eq!(aba_history, StaleUpdateAssessment::MatchesObservedCurrent);

    let loader_directory = TestDirectory::new("summary-loader");
    let missing = load_settings(&loader_directory.0.join("missing.json"));
    assert_eq!(assess_loaded_snapshot(&equal, missing), None);
    let unsupported_path = loader_directory.0.join("unsupported.json");
    fs::write(&unsupported_path, br#"{"schema_version":2}"#)
        .expect("write synthetic unsupported schema marker");
    let unsupported = load_settings(&unsupported_path);
    assert!(matches!(
        &unsupported,
        Err(SettingsStorageError::InvalidSettings(
            SettingsError::UnsupportedSchemaVersion { found: 2 }
        ))
    ));
    assert_eq!(assess_loaded_snapshot(&equal, unsupported), None);

    let directory = TestDirectory::new("pure-comparison");
    let path = directory.settings_path();
    let private_expected = Settings {
        vocabulary: vec![vocabulary_entry(
            "task71-private-entry",
            "task71 synthetic private spoken marker",
            "Task71 Synthetic Private Written Marker",
        )],
        ..Settings::default()
    };
    write_valid_settings(&path, &private_expected);
    let mut private_observed = private_expected.clone();
    private_observed.cleanup.remove_fillers = !private_observed.cleanup.remove_fillers;
    let expected_before = private_expected.clone();
    let observed_before = private_observed.clone();
    let file_before = fs::read(&path).expect("read synthetic file before pure assessments");

    let first_assessment = assess_stale_update(&private_expected, &private_observed);
    let second_assessment = assess_stale_update(&private_expected, &private_observed);
    assert_eq!(first_assessment, second_assessment);
    assert_eq!(
        first_assessment,
        StaleUpdateAssessment::RejectedStaleSnapshot
    );
    let outcome_text = format!("{first_assessment:?}");
    for entry in &private_expected.vocabulary {
        for private_value in [&entry.id, &entry.spoken, &entry.written] {
            assert!(!outcome_text.contains(private_value));
        }
    }
    assert_eq!(private_expected, expected_before);
    assert_eq!(private_observed, observed_before);
    assert_eq!(
        fs::read(&path).expect("read synthetic file after pure assessments"),
        file_before
    );

    println!("Outflow stale settings update contract");
    println!("Equal snapshots: MATCH");
    println!("Changed preference: STALE");
    println!("Independent stale update: REJECTED");
    println!("Refreshed snapshot: MATCH");
    println!("Changed vocabulary: STALE");
    println!("Vocabulary reorder: DETECTED");
    println!("A -> B -> A history: AMBIGUOUS");
    println!("Missing settings: NO POSITIVE MATCH");
    println!("Unsupported schema: NO POSITIVE MATCH");
    println!();
    println!("Production stale-write protection: NOT IMPLEMENTED");
    println!("Atomic compare-and-write: NOT IMPLEMENTED");
    println!("Automatic merge: NO");
    println!("File locking: NO");
    println!("Settings files modified by assessment: NO");
    println!("Real user settings accessed: NO");
}
