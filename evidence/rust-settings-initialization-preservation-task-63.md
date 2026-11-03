# Task 63 — Settings initialization preserves unsupported schemas

```yaml
task_id: "Task 63 request; repository T-063 status not changed"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-11]
acceptance: [AC-12, AC-13]
acceptance_scope: "Synthetic storage and initialization regression only; release and user-facing recovery acceptance remain incomplete"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; synthetic temporary directories under /var/folders on the local APFS data volume; Rust stable 1.99.1"
implementation: "Added schema-version-header preflight in Settings::from_json and one focused Unix test through the public loader, initializer, and writer"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 63 initialization contract and limits", "evidence/rust-settings-temporary-write-recovery-task-56.md: Task 63 row in the separate load-side matrix", "this evidence report"]
risks: "No user-facing recovery or migration flow exists; Windows and Linux filesystem behavior, cross-platform permission preservation, and crash recovery remain unverified"
review: PENDING
```

## Inspection and existing behavior

Confirmed the repository root, branch, worktree, and dirty state before editing. The active branch remains `codex/task23-moonshine-build-scope` at base `8c978bbd53617b7291fd0aeacf7c42cc260a0142`. Existing uncommitted Task 55–62 work was retained. The root `AGENTS.md` applies; no nested `AGENTS.md` or `AGENTS.override.md` was found.

Read PRD FR-13.4; ARCHITECTURE C-08 and §7; TECH_STACK TS-08; USER_FLOWS UF-11; TASKS T-017 and T-018; `crates/settings/src/storage.rs`; the schema-v1 DTO, JSON decoder, and validation in `crates/settings/src/lib.rs`; existing initialization, unsupported-schema, and reset tests; the Task 62 test and evidence; README settings sections; and the Task 55–61 recovery evidence and matrix.

Before this change, `load_settings(path)` returned `Ok(None)` for a missing path, and `initialize_settings(path)` returned validated `Settings::default()` in memory without writing. Existing supported schema-v1 settings were loaded unchanged. Malformed existing JSON returned `InvalidDocument`; a complete v1-shaped document declaring an unsupported version returned `InvalidSettings(UnsupportedSchemaVersion { .. })`. Explicit reset remained a separate `reset_settings_to_defaults(path, confirmed)` call.

Inspection found one classification defect in the decoder: `Settings::from_json` decoded every required schema-v1 field before checking the version. A syntactically valid future document containing only `{"schema_version":999}` was therefore rejected as `InvalidDocument` for missing v1 fields, even though the version itself was unsupported. No public error variant was needed.

## Implementation and regression coverage

`Settings::from_json` now first parses a small schema-version header that ignores unknown fields. Unsupported versions return the existing typed `SettingsError::UnsupportedSchemaVersion` before the schema-v1 DTO is required. A supported version is still fully decoded by the existing deny-unknown-fields v1 DTO and validated as before. This does not make schema v2 readable and does not migrate or write settings.

Added `storage::load_tests::settings_initialization_preserves_unsupported_schema_files`. The Unix-only test uses public `load_settings`, `initialize_settings`, and, for the supported baseline, `save_settings_atomic`. All data is synthetic and isolated in unique temporary directories.

- **Missing:** the loader returns `Ok(None)`; initialization returns `FirstRun(Settings::default())`; the settings path stays absent and the directory remains empty.
- **Supported schema v1:** the public writer stores non-default cleanup and startup values; initialization returns the same `Loaded` settings; the exact file bytes and directory entries remain unchanged.
- **Schema v2:** the fixture starts from the supported schema-v1 structure and changes only `schema_version`. The loader and two initializer attempts return `InvalidSettings(UnsupportedSchemaVersion { found: 2 })`.
- **Future schema 999:** the valid JSON fixture contains only `schema_version`. The loader and two initializer attempts return `InvalidSettings(UnsupportedSchemaVersion { found: 999 })`; no v1 fields are assumed.
- After each unsupported load/initialization attempt, the test compares exact bytes, file existence/type, Unix permission mode, device/inode identity, modification time, and sorted directory entries against the initial snapshot. Both fixtures remain unchanged and no extra files appear.

The test does not call reset. The public initializer only delegates to the loader and chooses defaults for `None`; reset remains a distinct explicit API. The existing `unsupported_schema_configuration_can_be_reset_explicitly` regression also passed in the package suite.

## Focused test output

`RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings settings_initialization_preserves_unsupported_schema_files --locked -- --nocapture` passed and printed:

```text
Outflow settings initialization preservation test

Missing settings:
  Loader returned Ok(None): PASS
  In-memory defaults returned: PASS
  Settings file created: NO

Valid schema v1:
  Existing settings loaded: PASS
  Stored values preserved: PASS
  File bytes unchanged: PASS

Unsupported schema v2:
  UnsupportedSchemaVersion returned: PASS
  Original bytes preserved: PASS
  Repeated initialization non-destructive: PASS

Unsupported future schema:
  UnsupportedSchemaVersion returned: PASS
  Original bytes preserved: PASS

Automatic downgrade: NO
Automatic reset: NO
Schema migration: NO
Real user settings accessed: NO
```

Each PASS line follows the corresponding public-entry-point and filesystem assertions. The test prints no settings JSON or stored values.

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; temporary test paths under `/var/folders` map to `/System/Volumes/Data`, whose mount is APFS local and journaled. Rust stable 1.99.1. No real user settings path was discovered or accessed.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings settings_initialization_preserves_unsupported_schema_files --locked -- --nocapture` | **PASS** — 1 focused test; all printed checks followed assertions. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings corrupted_settings_files_are_rejected_and_preserved_without_automatic_reset --locked -- --nocapture` | **PASS** — Task 62 corruption regression remains passing. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 211 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 304 unit/integration tests; 1 doc test. Tasks 55–61 writer cases, Task 62 corruption preservation, and explicit-reset tests passed. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — typecheck and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** — no whitespace errors in the final reviewed diff. |

The existing settings package had 210 unit/integration tests before this addition; it now has 211. The workspace count is 304 unit/integration tests, with 1 doc test reported separately. No dependency, manifest, or lockfile changed.

## Recovery scope and remaining requirements

This establishes that missing settings get existing in-memory defaults, supported v1 settings load without a rewrite, and invalid or unsupported existing settings remain preserved and return typed errors. Returning an error is not equivalent to the FR-13.4 user-facing recovery flow. A future recovery interface must offer safe defaults and user review without silently discarding an unsupported configuration. Any downgrade, migration, backup restoration, or reset must be explicit and separately verified. This task does not complete FR-13.4, T-017, T-018, T-038, T-039, T-053, T-063, or AC-08. No repository task completion marker was edited.

Task 62 remains the corrupted JSON rejection and preservation evidence. Tasks 55–61 remain the atomic-writer failure-recovery evidence. Task 63 is only the unsupported-schema initialization boundary. Schema-v2 loading, migration, downgrade, automatic reset, and a user-facing recovery interface remain unimplemented. Windows/Linux filesystem behavior, crash recovery, backup restoration, and cross-platform permission behavior remain unverified.

## Files

- `crates/settings/src/lib.rs` — reject unsupported version headers before requiring the schema-v1 DTO; schema v1 still fully validates.
- `crates/settings/src/storage.rs` — one focused public loader/initializer regression test and Unix metadata snapshot helpers.
- `README.md` — initialization contract, test command, and recovery/migration limitations.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — Task 63 reference in the separate load-side matrix.
- `evidence/rust-settings-initialization-preservation-task-63.md` — this report.
