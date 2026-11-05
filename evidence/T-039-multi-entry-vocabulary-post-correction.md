task_id: T-039
feature_ids: [F-08]
flows: [UF-05, UF-06]
acceptance: [AC-08]
owner: Codex coordinator
base_commit: 28232eeb9fa7d9eecf7e2d4a72e6f66518d643c8
result_commit: recorded as the implementation commit in the follow-up evidence commit
platform: macOS 26.6.2 (25G83) arm64; rustc 1.99.1; physical RAM not measured for this task
implementation: Pure collection-level correction in crates/core/src/vocabulary_post_correction.rs; runnable synthetic example in crates/core/examples/vocabulary_multi_post_correction.rs
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_multi_post_correction --locked', result: PASS, notes: 'Observed output exactly matched the three requested mappings; unrelated decomposed accent, Japanese text, and emoji remained unchanged; synthetic non-cascading example returned ubc.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'No formatting changes required.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Both workspace crates checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: 'All core/settings unit, integration, and documentation test executables completed successfully; core ran 126 unit tests and 2 doctests.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings after correcting a collapsible conditional.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build completed.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors after README, task ledger, source, example, and evidence edits.'}
metrics: NOT_MEASURED; no recognition, latency, memory, installed-size, battery, or release measurements were in scope
doc_updates: README.md multi-entry behavior/example/testing/troubleshooting; docs/TASKS.md partial T-039 record; this evidence file
risks:
  - Existing word-boundary classification is conservative and is not a full Unicode word-break implementation.
  - The function is not connected to Moonshine keyterm hints, the complete cleanup pipeline, settings persistence, or the frontend.
  - T-035, T-038, T-039, AC-07, AC-08, and all release gates remain open.
  - Existing unrelated untracked items `....` and `scripts/__pycache__/` were preserved.
review: PENDING

---

# Native Rust multi-entry vocabulary post-correction — partial T-039 evidence

## Outcome

Added the public pure-Rust API `apply_vocabulary_post_corrections(text, entries)` while retaining `apply_vocabulary_post_correction(text, spoken, written)`. The collection API first calls the existing collection validator, so it accepts 0–500 entries, applies the existing field limits, and rejects duplicate spoken phrases under Unicode 18 canonical-caseless keys before scanning any input text.

Both public functions now use one shared scanner and the existing boundary helpers. The scanner reads UTF-8 character boundaries from the original input, moves left to right, chooses the longest matching span at a start position, and stores the earliest entry for any equal key as the stable order fallback. It advances past each selected source span, so replacements cannot overlap and newly written text is never scanned. Output bytes outside a replacement are copied from the original text unchanged; replacement values are copied exactly.

Seventeen focused collection tests cover empty and maximum-size collections, one and multiple mappings, repeated occurrences, multiword expressions, overlap and longest-span precedence, non-cascading replacements, full Unicode case folding, canonical equivalence, combining-mark boundary protection, emoji/Japanese preservation, duplicate and invalid entries, error redaction, input immutability, and deterministic repeated calls. No dependency or lockfile changed.

The example uses only synthetic strings. Its observed stdout was:

```text
Input:
I visited north PARKADE.
I use rust at ubc.
The café serves 東京 tea 😀.
Output:
I visited North Parkade.
I use Rust at UBC.
The café serves 東京 tea 😀.
Unrelated Unicode preserved: true
Non-cascading input: rust
Non-cascading output: ubc
```

## Verification

Environment: macOS 26.6.2 (25G83), arm64, rustc 1.99.1. All requested commands ran from the repository root and passed. Workspace tests included 126 `outflow-core` unit tests and 2 core doctests; all `outflow-settings` unit, integration, and doctest executables also returned success. `git diff --check` passed after all documentation and source updates.

## Limits and status

This is a partial T-039 implementation. It does not provide Moonshine keyterm hints, the complete punctuation/filler pipeline, persistent settings, or speech/frontend integration. Word boundaries reuse the existing conservative classifier and do not claim complete Unicode word-break behavior. Physical 4 GB hardware, performance, recognition quality, installed size, and release criteria were not measured.

T-035, T-038, T-039, AC-07, AC-08, and every release gate remain incomplete. No task checkbox was changed.
