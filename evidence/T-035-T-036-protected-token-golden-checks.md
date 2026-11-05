# T-035/T-036 Protected-Token Golden Checks

**Result:** PASS for the requested test-only preservation infrastructure; partial T-035/T-036 progress only.
**Feature/acceptance:** F-07 / AC-07 infrastructure. AC-07 remains unchecked.
**Task branch:** `codex/task-035-036-protected-token-golden-checks`
**Base branch:** `codex/task-036-cleanup-interaction-fixtures`
**Base commit:** `629d43ee5553e06c83f1db53dfa9abd7fb561e3e`

## Implementation

The existing corpus remains exactly 72 fixtures and retains the same eight required fields. The optional test-only `protected_tokens` field is strict: token objects reject unknown fields and support `number` and `proper_noun`. The checker validates nonempty token text, ordered nonempty ranges, UTF-8 boundaries, exact input/output/expected bytes, and non-overlap in input and output. It uses `str::get` with checked bounds; token discovery is never performed by searching output text.

The 16 manually reviewed annotations are:

| Fixture ID | Kind | Token | Input bytes | Expected-output bytes |
|---|---|---|---:|---:|
| `numeric-version-spacing-protection` | number | `3 . 14` | `[8, 14)` | `[8, 14)` |
| `numeric-comma-grouping-and-year-preserved` | number | `1,234,567` | `[4, 13)` | `[4, 13)` |
| `numeric-comma-grouping-and-year-preserved` | number | `2026` | `[31, 35)` | `[31, 35)` |
| `numeric-version-like-string-preserved` | number | `v2 . 8 . 1` | `[6, 16)` | `[6, 16)` |
| `numeric-alphanumeric-identifiers-preserved` | number | `A7B-42` | `[4, 10)` | `[4, 10)` |
| `numeric-alphanumeric-identifiers-preserved` | number | `ZX9` | `[15, 18)` | `[15, 18)` |
| `numeric-multiple-tokens-preserved-in-sentence` | number | `9 . 30` | `[3, 9)` | `[3, 9)` |
| `numeric-multiple-tokens-preserved-in-sentence` | number | `2026-11-05` | `[13, 23)` | `[13, 23)` |
| `proper-noun-and-abbreviation-preservation` | proper_noun | `Élodie` | `[5, 12)` | `[4, 11)` |
| `proper-noun-accented-person-name-preserved` | proper_noun | `Élodie` | `[0, 7)` | `[0, 7)` |
| `proper-noun-multiword-place-preserved` | proper_noun | `North Parkade` | `[10, 23)` | `[10, 23)` |
| `proper-noun-uppercase-acronym-preserved` | proper_noun | `NASA` | `[0, 4)` | `[0, 4)` |
| `proper-nouns-several-in-one-sentence-preserved` | proper_noun | `Maya` | `[0, 4)` | `[0, 4)` |
| `proper-nouns-several-in-one-sentence-preserved` | proper_noun | `Montréal` | `[17, 26)` | `[17, 26)` |
| `proper-nouns-beside-eligible-punctuation-spacing` | proper_noun | `UBC` | `[6, 9)` | `[6, 9)` |
| `unicode-decomposed-accent-preserved` | proper_noun | `Zoë` | `[14, 18)` | `[14, 18)` |

The annotations span 12 fixtures: five numeric fixtures and seven proper-noun fixtures. Accented tokens `Élodie`, `Montréal`, and `Zoë` include multibyte UTF-8 characters. `Élodie` in the abbreviation fixture has distinct offsets because `Dr .` becomes `Dr.` before the name.

Five synthetic negative cases passed by confirming rejection of an incorrect declared value, changed actual output, an out-of-bounds range, a range inside the UTF-8 bytes for `é`, and overlapping ranges. Additional synthetic schema checks confirm optional metadata parses and unknown fixture/token fields are rejected. The golden fixture corpus was not corrupted for negative tests.

## Verification

**Environment:** macOS 26.6.2 (25G83), Darwin 25.6.0, arm64; MacBook Pro with Apple M3 Pro and 18 GB RAM. `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, `cargo 1.99.1 (5f94df478 2026-08-27)`, `RUSTUP_TOOLCHAIN=stable`. This is not a physical 4 GB release-gate machine.

Commands and observed results:

- `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture` — PASS, 1 test.

  ```text
  Cleanup golden corpus: 72 fixtures passed
  Categories covered: 8
  Protected numeric tokens: 8 (0 altered)
  Protected proper nouns: 8 (0 altered)
  Current behavior baseline: true
  ```

- `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked` — PASS, 9 tests.
- `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` — PASS.
- `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` — PASS.
- `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` — PASS, including doc tests.
- `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` — PASS after replacing a needless `as_bytes().len()` with `str::len()` for UTF-8 byte length.
- `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` — PASS.
- `git diff --check` — PASS before staging this evidence record.
- `git diff --cached --check` — PASS after staging the five task files.

## Limitations and status

These checks protect only the selected annotations. They do not guarantee preservation for arbitrary numeric tokens or proper nouns, do not change production cleanup behavior, and do not establish ASR quality or production integration. The 72-case corpus remains below AC-07's 200-case minimum and does not demonstrate its 99% exact-output threshold or final acceptance criteria. T-035, T-036, AC-07, T-017, T-018, T-023, T-038, T-039, AC-08, and all release gates remain unchecked. The Verbatim recommendation remains **PROPOSED — NOT APPROVED**.
