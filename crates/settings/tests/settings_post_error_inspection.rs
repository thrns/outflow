use outflow_settings::{
    CleanupMode, Settings, SettingsError, SettingsStorageError, VocabularyEntry, load_settings,
    save_settings_atomic,
};
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisibleSettingsInspection {
    OriginalVisible,
    AttemptedVisible,
    BothSnapshotsMatch,
    OtherValidSettingsVisible,
    Missing,
    InvalidDocument,
    UnsupportedSchema,
    ReadFailure,
}

/// Classifies one read-only loader snapshot. Another operation may change the
/// file afterward; this observation says nothing about crash durability.
fn inspect_visible_settings(
    path: &Path,
    original: &Settings,
    attempted: &Settings,
) -> VisibleSettingsInspection {
    match load_settings(path) {
        Ok(None) => VisibleSettingsInspection::Missing,
        Ok(Some(visible)) => {
            let matches_original = &visible == original;
            let matches_attempted = &visible == attempted;
            match (matches_original, matches_attempted) {
                (true, true) => VisibleSettingsInspection::BothSnapshotsMatch,
                (true, false) => VisibleSettingsInspection::OriginalVisible,
                (false, true) => VisibleSettingsInspection::AttemptedVisible,
                (false, false) => VisibleSettingsInspection::OtherValidSettingsVisible,
            }
        }
        Err(SettingsStorageError::InvalidSettings(SettingsError::UnsupportedSchemaVersion {
            ..
        })) => VisibleSettingsInspection::UnsupportedSchema,
        Err(
            SettingsStorageError::InvalidDocument { .. }
            | SettingsStorageError::InvalidUtf8
            | SettingsStorageError::SettingsDecodingFailed
            | SettingsStorageError::InvalidSettings(_),
        ) => VisibleSettingsInspection::InvalidDocument,
        Err(_) => VisibleSettingsInspection::ReadFailure,
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
                "outflow-settings-post-error-inspection-{}-{timestamp}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => {
                    fs::write(path.join("unrelated-sentinel.txt"), b"synthetic sentinel")
                        .expect("write isolated unrelated sentinel");
                    return Self(path);
                }
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

#[derive(Debug, PartialEq, Eq)]
struct DirectoryEntrySnapshot {
    name: OsString,
    is_file: bool,
    len: u64,
    modified: Option<SystemTime>,
    contents: Option<Vec<u8>>,
    #[cfg(unix)]
    mode: u32,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
}

fn snapshot_directory(directory: &Path) -> Vec<DirectoryEntrySnapshot> {
    let mut entries = fs::read_dir(directory)
        .expect("read isolated test directory")
        .map(|entry| {
            let entry = entry.expect("read isolated directory entry");
            let metadata = fs::symlink_metadata(entry.path()).expect("inspect directory entry");
            let is_file = metadata.file_type().is_file();
            DirectoryEntrySnapshot {
                name: entry.file_name(),
                is_file,
                len: metadata.len(),
                modified: metadata.modified().ok(),
                contents: is_file.then(|| fs::read(entry.path()).expect("read fixture file")),
                #[cfg(unix)]
                mode: {
                    use std::os::unix::fs::MetadataExt;
                    metadata.mode()
                },
                #[cfg(unix)]
                device: {
                    use std::os::unix::fs::MetadataExt;
                    metadata.dev()
                },
                #[cfg(unix)]
                inode: {
                    use std::os::unix::fs::MetadataExt;
                    metadata.ino()
                },
            }
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    entries
}

fn settings_variant(
    label: &str,
    cleanup_mode: CleanupMode,
    remove_fillers: bool,
    run_at_login: bool,
) -> Settings {
    let mut settings = Settings::default();
    settings.recording.push_to_talk = Some(format!("synthetic-{label}-push"));
    settings.recording.toggle = Some(format!("synthetic-{label}-toggle"));
    settings.recording.input_device = Some(format!("synthetic-{label}-device"));
    settings.cleanup.mode = cleanup_mode;
    settings.cleanup.remove_fillers = remove_fillers;
    settings.startup.run_at_login = run_at_login;
    settings.vocabulary.push(VocabularyEntry {
        id: format!("synthetic-{label}-vocabulary-id"),
        spoken: format!("synthetic {label} spoken phrase"),
        written: format!("Synthetic {label} written phrase"),
    });
    settings
}

fn original_settings() -> Settings {
    settings_variant("original", CleanupMode::Clean, false, true)
}

fn attempted_settings() -> Settings {
    settings_variant("attempted", CleanupMode::Verbatim, true, false)
}

fn other_settings() -> Settings {
    settings_variant("third", CleanupMode::Clean, true, false)
}

fn write_valid_settings(path: &Path, settings: &Settings) {
    save_settings_atomic(path, settings).expect("write synthetic settings through public writer");
}

fn inspect_twice_and_assert_unchanged(
    path: &Path,
    original: &Settings,
    attempted: &Settings,
    expected: VisibleSettingsInspection,
) -> VisibleSettingsInspection {
    let parent = path.parent().expect("fixture settings path has a parent");
    let before = snapshot_directory(parent);

    let first = inspect_visible_settings(path, original, attempted);
    let after_first = snapshot_directory(parent);
    let second = inspect_visible_settings(path, original, attempted);
    let after_second = snapshot_directory(parent);

    assert_eq!(first, expected);
    assert_eq!(second, expected);
    assert_eq!(
        first, second,
        "repeated classifications must be deterministic"
    );
    assert_eq!(
        after_first, before,
        "first inspection must be non-destructive"
    );
    assert_eq!(
        after_second, before,
        "second inspection must be non-destructive"
    );
    first
}

fn assert_private_classification(
    inspection: VisibleSettingsInspection,
    settings: &[&Settings],
    raw_document: Option<&[u8]>,
) {
    let debug = format!("{inspection:?}");
    assert!(!debug.contains("schema_version"));
    assert!(!debug.contains('{'));
    for settings in settings {
        for entry in &settings.vocabulary {
            for private_value in [&entry.id, &entry.spoken, &entry.written] {
                assert!(!debug.contains(private_value));
            }
        }
    }
    if let Some(raw_document) = raw_document {
        let raw_marker = String::from_utf8_lossy(raw_document);
        assert!(!debug.contains(raw_marker.as_ref()));
        assert!(!debug.contains("SYNTHETIC_RAW_MALFORMED"));
        assert!(!debug.contains("SYNTHETIC_RAW_UNSUPPORTED"));
    }
}

#[cfg(unix)]
fn assert_unix_metadata_is_present(directory: &Path, name: &str) {
    let before = snapshot_directory(directory);
    let entry = before
        .iter()
        .find(|entry| entry.name == name)
        .expect("expected fixture entry in directory snapshot");
    assert!(entry.is_file);
    assert!(
        entry.modified.is_some(),
        "filesystem exposes modification time"
    );
    assert_ne!(entry.device, 0);
    assert_ne!(entry.inode, 0);
    assert_eq!(
        entry.mode & 0o777,
        0o600,
        "public writer uses owner-only mode"
    );
}

#[test]
fn original_settings_are_visible_and_unchanged() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = original_settings();
    let attempted = attempted_settings();
    write_valid_settings(&path, &original);

    #[cfg(unix)]
    assert_unix_metadata_is_present(directory.path(), "settings.json");

    let inspection = inspect_twice_and_assert_unchanged(
        &path,
        &original,
        &attempted,
        VisibleSettingsInspection::OriginalVisible,
    );
    assert_private_classification(inspection, &[&original, &attempted], None);
}

#[test]
fn attempted_settings_are_visible_and_unchanged() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = original_settings();
    let attempted = attempted_settings();
    write_valid_settings(&path, &attempted);

    let inspection = inspect_twice_and_assert_unchanged(
        &path,
        &original,
        &attempted,
        VisibleSettingsInspection::AttemptedVisible,
    );
    assert_private_classification(inspection, &[&original, &attempted], None);
}

#[test]
fn third_valid_settings_are_classified_as_other_not_corruption() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = original_settings();
    let attempted = attempted_settings();
    let other = other_settings();
    assert_ne!(other, original);
    assert_ne!(other, attempted);
    write_valid_settings(&path, &other);

    let inspection = inspect_twice_and_assert_unchanged(
        &path,
        &original,
        &attempted,
        VisibleSettingsInspection::OtherValidSettingsVisible,
    );
    let loaded = load_settings(&path).expect("third valid schema-v1 file loads");
    assert_eq!(loaded, Some(other));
    assert_private_classification(inspection, &[&original, &attempted], None);
}

#[test]
fn equal_snapshots_remain_ambiguous_when_both_match() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = settings_variant("identical", CleanupMode::Verbatim, false, true);
    let attempted = settings_variant("identical", CleanupMode::Verbatim, false, true);
    assert_eq!(original, attempted);
    write_valid_settings(&path, &original);

    let inspection = inspect_twice_and_assert_unchanged(
        &path,
        &original,
        &attempted,
        VisibleSettingsInspection::BothSnapshotsMatch,
    );
    assert_private_classification(inspection, &[&original, &attempted], None);
}

#[test]
fn earlier_observation_stays_stale_after_an_explicit_write() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = original_settings();
    let attempted = attempted_settings();
    assert_ne!(original, attempted);

    write_valid_settings(&path, &original);
    let before_first_inspection = snapshot_directory(directory.path());
    let first = inspect_visible_settings(&path, &original, &attempted);
    assert_eq!(first, VisibleSettingsInspection::OriginalVisible);
    assert_eq!(
        snapshot_directory(directory.path()),
        before_first_inspection,
        "first inspection must leave the original settings unchanged"
    );

    save_settings_atomic(&path, &attempted)
        .expect("explicitly write attempted settings through the public writer");
    assert_eq!(
        first,
        VisibleSettingsInspection::OriginalVisible,
        "a later write must not reinterpret the retained earlier observation"
    );

    let before_fresh_inspection = snapshot_directory(directory.path());
    let fresh = inspect_visible_settings(&path, &original, &attempted);
    let after_fresh_inspection = snapshot_directory(directory.path());
    assert_eq!(fresh, VisibleSettingsInspection::AttemptedVisible);
    assert_eq!(
        load_settings(&path).expect("freshly written settings load"),
        Some(attempted.clone()),
        "a new loader call observes the attempted configuration"
    );
    assert_eq!(
        after_fresh_inspection, before_fresh_inspection,
        "inspection itself must not modify the explicitly written settings"
    );
    assert_eq!(first, VisibleSettingsInspection::OriginalVisible);
    assert_private_classification(first, &[&original, &attempted], None);
    assert_private_classification(fresh, &[&original, &attempted], None);
}

#[test]
fn equal_snapshots_are_ambiguous_across_distinct_write_histories() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = settings_variant("same-history", CleanupMode::Verbatim, false, true);
    let attempted = original.clone();
    assert_eq!(original, attempted);

    // History A: one explicit write, then an inspection with no intervening write.
    write_valid_settings(&path, &original);
    let before_no_intervening_write_inspection = snapshot_directory(directory.path());
    let no_intervening_write = inspect_visible_settings(&path, &original, &attempted);
    assert_eq!(
        snapshot_directory(directory.path()),
        before_no_intervening_write_inspection,
        "inspection must not modify History A"
    );
    assert_eq!(
        no_intervening_write,
        VisibleSettingsInspection::BothSnapshotsMatch
    );

    // History B: explicitly write the same values again before inspecting.
    save_settings_atomic(&path, &original)
        .expect("explicitly write the same configuration again through the public writer");
    let before_intervening_write_inspection = snapshot_directory(directory.path());
    let same_values_after_write = inspect_visible_settings(&path, &original, &attempted);
    assert_eq!(
        snapshot_directory(directory.path()),
        before_intervening_write_inspection,
        "inspection must not modify History B"
    );
    assert_eq!(
        same_values_after_write,
        VisibleSettingsInspection::BothSnapshotsMatch
    );

    assert_eq!(no_intervening_write, same_values_after_write);
    assert_private_classification(no_intervening_write, &[&original, &attempted], None);
    assert_private_classification(same_values_after_write, &[&original, &attempted], None);
}

#[test]
fn a_b_a_inspections_do_not_reveal_the_intermediate_history() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = original_settings();
    let attempted = attempted_settings();
    assert_ne!(original, attempted);

    write_valid_settings(&path, &original);
    let before_first_inspection = snapshot_directory(directory.path());
    let first = inspect_visible_settings(&path, &original, &attempted);
    assert_eq!(first, VisibleSettingsInspection::OriginalVisible);
    assert_eq!(
        snapshot_directory(directory.path()),
        before_first_inspection,
        "first A inspection must be read-only"
    );

    save_settings_atomic(&path, &attempted)
        .expect("explicitly write B through the public settings writer");
    let before_intermediate_inspection = snapshot_directory(directory.path());
    let intermediate = inspect_visible_settings(&path, &original, &attempted);
    assert_eq!(intermediate, VisibleSettingsInspection::AttemptedVisible);
    assert_eq!(
        snapshot_directory(directory.path()),
        before_intermediate_inspection,
        "intermediate B inspection must be read-only"
    );

    save_settings_atomic(&path, &original)
        .expect("explicitly write A again as a deliberate test operation");
    let before_final_inspection = snapshot_directory(directory.path());
    let final_observation = inspect_visible_settings(&path, &original, &attempted);
    assert_eq!(
        final_observation,
        VisibleSettingsInspection::OriginalVisible
    );
    assert_eq!(
        snapshot_directory(directory.path()),
        before_final_inspection,
        "final A inspection must be read-only"
    );

    assert_eq!(first, final_observation);
    assert_ne!(first, intermediate, "the intervening B state was observed");
    assert_eq!(
        load_settings(&path).expect("final A configuration loads"),
        Some(original.clone())
    );
    for inspection in [first, intermediate, final_observation] {
        assert_private_classification(inspection, &[&original, &attempted], None);
    }
}

#[test]
fn two_actors_observe_a_deterministically_ordered_write_between_inspections() {
    // This handshake proves ordering between completed operations only. It does
    // not exercise overlapping filesystem calls or establish linearizability.
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = original_settings();
    let attempted = attempted_settings();
    assert_ne!(original, attempted);
    write_valid_settings(&path, &original);

    let (inspection_complete_tx, inspection_complete_rx) =
        mpsc::channel::<VisibleSettingsInspection>();
    let (write_complete_tx, write_complete_rx) =
        mpsc::channel::<(VisibleSettingsInspection, bool)>();

    let inspector_path = path.clone();
    let inspector_original = original.clone();
    let inspector_attempted = attempted.clone();
    let inspector_directory = directory.path().to_path_buf();
    let inspector = thread::spawn(move || {
        let before_first = snapshot_directory(&inspector_directory);
        let first =
            inspect_visible_settings(&inspector_path, &inspector_original, &inspector_attempted);
        let after_first = snapshot_directory(&inspector_directory);
        inspection_complete_tx
            .send(first)
            .expect("signal completion of first inspection");

        let (writer_saw, write_succeeded) = write_complete_rx
            .recv()
            .expect("receive explicit writer completion");
        let before_fresh = snapshot_directory(&inspector_directory);
        let fresh =
            inspect_visible_settings(&inspector_path, &inspector_original, &inspector_attempted);
        let after_fresh = snapshot_directory(&inspector_directory);

        (
            first,
            before_first,
            after_first,
            writer_saw,
            write_succeeded,
            fresh,
            before_fresh,
            after_fresh,
        )
    });

    let writer_path = path.clone();
    let writer_attempted = attempted.clone();
    let writer = thread::spawn(move || {
        let first = inspection_complete_rx
            .recv()
            .expect("wait for first inspection completion");
        let write_succeeded = save_settings_atomic(&writer_path, &writer_attempted).is_ok();
        write_complete_tx
            .send((first, write_succeeded))
            .expect("signal explicit write completion");
    });

    let (
        first,
        before_first,
        after_first,
        writer_saw,
        write_succeeded,
        fresh,
        before_fresh,
        after_fresh,
    ) = inspector.join().expect("inspector actor joins");
    writer.join().expect("writer actor joins");

    assert_eq!(first, VisibleSettingsInspection::OriginalVisible);
    assert_eq!(writer_saw, VisibleSettingsInspection::OriginalVisible);
    assert!(write_succeeded, "writer actor completed its public write");
    assert_eq!(fresh, VisibleSettingsInspection::AttemptedVisible);
    assert_eq!(after_first, before_first, "first inspection was read-only");
    assert_eq!(after_fresh, before_fresh, "fresh inspection was read-only");
    assert_eq!(
        load_settings(&path).expect("attempted settings remain loadable after actors join"),
        Some(attempted.clone())
    );
    assert_private_classification(first, &[&original, &attempted], None);
    assert_private_classification(fresh, &[&original, &attempted], None);
}

#[test]
fn removing_a_file_after_inspection_does_not_rewrite_the_prior_observation() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = original_settings();
    let attempted = attempted_settings();
    write_valid_settings(&path, &original);

    let before_original_inspection = snapshot_directory(directory.path());
    let first = inspect_visible_settings(&path, &original, &attempted);
    assert_eq!(first, VisibleSettingsInspection::OriginalVisible);
    assert_eq!(
        snapshot_directory(directory.path()),
        before_original_inspection,
        "successful inspection must leave the synthetic file unchanged"
    );

    fs::remove_file(&path).expect("deliberately remove the synthetic settings fixture");
    assert_eq!(first, VisibleSettingsInspection::OriginalVisible);
    let before_missing_inspection = snapshot_directory(directory.path());
    let fresh = inspect_visible_settings(&path, &original, &attempted);
    assert_eq!(fresh, VisibleSettingsInspection::Missing);
    assert_eq!(
        snapshot_directory(directory.path()),
        before_missing_inspection,
        "missing-file inspection must not recreate or otherwise modify fixtures"
    );
    assert!(
        !path.exists(),
        "inspection must not recreate the removed file"
    );
    assert_eq!(
        snapshot_directory(directory.path())
            .iter()
            .filter(|entry| entry.name == "unrelated-sentinel.txt")
            .count(),
        1,
        "only the unrelated synthetic sentinel remains"
    );
    assert_private_classification(first, &[&original, &attempted], None);
    assert_private_classification(fresh, &[&original, &attempted], None);
}

#[test]
fn snapshot_boundary_summary_is_assertion_backed_and_content_free() {
    // Re-run each focused scenario here so the single captured summary is emitted
    // only after the assertions supporting every reported result have passed.
    earlier_observation_stays_stale_after_an_explicit_write();
    equal_snapshots_are_ambiguous_across_distinct_write_histories();
    a_b_a_inspections_do_not_reveal_the_intermediate_history();
    two_actors_observe_a_deterministically_ordered_write_between_inspections();
    removing_a_file_after_inspection_does_not_rewrite_the_prior_observation();

    let summary = "Outflow settings snapshot boundary checks\n\n\
Original observation: VERIFIED\n\
Intervening settings write: COMPLETED\n\
Fresh observation: UPDATED\n\
Earlier classification: UNCHANGED\n\n\
Equal snapshots with no new write: AMBIGUOUS\n\
Equal snapshots after another write: AMBIGUOUS\n\
A → B → A history: VERIFIED\n\n\
Two-actor ordered interleaving: PASS\n\
Missing file after prior inspection: DETECTED\n\n\
Inspection provides file locking: NO\n\
Inspection establishes write history: NO\n\
Inspection guarantees crash durability: NO\n\
Automatic retry or rollback: NO\n\
Real user settings accessed: NO\n\
Production concurrency control implemented: NO";
    assert!(!summary.contains("synthetic"));
    assert!(!summary.contains("schema_version"));
    assert!(!summary.contains('{'));
    println!("{summary}");
}

#[test]
fn missing_settings_remain_missing_without_default_creation() {
    let directory = TestDirectory::new();
    let path = directory.path().join("missing-settings.json");
    let original = original_settings();
    let attempted = attempted_settings();

    let inspection = inspect_twice_and_assert_unchanged(
        &path,
        &original,
        &attempted,
        VisibleSettingsInspection::Missing,
    );
    assert!(
        !path.exists(),
        "inspection must not initialize missing settings"
    );
    assert_private_classification(inspection, &[&original, &attempted], None);
}

#[test]
fn malformed_json_is_classified_and_its_exact_bytes_are_preserved() {
    let directory = TestDirectory::new();
    let path = directory.path().join("malformed-settings.json");
    let malformed = br#"{"schema_version":1,"private_marker":"SYNTHETIC_RAW_MALFORMED""#;
    fs::write(&path, malformed).expect("write malformed synthetic JSON directly");
    let original = original_settings();
    let attempted = attempted_settings();

    let inspection = inspect_twice_and_assert_unchanged(
        &path,
        &original,
        &attempted,
        VisibleSettingsInspection::InvalidDocument,
    );
    assert_eq!(fs::read(&path).expect("read malformed fixture"), malformed);
    assert_private_classification(inspection, &[&original, &attempted], Some(malformed));
}

#[test]
fn unsupported_schema_is_classified_and_original_bytes_are_preserved() {
    let directory = TestDirectory::new();
    let path = directory.path().join("unsupported-settings.json");
    let unsupported = br#"{"schema_version":999,"private_marker":"SYNTHETIC_RAW_UNSUPPORTED"}"#;
    fs::write(&path, unsupported).expect("write unsupported synthetic JSON directly");
    let original = original_settings();
    let attempted = attempted_settings();

    let inspection = inspect_twice_and_assert_unchanged(
        &path,
        &original,
        &attempted,
        VisibleSettingsInspection::UnsupportedSchema,
    );
    assert_eq!(
        fs::read(&path).expect("read unsupported fixture"),
        unsupported
    );
    assert_private_classification(inspection, &[&original, &attempted], Some(unsupported));
}

#[test]
fn non_regular_settings_path_is_a_conservative_read_failure() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings-directory");
    fs::create_dir(&path).expect("create non-regular synthetic settings path");
    let original = original_settings();
    let attempted = attempted_settings();

    let inspection = inspect_twice_and_assert_unchanged(
        &path,
        &original,
        &attempted,
        VisibleSettingsInspection::ReadFailure,
    );
    assert_private_classification(inspection, &[&original, &attempted], None);
}

#[test]
fn repeated_reads_preserve_bytes_metadata_and_unrelated_files() {
    let directory = TestDirectory::new();
    let original = original_settings();
    let attempted = attempted_settings();
    let cases = [
        (
            "original.json",
            original.clone(),
            VisibleSettingsInspection::OriginalVisible,
        ),
        (
            "attempted.json",
            attempted.clone(),
            VisibleSettingsInspection::AttemptedVisible,
        ),
    ];
    for (name, settings, _) in &cases {
        write_valid_settings(&directory.path().join(name), settings);
    }
    let malformed_path = directory.path().join("malformed.json");
    let malformed = br#"{"broken":"SYNTHETIC_RAW_MALFORMED""#;
    fs::write(&malformed_path, malformed).expect("write malformed repeated-read fixture");
    let unsupported_path = directory.path().join("unsupported.json");
    let unsupported = br#"{"schema_version":2,"private_marker":"SYNTHETIC_RAW_UNSUPPORTED"}"#;
    fs::write(&unsupported_path, unsupported).expect("write unsupported repeated-read fixture");

    for (name, settings, expected) in cases {
        let path = directory.path().join(name);
        assert_eq!(
            settings,
            if expected == VisibleSettingsInspection::OriginalVisible {
                original.clone()
            } else {
                attempted.clone()
            }
        );
        #[cfg(unix)]
        {
            let snapshot = snapshot_directory(directory.path());
            let entry = snapshot
                .iter()
                .find(|entry| entry.name == name)
                .expect("valid settings fixture remains in its directory");
            assert!(entry.modified.is_some());
            assert_ne!(entry.device, 0);
            assert_ne!(entry.inode, 0);
        }
        inspect_twice_and_assert_unchanged(&path, &original, &attempted, expected);
    }
    inspect_twice_and_assert_unchanged(
        &directory.path().join("missing.json"),
        &original,
        &attempted,
        VisibleSettingsInspection::Missing,
    );
    inspect_twice_and_assert_unchanged(
        &malformed_path,
        &original,
        &attempted,
        VisibleSettingsInspection::InvalidDocument,
    );
    inspect_twice_and_assert_unchanged(
        &unsupported_path,
        &original,
        &attempted,
        VisibleSettingsInspection::UnsupportedSchema,
    );
}

#[test]
fn inspection_debug_output_contains_no_settings_or_document_content() {
    let directory = TestDirectory::new();
    let path = directory.path().join("settings.json");
    let original = original_settings();
    let attempted = attempted_settings();
    write_valid_settings(&path, &original);
    let inspection = inspect_visible_settings(&path, &original, &attempted);
    let debug = format!("{inspection:?}");

    assert_eq!(debug, "OriginalVisible");
    assert_private_classification(inspection, &[&original, &attempted], None);
}

#[test]
fn post_error_inspection_summary_is_assertion_backed_and_content_free() {
    let directory = TestDirectory::new();
    let original = original_settings();
    let attempted = attempted_settings();
    let other = other_settings();
    let same_original = settings_variant("identical-summary", CleanupMode::Verbatim, false, true);
    let same_attempted = settings_variant("identical-summary", CleanupMode::Verbatim, false, true);
    assert_ne!(original, attempted);
    assert_ne!(other, original);
    assert_ne!(other, attempted);
    assert_eq!(same_original, same_attempted);

    let original_path = directory.path().join("original.json");
    let attempted_path = directory.path().join("attempted.json");
    let other_path = directory.path().join("other.json");
    let same_path = directory.path().join("same.json");
    let missing_path = directory.path().join("missing.json");
    let malformed_path = directory.path().join("malformed.json");
    let unsupported_path = directory.path().join("unsupported.json");
    write_valid_settings(&original_path, &original);
    write_valid_settings(&attempted_path, &attempted);
    write_valid_settings(&other_path, &other);
    write_valid_settings(&same_path, &same_original);
    let malformed = br#"{"private_marker":"SYNTHETIC_RAW_MALFORMED""#;
    fs::write(&malformed_path, malformed).expect("write malformed summary fixture");
    let unsupported = br#"{"schema_version":999,"private_marker":"SYNTHETIC_RAW_UNSUPPORTED"}"#;
    fs::write(&unsupported_path, unsupported).expect("write unsupported summary fixture");
    let before = snapshot_directory(directory.path());

    let original_observation = inspect_twice_and_assert_unchanged(
        &original_path,
        &original,
        &attempted,
        VisibleSettingsInspection::OriginalVisible,
    );
    let attempted_observation = inspect_twice_and_assert_unchanged(
        &attempted_path,
        &original,
        &attempted,
        VisibleSettingsInspection::AttemptedVisible,
    );
    let other_observation = inspect_visible_settings(&other_path, &original, &attempted);
    let same_observation = inspect_visible_settings(&same_path, &same_original, &same_attempted);
    let missing_observation = inspect_twice_and_assert_unchanged(
        &missing_path,
        &original,
        &attempted,
        VisibleSettingsInspection::Missing,
    );
    let malformed_observation = inspect_twice_and_assert_unchanged(
        &malformed_path,
        &original,
        &attempted,
        VisibleSettingsInspection::InvalidDocument,
    );
    let unsupported_observation = inspect_twice_and_assert_unchanged(
        &unsupported_path,
        &original,
        &attempted,
        VisibleSettingsInspection::UnsupportedSchema,
    );

    assert_eq!(
        other_observation,
        VisibleSettingsInspection::OtherValidSettingsVisible
    );
    assert_eq!(
        same_observation,
        VisibleSettingsInspection::BothSnapshotsMatch
    );
    assert!(!missing_path.exists());
    assert_eq!(
        snapshot_directory(directory.path()),
        before,
        "inspector must preserve all settings bytes, metadata, names, and sentinel"
    );
    assert_eq!(fs::read(&malformed_path).unwrap(), malformed);
    assert_eq!(fs::read(&unsupported_path).unwrap(), unsupported);
    assert!(directory.path().starts_with(std::env::temp_dir()));
    assert_private_classification(
        original_observation,
        &[
            &original,
            &attempted,
            &other,
            &same_original,
            &same_attempted,
        ],
        None,
    );
    assert_private_classification(
        attempted_observation,
        &[
            &original,
            &attempted,
            &other,
            &same_original,
            &same_attempted,
        ],
        None,
    );
    assert_private_classification(
        other_observation,
        &[
            &original,
            &attempted,
            &other,
            &same_original,
            &same_attempted,
        ],
        None,
    );
    assert_private_classification(same_observation, &[&same_original, &same_attempted], None);
    assert_private_classification(missing_observation, &[&original, &attempted], None);
    assert_private_classification(
        malformed_observation,
        &[&original, &attempted],
        Some(malformed),
    );
    assert_private_classification(
        unsupported_observation,
        &[&original, &attempted],
        Some(unsupported),
    );

    println!(
        "Outflow post-error settings inspection\n\n\
Original settings visible: VERIFIED\n\
Attempted settings visible: VERIFIED\n\
Other valid settings visible: VERIFIED\n\
Identical snapshots: AMBIGUOUS\n\
Missing settings: DETECTED\n\
Malformed settings: REJECTED\n\
Unsupported schema: DETECTED\n\
Repeated inspections: NON-DESTRUCTIVE\n\
Original file bytes: PRESERVED\n\
Unrelated files: PRESERVED\n\
Inspection report privacy: VERIFIED\n\n\
Settings files written by inspector: NO\n\
Automatic reset: NO\n\
Automatic retry: NO\n\
Automatic rollback: NO\n\
Crash durability inferred: NO\n\
Production caller implemented: NO\n\
Real user settings accessed: NO"
    );
}
