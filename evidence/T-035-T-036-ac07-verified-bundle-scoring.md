# T-035/T-036 AC-07 verified-bundle scoring evidence

**Result:** Synthetic scoring ran; formal AC-07 acceptance did not. The checked-in synthetic data remains unchanged and intentionally fails the numerical and protected-token gates.

## Branch and commits

- Branch: `codex/ac07-verified-bundle-scoring`
- Base commit: `bc1ace4b5bf62c63a6845e8eed6a35dadd514b50`
- Implementation commit: `5b80361f7342b35b8fdb7665303b143a6bc550d4`
- Final branch SHA: this evidence is committed after the implementation commit; the final SHA is reported with the task handoff.

## Files changed

- `scripts/score_cleanup_ac07_bundles.py` — validates the four in-memory objects, associates cases by exact ID in manifest order, then calls the existing shared coordinator.
- `scripts/tests/test_score_cleanup_ac07_bundles.py` — 26 standard-library tests for assembly, scoring, deterministic ordering, strict loading, privacy of demo output, immutability, and validation failures.
- `docs/AC07_EVALUATION_PLAN.md`, `docs/AC07_CORPUS_MANIFEST.md`, and `README.md` — document the development-only flow, expected synthetic failures, and formal acceptance prerequisites.
- `docs/TASKS.md` — records partial evaluation-infrastructure progress; T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and release gates remain unchecked.
- This evidence file.

The four synthetic JSON artifacts, existing validators and scorers, 200 characterization fixtures, Rust production code, PRD thresholds, and cleanup policy were not changed. `docs/CLEANUP_MODE_POLICY.md` remains **PROPOSED — NOT APPROVED**.

## Synthetic demonstration

Command:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/score_cleanup_ac07_bundles.py --demo
```

Observed output:

```text
AC-07 verified-bundle scoring: SYNTHETIC DEMO
Corpus ID: ac07-synthetic-manifest-demo
Revision: draft-001
Run ID: synthetic-run-001
Verified case associations: 3/3
Exact matches: 0
Mismatches: 3
Exact-output rate: 0.00%
Minimum corpus size: FAIL
Exact-output threshold: FAIL
Protected numeric tokens: 1 (1 altered)
Protected proper nouns: 1 (1 altered)
Protected-token gate: FAIL
Output source: SYNTHETIC DEMO
Production execution: NOT VERIFIED
Reference review: PENDING
Corpus approval: NOT ESTABLISHED
AC-07 acceptance: NOT EVALUATED
```

The command exited successfully after all four artifacts validated and all three cases were scored. Output contained only corpus/run metadata, counts, and statuses; no reference, actual-output, or annotation bundle contents were printed. The `demo-003` `proper_noun` annotation is a contract example and does not establish natural-language name preservation.

## Python verification

Environment: macOS 26.6.2, Darwin arm64; Python 3.13.13.

| Command | Result |
|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 scripts/score_cleanup_ac07_bundles.py --demo` | PASS — complete validation and scoring; expected failing synthetic gates above. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_score_cleanup_ac07_bundles -v` | PASS — 26 tests. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_actual_outputs -v` | PASS — 40 tests. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_evaluate_cleanup_ac07 -v` | PASS — 17 tests. |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS — 610 tests, 1 skipped. |

The integration tests confirm exact-ID assembly, manifest-order determinism, all three mismatches included, numeric and synthetic proper-noun alterations counted, all validated cases reaching the coordinator, three-case minimum and zero-match threshold failures, separate protection-gate failure, and independently authored in-memory matching outputs. The mismatch outside annotated ranges changed exact-output scoring without changing protected-token results.

Negative-path tests confirm that missing or duplicate actual IDs, a corrupted reference hash, an invalid protected annotation, and an invalid UTF-8 actual string raise before the shared coordinator is called and return no partial result. The reused strict JSON loader also rejected duplicate object keys, malformed JSON, and the unsupported `NaN` constant before scoring.

## Rust verification

Environment: Rust stable, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; Cargo 1.99.1.

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS — 1 test; the unchanged 200-fixture characterization corpus passed, with 8 numeric and 8 proper-noun tokens unchanged. It reported AC-07 acceptance not evaluated. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS — 9 tests. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — workspace unit, integration, and documentation tests completed successfully. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS. |
| `git diff --check` | PASS. |
| `git diff --cached --check` | PASS after staging only the intended implementation, tests, and documentation files. |

## Acceptance limits

These results show that the integration validates and scores the synthetic bundles. They do not establish formal AC-07 acceptance. Formal evaluation still requires an independently reviewed and approved acceptance corpus with at least 200 eligible transformations, approved Clean/Verbatim semantics, authenticated production outputs, production integration evidence, and passing formal numerical and protected-token gates on that approved corpus. No production cleanup was run, no production source was authenticated, and no acceptance decision was made. The synthetic score failures are expected and do not indicate an integration-program failure.
