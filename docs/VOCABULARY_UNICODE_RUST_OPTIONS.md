# Rust Unicode Matching Implementation Review

**Status: ICU4X 2.4.0 SELECTED AND APPROVED FOR TASK 45.** Unicode 18.0.0 canonical-caseless matching is the approved F-08 uniqueness policy. This document records the Task 45 implementation and dependency review. Milestone 3 later applied the approved validator to schema-v1 settings and added a reusable Rust persistence service; schema migration, transcript replacement, and frontend integration remain incomplete, and AC-08 is not complete.

The Task 45 snapshot notes below preserve the core-only status at that time. For current settings behavior and provenance limits, see [the Milestone 3 service update](VOCABULARY_MATCHING_MIGRATION.md#milestone-3-settings-service-update-2026-11-06).

## Selected implementation

Use exactly two direct dependencies in `outflow-core`:

```toml
icu_casemap = { version = "=2.4.0", default-features = false, features = ["compiled_data"] }
icu_normalizer = { version = "=2.4.0", default-features = false, features = ["compiled_data"] }
```

The versioned API is `comparison_key(phrase, VocabularyMatchingVersion::Unicode18CanonicalCaselessV1)` in `crates/core/src/vocabulary_matching.rs`. `CURRENT_VOCABULARY_MATCHING_VERSION` is used by the `outflow-core` collection validator. It applies the D145 transformation `NFD(toCasefold(NFD(input)))` with the default full, non-Turkic `CaseMapper::fold_string()` and borrowed NFD compiled data. The generated `icu_casemap_data`, `icu_normalizer_data`, and `icu_properties_data` 2.4.0 packages identify Unicode 18.0.0. No runtime provider or data download is used. At the Task 45 snapshot, settings validation was outside this task's core-only scope; Milestone 3 now uses the same collection validator for persisted settings.

Version names identify both algorithm and Unicode data. Future data or algorithm changes require a new enum variant and migration review. Original spoken and written values remain unchanged, and derived keys are temporary. This matching change does not implement persistence, CRUD, transcript replacement, or frontend integration.

## Resolved graph and feature selection

The ordinary Cargo resolver added 26 packages to `Cargo.lock`. The complete graph is reproducible with:

```sh
RUSTUP_TOOLCHAIN=stable cargo tree -p outflow-core --locked
RUSTUP_TOOLCHAIN=stable cargo tree -p outflow-core --locked --edges features
```

The 26 newly locked packages are:

```text
displaydoc 0.2.7
icu_casemap 2.4.0
icu_casemap_data 2.4.0
icu_collections 2.4.0
icu_locale_core 2.4.0
icu_normalizer 2.4.0
icu_normalizer_data 2.4.0
icu_properties 2.4.0
icu_properties_data 2.4.0
icu_provider 2.4.0
litemap 0.8.3
potential_utf 0.1.6
smallvec 1.16.4
stable_deref_trait 1.2.1
synstructure 0.14.0
tinystr 0.8.5
utf8_iter 1.1.0
utf_types 1.0.0
writeable 0.6.4
yoke 0.8.4
yoke-derive 0.8.4
zerofrom 0.1.8
zerofrom-derive 0.1.8
zerotrie 0.2.5
zerovec 0.11.9
zerovec-derive 0.11.7
```

`compiled_data` enables baked case-map and normalizer data; `icu_casemap` also enables the compiled properties data it uses. `default-features = false` avoids unrelated defaults. The feature tree does not enable `serde`, datagen, or experimental functionality. Existing workspace packages such as `proc-macro2`, `quote`, `syn`, and `unicode-ident` are reused; they are not additional ICU4X direct dependencies.

## Licensing and dependency review

Cargo metadata for the newly resolved packages reports Unicode-3.0 for ICU4X components, their generated data crates, and Unicode-derived support crates including `litemap`, `potential_utf`, `tinystr`, `writeable`, `yoke`, `zerofrom`, `zerotrie`, and `zerovec` families. `displaydoc`, `smallvec`, and `stable_deref_trait` are MIT OR Apache-2.0; `synstructure` is MIT; `utf8_iter` and `utf_types` are Apache-2.0 OR MIT. Preserve Unicode's applicable copyright and permission notice and include required crate license texts in redistributed notices. See Task 45 evidence for the full `cargo metadata` license inventory.

The generated data is compiled into the application; it does not add runtime network behavior. The increased locked supply-chain surface still needs ordinary source, vulnerability, and license monitoring. The lockfile pins versions and registry checksums but does not certify that every dependency is free of vulnerabilities. `cargo audit`/`cargo deny` results, if unavailable or not run, are recorded explicitly in the Task 45 evidence.

## Alternatives retained as historical review

| Option | Findings | Outcome |
|---|---|---|
| `caseless` + `unicode-normalization` | Audited `caseless` 0.2.2 and `unicode-normalization` 0.1.24 implement the right operation shape but use Unicode 16.0.0 data. | Does not meet approved Unicode 18.0.0 policy. |
| `unicode-casefold` + normalization | Audited `unicode-casefold` 0.2.0 uses Unicode 9.0.0 folding data, mismatched with current normalization data. | Does not meet policy. |
| Rust `str::to_lowercase()` | Locale independent, but not full case folding and does not normalize canonical equivalents. | Retained only as the benchmark baseline. |
| Custom generated tables | Would require maintaining full normalization algorithms, folding mappings, data provenance, conformance and license notices. | Rejected in favor of the approved ICU4X crates. |

## Performance and binary measurements

Task 45 records matched release measurements from the same Rust toolchain, macOS arm64 host, and Cargo release profile. In the paired benchmark, ASCII 500-entry validation measured 44,933.33 ns for the lowercase reference and 43,420.62 ns for ICU4X (0.97×); mixed multilingual 500-entry validation measured 202,558.33 ns and 642,786.88 ns (3.17×). The pre-change `vocabulary_validation` executable was 457,104 bytes; the post-change executable was 564,192 bytes (+107,088 bytes, 23.43%). Synthetic benchmark-process peak RSS changed from 2,228,224 bytes to 2,293,760 bytes (+65,536 bytes, 2.94%). Example size is not installer size; benchmark-process RSS is not the application's process-tree peak or a physical 4 GB release test. See [Task 45 evidence](../evidence/rust-vocabulary-icu4x-task-45.md) for all workloads, the separate pre-edit baseline, and measurement limits.

## Persistence and future migration

Task 45 did not persist matching keys. At that snapshot, the settings crate still used its pre-existing lowercase-only rule. Milestone 3 now validates schema-v1 settings with Unicode 18, but does not persist comparison keys or record their provenance in the schema. Before a future policy version update, scan saved entries for newly colliding keys and require recoverable resolution; do not overwrite, normalize, delete, or silently select an entry. Update policy versions only with reviewed Unicode data, UCD regressions, cross-platform checks, collision analysis, license review, and fresh size measurements.

## Primary references

- [ICU4X 2.4.0 release](https://github.com/unicode-org/icu4x/releases/tag/icu@2.4.0)
- [`icu_casemap` 2.4.0 API and manifest](https://docs.rs/icu_casemap/2.4.0/icu_casemap/)
- [`icu_normalizer` 2.4.0 API and manifest](https://docs.rs/icu_normalizer/2.4.0/icu_normalizer/)
- [Unicode 18.0.0 D145, Unicode core specification Chapter 3](https://www.unicode.org/versions/Unicode18.0.0/core-spec/chapter-3/)
- [Unicode 18.0.0 `CaseFolding.txt`](https://www.unicode.org/Public/18.0.0/ucd/CaseFolding.txt)
- [Unicode 18.0.0 `NormalizationTest.txt`](https://www.unicode.org/Public/18.0.0/ucd/NormalizationTest.txt)
- [Unicode data license](https://www.unicode.org/license.txt)
