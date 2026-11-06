# T-035/T-036 AC-07 Shared Evaluation Contract

```yaml
task_ids: [T-035, T-036]
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: Codex coordinator
branch: codex/ac07-shared-evaluation-contract
base_commit: 8ec1b7d194f634c63c6b7377311442dbe9d0e7b5
implementation_commit: 32f63b19763498ee0c4d4354a477d6f4be8a2a9b
platform: macOS 26.6.2 arm64; Python 3.13.13; Rust stable 1.99.1; Cargo 1.99.1
metrics: NOT_MEASURED (not applicable to this evaluator-only task)
review: PENDING
```

## Result

Added `scripts/evaluate_cleanup_ac07.py` with
`evaluate_cleanup_ac07_cases(cases)`. It materializes the input iterable once,
uses the existing protected-token scorer as the validator for the complete
shared records and reference offsets, then projects every validated record to
`id`, `expected`, and `actual` for the unchanged exact-output scorer. The
immutable result contains the two independent score objects and evaluation
fails if their case counts differ. It does not compute a combined percentage
or return an AC-07 acceptance flag.

The synthetic demo builds 200 cases in memory. It has 198 exact matches and
two punctuation mismatches after the annotated tokens. The explicitly
authored UTF-8 annotations cover one `2026` number and the accented proper
noun `Élodie`; neither is altered. The demo reads no cleanup fixtures, invokes
no Rust code, and writes no case contents to disk.

## Files changed

- `scripts/evaluate_cleanup_ac07.py`
- `scripts/tests/test_evaluate_cleanup_ac07.py`
- `docs/AC07_EVALUATION_PLAN.md`
- `docs/TASKS.md`
- `README.md`
- `evidence/T-035-T-036-ac07-protected-scoring-foundation.md` (delivery-status
  correction appended; failed-push history retained)
- `evidence/T-035-T-036-ac07-shared-evaluation-contract.md`

Neither existing scorer, Rust cleanup, fixtures, golden-corpus runner,
dependencies, lockfiles, persisted settings schema, PRD thresholds, or
`docs/CLEANUP_MODE_POLICY.md` changed. The policy remains **PROPOSED — NOT
APPROVED**. T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08,
and all release gates remain unchecked.

## Synthetic demo output

Command:

```text
PYTHONDONTWRITEBYTECODE=1 python3 scripts/evaluate_cleanup_ac07.py --demo
```

Observed output (visually checked against the requested summary):

```text
AC-07 combined evaluator: SYNTHETIC DEMO
Cases: 200
Exact matches: 198
Mismatches: 2
Exact-output rate: 99.00%
Minimum corpus size: PASS
Exact-output threshold: PASS
Protected numeric tokens: 1 (0 altered)
Protected proper nouns: 1 (0 altered)
Protected-token gate: PASS (synthetic only)
Shared case count: PASS
Final Clean/Verbatim policy: NOT APPROVED
AC-07 acceptance: NOT EVALUATED
```

## Verification

Commands ran from the repository root on macOS 26.6.2 arm64. Python was
3.13.13, `rustc` was stable 1.99.1, and Cargo was 1.99.1.

| Command | Result |
|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_evaluate_cleanup_ac07 -v` | PASS — 17 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_score_cleanup_ac07 scripts.tests.test_score_cleanup_protected_ac07 -v` | PASS — 48 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS — 460 tests, 1 skipped |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS — 1 test; all 200 characterization fixtures matched; acceptance remained `NOT EVALUATED` |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS — 9 tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — all workspace suites and doctests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS |
| `git diff --cached --check` | PASS after staging only intended task paths; unrelated untracked paths were excluded |

The complete Python suite printed diagnostic/help text from subprocess cases
while exiting successfully. No test or Rust check failed.

## Limitations

This verifies the coordinator and both existing scorers only. It does not
provide an independently reviewed acceptance corpus, approved Clean/Verbatim
semantics, production cleanup outputs for such a corpus, comprehensive
numeric/proper-noun coverage, or production integration evidence. The existing
200 characterization fixtures remain unapproved as acceptance data. Therefore
the demo and passing tests do not establish AC-07 acceptance.

The unrelated untracked `....` and `scripts/__pycache__/` paths were preserved
and excluded from the task commits.
