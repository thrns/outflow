# Task 59 — Atomic settings replacement-operation failure recovery

```yaml
task_id: "Task 59 request; repository T-059 is unrelated"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-11]
acceptance: [AC-13]
acceptance_scope: "Injected replacement-operation error handling only; AC-13 remains incomplete"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; temporary test directory under /var/folders on the APFS local journaled data volume; Rust stable 1.99.1"
implementation: "Added one focused Unix storage test and a private replacement-operation callback seam; retained schema v1 and the public settings API"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 55–59 recovery notes and command/output", "Task 55 and Task 56 recovery matrices", "Task 58 coverage limits", "this evidence report"]
risks: "The callback returns before fs::rename, so actual OS rename failure, platform replacement atomicity, post-replacement rollback, and crash durability remain untested"
review: PENDING
```

## Scope and writer sequence

This is Task 59 from the current request, separate from repository T-059. Related project tasks T-017, T-018, T-038, T-039, T-053, and acceptance criterion AC-08 remain incomplete. The repository task ledger was not changed.

The writer validates and serializes schema-v1 settings, creates an exclusive same-directory temporary file with mode `0600`, writes and flushes the complete JSON, synchronizes the temporary file, rechecks the destination and parent, calls the existing `before_replace` hook, performs destination replacement, disarms temporary-file cleanup, and synchronizes the parent directory only after replacement succeeds. On any error before successful replacement, the temporary-file guard removes the prepared file.

Task 55's existing `before_replace` hook fires before the replacement operation and remains unchanged. Since it could not exercise an error returned by the replacement operation, this task added a narrow private `replace_destination(source, destination)` callback at the current `fs::rename` site. Production supplies a closure calling `fs::rename`; a test-only helper supplies the injected error callback. No public API changed.

## Focused replacement-failure test

Test: `storage::tests::atomic_writer_replacement_failure_preserves_valid_settings_and_allows_retry`.

The test creates an isolated directory under `std::env::temp_dir()` and writes synthetic schema-v1 settings through the public writer. It reloads and validates the original document, snapshots its exact bytes and Unix permission bits, and records that the directory contains only the destination. It prepares a different valid settings value with synthetic vocabulary.

The write callback writes and flushes the complete replacement, then reads the temporary file back and verifies byte equality with the expected serialization and successful schema-v1 decoding. The writer's synchronization callback calls the real `File::sync_all()` and returns successfully before replacement. At the destination replacement callback, the test asserts it was called once with the exact recorded temporary source and existing destination paths, verifies the temporary file still has the complete expected bytes and decodes to the replacement settings, and confirms the original destination still exists with its original bytes. The callback returns a deterministic `io::ErrorKind::Other` in place of `fs::rename`.

The writer returns `SettingsStorageError::ReplacementFailed`, not a validation, serialization, creation, write, or synchronization error. Its formatted `Display` and `Debug` output do not contain replacement settings content. The test asserts that the parent-directory sync callback was not reached, the original destination is byte-for-byte unchanged, schema-v1 decoding and public reload return the original settings and ordered vocabulary, and permissions are unchanged. Directory inspection confirms the failed temporary path is gone, no backup or unrelated entry was removed, and only the original destination remains. A subsequent ordinary `save_settings_atomic` call succeeds and reloads the new settings as schema v1 with no temporary artifacts.

The successful `sync_all()` call establishes that the synchronization operation returned before the callback; it does **not** prove crash or power-loss durability. The test uses synthetic data only and never discovers or accesses the user's configuration directory.

Observed output from the focused test:

```text
Outflow atomic settings replacement failure test

Initial settings write: PASS
Original settings reload: PASS
Temporary replacement fully written: VERIFIED
Temporary-file synchronization: COMPLETED
Replacement callback invoked: VERIFIED
Replacement-operation error: INJECTED
Writer returned replacement error: PASS
Original destination bytes: PRESERVED
Original settings reload after failure: PASS
Original permissions: PRESERVED
Temporary artifact cleanup: PASS
Subsequent public write: PASS
New settings reload: PASS

Schema migration performed: NO
Real user settings accessed: NO
Actual OS rename failure tested: NO
Post-replacement rollback verified: NO
Crash durability verified: NO
```

## Recovery coverage matrix

| Scenario | Result |
|---|---|
| Temporary-file creation failure | Task 57 — injected at the creation callback |
| Partial temporary-file write failure | Task 56 — injected at the write callback |
| Temporary-file synchronization failure | Task 58 — injected before replacement; callback verifies complete contents |
| Failure before replacement | Task 55 — injected by the pre-replacement hook |
| Replacement-operation failure | Task 59 — injected by the callback at the replacement operation; `fs::rename` is not called |
| Actual OS-level rename failure | **NOT TESTED** |
| Parent-directory synchronization failure after successful replacement | **PASS (Task 60 injected callback)** — Task 60 calls the real rename first, verifies the new destination is already visible in the parent-sync callback, then injects `ReplacementDurabilityUncertain`; actual OS sync failure and crash durability remain untested |
| Post-replacement rollback | **NOT IMPLEMENTED OR VERIFIED** |
| Actual disk-full behavior | **NOT TESTED** |
| Crash or power-loss recovery | **NOT TESTED** |
| Windows ACL and replacement semantics | **NOT TESTED** |
| Linux filesystem behavior | **NOT TESTED** |

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; test temporary directories were on the APFS local journaled data volume mounted at `/System/Volumes/Data`; Rust stable 1.99.1. No Windows or Linux run was performed.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_replacement_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — one focused test; emitted the output above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_pre_replacement_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 55 test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_temporary_write_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 56 test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_temporary_creation_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 57 test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_temporary_sync_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 58 test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 207 unit/integration tests and 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 300 unit/integration tests and 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — TypeScript check and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS**. |

This task adds 1 Rust unit test to the 299 unit/integration-test baseline, for 300 unit/integration tests total; the 1 doc test is reported separately.

## Files and remaining requirements

Task 59 changes:

- `crates/settings/src/storage.rs` — private replacement-operation seam and one focused Unix test; Task 55–58 tests and the public settings API remain unchanged.
- `README.md` — Task 59 purpose, difference from Task 55, run command, expected output, and limits; clarified that prior tasks do not test actual OS-level rename failure.
- `evidence/rust-settings-writer-recovery-task-55.md` — corrected replacement-operation coverage matrix.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — added Task 59 and separate actual OS rename, parent sync, rollback, crash, and platform gaps.
- `evidence/rust-settings-sync-failure-recovery-task-58.md` — clarified Task 59 coverage without overstating actual OS rename testing.
- `evidence/rust-settings-replacement-failure-recovery-task-59.md` — this report.

No settings schema, migration, vocabulary CRUD, dependency, Cargo manifest, or lockfile change was made. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete; repository T-059 was not changed.

**Actual OS rename failure and post-replacement rollback were not tested.** Task 59 does not establish platform-specific replacement atomicity, handling of ambiguous syscall outcomes, or crash durability. Remaining T-018 requirements include actual filesystem failure evidence, parent-directory synchronization failure behavior, persistent last-known-good recovery, approved Windows owner-only ACL and replacement support, Linux filesystem behavior, and real disk-full and crash-recovery testing.
