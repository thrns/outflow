---
task_id: T-049
feature_ids: [F-08, F-11]
flows: [UF-06]
acceptance: [AC-08]
owner: /root (primary agent)
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: uncommitted
platform: macOS 26.6.2, arm64; RAM not independently queried for this task
implementation: Isolated serializable vocabulary policy marker and pure interpreter; proposed schema v2 fixture is test-only; no production persistence integration.
metrics: NOT_MEASURED (not a performance task)
doc_updates: README.md; docs/VOCABULARY_MATCHING_MIGRATION.md
risks: Trusted source-policy provenance, approved persistent schema migration, explicit conflict review, cross-platform writer permissions, and rollback/interruption acceptance remain unresolved.
review: PENDING
---

# Task 49 — Versioned vocabulary policy marker

## Implementation

- Added `crates/settings/src/vocabulary_policy_marker.rs` and re-exported its public API from `outflow-settings`.
- `VocabularyPolicyMarker::Unicode18CanonicalCaselessV1` serializes exactly as `"unicode18-canonical-caseless-v1"`. Its meaning is Unicode 18.0.0 canonical-caseless matching, default full non-Turkic case folding, NFD before and after folding, and marker interface version 1.
- `inspect_vocabulary_policy_marker(Option<&str>)` returns `Absent`, `Recognized(marker)`, or `Unsupported`. Recognition is case-sensitive; there are no aliases or fallbacks. Unsupported input is discarded, and deserialization errors do not echo arbitrary marker strings.
- A declaration does not establish `VocabularySourcePolicy` provenance. The test-only proposed schema-v2 fixture is not a production DTO and is not registered with the settings loader.
- The production Settings DTO, schema version v1, serialization, decoder, validator, load/save/initialization/reset behavior, and existing lowercase-only vocabulary validation are unchanged. No migration, schema field, conflict resolution, dependency, or task-ledger change was introduced.

## Verification

Pre-edit workspace baseline: **174** Rust unit/integration tests passed. Final workspace: **191** Rust unit/integration tests passed (174 existing + 17 added marker tests), plus **1** passing documentation test.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings vocabulary_policy_marker --locked -- --nocapture` | PASS — 17 tests; printed the requested marker-contract summary from real parser/preflight results |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_preflight --locked` | PASS — 11 tests |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked` | PASS — 87 unit, 11 integration, 1 doc test; includes serialization, load, initialization, reset, save, and preflight coverage |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 191 unit/integration, 1 doc test |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 136 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 258 tests |
| `git diff --check` | PASS |

The marker summary reported:

```text
Outflow vocabulary policy marker contract

Current identifier: unicode18-canonical-caseless-v1
Valid marker: RECOGNIZED
Missing marker: ABSENT
Unsupported marker: REJECTED
Verified provenance established: NO

Production settings schema: v1
Proposed future schema: v2
Schema migration applied: NO
Settings files modified: NO
```

## Documentation and remaining work

`README.md` and `docs/VOCABULARY_MATCHING_MIGRATION.md` describe the declaration/provenance distinction, the incomplete proposed v2 JSON fragment, fail-closed handling, compatibility cases, and future migration/rollback requirements. The existing settings writer's cross-platform permission and rollback acceptance work remains incomplete. Before any persistent schema change, obtain the required approval, establish a reviewed provenance contract, approve a versioned migration, and verify collision review, platform permissions, atomic recovery, rollback, and interruption behavior.

T-017, T-018, T-038, T-039, and AC-08 remain incomplete. Repository `docs/TASKS.md` T-049 was not modified. This task started on branch `codex/task23-moonshine-build-scope` with substantial pre-existing uncommitted work from the vocabulary preflight sequence; that work was preserved. Cargo manifests and `Cargo.lock` were already modified before this task, and received no Task 49 changes. No Moonshine CMake configuration was attempted.
