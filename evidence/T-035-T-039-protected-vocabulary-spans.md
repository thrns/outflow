# T-035/T-039 partial evidence — protected vocabulary replacement spans

```yaml
task_id: T-035/T-039 (partial)
feature_ids: [F-07, F-08]
flows: [UF-05, UF-06]
acceptance: [AC-07, AC-08]
owner: Codex / codex/task-035-vocabulary-protected-spans
base_commit: 118ec5860d7d10c7d930a89f48a7a304c07d8a69
result_commit: f5522f24b9bee17d1e6c7a6299dc398fac9940b5
platform: macOS 26.6.2 (25G83), arm64; rustc 1.99.1 (58cf8f9c3 2026-10-13); physical RAM not measured
implementation: Added VocabularyCorrectedText and apply_vocabulary_post_corrections_with_spans in crates/core/src/vocabulary_post_correction.rs. The shared original-input scanner records UTF-8 byte ranges while appending exact written replacements. The text-only public APIs retain their output. Added 13 focused span tests and crates/core/examples/vocabulary_protected_spans.rs; updated README.md and docs/TASKS.md.
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_protected_spans --locked', result: PASS, notes: 'Exact output and ranges asserted; printed both expected replacement slices and All range assertions passed: true.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'No formatting changes required.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Both outflow-core and outflow-settings checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '451 tests and doctests passed; 0 failed. outflow-core ran 170 unit tests and 7 doctests.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'Workspace Clippy completed with warnings denied.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build completed.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors.'}
metrics: NOT_MEASURED; physical RAM, latency, memory, installed size, inference quality, and release gates were not in scope
doc_updates: README.md documents the API, byte-range semantics, example output, verification commands, troubleshooting, and limitations; docs/TASKS.md adds this partial evidence without checking task boxes; this evidence record.
risks: Ranges are valid for the returned text and can become stale if later edits change its bytes. Existing conservative Unicode word-boundary behavior is unchanged. Punctuation cleanup is not applied. This does not satisfy full T-035/T-039 or AC-07/AC-08.
review: PENDING
```

## Outcome

`VocabularyCorrectedText` contains the corrected output and ordered, non-overlapping `Range<usize>` byte offsets into that output. The range for each match is captured from `output.len()` immediately before and after appending the configured written value. As a result, the range contains only that exact replacement; unmatched transcript punctuation, whitespace, and other input bytes are excluded. The matcher still validates the collection before scanning, reads only original input, uses Unicode 18 matching and the existing word boundaries, chooses the longest match at each leftmost start, and does not cascade replacement text.

The existing single-entry and multi-entry text-only APIs remain available. The multi-entry API returns the `.text` produced by the shared span-aware implementation. Empty collections and collections with no matches return an empty range list. The input and borrowed collection are not mutated; the API adds no logging or vocabulary-bearing errors and introduces no dependency.

Thirteen new focused unit tests cover empty/no-match, one and multiple replacements, repeats, multiword and longest matches, non-cascading output, exact multibyte offsets, canonical accent matching, combining marks, exact spaces/punctuation/newlines, invalid and duplicate collections, input/collection immutability, deterministic calls, and parity with the text-only API. Each reported range is checked for bounds, UTF-8 boundaries, output ordering, non-overlap, and exact replacement slicing.

## Example output

```text
Original input: "rust , at ubc ."
Corrected output: "Rust , Edition , at UBC ."
Protected ranges: 2
Protected replacement 1: "Rust , Edition"
Protected replacement 2: "UBC"
All range assertions passed: true
```

## Limits and task status

This is a provenance primitive only. It does not apply punctuation spacing or other cleanup, modify command recognition, or connect vocabulary to Moonshine, settings persistence, or the application pipeline. Later stages must not assume ranges remain valid after changing the returned string. The conservative matcher is not a complete Unicode word-break implementation. No hardware, resource, recognition-quality, installer, or release gate was measured.

**T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.**
