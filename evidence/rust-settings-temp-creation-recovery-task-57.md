# Task 57 — Atomic settings temporary-file creation failure

```yaml
task_id: "Task 57 request; repository T-057 was not modified"
feature_ids: [F-12, F-13]
flows: [UF-11]
acceptance: [AC-12, AC-13]
acceptance_scope: "Temporary-file creation failure and recovery evidence only; AC-12 and AC-13 remain incomplete"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0, build 25G83), arm64; local APFS journaled data volume; Rust stable 1.99.1"
implementation: "Private test-only temporary-file creation callback and one creation-failure preservation/retry test; README and Task 56 recovery matrix updated"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 57 temporary-file creation failure recovery check", "evidence/rust-settings-temporary-write-recovery-task-56.md: recovery matrix", "this evidence report"]
risks: "Actual permission denial, disk-full, OS-level sync/rename failures, crash recovery, Linux filesystem behavior, and Windows ACL/replacement behavior remain unverified"
review: PENDING
```

## Scope and actual writer operation

This request adds a deterministic temporary-file creation failure case to the settings writer's recovery coverage. It does not modify repository T-057 or complete T-017, T-018, T-038, T-039, T-053, or AC-08. No schema migration, vocabulary CRUD, persistence feature, dependency, Cargo manifest, or lockfile was changed.

On Unix, the writer generates a same-directory `.outflow-settings-tmp-<pid>-<timestamp>-<attempt>` path and normally opens it with `OpenOptions::new().write(true).create_new(true).mode(0o600).open(path)`. The loop retries only `AlreadyExists`, up to 128 generated names. Other open errors return immediately through the existing error mapper: permission denial becomes `PermissionDenied { operation: CreateTemporaryFile }`; other create failures become `TemporaryFileCreationFailed`.

The private `#[cfg(test)] save_settings_atomic_with_creation_hook` supplies a callback at the exact point where the writer normally invokes that exclusive open. The test verifies that callback receives the generated same-directory temporary path, that it is called once, and that its error is not treated as a filename collision. The callback returns a deterministic `io::ErrorKind::Other`; it substitutes for the OS open at that operation boundary, so no actual open, permission denial, or filesystem failure is claimed. Production continues to use the ordinary `OpenOptions::create_new(true)` operation and public API.

The existing typed error is `SettingsStorageError::TemporaryFileCreationFailed`. The current error model intentionally redacts the underlying I/O detail and retains no `io::Error` source for this case. The test confirms the injected error is not misclassified as settings validation, serialization, temporary write, synchronization, or replacement failure. No public error variant or settings API changed.

## Test scenario and observed result

Test: `storage::tests::atomic_writer_temporary_creation_failure_preserves_valid_settings_and_allows_retry`.

The test creates a unique isolated directory under `std::env::temp_dir()`, writes valid schema-v1 settings through public `save_settings_atomic`, reloads them, captures their exact bytes and the directory entries, then prepares different valid synthetic settings with synthetic vocabulary. The create callback verifies the generated filename and parent, confirms the candidate path is absent, records exactly one creation attempt, and returns the injected non-collision error.

Assertions verify the writer returns `TemporaryFileCreationFailed`, the write callback and pre-replacement boundary are not reached, the candidate temporary path does not exist, the original destination remains a regular file with byte-for-byte identical contents, and the directory entries exactly match the pre-failure snapshot. Reload succeeds with schema version 1 and the original vocabulary unchanged. Since creation failed, no temporary artifact was created; this test does not claim partial-file cleanup. A subsequent call to the public writer succeeds, reloads the replacement settings, and leaves only the destination file. The test removes its isolated directory on exit.

Observed focused output:

```text
Outflow atomic settings temporary-creation failure test

Initial settings write: PASS
Original settings reload: PASS
Temporary-file creation failure: INJECTED
Writer returned error: PASS
Replacement attempted: NO
Original destination bytes: PRESERVED
Original settings reload after failure: PASS
Unexpected temporary artifacts: NONE
Subsequent normal write: PASS
New settings reload: PASS

Schema migration performed: NO
Real user settings accessed: NO
Actual OS permission failure tested: NO
Actual disk-full failure tested: NO
```

## Recovery coverage matrix

| Failure scenario | Verification |
|---|---|
| Temporary-file creation | **PASS — Task 57**; synthetic callback error at the exclusive-create operation boundary; original file preserved and normal retry succeeds. |
| Partial temporary-file write | **PASS — Task 56**; partial bytes are verified before the injected write error, then cleanup, preservation, reload, and retry are checked. |
| Immediately before replacement | **PASS — Task 55**; failure is injected after preparation and final path checks, before rename. |
| Actual temporary-directory permission denial | **OUTSTANDING**; no actual OS permission failure was induced. |
| Actual disk-full failure | **OUTSTANDING**; the filesystem was not filled. |
| File synchronization failure | **OUTSTANDING at the OS operation**; the existing callback simulation does not produce a real `sync_all` failure. |
| Destination replacement failure | **OUTSTANDING**; no actual or injected `fs::rename` failure is covered. |
| Crash or power-loss recovery | **OUTSTANDING**. |
| Windows ACL and replacement behavior | **OUTSTANDING**; Windows writer support is not implemented or tested. |
| Linux filesystem behavior | **OUTSTANDING**; verification ran only on macOS/APFS. |

The Task 55 and Task 56 tests remain intact. The matrix extension is also recorded in the [Task 56 recovery evidence](rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix). T-018 remains incomplete, including its cross-platform requirements and actual filesystem-failure evidence.

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; `/System/Volumes/Data` is APFS, local, journaled; Rust stable 1.99.1. Tests use synthetic values and isolated temporary directories only. No real user configuration directory was accessed.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings temporary_creation --locked -- --nocapture` | **PASS** — one focused test passed; asserted output shown above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings temporary_write --locked` | **PASS** — Task 56 filter, one test passed. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings pre_replacement --locked` | **PASS** — three matching Task 55/pre-replacement/reset tests passed. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 205 unit/integration tests and 1 doc test passed. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 298 unit/integration tests and 1 doc test passed. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — typecheck and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** at final inspection. |

## Remaining requirements and limits

- T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. AC-12 and AC-13 are not completed by this test. Repository T-057 was not changed.
- The injected creation error is synthetic and verifies operation-level error handling, not actual filesystem permission denial, `ENOSPC`, synchronization failure, or OS replacement failure.
- Windows ACL/replacement support and Linux filesystem behavior remain unimplemented or unverified. Crash/power-loss recovery and persistent last-known-good migration rollback remain outstanding.
- No schema-v2 loading/writing, migration, vocabulary CRUD, model or UI work, unrelated native/ASR change, dependency, or lockfile update was introduced. No Task 55 or Task 56 test was changed.

Files changed for this task: `crates/settings/src/storage.rs`, `README.md`, `evidence/rust-settings-temporary-write-recovery-task-56.md`, and this evidence report. The worktree remains on `codex/task23-moonshine-build-scope`; changes are uncommitted and unstaged.
