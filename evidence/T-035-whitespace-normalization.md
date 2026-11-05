---
task_id: T-035
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: codex/task-035-whitespace-normalization
base_commit: 390633ab64c0f2f388376ef6494ef0e35bf5d98a
result_commit: committed branch tip codex/task-035-whitespace-normalization (SHA reported in final handoff)
platform: macOS 26.6.2 arm64; Rust 1.99.1
implementation: Pure single-pass whitespace normalizer, Rust example, table-driven/focused tests, README instructions, and partial T-035 task note; see crates/core/src/text_cleanup.rs
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_whitespace_normalization --locked', result: PASS, notes: 'Printed debug-formatted input/output and Idempotent: true; exact expected output assertion passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: FAIL, notes: 'Initial check identified module ordering and one wrapped assertion; ran cargo fmt and reran this check successfully.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all', result: PASS, notes: 'Applied rustfmt changes after the initial check.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'Final formatting check passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Core and settings workspace crates checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '408 tests passed: 131 core unit, 113 settings unit, 160 settings integration, and 4 doctests; zero failures.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build completed.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors in tracked working-tree changes before staging.'}
  - {command: 'git diff --cached --check', result: PASS, notes: 'No whitespace errors across all six staged task files, including new files.'}
metrics: NOT_MEASURED; no performance, WER, model, install-size, RSS, or release gates were measured
doc_updates: README.md; docs/TASKS.md partial T-035 note; this evidence record
risks:
  - T-023 remains unchecked, so this independent stage does not complete dependent production T-035.
  - The 32 table fixtures and four focused tests do not satisfy AC-07's 200-transformation production golden corpus or its quality gate.
  - Punctuation/capitalization correction, dictation commands, filler removal, vocabulary ordering/integration, and application wiring are out of scope.
  - T-035, T-036, T-038, T-039, AC-07, AC-08, and every release gate remain unchecked.
review: PENDING
---

# Native Rust whitespace normalization — partial T-035 evidence

## Outcome

Added `outflow_core::text_cleanup::normalize_transcript_whitespace(&str) -> String`. It changes CRLF and standalone CR to `\n`; collapses internal runs of ASCII spaces/tabs; and trims only those characters at each line edge. It preserves newline count and placement, including blank lines and leading/trailing newlines. Non-ASCII whitespace and all other Unicode code points, words, punctuation, capitalization, and numbers remain unchanged. The implementation uses one character scan, standard-library types only, and no regex or new dependency.

The test module contains 32 table fixtures and four additional focused tests. Each table fixture asserts exact output, idempotence, and input immutability. The focused tests verify exact newline placement/count, only ASCII space/tab trimming, unchanged punctuation/Unicode text, and empty input. The runnable example uses synthetic input only, prints Rust debug strings, asserts the exact expected result, and asserts idempotence.

## Verification

Environment: macOS 26.6.2 arm64, Rust 1.99.1. Commands ran from the repository root. The first format check found formatting differences; `cargo fmt --all` applied them, after which the final format check passed. The complete workspace run passed 408 tests: 131 core unit tests, 113 settings unit tests, 160 settings integration tests, and 4 doctests.

Observed example output:

```text
Before: "  I visited   north parkade.\r\n\r\n\tI use rust\t at  ubc.  "
After: "I visited north parkade.\n\nI use rust at ubc."
Idempotent: true
```

The implementation introduced no manifest or lockfile changes. No frontend, Tauri, ASR, clipboard, vocabulary persistence, or production settings files changed. No performance or release measurement was in scope.

## Limits and status

This is an isolated first stage, not the complete ordered cleanup pipeline. It does not correct punctuation/capitalization, interpret commands, remove fillers, or connect to vocabulary correction. T-023 remains unchecked and is a listed T-035 prerequisite. The current 32 table fixtures do not satisfy the production acceptance requirement of at least 200 transformations. Therefore this evidence does not complete T-035, T-036, T-038, T-039, AC-07, AC-08, or any release gate.
