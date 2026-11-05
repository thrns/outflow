# T-036 cleanup preference adapter — partial evidence

```yaml
task_id: T-036
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07, AC-08]
owner: Codex coordinator
branch: codex/task-036-cleanup-preference-adapter
base_commit: a237d1c0e65008d056d0442bcfab8a2f858e026c
result_commit: UNCOMMITTED_AT_EVIDENCE_CAPTURE; final commit SHA is in the task handoff
platform: macOS 26.6.2 (25G83), arm64; rustc 1.99.1; Cargo 1.99.1
implementation: Added a pure outflow-settings adapter that enables core filler removal only for Clean plus remove_fillers=true; added 15 focused tests and a four-case example; updated README/TASKS; corrected the prior T-036 evidence output typo.
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example cleanup_filler_preferences --locked', result: PASS, notes: 'All four outputs matched exact assertions; protected replacement bytes also matched.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings cleanup_policy --locked', result: PASS, notes: '15 focused tests passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'No formatting differences.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Both workspace crates checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '191 core unit tests, 128 settings unit tests, 160 integration tests, and 11 doctests passed; zero failures.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace built successfully.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors after final documentation edits.'}
  - {command: 'git diff --cached --check', result: PASS, notes: 'No whitespace errors in staged task files.'}
metrics: NOT_MEASURED; this adapter task does not exercise release hardware, installed size, or process memory.
doc_updates: README.md; docs/TASKS.md; corrected one observed-output line in evidence/T-036-protected-filler-integration.md; this evidence record.
risks: This adapter controls filler deletion only. It is not wired to the application, microphone, ASR, or Tauri. Broader Clean/Verbatim policy, T-017/T-018 and T-035 prerequisites, the full T-036 golden set, AC-07/AC-08, platform behavior, and release gates remain open.
review: PENDING
```

## Adapter behavior

`outflow_settings::cleanup_policy::process_transcript_with_filler_preference` accepts the existing `CleanupSettings`, `CleanupMode`, and borrowed core vocabulary entries. It passes the effective filler flag `cleanup.mode == CleanupMode::Clean && cleanup.remove_fillers` to `process_transcript_with_optional_fillers`. No settings fields, schema version, serialization, defaults, migrations, dependencies, or persistence behavior changed.

The example printed:

```text
Input: "um, rust , at ubc ."
Clean, remove_fillers=true: "Rust , Edition, at UBC."
Clean, remove_fillers=false: "um, Rust , Edition, at UBC."
Verbatim, remove_fillers=true: "um, Rust , Edition, at UBC."
Verbatim, remove_fillers=false: "um, Rust , Edition, at UBC."
```

The example asserts all four complete outputs and exact bytes for the protected vocabulary replacement `rust → um, keep this`. The focused tests also cover the default, supported `um, ` / `uh, ` cases, repeated and sentence-internal fillers, independent command/vocabulary/punctuation stages, Unicode and paragraph breaks, typed validation failures, input immutability, determinism, and schema-v1 JSON round-trip.

## Correction to prior T-036 evidence

The committed `crates/core/examples/text_cleanup_with_fillers.rs` expects the disabled output `um, Rust , Edition, at UBC.`. The disabled-output line in `evidence/T-036-protected-filler-integration.md` omitted `Edition`; that single output line was corrected. Its historical verification commands and results were not changed, and no new result was attributed to that earlier run.

## Remaining scope

This is a partial T-036 building block. Verbatim always disables filler deletion here, but all other existing stages continue to run; the adapter does not define complete Verbatim semantics. The full acceptance corpus and task prerequisites are not complete, so T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked. No platform, insertion, ASR, resource, or release certification is claimed.
