---
task_id: "Task 47 — vocabulary migration readiness (separate from repository T-047)"
feature_ids: [F-08, F-13]
flows: [UF-06]
acceptance: [AC-08]
owner: "Codex; existing worktree codex/task23-moonshine-build-scope"
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: uncommitted
platform: "macOS 26.6.2 arm64; Rust stable 1.99.1; available RAM not measured (system query denied)"
implementation: "Added pure outflow_core::vocabulary_migration::assess_vocabulary_migration_readiness with explicit source-policy provenance, readiness outcomes, focused tests, synthetic example output, docs and README guidance."
commands:
  - {command: "RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary_migration --locked", result: PASS, notes: "39 passed; 0 failed; 54 filtered out. Includes the existing checker tests and 15 new readiness tests."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_migration_check --locked", result: PASS, notes: "Existing conflict report remains; all five readiness labels match the requested output and come from assessment calls."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check", result: PASS, notes: "No formatting differences after formatting the changed Rust files."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked", result: PASS, notes: "Workspace checked."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked", result: PASS, notes: "163 Rust tests passed: 93 core and 70 settings. This is 15 new tests over the 148-test Task 46 snapshot."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings", result: PASS, notes: "No warnings."}
  - {command: "RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked", result: PASS, notes: "Workspace build completed."}
  - {command: "npm run typecheck", result: PASS, notes: "No type errors."}
  - {command: "npm run test -- --run", result: PASS, notes: "136 tests passed across 10 files."}
  - {command: "npm run lint", result: PASS, notes: "ESLint completed without findings."}
  - {command: "npm run build", result: PASS, notes: "Typecheck and Vite production build completed."}
  - {command: "python3 -m unittest discover -s scripts/tests -v", result: PASS, notes: "258 tests passed."}
  - {command: "git diff --check", result: PASS, notes: "Final diff whitespace check passed."}
metrics: "No performance, installer-size, or memory metrics required or measured. The host RAM query was denied by the environment."
doc_updates: "README.md and docs/VOCABULARY_MATCHING_MIGRATION.md updated; Task 47 evidence added. Settings implementation, docs/TASKS.md, manifests, and lockfiles were not changed by this task."
risks: "Settings schema v1 has no matching-policy marker and its validator still uses lowercase-only uniqueness. Schema version alone is not provenance. Settings integration and recoverable migration remain future work; no AC-08 or repository task completion is claimed."
review: PENDING
---

# Task 47 evidence — vocabulary migration readiness

## Scope and files

The worktree was already dirty with Task 45/46 vocabulary work and unrelated project work. Those changes were preserved. Task 47 edited:

- `crates/core/src/vocabulary_migration.rs`
- `crates/core/examples/vocabulary_migration_check.rs`
- `docs/VOCABULARY_MATCHING_MIGRATION.md`
- `README.md`
- `evidence/rust-vocabulary-readiness-task-47.md` (this file)

No settings implementation, task ledger, Cargo manifest, lockfile, frontend, or Python file was changed for this task. Repository T-047 remains untouched; T-017, T-018, T-038, T-039, and AC-08 are not marked complete.

## Public API and decisions

`outflow_core::vocabulary_migration::assess_vocabulary_migration_readiness(&[VocabularyEntryRef], VocabularySourcePolicy)` returns `Result<VocabularyMigrationReadiness, VocabularyMigrationError>`. It calls `inspect_vocabulary_migration` for the existing 500-entry cap, entry validation, and complete stable conflict report. It does not accept a settings path, mutate borrowed entries, or persist comparison keys.

`VocabularySourcePolicy::Unknown` represents absent or untrusted provenance. The verified legacy and Unicode 18 variants are caller assertions that must be backed by trusted evidence. Schema v1 is not such evidence and is not accepted as an input from which the function infers policy.

Decision rules:

- Any valid empty collection returns `EmptyCollection`, independent of source policy. This is only about vocabulary entries and does not authorize rewriting the containing settings file.
- A nonempty unknown-provenance collection returns `SourcePolicyUnknown`, retaining any checker conflicts. No conflicts does not make it eligible for automatic migration.
- Verified legacy or current data with any reported conflict returns `ManualResolutionRequired`, preserving pair indices and `ExistingLegacyConflict` / `NewUnicode18Conflict` classifications.
- Verified legacy data with no conflicts returns `CompatibleForExplicitMigration`; it is not persisted or automatically migrated.
- Verified Unicode 18 V1 data with no conflicts returns `AlreadyCurrent`.
- Size and entry validation errors are passed through from the existing checker in deterministic order.

The focused additions exercise unknown provenance with and without conflicts; empty collections under all three provenance variants; compatible, already-current, and manual-resolution outcomes; sharp-s, canonical-accent, sigma, Turkish dotted/dotless-I, and cross-script cases; invalid spoken/written fields; the 500/501 boundary; stable indices/classifications; input preservation and repeatability; and the absence of a persistence input.

## Demonstration readiness output

```text
Vocabulary migration readiness
Verified legacy, no conflicts: COMPATIBLE
Verified legacy, Unicode 18 conflict: MANUAL RESOLUTION
Verified current, no conflicts: ALREADY CURRENT
Unknown source policy: REVIEW REQUIRED
Empty collection: NO VOCABULARY MIGRATION NEEDED

Settings schema modified: NO
Migration applied: NO
Vocabulary values modified: NO
```

The synthetic example retains Task 46's conflict-reporting section. Each readiness label is based on a real call to the assessment function; synthetic strings are never printed.

## Settings limitations and remaining work

Inspection of `crates/settings/src/lib.rs` confirms schema v1 has no matching-policy field, unsupported schema versions are rejected, and `validate_vocabulary` compares `spoken.to_lowercase()`. Therefore settings validation remains lowercase-only and is not Unicode 18-compatible. There is no settings migration implementation, and this readiness API is not connected to load/save.

Future work must establish trusted policy provenance, require explicit conflict review while preserving all entries, and wait for a separately approved schema migration before adding a marker or changing persistent behavior. That migration must revalidate the full collection under Unicode 18, preserve original strings, write atomically, retain a recoverable last-known-good file, and verify rollback on write failure. These are architectural requirements, not functionality delivered by this task.

## Verification

All Rust format/check/test/Clippy/build commands in the YAML header passed. The workspace has 163 passing Rust tests (93 core and 70 settings), including the 15 new readiness tests. Frontend verification passed with 136 tests; Python verification passed with 258 tests. No frontend changes were needed. No installer, performance, or physical-hardware release gate was assessed.

The requested `sysctl -n hw.memsize` query was denied by the environment, so available RAM is recorded as not measured. No user settings/configuration files were read or written, and no migration was applied.
