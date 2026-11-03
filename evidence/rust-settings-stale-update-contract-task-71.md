# Task 71 — Test-only stale settings update contract

```yaml
task_id: "Task 71 request; repository T-071 is installer packaging and was not modified"
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: "Codex coordinator in the existing codex/task23-moonshine-build-scope worktree"
base_commit: "f4dd938bd7220395b39dcea7538fdcadec14d71d"
result_commit: "uncommitted"
platform: "macOS 26.6.2 (Darwin 25.6.0 build 25G83), arm64; Rust stable 1.99.1"
implementation: "Added a Unix-gated integration-test-only stale snapshot assessment; documented its limits in README and the settings recovery matrix"
metrics: "NOT_MEASURED; no performance or release metrics are in scope"
doc_updates: ["README.md: Task 71 stale-snapshot contract", "evidence/rust-settings-temporary-write-recovery-task-56.md: recovery matrix", "this evidence"]
risks: "The check is an inspection-time equality observation and is not an atomic write precondition; production stale-write prevention remains unimplemented"
review: PENDING
```

## Inspection and scope

Inspected the repository-root `AGENTS.md` (no nested `AGENTS.md` or `AGENTS.override.md`), `docs/PRD.md` F-08 and AC-08, `docs/ARCHITECTURE.md` C-08, `docs/TECH_STACK.md` TS-08, and `docs/TASKS.md` T-017/T-018/T-038/T-039/T-053/T-071. Also inspected `crates/settings/src/lib.rs`, `crates/settings/src/storage.rs`, `crates/settings/tests/settings_lost_update_audit.rs`, `crates/settings/tests/settings_post_error_inspection.rs`, Task 70 evidence, the recovery matrix, the README, and the workspace/package manifests and lockfile.

Worktree: `/Users/bixxie/Desktop/Projects/outflow`, branch `codex/task23-moonshine-build-scope`, base commit `f4dd938bd7220395b39dcea7538fdcadec14d71d`. The Task 70 README/recovery-matrix edits and its untracked test/evidence were already present at task start and were preserved. All work remains uncommitted.

The existing `Settings` derives `PartialEq` and `Eq` across its complete schema-v1 fields, including ordered `Vec<VocabularyEntry>`. The public `load_settings` API returns a validated `Settings`, `None` for a missing path, or a structured error. The public `save_settings_atomic` writer accepts a complete snapshot and does not compare it with the current file. No production API, writer, schema, dependency, manifest, or lockfile was changed.

## Contract and observed scenarios

The test-local `assess_stale_update(expected, observed_current)` returns `MatchesObservedCurrent` when the complete settings values compare equal and `RejectedStaleSnapshot` otherwise. The schema version is included as one normal field but is not treated as a concurrency revision. Vocabulary strings and other settings values are not included in the assessment result.

The independent-edit fixture writes synthetic A, loads A for both actors, prepares B by changing `cleanup.mode`, and prepares C by changing `startup.run_at_login`. Actor 1 writes B through the existing public writer. After loading B, the test compares actor 2's expected A with B and asserts `RejectedStaleSnapshot`; it reloads and confirms B remains unchanged. Actor 2's stale write is never submitted.

A separate refreshed-snapshot fixture writes A, writes B, reloads B for actor 2, reloads the observed current value, and asserts `MatchesObservedCurrent`. This is equality at the comparison point only; the test does not imply a safe later write or compare-and-swap.

Two valid schema-v1 settings values that differ only in their synthetic vocabulary are classified stale despite sharing the same schema version. Reversing two distinct valid entries is also stale, proving the existing equality compares vocabulary order. A preference change with schema version 1 on both values is independently asserted stale.

The A → B → A fixture captures A, writes different B, writes A again, reloads A, and gets `MatchesObservedCurrent`. This equality cannot detect the intervening history (the ABA limitation); no revision mechanism was introduced.

The loader wrapper issues no positive assessment for `Ok(None)` or an error. The test checks a missing path, malformed JSON, and unsupported schema v2 directly through `load_settings`; none is substituted with defaults. The summary test repeats assessments for determinism, checks that result formatting contains no vocabulary values, verifies in-memory inputs are unchanged, and compares synthetic settings-file bytes before and after pure assessments.

## Verification

Environment: macOS 26.6.2, Darwin 25.6.0 build 25G83, arm64; `rustc 1.99.1`, `cargo 1.99.1`, Node `v22.23.3`, Python `3.13.13`. All test settings and files were synthetic and placed in unique temporary directories. No real user settings path was accessed. This integration test is Unix-gated because the existing public writer returns `UnsupportedPlatform` on non-Unix targets; Windows was not tested.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_stale_update_contract --locked -- --nocapture` | **PASS** — 10 tests; printed the requested content-free assertion-backed summary. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test settings_lost_update_audit --locked` | **PASS** — 4 Task 70 tests. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked --quiet` | **PASS** — 259 unit/integration tests; 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS** — 352 unit/integration tests; 1 doc test. This is the 342-test Task 70 baseline plus 10 new tests. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS**. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS**. |
| `npm run typecheck` | **PASS**. |
| `npm run test -- --run` | **PASS** — 136 tests across 10 files. |
| `npm run lint` | **PASS**. |
| `npm run build` | **PASS**. |
| `python3 -m unittest discover -s scripts/tests -v` | **PASS** — 258 tests. |
| `git diff --check` | **PASS** after the final documentation edits. |

## Limits and remaining work

The contract is a test-only classification rule. Production stale-write protection, file locking, atomic compare-and-write, revision counters, automatic merge/retry, and rollback are **NOT IMPLEMENTED**. A matching value does not authorize a later write because another actor may modify settings after the comparison. The existing writer still permits ordered last-writer-wins updates. No simultaneous or cross-process write schedule was tested.

Repository T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. Repository T-071 (installer packaging) was not modified.
