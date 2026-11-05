# T-036 protected-aware filler integration — partial evidence

```yaml
task_id: T-036
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: Codex coordinator
branch: codex/task-036-protected-filler-integration
base_commit: e5344efbac3051bccea31d550a429045ce0a474d
result_commit: d508902d57c825cd15b492260090b14d744e57e0
platform: macOS 26.6.2, arm64; rustc 1.99.1; Cargo 1.99.1
implementation: Shared conservative filler recognizer; private protected-aware removal with adjusted UTF-8 ranges; five-stage public composition; focused exact-output tests; runnable example.
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_cleanup_with_fillers --locked', result: PASS, notes: 'Printed all four expected lines exactly and assertions passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core optional_filler_pipeline --locked', result: PASS, notes: '7 focused composed-pipeline tests passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'No formatting differences.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Both workspace crates checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: 'All workspace unit, integration, and documentation tests passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace built successfully.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors.'}
  - {command: 'git diff --cached --check', result: PASS, notes: 'No whitespace errors in staged task files.'}
metrics: NOT_MEASURED; no release/resource benchmark was part of this text-only task. Physical RAM was not measured.
doc_updates: README.md; docs/TASKS.md; this evidence record.
risks: This is a partial T-036 implementation; full Clean/Verbatim settings behavior, complete 50 additional task goldens, prerequisite completion, app integration, platform coverage, and release gates remain open.
review: PENDING
```

## Behavior and evidence

The new public `process_transcript_with_optional_fillers` API runs whitespace normalization, optional dictation commands, vocabulary post-correction, protected punctuation spacing, then optional protected-aware filler removal. It returns final text only. The new flag controls only the filler stage.

The internal filler helper shares the existing ASCII case-insensitive line-leading `um, ` / `uh, ` recognizer with the standalone API. It deletes a candidate only when the candidate range does not intersect a protected replacement. Overlap retains the whole candidate; otherwise the existing rule for the next alphabetic character and repeated fillers is unchanged. Deletions shift subsequent private protected ranges, and an internal test verifies UTF-8 range validity, ordering, non-overlap, and byte-for-byte preservation of every replacement across multiple Unicode and line-break offsets.

The runnable example's observed output was:

```text
Input: "um, rust , at ubc ."
Fillers enabled: "Rust , Edition, at UBC."
Fillers disabled: "um, Rust , Edition, at UBC."
Protected replacement preserved: true
```

Exact-output tests cover the requested protected cases, ordinary enabled/disabled output, command/vocabulary order, longest-match and non-cascading behavior, empty and invalid/duplicate collections, Unicode and line breaks, repeated/numeric/quoted filler preservation, input immutability, determinism, and existing API behavior. No dependency or persisted setting was added or changed.

T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked. No full Clean/Verbatim settings policy, platform insertion behavior, ASR integration, 200-case acceptance corpus, or resource certification is claimed.
