# Cleanup Golden Corpus Expansion: 168 to 192

```yaml
task_ids: [T-035, T-036]
feature_ids: [F-07]
flows: [] # Corpus-only work; no interactive user flow was exercised.
acceptance: [AC-07]
owner: /root (single-agent coordinator)
branch: codex/task-035-036-cleanup-192-fixtures
base_branch: codex/task-035-036-cleanup-168-fixtures
base_commit: 71be6c46888e6e1ec68373104567e652ba87222c
result_commit: PENDING_IMPLEMENTATION_COMMIT
platform: macOS 26.6.2, arm64; model/RAM not measured for this corpus-only task
toolchain: rustc 1.99.1 (58cf8f9c3 2026-10-13), cargo 1.99.1 (5f94df478 2026-08-27), stable override
review: PENDING
```

## Result

Appended 24 explicitly authored synthetic current-behavior fixtures, bringing the corpus from 168 to 192. Updated the runner's count and module documentation, the README corpus guide, and the partial T-035/T-036 progress record. No production cleanup or settings code, schema/defaults, dependency, lockfile, or cleanup policy changed. The Clean/Verbatim recommendation remains **PROPOSED — NOT APPROVED**.

## Added fixture IDs

### Group A — Numeric and proper-noun safety

- `numeric-signed-value-with-eligible-ordinary-punctuation`
- `numeric-scientific-notation-like-text-stays-literal`
- `numeric-date-version-and-ambiguous-sequence`
- `proper-noun-accented-multiword-place-boundary`
- `proper-noun-acronym-and-person-name-share-sentence`
- `safety-numeric-names-around-protected-vocabulary-replacement`

### Group B — Unicode whitespace and text boundaries

- `whitespace-nonbreaking-space-between-multilingual-words`
- `whitespace-thin-and-narrow-spaces-next-to-ascii-runs`
- `whitespace-ideographic-space-in-multilingual-sentence`
- `whitespace-tabs-adjacent-to-decomposed-combining-sequence`
- `whitespace-emoji-japanese-mixed-crlf-lf-with-command-disabled`
- `whitespace-nonascii-line-space-before-punctuation`

### Group C — Dictation commands and filler edges

- `commands-supported-new-line-at-transcript-start`
- `commands-supported-new-paragraph-at-transcript-end`
- `commands-consecutive-supported-commands-compose-breaks`
- `commands-punctuation-interruption-keeps-command-like-text`
- `commands-generated-line-keeps-repeated-filler-sequence`
- `commands-paragraph-removes-ordinary-filler-and-protects-vocabulary-filler`

### Group D — Protected vocabulary and stage-order safety

- `vocabulary-multiple-replacements-separated-by-accent-and-emoji`
- `vocabulary-longer-decomposed-accented-phrase-wins`
- `safety-written-punctuation-spacing-next-to-ordinary-comma`
- `safety-embedded-line-break-replacement-keeps-filler-like-lines`
- `safety-short-and-long-replacements-survive-prior-punctuation-edits`
- `commands-multiline-vocabulary-protected-punctuation-and-clean-fillers`

The additions are unannotated; the existing token annotations remain the separate protected-token baseline.

## Independent base-fixture preservation comparison

The required comparison was run inline with Python and `git show`; no permanent comparison script was added. Python's JSON parser compared parsed objects, not formatted file text:

```python
import json
import subprocess
from pathlib import Path

base_commit = "71be6c46888e6e1ec68373104567e652ba87222c"
fixture_path = "crates/settings/tests/fixtures/cleanup_current_behavior.json"
base = json.loads(subprocess.check_output(["git", "show", f"{base_commit}:{fixture_path}"]))
current = json.loads(Path(fixture_path).read_text(encoding="utf-8"))
assert len(base) == 168
assert len(current) == 192
assert current[:168] == base
base_annotations = [f.get("protected_tokens") for f in base]
assert [f.get("protected_tokens") for f in current[:168]] == base_annotations
assert sum(len(f.get("protected_tokens", [])) for f in base) == 16
assert sum(bool(f.get("protected_tokens")) for f in base) == 12
assert sum(t["kind"] == "number" for f in base for t in f.get("protected_tokens", [])) == 8
assert sum(t["kind"] == "proper_noun" for f in base for t in f.get("protected_tokens", [])) == 8
new = current[168:]
assert len(new) == 24 and all("protected_tokens" not in f for f in new)
assert len({f["id"] for f in current}) == 192
assert len({f["input"] for f in new}) == 24
assert not ({f["input"] for f in new} & {f["input"] for f in base})
assert {f["category"] for f in current} == {
    "whitespace", "commands", "vocabulary", "punctuation",
    "fillers", "modes", "unicode", "safety",
}
assert {(f["mode"], f["remove_fillers"]) for f in current} == {
    ("clean", True), ("clean", False), ("verbatim", True), ("verbatim", False),
}
assert {f["commands_enabled"] for f in current} == {True, False}
```

Actual result: **PASS** for every assertion. The first 168 parsed objects match the base commit exactly and remain in order. Their 16 protected annotations across 12 fixtures are unchanged: eight numeric tokens and eight proper nouns. Exactly 24 unannotated fixtures were appended; all 192 IDs are unique; all 24 new inputs are unique and do not duplicate a base input; all eight categories, four mode/filler combinations, and both command values remain represented.

## Expected-output review and findings

Expected strings were written explicitly from the reviewed Rust implementation before running the focused test. Relevant rules in `crates/core/src/text_cleanup.rs` and `crates/core/src/vocabulary_post_correction.rs` are: ASCII whitespace and line-ending normalization; opt-in command recognition with ASCII boundaries; Unicode 18 canonical-caseless vocabulary matching with longest-match and single-call non-cascading behavior; exact replacement copying with protected byte spans; conservative punctuation rules; and optional protected-aware filler deletion. Non-ASCII whitespace is copied; no broader Unicode normalization is performed.

The first focused run exposed two expectation mistakes, not production defects:

1. `numeric-scientific-notation-like-text-stays-literal` initially expected the space before `!` in `6.02e23 !` to be removed. Source review of `alphanumeric_token_number_flags` and `should_remove_punctuation_spacing` showed that the adjacent alphanumeric token contains digits, so this candidate is deliberately retained. The explicit expected value was corrected to preserve `6.02e23 !`.
2. `commands-consecutive-supported-commands-compose-breaks` initially expected three breaks before `done`. Source review of `apply_dictation_commands` showed the consecutive commands emit one (`new line`), two (`new paragraph`), and one (`new line`) line breaks. The explicit expected value was corrected to four additive breaks: `\n\n\n\ndone`.

The final focused run passed. No expectations were generated by running the implementation, and no production behavior was changed to satisfy a fixture.

## Verification

Focused test output (actual):

```text
Cleanup golden corpus: 192 fixtures passed
Categories covered: 8
Protected numeric tokens: 8 (0 altered)
Protected proper nouns: 8 (0 altered)
Current behavior baseline: true
```

Commands run from the repository root on macOS 26.6.2 arm64 with Rust/Cargo 1.99.1:

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS; 1 test, all 192 fixtures; actual summary above |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS; 9 passed, 0 failed |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; exit code 0, workspace tests and doctests passed |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS; recorded after final unstaged diff review |
| `git diff --cached --check` | PASS; run after staging only the five intended task files |

## Changed files

- `crates/settings/tests/fixtures/cleanup_current_behavior.json`
- `crates/settings/tests/cleanup_golden_corpus.rs`
- `README.md`
- `docs/TASKS.md`
- `evidence/T-035-T-036-cleanup-192-fixtures.md`

## Remaining acceptance and production limitations

This is independently verifiable partial T-035/T-036 corpus work only. AC-07 requires at least 200 acceptance fixtures, at least eight more than this 192-case characterization, ≥99% exact expected output, zero altered numeric/proper-noun tokens across its protected acceptance corpus, a demonstrated final Clean/Verbatim distinction, and production integration. These selected protected-token annotations do not prove universal preservation. T-035 prerequisites and complete production/idempotence acceptance remain open; T-036 policy and acceptance work remains open. `docs/CLEANUP_MODE_POLICY.md` is still **PROPOSED — NOT APPROVED**. No 4 GB hardware, performance, package-size, OS integration, ASR quality, privacy, accessibility, signature, or release gate was evaluated. **T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and every release gate remain unchecked.**
