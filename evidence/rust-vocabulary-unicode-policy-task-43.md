---
task_id: Task 43 (Unicode policy review; separate from repository T-043)
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: codex/task23-moonshine-build-scope
base_commit: 6f15c8f
result_commit: uncommitted
platform: macOS 26.6.2 (25G83), Apple M3 Pro, arm64; 18 GB installed physical RAM
implementation: "Proposed D145 policy record, Python standard-library reference and tests, and Rust regression characterization; production comparator unchanged"
commands:
  - {command: 'python3 scripts/check_vocabulary_unicode_policy.py --demo', result: PASS, notes: 'Printed all 12 candidate outcomes and Python Unicode data version 15.1.0.'}
  - {command: 'python3 -m unittest scripts.tests.test_check_vocabulary_unicode_policy -v', result: PASS, notes: '15 tests passed.'}
  - {command: 'python3 -m unittest discover -s scripts/tests -v', result: PASS, notes: '241 tests passed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked', result: PASS, notes: '39 vocabulary-filtered tests passed; 2 non-vocabulary crate tests filtered out.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'No formatting changes required.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Workspace checked.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '111 unit tests passed: 41 core and 70 settings; 0 doc tests.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace built.'}
  - {command: 'npm run typecheck', result: PASS, notes: 'TypeScript check completed.'}
  - {command: 'npm run test -- --run', result: PASS, notes: '136 tests across 10 files passed.'}
  - {command: 'npm run lint', result: PASS, notes: 'ESLint completed.'}
  - {command: 'npm run build', result: PASS, notes: 'Typecheck and Vite production build completed.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo tree --workspace --locked -i unicode-ident', result: PASS, notes: 'Confirmed unicode-ident is used transitively by proc-macro2/syn/serde_derive, not for D145 matching.'}
  - {command: 'git diff --check', result: PASS, notes: 'No whitespace errors.'}
metrics: "Python Unicode data version: 15.1.0 (Python 3.13.13); resource gates not applicable"
doc_updates: "README.md and docs/VOCABULARY_UNICODE_POLICY.md; docs/TASKS.md intentionally unchanged"
risks: "Candidate algorithm and production Unicode data version remain unapproved; Rust lowercase-only comparison is unchanged. T-017, T-018, T-038, T-039, and AC-08 remain incomplete."
review: PENDING
---

# Task 43 — Unicode vocabulary policy review

## Requirements and policy

Reviewed F-08, UF-06, C-06/C-08, TS-06/TS-08, T-017/T-018/T-038/T-039, and AC-08. The repository maps task ID T-043 to the macOS insertion adapter; this evidence is explicitly for the user's separate Task 43 and does not change T-043 or any task checkbox.

The candidate policy is Unicode canonical caseless matching D145: `NFD(toCasefold(NFD(input)))`. Its references are Unicode Standard Version 18.0.0 and UAX #15 Version 58 / Unicode 18.0.0. The Python 3.13.13 development runtime reports Unicode data 15.1.0, so the reference output does not claim Unicode 18.0 conformance.

The production collection validator remains unchanged and continues to derive duplicate keys using `spoken.to_lowercase()`. Existing error precedence remains collection size, all entry validations in input order, then duplicate checks in input order. Rust regression tests characterize ASCII and `CAFÉ`/`café` conflicts plus the current non-conflicts for sharp-s and composed/decomposed accented forms.

## Reference outcomes

Captured from `python3 scripts/check_vocabulary_unicode_policy.py --demo`:

```text
Python Unicode data version: 15.1.0
ASCII case variation: EQUAL
Non-ASCII case variation: EQUAL
German sharp-s variation: EQUAL
Canonical accent variation: EQUAL
Greek sigma variation: EQUAL
Greek final sigma variation: EQUAL
Multiword case variation: EQUAL
Trailing whitespace distinction: DIFFERENT
Turkish dotless-I distinction: DIFFERENT
Turkish dotted-I distinction: DIFFERENT
Compatibility numeral distinction: DIFFERENT
Cross-script confusable distinction: DIFFERENT
```

The demo also printed `Production Rust validator: UNCHANGED`, `Candidate production policy: NOT APPROVED`, `Vocabulary persistence: NOT IMPLEMENTED`, and `Text replacement: NOT IMPLEMENTED`.

The current Rust validator agrees on the tested `rust` / `Rust` and `CAFÉ` / `café` duplicates. Its lowercase-only keys disagree with the candidate for `straße` / `STRASSE` and composed `é` / decomposed `e\u{301}`: Rust currently treats each pair as distinct. These known differences are characterization tests, not approval of lowercase-only behavior.

## Dependency findings

The core crate currently has no dependencies; settings uses the approved `serde` and `serde_json` stack. The workspace has no direct Rust dependency providing both canonical normalization and full case folding. TS-06 describes Unicode-aware deterministic cleanup but does not select such a library. `cargo tree` shows `unicode-ident` 1.0.27 only transitively under `proc-macro2`/`syn`/`serde_derive`; it does not implement D145. New crate selection, version, license, Unicode data, and platform-behavior review require approval before production dependency changes. No manifest or lockfile changed.

## Verification results

- Python focused suite: 15 passed. Full `scripts/tests` suite: 241 passed (226 prior baseline + 15 new).
- Rust focused vocabulary filter: 39 passed, 2 filtered. Full workspace: 111 unit tests passed (41 core + 70 settings; 110 prior baseline + 1 new). Doc tests: 0.
- Rust format, workspace check, Clippy with warnings denied, and workspace build: PASS.
- Frontend typecheck, lint, and production build: PASS. Vitest: 136 tests across 10 files passed; no frontend source changed.
- `git diff --check`: PASS. Moonshine CMake configuration was not attempted.

No physical memory, installer-size, ASR, or platform insertion gate is relevant to this documentation/reference task; those remain unmeasured and incomplete elsewhere.

## Remaining decisions

- Select a verified Rust implementation and pinned Unicode data version; establish cross-platform deterministic keys.
- Decide how a future policy update handles existing entries that become duplicates without rewriting original values.
- Specify matching consistency and offset/boundary behavior for future word-boundary replacement.
- T-017 remains incomplete pending migration behavior and its T-001 prerequisite.
- T-018 remains incomplete: Windows owner-only ACL/atomic replacement is not implemented, and Linux filesystem behavior is untested.
- Complete T-017 and T-018 before T-038; keep T-038, T-039, and AC-08 incomplete.

## Git review

The existing branch remains `codex/task23-moonshine-build-scope` at base `6f15c8f`; no stage, commit, branch change, or history rewrite was made. Final status contains only the two intended modifications (`README.md`, `crates/core/src/vocabulary.rs`) and four new files (this evidence, the policy record, and the Python script/test). `git diff --check` passed. No dependency manifest, Cargo lockfile, settings schema, frontend source, or `docs/TASKS.md` changed.
