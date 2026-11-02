# Task 45 — Unicode 18.0.0 vocabulary matching with ICU4X

```yaml
task_id: Task 45 (Unicode matching implementation)
feature_ids: [F-08]
flows: []
acceptance: [AC-08]
owner: /root coordinator
base_commit: 6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b
result_commit: UNCOMMITTED
platform: macOS 26.6.2 / arm64 / Rust 1.99.1 / stable / host release profile
implementation: Unicode 18.0.0 D145 vocabulary matching through ICU4X 2.4.0; persistence and replacement excluded
doc_updates: README, ARCHITECTURE C-06, TECH_STACK TS-06, Unicode policy/options, third-party notice status, this evidence
risks: outflow-settings schema validation remains lowercase-only; packaging and cross-platform release gates remain unmeasured
review: PENDING
```

## Baseline captured before comparator/dependency edits

The baseline used the current dirty Task 44 worktree at base commit `6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b`; it was not substituted with an older commit. The branch was `codex/task23-moonshine-build-scope`. Existing uncommitted changes in `README.md`, `crates/core/src/vocabulary.rs`, the Task 43/44 policy/audit files, and Python scripts were preserved. The baseline source path used the actual `outflow-core` crate before adding the ICU4X dependencies or changing `spoken.to_lowercase()`.

Environment: `rustc 1.99.1 (58cf8f9c3 2026-10-13)`, host `aarch64-apple-darwin`, macOS 26.6.2 arm64. Cargo's standard release profile was used; the workspace defines no custom release LTO or strip profile, and no custom build flags were supplied. Both comparator and size baselines used this host/toolchain/profile.

Exact commands:

```sh
RUSTUP_TOOLCHAIN=stable cargo build -p outflow-core --example vocabulary_validation --release --locked
RUSTUP_TOOLCHAIN=stable cargo run --manifest-path /private/tmp/outflow-task45-baseline/Cargo.toml --release --offline
/usr/bin/time -l /private/tmp/outflow-task45-baseline/target/release/outflow-task45-baseline
```

The temporary harness was created under `/private/tmp`; it depended on the repository's current `crates/core` path and did not alter source files or retain baseline code in the repository. It measured seven runs per case after warm-up using `std::time::Instant` and `std::hint::black_box`. Key generation used 20,000 iterations/run with 2,000 warm-up calls. Collection validation used 10,000 iterations/run for one entry, 1,000 for 100 entries, and 200 for 500 entries; each used 100 warm-up calls. Timings below are medians in nanoseconds per operation. Inputs were synthetic ASCII or mixed multilingual text. Duplicate cases repeated the first spoken phrase only at the final index.

| Baseline case | Input/workload size | Median ns/op |
|---|---:|---:|
| ASCII individual lowercase key | 1 phrase | 37.96 |
| Mixed multilingual individual lowercase key | 1 phrase | 420.48 |
| ASCII valid collection | 1 entry | 121.44 |
| ASCII valid collection | 100 entries | 9,370.17 |
| ASCII valid collection | 500 entries | 44,831.04 |
| ASCII duplicate at last entry | 100 entries | 9,230.50 |
| ASCII duplicate at last entry | 500 entries | 44,849.58 |
| Mixed multilingual valid collection | 1 entry | 431.45 |
| Mixed multilingual valid collection | 100 entries | 40,267.00 |
| Mixed multilingual valid collection | 500 entries | 212,282.92 |
| Mixed multilingual duplicate at last entry | 100 entries | 43,925.54 |
| Mixed multilingual duplicate at last entry | 500 entries | 205,059.58 |

The pre-change `vocabulary_validation` release executable was **457,104 bytes**. Its Cargo build used the existing default release profile.

Peak RSS was measured with macOS `/usr/bin/time -l` on the synthetic baseline harness: **2,228,224 bytes maximum resident set size**; reported peak memory footprint was 1,605,944 bytes. The tool first returned `sysctl kern.clockrate: Operation not permitted` in the sandbox, so the same read-only measurement was retried with approved host measurement access. This is a benchmark-process measurement, not Outflow's application process tree.

Measurement limits: sub-microsecond key timings varied across separate process runs (ASCII key medians observed were 75.29 ns and 37.96 ns); interpret small changes cautiously. The host measurement is not a physical 4 GB device, and this executable is not an installer. No release-cap claim follows from it.

## Implementation

- Added `outflow_core::vocabulary_matching::VocabularyMatchingVersion::Unicode18CanonicalCaselessV1`, `CURRENT_VOCABULARY_MATCHING_VERSION`, `comparison_key`, and the stable label `Unicode 18.0.0 canonical caseless V1`.
- The key function applies `NFD(toCasefold(NFD(input)))` using `CaseMapper::new().fold_string()` and `DecomposingNormalizerBorrowed::new_nfd()` from compiled data. Borrowed data constants avoid owned provider construction. Default folding is full and non-Turkic.
- The ASCII path uses `to_ascii_lowercase()` only when `str::is_ascii()` holds. A test compares it with the ICU reference for every pair of 7-bit ASCII scalar values (16,384 pairs).
- `validate_vocabulary_collection` computes the versioned key only after size and all per-entry checks, preserving the API, 500-entry cap, 128-scalar original-value limit, typed errors, first-failure order, and redaction. Original spoken/written values are unchanged.
- Replaced the Task 43 lowercase-only limitation tests with the approved matrix and added full-fold, canonical-combining-order, confusable, script, whitespace, format-character, supplementary-plane, duplicate-replacement, and last-entry collision coverage.
- Updated the example to report live validation outcomes and the selected policy. Added a standard-library release benchmark comparing the prior lowercase calculation and the production ICU validator on matching synthetic datasets.
- No persistence, CRUD, text replacement, or frontend code was added.

## Dependency resolution and license review

The only new direct `outflow-core` dependencies are:

```toml
icu_casemap = { version = "=2.4.0", default-features = false, features = ["compiled_data"] }
icu_normalizer = { version = "=2.4.0", default-features = false, features = ["compiled_data"] }
```

Cargo resolved the lockfile normally. The generated `icu_casemap_data`, `icu_normalizer_data`, and `icu_properties_data` 2.4.0 sources report Unicode 18.0.0. The `outflow-core` feature tree enables the two `compiled_data` features and ICU's baked/provider data; it does not enable serde, datagen, or experimental functionality. No aggregate `icu`, `caseless`, or `unicode-normalization` dependency was added.

The 26 newly locked transitive/direct packages are `displaydoc 0.2.7`, `icu_casemap 2.4.0`, `icu_casemap_data 2.4.0`, `icu_collections 2.4.0`, `icu_locale_core 2.4.0`, `icu_normalizer 2.4.0`, `icu_normalizer_data 2.4.0`, `icu_properties 2.4.0`, `icu_properties_data 2.4.0`, `icu_provider 2.4.0`, `litemap 0.8.3`, `potential_utf 0.1.6`, `smallvec 1.16.4`, `stable_deref_trait 1.2.1`, `synstructure 0.14.0`, `tinystr 0.8.5`, `utf8_iter 1.1.0`, `utf_types 1.0.0`, `writeable 0.6.4`, `yoke 0.8.4`, `yoke-derive 0.8.4`, `zerofrom 0.1.8`, `zerofrom-derive 0.1.8`, `zerotrie 0.2.5`, `zerovec 0.11.9`, and `zerovec-derive 0.11.7`.

Cargo metadata license expressions for those additions group as follows:

| License expression | Packages |
|---|---|
| Unicode-3.0 | All nine `icu_*` component/data crates; `litemap`, `potential_utf`, `tinystr`, `writeable`, `yoke`, `yoke-derive`, `zerofrom`, `zerofrom-derive`, `zerotrie`, `zerovec`, `zerovec-derive` |
| MIT OR Apache-2.0 | `displaydoc`, `smallvec`, `stable_deref_trait` |
| MIT | `synstructure` |
| Apache-2.0 OR MIT | `utf8_iter`, `utf_types` |

The applicable Unicode copyright and permission notice must be retained with any redistribution. The root `THIRD_PARTY_NOTICES.md` remains a template, not a completed release notice bundle; it now records the ICU4X addition and links this inventory. `cargo audit` and `cargo deny` are not installed, so no advisory database or deny-policy scan was run. The locked version/source/checksum inventory is not a substitute for ongoing vulnerability, provenance, or license review.

Exact dependency inspection commands:

```sh
RUSTUP_TOOLCHAIN=stable cargo tree -p outflow-core --locked
RUSTUP_TOOLCHAIN=stable cargo tree -p outflow-core --locked --edges features
RUSTUP_TOOLCHAIN=stable cargo metadata --locked --format-version 1
```

All completed successfully. Cargo's ordinary resolver, not manual lockfile editing, wrote the 26 package records and checksums.

## Unicode regression data and results

The test source includes verified Unicode 18.0.0 UCD vectors: `CaseFolding.txt` specifies U+00DF → `ss`, U+FB00 → `ff`, default U+0130 → `i` + U+0307, and U+10400 → U+10428. `NormalizationTest.txt` Part 0 includes the canonical equivalence U+1E0A U+0323 = U+1E0C U+0307 and the corresponding combining-mark reordering. These sources are linked in the policy/options documents. This is focused regression coverage; the full UCD conformance suites were not run.

The Task 43 matrix passes: ASCII and accented case pairs, sharp-s expansion, composed/decomposed accent, Greek sigma/final sigma, and multiword case pairs match. Trailing whitespace, default dotted/dotless-I distinctions, circled digit vs ASCII digit, and Latin/Cyrillic confusables remain distinct. Additional tests pass for Cyrillic case, Japanese/Devanagari, Deseret, emoji, combining order, ligature folding, leading whitespace, zero-width space/joiner, deterministic keys, version label, original-text preservation, empty/500/501 boundaries, error order, and duplicate pairs with different replacements.

## Final benchmark comparison

Post-change benchmark command:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_matching_benchmark --release --locked
/usr/bin/time -l ./target/release/examples/vocabulary_matching_benchmark
```

The benchmark uses `Instant` and `black_box`, seven measured runs, warm-up of 2,000 key calls or 100 collection validations, and synthetic ASCII or multilingual phrases. Inputs are the same shape as the pre-change harness: one, 100, and 500 entries, plus a duplicate of the first spoken phrase at the final index. Key calls use 20,000 iterations per measured run; validation uses 10,000/1,000/200 iterations for 1/100/500 entries. Paired measurements alternate order. Times below are medians in nanoseconds per operation; the legacy comparator mirrors the pre-Task 45 validation order and `.to_lowercase()` key calculation.

| Case | Lowercase reference | ICU4X 2.4.0 | Change |
|---|---:|---:|---:|
| ASCII individual key | 40.20 ns | 40.26 ns | +0.15% (1.00×) |
| Mixed multilingual individual key | 541.03 ns | 1,528.89 ns | +182.59% (2.83×) |
| ASCII valid collection, 1 entry | 121.72 ns | 124.05 ns | +1.91% (1.02×) |
| ASCII valid collection, 100 entries | 9,153.88 ns | 8,796.75 ns | −3.90% (0.96×) |
| ASCII valid collection, 500 entries | 44,933.33 ns | 43,420.62 ns | −3.37% (0.97×) |
| ASCII duplicate at final entry, 100 entries | 9,141.83 ns | 8,798.17 ns | −3.76% (0.96×) |
| ASCII duplicate at final entry, 500 entries | 45,114.17 ns | 43,511.67 ns | −3.55% (0.96×) |
| Multilingual valid collection, 1 entry | 421.46 ns | 1,243.19 ns | +195.08% (2.95×) |
| Multilingual valid collection, 100 entries | 42,083.58 ns | 144,052.38 ns | +242.30% (3.42×) |
| Multilingual valid collection, 500 entries | 202,558.33 ns | 642,786.88 ns | +217.33% (3.17×) |
| Multilingual duplicate at final entry, 100 entries | 39,374.08 ns | 126,256.04 ns | +220.70% (3.21×) |
| Multilingual duplicate at final entry, 500 entries | 199,829.38 ns | 627,771.04 ns | +214.15% (3.14×) |

The ASCII fast path removed the initial large ASCII gap: the first ICU prototype measured an ASCII key at 458.60 ns vs 36.35 ns (12.62×) and 500-entry ASCII validation at 156,076.25 ns vs 45,334.79 ns (3.44×). After the tested fast path, paired ASCII key generation is within timing noise of lowercase and 500-entry ASCII validation is 0.97×.

## Binary size and memory

The same `vocabulary_validation` example target was built before and after integration with Rust 1.99.1 on macOS 26.6.2 arm64 using Cargo's default optimized release profile, without custom LTO/strip flags:

```sh
RUSTUP_TOOLCHAIN=stable cargo build -p outflow-core --example vocabulary_validation --release --locked
```

| Metric | Lowercase baseline | ICU4X 2.4.0 | Change |
|---|---:|---:|---:|
| Release example executable bytes | 457,104 | 564,192 | +107,088 (+23.43%) |
| Synthetic benchmark process max RSS | 2,228,224 | 2,293,760 | +65,536 (+2.94%) |
| `time -l` peak memory footprint | 1,605,944 | 1,638,712 | +32,768 (+2.04%) |

`/usr/bin/time -l` reports macOS process resource usage for the benchmark executable. These values do not measure an app process tree, final installer, physical 4 GB device, or release threshold. A separate first pre-edit benchmark invocation observed 75.29 ns for ASCII key generation vs 37.96 ns in the process used for baseline RSS, so small sub-microsecond cross-process changes are noisy. The final paired benchmark ratios are the more comparable timing result.

## Verification

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked` | PASS — 52 passed, 2 unrelated tests filtered |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS — 124 tests total (13 added by Task 45; 54 core, 70 settings); zero failures |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_validation --locked` | PASS — all generated cases PASS |
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_matching_benchmark --release --locked` | PASS — all synthetic cases completed |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS — 136 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS — typecheck and Vite production build |
| `python3 -m unittest discover -s scripts/tests -v` | PASS — 258 tests |
| `python3 scripts/count_installed_bytes.py --demo` | PASS — synthetic demo; no release certification |

`cargo audit --version` and `cargo deny --version` reported no such installed Cargo subcommands; advisory and deny-policy scans are **NOT RUN**. No Moonshine CMake configuration was attempted.

## Changed files

```text
README.md
Cargo.lock
crates/core/Cargo.toml
crates/core/src/lib.rs
crates/core/src/vocabulary.rs
crates/core/src/vocabulary_matching.rs
crates/core/examples/vocabulary_validation.rs
crates/core/examples/vocabulary_matching_benchmark.rs
docs/ARCHITECTURE.md
docs/TECH_STACK.md
docs/VOCABULARY_UNICODE_POLICY.md
docs/VOCABULARY_UNICODE_RUST_OPTIONS.md
THIRD_PARTY_NOTICES.md
evidence/rust-vocabulary-icu4x-task-45.md
```

`README.md`, `crates/core/src/vocabulary.rs`, `docs/VOCABULARY_UNICODE_POLICY.md`, and `docs/VOCABULARY_UNICODE_RUST_OPTIONS.md` already contained uncommitted Task 43/44 work before Task 45. That work was preserved and updated in place. Other pre-existing dirty files from the Task 43/44 audit were not changed. The task ledger was not changed, and T-017, T-018, T-038, T-039, and AC-08 remain incomplete.

## Remaining limitations and handoff

- `crates/settings/src/lib.rs` still uses lowercase-only comparison while validating serialized settings. This existing C-08 path was left untouched under the core-only Task 45 scope; reconcile it before relying on Unicode 18 uniqueness for stored vocabulary.
- No macOS integration beyond host compilation/tests was run; Windows/Linux builds and behavioral checks remain unverified.
- `cargo audit`/`cargo deny` are unavailable. The Cargo metadata license inventory is not legal sign-off, a complete third-party notice bundle, or redistribution approval.
- Example executable size and benchmark RSS do not prove installer size, whole-app process-tree peak RSS, or compliance with the 950 MB release budget.
- Existing-entry collision scanning/migration is not implemented because this task does not persist vocabulary. Before future persistence or version changes, scan for newly colliding pairs and require recoverable resolution without rewriting originals.
- Full Unicode 18 conformance suites were not run; the focused tests use checked Unicode 18 UCD vectors.

Branch/worktree remains `codex/task23-moonshine-build-scope` at base commit `6f15c8f3accbe6a0afb80fc6a8314f426cf9ad9b`, with the Task 45 changes uncommitted and unstaged. No task status, commit, push, merge, or release action was performed. Final review ran `git status --short --branch`, `git diff --check`, and `git diff -- Cargo.toml Cargo.lock crates/core/Cargo.toml`; the manifest diff contains only the two approved direct ICU4X dependencies, and the lockfile diff contains Cargo-resolved package records and checksums. Pre-existing uncommitted Task 43/44 files remain in the worktree, as listed in the status output, and were preserved.
