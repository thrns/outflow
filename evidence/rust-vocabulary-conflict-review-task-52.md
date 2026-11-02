# Task 52 — Read-only vocabulary conflict review groups

```yaml
task_id: Task 52 requested scope; repository T-052 unchanged
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: Codex on existing worktree codex/task23-moonshine-build-scope
base_commit: 6f15c8f
result_commit: uncommitted working tree
platform: macOS 26.6.2, arm64; CPU model and installed RAM not independently verified because sysctl access was denied
implementation: Test-only connected conflict grouping in crates/settings/tests/vocabulary_conflict_review_contract.rs; appended contract documentation to README.md and docs/VOCABULARY_MATCHING_MIGRATION.md
metrics: Not measured; no performance or release gate was part of this task
doc_updates: README.md conflict review section; docs/VOCABULARY_MATCHING_MIGRATION.md future manual review contract; repository docs/TASKS.md intentionally unchanged
risks: No production review API, UI, CRUD, persistent migration, or schema-v2 decoder exists. Trusted provenance, approved migration/schema behavior, validated CRUD, atomic persistence, recoverable originals, and cancellation behavior remain prerequisites.
review: PENDING
```

## Implementation

The new integration-test-only `ConflictReviewGroup` contains sorted unique `entry_indices`, the original `VocabularyMigrationConflict` records, and `manual_review_required: true`. The helper builds an adjacency map, walks connected components, sorts members and groups by their smallest index, and copies every matching original pair in input order. Duplicate pair records remain present in `conflicts` while their endpoints appear once in `entry_indices`. The helper accepts only pair reports, so it cannot inspect or choose vocabulary values and cannot infer source provenance.

The contract tests use synthetic reports from the existing core checker. They cover the German sharp-s pair, composed/decomposed accent pair, Greek sigma pair, legacy and Unicode 18 classifications, mixed classifications, transitive components, independent groups, deterministic output, privacy, and non-mutation. A separate decoder assertion confirms production settings still reject schema version 2. No production source, settings API, manifest, lockfile, matching algorithm, schema, or task ledger was changed for this task.

The focused demonstration prints only counts/statuses and matches the requested output:

```text
Outflow vocabulary conflict review contract

Single conflict: 1 GROUP
Three connected entries: 1 GROUP
Two independent conflicts: 2 GROUPS
Group ordering: VERIFIED
Conflict classifications: PRESERVED
Original vocabulary: UNCHANGED

Automatic winner selected: NO
Conflict resolution applied: NO
Settings files modified: NO
Production schema-v2 support: NOT IMPLEMENTED
```

The review report and its debug representation contain indices and conflict metadata only. The privacy test confirms that spoken phrases, written replacements, and a normalized comparison key are absent. The projection tests use in-memory synthetic values and do not call settings storage or migration operations; no user settings file was read or written. The original in-memory vocabulary and settings serialization remain unchanged.

## Verification

All listed final commands passed on the platform above. The full workspace totals are **261 unit/integration tests** (238 existing plus 23 added) and **1 settings doc test**; `outflow-core` has no doc tests.

```text
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_conflict_review_contract --locked -- --nocapture  PASS (23)
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_schema_v2_migration_plan --locked  PASS (25)
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_preflight --locked  PASS (11)
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary_migration --locked  PASS (39)
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check  PASS
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked  PASS
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked  PASS (261 unit/integration; 1 doc test)
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings  PASS
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked  PASS
npm run typecheck  PASS
npm run test -- --run  PASS (136)
npm run lint  PASS
npm run build  PASS
python3 -m unittest discover -s scripts/tests -v  PASS (258)
git diff --check  PASS
Untracked-file whitespace scan for the new test, migration doc, and evidence  PASS
```

The first focused compile and first Clippy run exposed issues in the new test file; those were corrected and the final focused test, workspace test, and Clippy runs passed. The first format check also requested formatting; `cargo fmt --all` was applied before the final passing format check.

## Remaining requirements

This evidence does not complete T-017, T-018, T-038, T-039, or AC-08. Repository T-052 remains unchanged. Future work needs trusted source-policy provenance, explicit local user decisions, cancellation that preserves all originals, complete collection validation, separately validated edit/delete CRUD, an approved versioned migration, atomic persistence, and recoverable original configuration. No hardware or release metrics were measured.
