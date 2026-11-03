# Task 58 — Atomic settings temporary-file synchronization failure recovery

```yaml
task_id: "Task 58 request; repository T-058 is unrelated"
date: "2026-11-02"
feature_ids: [F-12, F-13]
flows: [UF-11]
acceptance: [AC-13]
acceptance_scope: "Temporary-file synchronization failure recovery evidence only; AC-13 remains incomplete"
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "8c978bbd53617b7291fd0aeacf7c42cc260a0142"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0, build 25G83), arm64; test directory under /var/folders/4k/wvp_db_57gs6r0c2v4l_fg4h0000gn/T on the APFS local journaled data volume; Rust stable 1.99.1"
implementation: "Added one focused Unix-only writer recovery test using the existing private sync callback; updated README and the writer-recovery matrix"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 58 temporary-file synchronization section", "evidence/rust-settings-temporary-write-recovery-task-56.md: recovery matrix", "this evidence record"]
risks: "The injected callback error does not exercise an OS-level sync or rename failure, crash/power loss, post-replacement parent-directory sync failure, Linux filesystem behavior, or Windows ACL/replacement behavior"
review: PENDING
```

## Scope and writer sequence

This is Task 58 from the current work request, separate from repository T-058. Repository T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. No task ledger, schema, migration, vocabulary persistence, dependency, Cargo manifest, or lockfile was changed.

The inspected writer sequence is: validate settings; serialize to schema-v1 JSON; inspect and open the parent and inspect the destination; create a unique same-directory temporary file; set its mode to `0600`; write the full JSON; call `flush`; invoke the temporary-file synchronization callback (production passes `File::sync_all`); recheck the destination and parent; invoke the pre-replacement hook; call `fs::rename`; disarm temporary cleanup; then invoke the parent-directory synchronization callback. On a pre-replacement error, the guard removes the temporary file. The temporary-file sync is before replacement; parent-directory sync is a separate operation after replacement.

The existing `SettingsStorageError::TemporaryFileSynchronizationFailed` variant already identifies the requested failure without exposing settings content. The public `save_settings_atomic` API and production use of `File::sync_all` are unchanged. The private callback is the existing narrow test seam; the test returns an injected error from that callback instead of performing successful synchronization.

## Focused regression test

Test: `storage::tests::atomic_writer_temporary_sync_failure_preserves_valid_settings_and_allows_retry`.

The test creates a unique directory under `std::env::temp_dir()`, writes synthetic valid schema-v1 settings through the public writer, reloads them, captures their exact bytes and permission bits, and prepares a different valid replacement containing synthetic vocabulary. In the synchronization callback, it verifies the write callback completed, confirms exactly one regular temporary replacement exists, reads back bytes identical to the complete expected serialization, and decodes those bytes with `Settings::from_json` as the replacement schema-v1 settings. Reaching this callback proves the writer's separate `flush` step completed first. The callback then returns a controlled `io::Error` in place of `sync_all`.

Assertions verify that synchronization was reached exactly once, `TemporaryFileSynchronizationFailed` is returned, and the pre-replacement callback is not reached; since `fs::rename` occurs after that callback, destination replacement is not attempted. The original destination remains a regular file with byte-for-byte identical contents, the same Unix permission bits, and successfully reloads with its original schema and vocabulary. Directory-entry inspection confirms the exact prepared temporary path was removed and only the original destination remains. A subsequent call to the public writer succeeds, reloads the replacement settings, and leaves no temporary artifact. The test directory is then removed.

The test prints no settings JSON or vocabulary values. Its observed focused output was:

```text
Outflow atomic settings synchronization failure test

Initial settings write: PASS
Temporary replacement creation: PASS
Complete temporary write: VERIFIED
Temporary-file synchronization failure: INJECTED
Writer returned error: PASS
Destination replacement attempted: NO
Original destination bytes: PRESERVED
Original settings reload: PASS
Temporary artifact cleanup: PASS
Subsequent normal write: PASS
New settings reload: PASS

Schema migration performed: NO
Real user settings accessed: NO
Actual OS synchronization failure tested: NO
Crash durability verified: NO
```

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; temporary test directory is on the APFS local journaled data volume; Rust stable 1.99.1. Tests used synthetic settings and an isolated temporary directory only.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings atomic_writer_temporary_sync_failure_preserves_valid_settings_and_allows_retry --locked -- --nocapture` | **PASS** — one focused test; output shown above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings temporary_creation --locked` | **PASS** — Task 57 recovery test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings temporary_write --locked` | **PASS** — Task 56 recovery test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings pre_replacement --locked` | **PASS** — three matching Task 55 recovery/reset tests. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 206 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS** after applying rustfmt to the new test. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 299 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS** — TypeScript check and Vite production build. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS**. |

## Coverage boundaries

Task 58 verifies only a deterministic error at the temporary-file synchronization callback before destination replacement. It does not demonstrate any actual `fsync` failure mode, crash durability, or recovery after power loss. Task 59 separately injects a replacement-operation error after successful temporary-file synchronization; it does not exercise an actual OS-level rename failure. Parent-directory synchronization happens after successful replacement and remains a separate durability boundary. Actual disk-full behavior, OS-level sync or rename failures, Linux filesystem behavior, and Windows ACL/replacement behavior remain untested. This evidence does not complete T-018, T-017, T-038, T-039, T-053, or AC-08. Repository T-058 was not modified.

## Changed files attributable to Task 58

- `crates/settings/src/storage.rs` — added one focused Unix-only sync-failure test; production writer behavior and Task 55–57 tests remain intact.
- `README.md` — documented the Task 58 test, run command, expected output, how the test verifies preservation/cleanup/retry, and limits; clarified the nearby Task 55–56 failure-boundary descriptions.
- `evidence/rust-settings-temporary-write-recovery-task-56.md` — extended the matrix with Task 58 and clarified synthetic callback versus OS-level coverage.
- `evidence/rust-settings-sync-failure-recovery-task-58.md` — this evidence report.

The worktree already contained uncommitted Task 56/57 changes before this task began. Those changes were preserved; Task 57 evidence remains unmodified.
