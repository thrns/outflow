# Task 55 — Atomic settings writer recovery audit

```yaml
task_id: "Task 55 request; repository T-055 is unrelated"
feature_ids: [F-12, F-13]
flows: [UF-11]
acceptance: [AC-13]
acceptance_scope: "Partial settings-recovery evidence only; AC-13 remains incomplete"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0, build 25G83), arm64; Rust stable 1.99.1"
implementation: "Expanded private Unix settings-writer unit coverage; added README guidance and this audit evidence"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 55 pre-replacement recovery section", "evidence/rust-settings-writer-recovery-task-55.md"]
risks: "Windows and Linux behavior, actual disk-full and rename failures, corruption recovery, and crash/power-loss durability remain unverified"
review: PENDING
```

**Scope:** Task 55 from the current work request; this is separate from repository T-055. Relevant requirements are PRD FR-12.3 and FR-13.4, Architecture C-08 and §7, Tech Stack TS-08, and repository T-017/T-018. No task ledger or schema status was changed.

## Writer recovery coverage matrix

| Failure scenario | Evidence status |
|---|---|
| Invalid settings rejected before writing | **PASS** — `storage::tests::invalid_settings_do_not_modify_existing_file` asserts `InvalidSettings`, unchanged destination bytes, and no temp artifact. Validation is called before JSON encoding or filesystem operations in `save_settings_atomic_with_destination_policy`. |
| Temporary file creation failure | **NOT TESTED** — `create_new` failures are mapped to typed errors, but no deterministic create-failure hook/test exists. |
| Failure while writing temporary content | **PASS** — `storage::tests::simulated_partial_write_failure_preserves_file_and_cleans_temporary_file` writes a short prefix to the temporary file, returns an injected I/O error, then checks old bytes and cleanup. This is simulated write failure, not actual disk exhaustion. |
| Failure before atomic replacement | **PASS** — new `storage::tests::atomic_writer_pre_replacement_failure_preserves_valid_settings_and_allows_retry` injects after the temporary file is written, flushed, and synchronized and after the final destination/parent checks, immediately before `fs::rename`; exact old bytes and reloadability are asserted. |
| Destination replacement failure | **NOT TESTED** — the writer maps an actual `fs::rename` error to `ReplacementFailed`, but no test injects a rename failure. The pre-replacement hook test does not claim to exercise this operation. |
| Existing valid configuration preserved | **PASS** — the new recovery test writes and reloads schema-v1 settings through the public writer, retains the exact bytes, injects a pre-replacement error, then asserts the destination is a regular file, the bytes are identical, and the settings and vocabulary reload unchanged. |
| Temporary artifact cleanup | **PASS** — the partial-write, file-sync, pre-replacement, and new recovery tests assert that no `.outflow-settings-tmp-*` artifact remains. The new test also asserts that the isolated directory contains only the destination. |
| Corrupted settings recovery | **GAP** — loading/initialization reject and preserve malformed settings; defaults can be written only through explicit confirmed reset. There is no automatic last-known-good recovery or safe-default plus review flow for corrupted data, as called for by PRD FR-13.4. |
| Disk-full handling | **GAP** — a partial-write callback simulates an I/O failure and verifies cleanup, but no real `ENOSPC`/disk-full condition was exercised. The host filesystem was not filled. |
| Windows ACL and replacement behavior | **GAP** — the writer returns `UnsupportedPlatform` on non-Unix targets. There is no owner-only Windows ACL or Windows replacement implementation, and Windows was not tested. |
| Linux filesystem behavior | **NOT TESTED** — verification ran on macOS arm64. Shared `cfg(unix)` source is not evidence of a Linux filesystem run. |
| Crash and power-loss durability | **GAP** — the writer calls `sync_all` on the temporary file and parent directory around replacement, but no process-crash or power-loss test was run. Directory-entry durability across all supported filesystems is unproven. |

## Implementation audit

Implementation is in [`crates/settings/src/storage.rs`](../crates/settings/src/storage.rs), with the public typed storage API re-exported by [`crates/settings/src/lib.rs`](../crates/settings/src/lib.rs). The schema-v1 DTO and validation are in that library file. Tests and the existing isolated `TestDirectory` helper live in the private Unix-only `storage::tests` module; no temporary-directory dependency was added. The prior writer implementation and test record in [`evidence/T-018.md`](T-018.md) was also reviewed; it described hook-driven partial-write and pre-replacement cases but did not establish preservation of a previously valid, writer-created settings document.

The writer path currently performs these operations:

1. `save_settings_atomic_with_destination_policy` calls `Settings::validate` and the schema-v1 `to_json` codec before opening or creating destination files.
2. The Unix writer checks the parent directory and destination, opens the parent directory, then creates an exclusive temporary file in that same directory with `create_new`. It retries name collisions up to 128 attempts and creates the file with mode `0600`.
3. It applies mode `0600`, writes the serialized bytes, flushes, and synchronizes the temporary file.
4. Immediately before commit, the writer rechecks the destination and parent-directory identity.
5. The private `before_replace` callback runs after those checks, directly before `fs::rename`. Production `save_settings_atomic` supplies `|| Ok(())`; the test supplies a deterministic error.
6. On successful rename, the temporary-file cleanup guard is disarmed and the parent directory is synchronized. If that sync fails, the typed result is `ReplacementDurabilityUncertain`; the new file is already visible and is not rolled back.
7. On a pre-rename error, the old destination was never removed. The writer explicitly removes its temporary artifact; the guard has a best-effort `Drop` cleanup fallback. Cleanup failure is returned as `TemporaryFileWriteFailed`.

The public `save_settings_atomic` API and normal production behavior are unchanged. The only source adjustment was moving the existing private hook to the commit boundary after final path checks; this lets the test inject failure at the requested point, while production's no-op proceeds to rename normally. No new production seam was needed. Non-Unix builds fail closed with `UnsupportedPlatform`; mode `0600` is set for the replacement file rather than copying permissions from an existing destination. No replacement backup or persistent rollback mechanism exists.

The reader accepts an explicit path only, bounds file size, checks regular-file/symlink status, decodes schema v1, and validates it. It does not discover the user's configuration directory. The migration contract [`docs/VOCABULARY_MATCHING_MIGRATION.md`](../docs/VOCABULARY_MATCHING_MIGRATION.md) continues to specify that vocabulary migration is not implemented and persistent rollback is outstanding.

## Selected scenario and observed result

Test: `storage::tests::atomic_writer_pre_replacement_failure_preserves_valid_settings_and_allows_retry`.

The test uses a unique real directory under `std::env::temp_dir()` and synthetic settings. It creates the first schema-v1 file with `save_settings_atomic`, reloads it, parses and checks `schema_version == 1`, captures its exact bytes, and builds a distinct valid settings value. The `before_replace` callback records that it ran after the final path checks and returns an injected I/O error immediately before rename. Assertions verify the typed `ReplacementFailed`, the original pathname and exact bytes, successful reload of the original settings and vocabulary, no temporary or extra files, and a successful normal write/reload of the replacement value. The test drops its directory helper and asserts the isolated directory no longer exists. No settings JSON or vocabulary values are printed.

Observed output from the focused test on the current macOS host:

```text
Outflow atomic settings writer recovery test

Initial settings write: PASS
Original settings reload: PASS
Pre-replacement failure: INJECTED
Write rejected: PASS
Original bytes preserved: PASS
Original settings reload after failure: PASS
Subsequent normal write: PASS

Schema migration performed: NO
Real user settings accessed: NO
Power-loss recovery verified: NO
Windows ACL verification: NOT RUN
Linux filesystem verification: NOT RUN
```

The test is a filesystem preservation check, unlike Task 54's in-memory review-draft cancellation. It does not establish rollback of a schema migration.

## Verification

Environment: macOS 26.6.2 (Darwin 25.6.0, build 25G83), arm64; Rust stable `1.99.1`. All test directories and settings documents were synthetic and temporary. No actual user configuration path was inspected or written.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings pre_replacement --locked -- --nocapture` | **PASS** — 2 existing pre-replacement/reset tests passed before the new test was added. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_pre_replacement_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — 1 focused recovery test passed and printed the observed summary above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic --locked -- --nocapture` | **PASS** — 2 matching writer tests passed. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 203 unit/integration tests passed; 1 doc test passed. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all` | **PASS** — formatted the new assertion line after the first format check identified wrapping. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS** after formatting. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 296 unit/integration tests passed; 1 doc test passed. This is the stated baseline of 295 Rust tests plus this one new test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — typecheck and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS**. |

## Remaining requirements and limits

- T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. No task ledger completion state was changed; repository T-055 is separate and untouched.
- This task verifies one failure after preparation but before rename. It does not test temp-file creation failure, actual rename failure, actual disk full, corruption recovery, crash/power loss, or persistence migration rollback.
- Windows owner-only ACL and replacement semantics remain unsupported/unverified. Linux filesystem behavior was not run. The successful test is macOS-only evidence.
- Parent-directory sync is implemented after rename, but all-platform directory-entry durability and successful rollback after replacement are not established.
- No schema-v2 decoder/writer, vocabulary migration/CRUD, dependency, Cargo manifest, or lockfile change was introduced for this task.
- Release-sized, cross-platform, and real-user-configuration verification remain out of scope and unmeasured.

Final `git status --short --branch` remains on `codex/task23-moonshine-build-scope`. It contains the pre-existing uncommitted Task 43–54 changes plus this task's edits to `crates/settings/src/storage.rs`, an additive README subsection, and this new evidence file. No files were staged or committed.

**Task 56 extension:** [Temporary-write failure cleanup and retry evidence](rust-settings-temporary-write-recovery-task-56.md) adds a distinct failure during temporary-file writing and updates the writer recovery coverage matrix. Task 55's pre-replacement test remains unchanged.
