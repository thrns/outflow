task_id: request-scoped Task 42 (not repository T-042)
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: single agent on existing branch codex/task23-moonshine-build-scope
base_commit: a8b92fc4cc683abce23bcb528d62e0ce8457a5b1
result_commit: uncommitted
platform: macOS 26.6.2, Darwin 25.6.0, arm64; Rust 1.99.1 stable; RAM/hardware model not measured
implementation: 'Added borrowed collection validation in crates/core/src/vocabulary.rs, expanded the existing synthetic example, and appended Task 42 guidance to README.md.'
commands:
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked', result: PASS, notes: '38 vocabulary tests passed; 2 non-vocabulary core tests filtered out.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_validation --locked', result: PASS, notes: 'Synthetic entry and collection cases produced the expected output shown below.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'Formatting is clean.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Workspace checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '110 tests passed: 40 outflow-core and 70 outflow-settings. The supplied baseline was 87; this task adds 23 tests.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build succeeded.'}
  - {command: 'npm run typecheck', result: PASS, notes: 'Typecheck succeeded.'}
  - {command: 'npm run test -- --run', result: PASS, notes: '136 frontend tests passed across 10 files.'}
  - {command: 'npm run lint', result: PASS, notes: 'ESLint succeeded.'}
  - {command: 'npm run build', result: PASS, notes: 'Typecheck and Vite production build succeeded.'}
  - {command: 'python3 -m unittest discover -s scripts/tests -v', result: PASS, notes: '226 Python tests passed.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors in tracked diffs; task files also inspected directly while untracked.'}
metrics: 'Not applicable. This pure validation task does not claim product size, RSS, latency, quality, or platform integration measurements; system RAM could not be read in the sandbox.'
doc_updates: 'README.md Task 42 section; this evidence report. docs/TASKS.md and all task statuses are unchanged.'
risks: 'The spoken-key rule is provisional. str::to_lowercase() performs Unicode lowercase mapping but is not complete Unicode case folding or normalization. The final production uniqueness and replacement-matching policy remains a prerequisite for vocabulary CRUD. T-017 migration behavior and T-001 remain incomplete; T-018 Windows owner-only ACL support and Linux filesystem validation remain outstanding, so T-038 prerequisites are incomplete. No CRUD, persistence, replacement, or frontend integration was added.'
review: PENDING

## Implemented contract

`MAX_VOCABULARY_ENTRIES` is 500. `VocabularyEntryRef<'a>` borrows `spoken` and `written`; collection validation does not mutate or retain these references. The size check runs first and returns `TooManyEntries { count, maximum }`. Then each entry is passed to the existing `validate_vocabulary_entry` in input order; the first failure is returned as `InvalidEntry { index, source }`, retaining the typed `VocabularyValidationError`. Only after all entries validate does duplicate detection run in input order. It creates the explicit ephemeral key `spoken.to_lowercase()`, records the first index, and returns `DuplicateSpoken { first_index, duplicate_index }` for the first conflict. Errors contain indices/reasons/counts but no vocabulary values. Written values do not participate in uniqueness.

The tests include the Unicode limitation: `CAFÉ` / `café` conflicts under lowercase comparison, while `straße` / `STRASSE` remains valid because the derived keys differ. Inputs are neither trimmed nor normalized. No word-boundary or transcript replacement takes place. Unicode full case folding and normalization remain unimplemented and undecided.

## Example output

```text
Outflow vocabulary validation demo
Valid vocabulary mapping: PASS
Empty spoken phrase: REJECTED
Overlong written replacement: REJECTED
Unicode scalar boundary: PASS
Vocabulary collection validation
500 entries: PASS
501 entries: REJECTED
Duplicate spoken phrase: REJECTED
Case-insensitive ASCII conflict: REJECTED
Unicode matching policy: PROVISIONAL
Vocabulary persistence: NOT IMPLEMENTED
Text replacement: NOT IMPLEMENTED
```

## Files changed by this task

- `crates/core/src/vocabulary.rs` — public collection API, typed errors, validation order and 23 collection-focused unit tests. This file was untracked before the task as part of the existing Task 41 work; its entry validator and tests were preserved.
- `crates/core/examples/vocabulary_validation.rs` — real synthetic calls for collection cases. This existing untracked Task 41 example was extended, not replaced.
- `README.md` — appended Task 42 collection-validation documentation to an already modified README, preserving its existing content.
- `evidence/rust-vocabulary-collection-task-42.md` — this report.

No dependency, lockfile, settings schema, task ledger, native, ASR, benchmark, or frontend source was changed. The existing branch and pre-existing uncommitted files were preserved. Repository T-042 remains the separate target-identity/commit-once task. T-017, T-018, T-038, T-039, and AC-08 remain incomplete; this request does not mark them complete. The next product prerequisite is to complete T-017/T-018 (including Linux filesystem validation and an approved Windows owner-only ACL strategy for T-018) and decide the production uniqueness/replacement-matching policy before T-038 vocabulary CRUD.
