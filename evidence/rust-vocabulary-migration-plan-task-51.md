# Task 51 — Read-only vocabulary migration planning report

task_id: Task 51 request; repository T-051 remains unchanged
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: Codex
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: uncommitted
platform: macOS 26.6.2 arm64; Rust 1.99.1 stable; Node 22.23.3; Python 3.13.13
implementation: Test-only planner in crates/settings/tests/vocabulary_schema_v2_migration_plan.rs
metrics: NOT_MEASURED; no runtime performance or release resource benchmark is in scope
doc_updates: docs/VOCABULARY_MATCHING_MIGRATION.md, README.md
risks: Trusted provenance, production migration, atomic write, and rollback requirements remain outstanding
review: PENDING

## Scope and references

This focused task follows PRD F-08 and FR-13.4, ARCHITECTURE C-06/C-08, TECH_STACK TS-08, USER_FLOWS UF-06, and the existing T-017, T-018, T-038, T-039 and AC-08 boundaries. The repository task ledger was not changed; T-017, T-018, T-038, T-039, AC-08, and repository T-051 remain incomplete.

The new planner exists only in integration-test scope. It accepts a source Settings value decoded through the supported schema-v1 pathway, a synthetic proposed-v2 JSON fixture, and an explicit VocabularySourcePolicy. It parses the target schema and marker through the existing marker-inspection API, then delegates source assessment to preflight_vocabulary_compatibility and the core readiness implementation.

The report includes source and proposed-target schema versions, the exact recognized target marker identifier, explicit source-provenance classification, readiness status, conflict count, ordered typed conflict records, and migration_applied = false. Its types contain only policy/status values, indices, classifications, and counts. It does not include vocabulary strings or derived comparison keys.

The target declaration and source provenance remain independent. Recognizing unicode18-canonical-caseless-v1 does not change the supplied source policy. Verified source policies in this test are explicit synthetic assumptions. Empty vocabulary means only that no vocabulary-key migration is needed; it does not authorize rewriting the complete settings schema. Conflict-free vocabulary is not migration-approved. Conflict records never select a winner.

## Files created or modified for Task 51

- Created crates/settings/tests/vocabulary_schema_v2_migration_plan.rs.
- Updated docs/VOCABULARY_MATCHING_MIGRATION.md with the planner, decision mapping, decoder boundary, focused command, and remaining migration requirements.
- Updated README.md with a concise Task 51 summary and focused command. README.md already had unrelated uncommitted changes before this task; those changes were preserved.
- Created this evidence file.

No Task 50 shared helper was extracted; the planner uses a small local parser matching the existing test-only contract and calls the production marker inspection API. No production file, Cargo manifest, lockfile, or task ledger was changed by Task 51. Existing dirty Cargo manifests/lockfile and other files belong to pre-existing work and remain untouched.

## Decision mapping

| Core readiness | Planner result |
|---|---|
| EmptyCollection | NoVocabularyMigrationNeeded |
| SourcePolicyUnknown | SourceProvenanceReviewRequired, retaining any conflicts |
| ManualResolutionRequired | ManualConflictResolutionRequired |
| CompatibleForExplicitMigration | CompatibleForFutureMigrationReview |
| AlreadyCurrent | AlreadyCurrentForReview |
| Target contract or source validation error | Typed planning error |

The test covers exact marker recognition; missing, unsupported, wrong-case, and malformed marker values; unsupported target schema; malformed target vocabulary shape; unknown provenance with and without conflicts; explicit verified legacy/current assumptions; sharp-s, canonical-accent, and Greek-sigma collisions; stable multiple-conflict order; 500 valid entries; source preservation; and schema-v2 rejection by the production decoder.

The source boundary is deliberate. Invalid spoken/written values, lowercase collisions, and 501 entries are rejected by Settings::from_json under the existing schema-v1 validation rules. The new tests assert those decoder errors rather than constructing invalid Settings values for the planner or treating it as raw-file recovery. Unicode 18 collisions that pass lowercase validation, such as sharp-s, canonical accents, and sigma, are assessable. Existing Task 48 tests continue to cover direct preflight validation errors on already-constructed invalid values.

## Verification

Commands run on macOS 26.6.2 arm64 with Rust 1.99.1 stable:

| Command | Result |
|---|---|
| RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_schema_v2_migration_plan --locked -- --nocapture | PASS: 25 tests |
| RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked | PASS: 145 unit/integration tests and 1 doc test |
| RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked | PASS: 91 passed, 2 filtered |
| RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check | PASS |
| RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked | PASS |
| RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked | PASS: 238 unit/integration tests and 1 doc test |
| RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings | PASS |
| RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked | PASS |
| npm run typecheck | PASS |
| npm run test -- --run | PASS: 136 tests |
| npm run lint | PASS |
| npm run build | PASS |
| python3 -m unittest discover -s scripts/tests -v | PASS: 258 tests |

The pre-task baseline of 213 Rust unit/integration tests remains passing; this task adds 25 passing integration tests for a total of 238. The existing single settings doc test also passes. No Moonshine CMake configuration was attempted.

The focused --nocapture summary came from actual planner calls and reported:

~~~text
Outflow proposed vocabulary migration plan

Source schema: v1
Proposed target schema: v2
Target policy: unicode18-canonical-caseless-v1

Unknown provenance: REVIEW REQUIRED
Verified legacy, no conflicts: COMPATIBLE FOR REVIEW
Verified legacy, Unicode 18 conflict: MANUAL RESOLUTION
Verified current, no conflicts: ALREADY CURRENT
Empty vocabulary: NO VOCABULARY MIGRATION NEEDED
Unsupported target marker: REJECTED

Original settings modified: NO
Vocabulary entries deleted: NO
Migration applied: NO
Settings files written: NO
Production schema-v2 loading: NOT IMPLEMENTED
~~~

## Production boundary and privacy

Production schema remains v1. Schema-v1 serialization, decoding, lowercase-only validation, initialization, load/save, and reset code were not edited. The production decoder continues to reject schema v2. The Unicode 18 core matching implementation was not edited. No production schema-v2 DTO, loader, migration planner, conflict resolver, writer, or persistence behavior was added.

Only synthetic in-memory values were used. The planner has no filesystem path, file, network, logging, or telemetry input. No user settings were read, and no settings files were written. The report privacy test confirms that formatted report output excludes synthetic spoken and written values.

## Remaining work

Any future persistent migration still requires trusted source-policy provenance, a reviewed complete schema and migration contract, manual conflict review, complete target-settings validation, approved cross-platform atomic-write behavior, recoverable last-known-good state, and interruption/rollback verification. Windows owner-only ACL support and end-to-end rollback acceptance remain outstanding. This report is readiness evidence only and does not complete T-017, T-018, T-038, T-039, AC-08, or repository T-051.
