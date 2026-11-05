# Cleanup Golden Corpus Expansion: 144 to 168

```yaml
task_ids: [T-035, T-036]
feature_ids: [F-07]
flows: [] # Corpus-only work; no interactive user flow was exercised.
acceptance: [AC-07]
owner: /root (single-agent coordinator)
branch: codex/task-035-036-cleanup-168-fixtures
base_branch: codex/task-035-036-cleanup-144-fixtures
base_commit: f0bf356147f3fe0d63990f1365403f54130a9caa
result_commit: pending; the implementation commit hash will be recorded in a follow-up evidence commit
platform: macOS 26.6.2 (25G83), arm64
toolchain: Rust 1.99.1, cargo 1.99.1, stable override from rust-toolchain.toml
review: PENDING
```

## Result

Appended 24 explicitly authored current-behavior fixtures to the existing Rust cleanup corpus and changed the runner's required count and module documentation to 168. Updated README instructions/coverage/troubleshooting/acceptance gaps and the partial T-035/T-036 progress record. No production cleanup or settings code, schema, defaults, dependencies, lockfiles, or policy changed. The first 144 fixture objects remain unchanged, including all original protected-token annotations; no new annotations were added.

## Added fixture IDs

### Group A — Ambiguous punctuation and quotation boundaries

- `punctuation-supported-mark-next-to-curly-quotation`
- `punctuation-spaced-ellipsis-with-eligible-ending`
- `punctuation-question-exclamation-sequence`
- `punctuation-between-quoted-phrases`
- `punctuation-abbreviation-with-nearby-spacing`
- `punctuation-multiline-ambiguous-and-eligible-sequences`

### Group B — Unicode matching and boundary safety

- `unicode-precomposed-and-decomposed-accent-match-in-one-input`
- `unicode-capital-sharp-s-casefold-matches-street`
- `unicode-combining-mark-blocks-shorter-vocabulary-entry`
- `unicode-cafe-adjacent-to-japanese-punctuation`
- `unicode-emoji-separates-two-vocabulary-matches`
- `unicode-multiword-written-replacement-keeps-exact-spacing`

### Group C — Repeated vocabulary and precedence interactions

- `vocabulary-three-repeated-phrase-capitalizations`
- `vocabulary-longer-competing-phrase-wins-at-same-start`
- `vocabulary-multiple-mappings-with-eligible-punctuation`
- `vocabulary-written-replacement-does-not-cascade`
- `vocabulary-match-adjacent-to-punctuation-and-numbers`
- `safety-two-replacements-allow-spacing-outside-spans`

### Group D — Cross-stage cleanup and preservation

- `commands-clean-removes-fillers-and-corrects-vocabulary`
- `fillers-clean-preserves-fillers-when-removal-disabled`
- `modes-verbatim-retains-fillers-but-runs-enabled-command`
- `safety-numeric-token-neighbor-protected-replacement-and-spacing`
- `commands-accented-name-before-generated-paragraph-break`
- `safety-multiline-fillers-and-two-protected-replacements`

## Independent preservation comparison

The following one-off comparison used the requested base commit and Python's JSON parser. It was executed inline; no comparison script was added to the repository.

```sh
python3 - <<'PY'
import json
import subprocess
from pathlib import Path

base_commit = "f0bf356147f3fe0d63990f1365403f54130a9caa"
fixture_path = "crates/settings/tests/fixtures/cleanup_current_behavior.json"
base_bytes = subprocess.check_output(["git", "show", f"{base_commit}:{fixture_path}"])
base = json.loads(base_bytes)
current = json.loads(Path(fixture_path).read_text(encoding="utf-8"))
assert len(base) == 144
assert len(current) == 168
assert current[:144] == base
base_metadata = [fixture.get("protected_tokens") for fixture in base]
current_metadata = [fixture.get("protected_tokens") for fixture in current[:144]]
assert current_metadata == base_metadata
assert sum(len(fixture.get("protected_tokens", [])) for fixture in base) == 16
assert sum(bool(fixture.get("protected_tokens")) for fixture in base) == 12
assert sum(token["kind"] == "number" for fixture in base for token in fixture.get("protected_tokens", [])) == 8
assert sum(token["kind"] == "proper_noun" for fixture in base for token in fixture.get("protected_tokens", [])) == 8
assert all("protected_tokens" not in fixture for fixture in current[144:])
new = current[144:]
assert len(new) == 24
ids = [fixture["id"] for fixture in current]
assert len(ids) == len(set(ids)) == 168
assert {fixture["category"] for fixture in current} == {"whitespace", "commands", "vocabulary", "punctuation", "fillers", "modes", "unicode", "safety"}
assert {(fixture["mode"], fixture["remove_fillers"]) for fixture in current} == {("clean", True), ("clean", False), ("verbatim", True), ("verbatim", False)}
assert {fixture["commands_enabled"] for fixture in current} == {True, False}
base_inputs = {fixture["input"] for fixture in base}
new_inputs = [fixture["input"] for fixture in new]
assert len(new_inputs) == len(set(new_inputs)) == 24
assert not (set(new_inputs) & base_inputs)
print("PASS: parsed first 144 fixture objects exactly match git show base commit")
print("PASS: all 16 original protected-token annotations across 12 fixtures are unchanged (8 numbers, 8 proper nouns)")
print("PASS: exactly 24 unannotated fixtures appended; total 168")
print("PASS: 168 unique IDs; all eight categories; all four mode/filler combinations; both command settings")
print("PASS: all 24 new inputs are unique and none duplicates a base input")
PY
```

Result: **PASS** for every assertion. The first 144 parsed objects compare equal in order; all 16 original annotations across 12 fixtures match, consisting of eight numbers and eight proper nouns. Exactly 24 unannotated fixtures were appended. All 168 IDs are unique, all eight categories remain present, the four mode/filler pairs and both command settings are covered, and the 24 new inputs are unique and disjoint from the original inputs.

## Expected-output review

Expected values were written from source review before running the focused test. Relevant behavior reviewed in `crates/core/src/text_cleanup.rs` and `crates/core/src/vocabulary_post_correction.rs`:

- The order is whitespace normalization, optional commands, vocabulary correction, protected punctuation spacing, then optional protected-aware filler removal.
- Punctuation tightening remains conservative: supported marks only; alphabetic predecessor; numeric-token guard; mark followed by whitespace or end; ambiguous punctuation/quotation sequence guard; candidate must not overlap a protected replacement.
- Vocabulary matching retains Unicode 18 canonical-caseless matching, combining-mark and other word-continuation boundaries, longest-match selection, and non-cascading replacement behavior within one call.
- Replacement text is copied byte-for-byte and is not normalized afterward. Source breaks remain in addition to command-generated breaks.

The focused run found no mismatches. **No expected strings were corrected.** The `0 ,` spacing after the new numeric written suffix is an expected source-confirmed result: the preceding character is numeric, so the punctuation rule does not remove that space. Existing `2026 .` spacing remains as before. Neither is a production behavior change.

## Verification

Focused test output (actual):

```text
Cleanup golden corpus: 168 fixtures passed
Categories covered: 8
Protected numeric tokens: 8 (0 altered)
Protected proper nouns: 8 (0 altered)
Current behavior baseline: true
```

Commands and results on macOS 26.6.2 arm64, Rust 1.99.1:

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS; 1 test, all 168 fixtures |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS; 9 passed, 0 failed |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; exit code 0, all workspace tests and doctests passed |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS |
| `git diff --cached --check` | PASS; run after staging only the five intended task files |

No physical 4 GB device, release-size/RSS measurement, platform integration matrix, production ASR acceptance, or release gate was evaluated by this corpus task.

## Changed files

- `crates/settings/tests/fixtures/cleanup_current_behavior.json`
- `crates/settings/tests/cleanup_golden_corpus.rs`
- `README.md`
- `docs/TASKS.md`
- `evidence/T-035-T-036-cleanup-168-fixtures.md`

## Remaining limitations

This is partial T-035/T-036 characterization only. The 168 fixtures remain 32 below AC-07's minimum of 200 and do not establish its ≥99% acceptance exact-output threshold, comprehensive protected-number/proper-noun guarantee, final Clean/Verbatim distinction, or production integration. T-035 prerequisites, deterministic/idempotence and production acceptance criteria remain open; T-036 policy and acceptance work remains open. `docs/CLEANUP_MODE_POLICY.md` is still **PROPOSED — NOT APPROVED**. **T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.**
