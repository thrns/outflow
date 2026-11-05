# Cleanup interaction golden fixtures — partial T-035/T-036 evidence

```yaml
task_id: T-035/T-036 partial progress
feature_ids: [F-07]
flows: [UF-05]
acceptance: [AC-07]
owner: Codex, codex/task-036-cleanup-interaction-fixtures
base_commit: c124fc0c882ef1638310493f74b1ab70eb2e7919
result_commit: a4336e156eac1819b89f9aff52f9ddcdf9c7aa6c (fixture, runner, and documentation result; this evidence commit records the SHA)
platform: macOS 26.6.2 (25G83), arm64, Apple M3 Pro, 18 GB RAM; Rust 1.99.1; Cargo 1.99.1
implementation: Added 24 explicit current-behavior fixtures, updated the corpus count to 72, and documented partial progress. No production source, settings schema, dependency, or lockfile changed.
doc_updates: README.md cleanup corpus section; docs/TASKS.md partial progress; evidence/T-035-T-036-cleanup-interaction-fixtures.md
risks: AC-07 requires at least 200 cases, at least 99% exact output, protected-token evidence, mode distinction, and production integration; not complete here.
review: PENDING
```

## Fixture additions

Group A — dictation-command boundaries (6):

- `commands-uppercase-new-line-with-cleanup-enabled`
- `commands-mixed-case-new-paragraph-with-cleanup-enabled`
- `commands-disabled-preserve-both-literal-phrases`
- `commands-boundaries-reject-larger-words`
- `commands-quotes-and-punctuation-are-not-boundaries`
- `commands-multiple-breaks-interleaved-with-text`

Group B — filler removal and mode interactions (6):

- `fillers-removed-after-command-generated-line-breaks`
- `fillers-uh-removed-after-existing-paragraph-break`
- `modes-clean-with-filler-removal-disabled-preserves-prefix`
- `modes-verbatim-with-filler-setting-enabled-preserves-prefix`
- `fillers-repeated-um-uh-sequence-is-preserved`
- `fillers-sentence-internal-and-quoted-phrases-remain`

Group C — vocabulary matching and precedence (6):

- `vocabulary-longest-spoken-phrase-wins`
- `vocabulary-replaces-every-repeated-occurrence`
- `vocabulary-multiple-entries-correct-one-transcript`
- `unicode-vocabulary-matches-canonical-equivalent-spelling`
- `safety-vocabulary-word-boundaries-block-partial-words`
- `vocabulary-replacement-is-not-scanned-again-in-one-call`

Group D — punctuation and protected replacements (6):

- `punctuation-supported-marks-normalized-together`
- `punctuation-ambiguous-quote-sequences-stay-spaced`
- `safety-year-adjacent-period-spacing-remains-protected`
- `safety-written-replacement-internal-punctuation-spacing-protected`
- `safety-written-replacement-leading-trailing-space-stays-distinct`
- `safety-punctuation-and-fillers-around-two-protected-replacements`

## Review and observed behavior

Expected strings were written explicitly after review of FR-07.1–FR-07.4, AC-07, UF-05, C-06/TS-06, `docs/CLEANUP_MODE_POLICY.md`, the settings adapter, and the existing Rust cleanup and Unicode 18 vocabulary matching sources. They were not copied from program output. All original 48 fixture objects and expected values remain unchanged. No unexpected output mismatch was found.

The existing numeric guard retains `2026 .`: punctuation spacing is removed only under its current alphabetic-predecessor, nonnumeric-token, and nonambiguous-sequence conditions. Protected replacement bytes also remain exact. In particular, the case with written value `" North Parkade "` produces `" North Parkade  outside."`: the replacement's trailing space is protected, while whitespace from the original transcript is normalized separately and remains adjacent to it.

## Verification

Commands were run from the repository root on the environment listed above. Results:

- `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` — PASS. Observed output:

  ```text
  Cleanup golden corpus: 72 fixtures passed
  Categories covered: 8
  Current behavior baseline: true
  ```

- `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` — PASS, 9 tests.
- `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` — PASS.
- `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` — PASS.
- `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` — PASS; workspace unit, integration, and documentation test suites completed successfully.
- `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS.
- `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` — PASS.
- `git diff --check` — PASS.
- `git diff --cached --check` — PASS after staging the five intended task files.
- Original-fixture preservation assertion — PASS; the parsed first 48 objects match the starting commit and the appended set has 24 cases with 72 unique IDs.

No performance, audio-recognition, installed-size, or release metrics were measured. No production code or dependencies were changed.

## Remaining requirements and limitations

The 72 cases are a current-behavior characterization set. They do not meet AC-07's ≥200-case corpus, ≥99% exact-output requirement, complete protected numeric/proper-noun evidence, production integration, or release requirements. T-035 and T-036 remain partial and unchecked; T-017, T-018, T-023, T-038, T-039, AC-07, AC-08, and release gates remain unchecked. `docs/CLEANUP_MODE_POLICY.md` and its Verbatim recommendation remain **PROPOSED — NOT APPROVED**.
