#![cfg(unix)]

use outflow_settings::{CleanupMode, Settings, load_settings, save_settings_atomic};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SETTINGS_FILE: &str = "settings.json";

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new(label: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();

        for attempt in 0..128_u32 {
            let path = std::env::temp_dir().join(format!(
                "outflow-settings-lost-update-{label}-{}-{timestamp}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WriteOrder {
    ActorOneFirst,
    ActorTwoFirst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaleScenarioResult {
    stale_write_succeeded: bool,
    actor_one_change_visible: bool,
    actor_two_change_visible: bool,
    actor_one_change_lost: bool,
    actor_two_change_lost: bool,
}

fn load_required(path: &Path) -> Settings {
    load_settings(path)
        .expect("load synthetic settings")
        .expect("synthetic settings file exists")
}

fn prepare_two_snapshots(path: &Path) -> (Settings, Settings, Settings) {
    let initial = Settings::default();
    assert_eq!(initial.schema_version, 1);
    initial
        .validate()
        .expect("initial schema-v1 settings are valid");
    assert_eq!(initial.cleanup.mode, CleanupMode::Clean);
    assert!(!initial.startup.run_at_login);

    save_settings_atomic(path, &initial).expect("write initial schema-v1 settings");
    assert_valid_file(path, &initial);

    let actor_one_snapshot = load_required(path);
    let actor_two_snapshot = load_required(path);
    assert_eq!(actor_one_snapshot, initial, "actor 1 loaded A");
    assert_eq!(
        actor_two_snapshot, initial,
        "actor 2 loaded A independently"
    );

    let mut actor_one_update = actor_one_snapshot.clone();
    actor_one_update.cleanup.mode = CleanupMode::Verbatim;

    let mut actor_two_update = actor_two_snapshot.clone();
    actor_two_update.startup.run_at_login = true;

    actor_one_update
        .validate()
        .expect("actor 1 complete snapshot is valid");
    actor_two_update
        .validate()
        .expect("actor 2 complete snapshot is valid");

    assert_ne!(actor_one_update, initial);
    assert_ne!(actor_two_update, initial);
    assert_eq!(actor_one_update.startup, initial.startup);
    assert_eq!(actor_two_update.cleanup, initial.cleanup);
    assert_eq!(actor_one_update.schema_version, 1);
    assert_eq!(actor_two_update.schema_version, 1);

    (initial, actor_one_update, actor_two_update)
}

fn assert_valid_file(path: &Path, expected: &Settings) -> Settings {
    let loaded = load_required(path);
    assert_eq!(
        &loaded, expected,
        "public loader returns the complete snapshot"
    );
    assert_eq!(loaded.schema_version, 1);
    loaded
        .validate()
        .expect("loaded schema-v1 settings validate");

    let raw = fs::read_to_string(path).expect("read complete settings document");
    let decoded = Settings::from_json(&raw).expect("complete document decodes as schema v1");
    assert_eq!(decoded, loaded, "raw document has no partial serialization");

    let mode = fs::metadata(path)
        .expect("inspect settings permissions")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600, "settings file has owner-only permissions");

    let entries = fs::read_dir(path.parent().expect("settings parent exists"))
        .expect("inspect isolated settings directory")
        .map(|entry| entry.expect("read settings directory entry").file_name())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 1, "writer left no temporary artifacts");
    assert_eq!(
        entries[0],
        path.file_name().expect("settings filename exists")
    );

    loaded
}

fn run_stale_snapshot_scenario(order: WriteOrder, label: &str) -> StaleScenarioResult {
    let directory = TestDirectory::new(label);
    let path = directory.settings_path();
    let (initial, actor_one_update, actor_two_update) = prepare_two_snapshots(&path);

    match order {
        WriteOrder::ActorOneFirst => {
            save_settings_atomic(&path, &actor_one_update)
                .expect("actor 1 writes its complete snapshot");
            assert_valid_file(&path, &actor_one_update);

            // The current public writer has no stale-snapshot rejection contract: it accepts
            // complete Settings values and atomically replaces the destination with them.
            save_settings_atomic(&path, &actor_two_update)
                .expect("audited writer accepts actor 2's stale complete snapshot");
            let final_settings = assert_valid_file(&path, &actor_two_update);

            assert_eq!(final_settings.cleanup.mode, initial.cleanup.mode);
            assert!(final_settings.startup.run_at_login);
            assert_ne!(
                final_settings.cleanup.mode, actor_one_update.cleanup.mode,
                "actor 1's changed X value is no longer represented"
            );
            assert_eq!(
                final_settings.startup.run_at_login,
                actor_two_update.startup.run_at_login
            );

            StaleScenarioResult {
                stale_write_succeeded: true,
                actor_one_change_visible: false,
                actor_two_change_visible: true,
                actor_one_change_lost: true,
                actor_two_change_lost: false,
            }
        }
        WriteOrder::ActorTwoFirst => {
            save_settings_atomic(&path, &actor_two_update)
                .expect("actor 2 writes its complete snapshot");
            assert_valid_file(&path, &actor_two_update);

            save_settings_atomic(&path, &actor_one_update)
                .expect("audited writer accepts actor 1's stale complete snapshot");
            let final_settings = assert_valid_file(&path, &actor_one_update);

            assert_eq!(final_settings.cleanup.mode, CleanupMode::Verbatim);
            assert!(!final_settings.startup.run_at_login);
            assert_eq!(final_settings.cleanup.mode, actor_one_update.cleanup.mode);
            assert_ne!(
                final_settings.startup.run_at_login, actor_two_update.startup.run_at_login,
                "actor 2's changed Y value is no longer represented"
            );

            StaleScenarioResult {
                stale_write_succeeded: true,
                actor_one_change_visible: true,
                actor_two_change_visible: false,
                actor_one_change_lost: false,
                actor_two_change_lost: true,
            }
        }
    }
}

fn run_fresh_read_sequence(label: &str) {
    let directory = TestDirectory::new(label);
    let path = directory.settings_path();
    let initial = Settings::default();
    save_settings_atomic(&path, &initial).expect("write initial schema-v1 settings");
    assert_valid_file(&path, &initial);

    let mut actor_one_update = load_required(&path);
    assert_eq!(actor_one_update, initial);
    actor_one_update.cleanup.mode = CleanupMode::Verbatim;
    actor_one_update
        .validate()
        .expect("actor 1's complete snapshot is valid");
    save_settings_atomic(&path, &actor_one_update).expect("actor 1 writes field X update");
    assert_valid_file(&path, &actor_one_update);

    let mut actor_two_fresh_snapshot = load_required(&path);
    assert_eq!(actor_two_fresh_snapshot, actor_one_update);
    actor_two_fresh_snapshot.startup.run_at_login = true;
    actor_two_fresh_snapshot
        .validate()
        .expect("actor 2's refreshed complete snapshot is valid");
    save_settings_atomic(&path, &actor_two_fresh_snapshot)
        .expect("actor 2 writes after loading current settings");

    let final_settings = assert_valid_file(&path, &actor_two_fresh_snapshot);
    assert_eq!(final_settings.cleanup.mode, CleanupMode::Verbatim);
    assert!(final_settings.startup.run_at_login);
    assert_ne!(final_settings.cleanup.mode, initial.cleanup.mode);
    assert_ne!(
        final_settings.startup.run_at_login,
        initial.startup.run_at_login
    );
}

#[test]
fn stale_snapshots_actor_one_writes_first_and_loses_x() {
    let result = run_stale_snapshot_scenario(WriteOrder::ActorOneFirst, "actor-one-first");
    assert!(result.stale_write_succeeded);
    assert!(!result.actor_one_change_visible);
    assert!(result.actor_two_change_visible);
    assert!(result.actor_one_change_lost);
    assert!(!result.actor_two_change_lost);
}

#[test]
fn stale_snapshots_actor_two_writes_first_and_loses_y() {
    let result = run_stale_snapshot_scenario(WriteOrder::ActorTwoFirst, "actor-two-first");
    assert!(result.stale_write_succeeded);
    assert!(result.actor_one_change_visible);
    assert!(!result.actor_two_change_visible);
    assert!(!result.actor_one_change_lost);
    assert!(result.actor_two_change_lost);
}

#[test]
fn fresh_read_before_actor_two_write_preserves_both_changes() {
    run_fresh_read_sequence("fresh-read");
}

#[test]
fn lost_update_audit_summary_is_assertion_backed_and_content_free() {
    let actor_one_first =
        run_stale_snapshot_scenario(WriteOrder::ActorOneFirst, "summary-one-first");
    let actor_two_first =
        run_stale_snapshot_scenario(WriteOrder::ActorTwoFirst, "summary-two-first");
    run_fresh_read_sequence("summary-fresh-read");

    assert!(actor_one_first.stale_write_succeeded);
    assert!(actor_one_first.actor_one_change_lost);
    assert!(actor_one_first.actor_two_change_visible);
    assert!(actor_two_first.stale_write_succeeded);
    assert!(actor_two_first.actor_two_change_lost);
    assert!(actor_two_first.actor_one_change_visible);

    println!("Outflow settings lost-update audit");
    println!("Initial configuration: VALID");
    println!("Actor 1 loaded original snapshot: VERIFIED");
    println!("Actor 2 loaded original snapshot: VERIFIED");
    println!("Actor 1 changed field X: VERIFIED");
    println!("Actor 2 changed field Y: VERIFIED");
    println!("Actor 1 write: SUCCEEDED");
    println!("Actor 2 stale write: SUCCEEDED");
    println!("Final settings document: VALID");
    println!("Actor 2 change: VISIBLE");
    println!("Actor 1 change: OVERWRITTEN");
    println!("Reverse-order scenario: VERIFIED");
    println!("Fresh-read sequence: BOTH CHANGES PRESERVED");
    println!("Temporary artifacts: NONE");
    println!("\nLost-update prevention: NOT IMPLEMENTED");
    println!("Actual overlapping writes tested: NO");
    println!("Automatic merge: NO");
    println!("Production locking added: NO");
    println!("Real user settings accessed: NO");
}
