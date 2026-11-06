# T-035/T-036 Supporting Evidence — AC-07 Protected Reference Binding

**Status:** IN PROGRESS — reference-validation infrastructure only. This evidence
does not complete T-035, T-036, or AC-07.

```yaml
task_id: T-035/T-036 supporting infrastructure only
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: /root
branch: codex/ac07-protected-reference-binding
base_commit: 32f480e55e01e774cf2897abe568ca68dc139176
result_commit: e90f25b (implementation, synthetic fixture, tests, and docs)
platform: macOS 26.6.2 (25G83), arm64; Python 3.13.13; stable Rust 1.99.1
host_ram: NOT_MEASURED; sysctl access was denied. This is not 4 GB hardware certification.
implementation: Separate synthetic annotation bundle and validator bind exact IDs and declared kinds to SHA-256-verified reference strings; protected scorer is reused only for input/expected UTF-8 range validation with actual=expected, and its scores are discarded.
metrics: No production cleanup or actual-output scoring; AC-07 acceptance NOT EVALUATED; app resource/performance metrics NOT_MEASURED.
review: PENDING
```

## Changed files

- `scripts/testdata/synthetic_ac07_protected_annotations.json`
- `scripts/validate_cleanup_ac07_protected_references.py`
- `scripts/tests/test_validate_cleanup_ac07_protected_references.py`
- `docs/AC07_CORPUS_MANIFEST.md`
- `docs/AC07_EVALUATION_PLAN.md`
- `README.md`
- `docs/TASKS.md`
- This evidence file.

The pre-existing manifest, reference strings and hashes, protected scorer,
evaluation coordinator, cleanup policy, and Rust production cleanup code were
not changed. Unrelated untracked paths `....` and `scripts/__pycache__/` were
preserved.

## Synthetic demonstration

Command:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_protected_references.py --demo
```

Observed output:

```text
AC-07 protected-reference binding: SYNTHETIC DEMO
Corpus ID: ac07-synthetic-manifest-demo
Revision: draft-001
Reference cases: 3
Annotation cases: 3
Case-ID correspondence: PASS
Protected kind declarations: PASS
Protected numeric references: 1
Protected proper-noun references: 1
UTF-8 reference ranges: PASS
Reference review: PENDING
Corpus approval: NOT ESTABLISHED
AC-07 acceptance: NOT EVALUATED
```

Visual output review found only the fixed labels, corpus identity, revision,
and verified counts. It printed no reference strings, annotation text, or
production scores and made no approval or acceptance claim.

## Verification

All commands below ran from the repository root on the environment listed
above. Results are local checks, not cross-platform or release certification.

| Command | Result |
|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_protected_references -v` | PASS — 31 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_reference_bundle -v` | PASS — 31 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_score_cleanup_protected_ac07 -v` | PASS — 31 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_evaluate_cleanup_ac07 -v` | PASS — 17 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS — 544 tests, 1 skipped |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS — 1 test; 200 characterization fixtures; AC-07 acceptance remained NOT EVALUATED |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS — 9 tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — all workspace unit, integration, and doctests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS |
| `git diff --cached --check` | PASS after staging all eight intended task files, including this evidence |

The full Python discovery run printed output from existing CLI-oriented helper
tests while they exercised argument handling and invalid synthetic fixtures;
the suite exited successfully. No production cleanup was run by the new
validator.

## Source review and corrections

- Replaced the illustrative numeric `[0, 1]` ranges with manually counted
  offsets for the existing `demo-001` reference strings: input `[37, 40]`,
  expected `[40, 43]`. The strings and their manifest hashes were unchanged.
- Confirmed the existing reference binder validates manifest metadata, exact
  IDs, and SHA-256 content before annotation checks.
- Confirmed the protected scorer validates each input and expected range,
  UTF-8 boundaries, selected bytes, and overlap. Its result is discarded; its
  `actual=expected` argument is used only to reach those existing reference
  checks.
- Added coverage for unsupported versions, repeated same-kind annotations,
  expected-range overlap, content-redacted errors, malformed/invalid UTF-8
  files, and hash failure taking precedence over annotation checks.
- No defect was found in the existing scorer, coordinator, manifest, or
  reference bundle; none was modified.

## Remaining limitations

- The checked-in inputs, references, and annotations are synthetic only.
- The `AC-07` identifier in `demo-003` is labeled `proper_noun` only to
  demonstrate the annotation contract; this does not evidence natural-language
  proper-noun coverage.
- Valid ranges show only that annotations select the declared bytes in the
  hash-bound reference strings. They do not authenticate independent review or
  prove that production cleanup preserves any token.
- No production actual outputs, acceptance corpus, formal review, or AC-07
  evaluation was created or run. The authoritative AC-07 thresholds were not
  changed.
- T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and release
  gates remain unchecked. `docs/CLEANUP_MODE_POLICY.md` remains
  **PROPOSED — NOT APPROVED**.
- No 4 GB hardware, app performance, cross-platform, or release metrics were
  measured.

## Commit and handoff

Implementation/docs/tests commit: `e90f25b` on
`codex/ac07-protected-reference-binding`, based on
`32f480e55e01e774cf2897abe568ca68dc139176`. This evidence is being added in a
follow-up commit; final local/upstream/remote SHA equality is recorded in the
task handoff after push.
