---
task_id: "Task 46 — vocabulary matching compatibility checker (separate from repository T-046)"
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: "Codex; existing worktree codex/task23-moonshine-build-scope"
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: uncommitted
platform: "macOS 26.6.2 arm64; Rust stable 1.99.1"
implementation: "Added pure outflow_core::vocabulary_migration::inspect_vocabulary_migration, 24 focused tests, synthetic example, README section, and migration-boundary documentation."
commands:
  - {command: "RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked", result: PASS, notes: "76 passed; 0 failed; 2 filtered out. This includes vocabulary, matching, and migration tests."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_migration_check --locked", result: PASS, notes: "Output recorded below; all result categories and conflict indices came from checker calls."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check", result: PASS, notes: "No formatting differences."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked", result: PASS, notes: "Workspace checked."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked", result: PASS, notes: "148 Rust tests passed: 78 core and 70 settings. The new module adds 24 tests; the supplied pre-task baseline of 124 remains passing."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings", result: PASS, notes: "No warnings."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked", result: PASS, notes: "Workspace build completed."}
  - {command: "npm run typecheck", result: PASS, notes: "No type errors."}
  - {command: "npm run test -- --run", result: PASS, notes: "136 tests passed across 10 files."}
  - {command: "npm run lint", result: PASS, notes: "ESLint completed without findings."}
  - {command: "npm run build", result: PASS, notes: "Typecheck and Vite production build completed."}
  - {command: "python3 -m unittest discover -s scripts/tests -v", result: PASS, notes: "258 tests passed."}
  - {command: "git diff --check", result: PASS, notes: "Final reviewed diff had no whitespace errors."}
metrics: "No performance, installer-size, or memory metrics required or measured."
doc_updates: "README.md and docs/VOCABULARY_MATCHING_MIGRATION.md; task ledger and schema unchanged."
risks: "Settings persistence still validates lowercase-only and has no matching-policy marker or migration. Schema/build lineage must be established before persistent integration; conflicts require manual review."
review: PENDING
---

# Task 46 evidence — vocabulary migration compatibility

## Implementation

Added `crates/core/src/vocabulary_migration.rs` and exported it from `crates/core/src/lib.rs`. The checker enforces the existing 500-entry and per-field validation rules, then compares every spoken pair using actual Rust `to_lowercase()` and the existing ICU4X Unicode 18.0.0 comparison API. It returns stable pairwise index records classified as existing legacy conflicts or new Unicode 18 conflicts. Typed validation errors and diagnostics contain no entry text. Comparison keys are temporary; original strings and replacements remain unchanged.

The checker does not call the production collection validator, load settings, accept a path, persist keys, select winners, delete entries, or rewrite configuration. The existing production validator was not edited during this task. Its ICU4X integration was already present as uncommitted work when this task began.

## Classification results

Synthetic test cases produced these classifications:

- ASCII case variants and `ÉCOLE` / `école`: `ExistingLegacyConflict`.
- `straße` / `STRASSE`, composed/decomposed `é`, and `σ` / `ς`: `NewUnicode18Conflict`.
- `I` / `ı`, `İ` / `i`, Latin `a` / Cyrillic `а`, and a phrase with/without trailing space: no conflict.
- A mixed five-entry fixture reported `(0,1)` new, `(0,2)` existing, `(1,2)` new, then `(3,4)` new, preserving input pair order.

The suite checks empty and valid collections, exactly 500 and rejected 501 entries, invalid spoken and written fields, 128-scalar limits, different replacements, typed reasons and indices, redaction, repeatability, text preservation, and no automatic resolution or mutation. No user settings were read or written.

## Example output

```text
Outflow vocabulary migration compatibility check
Source policy: Legacy Rust lowercase
Target policy: Unicode 18.0.0 canonical caseless V1
Unchanged collection: COMPATIBLE
Existing legacy conflict: DETECTED
New Unicode 18 conflict: DETECTED
Multiple new conflicts: DETECTED
Conflict indices: VERIFIED
Migration applied: NO
Settings files modified: NO
User entries deleted: NO
Manual conflict resolution: REQUIRED
```

The executable uses only synthetic entries and prints no vocabulary text.

## Settings compatibility findings

The existing `crates/settings` schema is v1 and contains a vocabulary field but no matching-policy field. `validate_vocabulary` compares `spoken.to_lowercase()`. Validation occurs from `Settings::validate`, JSON decoding through `from_json`/`from_wire`, custom `Deserialize`, existing-file loading through `load_settings`, initialization, and validation before JSON encoding/atomic save. Missing files return validated defaults in memory; they are not written during initialization. There is no migration implementation; unsupported schema versions are rejected.

Schema v1 records data shape, not the matching algorithm used by every possible writer. A missing policy field alone cannot identify a file's matching lineage. A future migration needs trusted schema/build evidence tied to the implementation that validated the file, or an approved explicit policy marker/schema decision. Unrecognized lineage or unresolved conflicts must fail safely and request review. T-017, T-018, T-038, T-039, and AC-08 remain incomplete; repository T-046 remains the copy-only result panel.

## Scope and remaining risks

Created or updated for this request: the migration checker source, `lib.rs` export, synthetic example, `docs/VOCABULARY_MATCHING_MIGRATION.md`, README section, and this evidence. No dependency or lockfile change was made by this task. The worktree was already dirty with Task 45 ICU4X changes and other user work; those changes were preserved. Current settings compatibility, persistence migration, schema/build provenance, and user review/rollback behavior remain future work. No settings integration or task-status update was performed.

## Verification record

The full verification commands and results are listed in the YAML header. Final worktree review found no settings, manifest, task-ledger, or lockfile changes introduced by this request; pre-existing dirty files remain preserved. Python test-generated `__pycache__` directories were removed after the suite. `git diff --check` passed after the evidence file was added. One initial focused run found an incorrect test expectation indexing the second reported conflict; the test was corrected and the focused command then passed.
