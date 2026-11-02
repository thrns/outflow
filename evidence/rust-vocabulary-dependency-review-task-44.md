---
task_id: "Task 44 (user-scoped; repository T-044 remains the Windows adapter)"
feature_ids: [F-08]
flows: [UF-06]
acceptance: [AC-08]
owner: primary agent
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: UNCOMMITTED
platform: macOS 26.6.2 arm64; hardware model and RAM not measured
implementation: "Read-only Cargo dependency audit, synthetic unittest coverage, candidate source review, README/evaluation/evidence updates"
commands:
  - {command: 'python3 scripts/audit_vocabulary_unicode_rust_dependencies.py --repo .', result: PASS, notes: '3 workspace manifests inspected; none of the three candidates is direct or locked; no crate approval; comparator remains LOWERCASE-ONLY.'}
  - {command: 'python3 scripts/check_vocabulary_unicode_policy.py --demo', result: PASS, notes: '12 reference cases; Python Unicode data version 15.1.0, not a Unicode 18 conformance claim.'}
  - {command: 'python3 -m unittest discover -s scripts/tests -v', result: PASS, notes: '258 total Python tests; 17 new audit tests plus 241 existing tests.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check', result: PASS, notes: 'No formatting changes requested.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked', result: PASS, notes: 'Workspace check passed without dependency changes.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked', result: PASS, notes: '111 tests passed (41 core, 70 settings).'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings', result: PASS, notes: 'No warnings.'}
  - {command: 'RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked', result: PASS, notes: 'Workspace build passed; blocked Moonshine CMake configuration was not attempted.'}
  - {command: 'npm run typecheck', result: PASS, notes: 'Existing package script.'}
  - {command: 'npm run test -- --run', result: PASS, notes: '136 tests passed in 10 files.'}
  - {command: 'npm run lint', result: PASS, notes: 'Existing package script.'}
  - {command: 'npm run build', result: PASS, notes: 'Typecheck and Vite production build passed.'}
  - {command: 'git diff --check', result: PASS, notes: 'Final whitespace check; repository status recorded below.'}
metrics: "Unicode crate binary footprint, RSS, and Cargo resolution NOT_MEASURED; no candidate dependencies were added."
doc_updates: "README.md Task 44 section; docs/VOCABULARY_UNICODE_RUST_OPTIONS.md; this evidence file. No task ledger/policy-status change."
risks: "caseless 0.2.2 fold data is Unicode 16.0.0, while the proposed policy cites Unicode 18.0.0; recommendation is conditional on a maintainer data-version decision."
review: PENDING
---

# Task 44 — Rust vocabulary dependency review

## Scope and repository state

This is the user-scoped Task 44 dependency audit. It is **not** repository T-044, which remains the Windows insertion adapter. The working branch was `codex/task23-moonshine-build-scope`; base commit was `6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b`. Existing Task 43 work was present before this task and was preserved. This task did not edit the production comparator, Cargo manifests, `Cargo.lock`, the proposed policy status, task checkboxes, or public Rust APIs.

The comparator source still contains `spoken.to_lowercase()`. The audit script reads Cargo TOML and policy/source text only, uses Python's standard library and `tomllib`, does not execute repository content or invoke Cargo, and makes no network requests.

## Dependency audit results

`python3 scripts/audit_vocabulary_unicode_rust_dependencies.py --repo .` reported:

| Candidate | Version investigated | Direct in `outflow-core` | In `Cargo.lock` | Locked version | Reachability | Explicit crate approval |
|---|---:|---|---|---|---|---|
| `caseless` | 0.2.2 | No | No | None | Not present | No |
| `unicode-normalization` | 0.1.24 | No | No | None | Not present | No |
| `unicode-casefold` | 0.2.0 | No | No | None | Not present | No |

The lockfile contains `unicode-ident` 1.0.27 transitively, which is identifier-classification support and does not provide D145. No candidate is inferred to be directly usable merely from lockfile presence. Policy document status remains **PROPOSED — NOT APPROVED OR INTEGRATED**; D145 conformance is not established by the audit.

## Source review

Primary crate source, documentation, manifests, and official Unicode references were accessible during review:

- [`caseless` 0.2.2 API and exact source](https://docs.rs/caseless/0.2.2/src/caseless/lib.rs.html) implements NFD → default full case fold → NFD. Its exact [fold table](https://docs.rs/caseless/0.2.2/src/caseless/case_folding_data.rs.html) declares Unicode 16.0.0. The [manifest](https://docs.rs/crate/caseless/0.2.2/source/Cargo.toml) names `unicode-rs/rust-caseless`, MIT licensing, and `unicode-normalization = "0.1"`. The crate has no declared MSRV. Its APIs appear stable-Rust based on source inspection; compilation against Outflow was not attempted. Retain the required [Unicode data license notice](https://www.unicode.org/license.txt) when redistributing bundled data.
- [`unicode-normalization` 0.1.24 manifest](https://docs.rs/crate/unicode-normalization/0.1.24/source/Cargo.toml) reports MSRV 1.36, edition 2018, `MIT/Apache-2.0`, and `tinyvec` with `alloc`. Upstream's [0.1.24 update commit](https://github.com/unicode-rs/unicode-normalization/commit/fdfe8e0) confirms Unicode 16.0.0 tables. `alloc` selects `tinyvec_macros`; the exact resolved versions remain unknown because no Cargo resolution was performed.
- [`unicode-casefold` 0.2.0 source](https://docs.rs/unicode-casefold/0.2.0/src/unicode_casefold/lib.rs.html) exposes full versus simple folding and `Locale::NonTurkic`; its [table](https://docs.rs/unicode-casefold/0.2.0/src/unicode_casefold/tables.rs.html) declares Unicode 9.0.0. Its [manifest](https://docs.rs/crate/unicode-casefold/0.2.0/source/Cargo.toml) identifies `lfairy/unicode-casefold`, dual MIT/Apache-2.0 licensing, and no runtime dependencies; the repository is archived. It declares no MSRV and was not compiled here. Its Unicode-derived data notices should be checked in the exact package before redistribution.
- Current [stable Rust `str` documentation](https://doc.rust-lang.org/stable/std/primitive.str.html) provides lowercase and ASCII case-insensitive operations, not stable full default case-fold plus NFD. Rust 1.99.1 reports `std::char::UNICODE_VERSION = (17, 0, 0)`; Rust's documented case-fold APIs are nightly-only experimental and lack normalization. `unicode-ident` is not a matching implementation.
- [Unicode 18.0 Chapter 3 / D145](https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-3/) and [UAX #15 v58](https://www.unicode.org/reports/tr15/tr15-58.html) anchor the proposed Unicode 18 policy. Unicode 16.0.0 [CaseFolding.txt](https://www.unicode.org/Public/16.0.0/ucd/CaseFolding.txt) and the [Unicode license](https://www.unicode.org/license.txt) were used to review default full and Turkic folding data distinctions and notice requirements.

The 2026-10-16 `unicode-normalization` 0.1.26 generated table reports Unicode 18.0.0, but `unicode-casefold` 0.2.0 remains on Unicode 9.0.0. Combining those packages does not establish a synchronized D145 data set. A normalization 0.1.26 pin also cannot change the Unicode 16.0.0 fold table bundled in `caseless` 0.2.2.

Docs.rs source-package sizes observed during review were 159.1 kB for `caseless` 0.2.2, 725.1 kB for `unicode-normalization` 0.1.24, and 56.8 kB for `unicode-casefold` 0.2.0. These are not compiled binary or installed application measurements. Candidate dependency resolution, clean-cache offline build, cross-platform behavior, binary delta, and RSS are **UNVERIFIED / NOT MEASURED**.

## Recommendation and approval boundary

**RECOMMENDED — AWAITING APPROVAL:** Candidate A is the best verified implementation shape, conditionally using `caseless = "=0.2.2"` plus direct `unicode-normalization = "=0.1.24"`, only if the maintainer explicitly approves Unicode 16.0.0 as the production data version. This keeps the two data sources aligned and follows D145's operation order, but differs from the proposal's Unicode 18.0.0 reference.

Before integration, the maintainer must choose one:

1. Approve changing the proposed production target to Unicode 16.0.0 and approve the exact two crate versions; or
2. Keep Unicode 18.0.0 and authorize additional research until a stable implementation with verified Unicode 18.0.0 full folding and normalization data is identified.

No dependency was approved or integrated here. T-017, T-018, T-038, T-039, AC-08 and the proposal remain incomplete/unapproved. The full candidate table, limitations, migration requirements, and future test plan are in [`docs/VOCABULARY_UNICODE_RUST_OPTIONS.md`](../docs/VOCABULARY_UNICODE_RUST_OPTIONS.md).

## Required follow-up boundary

Only after approval, a separate implementation should add the exact approved versions; implement canonical-caseless comparison; preserve validation on original text, error precedence, typed errors and values; test all 12 Task 43 cases plus full-fold/normalization conformance data; add regressions for previously undetected duplicates; analyze and resolve existing-entry collisions before persistent CRUD; and measure compatibility and binary footprint. Unicode data updates must be explicitly versioned, reviewed, tested, migration-checked, and reflected in project documentation.

## Verification results

Executed on macOS 26.6.2, arm64, with Rust stable 1.99.1 and Python 3.13.13. Frontend checks used the repository's installed npm dependencies.

- Audit CLI: **PASS**; all three candidate packages are absent from both core direct dependencies and the lockfile; no approval found.
- Python policy demo: **PASS**; 12 comparison outcomes printed; Python's Unicode data version was 15.1.0, not Unicode 18.0.0.
- Python full suite: **PASS**, 258 tests total. The pre-existing baseline of 241 tests remains passing; this task adds 17 audit tests.
- Rust format, check, Clippy (`-D warnings`), and build: **PASS**. Workspace Rust tests: **111 passed**, 0 failed (41 core and 70 settings).
- Frontend typecheck, lint, and build: **PASS**. Frontend tests: **136 passed** across 10 files.
- No Moonshine CMake configuration was attempted.

## Git review

Final `git status --short --branch`:

```text
## codex/task23-moonshine-build-scope
 M README.md
 M crates/core/src/vocabulary.rs
?? docs/VOCABULARY_UNICODE_POLICY.md
?? docs/VOCABULARY_UNICODE_RUST_OPTIONS.md
?? evidence/rust-vocabulary-dependency-review-task-44.md
?? evidence/rust-vocabulary-unicode-policy-task-43.md
?? scripts/audit_vocabulary_unicode_rust_dependencies.py
?? scripts/check_vocabulary_unicode_policy.py
?? scripts/tests/test_audit_vocabulary_unicode_rust_dependencies.py
?? scripts/tests/test_check_vocabulary_unicode_policy.py
```

`git diff --check`: **PASS**. The `README.md` modification includes both prior Task 43 content and this task's appended section. The vocabulary source, policy note, Task 43 evidence, and Python reference files were pre-existing uncommitted Task 43 work and were preserved. No staging, commit, push, merge, or history rewrite was performed.
