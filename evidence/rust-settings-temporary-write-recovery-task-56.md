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
| Temporary-file creation failure | **PASS (Task 57 synthetic operation failure)** — the test injects at the exclusive temporary-file creation callback, verifies it is reached once without a collision retry, then checks original bytes, reload, no artifact, and normal retry. This is not an actual OS permission or filesystem failure. |
| Temporary-write failure after a verified nonempty prefix | **PASS (Task 56)** — the test injects an I/O error from the write callback, verifies partial bytes, original-file preservation, cleanup, reload, and retry. This is not actual `ENOSPC`. |
| Temporary-file synchronization failure before replacement | **PASS (Task 58 injected callback)** — the sync callback reads back and decodes the complete temporary schema-v1 document after the writer's flush, then returns the synthetic error in place of `sync_all`; the test verifies typed propagation, no replacement, original bytes and reload, cleanup, and normal retry. This is not an OS-level sync failure. |
| Failure immediately before replacement | **PASS (Task 55)** — the separate test injects after the replacement contents are fully written and synchronized, immediately before rename; that test remains unchanged and passes. |
| Actual temporary-directory permission denial | **NOT TESTED** — callback-based permission mapping coverage is not an actual directory permission failure. |
| Actual disk-full failure | **NOT TESTED** — no disk space was exhausted. |
| Actual OS-level temporary-file synchronization error | **NOT TESTED** — Task 58 returns a controlled error from the private callback instead of calling `sync_all`. |
| Parent-directory synchronization error after successful replacement | **PASS (Task 60 injected callback)** — the test uses the real `fs::rename`, proves the complete new bytes are visible when the parent-sync callback is entered, injects an error there, verifies `ReplacementDurabilityUncertain`, reloads the new settings, checks permissions and cleanup, and succeeds on a later public write. This does not reproduce an OS-level sync failure or establish crash durability. |
| Confirmed reset parent-directory synchronization error after successful replacement | **PASS (Task 66 injected callback)** — reset prepares and synchronizes schema-v1 defaults, uses the real rename, then returns `ReplacementDurabilityUncertain` from the parent-sync callback. The test verifies visible/reloadable defaults, cleared vocabulary and preferences, permissions, sentinel preservation, temp cleanup, and a later public write. This does not prove OS-level sync failure or crash durability. |
| Synthetic parent-directory synchronization callback error mapping | **PASS (injected callback only)** — existing `parent_sync_failure_is_distinguished_after_replacement` checks `ReplacementDurabilityUncertain`; the replacement is already visible and no rollback is claimed. |
| Destination replacement-operation failure | **PASS (Task 59 injected callback)** — the replacement callback is reached with the expected temporary source and existing destination after complete write, flush, and successful temporary-file synchronization; it returns a controlled I/O error. This tests writer error handling, not an actual OS-level rename failure. |
| Caller-visible distinction between replacement errors | **PASS (Task 61 contract test)** — one paired test confirms both calls return typed `Err` while the injected pre-replacement callback preserves the old destination and the post-replacement sync callback leaves the new destination visible. The classification is test-local and limited to these injected scenarios. |
| Read-only caller guidance after persistence errors | **PASS (Task 67 test-only contract)** — distinguishes the typed writer outcome from a subsequent `load_settings` observation, routes missing/unreadable/unexpected settings to review, and prohibits automatic recovery. This is not production caller behavior or filesystem failure evidence. |
| Post-error visible-settings inspection | **PASS (Task 68 test-only contract)** — calls `load_settings` once per inspection and returns only a classification for original, attempted, equal snapshots, other valid settings, missing, invalid document, unsupported schema, or other read failures. Repeated reads preserve fixture bytes, metadata, and directory entries; no production caller or durability claim is added. |
| Settings snapshot freshness and history ambiguity | **PASS (Task 69 test-only contract)** — verifies that a retained A observation stays `OriginalVisible` after an explicit public write of B, while a fresh loader call reports `AttemptedVisible`; equal snapshots classify as `BothSnapshotsMatch` both with and without an intervening same-value write; A → B → A returns to `OriginalVisible`; a channel-ordered two-actor schedule observes A, then B; and a removed fixture is classified `Missing` by a later inspection. This proves only the scheduled completed-operation order, not overlapping filesystem safety, linearizability, locking, transaction isolation, or durability. |
| Actual OS-level rename failure | **NOT TESTED** — Task 59 returns the injected error instead of calling `fs::rename`; platform replacement atomicity and ambiguous syscall outcomes are not established. |
| Post-replacement rollback | **NOT IMPLEMENTED OR VERIFIED** — no rollback mechanism or test was added. |
| Crash or power-loss recovery | **NOT TESTED** — no process interruption or power-loss test was run. |
| Windows ACL and replacement behavior | **NOT TESTED** — the writer fails closed with `UnsupportedPlatform` on non-Unix; Windows was not run. |
| Linux filesystem behavior | **NOT TESTED** — source uses the Unix implementation, but verification ran only on macOS/APFS. |
| Permission denial and temporary-file cleanup | **PASS (injected callback only)** — existing permission-denial mapping coverage remains passing; it does not substitute for actual OS denial or a cross-platform permission matrix. |

### Separate load-side corruption coverage

| Read scenario | Evidence status |
|---|---|
| Truncated, malformed, empty, and structurally invalid existing settings | **PASS (Task 62)** — each fixture returns `InvalidDocument`; bytes, Unix mode, file identity, modification time, and directory entries remain unchanged across repeated loader and initializer failures. This is read-side preservation evidence, not a writer-failure injection. |
| Missing settings versus invalid existing settings | **PASS (Task 62)** — the loader returns `Ok(None)` only for the missing path; initialization returns in-memory first-run defaults without creating a file, while invalid existing files remain errors. |
| Unsupported-schema initialization preservation | **PASS (Task 63)** — a v1-shaped schema-v2 fixture and a minimal `{"schema_version":999}` future fixture return `InvalidSettings(UnsupportedSchemaVersion { .. })`; repeated initialization preserves bytes, Unix mode, device/inode, modification time, and directory entries. Ordinary loading/initialization do not reset, downgrade, or migrate; the separate confirmed reset API can replace a selected file with schema-v1 defaults. No schema-v2 decoder or migration is implemented. |
| Explicit reset isolation vs ordinary loading/initialization | **PASS (Task 64)** — a synthetic valid schema-v1 fixture confirms only an explicit confirmed call replaces settings with persisted defaults and removes vocabulary from the active file; ordinary valid, malformed, and schema-v2 loads/initialization preserve their files. The sibling sentinel and directory entries remain unchanged, repeat reset succeeds, and explicit replacement of a separate schema-v2 fixture is not migration. No human confirmation UI is tested. |
| Confirmed reset failure immediately before replacement | **PASS (Task 65 injected callback)** — after the reset prepares and synchronizes schema-v1 defaults, the reused private pre-replacement hook returns a controlled error. The test verifies `ReplacementFailed`, exact original bytes, reloadable ordered vocabulary and preferences, unchanged permissions and sentinel, temporary-file cleanup, then a separate public confirmed reset that persists defaults. This is not an OS-level failure and does not cover post-replacement errors. |

Task 62 covers corrupted JSON rejection; Task 63 separately covers unsupported-schema initialization preservation; Task 64 audits the explicit reset boundary; Task 65 adds reset-specific pre-replacement failure and explicit retry coverage; Task 66 adds reset-specific post-replacement durability uncertainty coverage. Tasks 55–61 cover atomic-writer failure paths. Tasks 55–61 establish separate pre-replacement, partial-write, temporary-creation, temporary-file synchronization, replacement-operation, and post-replacement parent-directory synchronization injected paths on macOS/APFS. These observations apply only to the specified injected scenarios: Task 65 shows that the original destination remains intact when its pre-replacement hook fails, while Task 66 shows that defaults are already visible when parent-directory synchronization fails after the real replacement. Task 66 does not claim that defaults are crash durable, that all reset errors preserve the original, or that rollback or automatic retry exists. These tests do not prove actual permission denial, disk-full recovery, OS-level file-sync, rename, or directory-sync failure, rollback after replacement, persistent last-known-good recovery, or crash durability. Task 64 and Task 66 do not add confirmation UI, automatic recovery, or schema migration; Tasks 65 and 66 have no human confirmation UI. Remaining T-018 work includes real filesystem failure evidence, Linux behavior, approved Windows owner-only ACL and replacement support, and the broader cross-platform writer requirements.

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
