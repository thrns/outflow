use outflow_settings::{
    Settings, SettingsStorageError, VocabularyEntry, load_settings, save_settings_atomic,
};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttemptKind {
    SettingsUpdate,
    ConfirmedReset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleSettingsState {
    NotInspected,
    OriginalVisible,
    AttemptedVisible,
    OtherValidSettingsVisible,
    Missing,
    Unreadable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriterOutcome {
    ReplacementReportedUnsuccessful,
    ReplacementDurabilityUncertain,
    OtherStorageFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GuidanceStatus {
    CurrentSettingsNotInspected,
    OriginalSettingsCurrentlyVisible,
    AttemptedSettingsCurrentlyVisible,
    UnexpectedSettingsCurrentlyVisible,
    MissingOrUnreadableSettingsNeedReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RecommendedAction {
    InspectReadOnly,
    PresentObservedSettings,
    RecoveryReviewRequired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutomaticRecoveryPolicy {
    /// No implicit retry, rollback, reset, deletion, migration, or repair.
    Prohibited,
}

/// Test-only proposal for a future caller. This value contains classifications
/// only: it never owns or serializes any settings data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PersistenceGuidance {
    attempt: AttemptKind,
    writer_outcome: WriterOutcome,
    observation: VisibleSettingsState,
    status: GuidanceStatus,
    recommended_action: RecommendedAction,
    automatic_recovery: AutomaticRecoveryPolicy,
    durable_persistence_confirmed: bool,
    durability_uncertainty_reported: bool,
    rollback_confirmed: bool,
}

fn guidance_for_failed_settings_operation(
    error: &SettingsStorageError,
    attempt: AttemptKind,
    observation: VisibleSettingsState,
) -> PersistenceGuidance {
    let writer_outcome = match error {
        SettingsStorageError::ReplacementFailed => WriterOutcome::ReplacementReportedUnsuccessful,
        SettingsStorageError::ReplacementDurabilityUncertain => {
            WriterOutcome::ReplacementDurabilityUncertain
        }
        _ => WriterOutcome::OtherStorageFailure,
    };

    let status = match observation {
        VisibleSettingsState::NotInspected => GuidanceStatus::CurrentSettingsNotInspected,
        VisibleSettingsState::OriginalVisible => GuidanceStatus::OriginalSettingsCurrentlyVisible,
        VisibleSettingsState::AttemptedVisible => GuidanceStatus::AttemptedSettingsCurrentlyVisible,
        VisibleSettingsState::OtherValidSettingsVisible => {
            GuidanceStatus::UnexpectedSettingsCurrentlyVisible
        }
        VisibleSettingsState::Missing | VisibleSettingsState::Unreadable => {
            GuidanceStatus::MissingOrUnreadableSettingsNeedReview
        }
    };

    let recommended_action = match observation {
        VisibleSettingsState::NotInspected => RecommendedAction::InspectReadOnly,
        VisibleSettingsState::OriginalVisible | VisibleSettingsState::AttemptedVisible => {
            RecommendedAction::PresentObservedSettings
        }
        VisibleSettingsState::OtherValidSettingsVisible
        | VisibleSettingsState::Missing
        | VisibleSettingsState::Unreadable => RecommendedAction::RecoveryReviewRequired,
    };

    PersistenceGuidance {
        attempt,
        writer_outcome,
        observation,
        status,
        recommended_action,
        automatic_recovery: AutomaticRecoveryPolicy::Prohibited,
        // No failed operation proves durable success. The uncertainty variant
        // separately records that the writer specifically reported this risk.
        durable_persistence_confirmed: false,
        durability_uncertainty_reported: matches!(
            writer_outcome,
            WriterOutcome::ReplacementDurabilityUncertain
        ),
        rollback_confirmed: false,
    }
}

fn observe_settings(
    path: &Path,
    original: &Settings,
    attempted: &Settings,
) -> VisibleSettingsState {
    match load_settings(path) {
        Ok(None) => VisibleSettingsState::Missing,
        Err(_) => VisibleSettingsState::Unreadable,
        Ok(Some(current)) if &current == original => VisibleSettingsState::OriginalVisible,
        Ok(Some(current)) if &current == attempted => VisibleSettingsState::AttemptedVisible,
        Ok(Some(_)) => VisibleSettingsState::OtherValidSettingsVisible,
    }
}

fn destination_state_label(guidance: PersistenceGuidance) -> &'static str {
    match guidance.status {
        GuidanceStatus::CurrentSettingsNotInspected => "NOT VERIFIED",
        GuidanceStatus::OriginalSettingsCurrentlyVisible => "ORIGINAL CURRENTLY VISIBLE",
        GuidanceStatus::AttemptedSettingsCurrentlyVisible => "REPLACEMENT CURRENTLY VISIBLE",
        GuidanceStatus::UnexpectedSettingsCurrentlyVisible => "UNEXPECTED SETTINGS VISIBLE",
        GuidanceStatus::MissingOrUnreadableSettingsNeedReview => "MISSING OR UNREADABLE",
    }
}

fn automatic_action_label(guidance: PersistenceGuidance) -> &'static str {
    match guidance.automatic_recovery {
        AutomaticRecoveryPolicy::Prohibited => "NO",
    }
}

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for attempt in 0..128_u32 {
            let path = std::env::temp_dir().join(format!(
                "outflow-settings-guidance-test-{}-{timestamp}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create isolated test directory: {error}"),
            }
        }
        panic!("could not create a unique isolated test directory");
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn settings_with_vocabulary(spoken: &str, written: &str) -> Settings {
    let mut settings = Settings::default();
    settings.vocabulary.push(VocabularyEntry {
        id: "synthetic-entry".to_owned(),
        spoken: spoken.to_owned(),
        written: written.to_owned(),
    });
    settings
}

fn create_settings_file(path: &Path, settings: &Settings) {
    save_settings_atomic(path, settings).expect("write synthetic settings file");
}

#[test]
fn replacement_failed_without_reload_does_not_imply_preservation() {
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementFailed,
        AttemptKind::SettingsUpdate,
        VisibleSettingsState::NotInspected,
    );

    assert_eq!(guidance.attempt, AttemptKind::SettingsUpdate);
    assert_eq!(
        guidance.writer_outcome,
        WriterOutcome::ReplacementReportedUnsuccessful
    );
    assert_eq!(guidance.status, GuidanceStatus::CurrentSettingsNotInspected);
    assert_eq!(
        guidance.recommended_action,
        RecommendedAction::InspectReadOnly
    );
    assert!(!guidance.durable_persistence_confirmed);
    assert!(!guidance.rollback_confirmed);
    assert_eq!(
        guidance.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
}

#[test]
fn replacement_failed_with_original_visible_reports_only_that_observation() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = settings_with_vocabulary("synthetic original phrase", "Original Written");
    let attempted = settings_with_vocabulary("synthetic attempted phrase", "Attempted Written");
    create_settings_file(&path, &original);

    let observation = observe_settings(&path, &original, &attempted);
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementFailed,
        AttemptKind::SettingsUpdate,
        observation,
    );

    assert_eq!(observation, VisibleSettingsState::OriginalVisible);
    assert_eq!(
        guidance.status,
        GuidanceStatus::OriginalSettingsCurrentlyVisible
    );
    assert_eq!(
        guidance.recommended_action,
        RecommendedAction::PresentObservedSettings
    );
    assert!(!guidance.rollback_confirmed);
    assert_eq!(
        guidance.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
}

#[test]
fn replacement_failed_with_attempted_settings_visible_does_not_claim_rollback() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = settings_with_vocabulary("synthetic original phrase", "Original Written");
    let attempted = settings_with_vocabulary("synthetic attempted phrase", "Attempted Written");
    create_settings_file(&path, &attempted);

    let observation = observe_settings(&path, &original, &attempted);
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementFailed,
        AttemptKind::SettingsUpdate,
        observation,
    );

    assert_eq!(
        guidance.status,
        GuidanceStatus::AttemptedSettingsCurrentlyVisible
    );
    assert!(!guidance.rollback_confirmed);
    assert!(!guidance.durable_persistence_confirmed);
    assert_eq!(
        guidance.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
}

#[test]
fn durability_uncertain_without_reload_reports_uncertainty() {
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementDurabilityUncertain,
        AttemptKind::SettingsUpdate,
        VisibleSettingsState::NotInspected,
    );

    assert_eq!(
        guidance.writer_outcome,
        WriterOutcome::ReplacementDurabilityUncertain
    );
    assert_eq!(guidance.status, GuidanceStatus::CurrentSettingsNotInspected);
    assert!(guidance.durability_uncertainty_reported);
    assert!(!guidance.durable_persistence_confirmed);
    assert_eq!(
        guidance.recommended_action,
        RecommendedAction::InspectReadOnly
    );
}

#[test]
fn durability_uncertain_with_attempted_settings_visible_keeps_durability_warning() {
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementDurabilityUncertain,
        AttemptKind::SettingsUpdate,
        VisibleSettingsState::AttemptedVisible,
    );

    assert_eq!(
        guidance.status,
        GuidanceStatus::AttemptedSettingsCurrentlyVisible
    );
    assert!(guidance.durability_uncertainty_reported);
    assert!(!guidance.durable_persistence_confirmed);
    assert_eq!(
        guidance.recommended_action,
        RecommendedAction::PresentObservedSettings
    );
}

#[test]
fn durability_uncertain_with_original_visible_does_not_invent_rollback() {
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementDurabilityUncertain,
        AttemptKind::ConfirmedReset,
        VisibleSettingsState::OriginalVisible,
    );

    assert_eq!(
        guidance.status,
        GuidanceStatus::OriginalSettingsCurrentlyVisible
    );
    assert!(guidance.durability_uncertainty_reported);
    assert!(!guidance.rollback_confirmed);
    assert_eq!(
        guidance.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
}

#[test]
fn unexpected_valid_configuration_requires_recovery_review() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = settings_with_vocabulary("synthetic original phrase", "Original Written");
    let attempted = settings_with_vocabulary("synthetic attempted phrase", "Attempted Written");
    let other = settings_with_vocabulary("synthetic third phrase", "Third Written");
    create_settings_file(&path, &other);

    let observation = observe_settings(&path, &original, &attempted);
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementFailed,
        AttemptKind::SettingsUpdate,
        observation,
    );

    assert_eq!(observation, VisibleSettingsState::OtherValidSettingsVisible);
    assert_eq!(
        guidance.status,
        GuidanceStatus::UnexpectedSettingsCurrentlyVisible
    );
    assert_eq!(
        guidance.recommended_action,
        RecommendedAction::RecoveryReviewRequired
    );
    assert_eq!(
        guidance.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
}

#[test]
fn missing_settings_are_not_initialized_or_created_by_observation() {
    let directory = TestDirectory::new();
    let path = directory.path().join("missing-settings.json");
    let original = Settings::default();
    let attempted = settings_with_vocabulary("synthetic attempted phrase", "Attempted Written");

    let observation = observe_settings(&path, &original, &attempted);
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementFailed,
        AttemptKind::SettingsUpdate,
        observation,
    );

    assert_eq!(observation, VisibleSettingsState::Missing);
    assert_eq!(
        guidance.status,
        GuidanceStatus::MissingOrUnreadableSettingsNeedReview
    );
    assert_eq!(
        guidance.recommended_action,
        RecommendedAction::RecoveryReviewRequired
    );
    assert!(
        !path.exists(),
        "read-only observation must not create defaults"
    );
    assert_eq!(
        guidance.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
}

#[test]
fn unreadable_settings_are_not_reset_or_repaired() {
    let directory = TestDirectory::new();
    let path = directory.path().join("unreadable-settings.json");
    let malformed = br#"{"schema_version": "synthetic unreadable fixture""#;
    fs::write(&path, malformed).expect("write malformed synthetic settings fixture");
    let original = Settings::default();
    let attempted = settings_with_vocabulary("synthetic attempted phrase", "Attempted Written");

    let observation = observe_settings(&path, &original, &attempted);
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementDurabilityUncertain,
        AttemptKind::ConfirmedReset,
        observation,
    );

    assert_eq!(observation, VisibleSettingsState::Unreadable);
    assert_eq!(
        guidance.recommended_action,
        RecommendedAction::RecoveryReviewRequired
    );
    assert_eq!(
        fs::read(&path).expect("read unchanged malformed fixture"),
        malformed
    );
    assert_eq!(
        guidance.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
}

#[test]
fn confirmed_reset_failure_never_repeats_reset_automatically() {
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementDurabilityUncertain,
        AttemptKind::ConfirmedReset,
        VisibleSettingsState::NotInspected,
    );

    assert_eq!(guidance.attempt, AttemptKind::ConfirmedReset);
    assert_eq!(
        guidance.recommended_action,
        RecommendedAction::InspectReadOnly
    );
    assert!(!guidance.rollback_confirmed);
    assert_eq!(
        guidance.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
}

#[test]
fn guidance_does_not_contain_settings_content_or_serialized_json() {
    let original = settings_with_vocabulary(
        "synthetic private spoken phrase",
        "Synthetic Private Written Value",
    );
    let attempted = settings_with_vocabulary(
        "synthetic attempted spoken phrase",
        "Synthetic Attempted Written Value",
    );
    let error = SettingsStorageError::ReplacementFailed;
    let guidance = guidance_for_failed_settings_operation(
        &error,
        AttemptKind::SettingsUpdate,
        VisibleSettingsState::AttemptedVisible,
    );
    let output = format!("{guidance:?} {error:?} {error}");

    for private_value in [
        original.vocabulary[0].spoken.as_str(),
        original.vocabulary[0].written.as_str(),
        attempted.vocabulary[0].spoken.as_str(),
        attempted.vocabulary[0].written.as_str(),
    ] {
        assert!(!output.contains(private_value));
    }
    assert!(!output.contains("\"schema_version\""));
    assert!(!output.contains("synthetic-entry"));
}

#[test]
fn guidance_is_deterministic_and_other_errors_use_a_conservative_fallback() {
    let error = SettingsStorageError::TemporaryFileWriteFailed;
    let first = guidance_for_failed_settings_operation(
        &error,
        AttemptKind::SettingsUpdate,
        VisibleSettingsState::NotInspected,
    );
    let second = guidance_for_failed_settings_operation(
        &error,
        AttemptKind::SettingsUpdate,
        VisibleSettingsState::NotInspected,
    );

    assert_eq!(first, second);
    assert_eq!(first.writer_outcome, WriterOutcome::OtherStorageFailure);
    assert_eq!(first.status, GuidanceStatus::CurrentSettingsNotInspected);
    assert_eq!(first.recommended_action, RecommendedAction::InspectReadOnly);
    assert!(!first.durable_persistence_confirmed);
    assert!(!first.rollback_confirmed);
    assert!(!first.durability_uncertainty_reported);
    assert_eq!(
        first.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
}

#[test]
fn read_only_loader_observation_does_not_change_existing_production_behavior() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = settings_with_vocabulary("synthetic unchanged phrase", "Unchanged Written");
    let attempted = settings_with_vocabulary("synthetic attempted phrase", "Attempted Written");
    create_settings_file(&path, &original);
    let before = fs::read(&path).expect("capture synthetic settings bytes");

    let observation = observe_settings(&path, &original, &attempted);
    let guidance = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementFailed,
        AttemptKind::SettingsUpdate,
        observation,
    );
    let loaded_after = load_settings(&path)
        .expect("existing public loader remains usable")
        .expect("synthetic settings remain present");
    let after = fs::read(&path).expect("read synthetic settings bytes after observation");

    assert_eq!(guidance.observation, VisibleSettingsState::OriginalVisible);
    assert_eq!(loaded_after, original);
    assert_eq!(after, before, "guidance must not write the settings file");
    assert!(directory.path().starts_with(std::env::temp_dir()));
}

#[test]
fn caller_guidance_summary_is_assertion_backed_and_content_free() {
    let directory = TestDirectory::new();
    let original = settings_with_vocabulary("synthetic original vocabulary", "Original Written");
    let attempted_update =
        settings_with_vocabulary("synthetic replacement vocabulary", "Replacement Written");
    let reset_defaults = Settings::default();
    let original_path = directory.path().join("original.json");
    let replacement_path = directory.path().join("replacement.json");
    let reset_path = directory.path().join("reset.json");
    let unreadable_path = directory.path().join("unreadable.json");
    create_settings_file(&original_path, &original);
    create_settings_file(&replacement_path, &attempted_update);
    create_settings_file(&reset_path, &reset_defaults);
    let malformed = br#"{"schema_version": "synthetic unreadable fixture""#;
    fs::write(&unreadable_path, malformed).expect("write malformed synthetic fixture");
    let settings_paths = [
        &original_path,
        &replacement_path,
        &reset_path,
        &unreadable_path,
    ];
    let settings_bytes_before = settings_paths
        .iter()
        .map(|path| fs::read(path).expect("capture synthetic settings file"))
        .collect::<Vec<_>>();

    let failed_not_reloaded = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementFailed,
        AttemptKind::SettingsUpdate,
        VisibleSettingsState::NotInspected,
    );
    let failed_original = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementFailed,
        AttemptKind::SettingsUpdate,
        observe_settings(&original_path, &original, &attempted_update),
    );
    let uncertain_replacement = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementDurabilityUncertain,
        AttemptKind::SettingsUpdate,
        observe_settings(&replacement_path, &original, &attempted_update),
    );
    let uncertain_reset = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementDurabilityUncertain,
        AttemptKind::ConfirmedReset,
        observe_settings(&reset_path, &original, &reset_defaults),
    );
    let unreadable = guidance_for_failed_settings_operation(
        &SettingsStorageError::ReplacementDurabilityUncertain,
        AttemptKind::ConfirmedReset,
        observe_settings(&unreadable_path, &original, &reset_defaults),
    );

    assert_eq!(
        failed_not_reloaded.status,
        GuidanceStatus::CurrentSettingsNotInspected
    );
    assert_eq!(
        failed_not_reloaded.writer_outcome,
        WriterOutcome::ReplacementReportedUnsuccessful
    );
    assert_eq!(
        failed_not_reloaded.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
    assert_eq!(
        failed_original.status,
        GuidanceStatus::OriginalSettingsCurrentlyVisible
    );
    assert!(!failed_original.rollback_confirmed);
    assert_eq!(
        uncertain_replacement.status,
        GuidanceStatus::AttemptedSettingsCurrentlyVisible
    );
    assert!(uncertain_replacement.durability_uncertainty_reported);
    assert!(!uncertain_replacement.durable_persistence_confirmed);
    assert!(original.vocabulary.len() == 1);
    assert!(reset_defaults.vocabulary.is_empty());
    assert_eq!(uncertain_reset.attempt, AttemptKind::ConfirmedReset);
    assert_eq!(
        uncertain_reset.status,
        GuidanceStatus::AttemptedSettingsCurrentlyVisible
    );
    assert_eq!(
        uncertain_reset.automatic_recovery,
        AutomaticRecoveryPolicy::Prohibited
    );
    assert_eq!(unreadable.observation, VisibleSettingsState::Unreadable);
    assert_eq!(
        unreadable.recommended_action,
        RecommendedAction::RecoveryReviewRequired
    );
    for (path, before) in settings_paths.iter().zip(&settings_bytes_before) {
        assert_eq!(
            fs::read(path)
                .expect("reread synthetic settings file")
                .as_slice(),
            before.as_slice()
        );
    }
    assert!(
        std::any::type_name::<PersistenceGuidance>()
            .contains("settings_persistence_guidance_contract")
    );
    assert!(directory.path().starts_with(std::env::temp_dir()));
    assert!(directory.path().file_name().is_some_and(|name| {
        name.to_string_lossy()
            .starts_with("outflow-settings-guidance-test-")
    }));

    println!("Outflow settings persistence caller guidance\n");
    println!("ReplacementFailed, not reloaded:");
    println!(
        "  Destination state: {}",
        destination_state_label(failed_not_reloaded)
    );
    println!(
        "  Automatic retry: {}\n",
        automatic_action_label(failed_not_reloaded)
    );
    println!("ReplacementFailed, original visible:");
    println!(
        "  Destination state: {}",
        destination_state_label(failed_original)
    );
    println!(
        "  Universal rollback guarantee: {}\n",
        if failed_original.rollback_confirmed {
            "YES"
        } else {
            "NO"
        }
    );
    println!("ReplacementDurabilityUncertain, replacement visible:");
    println!(
        "  Destination state: {}",
        destination_state_label(uncertain_replacement)
    );
    println!(
        "  Crash durability: {}",
        if uncertain_replacement.durability_uncertainty_reported
            && !uncertain_replacement.durable_persistence_confirmed
        {
            "UNCONFIRMED"
        } else {
            "NOT ESTABLISHED"
        }
    );
    println!(
        "  Automatic rollback: {}\n",
        automatic_action_label(uncertain_replacement)
    );
    println!("Confirmed reset with durability uncertainty:");
    println!(
        "  Original vocabulary assumed preserved: {}",
        if uncertain_reset.status == GuidanceStatus::AttemptedSettingsCurrentlyVisible
            && reset_defaults.vocabulary.is_empty()
            && !original.vocabulary.is_empty()
        {
            "NO"
        } else {
            "UNKNOWN"
        }
    );
    println!(
        "  Automatic reset retry: {}\n",
        automatic_action_label(uncertain_reset)
    );
    println!("Unreadable settings:");
    println!(
        "  Recovery review required: {}",
        if unreadable.recommended_action == RecommendedAction::RecoveryReviewRequired {
            "YES"
        } else {
            "NO"
        }
    );
    println!(
        "  Automatic overwrite: {}\n",
        automatic_action_label(unreadable)
    );
    println!(
        "Settings files modified by guidance: {}",
        if settings_paths
            .iter()
            .zip(&settings_bytes_before)
            .all(|(path, before)| fs::read(path).is_ok_and(|after| after == *before))
        {
            "NO"
        } else {
            "YES"
        }
    );
    println!(
        "Real user settings accessed: {}",
        if directory.path().starts_with(std::env::temp_dir())
            && directory.path().file_name().is_some_and(|name| name
                .to_string_lossy()
                .starts_with("outflow-settings-guidance-test-"))
        {
            "NO"
        } else {
            "YES"
        }
    );
    println!(
        "Production caller implemented: {}",
        if std::any::type_name::<PersistenceGuidance>()
            .contains("settings_persistence_guidance_contract")
        {
            "NO"
        } else {
            "YES"
        }
    );
}
