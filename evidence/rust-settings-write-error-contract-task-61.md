# Task 61 — Caller-visible settings write error contract

```yaml
task_id: "Task 61 request; repository T-061 status not changed"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-11]
acceptance: [AC-12, AC-13]
acceptance_scope: "Typed distinction and destination state for two injected settings-writer failures; release acceptance remains incomplete"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; temporary test paths under /var/folders on the local APFS data volume; Rust stable"
implementation: "Added one paired Unix storage contract test; clarified ReplacementFailed documentation and redacted Display text without changing variants or signatures"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 61 interpretation, caller guidance, command and limits", "evidence/rust-settings-temporary-write-recovery-task-56.md: Task 61 contract-matrix row", "this evidence report"]
risks: "Fault hooks verify only the injected scenarios; real OS-level failures, crash durability, rollback, Windows and Linux remain untested"
review: PENDING
```

## Inspection and caller audit

Inspected the root `AGENTS.md` (no nested `AGENTS.md` or `AGENTS.override.md` under the relevant source, docs, or evidence trees); PRD §7 and FR-13.4; ARCHITECTURE C-08 and §7; TECH_STACK TS-08; TASKS T-017/T-018; `crates/settings/src/storage.rs` and `crates/settings/src/lib.rs`; the Task 55–60 recovery tests; the Task 59 and Task 60 evidence; README recovery notes; and all repository uses of `ReplacementFailed`, `ReplacementDurabilityUncertain`, `SettingsStorageError`, and `save_settings_atomic`.

The public `save_settings_atomic` function returns `Result<(), SettingsStorageError>` unchanged. The settings reset path reaches the same writer and propagates its typed result; `initialize_settings` is read-only. The settings crate exports the error type. The only other writer references are examples and tests. The workspace currently contains only `core` and `settings`; no Tauri settings-update command, application service, or higher-level production writer caller exists. No production caller flattened the two variants, assumed every `Err` preserved the old destination, or automatically retried either error. Existing recovery tests make explicit subsequent writes as test actions; they do not implement a production auto-retry.

One caller-facing overstatement was found in the `ReplacementFailed` doc comment and display text: it said the existing settings were kept. That claim exceeded what the typed variant alone establishes for every OS-level replacement error. The comment now says the operation returned an error without implying the destination state, and display text is neutral. The enum variants, public signatures, and error schema are unchanged. `ReplacementDurabilityUncertain` continues to identify a completed replacement followed by failed parent-directory synchronization; it does not claim durable persistence or rollback.

## Contract test and observed results

Added `storage::tests::settings_write_error_outcome_contract`, using the existing private replacement and synchronization hooks and synthetic schema-v1 settings in two isolated temporary directories. A test-local classifier combines the typed error with bytes actually read from the destination; it is not a production API or a general filesystem classifier.

**Injected replacement-operation error:** the test writes and synchronizes the complete temporary replacement, then returns an injected error from the replacement callback without calling rename. The writer returns `Err(SettingsStorageError::ReplacementFailed)`. The destination bytes equal the captured original bytes, the replacement bytes differ, the original settings reload as schema v1, and no temporary file remains. Parent-directory synchronization was not reached.

**Injected parent-directory synchronization error:** the writer performs the real replacement. At the existing parent-sync callback, before returning an injected error, the test reads and decodes the replacement bytes at the destination. The writer returns `Err(SettingsStorageError::ReplacementDurabilityUncertain)`. After the error, those replacement bytes remain visible and reload as schema v1; they differ from the original bytes. The test performs no rollback or subsequent write.

Both error `Display` and `Debug` values are checked against complete original/replacement JSON and all synthetic vocabulary IDs, spoken phrases, and written replacements. The test prints only asserted labels and status values; no settings content is printed. No transcript, audio, or real user settings are used.

Focused `--nocapture` output:

```text
Outflow settings write error contract

Replacement-operation error:
  Typed error: ReplacementFailed
  Original destination: PRESERVED
  Replacement committed: NO

Post-replacement directory-sync error:
  Typed error: ReplacementDurabilityUncertain
  Replacement destination: VISIBLE
  Replacement settings reload: PASS
  Crash durability: UNKNOWN

Both writer operations returned errors: VERIFIED
Distinct destination outcomes: VERIFIED
Automatic rollback: NO
Real user settings accessed: NO
Actual OS-level failures reproduced: NO
```

The first result is specific to the callback that returned before performing the replacement operation. The second establishes visibility in the tested post-replacement sync-error scenario only. Neither result generalizes to every OS rename error, process crash, or hardware failure. `Result::Err` alone does not establish that the destination is unchanged. Visible replacement contents do not establish power-loss durability. No automatic rollback is attempted from a directory-sync error.

Future callers should report a verified pre-replacement failure as a failed save, keep the prior settings available, and allow only a deliberate retry. For `ReplacementDurabilityUncertain`, they must report persistence as unconfirmed, must not claim old settings were restored or durable success, and must not automatically overwrite the path with stale settings. Reload the visible configuration before further user actions when appropriate. These requirements are recorded in README and the public writer documentation; no speculative UI, service, or retry flow was added.

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; Rust stable 1.99.1. `std::env::temp_dir()` resolved to `/var/folders/4k/wvp_db_57gs6r0c2v4l_fg4h0000gn/T`; `mount` reported `/System/Volumes/Data` as `apfs, local, journaled`. Only isolated temporary directories were used.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings settings_write_error_outcome_contract --locked -- --nocapture` | **PASS** — 1 focused test; printed the asserted summary above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_replacement_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 59 regression. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_parent_directory_sync_failure_preserves_replaced_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 60 regression. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 209 unit/integration tests; 1 doc test. Tasks 55–60 remain passing. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all` | **PASS** — applied rustfmt after the first format check identified two formatting-only differences. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS** on rerun. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 302 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — TypeScript check and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** after final diff review. |

This adds one Rust unit test to the 301 unit/integration-test baseline: 302 total, with the one doc test reported separately. The settings package total is 209 unit/integration tests plus one doc test. The current Cargo workspace has only `outflow-core` and `outflow-settings` and contains no Moonshine CMake configuration; none was attempted.

## Files and remaining gaps

Task 61 changes:

- `crates/settings/src/storage.rs` — one paired contract test; clarified replacement-error docs and neutralized misleading display wording.
- `README.md` — contract interpretation, future caller guidance, focused test command, and platform/crash limitations.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — short contract-test row in the existing recovery matrix.
- `evidence/rust-settings-write-error-contract-task-61.md` — this evidence record.

No Cargo manifest or lockfile, dependency, settings schema, migration, vocabulary CRUD, UI, application service, automatic retry, or rollback changed. Pre-existing uncommitted Task 55–60 work was preserved. Repository `docs/TASKS.md` was not edited; T-017 and T-018 remain incomplete.

| Limitation | Status |
|---|---|
| Actual OS-level rename failure | **NOT TESTED** |
| Actual OS-level synchronization failure | **NOT TESTED** |
| Crash/power-loss durability | **NOT TESTED** |
| Automatic post-replacement rollback | **NOT IMPLEMENTED** |
| Windows replacement and ACL behavior | **NOT TESTED** |
| Linux filesystem behavior | **NOT TESTED** |
| Actual disk-full behavior | **NOT TESTED** |

Remaining T-018 requirements include validated cross-platform owner-only permissions and replacement semantics (including approved Windows ACL support), Linux filesystem verification, real filesystem failure and disk-full evidence, and recovery/last-known-good behavior for interruption and crash durability. Task 61 does not complete T-018 or FR-13.4 persistent recovery.
