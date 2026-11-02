---
task_id: Focused Task 54 (separate from repository T-054)
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: Codex session on the existing shared branch
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: uncommitted
platform: macOS 26.6.2 arm64; RAM not measured (system query denied)
implementation: Test-only review lifecycle in crates/settings/tests/vocabulary_resolution_proposal.rs
doc_updates: README.md; docs/VOCABULARY_MATCHING_MIGRATION.md
risks: Persistent settings migration and filesystem rollback remain unimplemented and untested
review: PENDING
---

# Task 54 — Vocabulary review cancellation and retry

## Scope and implementation

The existing Task 53 validator remains the only proposal validator. The review helper exists only in `crates/settings/tests/vocabulary_resolution_proposal.rs` and holds an immutable borrow of the original `VocabularyEntryRef` collection plus a separate partial `Vec<ResolutionDecision>` draft.

- A decision can be added or revised by original index; no winner is inferred and the original collection is not reordered, edited, or deleted.
- Validation calls `validate_resolution_proposal(...)`. Incomplete decisions, out-of-range indices, duplicates, invalid entries, and remaining Unicode 18 conflicts remain typed proposal errors.
- Failed validation leaves the review active and draft unchanged, so a corrected proposal can be retried through the same Unicode 18 validator.
- Cancellation clears pending decisions and transitions the session to terminal `Cancelled`. Repeated cancellation returns `AlreadyCancelled`; validation/editing after cancellation return typed lifecycle errors. A new session starts with a fresh empty draft.
- The helper exposes no settings, migration, or filesystem operations. No production code, schema, settings writer, manifest, or lockfile was changed for this task.

Synthetic preservation checks include ASCII, German sharp-s, composed/decomposed accents, Greek sigma variants, and different written replacements. Tests compare a separate deep owned snapshot, including every spoken/written value at its original position, after drafting, successful and failed validation, corrections, cancellation, repeated cancellation, discarding a cancelled session, and starting a fresh review. No vocabulary values are printed.

## Verification

All listed commands passed on macOS 26.6.2 arm64. No CMake configuration was attempted; the workspace contains only `outflow-core` and `outflow-settings`.

Focused and related Rust checks:

- `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_resolution_proposal --locked -- --nocapture` — **PASS**, 34 tests; 10 added for Task 54.
- `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_conflict_review_contract --locked` — **PASS**, 23 tests.
- `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_schema_v2_migration_plan --locked` — **PASS**, 25 tests.
- `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_preflight --locked` — **PASS**, 11 tests.
- `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` — **PASS**, 202 unit/integration tests and 1 doc test.

Full Rust regression:

- `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` — **PASS**.
- `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` — **PASS**.
- `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` — **PASS**, 295 unit/integration tests and 1 doc test. This is the stated 285-test baseline plus 10 new tests; the core doc-test target ran 0 tests and the settings doc-test target ran 1.
- `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` — **PASS**.
- `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` — **PASS**.

Frontend and Python regression:

- `npm run typecheck` — **PASS**.
- `npm run test -- --run` — **PASS**, 136 tests across 10 files.
- `npm run lint` — **PASS**.
- `npm run build` — **PASS**.
- `python3 -m unittest discover -s scripts/tests -v` — **PASS**, 258 tests.

The `--nocapture` demonstration printed:

~~~text
Outflow vocabulary review cancellation checks

Incomplete draft cancellation: PASS
Invalid proposal rejected: PASS
Corrected proposal validation: PASS
Cancellation after validation: PASS
Repeated cancellation: IDEMPOTENT
Original vocabulary: UNCHANGED
Draft decisions after cancellation: DISCARDED

Settings files modified: NO
Migration applied: NO
Persistent rollback tested: NO
Production schema-v2 support: NOT IMPLEMENTED
~~~

The `PASS` outcomes follow assertions over real session operations and proposal-validator calls. The test helper did not read or write settings files and produced no migration result. `Settings` production schema remains v1, which continues to reject schema v2.

## Rollback boundary and remaining requirements

**Task 54 verifies pre-commit cancellation and in-memory recovery invariants only. Actual filesystem rollback: NOT TESTED.** Existing settings-writer tests, including injected failure tests, do not prove a future schema-v2 migration can roll back safely.

Persistent migration remains blocked on an approved production schema/migration and writer requirements, including temporary-file write failure, real disk-full behavior, permission denial, failures before and during platform-specific replacement, crash/interruption recovery, preservation of valid last-known settings, file-permission preservation, Windows replacement and owner-only ACL behavior, Linux filesystem testing, and unsupported-schema downgrade handling. Windows writer support and end-to-end migration rollback remain incomplete. Trusted provenance, migration authorization, vocabulary CRUD, and production schema-v2 load/save support also remain prerequisites.

T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. Repository T-054 (onboarding demo flow) was not changed. No task ledger status was edited.

## Git handoff

Branch: `codex/task23-moonshine-build-scope`. The worktree was already dirty before this task, including user-owned vocabulary work and uncommitted changes to `Cargo.lock`, Cargo manifests, README, and architecture/stack documentation. Those pre-existing changes were preserved. Task 54 touched only the existing proposal integration test, this evidence file, `README.md`, and `docs/VOCABULARY_MATCHING_MIGRATION.md`. No stage, commit, push, merge, or history rewrite was performed.

Persistent rollback checks are **NOT TESTED**; continue only after the migration/schema and platform-specific writer prerequisites are approved and defined.
