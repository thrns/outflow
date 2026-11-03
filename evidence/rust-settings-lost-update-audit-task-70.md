# Task 70 — Settings lost-update audit

**Scope:** Deterministic audit tests for ordered writes of two complete settings snapshots prepared from one earlier read. This task is separate from repository T-070, which concerns resource release gates. No production concurrency mechanism or settings API change was made.

## Repository and requirements inspected

- Worktree: `/Users/bixxie/Desktop/Projects/outflow`, branch `codex/task23-moonshine-build-scope`.
- Base commit observed before edits: `f4dd938bd7220395b39dcea7538fdcadec14d71d`. Result remains uncommitted.
- Root `AGENTS.md`; no nested `AGENTS.md` or `AGENTS.override.md` was present.
- `docs/PRD.md` F-08 persistence and §7 failure/recovery; `docs/ARCHITECTURE.md` C-08 and §7 persisted data; `docs/TECH_STACK.md` TS-08; `docs/USER_FLOWS.md` UF-10/UF-11; `docs/TASKS.md` T-017/T-018.
- `crates/settings/src/lib.rs`, `crates/settings/src/storage.rs`, `crates/settings/tests/settings_post_error_inspection.rs`, `crates/settings/tests/settings_persistence_guidance_contract.rs`, and `evidence/rust-settings-snapshot-boundaries-task-69.md`.
- Repository manifests and lockfiles were inspected. The Task 70 diff changes no manifest or lockfile.

## Existing writer contract found

The public interface is `save_settings_atomic(path: &Path, settings: &Settings)`. It accepts a complete `Settings` snapshot, validates and serializes that object, writes a same-directory temporary file, and atomically replaces the destination on Unix. Its public API does not accept an update patch or expected revision.

The implementation does not read the current destination to compare it with the supplied snapshot, reject stale values, merge fields, or expose a typed stale-write conflict. `schema_version` is validated as the document-format version; it is not a concurrency revision. The writer has no settings mutex or file-lock operation, and the repository search found no settings-write serialization or compare-and-swap mechanism. The writer's use of a temporary file, synchronization, and rename concerns atomic file integrity; it does not isolate logical updates.

The settings crate has no production application caller of this writer at present. Repository references outside its implementation are examples, tests, and documentation; `initialize_settings` only loads and validates. This audit therefore establishes the public writer's behavior, not an application's current multi-writer schedule. The storage error type has no stale-update conflict variant.

## Test fixture and ordered operations

The new test target is `crates/settings/tests/settings_lost_update_audit.rs`. It is Unix-gated because the current public writer returns `UnsupportedPlatform` on non-Unix targets. Each test creates an isolated directory under `std::env::temp_dir()` and uses only synthetic schema-v1 settings.

Initial A is `Settings::default()`. Independent fields are `cleanup.mode` (X) and `startup.run_at_login` (Y). Actor 1 and actor 2 load A separately. Actor 1 changes only X in B; actor 2 changes only Y in C. Both complete snapshots pass validation and retain schema v1. No fixture values or settings documents are printed.

| Scenario | Explicit operation order | Observed final settings |
|---|---|---|
| Actor 1 first | Both load A; write B; reload; write stale C; reload | C is accepted. Y reflects actor 2; X has returned to A. Actor 1's logical update is lost. |
| Actor 2 first | Both load A; write C; reload; write stale B; reload | B is accepted. X reflects actor 1; Y has returned to A. Actor 2's logical update is lost. |
| Fresh read | Write B; actor 2 loads current B; actor 2 changes Y and writes; reload | Both X and Y changes are present in the controlled sequence. |

The stale writes were actually accepted by `save_settings_atomic` and returned success. The tests assert the resulting complete settings snapshots, including the field that reverted. This is a lost logical update, not physical file corruption. The fresh-read case is not transaction isolation: another writer could still update the destination after the read and before the write.

## Integrity, privacy, and scope boundaries

After each write, assertions reload the complete settings through the public loader, compare it with the expected full snapshot, validate schema v1, decode the raw document with `Settings::from_json`, check Unix permissions are `0600`, and verify that the isolated directory contains only `settings.json`. The directory guard removes each synthetic fixture after the test. No vocabulary, audio, transcript, clipboard, or real user settings are used.

**Atomic file integrity:** Existing writer failure and replacement tests passed as part of the settings crate and workspace suites. The new tests also found complete readable JSON after each ordered write.

**Lost-update prevention:** Not implemented. Current stale complete-snapshot writes are accepted and last-writer-wins.

**Simultaneous filesystem writes:** Not tested. There are no threads, sleeps, overlapping write calls, or external processes in this task. Linearizability, simultaneous rename safety, and cross-process exclusion are not established.

**Production concurrency solution:** Not implemented. No locking, versioning, compare-and-swap, merge, retry, recovery snapshot, schema change, persistence architecture, or public API was added.

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0, arm64; Rust stable 1.99.1; Node v22.23.3; Python 3.13.13. The test temporary directory is on the local APFS data volume. Windows and Linux were not tested.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_lost_update_audit --locked -- --nocapture` | **PASS** — 4 tests. Printed the assertion-backed, content-free audit summary; both stale writes succeeded, both write orders were verified, and the fresh-read sequence retained both changes. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_post_error_inspection --locked` | **PASS** — 17 Task 69 tests. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_persistence_guidance_contract --locked` | **PASS** — 14 Task 67 tests. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | **PASS** — 249 unit/integration tests and 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked --quiet` | **PASS** — same 249 unit/integration tests and 1 doc test; used to verify the total. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS** after applying `cargo fmt --all` to the new test file. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 342 unit/integration tests and 1 doc test. Baseline was 338 unit/integration tests plus 1 doc test; Task 70 adds four tests. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS**. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** after final documentation and test edits. |

The full application integration and resource release gates were not part of these checks. The blocked Moonshine CMake configuration was not attempted.

## Files changed and remaining requirements

- Added `crates/settings/tests/settings_lost_update_audit.rs` with three deterministic scenarios and one assertion-backed summary test.
- Updated `README.md` with the atomicity/lost-update distinction, observed stale-write behavior, focused command, and limitations.
- Added a Task 70 row to the existing `evidence/rust-settings-temporary-write-recovery-task-56.md` recovery matrix.
- No production source, settings schema, task ledger, dependency, manifest, or lockfile changed.

The recovery matrix and README distinguish atomic file integrity from lost-update prevention and simultaneous writes. Overlapping writes, cross-process behavior, crash durability, Linux filesystem behavior, and Windows replacement/ACL behavior remain outstanding. Repository T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete; repository T-070 was not modified.
