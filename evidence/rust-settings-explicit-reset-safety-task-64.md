# Task 64 — Explicit settings reset safety audit

```yaml
task_id: "Task 64 request; repository T-064 is unrelated and remains unchecked"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-10, UF-11]
acceptance: [AC-12, AC-13]
acceptance_scope: "Storage-library reset and initialization regression only; no recovery UI, human confirmation, migration, or release acceptance"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0, build 25G83), arm64; Rust stable 1.99.1; test temporary directory on APFS local journaled data volume"
implementation: "Added one focused Unix storage test for explicit reset scope, read/initialization separation, unsupported schemas, redaction, and repeat behavior; updated README and storage recovery matrix"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: explicit reset contract and Task 64 focused test", "evidence/rust-settings-temporary-write-recovery-task-56.md: Task 64 reset isolation row", "this report"]
risks: "The reset API's boolean is caller intent, not evidence of human confirmation; no production reset caller or confirmation UI exists. Windows is unsupported by the current writer; Linux filesystem behavior and crash durability were not tested. Recovery, backup restoration, and schema migration remain unimplemented."
review: PENDING
```

## Scope and inspected contract

Confirmed repository root `/Users/bixxie/Desktop/Projects/outflow`, branch `codex/task23-moonshine-build-scope`, worktree, and existing dirty state before editing. The existing uncommitted Task 55–63 work was retained. The repository-root `AGENTS.md` applies; no nested `AGENTS.md` or `AGENTS.override.md` was present. No branch, worktree, commit, or task completion marker was changed.

Read `docs/PRD.md` F-12, FR-13.4, and AC-08; `docs/ARCHITECTURE.md` C-08 and §7; `docs/USER_FLOWS.md` UF-10 and UF-11; `docs/TECH_STACK.md` TS-08; `docs/TASKS.md` T-017 and T-018; `crates/settings/src/lib.rs`; `crates/settings/src/storage.rs`; the reset example and all Rust call sites; Task 62 and Task 63 evidence; the current README reset, corruption, and initialization sections; and the existing storage recovery matrix.

The existing public API is `reset_settings_to_defaults(path: &Path, confirmed: bool) -> Result<(), SettingsStorageError>`.

- **Initiation and confirmation:** only a caller that invokes the public function initiates reset. `confirmed=false` returns `ResetNotConfirmed` before filesystem inspection. The boolean means caller intent only. Repository search found the API definition, tests, and `crates/settings/examples/reset_settings.rs`; no production caller, Tauri command, or user confirmation UI exists.
- **Effect:** with `confirmed=true`, the API replaces the caller-selected existing regular file with validated compiled schema-v1 defaults through the existing atomic writer. It does not delete the settings path or return only in-memory defaults. Persisted vocabulary becomes empty along with non-default preferences.
- **Input handling:** reset does not read or decode the existing contents. The same explicit operation can therefore replace malformed JSON or an unsupported schema. This is destructive reset to defaults, not decoding, migration, or automatic downgrade.
- **Path scope:** the caller supplies the destination. Source inspection shows the operation inspects and replaces that path, creates a transient temporary file in its parent directory, and synchronizes the same parent. It does not discover the user configuration directory, write a backup, or access another configured path.
- **Preconditions and errors:** missing destinations return `DestinationMissing` and are not created; symbolic links and non-regular destinations return `UnsafeDestination`. Invalid paths/parents, permission failures, temporary creation/permission/write/sync failures, replacement failures, and unsupported platforms use existing typed variants. A post-replacement parent-sync error returns `ReplacementDurabilityUncertain`: defaults may already be visible, and reset is not automatically retried. `ReplacementFailed` alone does not establish post-error destination state.
- **Privacy:** error formatting is redacted and retains neither paths, JSON, vocabulary, nor shortcut values. The operation does not promise secure erasure; filesystem snapshots or OS backups may retain old bytes.
- **Repeat behavior:** the API has no separate idempotence guarantee. On the current implementation, a second confirmed call succeeds while the destination remains a regular file and writes the same defaults again.
- **Platform behavior:** the existing owner-only atomic writer is implemented for Unix. It creates the replacement with mode `0600`; non-Unix targets fail closed with `UnsupportedPlatform`.

The recovery boundary remains distinct: **loading and initialization are read-only**; **explicit reset can replace persisted configuration**; a **future reset UI must establish user confirmation and accessibility**; and **persistent migration remains separate and unimplemented**. Task 64 does not complete FR-13.4, AC-08, T-017, or T-018. T-038, T-039, and T-053 also remain incomplete. No repository `T-064` marker was changed.

## Regression test and observed behavior

Added `storage::load_tests::explicit_settings_reset_is_scoped_and_separate_from_loading` in `crates/settings/src/storage.rs`. It uses three unique `TestDirectory` fixtures rooted at `std::env::temp_dir()` and synthetic content only; no actual user configuration directory is referenced.

1. The main fixture writes non-default schema-v1 settings through `save_settings_atomic`, reloads them, confirms two synthetic vocabulary entries, and captures both the settings bytes and an unrelated sentinel's bytes plus sorted directory entries.
2. Public loading and normal initialization both return the original settings. File bytes, sentinel bytes, and directory entries remain unchanged. An unconfirmed reset returns `ResetNotConfirmed` without mutation; its Display and Debug forms contain neither vocabulary nor JSON.
3. A confirmed reset returns `Ok(())`, leaves a valid settings file containing `Settings::default()` at schema v1, clears the persisted vocabulary, and sets Unix permissions to `0600`. Subsequent loading and initialization return the persisted defaults.
4. A second confirmed reset succeeds and leaves the default settings bytes, sentinel, and directory entries unchanged.
5. A separate malformed-JSON fixture and a separate schema-v2 fixture each have a sentinel. The loader and initializer return their existing typed errors and preserve both files and the exact directory entries. Display and Debug errors do not include synthetic vocabulary or raw JSON.
6. Only after those preservation assertions, the explicit confirmed reset replaces the separate schema-v2 fixture with persisted schema-v1 defaults. This demonstrates current explicit reset behavior, not migration. Its sentinel and directory entries remain unchanged.

Exact directory-entry equality verifies that no unexpected temporary, backup, transcript, or audio file remains in any fixture. The code path is limited by inspection to the supplied settings path and its same-directory temporary file; this test does not instrument operating-system file access outside the fixture.

Focused output, printed only after the assertions pass:

```text
Outflow explicit settings reset safety test

Valid settings before reset: VERIFIED
Stored vocabulary before reset: VERIFIED
Load without reset: NON-DESTRUCTIVE
Initialization without reset: NON-DESTRUCTIVE
Unconfirmed reset: REJECTED WITHOUT MUTATION
Explicit reset invoked: VERIFIED
Documented reset result: DEFAULTS PERSISTED AS SCHEMA V1
Previous vocabulary after reset: NOT PRESENT IN PERSISTED SETTINGS
Unrelated sentinel files: PRESERVED
Unexpected temporary, backup, transcript, or audio files: NONE
Repeated reset behavior: SUCCEEDED; DEFAULTS UNCHANGED
Malformed and unsupported settings during initialization: PRESERVED
Explicit schema-v2 reset: DEFAULTS PERSISTED; MIGRATION NOT PERFORMED
Automatic reset: NO
Schema migration: NO
Real user settings accessed: NO
User confirmation UI tested: NO
```

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; Rust/Cargo stable 1.99.1. `std::env::temp_dir()` resolved to `/var/folders/4k/wvp_db_57gs6r0c2v4l_fg4h0000gn/T/`, on the APFS local journaled data volume `/dev/disk3s5` mounted at `/System/Volumes/Data`. The workspace contains only `outflow-core` and `outflow-settings`; requested Cargo workspace commands did not invoke the blocked Moonshine CMake configuration.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings explicit_settings_reset_is_scoped_and_separate_from_loading --locked -- --nocapture` | **PASS** — 1 focused test; printed the assertion-backed summary above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings reset --locked` | **PASS** — 11 reset-related tests, including existing reset, error, corruption, and new isolation coverage. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings corrupted_settings_files_are_rejected_and_preserved_without_automatic_reset --locked -- --nocapture` | **PASS** — Task 62 corruption test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings initialization_rejects_and_preserves_invalid_configurations --locked` | **PASS** — existing initialization classification test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings settings_initialization_preserves_unsupported_schema_files --locked -- --nocapture` | **PASS** — Task 63 unsupported-schema initialization test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings unsupported_schema --locked` | **PASS** — 7 unsupported-schema settings tests. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 212 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 305 unit/integration tests; 1 doc test. Tasks 55–63 remain passing. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS** — core and settings workspace only; Moonshine CMake not attempted. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — typecheck and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS**. |

## Files and remaining limits

- `crates/settings/src/storage.rs` — one focused Unix-only regression test; no reset implementation or public API changed.
- `README.md` — explicit reset scope, repeat behavior, focused test command/expectations, confirmation boundary, and corrected unsupported-schema wording.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — brief Task 64 reference in the storage recovery matrix.
- `evidence/rust-settings-explicit-reset-safety-task-64.md` — this audit evidence.

The valid and unsupported reset fixtures write only the caller-supplied temporary settings path. Sibling sentinel bytes were preserved and no unexpected files remained. The unsupported schema was preserved until an explicit confirmed reset call; there is no automatic schema-v2 loading, writing, reset, downgrade, or migration. A reset call with `confirmed=true` is not proof of user consent, and no confirmation UI or accessibility behavior was tested. The current reset has no backup or last-known-good restoration path and is not secure erasure. Windows writer support, Linux filesystem behavior, crash/power-loss durability, persistent schema migration, and the broader FR-13.4 recovery flow remain unimplemented or unverified.

No dependency, manifest, or lockfile changed. No frontend, native, ASR, or product UI code changed. Pre-existing Task 55–63 uncommitted work remains in place. No files were staged or committed, and no task completion status was updated.
