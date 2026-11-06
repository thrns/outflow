# T-035/T-036 AC-07 Reference-Bundle Binding — Partial Infrastructure

```yaml
task_id: T-035/T-036 infrastructure only
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: codex/ac07-reference-bundle-binding
base_commit: 35f833324c1094939d41e348de8a078c55a2fadb
result_commit: e97e02717f98faafe64f2414f2420f13342835f0
platform: macOS 26.6.2 (25G83), Darwin arm64; Python 3.13.13; Rust 1.99.1 stable
implementation: Standalone standard-library validator binds exact case IDs and UTF-8 SHA-256 input/expected content to existing manifest hashes; synthetic bundle only.
metrics: NOT_MEASURED; no production cleanup, recognition, quality, performance, or release measurements
review: PENDING
```

## Changed files

- `scripts/validate_cleanup_ac07_reference_bundle.py`
- `scripts/testdata/synthetic_ac07_reference_bundle.json`
- `scripts/tests/test_validate_cleanup_ac07_reference_bundle.py`
- `docs/AC07_CORPUS_MANIFEST.md`
- `docs/AC07_EVALUATION_PLAN.md`
- `docs/TASKS.md`
- `README.md`
- `evidence/T-035-T-036-ac07-reference-bundle-binding.md`

The existing manifest and hashes, manifest validator, scorers/coordinator,
Rust cleanup, 200 characterization fixtures, and Clean/Verbatim policy were
not changed. T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08,
and all release gates remain unchecked. `docs/CLEANUP_MODE_POLICY.md` remains
**PROPOSED — NOT APPROVED**.

## Demonstration

Command:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_reference_bundle.py --demo
```

Observed output:

```text
AC-07 reference-bundle binding: SYNTHETIC DEMO
Corpus ID: ac07-synthetic-manifest-demo
Revision: draft-001
Manifest cases: 3
Bundle cases: 3
Case-ID correspondence: PASS
Input SHA-256 hashes verified: 3/3
Expected SHA-256 hashes verified: 3/3
Reference review: PENDING
Corpus approval: NOT ESTABLISHED
AC-07 acceptance: NOT EVALUATED
```

The output was visually checked against the requested summary. It contains no
input or expected-output strings. It reports success only after all three IDs
and all six hashes match.

## Verification

| Command | Result |
|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_reference_bundle -v` | PASS — 31 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_manifest -v` | PASS — 22 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_evaluate_cleanup_ac07 -v` | PASS — 17 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS — 513 tests, 1 skipped |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS — 200 current-behavior characterization fixtures; output states AC-07 acceptance is not evaluated |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS — 9 tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — workspace tests and doctests; exit 0 |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS |
| `git diff --cached --check` | PASS — only the eight intended task files staged |

## Limitations

This verifies only correspondence between the synthetic bundle and the
pre-existing manifest's declared hashes. A match does not prove that expected
outputs are correct or independently approved. The validator does not run
cleanup, score accuracy, create evaluation records, authenticate review
evidence, or establish AC-07 acceptance. No acceptance corpus, model, audio,
customer transcript, production output, resource metric, or release gate was
used or evaluated. Existing 200-case cleanup fixtures remain
current-behavior characterization only.

## Next handoff

The implementation commit is recorded above. This evidence update is a
follow-up commit; push the task branch and verify the local HEAD, upstream
reference, and remote SHA match. Preserve the unrelated untracked `....` and
`scripts/__pycache__/` paths.
