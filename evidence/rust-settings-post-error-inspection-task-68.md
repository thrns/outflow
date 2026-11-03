# Task 68 — Read-only settings inspection after persistence errors

```yaml
task_id: "Task 68 request; repository T-068 status not changed"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-10, UF-11]
acceptance: [AC-13]
acceptance_scope: "Test-only classification of the settings visible through the existing loader; no production recovery behavior"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; isolated temporary directories on APFS local journaled data volume; Rust stable 1.99.1"
implementation: "Added one test-only integration target with 11 tests; documented the inspector in README and the existing recovery matrix"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 68 purpose, outcomes, limitations, and command", "evidence/rust-settings-temporary-write-recovery-task-56.md: Task 68 matrix row", "this report"]
risks: "No production caller, user-facing recovery, schema migration, OS-level failure reproduction, crash durability evidence, Windows testing, or Linux filesystem testing"
review: PENDING
```

## Scope and inspected interfaces

The repository was `/Users/bixxie/Desktop/Projects/outflow`, on branch `codex/task23-moonshine-build-scope`, based at `8c978bbd53617b7291fd0aeacf7c42cc260a0142`. The initial worktree already contained uncommitted settings source, README, Task 55/56 evidence, the Task 67 test target, and Task 57–66 evidence. Those changes were preserved. The root `AGENTS.md` applies; no nested `AGENTS.md` or `AGENTS.override.md` was found under the repository. Repository task completion markers were not changed.

Inspected PRD FR-13.4 and the related F-12/F-13 acceptance sections; ARCHITECTURE C-08 and §7; USER_FLOWS UF-10/UF-11; TECH_STACK TS-08; TASKS T-017/T-018; `crates/settings/src/lib.rs`; `crates/settings/src/storage.rs`; the existing loader, initialization, and corruption-preservation tests; the Task 61, 62, 63, 66, and 67 evidence; and the existing settings recovery matrix.

The relevant public interfaces are:

- `load_settings(path: &Path) -> Result<Option<Settings>, SettingsStorageError>` reads a supplied regular file, returns `Ok(None)` when missing and `Ok(Some(settings))` for valid schema v1, and never writes or substitutes defaults.
- `save_settings_atomic(path: &Path, settings: &Settings) -> Result<(), SettingsStorageError>` is the public writer used only to establish valid synthetic test fixtures in this new target.
- `initialize_settings(path)` delegates to `load_settings` but returns in-memory defaults on `Ok(None)`, so it is not appropriate for observing whether a missing file is currently visible.
- Malformed JSON is reported as `SettingsStorageError::InvalidDocument`. Unsupported versions are reported as `SettingsStorageError::InvalidSettings(SettingsError::UnsupportedSchemaVersion { .. })`. Other typed errors include invalid UTF-8, invalid settings, file-size and path failures, permission denial, and filesystem read failure.

Task 67 remains in the separate `settings_persistence_guidance_contract` integration target. Source review found no production settings service or caller wiring the guidance helper. Task 68 neither imports that target nor duplicates its guidance logic.

## Test-only inspection contract

Created `crates/settings/tests/settings_post_error_inspection.rs`. Its private helper calls `load_settings` once per inspection and returns only this unit-only enum:

```rust
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
```

`Settings` already derives complete `PartialEq`/`Eq`. The helper compares the entire loaded value to both caller snapshots. Equal snapshots that both match produce `BothSnapshotsMatch`; the inspector cannot determine which write, if any, caused that visible state. A valid configuration matching neither snapshot is `OtherValidSettingsVisible`, not corruption. Typed malformed/content validation errors are classified as `InvalidDocument`; unsupported schema is matched first and stays separate. All other loader errors conservatively become `ReadFailure`.

The helper has no settings writer, initializer, reset, backup, repair, or retry call. It retains no settings content in its result, uses no timestamp, inode, filename, or error details to guess which state resulted from a write, and makes no durability claim. The result describes one loader snapshot; a later operation can change the file.

## Synthetic fixtures and tests

All files are in uniquely named directories under `std::env::temp_dir()`, with an unrelated synthetic sentinel. Valid fixtures are created with `save_settings_atomic`; malformed JSON and unsupported-schema JSON are written directly by test setup. No actual user configuration path or user data is used.

Original, attempted, and third valid schema-v1 settings use distinct non-default recording preferences, cleanup preferences, startup preferences, and synthetic vocabulary values. A separate pair of independently constructed but equal snapshots checks ambiguity. Unsupported input is valid JSON with `schema_version: 999`; no schema-v2 loading or migration is added.

The 11 integration tests cover original, attempted, third valid, equal snapshots, missing, malformed, unsupported, and other typed loader failures. They also cover repeated deterministic inspections, exact file-byte preservation, full temporary-directory entry and file-metadata snapshots, unrelated-file preservation, and content-free results. Unix snapshots compare permissions, device/inode identity, and modification time where available. The writer-created valid fixture is also checked for owner-only `0600` mode. These metadata comparisons establish only that the tested reads did not change those local fixtures; they do not establish atomicity or durability.

Privacy assertions verify that returned `Debug` output is only a classification and contains no vocabulary ID, spoken phrase, written phrase, JSON field, malformed raw input, or unsupported document marker. The printed summary contains only fixed labels. No content-bearing logging was added.

The test output produced after its assertions passed:

```text
Outflow post-error settings inspection

Original settings visible: VERIFIED
Attempted settings visible: VERIFIED
Other valid settings visible: VERIFIED
Identical snapshots: AMBIGUOUS
Missing settings: DETECTED
Malformed settings: REJECTED
Unsupported schema: DETECTED
Repeated inspections: NON-DESTRUCTIVE
Original file bytes: PRESERVED
Unrelated files: PRESERVED
Inspection report privacy: VERIFIED

Settings files written by inspector: NO
Automatic reset: NO
Automatic retry: NO
Automatic rollback: NO
Crash durability inferred: NO
Production caller implemented: NO
Real user settings accessed: NO
```

The relationship to Task 67 is a proposed future flow only:

```text
Settings write or reset returns an error
                  |
                  v
Optional, read-only settings inspection
                  |
                  v
Visible-state classification
                  |
                  v
Future caller guidance (Task 67 contract)
```

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; Rust stable 1.99.1; Node 22.23.3; Python 3.13.13. `std::env::temp_dir()` test directories were on `/System/Volumes/Data`, mounted as `apfs, local, journaled`. No other operating system or filesystem was tested.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_post_error_inspection --locked -- --nocapture` | **PASS** — 11 tests; printed the assertion-backed summary above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_persistence_guidance_contract --locked` | **PASS** — Task 67's 14 existing tests. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings corrupted_settings_files_are_rejected_and_preserved_without_automatic_reset --locked` | **PASS** — Task 62 loader-preservation regression. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings settings_initialization_preserves_unsupported_schema_files --locked` | **PASS** — Task 63 unsupported-schema preservation regression. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 239 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable rustfmt --edition 2024 crates/settings/tests/settings_post_error_inspection.rs` | **PASS** — formatted only the new test target. |
| `rustfmt --edition 2024 crates/settings/tests/settings_post_error_inspection.rs` | **BLOCKED initially** — rustup tried to create a temporary file outside the writable workspace; the stable-toolchain override above succeeded. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 332 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS**. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** — final working-tree diff checked after documentation updates. |

The Task 67 baseline was 321 Rust unit/integration tests plus one doc test. Task 68 adds 11 integration tests, giving 332 unit/integration tests plus one doc test. The `outflow-settings` package has 239 unit/integration tests plus one doc test. No Rust dependency, manifest, or lockfile changed. The blocked Moonshine CMake configuration was not attempted.

## Changed files and remaining gaps

- `crates/settings/tests/settings_post_error_inspection.rs` — test-only helper, isolated fixtures, metadata/content-preservation checks, privacy assertions, and 11 tests.
- `README.md` — concise purpose, outcomes, ambiguity, loader rationale, limitations, test command, and Task 67 relationship.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — brief Task 68 coverage-matrix row.
- `evidence/rust-settings-post-error-inspection-task-68.md` — this evidence report.

No production settings API, schema, dependency, manifest, lockfile, Tauri command, frontend UI, automatic persistence operation, or task completion marker changed. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. No production inspector or recovery service exists. Actual OS rename/synchronization failures, crash or power-loss durability, Windows filesystem behavior, and Linux filesystem behavior remain untested. The inspector does not recommend actions or establish what operation caused the currently visible file.
