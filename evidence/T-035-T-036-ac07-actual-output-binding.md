---
task_id: T-035/T-036 evaluation-infrastructure only
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: codex/ac07-actual-output-binding
base_commit: a8073acca85c4be634c787170571885fe4c7a88b
result_commit: pending implementation commit
platform: macOS 26.6.2 (25G83), Darwin arm64; Python 3.13.13; Rust/Cargo 1.99.1 stable
implementation: Standalone standard-library binder associates a separate synthetic actual-output bundle with verified AC-07 reference identity and case IDs
metrics: NOT_MEASURED; no production cleanup, accuracy, protected-token scoring, performance, or release measurements
doc_updates: docs/AC07_CORPUS_MANIFEST.md; docs/AC07_EVALUATION_PLAN.md; docs/TASKS.md; README.md
risks: Source declarations are unauthenticated; no production output or acceptance corpus was used; AC-07 correctness and acceptance remain unevaluated
review: PENDING
---

# T-035/T-036 AC-07 actual-output binding — partial infrastructure

## Outcome

Added a version 1 synthetic actual-output bundle and a standard-library
validator. It calls the existing protected-reference validator before
inspecting actual outputs, then checks corpus/revision identity, source-kind
declarations, exact case-ID correspondence, and UTF-8 encodability. Cases are
matched independent of array order; empty output strings remain valid. The
frozen result contains identity and counts only, and its production-verification
field is fixed to `false`.

The validator does not run cleanup, derive actual strings from expected
strings, compare actual to expected, score protected-token preservation, or
invoke the shared AC-07 coordinator. A nonempty production execution-evidence
reference is only a declaration, not proof production code ran.

## Changed files

- `scripts/validate_cleanup_ac07_actual_outputs.py`
- `scripts/testdata/synthetic_ac07_actual_outputs.json`
- `scripts/tests/test_validate_cleanup_ac07_actual_outputs.py`
- `docs/AC07_CORPUS_MANIFEST.md`
- `docs/AC07_EVALUATION_PLAN.md`
- `README.md`
- `docs/TASKS.md`
- `evidence/T-035-T-036-ac07-actual-output-binding.md`

Existing manifest hashes, reference strings, protected annotations, validators,
scorers, coordinator behavior, Rust cleanup/settings code, the 200
characterization fixtures, dependency files, and `docs/CLEANUP_MODE_POLICY.md`
were not changed. T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07,
AC-08, and every release gate remain unchecked. The Clean/Verbatim policy
remains **PROPOSED — NOT APPROVED**.

## Demonstration

Command:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_actual_outputs.py --demo
```

Observed output:

```text
AC-07 actual-output binding: SYNTHETIC DEMO
Corpus ID: ac07-synthetic-manifest-demo
Revision: draft-001
Run ID: synthetic-run-001
Reference cases: 3
Actual-output cases: 3
Case-ID correspondence: PASS
Actual UTF-8 strings validated: 3/3
Output source: SYNTHETIC DEMO
Production execution: NOT VERIFIED
Reference review: PENDING
Corpus approval: NOT ESTABLISHED
AC-07 acceptance: NOT EVALUATED
```

The output was visually checked line by line against the requested summary.
It contains no input, expected, actual, or protected-token strings and makes no
production or acceptance claim.

## Verification

All commands ran from the repository root. The Python full-suite run printed
diagnostic output from existing negative-path CLI tests and exited successfully.

| Command | Result |
|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_actual_outputs.py --demo` | PASS; output above |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_actual_outputs -v` | PASS; 40 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_protected_references -v` | PASS; 31 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_reference_bundle -v` | PASS; 31 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_evaluate_cleanup_ac07 -v` | PASS; 17 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS; 584 tests, 1 skipped |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS; 1 test and 200 current-behavior fixtures; AC-07 acceptance not evaluated |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS; 9 tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; workspace tests and doctests, exit 0 |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS before staging |
| `git diff --cached --check` | PASS; only the eight intended task files staged |

Environment: macOS 26.6.2 (25G83), Darwin arm64, Python 3.13.13, Rust/Cargo
1.99.1 stable. Hardware model and memory were not measured; the attempted
`hw.memsize` query was denied by the environment. This task measured no
performance or release-resource metrics.

## Defects corrected

No implementation defects surfaced in verification. One test assertion was
corrected during authoring, before the focused test run, so that the demo test
checks printed case content rather than treating a shared identifier string
as a protected-token value.

## Limitations

- The sample is synthetic test data and is not an acceptance corpus.
- `production_declared` plus a reference string remains unauthenticated; the
  summary always reports production execution as unverified.
- Identity binding does not establish cleanup correctness, exact-output
  accuracy, or protected-token preservation.
- Formal evaluation still requires an approved acceptance corpus, approved
  Clean/Verbatim semantics, authenticated production execution, and integration
  evidence.
- The authoritative AC-07 thresholds and release gates are unchanged. No Rust
  output was generated for this task.

## Branch and commits

Branch: `codex/ac07-actual-output-binding`.

Base commit: `a8073acca85c4be634c787170571885fe4c7a88b`.

The implementation commit SHA and final evidence/branch SHA will be recorded
in the closeout handoff after commit and push verification.
