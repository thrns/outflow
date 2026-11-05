```yaml
task_id: T-035/T-039 (partial integration)
feature_ids: [F-07, F-08]
flows: [UF-05, UF-06]
acceptance: [AC-07, AC-08]
owner: Codex coordinator
base_commit: 7df51c510ba88bd9fe1433e64d6aa7b032513bc5
result_commit: 38b8475ed9e590b254ee7984e0300a8ecdea4fdf
platform: macOS 26.6.2, Darwin 25.6.0, arm64; exact hardware model and RAM NOT_MEASURED
implementation: >-
  Added one protected-aware punctuation formatter shared by the existing
  standalone API and the new four-stage composition. Added exact-output tests,
  a runnable example, README usage/troubleshooting, task progress, and this
  evidence. No dependency or lockfile changes.
commands:
  - command: RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core text_cleanup --locked
    result: PASS
    notes: 42 passed, 0 failed.
  - command: RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_protected_punctuation_pipeline --locked
    result: PASS
    notes: >-
      Printed Input: "rust , at ubc ."; Output: "Rust , Edition, at UBC.";
      Matches expected: true; Protected replacement preserved: true.
  - command: RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
    result: PASS
  - command: RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
    result: PASS
  - command: RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
    result: PASS
    notes: 463 unit, integration, and doctests passed; 0 failed.
  - command: RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
    result: PASS
  - command: RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
    result: PASS
  - command: git diff --check
    result: PASS
  - command: git diff --cached --check
    result: PASS
metrics: >-
  Recognition quality, latency, installed bytes, and aggregate RSS NOT_MEASURED.
  No physical 4 GB release-hardware or platform integration gate was run.
doc_updates: README.md protected composition section; docs/TASKS.md partial progress entry.
risks: >-
  The complete pipeline can change on repeated application when vocabulary
  outputs match other entries; this is documented and tested. Full T-035/T-039,
  T-023 prerequisites, AC-07/AC-08, and release gates remain incomplete.
review: PENDING
```

## Implementation and preservation evidence

The new public API runs whitespace normalization, optional dictation commands,
vocabulary correction with ordered UTF-8 replacement ranges, and punctuation
spacing in that exact sequence. The formatter shares the standalone
punctuation rules, retains an entire candidate whitespace run if it intersects
a protected range, and adjusts internal provenance offsets for any earlier
spacing removals. The public API returns only final text.

The tests compare each vocabulary stage range to its corresponding adjusted
final range in order. This verifies every replacement byte-for-byte even when
replacement strings repeat. Exact fixtures cover the requested outputs,
leading/trailing/internal spaces, repeated and multiple replacements, outside
punctuation eligibility, emoji and accented UTF-8 prefixes, combining marks,
replacement newlines, paragraph breaks, numerical/version tokens,
abbreviations, ambiguous punctuation and quotes, command ordering, longest
match, non-cascading behavior, validation, immutability, determinism, and
compatibility of the existing APIs.

The requested runnable example printed:

```text
Input: "rust , at ubc ."
Output: "Rust , Edition, at UBC."
Matches expected: true
Protected replacement preserved: true
```

## Scope and handoff

The implementation is limited to `crates/core/src/text_cleanup.rs`, the new
example `crates/core/examples/text_protected_punctuation_pipeline.rs`,
`README.md`, this evidence, and the partial progress paragraph in
`docs/TASKS.md`. The two pre-existing untracked paths (`....` and
`scripts/__pycache__/`) were preserved and are not part of this task.

No punctuation insertion, capitalization, filler policy, Moonshine hinting,
settings persistence, ASR/frontend integration, or release validation was
added. T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates
remain unchecked. The next step for this slice is review of the task branch;
it does not authorize broader completion claims.
