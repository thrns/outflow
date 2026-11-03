# Task 67 — Read-only caller guidance for settings persistence failures

```yaml
task_id: "Task 67 request; repository T-067 is unrelated and unchanged"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-10, UF-11]
acceptance: [AC-13]
acceptance_scope: "Test-only interpretation contract; no production settings caller or recovery flow"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; Rust stable 1.99.1; test temporary directory on APFS local journaled data volume"
implementation: "Added a pure guidance helper and 14 integration tests under the settings test target; documented caller boundary in README and existing recovery matrix"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 67 caller guidance", "evidence/rust-settings-temporary-write-recovery-task-56.md: Task 67 matrix row", "this report"]
risks: "Guidance is not wired to a production caller. OS-level rename/sync failure, crash durability, Windows, and Linux remain unverified."
review: PENDING
```

## Scope and inspected implementation

The repository is `/Users/bixxie/Desktop/Projects/outflow`, on branch `codex/task23-moonshine-build-scope`, based at `8c978bbd53617b7291fd0aeacf7c42cc260a0142`. The worktree already had uncommitted changes to `README.md`, `crates/settings/src/lib.rs`, `crates/settings/src/storage.rs`, Task 55/56 evidence, and untracked Task 57–66 evidence. Those changes were preserved. The root `AGENTS.md` applies; no nested `AGENTS.md` or `AGENTS.override.md` was found. No task completion marker, manifest, lockfile, dependency, public API, or production source was changed for Task 67.

Inspected the requested repository documents and code: `docs/PRD.md` (FR-13.4/recovery), `docs/ARCHITECTURE.md` (C-08), `docs/USER_FLOWS.md` (UF-10/UF-11), `docs/TECH_STACK.md` (TS-08), `docs/TASKS.md` (T-017/T-018), `crates/settings/src/lib.rs`, `crates/settings/src/storage.rs`, the Task 59–61 writer tests, Task 64–66 reset tests, Task 61 evidence, Task 66 evidence, the README recovery material, and the existing settings recovery matrix.

The writer variants remain distinct and unchanged. `ReplacementFailed` says the replacement operation returned an error but does not universally establish the destination contents. Task 61 corrected its documentation so it no longer promises original-file preservation from that variant alone. `ReplacementDurabilityUncertain` is returned after replacement succeeds but parent-directory synchronization fails; the attempted settings may already be visible, while crash durability remains unconfirmed. Task 66 confirms that a confirmed reset may return this error after defaults are already visible. Neither error establishes a universal rollback guarantee.

The source search found no production settings service or Tauri command handling either error. `initialize_settings` calls the public read-only `load_settings`; the settings examples call explicit APIs directly. The app source is a frontend preview and has no bridge to settings storage. No automatic retry, rollback, reset, migration, or reset confirmation UI was found. The `confirmed` boolean remains caller intent only.

## Test-only guidance contract

Added `crates/settings/tests/settings_persistence_guidance_contract.rs`. Its private enums classify the attempted operation, observed file state, typed writer result, guidance status, recommended next action, and automatic-recovery policy. The helper accepts `&SettingsStorageError`, `AttemptKind`, and `VisibleSettingsState`. It matches typed variants; it does not parse error strings.

The guidance distinguishes `ReplacementReportedUnsuccessful`, `ReplacementDurabilityUncertain`, and a conservative `OtherStorageFailure`. Status and next action derive from the independent read-only observation: inspect without writing, present currently visible original or attempted settings, or require recovery review for an unexpected, missing, or unreadable configuration. The observation retains `Missing` and `Unreadable` as separate categories. The guidance always reports durable persistence unconfirmed and rollback unconfirmed; a separate flag preserves the writer's explicit durability uncertainty result. Every result prohibits implicit retry, rollback, reset, deletion, migration, and repair.

Where classification is needed, the test calls the existing `load_settings`. It compares the loaded `Settings` temporarily with captured original and attempted values and retains only an enum. It does not call initialization or use defaults on a failed load. Synthetic files are under an isolated `std::env::temp_dir()` directory. Missing-file observations leave the path absent; malformed-file observations preserve exact bytes. Guidance retains no settings object, serialized JSON, vocabulary, or comparison key.

Fourteen tests cover the requested replacement-error cases, visible original/attempted/other settings, missing and unreadable files, reset and update retry prohibitions, content redaction, deterministic fallback for other errors, read-only behavior, and an assertion-backed content-free summary. The summary output is:

```text
Outflow settings persistence caller guidance

ReplacementFailed, not reloaded:
  Destination state: NOT VERIFIED
  Automatic retry: NO

ReplacementFailed, original visible:
  Destination state: ORIGINAL CURRENTLY VISIBLE
  Universal rollback guarantee: NO

ReplacementDurabilityUncertain, replacement visible:
  Destination state: REPLACEMENT CURRENTLY VISIBLE
  Crash durability: UNCONFIRMED
  Automatic rollback: NO

Confirmed reset with durability uncertainty:
  Original vocabulary assumed preserved: NO
  Automatic reset retry: NO

Unreadable settings:
  Recovery review required: YES
  Automatic overwrite: NO

Settings files modified by guidance: NO
Real user settings accessed: NO
Production caller implemented: NO
```

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; Rust stable `1.99.1`; Node `v22.23.3`; Python `3.13.13`. Test directories are created below `$TMPDIR`, on `/System/Volumes/Data` (`apfs, local, journaled`). No user settings path was accessed.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_persistence_guidance_contract --locked -- --nocapture` | **PASS** — 14 new integration tests; assertion-backed summary printed. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings settings_write_error_outcome_contract --locked -- --nocapture` | **PASS** — existing Task 61 test; unchanged. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings confirmed_reset_directory_sync_failure_leaves_defaults_visible --locked -- --nocapture` | **PASS** — existing Task 66 test; unchanged. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 228 unit/integration tests and 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 321 unit/integration tests and 1 doc test; prior baseline 307 plus 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — TypeScript check and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** — final tree check recorded after all edits. |

No Moonshine CMake configuration was attempted.

## Changed files and remaining limits

- `crates/settings/tests/settings_persistence_guidance_contract.rs` — test-only guidance types/helper and 14 tests.
- `README.md` — short caller guidance and focused command.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — Task 67 matrix row.
- `evidence/rust-settings-persistence-guidance-task-67.md` — this report.

No production settings caller was added. No human confirmation UI exists. No automatic retry or rollback exists. No OS-level rename or synchronization failure was reproduced. Crash durability remains unverified. Windows and Linux filesystem behavior remain unverified. No persistent recovery metadata, schema-v2 support, migration, vocabulary CRUD, or production reload-on-error behavior was added. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete; repository T-067 and all other task markers were not edited.
