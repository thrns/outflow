# Cleanup golden corpus expansion to 120 fixtures

**Status:** Partial T-035/T-036 characterization; not task completion or AC-07 acceptance.

| Field | Value |
|---|---|
| Task IDs | T-035, T-036 |
| Requirement | FR-07; AC-07 remains open |
| Branch | `codex/task-035-036-cleanup-120-fixtures` |
| Base commit | `6f4a31222a5b0f0fe40b529052491da1a0cac8c7` |
| Fixture/docs commit | `521b7475c8cba4745f48895c5bbb9d1d0fe128bc` |
| Final branch commit | Branch HEAD containing this evidence update; exact SHA is reported in the final handoff |
| Environment | macOS 26.5.0, Darwin 25.6.0, arm64; Rust 1.99.1; Cargo 1.99.1 |
| Hardware RAM | Not measured for this task; no 4 GB resource gate was evaluated |
| Production behavior / dependencies | Unchanged; no dependency or lockfile changes |

## Change summary

Appended 24 explicitly authored synthetic cases to `crates/settings/tests/fixtures/cleanup_current_behavior.json`, bringing the current-behavior corpus to 120. Updated the exact-count assertion and module documentation in `crates/settings/tests/cleanup_golden_corpus.rs`, documented the coverage and authoring workflow in `README.md`, and recorded partial progress in `docs/TASKS.md`. This remains a current-behavior characterization. The Clean/Verbatim recommendation in `docs/CLEANUP_MODE_POLICY.md` remains **PROPOSED — NOT APPROVED**.

The runner retains its strict JSON schema and unknown-field rejection, nonempty unique IDs, eight-category check, four mode/filler combinations, both command-flag values, exact output and deterministic repeat checks, input/vocabulary/settings immutability, protected-token byte-range and UTF-8 checks, 16 annotation count assertions, and five synthetic negative checks.

## New fixture IDs

### Group A — Whitespace and small-input boundaries

- `whitespace-ascii-space-tab-only-trims-to-empty`
- `whitespace-outer-horizontal-space-trimmed-short-sentence`
- `whitespace-lf-crlf-and-standalone-cr-canonicalized`
- `whitespace-spaced-consecutive-blank-lines-preserved`
- `whitespace-unicode-words-with-ascii-separators-collapsed`
- `whitespace-trims-around-emoji-across-paragraph-break`

Observed behavior: spaces/tabs-only content trims to empty; line-leading and line-trailing ASCII spaces/tabs are removed; internal ASCII horizontal runs collapse to one space; LF, CRLF, and standalone CR each become LF while their count and positions remain; spaces/tabs on otherwise blank lines disappear but the blank-line breaks remain; Unicode text is copied, non-ASCII whitespace remains unchanged, and the two line breaks around the emoji paragraph boundary remain two LFs.

### Group B — Clean/Verbatim and filler boundaries

- `modes-clean-removes-mixed-case-line-leading-fillers`
- `modes-clean-disabled-retains-mixed-case-fillers`
- `modes-verbatim-keeps-supported-fillers-with-removal-enabled`
- `modes-verbatim-still-corrects-vocabulary-and-punctuation`
- `fillers-repeated-sequence-after-command-generated-newline-is-preserved`
- `safety-protected-filler-like-replacement-and-next-line-filler-removal`

Observed behavior: the existing line-leading `um, ` and `uh, ` checks are ASCII case-insensitive; Clean removes them only when enabled. Verbatim keeps them even when `remove_fillers` is true, while vocabulary and punctuation still run. A repeated supported filler sequence remains intact after a command-generated line break. In the final fixture, the written replacement `um, keep this` stays protected while the ordinary `uh, ` prefix on the next, source-generated command line is removed in the same pipeline call. No broader filler recognition is added.

### Group C — Unicode vocabulary matching and word boundaries

- `unicode-vocabulary-matches-canonical-equivalent-spoken-phrase`
- `unicode-vocabulary-matches-multiword-unicode-phrase`
- `unicode-combining-mark-blocks-partial-vocabulary-match`
- `unicode-emoji-separates-vocabulary-match-from-neighboring-words`
- `unicode-vocabulary-corrects-repeated-canonical-phrase-occurrences`
- `unicode-vocabulary-longest-overlapping-accented-phrase-wins`

Observed behavior: canonical-equivalent accented spoken forms match under the existing Unicode 18 policy; a complete multiword Unicode phrase matches; a combining mark is a word continuation and blocks a partial match; emoji delimit rather than join the neighboring vocabulary phrase; repeated matches are replaced; and the longer overlapping accented phrase wins even though its shorter entry appears first. Configured written values are not normalized after insertion.

### Group D — Punctuation and preservation interactions

- `punctuation-ordinary-spacing-corrected-next-to-protected-decimal`
- `safety-spaced-punctuation-after-digit-bearing-identifier-is-preserved`
- `punctuation-supported-spacing-corrected-next-to-accented-names`
- `safety-vocabulary-written-leading-trailing-ascii-spaces-preserved`
- `safety-protected-replacements-across-paragraph-with-external-punctuation`
- `commands-multiline-vocabulary-punctuation-and-filler-stages-compose`

Observed behavior: eligible punctuation around ordinary words is tightened while the decimal `3 . 14` remains spaced; punctuation spacing after `ZX9` remains because the adjacent alphanumeric token includes a number; supported punctuation beside accented names is corrected; intentional leading/trailing replacement spaces remain byte-for-byte and protect the adjacent spacing; two vocabulary replacements remain exact across a paragraph break while eligible punctuation outside their protected spans is corrected; and the final case composes command recognition, vocabulary, punctuation, and Clean filler removal in the documented stage order.

## Independent preservation comparison

Used Python's built-in JSON parser and `subprocess.run(["git", "show", "<base>:<path>"])` for a one-off development-time comparison. No script was added to the repository. The comparison parsed both arrays, asserted that the current first 96 objects equal all 96 base objects, separately compared each original `protected_tokens` value, checked that 24 objects were appended and all 120 IDs are unique, and rechecked annotations, categories, mode/filler combinations, and command flags.

Actual comparison output:

```text
PASS: 96 base objects match parsed current objects; 24 appended; 120 unique IDs.
PASS: all original protected_tokens metadata matches exactly: 16 tokens, 8 numbers, 8 proper nouns, 12 fixtures.
PASS: all 8 categories, all 4 mode/filler combinations, and both command flags remain covered.
```

Because the comparison checked complete parsed objects, it also confirms that the first 96 fixture fields, expected strings, and optional metadata are unchanged. The protected annotations remain in the same 12 fixtures with their original byte ranges and expected token values.

## Verification

Platform: macOS 26.5.0, Darwin 25.6.0, arm64. The host was a development machine; physical 4 GB hardware and release resource limits were not evaluated.

Focused corpus command:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture
```

Actual focused output:

```text
Finished `test` profile [unoptimized + debuginfo] target(s) in 2.27s
Running tests/cleanup_golden_corpus.rs

running 1 test
Cleanup golden corpus: 120 fixtures passed
Categories covered: 8
Protected numeric tokens: 8 (0 altered)
Protected proper nouns: 8 (0 altered)
Current behavior baseline: true
test current_behavior_fixtures_match_exactly_and_are_deterministic ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Other requested checks:

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS — 9 passed; 0 failed |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS — exit 0 |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS — exit 0 |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — exit 0; all unit, integration, and doc-test targets completed successfully |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS — exit 0 |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS — exit 0 |
| `git diff --check` | PASS — exit 0 |
| `git diff --cached --check` | PASS — exit 0 after staging only the five task files |
| `git push -u origin codex/task-035-036-cleanup-120-fixtures` | PASS — branch created on origin at `521b7475c8cba4745f48895c5bbb9d1d0fe128bc` |
| `git ls-remote origin refs/heads/codex/task-035-036-cleanup-120-fixtures` | PASS — remote tip matched local `HEAD` and upstream at `521b7475c8cba4745f48895c5bbb9d1d0fe128bc` at verification time |

Expected strings were written from the reviewed Rust stage rules before the focused run; execution was used to verify them, not to generate them.

## Findings and limitations

- No unexpected behavior required a production change or policy decision.
- Punctuation spacing after a year remains `2026 .` under the current numeric guard. The new decimal and digit-bearing-identifier cases characterize the same conservative protection boundary.
- A `new line` command creates one LF and `new paragraph` creates two; existing normalized source line breaks remain, so command and source breaks can add together.
- Written vocabulary replacements preserve configured whitespace and punctuation byte-for-byte. Outside source spacing can therefore remain adjacent to intentional replacement spaces.
- Filler recognition remains limited to line-leading ASCII `um, ` / `uh, ` with an alphabetic next character; repeated filler sequences stay intact. Protected replacement text is excluded from removal.
- Canonical matching keeps the existing Unicode 18 behavior and conservative continuation boundaries. No normalization of a written replacement was introduced.
- The 120 cases are below AC-07's ≥200 corpus requirement. They do not establish the ≥99% criterion, universal numeric/proper-noun preservation, production speech integration, complete Verbatim policy, certified platform coverage, or release gates. T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.
- The unrelated untracked paths `....` and `scripts/__pycache__/` were left untouched and excluded from staging.

## Files and handoff

Changed task files: `README.md`, `crates/settings/tests/cleanup_golden_corpus.rs`, `crates/settings/tests/fixtures/cleanup_current_behavior.json`, `docs/TASKS.md`, and this evidence file. No production source or lockfile changed.

Review state: pending maintainer review. The fixture/docs commit was pushed, and `git ls-remote` confirmed the origin branch matched the local branch at `521b7475c8cba4745f48895c5bbb9d1d0fe128bc`. This evidence update is a follow-up commit; its final branch and remote-tip SHA are reported in the final handoff. The unrelated untracked paths remain unstaged. No release, acceptance, or policy status is changed.
