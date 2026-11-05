---
task_id: T-036
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: Codex / codex/task-036-leading-filler-removal
base_commit: 49f4a8de2ed954053cd4eb4adb8406cf6e4b9007
result_commit: 12f4486d79ebfb8d525901afc026c249dc0c127d
platform: macOS 26.6.2 (25G83), arm64; hardware model and RAM not verified in this run
implementation: Standalone opt-in line-leading um/uh removal in crates/core/src/text_cleanup.rs; 55 exact-output fixtures; synthetic Rust example; README and task documentation
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_filler_removal --locked', result: PASS, notes: 'Printed exact Clean and Verbatim outputs; assertions passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core leading_filler --locked', result: PASS, notes: '2 focused tests passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'No formatting changes required.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Both workspace crates checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: 'Workspace unit, integration, and documentation tests passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'Passed after replacing a manual newline predicate with a char-array pattern.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build completed.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors.'}
  - {command: 'git diff --cached --check', result: PASS, notes: 'No staged whitespace errors at final check.'}
metrics: NOT_MEASURED; no speech model, performance, installed-size, or release-gate measurements were part of this pure text stage
doc_updates: README.md; docs/TASKS.md partial T-036 entry
risks: Not integrated into the composed pipeline; no settings adapter or protected vocabulary range input. Any future integration must run after punctuation spacing and preserve protected replacement ranges. T-035/T-017 prerequisites and full AC-07 acceptance remain incomplete.
review: PENDING
---

# T-036: Native leading filler removal

## Scope

This is a partial T-036 implementation for FR-07.2, FR-07.3, C-06, and UF-05. The new public function is `outflow_core::text_cleanup::remove_leading_isolated_fillers(input, enabled)`. When enabled, it removes only ASCII case-insensitive `um, ` and `uh, ` at the start of a line if the following character is alphabetic. Disabled calls preserve input text. Repeated supported filler prefixes such as `um, uh, yes` remain intact. The function does not change capitalization, punctuation, or other UTF-8 bytes.

The function assumes normalized transcript input, including exactly one ASCII space after the comma. It preserves LF, CRLF, and CR bytes. It remains separate from all composed cleanup APIs and does not consume settings or protected-range metadata.

## Exact example output

```text
Input: "um, so I visited north parkade.\nUh, I use Rust at UBC.\nI said um, not uh."
Clean: "so I visited north parkade.\nI use Rust at UBC.\nI said um, not uh."
Verbatim: "um, so I visited north parkade.\nUh, I use Rust at UBC.\nI said um, not uh."
```

## Verification

The focused unit test uses 55 exact-output fixtures covering empty and disabled inputs, both supported fillers and ASCII capitalization, start/middle/end positions, repeated and incomplete phrases, punctuation and quotes, numbers, abbreviations, proper nouns, indentation, paragraph and line-ending preservation, accents and combining marks, Japanese, emoji, input immutability, deterministic calls, and idempotence. The runnable example independently asserts the requested output, Verbatim preservation, idempotence, and original-input immutability.

All commands in the metadata block passed in the final validation run. During implementation, the fixture-count assertion initially expected 54 entries and failed against the 55-entry table; the count was corrected. The first Clippy run identified a manual character predicate; it was changed to a character-array pattern and the final Clippy run passed with warnings denied.

`sysctl -n hw.model` and `sysctl -n hw.memsize` returned `Operation not permitted`, so hardware model and physical RAM were not verified in this run. This does not affect the pure Rust checks; it is not evidence for performance or any release gate.

## Limitations and status

Filler removal is not in the composed cleanup pipeline. No settings schema, persistence, model/ASR path, React/Tauri UI, microphone, clipboard, new dependency, or integration behavior changed. A future adapter must map Clean/Verbatim and `remove_fillers` into `enabled`; a future pipeline integration must run after punctuation spacing and honor protected vocabulary replacement ranges.

The full T-036 task remains incomplete because prerequisite tasks and the complete cleanup policy/acceptance corpus are still outstanding. T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.
