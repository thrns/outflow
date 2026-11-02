---
task_id: "User Task 50: proposed schema-v2 vocabulary contract fixture"
feature_ids: [F-08, F-11]
flows: [UF-06]
acceptance: [AC-08]
owner: /root (primary agent)
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: uncommitted
platform: macOS 26.6.2, arm64; RAM not independently queried for this task
implementation: Test-only partial proposed schema-v2 envelope and integration contract checks; no production schema, loader, writer, or migration changes.
metrics: NOT_MEASURED (not a performance task)
doc_updates: README.md; docs/VOCABULARY_MATCHING_MIGRATION.md
risks: Trusted policy provenance, approved complete schema migration, manual conflict resolution, cross-platform writer permissions, and rollback/interruption acceptance remain outstanding.
review: PENDING
---

# Task 50 — Proposed schema-v2 vocabulary contract fixture

## Implementation

Created `crates/settings/tests/vocabulary_schema_v2_contract.rs` with test-only Serde DTOs for the partial shape `{ schema_version, vocabulary_matching_policy, vocabulary: [{ spoken, written }] }`. It uses the existing settings crate Serde dependencies and synthetic vocabulary values. Its contract parser checks schema-version boundaries before deserializing the test DTO and never participates in production settings decoding.

The test accepts schema v2 only with the exact marker `unicode18-canonical-caseless-v1`. Missing, unsupported, wrong-case, future-policy-version, unsupported-Unicode-version, empty, and malformed-type markers fail closed. Missing, wrongly typed, v1, and v3 schema-version cases are separately rejected. Missing or malformed vocabulary shapes are rejected; an empty vocabulary array is a valid empty collection.

Read-only core calls validate valid entries with the Unicode 18 matcher and run `assess_vocabulary_migration_readiness` over borrowed `VocabularyEntryRef` values. The fixture detects the new conflicts for `straße` / `STRASSE`, composed U+00E9 / `e` plus U+0301, and `σ` / `ς`, with ordered indices and `NewUnicode18Conflict` classifications. Both original spoken and written strings remain unchanged.

Marker recognition remains separate from source provenance. A conflict-free collection with a recognized marker still returns `SourcePolicyUnknown` when the caller supplies unknown provenance. Verified legacy and verified Unicode 18 outcomes are exercised only with explicitly supplied **test assumptions**, not values inferred from the marker, schema version, or absence of conflicts.

The production regression test asserts `CURRENT_SCHEMA_VERSION == 1`, the schema-v1 default JSON structure, absence of a persisted policy marker, and schema-v1 round-trip stability. The real production decoder rejects schema version 2. The new integration test uses only in-memory JSON and borrowed vocabulary values; it calls no settings file, save, reset, initialization, or migration API. No vocabulary entries are written or changed.

README and the migration compatibility document now describe the isolated fixture, marker and rejection rules, Unicode collision examples, focused command, unchanged production schema v1, and outstanding migration/rollback requirements. `docs/TASKS.md` was not modified; T-017, T-018, T-038, T-039, and AC-08 remain incomplete, and repository T-050 was not changed.

## Verification

The initial worktree was already dirty on branch `codex/task23-moonshine-build-scope`, with unrelated Task 43–49 vocabulary work and other project changes present. That work was preserved. `Cargo.toml`, crate manifests, `Cargo.lock`, and production Rust source received no Task 50 changes.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_schema_v2_contract --locked -- --nocapture` | PASS — 22 integration tests; printed all requested contract lines from asserted outcomes |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | PASS — 87 unit + 11 preflight integration + 22 new integration tests; 1 doc test |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked` | PASS — 91 vocabulary tests; 2 unrelated tests filtered out |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 213 unit/integration tests total (191 prior baseline + 22 new); 1 doc test |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 136 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 258 tests |
| `git diff --check` | PASS |

The focused summary printed:

```text
Outflow proposed schema-v2 contract

Valid marker: ACCEPTED
Missing marker: REJECTED
Unsupported marker: REJECTED
Malformed marker: REJECTED
Unicode 18 vocabulary conflict: DETECTED
Unknown provenance: REVIEW REQUIRED

Production settings schema: v1
Proposed fixture schema: v2
Production schema-v2 loading: NOT IMPLEMENTED
Migration applied: NO
Settings files modified: NO
```

Toolchain used for verification: rustc/cargo 1.99.1 stable, Node.js v22.23.3, Python 3.13.13, macOS 26.6.2 arm64. The repository-pinned toolchain was not invoked; commands used the requested `RUSTUP_TOOLCHAIN=stable` override. The blocked Moonshine CMake configuration was not attempted.

## Remaining work

No persistent schema migration or rollback behavior is implemented. Future work still requires approved complete schema and marker semantics, trusted provenance rules, explicit manual collision resolution, full target-settings validation, atomic writes with platform-specific permissions, last-known-good recovery, and interruption/rollback tests. The schema-v1 production decoder and serializer remain unchanged.
