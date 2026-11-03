# Task 66 — Confirmed reset after replacement with uncertain durability

```yaml
task_id: "Task 66 request; repository T-066 is unrelated and remains unchecked"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-10, UF-11]
acceptance: [AC-13]
acceptance_scope: "One injected confirmed-reset parent-directory synchronization failure after real replacement; no release or general recovery acceptance"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; Rust stable 1.99.1; test temporary directory on the APFS local journaled data volume"
implementation: "Added one Unix-only reset-path regression test using the existing private parent-directory synchronization callback; production code and public API unchanged"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 66 failure scenario, focused command, output, and limits", "evidence/rust-settings-temporary-write-recovery-task-56.md: Task 66 matrix row and Task 65/66 comparison", "this report"]
risks: "The callback substitutes a deterministic error for parent sync; it is not an OS-level sync failure and does not establish crash durability, rollback, human confirmation, or Windows/Linux behavior"
review: PENDING
```

## Scope and inspected implementation

The repository root is `/Users/bixxie/Desktop/Projects/outflow`, branch `codex/task23-moonshine-build-scope`, at base commit `8c978bbd53617b7291fd0aeacf7c42cc260a0142`. The worktree already contained uncommitted settings, README, and Task 55–65 evidence changes; those were preserved. The root `AGENTS.md` applies and no nested `AGENTS.md` or `AGENTS.override.md` exists. No task completion marker, branch, commit, or Git history was changed.

Inspected `docs/PRD.md` (F-12/F-13, FR-13.4, AC-08/AC-13), `docs/ARCHITECTURE.md` (C-08 and §7), `docs/TECH_STACK.md` (TS-08), `docs/USER_FLOWS.md` (UF-10/UF-11), `docs/TASKS.md` (T-017/T-018 and the unrelated repository T-066), `crates/settings/src/lib.rs`, `crates/settings/src/storage.rs`, the Task 60 and Task 65 evidence, and the Task 60/61/64/65 regression tests.

The public `reset_settings_to_defaults(path, confirmed)` delegates to private `reset_settings_to_defaults_with_hooks`. `confirmed=false` returns `ResetNotConfirmed` before the writer; Task 64's existing test was rerun and verified no mutation. A confirmed reset constructs and validates `Settings::default()`, then delegates to `save_settings_atomic_with_destination_policy` with `require_existing_destination=true`. The Unix writer writes and synchronizes the same-directory temporary file, rechecks its paths, calls the pre-replacement hook, uses the ordinary `fs::rename` replacement operation, disarms temp cleanup, and only then invokes the parent-directory synchronization callback. A callback error maps to `ReplacementDurabilityUncertain` after the new destination is already visible.

The new test calls the existing private reset hook. It supplies real `File::sync_all` for temporary-file synchronization, a no-op pre-replacement callback, and an injected error only in the existing parent-sync callback. It does not override replacement. No production code, public API, dependency, manifest, lockfile, filesystem abstraction, environment control, or global state changed.

## Regression test and observed result

Added `storage::tests::confirmed_reset_directory_sync_failure_leaves_defaults_visible` in `crates/settings/src/storage.rs`. It creates a unique `TestDirectory` under `std::env::temp_dir()`, synthetic schema-v1 settings with three vocabulary mappings and non-default cleanup/startup preferences, and an unrelated sentinel. It writes and reloads the initial settings through the public writer and loader, then captures the exact settings bytes, Unix mode, sentinel bytes, and directory entries.

The reset is called with `confirmed=true`. The test verifies that the temporary file contains valid schema-v1 defaults and that the temporary-file sync operation succeeds. The pre-replacement hook confirms the original destination bytes remain intact at that boundary and returns successfully. At the parent-sync callback, the test verifies the expected settings path exists as a regular file; its bytes differ from the original, decode as schema v1, equal the documented defaults, contain no vocabulary, and have reset preferences. It also confirms that the temp path has been consumed by the real replacement before returning the injected I/O error. The callback is reached exactly once.

The reset returns `SettingsStorageError::ReplacementDurabilityUncertain`, not `ReplacementFailed` or `ResetNotConfirmed`, and returns `Err` rather than success. Its display/debug forms disclose neither the settings JSON nor synthetic vocabulary. Immediately after the error, the destination bytes equal the bytes observed inside the callback and differ from the original; `load_settings` returns schema-v1 defaults with empty vocabulary and default preferences. File mode is `0600`, the sentinel is byte-for-byte unchanged, and the directory contains only the settings file and sentinel with no temp or backup artifacts. The test does not restore the original bytes.

Only after these post-error assertions, the test makes a separate public `save_settings_atomic` call with a third schema-v1 value distinct from the original and defaults. The write and reload succeed, the sentinel remains unchanged, and no temporary artifact remains. This is a deliberate later operation, not a retry or rollback performed by reset.

Focused test output:

```text
Outflow confirmed reset durability uncertainty test

Initial settings write: PASS
Original vocabulary persisted: VERIFIED
Confirmed reset requested: VERIFIED
Temporary replacement synchronized: VERIFIED
Real destination replacement: COMPLETED
Parent-directory sync callback: VERIFIED
Directory synchronization error: INJECTED
Reset returned ReplacementDurabilityUncertain: PASS

Default settings visible after error: VERIFIED
Original vocabulary no longer visible: VERIFIED
Non-default preferences reset: VERIFIED
Settings reload after error: PASS
File permissions: VERIFIED
Unrelated sentinel: PRESERVED
Unexpected temporary artifacts: NONE

Subsequent public write: PASS
Third settings value reload: PASS

Automatic retry: NO
Automatic rollback: NO
Crash durability verified: NO
Actual OS-level sync failure tested: NO
Human confirmation UI tested: NO
Real user settings accessed: NO
Schema migration performed: NO
```

## Reset failure comparison

| Scenario | Observed result |
|---|---|
| Task 65: confirmed reset fails before replacement | Original settings and vocabulary remain visible in the injected pre-replacement scenario. |
| Task 66: confirmed reset fails during directory sync after replacement | Default settings are already visible and reloadable; crash durability remains uncertain in the injected post-replacement scenario. |

These are observations for the specified injected failures, not universal guarantees for arbitrary operating-system failures. The Task 66 error means durability could not be confirmed, not that the reset rolled back. Original preferences and vocabulary must not be assumed to survive. No automatic retry or post-replacement rollback exists.

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; Rust stable `1.99.1`; Node `v22.23.3`; Python `3.13.13`. The test helper uses `std::env::temp_dir()` under `TMPDIR` on `/System/Volumes/Data`, mounted as APFS local and journaled. Only synthetic values in isolated temporary directories were used. No user configuration directory was found or accessed.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings confirmed_reset_directory_sync_failure --locked -- --nocapture` | **PASS** — new reset-level test, 1 passed; output shown above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_parent_directory_sync_failure_preserves_replaced_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 60. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings settings_write_error_outcome_contract --locked -- --nocapture` | **PASS** — Task 61. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings explicit_settings_reset_is_scoped_and_separate_from_loading --locked -- --nocapture` | **PASS** — Task 64, including `confirmed=false` non-mutation. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings confirmed_reset_pre_replacement_failure_preserves_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 65. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 214 unit/integration tests and 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 307 unit/integration tests and 1 doc test; baseline 306 unit/integration tests plus 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — TypeScript check and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** — checked after documentation and test changes. |

The blocked Moonshine CMake configuration was not attempted.

## Limitations and changed files

- `crates/settings/src/storage.rs` — one new Unix-only reset-level regression test; existing Task 55–65 tests remain intact.
- `README.md` — Task 66 scenario, distinction from Task 65, command, expected output, and limitations.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — Task 66 recovery-matrix row and explicit Task 65/66 comparison.
- `evidence/rust-settings-reset-durability-uncertain-task-66.md` — this evidence report.

One Rust unit test was added: the prior workspace baseline of 306 unit/integration tests plus 1 doc test is now 307 plus 1. The settings crate now has 214 unit/integration tests plus 1 doc test. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. Repository T-066 was not changed.

Actual OS-level parent-directory synchronization failure, power-loss/crash recovery, post-replacement rollback, Windows behavior, Linux filesystem behavior, and human confirmation UI remain untested or unimplemented. No schema-v2 migration was performed. The injected callback is not evidence of crash durability.
