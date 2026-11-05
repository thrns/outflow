task_id: T-035/T-039
feature_ids: [F-07, F-08]
flows: [UF-05, UF-06]
acceptance: [AC-07, AC-08]
owner: codex/task-039-text-cleanup-composition
base_commit: 7cc3f89b5b6bc938d5bee05676cd41dbe96f9185
result_commit: 61cfccd32fcac94a20e71d8d681eefed944ad471
platform: macOS 26.6.2 (25G83) arm64; rustc 1.99.1 stable; physical RAM not measured for this task (repository inventory lists 18 GB)
implementation: Public staged API in crates/core/src/text_cleanup.rs; module summary in crates/core/src/lib.rs; synthetic example in crates/core/examples/text_cleanup_composition.rs; README and task ledger updated
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_cleanup_composition --locked', result: PASS, notes: 'Printed debug-formatted CRLF/tab input and exact expected output; both output-match and unrelated-Unicode checks printed true.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'No formatting changes required.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Both outflow-core and outflow-settings checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '425 tests passed: 147 core unit, 113 settings unit, 160 settings integration, and 5 doctests; zero failures.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build completed.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors in the then-unstaged README.md and docs/TASKS.md changes.'}
  - {command: 'git diff --cached --check', result: PASS, notes: 'No whitespace errors in the staged task documentation/evidence commit.'}
metrics: NOT_MEASURED; no recognition, latency, memory, installed-size, battery, or release measurements were in scope
doc_updates: README.md setup/example/checks/troubleshooting/limitations; docs/TASKS.md partial T-035/T-039 note; this evidence record
risks:
  - This composition is a building block only; command recognition, punctuation/capitalization, filler policy, Moonshine keyterm hints, persistence, and app integration remain out of scope.
  - Single-call non-cascading matching does not make repeated application of the complete function universally idempotent; the rustdoc, README, example tests, and task note preserve this distinction.
  - T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.
  - No dependency or lockfile changed. Existing unrelated untracked items `....` and `scripts/__pycache__/` were preserved.
review: PENDING

---

# Native Rust text cleanup composition — partial T-035/T-039 evidence

## Outcome

Added `outflow_core::text_cleanup::normalize_whitespace_and_apply_vocabulary(input, entries)`. It calls the existing whitespace normalizer first and passes that result to the existing multi-entry vocabulary corrector. Collection validation, Unicode 18 matching, conservative boundary behavior, longest-match precedence, and single-call non-cascading replacement remain owned by the existing functions. The wrapper does not mutate the borrowed input or entries and does not normalize output after replacement.

Sixteen focused tests exercise the new API. The example uses only the requested synthetic three-entry vocabulary and synthetic transcript. No dependency, lockfile, settings, native speech, or frontend integration changed.

Observed example output:

```text
Input: "  I visited   north PARKADE.\r\n\r\n\tI use rust\t at  ubc.  \r\nThe cafe\u{301} serves 東京 tea 😀."
Output: "I visited North Parkade.\n\nI use Rust at UBC.\nThe cafe\u{301} serves 東京 tea 😀."
Output matches expected: true
Unrelated Unicode preserved: true
```

## Verification

The commands above ran from the repository root on macOS 26.6.2 (25G83), arm64, with Rust 1.99.1 stable. Final format, workspace check, all 425 workspace tests including doctests, Clippy with warnings denied, workspace build, and diff checks passed. An earlier test run exposed an incorrect expected trailing space in a new fixture; the fixture was corrected to match the existing normalizer's documented line-edge trimming, then the complete workspace suite passed.

## Limits and status

The result demonstrates only whitespace normalization followed by vocabulary correction. It does not recognize dictation commands, repair punctuation/capitalization, apply Clean/Verbatim filler policy, add Moonshine keyterm hints, persist vocabulary, or connect to ASR, React, or Tauri. The current conservative word-boundary policy is unchanged and is not a full Unicode word-break implementation. No production quality, performance, hardware, or release gate was measured.

Within one call, inserted replacement text is not scanned again. That does not promise repeated-call idempotence: `rust` → `ubc` and `ubc` → `UBC` produce `ubc` on the first call and `UBC` when the pipeline is applied again to that result. Mappings remain unchanged.

T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain incomplete. The requested unrelated untracked items were not modified.
