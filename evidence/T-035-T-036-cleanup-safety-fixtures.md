# Cleanup safety fixture expansion — T-035/T-036 partial evidence

```yaml
task_id: T-035/T-036 (partial)
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: Codex
branch: codex/task-036-cleanup-safety-fixtures
base_commit: 76a09a72a757b4b902e1cb1ce9d438ec0ce66baa
implementation_commit: 9cb63cb5e71f1cdf3891f05c36ed8b44db8e5468
platform: macOS 26.6.2; Rust 1.99.1; Cargo 1.99.1; hardware metrics not applicable or measured
result: VERIFIED LOCALLY, COMMITTED, AND PUSHED; local/remote branch tips confirmed equal
review: PENDING
```

## Change

Expanded `crates/settings/tests/fixtures/cleanup_current_behavior.json` from 24 to 48 explicit synthetic fixtures. Added 24 cases, six each for numeric preservation, proper nouns/abbreviations, Unicode preservation, and protected vocabulary interactions. Updated the existing corpus runner's exact fixture-count assertion and the README/TASKS documentation. No production cleanup/settings code, fixture schema, dependencies, or Verbatim policy changed.

The existing runner continues to check unique/nonempty IDs, required fields and allowed categories, all eight categories, all four mode/filler combinations, both command-flag values, exact expected output, deterministic repeat calls, and input/vocabulary/settings immutability.

## Verification

The focused corpus command was first run with an explicitly reviewed but incorrect expected value `2026.` for `numeric-comma-grouping-and-year-preserved`. It failed because current output is `2026 .`. Source review confirmed that the punctuation rule only removes spacing when the preceding character is alphabetic and the adjacent alphanumeric token contains no number. The expected value was then set to that source-confirmed current behavior; production code was not changed. The same focused command passed after this review.

| Command | Result | Notes |
|---|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | FAIL, then PASS | Initial expectation mismatch described above; final run passed 2 tests and printed the summary below. |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS | 9 passed. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS | No formatting changes required. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS | Workspace checked. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS | 501 tests passed, including unit, integration, and doctests. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | No warnings. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS | Workspace built. |
| `git diff --check` | PASS | Passed after final documentation/evidence edits. |
| `git diff --cached --check` | PASS | Passed after staging only the five intended task files. |

Observed final focused output:

```text
Cleanup golden corpus: 48 fixtures passed
Categories covered: 8
Current behavior baseline: true
```

## Limitations and remaining work

- The 48 fixtures are synthetic current-behavior baselines. They do not satisfy AC-07's ≥200-case corpus or ≥99% exact-output threshold, and they do not prove universal protection of numeric tokens or proper nouns.
- AC-07 still requires the protected-token corpus gate and an explicit Clean/Verbatim distinction. T-035's ≥150 golden target and T-036's 50 additional-golden target remain incomplete; production integration and broader acceptance coverage remain open.
- Complete Verbatim behavior remains unresolved and **PROPOSED — NOT APPROVED** in `docs/CLEANUP_MODE_POLICY.md`.
- No real ASR, hardware, performance, application-integration, or release gates were measured by this task.
- The unrelated untracked `....` and `scripts/__pycache__/` items were preserved.

T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.
