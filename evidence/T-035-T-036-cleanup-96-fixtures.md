# Cleanup golden corpus expansion to 96 fixtures

```yaml
task_ids: [T-035, T-036] # partial characterization task only
feature_ids: [F-07]
acceptance: [AC-07]
owner: Codex
base_branch: codex/task-035-036-protected-token-golden-checks
base_commit: eb9393898a37061cb0e1461f5c1f82a7cd85d29c
result_branch: codex/task-035-036-cleanup-96-fixtures
result_commit: final tip of result_branch (reported in the task handoff)
platform: macOS arm64; Darwin 25.6.0; Rust 1.99.1; Cargo 1.99.1
result: PASS for requested characterization and local workspace checks
review: PENDING
```

## Scope and changes

Appended 24 synthetic fixtures to `crates/settings/tests/fixtures/cleanup_current_behavior.json`, changed the corpus size assertion and module description in `crates/settings/tests/cleanup_golden_corpus.rs`, and updated the README and task ledger. No production cleanup code, settings behavior/schema/default, dependency, or lockfile changed. `docs/CLEANUP_MODE_POLICY.md` remains **PROPOSED — NOT APPROVED**.

The new IDs are grouped by the requested coverage:

### A — Multiline whitespace and commands

1. `multiline-whitespace-preserves-consecutive-paragraph-breaks`
2. `multiline-crlf-and-standalone-cr-normalize-together`
3. `multiline-normalized-new-line-command`
4. `multiline-new-paragraph-command-retains-existing-breaks`
5. `command-like-phrase-next-to-punctuation-remains-literal`
6. `commands-disabled-multiline-keeps-both-command-phrases`

### B — Punctuation and numeric boundaries

1. `punctuation-multiple-eligible-spacing-corrections`
2. `punctuation-ellipsis-and-repeated-marks-stay-ambiguous`
3. `punctuation-version-decimal-and-year-boundaries`
4. `punctuation-alphanumeric-identifier-digit-boundary`
5. `punctuation-unicode-alphabetic-predecessors`
6. `punctuation-spacing-before-existing-line-breaks`

### C — Clean/Verbatim filler boundaries

1. `clean-multiline-removes-both-supported-line-leading-fillers`
2. `clean-disabled-filler-removal-keeps-fillers-after-vocabulary`
3. `verbatim-enabled-filler-setting-preserves-multiline-fillers`
4. `clean-filler-before-nonalphabetic-character-is-kept`
5. `verbatim-disabled-filler-removal-preserves-repeated-line-leading-fillers`
6. `clean-command-paragraph-break-then-removes-filler`

### D — Unicode and protected vocabulary interactions

1. `unicode-vocabulary-canonical-equivalent-spoken-match`
2. `unicode-emoji-surrounds-vocabulary-replacement`
3. `vocabulary-written-punctuation-and-whitespace-stay-exact`
4. `vocabulary-replacements-remain-separated-by-paragraph-break`
5. `vocabulary-multiline-filler-like-replacement-stays-protected`
6. `vocabulary-longest-match-next-to-punctuation`

## Preservation verification

The first 72 parsed fixture objects were compared in order to the base commit's JSON objects using compact JSON serialization with insertion order retained. The same comparison confirmed all original `protected_tokens` values. The final JSON had exactly 96 items, 24 unique appended IDs, 96 unique total IDs, and all four `(mode, remove_fillers)` pairs plus both `commands_enabled` values.

The one-off development check was run inline; no permanent script was added:

```sh
python3 - <<'PY'
import json, subprocess
from pathlib import Path
base='eb9393898a37061cb0e1461f5c1f82a7cd85d29c'
path='crates/settings/tests/fixtures/cleanup_current_behavior.json'
old=json.loads(subprocess.check_output(['git','show',f'{base}:{path}']))
new=json.loads(Path(path).read_text(encoding='utf-8'))
canon=lambda value: json.dumps(value, ensure_ascii=False, separators=(',', ':'), sort_keys=False)
assert len(old)==72 and len(new)==96
for i in range(72):
    assert canon(old[i])==canon(new[i]), f'original fixture changed at index {i}: {old[i].get("id")}'
old_annotations={f['id']:f.get('protected_tokens',[]) for f in old if f.get('protected_tokens')}
new_annotations={f['id']:f.get('protected_tokens',[]) for f in new[:72] if f.get('protected_tokens')}
assert old_annotations==new_annotations
all_ids=[f['id'] for f in new]
added_ids=[f['id'] for f in new[72:]]
assert len(set(added_ids))==24 and len(set(all_ids))==96
counts={kind:sum(t['kind']==kind for f in old for t in f.get('protected_tokens',[])) for kind in ['number','proper_noun']}
assert counts=={'number':8,'proper_noun':8}
assert {(f['mode'],f['remove_fillers']) for f in new}=={('clean',True),('clean',False),('verbatim',True),('verbatim',False)}
assert {f['commands_enabled'] for f in new}=={True,False}
print('Original objects preserved exactly:', len(old))
print('Original protected annotations unchanged:', sum(map(len,old_annotations.values())), 'across', len(old_annotations), 'fixtures')
print('Protected counts:', counts)
print('New IDs unique:', len(added_ids))
print('All IDs unique:', len(set(all_ids)))
print('Four mode/filler combinations: PASS')
print('Both command settings covered: PASS')
PY
```

Result: PASS. The base had 72 objects; the final file has 96. The 16 original annotations (eight numeric and eight proper-noun) are unchanged across the same 12 fixtures. The 24 additions have unique IDs, and all 96 IDs are unique. All four mode/filler combinations and both command settings are present.

The Rust runner still performs exact full-output comparison in addition to optional protected-token assertions. Its five synthetic negative checks, schema checks, strict UTF-8 boundary/range validation, exact-byte checks, deterministic re-run checks, immutability checks, eight numeric/eight proper-noun count assertions, and minimum 12 annotated-fixture assertion were not changed.

## Expected-output review notes

Expectations were written from source review of `text_cleanup.rs`, `vocabulary_post_correction.rs`, and `cleanup_policy.rs`, then compared by the focused test. They were not copied from pipeline output.

- The first whitespace expectation mistakenly counted three newline characters after `Second`; the fixture input contains two. Source inspection confirmed whitespace normalization preserves the actual two, so the explicit expected string was corrected to two.
- A `new paragraph` command between existing line breaks emits two command newlines and retains the existing break before and after the phrase. The characterized output therefore has four newline characters between `Top` and `Bottom`.
- The first draft of the canonical-equivalent vocabulary case mapped only `café` while the source also contained `Society`. The mapper correctly kept that unmatched suffix, yielding `Café Society Society!`. The fixture was refined to match the complete decomposed `café society` phrase; its manually specified output is `Café Society!`.
- `punctuation-version-decimal-and-year-boundaries` retains the conservative `2026 .` spacing because the adjacent token contains a number; its eligible comma spacing is still corrected.
- Verbatim with `remove_fillers: true` preserves fillers, while vocabulary and punctuation stages still run according to the current adapter. Multiline written replacements remain exact, including filler-like lines and intentional spaces/punctuation.

No production behavior was changed to accommodate these fixtures.

## Validation

Environment: macOS arm64 (Darwin 25.6.0), Rust 1.99.1, Cargo 1.99.1.

Focused test output:

```text
Cleanup golden corpus: 96 fixtures passed
Categories covered: 8
Protected numeric tokens: 8 (0 altered)
Protected proper nouns: 8 (0 altered)
Current behavior baseline: true
```

Commands and results:

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` | PASS — 96 fixtures; exact output above |
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` | PASS — 9 tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — all unit, integration, and doctest binaries exited successfully |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `git diff --check` | PASS |
| `git diff --cached --check` after staging intended files only | PASS — five intended task files staged; unrelated untracked paths left unstaged |

## Remaining limitations

This is a 96-fixture current-behavior characterization slice. AC-07 still requires at least 200 transformations and its separate ≥99% exact-output and protected-corpus acceptance evidence. These fixtures do not prove universal protection, final product semantics, application integration, model quality, platform support, or release readiness. The proposed Verbatim alternative remains **NOT APPROVED**. T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked. The result branch tip after commit is the deliverable commit reference.
