# T-035/T-036 Cleanup Golden Corpus — 144 Fixtures

**Status:** Partial current-behavior characterization; T-035, T-036, and AC-07 remain incomplete.
**Date:** 2026-11-05
**Task branch:** `codex/task-035-036-cleanup-144-fixtures`
**Base branch:** `codex/task-035-036-cleanup-120-fixtures`
**Base commit:** `815d00ebbfc4bcda50eeaddde4e2ab5ee202a9ee`
**Implementation commit:** `bf57f25` (`test(cleanup): expand golden corpus to 144 fixtures`).

## Scope and result

Appended 24 explicit synthetic cases to `crates/settings/tests/fixtures/cleanup_current_behavior.json`, increasing the exact current-behavior corpus count from 120 to 144. Updated the fixture runner's module documentation and exact count assertion, the cleanup corpus instructions in `README.md`, and the partial progress record in `docs/TASKS.md`. No production cleanup/settings source, settings schema/defaults/migrations, dependency, lockfile, or policy decision changed. `docs/CLEANUP_MODE_POLICY.md` remains **PROPOSED — NOT APPROVED**.

The runner still checks strict required/unknown fields, nonempty unique IDs, all eight categories, all four Clean/Verbatim × filler-setting combinations, both command settings, exact output, repeated-call determinism, input/vocabulary/settings immutability, protected-token UTF-8 offsets and exact bytes, and all five synthetic rejection cases.

## Added fixture IDs

### Group A — Whitespace preservation and boundaries

- `whitespace-empty-lines-at-both-transcript-edges`
- `whitespace-multiple-horizontal-runs-around-punctuation`
- `whitespace-ascii-around-lf-and-crlf-boundaries`
- `whitespace-nonascii-whitespace-only-line-is-preserved`
- `whitespace-ascii-run-next-to-combining-marks`
- `whitespace-unicode-emoji-and-three-consecutive-blank-lines`

### Group B — Filler false-positive prevention

- `fillers-line-leading-supported-prefix-before-alphabetic-word`
- `fillers-filler-like-token-without-comma-remains`
- `fillers-punctuated-continuation-is-not-a-filler-match`
- `fillers-repeated-prefixes-with-mixed-ascii-capitalization-remain`
- `fillers-quoted-line-leading-phrase-remains`
- `fillers-protected-replacement-and-independent-line-prefix`

### Group C — Vocabulary boundaries and precedence

- `vocabulary-phrase-beside-ascii-punctuation`
- `vocabulary-unicode-punctuation-canonical-caseless-match`
- `vocabulary-embedded-alphabetic-substring-does-not-match`
- `vocabulary-combining-mark-continuation-blocks-partial-match`
- `vocabulary-longest-overlapping-spoken-phrase-wins`
- `vocabulary-written-phrase-is-not-cascaded-in-same-call`

### Group D — Multi-stage safety interactions

- `safety-numeric-period-stays-spaced-while-eligible-spacing-corrects`
- `safety-accented-proper-noun-beside-vocabulary-correction`
- `safety-enabled-new-line-then-removable-filler`
- `safety-disabled-command-remains-available-to-vocabulary`
- `safety-protected-replacements-across-punctuation-and-paragraph`
- `safety-whitespace-command-vocabulary-punctuation-filler-composition`

## Independent preservation comparison

Used a one-off local Python check, not committed as a script. Python's `subprocess.check_output` retrieved the base fixture JSON with `git show 815d00ebbfc4bcda50eeaddde4e2ab5ee202a9ee:crates/settings/tests/fixtures/cleanup_current_behavior.json`; `json.loads` parsed that data and the working fixture file. The script asserted parsed-object equality for the first 120 fixtures, independently compared each annotated fixture ID and its complete `protected_tokens` list, checked the appended count and lack of new annotations, and checked all requested coverage sets.

Observed comparison output:

```text
Base commit: 815d00ebbfc4bcda50eeaddde4e2ab5ee202a9ee
Base fixture count: 120; current fixture count: 144; appended: 24
First 120 parsed objects match base exactly: PASS
Protected annotations identical: PASS (16 tokens across 12 fixtures; 8 number, 8 proper_noun)
New protected annotations: 0
All 144 IDs unique: PASS
New input strings differ from all first-120 inputs: PASS
All eight categories represented: PASS
Clean/Verbatim × filler-setting combinations: 4/4 covered
commands_enabled true/false: both covered
```

## Verification

Environment: macOS 26.6.2, arm64; `rustc 1.99.1 (58cf8f9c3 2026-10-13)`; `cargo 1.99.1 (5f94df478 2026-08-27)`; stable toolchain.

The focused corpus command passed. Its actual visible summary was:

```text
Cleanup golden corpus: 144 fixtures passed
Categories covered: 8
Protected numeric tokens: 8 (0 altered)
Protected proper nouns: 8 (0 altered)
Current behavior baseline: true
```

The test runner also reported `1 passed; 0 failed`. The Clean/Verbatim contract test reported `9 passed; 0 failed`. The workspace test command exited 0 and reported 500 passing tests across its suites, with no failures. The requested verification commands and results were:

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS; exact summary above |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS; 9 passed, 0 failed |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 500 passed, 0 failed |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS |
| `git diff --cached --check` | PASS; run after staging only the intended task files |

## Source-reviewed findings

The first focused run found one mistaken expected value in the long Group D composition. Its input has an enabled `new paragraph` command immediately before a source CRLF. The command stage inserts two line breaks; whitespace normalization has already represented the CRLF as one line break, and that source break remains after the command stage. Source review and the existing command contract therefore support three consecutive line breaks. The fixture expectation was corrected from two to three, production code was left unchanged, and the focused corpus passed on rerun.

The new cases also record that the numeric token's `2026 .` spacing remains under the existing conservative numeric guard; non-ASCII whitespace on an otherwise whitespace-only line is copied unchanged; combining marks block partial vocabulary boundaries; repeated fillers remain intact; and configured replacement bytes remain protected while independent punctuation/filler transformations proceed outside those spans.

## Remaining limitations

- This is a current-behavior characterization set, not the AC-07 acceptance corpus. It has 144 fixtures, 56 fewer than the ≥200 required, and does not demonstrate the ≥99% acceptance threshold.
- The 16 annotated protected-token examples are selected cases, not a universal guarantee for arbitrary numbers or proper nouns.
- T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and every release gate remain unchecked.
- The final Verbatim semantics remain unresolved; `docs/CLEANUP_MODE_POLICY.md` is still **PROPOSED — NOT APPROVED**.
- No hardware performance, ASR quality, packaging, UI, or cross-platform release claim is made by this fixture-only work.

**Review:** Pending maintainer review. This evidence records only the scoped corpus expansion and local checks above.
