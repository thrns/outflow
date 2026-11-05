task_id: T-039
feature_ids: [F-08]
flows: [UF-05, UF-06]
acceptance: [AC-08]
owner: Codex coordinator
base_commit: 6241ee2fb253ab7fb8dd8cd6d06bd5601d9802fc
result_commit: task branch commit; exact SHA recorded in the final handoff
platform: macOS 26.6.2 arm64; Rust 1.99.1; physical RAM not measured for this task
implementation: Pure single-mapping post-correction in crates/core/src/vocabulary_post_correction.rs; exported from crates/core/src/lib.rs; synthetic example in crates/core/examples/vocabulary_post_correction.rs
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_post_correction --locked', result: PASS, notes: 'Corrected north PARKADE; retained decomposed accent, Japanese text, and emoji; reported unrelated Unicode preserved.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'No formatting changes required.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Both workspace crates checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '109 core unit tests, 113 settings unit tests, 160 settings integration tests, and 2 doctests passed; 384 total.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build completed.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors.'}
metrics: NOT_MEASURED; no recognition, latency, memory, installed-size, or release measurements were in scope
doc_updates: README.md synthetic example/setup/troubleshooting; docs/TASKS.md partial T-039 evidence; this evidence record
risks:
  - One spoken/written mapping is accepted per call; multi-entry precedence and keyterm hints are not implemented.
  - This function is not wired into Moonshine, a complete cleanup pipeline, or production settings.
  - Unicode word-continuation handling is conservative for alphanumeric characters, combining marks, joiners, variation selectors, enclosing marks, and common invisible format controls; it is not a complete Unicode word-break implementation.
  - No performance or release gate was measured, and T-035, T-038, T-039, AC-07, AC-08, and all release gates remain incomplete.
review: PENDING

---

# Native Rust vocabulary post-correction — partial T-039 evidence

## Outcome

Added `apply_vocabulary_post_correction(text, spoken, written)` to `outflow-core`. The function reuses `validate_vocabulary_entry` for the existing nonempty and 128-scalar validation and calls `comparison_key` with `CURRENT_VOCABULARY_MATCHING_VERSION` for both the mapping and candidate input spans. It replaces all complete, non-overlapping occurrences of that one mapping in a call. The output replacement is exactly the configured written phrase; text outside matches is copied from the original input without normalization or case changes.

Candidate spans begin and end on UTF-8 character boundaries. Unicode alphanumeric characters, canonical combining marks, underscore, internal apostrophes, join controls, variation selectors, enclosing marks, and common invisible format controls prevent partial-word matches. Punctuation and symbols are preserved around valid matches. This includes the regression where `cafe` must not match the prefix of `cafe` followed by U+0301 COMBINING ACUTE ACCENT.

The example uses only fixed synthetic strings. It demonstrates case-insensitive correction, retains an unrelated decomposed accent, Japanese text, and emoji, and prints a boolean preservation check. It does not load speech, settings, model assets, or external data.

## Verification

Environment: macOS 26.6.2 arm64, Rust 1.99.1. Commands were run from the repository root:

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_post_correction --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 109 core unit tests, 113 settings unit tests, 160 settings integration tests, 2 doctests (384 total) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS |

Example output:

```text
Input: I visited north PARKADE today.
The café serves 東京 tea 😀.
Output: I visited North Parkade today.
The café serves 東京 tea 😀.
Unrelated Unicode preserved: true
```

The workspace dependency manifests and lockfile were not changed. No frontend files or dependencies changed, so frontend checks were not rerun. No performance or release metrics were measured.

## Limits and status

The implementation handles one mapping per call. It does not choose precedence among entries, provide model keyterm hints, implement ordered cleanup, or connect to Moonshine or persistent settings. Its word-continuation classifier covers common Unicode letters, numbers, combining marks, joiners, and extenders but is not a complete Unicode word-break algorithm. The function is an offline pure-Rust capability, not evidence of end-to-end dictation behavior.

This evidence does not complete T-035, T-038, T-039, AC-07, AC-08, or any release gate. T-039's other requirements and dependencies remain outstanding.
