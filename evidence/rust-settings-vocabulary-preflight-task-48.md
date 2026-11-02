---
task_id: Task 48 (settings vocabulary compatibility preflight; separate from repository T-048)
feature_ids: [F-08, F-11]
flows: [UF-06]
acceptance: [AC-08]
owner: Codex on shared worktree `codex/task23-moonshine-build-scope`
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: uncommitted
platform: macOS host, Darwin 25.6.0, arm64; Rust stable 1.99.1; physical machine model and RAM not measured
implementation: Opt-in settings DTO adapter in `crates/settings/src/vocabulary_preflight.rs`; ordered borrowed vocabulary views are passed to core readiness assessment.
metrics: Release size, runtime memory, and performance metrics NOT_MEASURED; not required for this read-only adapter task.
doc_updates: `README.md`; `docs/VOCABULARY_MATCHING_MIGRATION.md`; this evidence file. `docs/TASKS.md` unchanged.
risks: Schema v1 does not identify matching-policy provenance. Existing decode/load/initialization paths validate with lowercase-only matching and may reject malformed entries or existing lowercase conflicts before preflight. No raw-file recovery or automatic integration is implemented.
review: PENDING
---

# Task 48 — Settings vocabulary compatibility preflight

## Result

Added the public API:

```rust
outflow_settings::preflight_vocabulary_compatibility(
    settings: &Settings,
    source_policy: VocabularySourcePolicy,
) -> Result<VocabularyMigrationReadiness, VocabularyMigrationError>
```

The function borrows ordered `spoken` and `written` values from each owned `VocabularyEntry`, creates temporary `VocabularyEntryRef` values, and calls `outflow_core::vocabulary_migration::assess_vocabulary_migration_readiness`. The existing core readiness, conflict, policy, and error types are re-exported from `outflow-settings`. Entry IDs are not part of matching. The adapter does not clone vocabulary phrases, retain comparison keys, log content, use the filesystem or network, mutate settings, resolve conflicts, or select a winner.

## Dependency graph

Before this task, the workspace had two members: `outflow-core` and `outflow-settings`. Core had no settings dependency, and settings did not yet depend on core. Adding the direct local `outflow-core = { path = "../core" }` edge to settings is acyclic and preserves the rule that core does not depend on settings.

No external dependency or ICU4X feature was added. `Cargo.lock` was already dirty before Task 48 because of the uncommitted Task 45–47 core vocabulary work. Compared with that pre-task-48 lockfile, this task only adds `outflow-core` to the `outflow-settings` package dependency list; it changes no package versions or checksums.

## Behavior and limitations

- Empty vocabulary returns `EmptyCollection` regardless of provenance.
- For nonempty vocabulary, explicit `Unknown` provenance returns `SourcePolicyUnknown`, even without detected conflicts. Schema v1 does not prove the source policy.
- Verified legacy data without conflicts returns `CompatibleForExplicitMigration`; this does not mean migrated or saved.
- Verified current data without conflicts returns `AlreadyCurrent`.
- Conflicts preserve original and conflicting indices, legacy-vs-Unicode-18 classification, and deterministic pairwise order. Both verified provenance variants require manual resolution when conflicts exist.
- The core performs the existing 500-entry and per-field checks. Invalid fields and capacity errors are typed and do not expose phrase content.
- Synthetic cases cover `straße` / `STRASSE`, composed `café` / decomposed `cafe` plus combining acute, Greek `σ` / `ς`, and an existing lowercase conflict.
- A schema-v1 JSON fixture containing `straße` / `STRASSE` passes the existing lowercase validator and reaches the preflight. A separate `Hello` / `hello` fixture remains rejected by the existing settings decoder before preflight.
- Existing decoding, settings validation, JSON format, defaults, initialization, load/save, reset, and atomic-writer code were not changed. The API is not called from any of those pathways. It cannot recover data rejected by current decode/load validation.

The adapter does not complete T-017, T-018, T-038, T-039, or AC-08. Automatic settings integration still needs trusted provenance, an approved versioned schema/migration decision, explicit conflict review, and verified atomic write/rollback behavior. Repository T-048 is a separate task and is unchanged.

## Verification

Baseline was run before implementation: the Rust workspace had 163 passing tests, frontend had 136 passing tests, and Python had 258 passing tests.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked` | PASS, 91 passed; 2 non-vocabulary core tests filtered |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | PASS, 70 unit tests + 11 integration tests |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_preflight integration_summary_uses_readiness_results_from_real_preflight_calls --locked -- --nocapture` | PASS; printed the requested summary from real preflight calls |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS, 174 total (93 core + 70 settings unit + 11 new integration tests) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS, 136 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS, 258 tests |
| `git diff --check` | PASS |

The integration summary output was:

```text
Settings vocabulary compatibility preflight
Schema: v1
Source policy: UNKNOWN
Nonempty vocabulary: REVIEW REQUIRED
Verified legacy, no conflicts: COMPATIBLE
Verified legacy, Unicode 18 conflict: MANUAL RESOLUTION
Verified current, no conflicts: ALREADY CURRENT
Settings loaded automatically: NO
Settings saved: NO
Migration applied: NO
Original vocabulary modified: NO
```

## Changed files for this task

- `crates/settings/Cargo.toml`
- `crates/settings/src/lib.rs`
- `crates/settings/src/vocabulary_preflight.rs` (new)
- `crates/settings/tests/vocabulary_preflight.rs` (new)
- `Cargo.lock` (adds the local settings-to-core dependency; file contained prior Task 45–47 changes)
- `README.md` (file contained prior Task 47 changes)
- `docs/VOCABULARY_MATCHING_MIGRATION.md` (file was already untracked for Task 47)
- `evidence/rust-settings-vocabulary-preflight-task-48.md` (new)

The pre-existing uncommitted files were preserved. No user settings files were created or modified. `docs/TASKS.md` was not changed; T-017, T-018, T-038, T-039, and AC-08 remain incomplete.
