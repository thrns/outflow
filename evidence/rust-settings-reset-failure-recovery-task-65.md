# Task 65 — Confirmed settings reset pre-replacement failure and retry

```yaml
task_id: "Task 65 request; repository T-065 is separate and remains unchecked"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-10, UF-11]
acceptance: [AC-12, AC-13]
acceptance_scope: "One deterministic reset pre-replacement failure and a new explicit retry; no UI, migration, release, or general recovery acceptance"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0, build 25G83), arm64; Rust stable 1.99.1; temporary test directory on the local APFS data volume"
implementation: "Added one Unix storage unit test using the existing private reset/writer pre-replacement hook; updated README and the settings recovery matrix"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 65 failure/retry limits and focused command", "evidence/rust-settings-temporary-write-recovery-task-56.md: Task 65 matrix row", "this report"]
risks: "Injected callback only; no real OS-level disk or rename failure, post-replacement rollback, crash durability, Windows/Linux filesystem, or human confirmation UI was tested"
review: PENDING
```

## Scope and inspected implementation

The repository root is `/Users/bixxie/Desktop/Projects/outflow`, branch `codex/task23-moonshine-build-scope`, at base commit `8c978bbd53617b7291fd0aeacf7c42cc260a0142`. The worktree already contained uncommitted Task 55–64 changes before this task. Those changes were preserved; no branch, worktree, task marker, commit, or Git history was changed. The root `AGENTS.md` applies; no nested `AGENTS.md` or `AGENTS.override.md` was found.

Inspected the relevant requirements and existing implementation in `docs/PRD.md` (F-12, F-13, FR-13.4, AC-08, AC-13), `docs/ARCHITECTURE.md` (C-08), `docs/USER_FLOWS.md` (UF-10, UF-11), `docs/TECH_STACK.md` (TS-08), `docs/TASKS.md` (T-017, T-018), `crates/settings/src/lib.rs`, `crates/settings/src/storage.rs`, Task 64 evidence, and the Task 55–61 writer-failure tests and recovery notes.

The public `reset_settings_to_defaults(path, confirmed)` delegates to private `reset_settings_to_defaults_with_hooks`. `confirmed=false` returns `ResetNotConfirmed` before destination inspection or writer entry; Task 64's test was rerun and still verifies no mutation for that boundary. A confirmed call first inspects the selected destination and validates compiled defaults, then delegates to `save_settings_atomic_with_destination_policy` with `require_existing_destination=true`. The same existing writer creates, writes, flushes, and synchronizes the same-directory temporary file, rechecks the destination and parent, invokes `before_replace`, performs `fs::rename`, and syncs the parent. The reset itself has no replacement path outside that writer.

The existing private `before_replace` hook is reused; no second hook, public fault parameter, global state, environment switch, dependency, manifest, or lockfile change was added. The focused test verifies that this hook sees the prepared temporary file containing exactly schema-v1 defaults before returning a deterministic I/O error. The writer maps that injected pre-replacement error to `SettingsStorageError::ReplacementFailed`. The current public error variant does not retain the underlying I/O error as a source, so the test preserves and asserts the typed variant supported by the existing API; it does not suppress a richer source that the API exposes.

## Test behavior

Added `storage::tests::confirmed_reset_pre_replacement_failure_preserves_settings_and_allows_retry` in `crates/settings/src/storage.rs`. It uses a unique `TestDirectory` under `std::env::temp_dir()`, synthetic valid schema-v1 settings, and an unrelated sentinel. No user configuration directory is discovered or accessed.

The test first writes and reloads two ordered synthetic vocabulary entries and non-default cleanup/startup preferences, then captures the exact settings bytes, Unix permission mode, sentinel bytes, and directory entries. It requests reset with `confirmed=true`. At the reused pre-replacement hook, it verifies the temp file contains the compiled defaults JSON and the destination still contains the original bytes, then injects a controlled error.

Assertions verify the returned error is `ReplacementFailed`; the destination remains a regular file whose bytes exactly match the snapshot; settings reload as the original values with vocabulary order and preferences intact; defaults were not committed; the original permission mode remains unchanged; the temporary file is removed; the directory contains only its original settings and sentinel files; and sentinel bytes are unchanged. The test does not manually restore the settings file.

Only after preservation checks, the test calls the public API again with `confirmed=true`. That new call succeeds, and a reload verifies schema-v1 defaults, empty vocabulary, and reset preferences. The stored defaults JSON is checked, replacement permissions remain `0600`, temporary artifacts are absent, and the sentinel remains unchanged.

Observed focused output:

```text
Outflow confirmed settings reset failure test

Initial settings write: PASS
Initial vocabulary persisted: VERIFIED
Confirmed reset requested: VERIFIED
Pre-replacement failure: INJECTED
Reset returned storage error: PASS
Original destination bytes: PRESERVED
Original vocabulary: PRESERVED
Original preferences: PRESERVED
Temporary artifact cleanup: PASS
Unrelated sentinel: PRESERVED

Explicit retry requested: VERIFIED
Retry reset succeeded: PASS
Schema-v1 defaults persisted: PASS
Previous vocabulary cleared: VERIFIED
Unrelated sentinel after retry: PRESERVED

Automatic retry: NO
Automatic rollback: NO
Real user settings accessed: NO
Human confirmation UI tested: NO
Actual OS-level failure reproduced: NO
```

## Failure boundary and remaining gaps

**Pre-replacement reset failure:** the original configuration was preserved in this tested injected scenario. No automatic retry occurred; recovery was a new explicit caller invocation.

**Post-replacement durability failure:** not tested by Task 65. The existing `ReplacementDurabilityUncertain` path can return an error after defaults are already visible when parent-directory synchronization fails. Task 61's separate writer test covers that distinction. Task 65 does not assert that every reset error is non-destructive, implement rollback, or establish crash or power-loss durability.

The callback test does not reproduce OS-level disk-full, filesystem, or rename failures. The run covers macOS 26.6.2 arm64 with the temporary directory on the local APFS data volume only. Windows ACL/replacement and Linux filesystem behavior remain unverified. The boolean is caller intent only; no human confirmation UI, Tauri command, accessibility behavior, reset recovery UI, backup restoration, or schema migration exists. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. Repository T-065 was not modified.

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; Rust stable 1.99.1; Node v22.23.3; Python 3.13 environment. Test temporary paths use `std::env::temp_dir()` on the APFS local data volume. Workspace members are `outflow-core` and `outflow-settings`; the blocked Moonshine CMake configuration was not attempted.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings confirmed_reset_pre_replacement_failure --locked -- --nocapture` | **PASS** — new reset failure/retry test, 1 passed. The first compile attempt lacked the test-module import for the public retry call; the import was added and this command then passed. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings explicit_settings_reset_is_scoped_and_separate_from_loading --locked -- --nocapture` | **PASS** — Task 64 reset safety test, 1 passed; includes the unconfirmed reset non-mutation boundary. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_pre_replacement_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 55 generic writer pre-replacement regression, 1 passed unchanged. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 213 unit/integration tests and 1 doc test passed. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS** after formatting the new test. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 306 unit/integration tests and 1 doc test passed (baseline 305 unit/integration tests plus 1 doc test; Task 65 adds one test). |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — TypeScript check and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** — no whitespace errors in tracked changes. The new untracked Task 65 report was also checked for trailing whitespace. |

## Task 65 changes

- `crates/settings/src/storage.rs` — one focused reset failure/retry unit test and its test-only import; public API and production behavior unchanged.
- `README.md` — concise description of preservation, explicit retry, error distinction, focused command, and limitations.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — Task 65 row and pre-/post-replacement distinction in the recovery matrix.
- `evidence/rust-settings-reset-failure-recovery-task-65.md` — this evidence report.

No dependency, Cargo manifest, lockfile, schema, migration, UI, native, ASR, or task completion marker was changed. No files were staged or committed. Pre-existing uncommitted Task 55–64 changes remain preserved.
