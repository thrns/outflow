# T-035/T-036 AC-07 Protected-Token Scoring Foundation

```yaml
task_ids: [T-035, T-036]
feature_ids: [F-07]
acceptance: [AC-07]
owner: Codex coordinator
branch: codex/ac07-protected-token-evaluator
base_commit: 333007035812aae27e6d62f1d9599dd866a647d2
implementation_commit: 6b1a051b1d0f4ecc28132f0d3723afdf9496ce3b
evidence_snapshot_commit: f786bce9eba1a40e3a598540222027943b696192
platform: macOS 26.6.2 arm64; Python 3.13.13; Rust stable 1.99.1; Cargo 1.99.1
metrics: NOT_MEASURED (not applicable to this evaluator-only task)
review: PENDING
```

## Result

Added an independent, standard-library Python evaluator that validates
explicit numeric and proper-noun annotations against UTF-8 input and reviewed
expected-output byte ranges, then checks those expected-output offsets in
actual output. Invalid reference metadata raises
`ProtectedCleanupValidationError`; actual-output differences, missing ranges,
out-of-bounds ranges, and ranges that split an actual UTF-8 character count as
alterations. The immutable summary reports case/token counts, kind coverage,
zero-alteration status, and the protected-token gate.

The evaluator does not read fixtures, derive expected values or annotations
from actual output, or invoke Rust cleanup. The exact-output scorer's behavior
is unchanged.

## Files changed

- `scripts/score_cleanup_protected_ac07.py`
- `scripts/tests/test_score_cleanup_protected_ac07.py`
- `docs/AC07_EVALUATION_PLAN.md`
- `README.md`
- `docs/TASKS.md`
- `evidence/T-035-T-036-ac07-protected-scoring-foundation.md`

No production Rust cleanup, Rust runner, fixtures, dependencies, lockfiles,
settings schema/defaults, PRD thresholds, or Clean/Verbatim policy changed.

## Synthetic demo output

Command:

```text
PYTHONDONTWRITEBYTECODE=1 python3 scripts/score_cleanup_protected_ac07.py --demo
```

Observed output:

```text
AC-07 protected-token scorer: SYNTHETIC DEMO
Cases: 2
Annotated cases: 2
Protected numeric tokens: 1 (0 altered)
Protected proper nouns: 1 (0 altered)
Both token kinds represented: PASS
Protected-token gate: PASS (synthetic only)
AC-07 acceptance: NOT EVALUATED
```

The two manual annotations use separate input/expected offsets for a
punctuation shift and include the accented proper noun `Élodie`. The demo
prints only counts and statuses.

## Verification

All commands ran from the repository root. Results:

| Command | Result |
|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_score_cleanup_protected_ac07 -v` | PASS — 31 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_score_cleanup_ac07 -v` | PASS — 17 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS — 443 tests, 1 skipped |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS — 1 runner test; printed 200 current-behavior fixtures and `AC-07 acceptance: NOT EVALUATED` |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS — 9 tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — all workspace test suites and doctests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS |
| `git diff --cached --check` | PASS |

## Corrections and limitations

- A protected token string containing an unpaired surrogate initially could
  escape as a raw `UnicodeEncodeError`. The evaluator now reports it as a
  content-free `ProtectedCleanupValidationError`; a focused test covers it.
- Missing fields are named in validation errors, while unknown field names and
  transcript contents are not printed.
- The demo and tests use synthetic in-memory records only. They do not prove
  comprehensive number or proper-noun preservation, cleanup quality, or AC-07
  acceptance.
- An insertion or deletion before a token in actual output can move it away
  from its independently declared expected-output offset and conservatively
  count as an alteration. The evaluator intentionally does not search for or
  adjust the token's offset.
- Formal AC-07 evaluation still requires both independent scorers to run on
  the same independently approved acceptance corpus and corresponding
  production outputs, plus final Clean/Verbatim semantics and production
  integration evidence. The current 200-case characterization is not thereby
  approved as that corpus.
- `docs/CLEANUP_MODE_POLICY.md` remains **PROPOSED — NOT APPROVED**. T-017,
  T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and every release
  gate remain unchecked.
- The original untracked `....` and `scripts/__pycache__/` paths were left
  untouched.

## Branch delivery status

The implementation and evidence are committed locally on
`codex/ac07-protected-token-evaluator`. Two
`git push --set-upstream origin codex/ac07-protected-token-evaluator` attempts
were rejected with `remote: fatal error in commit_refs`; a subsequent
read-only `git ls-remote --heads` found no remote branch. The connected GitHub
branch-creation tool also returned HTTP 403 (`Resource not accessible by
integration`). Therefore the local branch has no upstream tracking reference,
and local HEAD, upstream, and remote branch SHA cannot be confirmed equal.
Retry publication after repository write access or remote ref updates are
available; do not treat the branch as pushed.
