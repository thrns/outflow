# Rust custom vocabulary validation — focused Task 41 request

```yaml
task_id: request-task-41-rust-vocabulary-validation
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: Codex single-agent coordinator
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: uncommitted
platform: macOS 26.6.2 arm64; Rust 1.99.1 stable; Node.js 22.23.3; npm 10.9.9; Python 3.13.13; RAM NOT_MEASURED
implementation: 'Added a pure validator, typed redacted errors, 15 unit tests, a synthetic runnable example, README guidance, and this report.'
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked', result: PASS, notes: '15 vocabulary tests passed; 2 non-vocabulary core tests filtered out.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_validation --locked', result: PASS, notes: 'Printed all six expected lines; synthetic values only.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all', result: PASS, notes: 'Formatted the Rust additions.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'Workspace formatting check passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Both workspace crates checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '87 tests passed: 17 core and 70 settings; all 72 pre-existing tests remain passing.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'All targets passed with warnings denied.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo tree -p outflow-core --locked', result: PASS, notes: 'outflow-core has no dependencies.'}
  - {command: 'npm run typecheck', result: PASS, notes: 'Frontend typecheck passed.'}
  - {command: 'npm run test -- --run', result: PASS, notes: '10 files and 136 frontend tests passed.'}
  - {command: 'npm run lint', result: PASS, notes: 'ESLint passed.'}
  - {command: 'npm run build', result: PASS, notes: 'Vite 8.3.5 production build passed.'}
  - {command: 'python3 -m unittest discover -s scripts/tests -v', result: PASS, notes: '226 tests passed; Moonshine configure-runner tests mocked subprocesses. No CMake configuration was attempted.'}
  - {command: 'git diff --check', result: PASS, notes: 'Tracked diff has no whitespace errors; new files also inspected and scanned directly.'}
metrics: 'NOT_MEASURED; no performance or release-resource claim is needed for this pure validation slice.'
doc_updates: 'README.md and this evidence report. docs/TASKS.md was not modified.'
risks: 'Embedded control/pathological-input policy is unresolved by the repository and remains unenforced. UTF-16 surrogate validation belongs at the JavaScript-to-Rust conversion boundary. T-017 and T-018 remain incomplete, so T-038 prerequisites are outstanding. This request calls itself Task 41, while repository T-041 is the onboarding permission checklist; no repository task or acceptance status was changed.'
review: PENDING
```

## Requirement and implementation

The change addresses PRD F-08 / FR-08.1 and FR-08.4, UF-06, ARCHITECTURE C-06 and C-08, and TECH_STACK TS-06 and TS-08. The public API is `outflow_core::vocabulary::validate_vocabulary_entry(spoken, written) -> Result<(), VocabularyValidationError>`. Its error variants carry `VocabularyField::Spoken` or `VocabularyField::Written` and a failure reason of `Empty` or `TooLong`.

The function checks each field independently. Empty and Unicode-whitespace-only strings are rejected using `char::is_whitespace`; each field is limited to 128 Unicode scalar values using `str::chars()`. The count stops at 129 after the whitespace-only check. This correctly accepts 128 emoji, rejects 129 emoji, and treats a base letter plus combining mark as two scalar values. No accepted phrase is changed, and errors do not contain phrase contents. Rust `&str` already guarantees valid UTF-8; malformed JavaScript UTF-16 must be rejected by the conversion API before this validator can be called.

The module has no filesystem, network, logging, settings mutation, global state, or external crate use. It does not implement vocabulary collection CRUD, the 500-entry cap, duplicate/conflict policy, persistence, editing/deletion, or text replacement. The existing settings schema and React form were not changed. The current repository defines no policy for embedded controls or other pathological input, so that portion of FR-08.4 remains unresolved.

## Example output observed

```text
Outflow vocabulary validation demo
Valid vocabulary mapping: PASS
Empty spoken phrase: REJECTED
Overlong written replacement: REJECTED
Unicode scalar boundary: PASS
Vocabulary persistence: NOT IMPLEMENTED
Text replacement: NOT IMPLEMENTED
```

## Unicode and regression results

- Exactly 128 and 129 ASCII scalars were checked in both Spoken and Written fields.
- Exactly 128 and 129 emoji scalars were checked in both fields; emoji are counted as scalars rather than UTF-8 bytes.
- Empty, ASCII whitespace-only, and Unicode whitespace-only cases identified the correct field and `Empty` reason.
- `café`, `München`, and decomposed `e` plus combining acute were accepted; the decomposed sequence was directly verified to contain two scalars.
- Accepted text with leading/trailing spaces, capitalization, and punctuation remained unchanged.
- Repeated calls returned equal results; error display omitted synthetic phrase contents.
- 15 new tests passed. The workspace now has 87 passing tests, including all 72 that existed before this change.
- Frontend: typecheck, 136 tests, lint, and build passed. Python: all 226 tests passed.

## Dependencies, task status, and scope

`cargo tree -p outflow-core --locked` shows only `outflow-core`; no dependency was added. `Cargo.toml`, crate manifests, and `Cargo.lock` were not changed.

The task ledger records T-017 as unchecked because migration behavior and T-001 remain outstanding. T-018 is unchecked because Windows owner-only ACL support is still blocked and Linux behavior is untested. Therefore T-038's T-017/T-018 prerequisites remain incomplete. The README and this report do not mark T-038 or AC-08 complete, and `docs/TASKS.md` was left unchanged. Repository T-041 remains the separately defined onboarding permission checklist; this evidence uses a request-scoped identifier to avoid conflating the two tasks.

## Files and Git state

Scoped files changed or added:

- `crates/core/src/lib.rs`
- `crates/core/src/vocabulary.rs`
- `crates/core/examples/vocabulary_validation.rs`
- `README.md` (appended a focused section to preserve prior uncommitted README work)
- `evidence/rust-vocabulary-validation-task-41.md`

The existing branch `codex/task23-moonshine-build-scope` and base commit were preserved. The worktree already contained unrelated uncommitted frontend, README, evidence, and Moonshine-probe changes before this task; they were preserved. No staging, commit, push, merge, lockfile change, task-ledger edit, or CMake configure operation was performed.
