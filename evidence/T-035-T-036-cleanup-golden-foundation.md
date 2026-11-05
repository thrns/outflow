# T-035/T-036 Cleanup Golden Corpus Foundation

**Status:** Partial characterization foundation; T-035 and T-036 remain incomplete.
**Date:** 2026-11-05
**Branch:** `codex/task-036-cleanup-golden-foundation`
**Starting commit:** `0f1a8d53200a83c09be206e0d17067d1367d689b`
**Implementation commit:** `b7f3f30` (`test(cleanup): add current behavior golden corpus`).
**Pushed checkpoint verified:** `d8b80770cd39eacafa0b6f377ffc4300cfd441b7`.

## Scope and result

Added a reusable fixture runner at `crates/settings/tests/cleanup_golden_corpus.rs` and 24 independently written synthetic cases in `crates/settings/tests/fixtures/cleanup_current_behavior.json`. The runner deserializes typed fixtures with unknown-field rejection, validates IDs and categories, parses the existing `CleanupMode`, calls `process_transcript_with_filler_preference`, compares full exact output, repeats each call for determinism, and verifies the input, vocabulary strings, and settings remain unchanged.

Observed fixture summary:

```text
Cleanup golden corpus: 24 fixtures passed
Categories covered: 8
Current behavior baseline: true
```

Coverage categories: `whitespace`, `commands`, `vocabulary`, `punctuation`, `fillers`, `modes`, `unicode`, and `safety`. All four Clean/Verbatim × `remove_fillers` combinations occur, as do both `commands_enabled` values. Fixtures cover empty and normalized input; CRLF, tabs and paragraphs; enabled/disabled `new line` and `new paragraph`; single/multiple vocabulary mappings; protected punctuation and filler-like replacements; conservative punctuation spacing, numeric/version and ambiguity cases; filler-setting behavior and repeated fillers; accented/combining text; emoji/Japanese; and proper nouns/abbreviations.

Expected strings were written explicitly after reviewing the current Rust stage implementations and existing contract tests. They were not generated from test execution. The fixtures characterize current implementation behavior only; they do not approve a final Verbatim policy.

## Changed files

- `crates/settings/tests/fixtures/cleanup_current_behavior.json` — 24 explicit synthetic cases.
- `crates/settings/tests/cleanup_golden_corpus.rs` — typed, data-driven exact-output and schema/coverage tests.
- `docs/CLEANUP_MODE_POLICY.md` — corrected FR-07.2/FR-07.4/Rust-doc attribution; recommendation remains **PROPOSED — NOT APPROVED**.
- `README.md` — schema, fixture authoring, commands, summary, troubleshooting, and characterization-vs-acceptance guidance.
- `docs/TASKS.md` — partial T-035/T-036 progress; all listed task and acceptance checkboxes remain unchecked.
- `evidence/T-035-T-036-cleanup-golden-foundation.md` — this evidence record.

No production cleanup code, settings fields/defaults/schema, dependencies, lockfiles, or speech benchmark manifest changed. Pre-existing untracked `....` and `scripts/__pycache__/` were left untouched.

## Verification

Environment: macOS 26.6.2 (25G83), arm64; `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; `cargo 1.99.1 (5f94df478 2026-08-27)`. The host is not a 4 GB reference machine; no resource or release metric was measured.

| Command | Result | Observed result |
|---|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS | 2 tests passed; printed the 24-fixture summary above. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS | 9 tests passed. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS | Passed after formatting the new Rust test. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS | Workspace check completed. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS | Workspace unit, integration, and doc tests completed with no failures; includes the new corpus test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | Completed with warnings denied. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS | Workspace build completed. |
| `git diff --check` | PASS | Final unstaged diff check completed after the documentation edits. |
| `git diff --cached --check` | PASS | Staged diff passed after staging only the six task files listed above. |

## Limits and open acceptance work

This 24-case baseline is not the ≥200-case AC-07 acceptance corpus and does not measure the ≥99% exact-output threshold or establish zero alteration of numeric tokens/proper nouns across a protected fixture corpus. It does not replace the complete production cleanup integration, T-035/T-036 prerequisites, speech quality assessment, or platform/release checks. No pipeline idempotence requirement was added.

The Verbatim policy remains unresolved. FR-07.2 explicitly disables filler deletion in Verbatim; FR-07.4 establishes the available modes and Clean default; the broader wording-preservation phrase is from the existing Rust `CleanupMode::Verbatim` documentation. The policy recommendation remains proposed and unapproved. **T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.**

The branch push succeeded. At pushed checkpoint `d8b80770cd39eacafa0b6f377ffc4300cfd441b7`, `git rev-parse HEAD origin/codex/task-036-cleanup-golden-foundation` returned the same SHA for both refs, and `git ls-remote origin refs/heads/codex/task-036-cleanup-golden-foundation` returned that same SHA. This evidence update is a follow-up documentation commit; the final branch-tip SHA is recorded in the task handoff.
