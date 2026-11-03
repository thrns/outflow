# Task 62 — Corrupted settings preservation

```yaml
task_id: "Task 62 request; repository T-062 status not changed"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-11]
acceptance: [AC-12, AC-13]
acceptance_scope: "Reader-side corruption rejection and preservation tests only; release acceptance remains incomplete"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; synthetic temporary directories under /var/folders on the local APFS data volume; Rust stable 1.99.1"
implementation: "Added one focused Unix settings-loader preservation test covering four corruption classes, missing-path distinction, redaction, repeated failures, and filesystem snapshots"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 62 safe-rejection behavior, test command and limits", "evidence/rust-settings-temporary-write-recovery-task-56.md: separate load-side coverage matrix", "this evidence report"]
risks: "Windows and Linux filesystem behavior, cross-platform permissions, crash recovery, backup restoration, and the PRD's future user-review recovery flow remain unverified or unimplemented"
review: PENDING
```

## Inspection and existing behavior

Inspected the root `AGENTS.md` and searched for nested `AGENTS.md`/`AGENTS.override.md`; none were found under the repository. Read PRD settings/privacy/recovery requirements including FR-13.4 and AC-13; ARCHITECTURE C-08 and §7; TECH_STACK TS-08; USER_FLOWS UF-11 and reset references; TASKS T-017/T-018; `crates/settings/src/storage.rs`; `crates/settings/src/lib.rs`; existing schema-v1 validation and decoding; settings load, initialization, and reset tests; the Task 55–61 storage evidence and recovery matrix; and the README storage sections.

The public `load_settings(path)` is read-only. It validates the caller-supplied path, bounds the file read, decodes UTF-8, and delegates to `Settings::from_json`. It returns `Ok(None)` for a missing path and `Ok(Some(settings))` for a valid schema-v1 file. Malformed JSON, empty content, missing required fields, and wrong JSON field types map to `SettingsStorageError::InvalidDocument { line, column }`. Valid JSON with an unsupported schema version remains `SettingsStorageError::InvalidSettings(SettingsError::UnsupportedSchemaVersion { found })`. The error model already distinguishes read failures, invalid UTF-8, validation errors, and unsupported versions; no new public error variant was needed.

`initialize_settings(path)` propagates errors for existing invalid files. For a missing file only, it returns validated defaults as `SettingsInitialization::FirstRun` in memory and does not create a file. Reset is a separate `reset_settings_to_defaults(path, confirmed)` call; an unconfirmed reset is rejected without mutation, while an explicitly confirmed reset may replace an existing malformed file. No Tauri settings service or user-facing reset UI exists in this workspace.

The PRD's broader FR-13.4 recovery target (safe defaults and user review for corrupted settings) remains a future product flow. This task follows the explicit Task 62 scope: safe rejection and preservation, with any manual recovery or reset requiring separate design and user confirmation.

## Corruption preservation test

Added `storage::load_tests::corrupted_settings_files_are_rejected_and_preserved_without_automatic_reset` in `crates/settings/src/storage.rs`. It uses unique temporary directories and synthetic schema-v1 settings containing private-marker strings. The public writer creates each valid baseline and the public loader confirms it loads before test-only `fs::write` operations introduce corruption.

The focused test covers:

- Truncated JSON derived from the captured valid schema-v1 bytes.
- Syntactically malformed JSON with an invalid delimiter.
- A zero-byte existing settings file.
- Syntactically valid JSON with an invalid type for the required `language` field.
- Missing-path behavior compared with an existing invalid file.

Every corrupted fixture is rejected as `InvalidDocument`, not loaded as settings, treated as missing, reset, or replaced with defaults. The test repeats loader and initializer failures twice for each fixture. Across the attempts it verifies exact bytes, file existence, Unix mode, device/inode identity, modification time, and unchanged directory entries. The directory contains only the settings file; no temporary, backup, or replacement file appears. Both `Display` and `Debug` error forms are checked for leakage of synthetic vocabulary IDs/phrases or raw JSON. The test does not call the reset API.

Unsupported schema behavior remains covered by existing tests: a syntactically valid schema version 7 document is rejected as `InvalidSettings(UnsupportedSchemaVersion { found: 7 })`, and the production decoder continues to reject schema v2. No schema-v2 loader or migration was added.

Focused `--nocapture` output:

```text
Outflow corrupted settings preservation test

Valid schema-v1 settings baseline: PASS
Truncated JSON: REJECTED
Malformed JSON: REJECTED
Empty existing file: REJECTED
Structurally invalid JSON: REJECTED
Missing file distinction: VERIFIED

Corrupted file bytes: PRESERVED
Corrupted file permissions: PRESERVED
Repeated failed loads: NON-DESTRUCTIVE
Unexpected temporary artifacts: NONE
Automatic reset performed: NO
Original vocabulary recovered automatically: NO

Real user settings accessed: NO
Production schema-v2 loading: NOT IMPLEMENTED
Automatic corruption repair: NOT IMPLEMENTED
```

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; Rust stable 1.99.1. The temporary directories used by the tests reside under `/var/folders` on the local APFS data volume. Only synthetic settings were used.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings corrupted_settings --locked -- --nocapture` | **PASS** — 1 focused test; printed the asserted summary above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings initialization_rejects_and_preserves_invalid_configurations --locked` | **PASS** — existing malformed, unsupported-version, invalid-settings, UTF-8, and oversized-file classifications remain intact. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings valid_v1_file_loads_equal_settings_without_changing_the_file --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings missing_file_returns_none_without_creating_a_file --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 210 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 303 unit/integration tests; 1 doc test. Existing schema-v2 rejection and Tasks 55–61 tests passed. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — typecheck and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** after final diff review. |

Task 62 adds one Rust unit test to the pre-existing Task 61 workspace count of 302 unit/integration tests: 303 total, with one doc test reported separately. The settings package now runs 210 unit/integration tests plus one doc test.

## Files and remaining limits

- `crates/settings/src/storage.rs` — one focused Unix test and private test helpers; production loader, reset, writer, schema, and public APIs are unchanged.
- `README.md` — concise Task 62 handling, test command, expected guarantees, and limitations.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — separate read-side corruption coverage matrix linked to Task 62; writer fault-injection rows remain distinct.
- `evidence/rust-settings-corruption-preservation-task-62.md` — this report.

No dependency, manifest, lockfile, settings schema, migration, vocabulary CRUD, recovery UI, backup policy, or automatic repair changed. No repository completion marker was edited. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete; repository T-062 is unrelated and untouched.

Windows and Linux filesystem behavior, cross-platform permission preservation, crash/power-loss recovery, backup restoration, and a future user-reviewed recovery flow remain unverified or unimplemented. The PRD's broader FR-13.4 behavior and full AC-13 recovery requirements are not completed by this test.
