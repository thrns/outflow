# Task 60 — Parent-directory synchronization failure after replacement

```yaml
task_id: "Task 60 request; repository T-060 completion marker not updated"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-11]
acceptance: [AC-12, AC-13]
acceptance_scope: "Injected parent-directory synchronization error after real replacement; release acceptance remains incomplete"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; Rust stable 1.99.1; test temp directory under TMPDIR on APFS local journaled /System/Volumes/Data"
implementation: "Added one Unix-only storage test using the existing private synchronization callback; production settings writer API and replacement code are unchanged"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 60 behavior, command, output and limits", "evidence/rust-settings-temporary-write-recovery-task-56.md: recovery matrix", "evidence/rust-settings-replacement-failure-recovery-task-59.md: recovery matrix", "this evidence report"]
risks: "Injected callback failure is not an OS-level directory sync failure and does not establish crash durability, rollback, or Windows/Linux behavior"
review: PENDING
```

## Writer path and fault boundary

The inspected Unix writer sequence is: validate and serialize settings; open and validate the parent directory and destination; create a same-directory temporary file with mode `0600`; write and flush the complete schema-v1 JSON; call `File::sync_all` on that temporary file; recheck destination and parent identity; run the pre-replacement hook; call `fs::rename` through the normal replacement operation; disarm temporary-file cleanup; then invoke the parent-directory synchronization callback with the already-open parent directory handle. The public writer supplies `File::sync_all` for this final callback.

The test uses the existing private `save_settings_atomic_with_hooks` callback pattern. It keeps real temporary-file synchronization and the production `fs::rename` replacement path. Only the final parent-directory synchronization callback is substituted with a deterministic `io::Error`. No public API, manifest, lockfile, schema, filesystem abstraction, or global fault state changed.

## Observed result

Test: `storage::tests::atomic_writer_parent_directory_sync_failure_preserves_replaced_settings_and_allows_retry`.

The test creates an isolated temporary directory and writes synthetic valid schema-v1 settings through the public writer. It reloads those settings, captures their exact bytes and mode (`0600`), and prepares different valid settings with two ordered synthetic vocabulary entries. The write callback verifies the complete replacement serialization on the temporary path. The writer then calls real temporary-file `sync_all()`.

Before returning the injected error, the parent-directory callback is reached once. Inside it, the test reads the destination and confirms its bytes equal the complete replacement serialization and differ from the initial bytes; it decodes the destination as schema v1 and confirms the replacement settings and vocabulary order; and it confirms the original temporary path has been consumed. This callback is after the writer's real `fs::rename`, so replacement completion is established by the observed on-disk state rather than inferred from the later error.

The writer returns `SettingsStorageError::ReplacementDurabilityUncertain`. Its display text says directory durability could not be confirmed, does not identify temporary-file synchronization as the failure, and contains no settings or vocabulary content. After the error, the destination remains a regular file whose exact bytes decode to the replacement settings. Its mode remains `0600`; there are no temporary, backup, or other unexpected entries. A third, distinct value then saves successfully through the public writer and reloads successfully. No second write attempts to restore the original settings.

**The injected directory synchronization error did not roll back the completed replacement. The test does not establish power-loss durability or real OS-level synchronization failure behavior.** The previous settings were no longer the visible destination contents after the injected error; this does not establish whether those bytes exist in any filesystem or operating-system backup.

Focused test output:

```text
Outflow atomic settings parent-directory sync failure test

Initial settings write: PASS
Original settings reload: PASS
Temporary replacement fully written: VERIFIED
Temporary-file synchronization: COMPLETED
Destination replacement: COMPLETED
Parent-directory sync callback invoked: VERIFIED
Parent-directory sync error: INJECTED
Writer returned directory sync error: PASS
New destination bytes visible: VERIFIED
Replacement settings reload: PASS
File permissions: VERIFIED
Unexpected temporary artifacts: NONE
Subsequent public write: PASS
Third settings value reload: PASS

Old configuration rolled back: NO
Post-replacement visibility: VERIFIED
Crash durability: NOT VERIFIED
Actual OS-level directory sync failure: NOT TESTED
Real user settings accessed: NO
Schema migration performed: NO
```

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; Rust `1.99.1 (58cf8f9c3 2026-10-13)`. The test helper creates directories under `std::env::temp_dir()`; `TMPDIR` resolves under `/var/folders/.../T`, on `/System/Volumes/Data` mounted as `apfs, local, journaled`. Only isolated temporary directories and synthetic settings were used. No user settings path was discovered or accessed.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings parent_directory_sync --locked -- --nocapture` | **PASS** — 1 focused Task 60 test; all printed assertions passed. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_pre_replacement_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 55. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_temporary_write_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 56. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_temporary_creation_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 57. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_temporary_sync_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 58. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_replacement_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 59. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 208 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS** after formatting the new test. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 301 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS**. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS**. |

The workspace run adds one Rust unit test to the prior 300 unit/integration-test baseline. The one existing Rust doc test is reported separately. No blocked Moonshine CMake configuration was attempted.

## Coverage matrix and remaining requirements

| Scenario | Coverage |
|---|---|
| Temporary-file creation failure | Task 57 — injected |
| Partial temporary-file write failure | Task 56 — injected |
| Temporary-file synchronization failure | Task 58 — injected before replacement |
| Pre-replacement failure | Task 55 — injected; prior destination remains intact |
| Replacement-operation failure | Task 59 — injected instead of calling `fs::rename` |
| Parent-directory synchronization after successful replacement | Task 60 — injected callback after real `fs::rename`; new destination observed and reloaded |
| Actual OS-level directory synchronization failure | **NOT TESTED** |
| Actual OS rename failure | **NOT TESTED** |
| Actual disk-full behavior | **NOT TESTED** |
| Post-replacement rollback | **NOT IMPLEMENTED OR VERIFIED** |
| Crash or power-loss recovery | **NOT TESTED** |
| Windows ACL and replacement behavior | **NOT TESTED** |
| Linux filesystem behavior | **NOT TESTED** |

Pre-replacement failures in Tasks 55–58 preserve the previously valid destination. Task 60 fails at a post-replacement synchronization boundary: the new destination is visible, while directory-entry crash durability remains uncertain. The injected error does not prove that the previous configuration survived and does not trigger automatic rollback.

T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. No repository task completion marker, including T-060, was changed. Actual disk-full behavior, OS-level sync/rename failures, rollback, crash/power-loss recovery, Linux behavior, and Windows ACL/replacement remain open. No schema-v2 loading or migration, vocabulary CRUD, dependency, manifest, or lockfile change was introduced.
