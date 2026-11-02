# Task 53 — Test-only vocabulary resolution proposal validator

```yaml
task_id: T-053
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: Codex / task work in existing worktree
branch: codex/task23-moonshine-build-scope
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: uncommitted
platform: macOS 26.6.2 arm64; Rust 1.99.1 stable
review: PENDING
```

## Implementation

Created [`crates/settings/tests/vocabulary_resolution_proposal.rs`](../crates/settings/tests/vocabulary_resolution_proposal.rs). Its private test-only model is `ProposedDisposition::{Retain, OmitFromProposedCollection}`, `ResolutionDecision { original_index, disposition }`, `ResolutionProposalAssessment`, and `ResolutionProposalError`.

The helper first calls the existing core migration inspector to preserve typed original size and field-validation errors. The inspector's original conflict report is deliberately not a rejection gate. It then rejects out-of-range and duplicate decision indices, rejects the first missing original index, and constructs borrowed retained entries by iterating original positions. Candidate positions map to original indices. The existing `validate_vocabulary_collection` checks the hypothetical target; typed size/field errors are preserved and any duplicate error becomes an index-only Unicode 18 conflict with original indices.

The assessment reports the original count and retained/omitted original indices in source order. It carries no provenance, migration authorization, or persistence result. The helper accepts only borrowed vocabulary and explicit decisions; it has no settings, filesystem, CRUD, or writer interface. Original strings are not normalized or changed, and omission never performs deletion.

## Test coverage and synthetic results

The focused integration test adds 24 passing tests for empty and valid proposals, explicit omissions, connected and independent conflicts, unresolved conflicts, sharp-s, composed/decomposed accents, Greek sigma, missing/duplicate/out-of-range decisions, decision-order stability, original-index remapping, source preservation, the 500-entry cap, original field errors, all-omitted proposals, metadata-only outcomes, and redacted typed errors. Its `--nocapture` summary is generated after validation calls and assertions. It prints no vocabulary values.

Synthetic outcomes: two-entry conflict reduced to one retained entry — valid; three connected entries reduced to one — valid; two conflicting retained entries — rejected; incomplete and duplicate decisions — rejected. A conflict at candidate positions 0 and 1 after omissions correctly reports original indices 1 and 3. The owned source strings and original entry count remain unchanged.

No settings were loaded or written; no entries were deleted; no migration was authorized. No production schema-v2 loading, production API, settings schema, existing settings duplicate validation, ICU4X matching algorithm, Cargo manifest, or lockfile was changed by this task. The pre-existing uncommitted changes to those paths remain preserved. No Task ledger status was changed.

## Verification

Commands run from the repository root; all results were PASS:

```text
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_resolution_proposal --locked -- --nocapture  (24 passed)
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_conflict_review_contract --locked  (23 passed)
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_schema_v2_migration_plan --locked  (25 passed)
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_preflight --locked  (11 passed)
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked  (91 passed; 2 filtered)
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked  (285 unit/integration passed; 1 doc test passed)
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run  (136 passed)
npm run lint
npm run build
python3 -m unittest discover -s scripts/tests -v  (258 passed)
```

The workspace has the stated 261 existing Rust unit/integration tests plus this test's 24, for 285 passing; its existing single doc test also passes. The frontend remains at 136 passing tests and Python at 258. Release size, process RSS, hardware, and cross-platform gates were not measured by this task.

## Documentation and remaining work

Updated `docs/VOCABULARY_MATCHING_MIGRATION.md` and `README.md` with the decision contract, required explicit decisions, source-order projection, Unicode 18 reuse, original-index errors, focused command, and the distinction between validation and migration authorization.

Future real conflict resolution still requires manual review and explicit user intent, trustworthy source-policy provenance, approved complete schema migration, target validation, and tested atomic persistence, recovery, cancellation, and rollback. Settings CRUD and UI remain separate work. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. The centralized task ledger was not modified.
