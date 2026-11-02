# Vocabulary Unicode Matching Policy

**Status: APPROVED AND IMPLEMENTED IN TASK 45.** The maintainer approved Unicode 18.0.0 canonical-caseless matching and the exact ICU4X 2.4.0 dependencies; focused and full local verification passed as recorded in the Task 45 evidence. This policy changes spoken-phrase uniqueness only; it does not implement vocabulary persistence, CRUD, transcript replacement, or frontend integration. AC-08, T-017, T-018, T-038, and T-039 remain incomplete.

## Selected policy

Use Unicode 18.0.0 canonical-caseless matching, Unicode definition D145. For each original spoken phrase, derive a comparison key with:

```text
NFD(toCasefold(NFD(input)))
```

`toCasefold` is the default locale-independent **full** case fold, including mappings that expand to multiple characters. The second NFD is required because case folding does not preserve normalization form. Compare keys by exact string equality.

The implementation is in [`crates/core/src/vocabulary_matching.rs`](../crates/core/src/vocabulary_matching.rs), exposed as:

```rust
comparison_key(phrase, VocabularyMatchingVersion::Unicode18CanonicalCaselessV1)
```

`CURRENT_VOCABULARY_MATCHING_VERSION` identifies the version used by collection validation. The stable label is `Unicode 18.0.0 canonical caseless V1`. Add a new enum variant when either the algorithm or data version changes; do not change the meaning of an existing version. The default policy is selected in Rust core, not in the UI.

## ICU4X implementation and Unicode data

`outflow-core` directly depends on exactly:

```toml
icu_casemap = { version = "=2.4.0", default-features = false, features = ["compiled_data"] }
icu_normalizer = { version = "=2.4.0", default-features = false, features = ["compiled_data"] }
```

`CaseMapper::new().fold_string()` performs default full folding. `DecomposingNormalizerBorrowed::new_nfd().normalize()` performs canonical NFD. Both use ICU4X's compiled borrowed data; no runtime provider, locale, downloaded data, or data service is constructed. No aggregate `icu` crate, `caseless`, `unicode-normalization`, `serde`, datagen, or experimental component is enabled for this work.

The exact `icu_casemap_data`, `icu_normalizer_data`, and `icu_properties_data` 2.4.0 packages identify generated Unicode 18.0.0 data. ICU4X 2.4 release materials also identify Unicode 18.0.0. The task tests include vectors present in the official [Unicode 18.0.0 `CaseFolding.txt`](https://www.unicode.org/Public/18.0.0/ucd/CaseFolding.txt) and [Unicode 18.0.0 `NormalizationTest.txt`](https://www.unicode.org/Public/18.0.0/ucd/NormalizationTest.txt); this is focused regression coverage, not a claim that the full Unicode conformance suite was run.

## Behavior and boundaries

Original spoken and written values remain unchanged. Keys are temporary values for duplicate detection and are not persisted. Existing per-entry validation still checks original input before duplicate matching, including the 128-scalar limit. Collection validation still reports errors in this order: collection size, invalid entries in input order, then duplicate pairs in input order. Written replacements do not affect uniqueness.

| Pair | Result | Reason |
|---|---|---|
| `rust` / `Rust` | Equal | Case folding |
| `CAFÉ` / `café` | Equal | Case folding |
| `straße` / `STRASSE` | Equal | Full sharp-s expansion |
| `é` / `e\u0301` | Equal | Canonical equivalence |
| `Σ` / `σ` | Equal | Greek case folding |
| `σ` / `ς` | Equal | Final sigma folds to sigma |
| `North Parkade` / `north parkade` | Equal | Case folding |
| `north parkade` / `north parkade ` | Different | No trimming |
| `I` / `ı` | Different | Default, non-Turkic fold |
| `İ` / `i` | Different | Default full fold retains combining dot |
| `①` / `1` | Different | No compatibility normalization |
| Latin `a` / Cyrillic `а` | Different | No cross-script confusable mapping |

Regression coverage also exercises Cyrillic case, Japanese and Devanagari uncased text, Deseret supplementary-plane case, emoji, canonical combining-mark reordering, the `ﬀ` full-fold mapping, leading/trailing whitespace, zero-width space, and zero-width joiner. Accents are retained (`é` differs from `e`). Full case folding can equate some compatibility-related forms, such as `ﬀ` and `ff`; this is a consequence of the Unicode full-fold mapping and is not NFKC.

No locale-sensitive casing, accent removal, NFKC/NFKD, phonetic matching, or visual-confusable substitution is applied. This policy does not define word-boundary text replacement.

## Difference from the provisional lowercase behavior

The former key `spoken.to_lowercase()` detected ordinary lowercase variants but did not implement full Unicode case folding or normalization. The approved key now rejects `straße` / `STRASSE` and composed `é` / decomposed `e\u0301` as duplicate pairs. It also treats dotted/dotless I according to default Unicode folding rather than a selected language. Existing saved entries are not yet part of this repository implementation because vocabulary persistence is not implemented.

## Dependency and license review

The lockfile resolution adds 26 packages to support the two direct ICU4X dependencies. The full resolved graph and enabled features are recorded by the exact commands in [Task 45 evidence](../evidence/rust-vocabulary-icu4x-task-45.md). The ICU component, generated-data, and Unicode-derived support crates use the Unicode-3.0 license. Other new support crates use MIT, Apache-2.0, or MIT OR Apache-2.0; `synstructure` is MIT. Retain the applicable Unicode copyright and permission notice and each crate's required license text when distributing the product. The lockfile pins package versions and checksums; a dependency graph is not a substitute for ongoing vulnerability, source-provenance, or license review.

Compiled data removes runtime data downloads and host-OS Unicode-version differences, but the linked library/data graph increases build inputs and binary bytes. No package-size estimate is treated as a binary-size result. Task 45 measures the example executable and benchmark RSS; those measurements do not certify the packaged application or release budget.

## Task 45 local performance and footprint results

On macOS 26.6.2 arm64 with Rust 1.99.1 and the default release profile, the paired benchmark measured ASCII individual key generation at 40.20 ns for the lowercase reference and 40.26 ns for ICU4X with the verified ASCII fast path. A mixed multilingual key measured 541.03 ns and 1,528.89 ns respectively (2.83×). Valid 500-entry validation measured 44,933.33 ns vs 43,420.62 ns for ASCII (0.97×), and 202,558.33 ns vs 642,786.88 ns for multilingual text (3.17×). The full per-workload results and the separately captured pre-edit baseline are in [Task 45 evidence](../evidence/rust-vocabulary-icu4x-task-45.md).

The same `vocabulary_validation` release example grew from 457,104 bytes before ICU4X to 564,192 bytes after integration (+107,088 bytes, 23.43%). The synthetic benchmark process maximum RSS measured 2,228,224 bytes before and 2,293,760 bytes after (+65,536 bytes, 2.94%). These are example-process measurements only; they do not measure the packaged installer or Outflow's full process tree and cannot establish the 950 MB release gate.

## Key migration and release considerations

- Do not persist comparison keys in this task. If keys are persisted in a later schema, persist their policy version and derive them from unchanged original strings.
- The Unicode 18 key is currently integrated into the `outflow-core` collection validator. `crates/settings/src/lib.rs` still performs its existing lowercase-only uniqueness check while validating serialized settings. That C-08 path is intentionally unchanged here; connect it to this policy in a separately scoped settings-integration task before relying on it for stored vocabulary conflicts.
- Before loading or migrating persisted vocabulary under a new key version, scan for newly colliding pairs. Report conflicts and require recoverable user resolution; never discard, rewrite, trim, or silently select an entry.
- Any Unicode data or algorithm update requires a new reviewed matching version, updated UCD regression vectors, a collision migration plan, and new cross-platform and size measurements.
- Cross-platform behavior follows the pinned compiled data rather than host Unicode libraries, but native macOS, Windows, and Linux builds still need normal release verification.
- Example-executable size and RSS are development measurements. The installed application, process-tree peak RSS, offline installer, accessibility, and hardware release gates remain unverified.

## References

- [Unicode 18.0.0 core specification, Chapter 3, §3.13.5, D145](https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-3/)
- [Unicode Standard Annex #15, Version 58](https://www.unicode.org/reports/tr15/tr15-58.html)
- [Unicode 18.0.0 `CaseFolding.txt`](https://www.unicode.org/Public/18.0.0/ucd/CaseFolding.txt)
- [Unicode 18.0.0 `NormalizationTest.txt`](https://www.unicode.org/Public/18.0.0/ucd/NormalizationTest.txt)
- [ICU4X 2.4.0 release](https://github.com/unicode-org/icu4x/releases/tag/icu@2.4.0)
- [ICU4X `icu_casemap` 2.4.0](https://docs.rs/icu_casemap/2.4.0/icu_casemap/)
- [ICU4X `icu_normalizer` 2.4.0](https://docs.rs/icu_normalizer/2.4.0/icu_normalizer/)
- [Unicode data license](https://www.unicode.org/license.txt)
