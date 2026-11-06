---
task_id: T-035/T-036 partial evaluation-infrastructure work
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: coordinator / codex/ac07-corpus-manifest-foundation
base_commit: 3f2cccecba56aa7d02dc9e760690d55961f0a423
result_commit: 84a19e2a0c4e95985877544bfb9a2fd842739ffb
platform: macOS 26.6.2 (25G83), arm64; Python 3.13.13; Rust/Cargo 1.99.1
implementation: Version 1 metadata-only manifest schema, standard-library validator, three-case synthetic draft, tests, and synchronized documentation
doc_updates: docs/AC07_CORPUS_MANIFEST.md; docs/AC07_EVALUATION_PLAN.md; README.md; docs/TASKS.md
risks: Hashes are syntax-checked only; no separate case-data bundle or approved references were verified; review evidence is not authenticated; no corpus approval or AC-07 acceptance was established
review: PENDING
---

# T-035/T-036 AC-07 corpus manifest foundation

## Outcome

Added a strict version 1 JSON metadata contract and standalone Python validator. The frozen summary reports corpus identity and declared review state, indexed/unique case counts, represented categories, Clean/Verbatim counts, cases declaring numeric/proper-noun protection, pending reference-review count, and syntactic validity. The validator rejects missing/unknown fields, duplicate JSON keys, invalid types/values, malformed digests, duplicate IDs/protection kinds, and synthetic review claims. Error messages identify fields or case positions and do not print case content.

The three-case synthetic manifest uses genuine lowercase SHA-256 digests. Its hashes are tested against explicit synthetic source strings held only in the Python test file; no case strings are stored in the manifest. The manifest is pending and provides no approval or acceptance flag.

## Files changed

- `docs/AC07_CORPUS_MANIFEST.md` — schema, review/hash limits, command usage.
- `docs/AC07_EVALUATION_PLAN.md` — separates the exact-output scorer, protected-token evaluator, shared coordinator, and manifest/validator; adds future hash/reference/output/ID verification requirements.
- `README.md` — manifest purpose, command, expected output, digest format, and troubleshooting.
- `docs/TASKS.md` — partial T-035/T-036 infrastructure progress; requested task and release statuses remain unchecked.
- `scripts/validate_cleanup_ac07_manifest.py` — standalone metadata validator and CLI.
- `scripts/testdata/synthetic_ac07_manifest.json` — pending synthetic metadata sample, three cases in three categories.
- `scripts/tests/test_validate_cleanup_ac07_manifest.py` — 22 standard-library tests.
- `evidence/T-035-T-036-ac07-corpus-manifest-foundation.md` — this verification record.

The existing scorers and coordinator, 200 characterization fixtures and Rust runner, production cleanup/settings code, lockfiles, and `docs/CLEANUP_MODE_POLICY.md` were not changed. The policy remains **PROPOSED — NOT APPROVED**.

## Demonstration output

Command:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_manifest.py --demo
```

Observed output after successful validation:

```text
AC-07 corpus manifest: SYNTHETIC DRAFT
Schema version: 1
Cases indexed: 3
Unique case IDs: PASS
Categories represented: 3/8
Clean and Verbatim represented: PASS
Content hashes: FORMAT VALIDATED ONLY
Reference review: PENDING
Corpus approval: NOT ESTABLISHED
AC-07 acceptance: NOT EVALUATED
```

## Verification

All checks below ran from the repository root on the environment recorded above.

| Command | Result |
|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_manifest.py --demo` | PASS; output above |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_manifest -v` | PASS; 22 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_evaluate_cleanup_ac07 -v` | PASS; 17 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -p 'test_*.py'` | PASS; 482 tests, 1 skipped |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS; 200 characterization fixtures across eight categories; current behavior baseline only; AC-07 acceptance not evaluated |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS; 9 tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 500 tests across workspace test targets and doctests |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS before staging |
| `git diff --cached --check` | PASS after staging only the intended task files |

The demo output was visually inspected against the requested ten-line summary. Both explicit status lines state that corpus approval is not established and AC-07 acceptance was not evaluated.

The full Python suite printed diagnostic text from existing negative-path CLI tests and reported one skipped test; the suite exited successfully. No check failed.

## Limitations and remaining work

- Hashes are checked for 64-character lowercase hexadecimal form only. There is no supplied case-data bundle to verify digest-to-byte correspondence.
- A `review_evidence_ref` is only a declaration; this validator cannot authenticate evidence or approve a corpus.
- The sample is synthetic metadata, not an acceptance corpus. No production cleanup was run and neither scorer was called by this validator.
- Case-ID correspondence, independently approved reference bytes, production outputs, final Clean/Verbatim policy, and all formal AC-07 gates remain for later work. The 200 characterization cases remain unchanged and are not accepted as oracle data.
- T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked. The 200-case and 99% thresholds are unchanged.

## Branch and status

- Task branch: `codex/ac07-corpus-manifest-foundation`
- Base branch and commit: `codex/ac07-shared-evaluation-contract` at `3f2cccecba56aa7d02dc9e760690d55961f0a423`
- Implementation result commit: `84a19e2a0c4e95985877544bfb9a2fd842739ffb` on the task branch. This evidence file is finalized in a subsequent evidence-only commit; the final branch tip is recorded in the task delivery.
- Corpus review: **PENDING**
- Corpus approval: **NOT ESTABLISHED**
- AC-07 acceptance: **NOT EVALUATED**
- Review: **PENDING**
