# T-035/T-036 AC-07 scoring foundation

**Status:** Partial evaluation-tool progress. T-035, T-036, and AC-07 remain
open; AC-07 acceptance is **NOT EVALUATED**.

## Task and revision references

- Task IDs: partial T-035/T-036 evaluation infrastructure
- Feature/flow/acceptance: F-07 / UF-05 / AC-07
- Task branch: `codex/ac07-exact-output-scorer`
- Base branch: `codex/task-035-036-cleanup-200-fixtures`
- Base commit: `1f6670b60952ceb4c9b40ff43afb1ba1fb2331f9`
- Implementation commit: `9acd1393281e67c6556d317abba3722b63e17c11`
- Evidence record: committed separately after the implementation commit
- Environment: macOS 26.6.2 (25G83), arm64; Python 3.13.13; Rust/Cargo
  stable 1.99.1

## Implementation

Added an importable, standard-library-only scorer that takes in-memory records
with unique nonempty string IDs and string `expected` and `actual` values. It
uses exact string equality and does not normalize case, punctuation,
whitespace, or Unicode. Empty outputs are valid. The result reports case,
match, mismatch, percentage, minimum-size, and threshold values. The 99%
threshold decision uses integer arithmetic.

The synthetic demo creates 200 string pairs in memory, with 198 exact matches
and two mismatches. It does not open `cleanup_current_behavior.json`, print
per-case values, or write case content to disk. The standard-library tests
build their synthetic collections in memory. No Python/Rust dependencies,
lockfiles, production code, fixture data, or cleanup policy were changed.

Actual demonstration output:

```text
AC-07 exact-output scorer: SYNTHETIC DEMO
Cases: 200
Exact matches: 198
Mismatches: 2
Exact-output rate: 99.00%
Minimum corpus size: PASS
Exact-output threshold: PASS
Protected-token acceptance: NOT EVALUATED
Final Clean/Verbatim policy: NOT APPROVED
AC-07 acceptance: NOT EVALUATED
```

## Commands and results

Python commands set `PYTHONDONTWRITEBYTECODE=1` to leave the pre-existing
untracked `scripts/__pycache__/` unchanged.

| Command | Result |
|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 scripts/score_cleanup_ac07.py --demo` | PASS; output above, visually inspected |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_score_cleanup_ac07 -v` | PASS; 17 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS; 412 tests, 1 skipped |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS; 200 current-behavior fixtures, 8 categories, 8 protected numeric and 8 protected proper-noun annotations unchanged; runner reports AC-07 acceptance NOT EVALUATED |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS; 9 tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; exit 0 |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS |
| `git diff --cached --check` | PASS; only the five intended implementation/documentation paths were staged for the implementation commit |
| `git diff --quiet 1f6670b60952ceb4c9b40ff43afb1ba1fb2331f9..HEAD -- crates/settings/tests/cleanup_golden_corpus.rs crates/settings/tests/fixtures/cleanup_current_behavior.json` | PASS; neither existing Rust runner nor fixture file changed |

## Limitations and remaining AC-07 evidence

The synthetic score verifies arithmetic only. It is not a cleanup accuracy
measurement and does not establish acceptance. The 200 current-behavior
fixtures remain characterization fixtures; their passing tests do not make
them an independently approved reference oracle. Still required are:

- independently reviewed expected outputs and an explicitly identified
  acceptance corpus;
- actual outputs from the production cleanup path;
- a separate protected-token evaluation showing zero altered numeric and
  proper-noun tokens in the protected acceptance cases;
- approved Clean/Verbatim semantics and verified distinction; and
- relevant production integration evidence.

[`docs/AC07_EVALUATION_PLAN.md`](../docs/AC07_EVALUATION_PLAN.md) describes the
proposed procedure. [`docs/CLEANUP_MODE_POLICY.md`](../docs/CLEANUP_MODE_POLICY.md)
remains **PROPOSED — NOT APPROVED**. This task does not implement the protected
token evaluator, modify PRD thresholds, or complete T-035/T-036. T-017, T-018,
T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain
unchecked. No 4 GB hardware, release, or platform acceptance claim is made.

## Changed paths and review

- `scripts/score_cleanup_ac07.py`
- `scripts/tests/test_score_cleanup_ac07.py`
- `docs/AC07_EVALUATION_PLAN.md`
- `README.md`
- `docs/TASKS.md`
- `evidence/T-035-T-036-ac07-scoring-foundation.md`

The existing untracked `....` and `scripts/__pycache__/` paths were preserved
and not staged. Maintainer review: **PENDING**.
