# T-035 partial evidence — Native opt-in dictation commands

```yaml
task_id: T-035
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: Codex / codex/task-035-dictation-commands
base_commit: 57c8d619a25f1810700e8c08693222ef4f984078
result_commit: a176ac048c339893aa8f7e9583688a090fd27c1e
platform: macOS Darwin 25.6.0 arm64; rustc 1.99.1 (58cf8f9c3 2026-10-13)
implementation: Standalone two-command scanner and FR-07.3 text composition in crates/core/src/text_cleanup.rs; runnable example in crates/core/examples/text_dictation_commands.rs; README.md and docs/TASKS.md updated.
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_dictation_commands --locked', result: PASS, notes: 'Exact enabled/disabled assertions passed; observed output recorded below.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: FAIL, notes: 'Initial check reported rustfmt-only layout changes in the new source and example.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all', result: PASS, notes: 'Applied rustfmt layout.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'Final formatting check passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'outflow-core and outflow-settings checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: 'All workspace unit, integration, and documentation tests exited successfully; includes 37 command fixtures and seven additional composition tests.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build completed.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors.'}
  - {command: 'git diff -- Cargo.toml Cargo.lock', result: PASS, notes: 'No dependency or lockfile changes.'}
metrics: NOT_MEASURED; no latency, memory, installer-size, recognition-quality, or release metrics were part of this deterministic text-stage slice.
doc_updates: README.md opt-in behavior/example/tests/troubleshooting/limitations; docs/TASKS.md partial T-035 evidence; this evidence record.
risks: Exact phrases can be ordinary spoken text; the feature is opt-in and is not a semantic intent classifier. Command boundaries intentionally reject adjacent punctuation and quotes. The standalone command function expects normalized input; the composition normalizes first. T-023 and app/settings integration remain open. Punctuation, capitalization, filler handling, AC-07 corpus acceptance, and release gates are incomplete.
review: PENDING
```

## Observed example output

```text
input: "  I use  rust  new line   at ubc  new paragraph  north parkade"
commands enabled: "I use Rust\nat UBC\n\nNorth Parkade"
commands disabled: "I use Rust new line at UBC new paragraph North Parkade"
```

## Automated coverage

The standalone command fixture table contains 37 exact input/flag/output cases. It covers disabled input, command positions and case, repeated/consecutive commands, separator trimming, existing LF/CRLF breaks, punctuation and quotation boundaries, longer words, incomplete phrases, Unicode and emoji, and non-ASCII boundaries. Seven additional focused tests cover disabled identity, normalization before matching, command precedence over vocabulary, vocabulary correction after commands without output normalization, longest-match and non-cascading correction, typed validation errors, input/entry immutability, and deterministic repeated calls.

The workspace test command exited 0, including the existing core/settings tests and doctests. No existing test or example was removed.

## Scope and limitations

The implementation changes only text cleanup. It adds no dependency, settings persistence, UI, Tauri, speech-recognition, microphone, clipboard, punctuation, capitalization, or filler-removal behavior. The existing compatibility composition keeps command recognition disabled.

This is partial T-035 evidence. T-023 remains an unmet prerequisite; this slice does not satisfy the complete cleanup pipeline, the ≥150-case T-035 golden corpus, AC-07, T-036, vocabulary persistence/hints, or any release gate. T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.
