# T-035 partial evidence — Native punctuation spacing

```yaml
task_id: T-035
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: Codex / codex/task-035-punctuation-spacing
base_commit: ab07a93a3ecec14a172b76220dc541ef625343fd
result_commit: UNCOMMITTED
platform: macOS 26.6.2 (25G83), arm64; rustc 1.99.1 (58cf8f9c3 2026-10-13)
implementation: Standalone punctuation-spacing function and 62 exact-output fixtures in crates/core/src/text_cleanup.rs; runnable example in crates/core/examples/text_punctuation_spacing.rs; README.md, docs/TASKS.md, and this evidence record updated.
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_punctuation_spacing --locked', result: PASS, notes: 'Printed requested debug-formatted input/output; exact output and idempotence assertions passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'Final format check passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Both workspace crates checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: 'All workspace unit, integration, and documentation tests passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked --quiet', result: PASS, notes: '437 tests/doctests passed, 0 failed; includes the 62 exact-output fixtures and input-preservation test.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build completed.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors.'}
metrics: NOT_MEASURED; no inference, latency, memory, installer-size, or release measurements apply to this pure text stage.
doc_updates: README.md documents setup, run output, checks, troubleshooting, and limits; docs/TASKS.md records this as partial T-035 without changing task or acceptance checkboxes; this focused evidence record.
risks: This is not complete punctuation restoration and does not change the existing cleanup composition. A future integration must run after vocabulary correction and test effects on written replacements. Numeric-adjacent and ambiguous punctuation/quote sequences are intentionally preserved. T-023 remains unmet; full T-035, T-036, AC-07, and release gates remain incomplete.
review: PENDING
```

## Observed example output

```text
Before: "Hello , world !\nI visited north parkade .\nThe café , 東京 tea 😀 .\nVersion 3 . 14 remains unchanged."
After: "Hello, world!\nI visited north parkade.\nThe café, 東京 tea 😀 .\nVersion 3 . 14 remains unchanged."
```

## Policy and coverage

`normalize_punctuation_spacing` scans the original input and returns a new string. It removes only ASCII spaces/tabs before `, . ! ? ; :` when the preceding non-whitespace character is alphabetic, the mark is followed by whitespace or input end, the contiguous alphanumeric token has no numeric character, and no ambiguous punctuation or quote delimiter follows through horizontal whitespace. Everything else—including punctuation, letter case, numbers, Unicode scalars, emoji, and line breaks—is copied unchanged. The function is deterministic, idempotent, linear-time, and not integrated into the existing cleanup composition.

The 62 exact-output fixtures include empty and already-correct input; each mark; repeated spaces/tabs; multiple fixes; newlines and paragraph breaks; decimals, dates, versions and numeric-containing words; abbreviations and ellipses; quoted and ambiguous punctuation; Unicode alphabetic scripts, precomposed and combining text, Japanese and emoji; unrelated whitespace preservation; and repeatability/idempotence. A separate test verifies that the original `String` remains unchanged. The Rust example checks the requested exact output and idempotence.

This partial result does not satisfy the full T-035 golden-corpus requirement, AC-07, T-036, application integration, or release gates. T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.
