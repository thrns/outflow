# Task 56 — Atomic settings temporary-write failure cleanup

```yaml
task_id: "Task 56 request; repository T-056 is unrelated"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-11]
acceptance: [AC-13]
acceptance_scope: "Partial settings-writer recovery evidence only; AC-13 remains incomplete"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "82dcf0f55ed4c1adaa5636c45a6ec97b4bc80ee4"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0, build 25G83), arm64; temporary directory backed by the APFS local journaled data volume; Rust stable 1.99.1"
implementation: "Added focused private Unix unit coverage using the existing write callback; updated README and linked Task 55 evidence"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 56 temporary-write section", "Task 55 evidence link", "this evidence and recovery coverage matrix"]
risks: "Actual disk exhaustion, OS-level synchronization and replacement errors, crash durability, Linux filesystem behavior, and Windows ACL/replacement behavior remain unverified"
review: PENDING
```

## Scope and writer path

This is Task 56 from the current work request, separate from repository T-056. It covers failure during temporary replacement-file writing. Repository T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. No task ledger, schema, migration, vocabulary persistence, dependency, Cargo manifest, or lockfile was changed.

The existing writer validates and serializes settings, creates an exclusive same-directory temporary file with mode `0600`, and calls its private `write_json` callback. On successful return it flushes and syncs the temporary file, checks the destination and parent again, calls the private pre-replacement hook, renames the temp file, then syncs the parent directory. If the write callback returns an error, later sync/check/replacement steps are skipped; the writer removes the temporary path through its cleanup guard and returns `TemporaryFileWriteFailed`. The existing pre-replacement hook and public `save_settings_atomic` API were left unchanged.

## Temporary-write test

Test: `storage::tests::atomic_writer_temporary_write_failure_preserves_valid_settings_and_allows_retry`.

The test uses `Settings::default()` as its initial valid schema-v1 document and toggles only `cleanup.remove_fillers` for a different valid replacement. It writes to a unique `TestDirectory` under `std::env::temp_dir()` and never discovers or accesses the user's configuration directory.

The private `write_json` callback writes and flushes a nonempty 16-byte prefix, locates the one newly created `.outflow-settings-tmp-*` file, confirms it is a regular file, reads it back, and compares the bytes with the serialized prefix. Only after those checks does the callback return a deterministic synthetic I/O error. A separate flag on the existing commit-boundary callback proves that the pre-replacement stage was not reached.

Assertions then confirm the typed `TemporaryFileWriteFailed`, an existing regular destination with bytes exactly equal to the captured original bytes, and exact directory-entry equality before and after failure (only the original destination remains). They reload the original settings, retry through public `save_settings_atomic`, reload the replacement settings, and confirm no temporary or other stale artifact remains. The settings contain no vocabulary entries, and neither the injected error nor the printed summary contains settings JSON or vocabulary content.

Observed focused output on this host:

```text
Outflow atomic settings temporary-write failure test

Initial settings write: PASS
Temporary replacement created: PASS
Partial temporary write: VERIFIED
Write failure: INJECTED
Destination replacement attempted: NO
Original destination bytes: PRESERVED
Original settings reload: PASS
Partial temporary file cleanup: PASS
Subsequent normal write: PASS
Stale temporary artifacts: NONE

Schema migration performed: NO
Real user settings accessed: NO
Actual disk-full failure tested: NO
Crash recovery verified: NO
Windows ACL verification: NOT RUN
Linux filesystem verification: NOT RUN
```

## Recovery coverage matrix

| Failure scenario | Evidence status |
|---|---|
| Temporary-write failure after a verified nonempty prefix | **PASS** — the new Task 56 test injects an I/O error from the write callback, verifies partial bytes, original-file preservation, cleanup, reload, and retry. This is not actual `ENOSPC`. |
| Failure immediately before replacement | **PASS** — Task 55's separate test injects after the replacement contents are fully written and synchronized, immediately before rename; that test remains unchanged and passes. |
| Actual disk-full failure | **NOT TESTED** — no disk space was exhausted; errors during the writer's later flush or `sync_all` are not represented by the new injection. |
| Temporary-file creation failure | **NOT TESTED** — no deterministic create failure is injected. |
| Temporary-file synchronization failure | **PASS (injected callback only)** — existing `simulated_file_sync_failure_preserves_file_and_cleans_temporary_file` checks typed handling and cleanup. No OS-level `fsync` failure was induced. |
| Parent-directory synchronization failure after rename | **PASS (injected callback only)** — existing `parent_sync_failure_is_distinguished_after_replacement` checks `ReplacementDurabilityUncertain`; the replacement is already visible and no rollback is claimed. |
| Destination replacement (`fs::rename`) failure | **NOT TESTED** — there is no deterministic rename-failure hook. |
| Crash or power-loss recovery | **NOT TESTED** — no process interruption or power-loss test was run. |
| Windows ACL and replacement behavior | **NOT TESTED** — the writer fails closed with `UnsupportedPlatform` on non-Unix; Windows was not run. |
| Linux filesystem behavior | **NOT TESTED** — source uses the Unix implementation, but verification ran only on macOS/APFS. |
| Permission denial and temporary-file cleanup | **PASS (injected callback)** — existing permission-denial coverage remains passing; it does not substitute for a cross-platform permission matrix. |

The test establishes one synthetic partial-write cleanup path on macOS/APFS. It does not prove actual disk-full recovery, rollback after replacement, persistent last-known-good recovery, or crash durability. Remaining T-018 work includes real filesystem failure evidence, Linux behavior, approved Windows owner-only ACL and replacement support, and the broader cross-platform writer requirements.

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; the unique temporary test directory is on the APFS local journaled data volume; Rust stable 1.99.1. Test settings and files were synthetic and isolated under that directory.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings temporary_write --locked -- --nocapture` | **PASS** — focused Task 56 test passed and printed the summary above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_pre_replacement_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — Task 55 test passed unchanged. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 204 unit/integration tests passed; 1 doc test passed. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 297 unit/integration tests passed; 1 doc test passed. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — typecheck and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS**. |

## Changed files and limits

- `crates/settings/src/storage.rs` — one focused Unix-only test; existing production writer and Task 55 test unchanged.
- `README.md` — concise Task 56 description, focused command, and scope limits.
- `evidence/rust-settings-writer-recovery-task-55.md` — link to this coverage extension.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — this evidence record.

No public settings API or schema changed. No migration, vocabulary persistence, or general-purpose fault-injection framework was added. Actual disk-full behavior, real synchronization and rename failures, crash/power-loss recovery, Linux filesystem behavior, and Windows ACL/replacement behavior remain unverified. Repository T-056 was not modified.
