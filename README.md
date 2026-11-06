# Outflow

Outflow is an offline desktop dictation application in development. This repository contains a browser-based React foundation screen and Settings preview, a Rust workspace foundation, and a native Rust settings schema v1 model with JSON serialization, validated decoding, explicit-path loading and initialization, an explicitly confirmed reset, and an atomic JSON writer on Unix. The production Tauri shell and dictation flow, Windows owner-only ACL writing, and release packaging have not been implemented. A separate development-only native macOS microphone probe is documented below.

## Prerequisites

- Node.js **22.12.0 or newer**. The Vite 8 and Vitest 5 requirements set this minimum. The suggested version is recorded in `.node-version`.
- npm, included with Node.js.
- Rust **1.99.1** and Cargo. The root `rust-toolchain.toml` selects the documented toolchain and installs the `rustfmt` and `clippy` components when needed.

Vite documents its [Node.js support range](https://vite.dev/guide/) and Vitest documents its [Node.js requirement](https://vitest.dev/guide/).

## Supported hardware and OS test matrix

The development hardware inventory is in [`docs/BENCHMARK_MATRIX.md`](docs/BENCHMARK_MATRIX.md), with its machine-readable source at [`docs/benchmark-matrix.json`](docs/benchmark-matrix.json). It follows the current support tiers: macOS 13+ Apple Silicon arm64 (Tier 1, Phase 0 gated), macOS 13+ Intel x64 (Tier 2 gated), Windows 10/11 x64 (Tier 1), Ubuntu-family Linux X11 x64 (Tier 1), Fedora-family Linux X11 x64 (Tier 2), and GNOME/KDE Wayland x64 (best-effort, portal/compositor dependent).

The current development host is `MAC-ARM64-01`: macOS 26.6.2 (25G83), Apple M3 Pro, arm64, with **18 GB installed physical RAM**. This machine is available for macOS platform work but is not a physical 4 GB reference system. No physical 4 GB test machine is verified for any listed target. Intel macOS, Windows, Ubuntu-family X11, Fedora-family X11, and GNOME/KDE Wayland machines are unavailable in the current task environment. The exact Linux distribution release minima are not yet specified in `docs/TECH_STACK.md`.

Every target requires CPU-only testing on 4 GB installed physical RAM for the project gate. A VM, cgroup, or other memory limit is supplemental and does not certify the physical 4 GB requirement. Hardware identification and successful platform smoke tests do not establish performance certification; RSS, installed size, latency, real-time factor, speech quality, and battery gates remain unmeasured here. Wayland support remains best-effort with explicit Copy fallback when compositor or portal capabilities are missing.

Run the development-only standard-library validator and its `unittest` suite from the repository root:

```sh
python3 scripts/validate_benchmark_matrix.py
python3 -m unittest discover -s scripts/tests -v
```

The script checks required target IDs, availability labels, evidence fields, 4 GB claims, and prohibited identifying field names. It does not discover hardware, change application behavior, or become an Outflow runtime dependency.

### Hardware matrix troubleshooting

- If `python3` is missing, install Python 3 with its standard library and rerun the commands above; no pip package is needed.
- If validation reports a missing or duplicate target, correct `id` in `docs/benchmark-matrix.json`; each required target ID must appear exactly once.
- For `AVAILABLE`, record the observed OS version/build, CPU model, architecture, installed physical RAM, and source commands in `hardware` and `evidence`. Keep unknown values `null` for `PENDING` or `UNAVAILABLE` records.
- If a hardware query is denied, record that command as unavailable and use a direct OS hardware profile only when it reports the fact. Do not infer a CPU or RAM value from the target definition.
- Do not change the current machine's 18 GB to 4 GB or record a simulated memory limit as installed physical RAM. A physical 4 GB claim requires matching hardware facts and RAM evidence; application performance certification still needs the separate benchmark protocol in `docs/ARCHITECTURE.md` §8.

## Benchmark audio fixtures

The English ASR evaluation fixture manifest is [`docs/benchmark-fixtures.json`](docs/benchmark-fixtures.json), and its provenance, transcription, coverage, normalization, and redistribution policy is [`docs/BENCHMARK_FIXTURES.md`](docs/BENCHMARK_FIXTURES.md). These are development-only evaluation inputs; Outflow does not record user microphone audio for this corpus.

Validate the manifest and run all standard-library script tests from the repository root:

```sh
python3 scripts/validate_fixture_manifest.py
python3 -m unittest discover -s scripts/tests -v
```

To add a fixture safely, follow the fixture policy: verify the exact source and redistribution rights, select non-sensitive audio, create and human-check a separate reference file, record the observed recording condition, compute the SHA-256 of the final audio bytes, and add the metadata and verification evidence to the manifest. Then run both commands above and review the diff for personal recordings, customer speech, or sensitive transcripts. Never add audio or references before privacy and license review.

A fixture is benchmark-ready only when its local audio and non-empty human reference file exist, audio checksum matches, provenance and redistribution permission are verified with reviewable evidence, and its human transcription is verified. Planned entries currently have no reference recordings, verified references, permission evidence, or audio hashes; ready audio fixtures: **0**. Consequently the corpus is not benchmark-ready and T-003 and AC-06 remain incomplete.

Troubleshooting:

- **Manifest valid but ASR readiness is NOT READY:** this is expected while entries are planned. Validation does not create or download audio.
- **A ready entry is rejected for a missing file:** use a repository-relative path to an existing regular file and ensure symlinks resolve inside the repository.
- **Checksum mismatch:** recompute SHA-256 from the exact final audio bytes after any re-encoding or edit, then review the fixture as a changed asset.
- **Provenance or permission is not verified:** keep the fixture planned until the specific audio's source and redistribution rights are documented.
- **Human reference is not verified:** compare the entire recording against its separate reference; do not use an ASR output as the gold transcript.

## Synthetic Word Error Rate calculator

The development-only calculator scores lexical word errors between a reference transcription and a prediction. It uses word-level Levenshtein alignment and reports substitutions (**S**), deletions (**D**), insertions (**I**), and the reference word count (**N**) with:

```text
WER = (S + D + I) / N
```

Run the self-contained demonstration from the repository root:

```sh
python3 scripts/score_wer.py --demo
```

It uses the synthetic reference `the quick brown fox` and prediction `the quick fox`, and prints:

```text
Outflow WER calculator ready
Reference words: 4
Substitutions: 0
Deletions: 1
Insertions: 0
Word Error Rate: 25.00%
```

Code can also call `scripts.score_wer.calculate_wer(reference, prediction)`. The result contains edit counts, reference word count, edit distance, and `wer` as a fraction (`0.25` means 25%). For an empty normalized reference, `wer` is `None` because its denominator is zero; insertion counts are still returned. WER may exceed 100% when insertions outnumber reference words.

Normalization follows the fixture policy's NFC normalization, Unicode lowercase, collapsed whitespace, and punctuation-free lexical scoring. The policy does not define exact Unicode token boundaries. For this synthetic calculator only, the provisional `provisional-unicode-boundary-v1` tokenizer treats runs of Unicode letters, numbers, and combining marks as words, with punctuation and other characters separating tokens. It preserves token spellings, does not expand contractions or rewrite numbers, and does not apply vocabulary corrections. This provisional tokenizer is not a frozen benchmark rule; decide and document the exact tokenization policy before scoring any real corpus. Punctuation accuracy remains a separate evaluation and is not calculated here.

### Aggregate corpus WER

Individual WER describes one reference/prediction pair. Corpus WER combines all scored pairs by summing their alignment errors and reference words:

```text
Corpus WER = (total substitutions + total deletions + total insertions) / total reference words
```

This weights each utterance by its reference word count. Do not average individual WER percentages: the synthetic example below has individual WERs of 100% and 0%, whose arithmetic mean is 50%, while the single error is one of ten total reference words, so corpus WER is 10%.

Run the aggregate demonstration from the repository root:

```sh
python3 scripts/score_corpus_wer.py --demo
```

Expected output:

```text
Outflow aggregate WER ready
Utterances scored: 2
Reference words: 10
Substitutions: 1
Deletions: 0
Insertions: 0
Corpus Word Error Rate: 10.00%
```

Code can call `scripts.score_corpus_wer.calculate_corpus_wer()` with an iterable of `(reference, prediction)` string pairs. Its `wer` is a fraction; the result also reports utterances scored, total reference words, and summed edit counts. It reuses `scripts.score_wer.calculate_wer()` for each pair and the same provisional normalization.

The individual calculator preserves `wer=None` when a reference normalizes to no words. Corpus scoring rejects an empty collection and raises a clear error for any empty normalized reference; it never excludes such a sample or treats its undefined WER as 0%. Silence and no-speech detection must be evaluated separately from speech-recognition accuracy.

Both WER demonstrations and their tests use synthetic strings only. They do not run speech recognition, read audio, model files, real fixture transcripts, or persist evaluation results. Passing synthetic scoring tests is not a real recognition benchmark; benchmark scoring still requires the frozen, licensed, human-verified corpus and the separate model/hardware gates.

### Synthetic ASR results validation

The results validator checks whether per-sample numerical error counts, declared aggregate totals, and corpus WER agree. It accepts only schema version 1 with `result_type: "synthetic"`; it does not process audio or run a recognition model.

Run it from the repository root:

```sh
python3 scripts/validate_asr_results.py --file scripts/testdata/synthetic_asr_results.json
```

Expected output:

```text
Outflow ASR results validation passed
Result type: SYNTHETIC
Samples validated: 2
Reference words: 10
Substitutions: 1
Deletions: 0
Insertions: 0
Corpus WER: 10.00%
Real ASR benchmark status: NOT ASSESSED
```

See [`docs/BENCHMARK_RESULTS.md`](docs/BENCHMARK_RESULTS.md) for the schema and its limits. The validator rejects unknown fields, so result files cannot include transcripts, audio, model claims, or identifying metadata. It reports structural or arithmetic errors without printing input values and creates no output files. The complete test suite, including validator tests, runs with:

```sh
python3 -m unittest discover -s scripts/tests -v
```

A valid synthetic file demonstrates only arithmetic consistency. It does not verify how counts were obtained, run a real ASR model, certify benchmark performance, complete T-003 or T-010, or satisfy AC-06. Future measured results need a verified frozen corpus, fixed normalization, pinned model and licensing evidence, named hardware, and reproducible evaluation under the benchmark protocol. The manifest remains at 8 planned entries and 0 ready recordings.

Troubleshooting:

- **Unsupported result type:** only `synthetic` is accepted. Measured or certified results are outside this validator's scope.
- **Totals do not match:** recompute each aggregate from the sample counts and calculate WER from total errors divided by total reference words.
- **Declared WER does not match:** WER is a fraction in JSON (`0.1` means 10%), with a `1e-9` absolute comparison tolerance.
- **Unknown field:** remove it; only the documented schema is accepted. Never add audio, transcript, or personal information to a result file.
- **WER exceeds 100%:** this can be valid when insertions outnumber reference words; the validator does not cap WER at 100%.

## Installed-byte counter

[`scripts/count_installed_bytes.py`](scripts/count_installed_bytes.py) recursively measures logical file bytes in a directory supplied explicitly by the caller. It counts regular files without filtering filenames or extensions, reports symbolic links separately, and compares the aggregate logical size to the strict 950,000,000-byte decimal release threshold. See [`docs/INSTALLED_SIZE_MEASUREMENT.md`](docs/INSTALLED_SIZE_MEASUREMENT.md) for counting rules, platform limits, and the distinction between a directory measurement and release certification.

Run the synthetic demonstration:

```sh
python3 scripts/count_installed_bytes.py --demo
```

Expected output:

```text
Outflow installed-byte counter ready
Files counted: 3
Total logical bytes: 112
Limit bytes: 950000000
Budget comparison: BELOW LIMIT
Measurement type: SYNTHETIC
Release certification: NOT ASSESSED
```

Measure an installed application directory explicitly:

```sh
python3 scripts/count_installed_bytes.py --root <installation-directory>
```

The command prints structured JSON without the supplied path. Regular-file entries with hard links count once per directory name; symbolic-link entry lengths are reported separately and included in the aggregate total, while link targets are validated but never traversed. Missing, non-directory, or symlink roots; broken or escaping links; special entries; and unreadable metadata cause a controlled error.

Run the focused or full Python test suite with:

```sh
python3 -m unittest scripts.tests.test_count_installed_bytes -v
python3 -m unittest discover -s scripts/tests -v
```

The demo's 112 bytes come only from temporary synthetic placeholders. Neither the demo nor measuring an arbitrary directory certifies AC-01 or measures an actual Outflow installation. Actual release evidence requires installing the complete release on each target and counting its application directory, including the bundled model and native assets.

Troubleshooting:

- **Root error:** pass an existing directory with `--root`; the root itself cannot be a symbolic link.
- **Link error:** repair broken links and ensure links resolve inside the supplied root. Internal links are measured as link entries; their targets are not counted again through the link.
- **Special entry or permission error:** remove unsupported special entries from the measured tree or provide directory search/read and metadata access. Errors omit the path.
- **At or above limit:** the comparison requires fewer than 950,000,000 bytes. Review the actual installed tree; the synthetic demonstration does not represent release size.

Run the standard-library tests with:

```sh
python3 -m unittest discover -s scripts/tests -v
```

The WER tests use synthetic strings only; existing manifest-validator tests may create temporary synthetic files. The suite does not run speech recognition, load real recordings, read real fixture transcripts, or store benchmark results. Passing synthetic scoring tests is not a real ASR benchmark and does not satisfy T-003, T-010, or AC-06. Real evaluation still requires a frozen, licensed, human-verified corpus and the hardware/model gates in the benchmark protocol.

Troubleshooting:

- **The demo reports an argument error:** run it with `--demo` from the repository root.
- **An empty-reference result has no numeric WER:** this is intentional; there is no reference-word denominator. Silence/no-speech evaluation is separate.
- **A punctuation or contraction case is tokenized unexpectedly:** the punctuation boundary rule is provisional. Do not use it as a benchmark policy until the fixture policy is explicitly frozen.
- **You need benchmark results:** the manifest currently has zero ready audio fixtures; this calculator does not load audio, run an ASR model, or create results.

## Rust workspace foundation

The Cargo workspace is separate from the React/Vite app. `outflow-core` uses pinned ICU4X dependencies for Unicode 18 vocabulary matching; `outflow-settings` uses pinned Serde and `serde_json` dependencies for schema v1 JSON:

```text
Cargo.toml
rust-toolchain.toml
crates/
├── core/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── text_cleanup.rs
│   │   ├── vocabulary.rs
│   │   ├── vocabulary_matching.rs
│   │   ├── vocabulary_migration.rs
│   │   └── vocabulary_post_correction.rs
│   └── examples/
│       ├── foundation.rs
│       ├── text_cleanup_composition.rs
│       ├── text_filler_removal.rs
│       ├── vocabulary_matching_benchmark.rs
│       ├── vocabulary_validation.rs
│       ├── vocabulary_migration_check.rs
│       ├── vocabulary_post_correction.rs
│       └── vocabulary_multi_post_correction.rs
└── settings/
    ├── Cargo.toml
    ├── src/lib.rs
    ├── examples/defaults.rs
    ├── examples/json_roundtrip.rs
    ├── examples/version_compatibility.rs
    ├── examples/load_settings.rs
    ├── examples/initialize_settings.rs
    └── examples/reset_settings.rs
```

The `outflow-core` library provides vocabulary validation, Unicode comparison, migration inspection, whitespace normalization, a partial whitespace-and-vocabulary composition function, single-entry and multi-entry post-correction functions, plus a development-only status function. Run its foundation example from the project root:

```sh
cargo run -p outflow-core --example foundation
```

Expected output:

```text
Outflow Rust foundation ready
```

Run the Rust workspace checks independently of Vite or a browser:

```sh
cargo fmt --all -- --check
cargo check --workspace --locked
cargo build --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

These are development checks only; the examples are not application runtime features. No Tauri windows, automatic settings recovery, microphone capture, or dictation behavior is part of this Rust foundation.

### Native vocabulary post-correction

`outflow-core::vocabulary_post_correction::apply_vocabulary_post_correction` applies one spoken-to-written mapping per call. It reuses the existing field validation and Unicode 18 canonical-caseless comparison policy. Matches must occupy complete words or phrases; unmatched input bytes, punctuation, whitespace, line breaks, capitalization, and Unicode remain as supplied. Invalid empty, whitespace-only, or over-128-scalar mappings return a typed validation error.

The function is deterministic and standalone. It does not access speech recognition, Moonshine, settings, files, the network, or the UI. The multi-entry API below applies a complete validated collection in one original-text pass; keyterm hints and integration into the full cleanup pipeline remain outside this step.

Setup: install the repository's stable Rust toolchain with `rustfmt` and `clippy`, then run the commands below from the repository root. The example needs no microphone, speech model, browser, or service.

Run the synthetic example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_post_correction --locked
```

Expected output:

```text
Input: I visited north PARKADE today.
The café serves 東京 tea 😀.
Output: I visited North Parkade today.
The café serves 東京 tea 😀.
Unrelated Unicode preserved: true
```

The accent in `café` is represented by `e` followed by a combining acute accent; the example keeps it byte-for-byte while correcting only the synthetic `north parkade` phrase.

Run the core tests or all workspace checks from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
```

Troubleshooting:

- **`rustup` cannot find the stable toolchain or a component:** install the stable Rust toolchain with `rustfmt` and `clippy`, then rerun from the repository root.
- **A validation error is returned:** spoken and written phrases must each be nonempty, contain non-whitespace content, and be no longer than 128 Unicode scalar values. The error identifies the field and reason without echoing phrase content.
- **A larger word is left unchanged:** this is intentional boundary protection. The function will not replace `rust` in `crust`, `rusty`, or a phrase adjacent to letters, numbers, combining marks, or common word joiners.
- **Accented text looks unchanged in a terminal:** composed and decomposed accents may render identically. The tests also verify exact Rust string values, including the combining-accent regression.
- **The example output differs:** confirm that the repository's locked dependencies are available and rerun with `--locked`; this example uses fixed synthetic strings and does not need a microphone, model, or network service.

This is a partial development building block only. It does not complete T-035, T-038, T-039, AC-07, AC-08, or any release gate.

### Native Rust multi-entry vocabulary post-correction

`outflow-core::vocabulary_post_correction::apply_vocabulary_post_corrections` validates a collection of zero to 500 `VocabularyEntryRef` mappings before scanning the input. It reuses the existing 128-Unicode-scalar field validation, rejects canonical-caseless duplicate spoken phrases, and compares text with the pinned Unicode 18 D145 policy. The scan moves left to right through the original input. At the same start position the longest matching original-text span wins; remaining ties use vocabulary entry order. Matches cannot overlap, and replacement text is never scanned again during that call. Replacements are copied exactly and all unmatched text remains byte-for-byte unchanged.

Run the synthetic three-mapping example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_multi_post_correction --locked
```

Expected output:

```text
Input:
I visited north PARKADE.
I use rust at ubc.
The café serves 東京 tea 😀.
Output:
I visited North Parkade.
I use Rust at UBC.
The café serves 東京 tea 😀.
Unrelated Unicode preserved: true
Non-cascading input: rust
Non-cascading output: ubc
```

The final two lines use separate synthetic mappings `rust` → `ubc` and `ubc` → `UBC`; the first replacement remains `ubc` because inserted text is not processed again. The accent in `café` is a decomposed `e` plus combining acute accent and is preserved, along with Japanese text and emoji.

Run the workspace verification from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_multi_post_correction --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
```

Troubleshooting and limitations:

- **Collection rejected:** use no more than 500 entries. Each spoken and written value must contain non-whitespace text and fit within 128 Unicode scalar values. Errors identify the failing index and reason without printing vocabulary text.
- **Duplicate spoken phrase rejected:** spoken keys use Unicode 18 canonical-caseless matching, so case-folded and canonically equivalent spellings conflict even when their bytes differ.
- **A phrase did not match:** matching is exact apart from canonical-caseless comparison. It does not trim or collapse spaces; a doubled space in the input does not match a single space in the configured phrase. Complete-word boundaries also reject matches adjacent to letters, numbers, combining marks, and covered joiners/extenders.
- **An overlap resolved unexpectedly:** the leftmost original-text start is handled first, and the longest valid span at that start wins. Later overlapping spans are skipped; replacement text is not searched again.
- **A terminal renders the accent differently:** the example uses a decomposed accent. Tests check the exact string value so visual terminal normalization does not change the evidence.
- **Unicode boundary behavior:** the existing conservative classifier is not a full Unicode word-break implementation. It avoids adding a separate word-break dependency and may treat scripts or uncommon format marks differently from a complete segmentation standard.

This remains a partial T-039 building block. It does not implement Moonshine keyterm hints, the full punctuation/filler cleanup pipeline, settings persistence, or ASR/frontend integration. T-035, T-038, T-039, AC-07, AC-08, and all release gates remain open.

### Protected vocabulary replacement spans

`outflow-core::vocabulary_post_correction::apply_vocabulary_post_corrections_with_spans` returns a `VocabularyCorrectedText` with the corrected `text` and a `protected_ranges` vector. Each `std::ops::Range<usize>` is a UTF-8 byte range into that returned text. It covers exactly one written vocabulary replacement, excluding adjacent transcript punctuation, spaces, and other unmatched input. Ranges appear in output order, do not overlap, and start and end at valid UTF-8 boundaries. The scanner records offsets as each replacement is appended; it does not search the output afterward. No spoken phrase or vocabulary ID is included in the metadata.

The API reuses the existing vocabulary collection validator, Unicode 18 canonical-caseless matching and boundaries, left-to-right non-overlap, and longest-match precedence. Written text is preserved exactly, and inserted text is not scanned again. Empty collections and inputs with no matches return no ranges. The existing text-only single-entry and multi-entry APIs remain available and return the same corrected strings.

Run the synthetic example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_protected_spans --locked
```

Expected output (Rust debug formatting):

```text
Original input: "rust , at ubc ."
Corrected output: "Rust , Edition , at UBC ."
Protected ranges: 2
Protected replacement 1: "Rust , Edition"
Protected replacement 2: "UBC"
All range assertions passed: true
```

The example checks the complete output, exact byte ranges, UTF-8 boundaries, ordering, non-overlap, and the two exact replacement slices. It uses synthetic strings only and exits unsuccessfully if an assertion fails.

Run the focused core tests and workspace verification from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core --locked
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_protected_spans --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
```

Troubleshooting and limitations:

- **No protected range is returned:** only successful vocabulary replacements receive ranges. An empty collection or an unmatched phrase correctly produces an empty list.
- **An offset looks too large for the visible text:** offsets count UTF-8 bytes, not Unicode scalar values or grapheme clusters. Slice the returned `text` with the returned range; do not use character counts as byte offsets.
- **Collection validation fails:** use at most 500 entries with nonempty, non-whitespace spoken and written fields of at most 128 Unicode scalar values. Canonically equivalent spoken phrases are duplicates. Errors do not include vocabulary text.
- **Ranges are used after further text edits:** any edit to the returned string can invalidate its ranges. Apply later formatting with awareness of these protected spans or recompute from the original text.
- **A Unicode boundary differs from expectations:** matching uses the current conservative boundary classifier, not a complete Unicode word-break algorithm.
- **Punctuation remains unchanged:** this API only reports provenance. It does not run punctuation cleanup, capitalization, filler removal, dictation commands, or any later formatting stage.

This is a partial T-035/T-039 building block. It is not connected to Moonshine, settings persistence, the application pipeline, or punctuation-spacing cleanup. T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.

### Native Rust whitespace and vocabulary composition

`outflow-core::text_cleanup::normalize_whitespace_and_apply_vocabulary` composes the existing whitespace normalizer and validated multi-entry vocabulary corrector. It normalizes ASCII spaces, tabs, CRLF, and standalone carriage returns first, then validates the complete vocabulary collection and applies Unicode 18 canonical-caseless matching with the existing word boundaries and longest-match precedence. Paragraph breaks and unrelated text remain intact. Written replacement values are copied exactly, including repeated spaces and line breaks; the output is not normalized again.

This is deliberately a partial pipeline. FR-07.3 places dictation-command recognition between whitespace normalization and vocabulary correction when that stage is implemented. Replacements do not cascade within one call, but the whole function is not universally idempotent across repeated calls. For example, `rust` → `ubc` and `ubc` → `UBC` produce `ubc` on the first call and `UBC` if the pipeline is applied again to that result. Mapping values are not changed to force idempotence.

Setup: install the repository's stable Rust toolchain with `rustfmt` and `clippy`, then run the commands below from the repository root. The example uses only synthetic in-memory text and vocabulary; it needs no microphone, model, browser, or service.

Run the composition example:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_cleanup_composition --locked
```

Expected output:

```text
Input: "  I visited   north PARKADE.\r\n\r\n\tI use rust\t at  ubc.  \r\nThe cafe\u{301} serves 東京 tea 😀."
Output: "I visited North Parkade.\n\nI use Rust at UBC.\nThe cafe\u{301} serves 東京 tea 😀."
Output matches expected: true
Unrelated Unicode preserved: true
```

The debug output displays the decomposed accent in `cafe\u{301}` as a Rust escape. The runnable example compares the exact output string and exits unsuccessfully if either the result or unrelated Unicode preservation differs.

Run the core tests or all requested workspace checks from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
```

Troubleshooting and limitations:

- **Stable toolchain or component missing:** install the stable Rust toolchain with `rustfmt` and `clippy`, then rerun from the repository root.
- **Vocabulary rejected:** the existing validator permits at most 500 entries, limits each field to 128 Unicode scalar values, and rejects empty/whitespace-only values and duplicate spoken phrases under Unicode 18 canonical-caseless matching.
- **A normalized multiword phrase does not match:** transcript ASCII spaces and tabs are collapsed before matching. Configure the spoken phrase with its intended single spaces; vocabulary fields themselves are not rewritten.
- **Non-ASCII spacing remains:** the whitespace stage deliberately changes only ASCII spaces/tabs and line endings. Other Unicode whitespace and characters are preserved.
- **Replacement spacing looks unusual:** written values are inserted exactly, so repeated spaces and embedded line breaks are intentional and are not normalized afterward.
- **A second pipeline call changes a result:** this is expected for maps whose written value matches another spoken phrase. Non-cascading behavior applies within one call only.

The API does not implement punctuation or capitalization correction, dictation commands, Clean/Verbatim filler handling, Moonshine integration, persistence, or React/Tauri wiring. It does not complete T-035 or T-039: their prerequisites and broader production criteria remain unmet. T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.

## Native Rust settings schema v1

The independent `outflow-settings` crate under `crates/settings/` defines local configuration data, JSON serialization, validation, an explicit-path read-only loader and initializer, and an explicit-path atomic writer. It uses `serde = 1.0.229` with derive support and `serde_json = 1.0.154`. It has no Tauri or operating-system integration dependency and performs no network operations. The loader uses standard-library filesystem APIs; the writer is currently implemented on Unix and these APIs do not connect to the React preview.

Schema version **1** is the only version supported by this model. Defaults are:

| Setting | Default |
|---|---|
| Schema version | `1` |
| Language | English (`en`) |
| Push-to-talk and toggle shortcuts | Unconfigured (`None`); no universal shortcut is selected |
| Input device | System default (`None`) |
| Cleanup mode | `Clean` |
| Remove fillers | `true` |
| Run at login | `false` |
| Update checks | User-initiated only (`true`) |
| Vocabulary | Empty |

The JSON schema v1 shape is:

```json
{
  "schema_version": 1,
  "language": "en",
  "recording": {
    "push_to_talk": null,
    "toggle": null,
    "input_device": "system-default"
  },
  "cleanup": {
    "mode": "clean",
    "remove_fillers": true
  },
  "startup": {
    "run_at_login": false
  },
  "updates": {
    "user_initiated_only": true
  },
  "vocabulary": []
}
```

Configured shortcuts are strings; an unconfigured shortcut is explicitly `null`. In the Rust API, `RecordingSettings::input_device == None` means the system default and maps to the JSON string `"system-default"`; a configured device key maps to its string value. Cleanup modes serialize as `"clean"` and `"verbatim"`. Field names are snake case, and Unicode vocabulary strings round-trip unchanged.

`Settings::validate()` rejects unsupported schema versions and languages, automatic update checking, duplicate or empty configured shortcuts, more than 500 vocabulary entries, empty source/replacement phrases, phrases over 128 Unicode scalar values, and duplicate source phrases ignoring case. `Settings::to_json()` validates before encoding. `Settings::from_json()` rejects malformed documents, unsupported schema versions, missing required fields (including omitted nullable shortcut keys), unknown fields, and invalid settings; it reuses `Settings::validate()` for schema, language, update-policy, shortcut, and vocabulary rules. Errors retain parser locations or safe validation details, never raw JSON or vocabulary/shortcut values.

Schema version **1** is the only supported version; unsupported versions are rejected. JSON encoding and decoding are in-memory operations. The separate atomic writer can save validated settings on Unix, and the explicit-path loader can read them; schema migration and frontend/Tauri integration are not implemented. Do not treat the schema codec or file operations as evidence that `T-017`, `T-018`, `AC-11`, or `AC-12` is complete; their broader prerequisites and definitions of done remain outstanding.

Run the non-sensitive defaults example and Rust tests with:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example defaults
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example json_roundtrip
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
```

The example prints:

```text
Outflow settings schema v1 ready
Language: English
Cleanup: Clean
Launch at Login: Off
```

These Rust defaults are available to native code; file loading and writing are available through separate explicit-path APIs below, with no frontend bridge. The React Launch at Login switch remains an independent preview-only boolean held in React memory; changing it does not mutate a Rust `Settings` value or alter operating-system startup behavior.

The JSON round-trip example constructs defaults, serializes and decodes them in memory, validates the result, and checks equality. It prints only:

```text
Outflow settings JSON round-trip passed
Schema version: 1
Validation: passed
```

### Settings schema version compatibility

Schema version **1** is the only supported version. Version **0** is rejected because the project has no historical v0 settings shape; it must not be guessed or converted. Versions greater than 1 are rejected until an explicit, tested migration path exists. Missing, `null`, negative, non-integer, string, malformed, or out-of-range version fields are rejected. The decoder never treats an unknown version as v1 and never substitutes defaults after an error. Errors do not include raw JSON or vocabulary text.

The loader accepts schema v1 only. A version may be migrated only through an explicitly defined and tested path; unknown newer versions must be rejected rather than downgraded. Initialization never overwrites invalid or unsupported settings. The separate confirmed reset below is the deliberate discard path and does not migrate the old document. Any future migration must preserve user settings and vocabulary, or report failure without changing the original configuration. No v0-to-v1 migration is planned without evidence of a real historical v0 schema.

Run the compatibility example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example version_compatibility
```

The example constructs synthetic documents in memory, performs no file or network operations, and prints no configuration values. Expected output:

```text
Outflow settings version policy ready
Schema v1: supported
Schema v0: unsupported
Schema v2: unsupported
Missing version: rejected
```

### Settings file loading

`outflow_settings::load_settings(&path)` reads only the explicit path supplied by its caller and returns `Result<Option<Settings>, SettingsStorageError>`. It does not discover an Outflow configuration directory, create a file, apply settings, migrate versions, or change the source file. `Ok(None)` means the file is genuinely missing; a startup layer may choose `Settings::default()` in that case. Errors never fall back to defaults.

The loader rejects directories, symbolic links, files larger than **1 MiB (1,048,576 bytes)**, invalid UTF-8, malformed JSON, unsupported schema versions, and schema v1 values that fail validation. It reuses `Settings::from_json()` for decoding and validation. Reads are capped at one byte over the maximum so a file that grows after metadata inspection is rejected without being read fully into memory. Some schema v1 string fields have no separate byte-length limit, so unusually large but otherwise well-formed files are also rejected by this loader cap. Filesystem errors identify the safe operation and permission failures without including paths or operating-system error text. JSON errors retain only a line and column; errors do not include raw JSON, vocabulary, or shortcut values.

The loader checks path metadata before opening and checks the opened file and path again. The standard library does not provide a portable no-follow open operation for all supported targets, so a process that can concurrently replace the file may still race these path-based checks on some platforms. This is a documented limitation, not complete filesystem attack resistance. Real filesystem tests have run on **macOS 26.6.2 arm64 only**. Linux and Windows have not been tested. The existing atomic writer remains Unix-only; Windows writing still fails closed pending an approved owner-only ACL implementation.

Run the example from the repository root. It creates synthetic files in a temporary directory, prints no configuration values, and removes its fixtures on exit:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example load_settings
```

Expected output:

```text
Outflow settings load ready
Valid v1: loaded
Missing file: defaults available
Invalid file: rejected
```

Run loader and atomic-writer regression tests with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
```

### Safe settings initialization

`outflow_settings::initialize_settings(&path)` uses the explicit-path loader and returns one of two results:

- `SettingsInitialization::Loaded(settings)` when an existing schema v1 file is valid. The file is left unchanged.
- `SettingsInitialization::FirstRun(settings)` when the file is missing. The compiled-in defaults are validated and returned in memory; no file is created.

Malformed JSON, unsupported schema versions, invalid values, invalid UTF-8, oversized files, unsafe paths, permission denials, and other filesystem failures return the existing structured `SettingsStorageError`. Initialization never turns a loading error into defaults and never repairs, deletes, renames, backs up, or overwrites a file. Error text contains safe error categories and parser locations, not raw JSON or vocabulary/shortcut values.

Initialization does not save defaults automatically so first-run does not silently create persistent settings, and a damaged or newer configuration cannot be silently replaced. Invalid input remains available for deliberate recovery with the separately confirmed reset described below. Backup restore, schema migration, configuration-directory discovery, a Tauri command, and frontend synchronization are not implemented.

Run the synthetic temporary-directory example and the Rust tests from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example initialize_settings
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
```

The example removes its temporary directory on exit and prints:

```text
Outflow settings initialization ready
Missing configuration: defaults initialized
Valid configuration: loaded
Invalid configuration: recovery required
Original invalid file: preserved
```

The verified run on macOS 26.6.2 arm64 passed all **63 Rust workspace tests** (61 settings and 2 core). The existing **16 frontend tests** also passed; no frontend source changed.

Troubleshooting:

- **First run returns defaults:** this is expected when the explicit path is missing; defaults remain in memory until an approved caller saves them.
- **Malformed JSON, unsupported version, or invalid settings:** initialization stops with a redacted error and leaves the original file unchanged. To replace it, use the separate reset only after explicit confirmation; no automatic repair or migration occurs.
- **Invalid UTF-8 or file too large:** the loader rejects the file before decoding and does not truncate or replace it.
- **Permission or filesystem error:** initialization returns the loader's structured failure instead of defaults. Check that the supplied path is a readable regular file and that the parent is accessible. Error messages omit paths and operating-system error text.

These behaviors were exercised on macOS 26.6.2 arm64 only. Linux and Windows were not tested in this task; the existing writer remains Unix-only.

### Explicit settings reset

`outflow_settings::reset_settings_to_defaults(path, confirmed)` replaces an existing settings file with the compiled, validated schema v1 defaults only when `confirmed` is `true`. Confirmation is required because the operation discards local preferences and custom vocabulary. The boolean records caller intent; it is not a security boundary or a substitute for a future native confirmation prompt. No Reset button, dialog, Tauri command, or frontend connection exists yet.

Reset clears the custom vocabulary and returns settings to their defaults: English, unconfigured shortcuts, the system-default input device, Clean mode with filler removal, Launch at Login off, and user-initiated update checks. The reset checks for an existing regular file and rejects symbolic links. It does not read or parse the old contents, so the same explicit operation can replace malformed JSON, invalid settings, or an unsupported schema version. A missing file returns `DestinationMissing`; reset never creates a first-run file. `initialize_settings()` remains read-only and never calls reset, so invalid or unreadable settings continue to produce errors with the source file unchanged until reset is explicitly invoked.

On Unix, reset uses the existing same-directory temporary file and atomic rename path. The replacement file is owner-only (`0600`), and the original is not truncated or deleted before rename. Any failure before replacement preserves the original file. If rename succeeds but parent-directory synchronization fails, reset returns `ReplacementDurabilityUncertain`: the new defaults are visible, but crash durability is not confirmed. Callers must not automatically retry this uncertain outcome. No backup is created. Filesystem snapshots and operating-system backups may retain older settings; reset is not secure erasure.

The reset is currently available only where the atomic writer is implemented: Unix. Windows and other non-Unix targets fail closed with `UnsupportedPlatform` until owner-only ACL and atomic replacement guarantees are implemented. Filesystem tests in this task ran on macOS 26.6.2 arm64; Linux behavior has not been tested.

The operation modifies only the caller-supplied settings path, using a transient temporary file in that path's directory; it does not discover the user configuration directory or modify sibling files. A second confirmed call currently succeeds and writes the same defaults again; the API does not separately promise cross-platform idempotence. The Task 64 isolated regression checks persisted vocabulary removal, load/initialization preservation, repeat behavior, and a byte-for-byte unchanged sibling sentinel:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  explicit_settings_reset_is_scoped_and_separate_from_loading \
  --locked -- --nocapture
```

Expected output reports defaults persisted as schema v1, previous vocabulary absent from persisted settings, the sentinel preserved, no unexpected temporary/backup/transcript/audio files, and repeated reset success. The same test confirms malformed and schema-v2 files survive ordinary loading and initialization; a separate explicit reset of the synthetic v2 fixture writes defaults and is not a schema migration. Passing `confirmed=true` in a storage call is only caller intent: this repository has no production reset caller, Tauri command, or user confirmation UI, and this storage test does not establish human confirmation. User-reviewed recovery for corrupted or future-version files, persistent schema migration, backup restoration, Windows support, Linux filesystem behavior, and crash recovery remain separate and unimplemented or unverified. See the [Task 64 evidence](evidence/rust-settings-explicit-reset-safety-task-64.md) for inspected semantics and exact results.

### Confirmed reset failure before replacement (Task 65)

A confirmed reset that fails before replacement must leave the existing preferences and vocabulary available: the prepared defaults have not been committed yet. The Task 65 test reuses the atomic writer's private pre-replacement hook after it has prepared and synchronized a defaults file. It compares the original settings bytes exactly, reloads the original ordered vocabulary and non-default preferences, checks permissions and temporary-file cleanup, and confirms a sibling sentinel is unchanged. A new, separate public confirmed-reset call then persists schema-v1 defaults and clears the vocabulary.

Run the focused test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  confirmed_reset_pre_replacement_failure_preserves_settings_and_allows_retry \
  --locked -- --nocapture
```

The failure is injected in-process; this does not reproduce an OS-level disk or rename error. Automatic retry is prohibited because another failure class, `ReplacementDurabilityUncertain`, means defaults may already be visible after an error. Task 65 does not establish that every reset error preserves the original file, add rollback, or verify crash durability. It was tested on macOS 26.6.2 with an APFS temporary filesystem; Windows and Linux remain unverified. The boolean is caller intent only: no human confirmation UI or accessibility flow exists. See the [Task 65 evidence](evidence/rust-settings-reset-failure-recovery-task-65.md).

### Confirmed reset durability uncertainty after replacement (Task 66)

Task 65 injects failure before replacement, so the original settings remain visible. Task 66 injects a parent-directory synchronization error after the reset's real rename succeeds. The reset returns `ReplacementDurabilityUncertain` while the schema-v1 defaults are already visible and reloadable; the original vocabulary and non-default preferences must not be assumed to survive. This error does not mean rollback completed, and it does not prove crash durability. No automatic retry or rollback occurs.

Run the reset-level regression test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  confirmed_reset_directory_sync_failure \
  --locked -- --nocapture
```

Expected output after assertions pass:

```text
Outflow confirmed reset durability uncertainty test

Initial settings write: PASS
Original vocabulary persisted: VERIFIED
Confirmed reset requested: VERIFIED
Temporary replacement synchronized: VERIFIED
Real destination replacement: COMPLETED
Parent-directory sync callback: VERIFIED
Directory synchronization error: INJECTED
Reset returned ReplacementDurabilityUncertain: PASS

Default settings visible after error: VERIFIED
Original vocabulary no longer visible: VERIFIED
Non-default preferences reset: VERIFIED
Settings reload after error: PASS
File permissions: VERIFIED
Unrelated sentinel: PRESERVED
Unexpected temporary artifacts: NONE

Subsequent public write: PASS
Third settings value reload: PASS

Automatic retry: NO
Automatic rollback: NO
Crash durability verified: NO
Actual OS-level sync failure tested: NO
Human confirmation UI tested: NO
Real user settings accessed: NO
Schema migration performed: NO
```

This callback test ran on macOS/APFS only; it does not reproduce an OS-level sync error or power loss. No human confirmation UI, crash recovery, Windows behavior, or Linux filesystem behavior was tested. See [Task 66 evidence](evidence/rust-settings-reset-durability-uncertain-task-66.md) and the [settings recovery matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix). T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete.

### Task 67 caller guidance after settings persistence errors

A writer error describes the operation or failed phase; a later `load_settings` call separately describes the valid configuration currently visible. `ReplacementFailed` alone does not establish whether the original or attempted settings remain visible. `ReplacementDurabilityUncertain` means replacement may already be visible while crash durability remains unconfirmed. Even when a reload shows the replacement, that observation does not prove power-loss durability.

Future callers should reload through the read-only loader and report the observed category without substituting defaults on missing or unreadable files. Those cases need recovery review. A confirmed-reset error must not trigger another reset, assume the original vocabulary survived, or claim that `confirmed=true` represents a human confirmation interface. No automatic retry, rollback, reset, deletion, migration, or repair is allowed. After presenting the observation, any later change must be a separate deliberate operation.

The proposed guidance helper exists only in the integration-test target; it is not wired to a production settings service, Tauri command, or UI. Run its contract tests with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test settings_persistence_guidance_contract \
  --locked -- --nocapture
```

The tests use synthetic settings in isolated temporary directories and verify that the loader observation does not write them. No production caller, human confirmation UI, automatic retry or rollback, schema migration, or persistent recovery metadata was added. No OS-level rename or synchronization failure was reproduced; crash durability remains unverified, as do Windows and Linux filesystem behavior. See [Task 67 evidence](evidence/rust-settings-persistence-guidance-task-67.md) and the [settings recovery matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix). T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete.

### Task 68 read-only settings inspection after persistence errors

Task 68 adds a test-only inspector for the configuration visible through the public `load_settings` reader after a hypothetical write or reset error. It reports `OriginalVisible`, `AttemptedVisible`, `BothSnapshotsMatch`, `OtherValidSettingsVisible`, `Missing`, `InvalidDocument`, `UnsupportedSchema`, or `ReadFailure`. A valid third configuration is still valid settings, not corruption. If the supplied original and attempted snapshots are equal and both match, the result is `BothSnapshotsMatch`: the inspection cannot identify which write, if any, produced the visible state.

The inspector uses `load_settings` instead of `initialize_settings` because a missing file must remain missing rather than be replaced with in-memory defaults. Malformed or unsupported files are reported without reset, repair, retry, backup, or rollback. The classification is a snapshot of what the loader sees at that moment; another operation can change the file later, and visibility does not establish crash durability.

Proposed future workflow:

```text
Settings write or reset returns an error
                  |
                  v
Optional, read-only settings inspection
                  |
                  v
Visible-state classification
                  |
                  v
Future caller guidance (Task 67 contract)
```

Run the focused contract with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test settings_post_error_inspection \
  --locked -- --nocapture
```

The helper exists only in this integration-test target. It is not connected to a production settings service, Tauri command, UI, or automatic recovery workflow. Task 67's caller guidance remains a separate test-only contract. See the [Task 68 evidence](evidence/rust-settings-post-error-inspection-task-68.md) and [settings recovery matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix). T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete.

### Task 69 settings snapshot boundaries

Task 69 extends the test-only inspector introduced in Task 68 and changes no production settings API. Each inspection is a snapshot from one call to the public settings loader. A later explicit write does not update an earlier classification; a fresh inspection is needed to observe the newly visible configuration. Equal original and attempted values produce `BothSnapshotsMatch` whether no write occurred between observations or the same values were explicitly written again, so the result cannot establish operation history, reset, durable commit, or resolution of an earlier error. The A → B → A test observes `OriginalVisible`, then `AttemptedVisible`, then `OriginalVisible`; the final classification does not reveal the intermediate B state.

The two-actor test uses channels to order a completed inspection of A, an explicit public write of B, and a fresh inspection of B. It demonstrates that separate actors can observe different snapshots in that schedule. It does not overlap filesystem operations or prove linearizability, locking, transaction isolation, lost-update prevention, or crash durability. Inspection remains read-only and cannot replace filesystem locking or transactional guarantees.

Run the focused tests and assertion-backed summary with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test settings_post_error_inspection \
  --locked -- --nocapture
```

All fixtures are synthetic and isolated in temporary directories. See the [Task 69 evidence](evidence/rust-settings-snapshot-boundaries-task-69.md) and [settings recovery matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix).

### Task 70 settings lost-update audit

Atomic replacement and lost-update prevention are separate properties. The existing `save_settings_atomic(path, &settings)` writer atomically replaces the file with the complete validated snapshot it receives; it does not compare that snapshot with the current file, require an expected revision, merge fields, serialize settings updates, or report stale-write conflicts. The stored `schema_version` identifies the document format, not a concurrency revision. In the deterministic two-actor test, both actors load A, then change separate ordinary preferences. With actor 1 writing first, actor 2's stale snapshot succeeds and replaces actor 1's field; reversing the write order reverses which change is lost. The file remains valid schema v1 with no partial JSON or temporary artifacts. This is a lost logical update, not file corruption.

A fresh read before actor 2's update preserves both changes in the controlled sequence because that snapshot includes actor 1's write. It is not transaction isolation: another writer could still change the file between that read and the next write. No production locking, compare-and-swap, versioning, or automatic merge is present. The tests use ordered operations and do not exercise overlapping writes, cross-process behavior, or Windows/Linux filesystem semantics. Run the focused audit with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test settings_lost_update_audit \
  --locked -- --nocapture
```

See the [Task 70 evidence](evidence/rust-settings-lost-update-audit-task-70.md) and [settings recovery matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix).

### Task 71 test-only stale-snapshot contract

The test-only assessment compares the caller's expected snapshot with a successfully loaded current `Settings` value. It compares the complete object, including every preference and the vocabulary entries in order, because a selected field or vocabulary count would miss other persisted changes. `schema_version` identifies the document format and is not a concurrency revision. Equal values match only at inspection time; another writer can change the file immediately afterward, so this is not an atomic write precondition or compare-and-swap. A → B → A also compares equal and cannot reveal that intervening history.

The production writer is unchanged and still accepts complete snapshots with last-writer-wins behavior. Task 71 adds only a test contract for classifying a different observed value as stale; it does not prevent lost updates in production, lock files, merge, retry, or roll back settings. Run the focused contract and assertion-backed summary with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test settings_stale_update_contract \
  --locked -- --nocapture
```

See the [Task 71 evidence](evidence/rust-settings-stale-update-contract-task-71.md) and [settings recovery matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix).

Run the example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example reset_settings
```

It uses synthetic settings in a temporary directory, performs no network operations, prints no settings values or absolute paths, and removes its temporary files on exit. Expected output:

```text
Outflow settings reset ready
Unconfirmed reset: cancelled
Confirmed reset: defaults restored
Invalid configuration: recovered
Schema version: 1
```

Run the automated Rust and frontend checks with:

```sh
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
```

Troubleshooting:

- **Reset says it was not confirmed:** the file is untouched. Obtain explicit user confirmation in the calling UI before passing `true`.
- **Reset reports a missing or unsafe destination:** it only accepts an existing regular file, not a missing path, directory, or symbolic link. Create or select the intended settings file through the caller; reset itself does not create one.
- **Initialization rejects malformed JSON or an unsupported version:** that behavior is intentional and leaves the file untouched. To recover, explicitly confirm a reset; reset does not need the old document to parse.
- **Reset returns `ReplacementFailed`:** the destination-replacement operation returned an error. This variant alone does not establish the destination state for every filesystem failure; do not assume rollback. Resolve the filesystem issue before a later user-directed action.
- **Reset returns `ReplacementDurabilityUncertain`:** rename already happened and defaults are visible. Do not automatically retry; inspect the resulting file or let the user decide what to do next.
- **Reset returns `UnsupportedPlatform`:** the current atomic writer has no owner-only implementation on that platform. It does not fall back to a less restrictive write.
- **Old data remains in a system backup or snapshot:** reset replaces Outflow's settings file, but it does not securely erase snapshots or OS-managed backups.

### Atomic settings file writing

`outflow-settings::save_settings_atomic(path, &settings)` validates the value, encodes it with the existing schema v1 JSON codec, and writes to a unique temporary file in the supplied destination directory. On Unix it creates that file with owner-only mode `0600`, writes and flushes the complete document, synchronizes the file, then atomically renames it over the destination and synchronizes the parent directory. It rejects symbolic-link and non-regular destinations. A failure before rename leaves an existing destination intact; the writer never deletes the old file first. If the rename succeeds but parent-directory synchronization fails, it returns `ReplacementDurabilityUncertain` because the new file is visible but crash durability could not be confirmed.

Windows is currently unsupported by this writer. The repository has no approved Windows ACL dependency or implementation, and the API returns an explicit unsupported-platform error instead of writing with inherited permissions. The local filesystem behavior was exercised on macOS arm64; Windows and Linux were not tested. POSIX mode `0600` was verified for the resulting file, but ACL behavior on arbitrary custom parent directories has not been audited.

The writer takes an explicit path and does not discover a configuration directory. It does not load, recover, migrate, watch, back up, or connect settings to Tauri or the frontend. The example below uses only synthetic defaults in a temporary test directory and removes that directory on exit:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example atomic_write
```

Expected output:

```text
Outflow atomic settings write passed
Settings validation: passed
JSON file: verified
```

Run the workspace checks from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
```

The storage tests cover schema v1 output and decoding, replacement, invalid and unsupported values, permission denial, simulated partial writes and file-sync failure, pre-replacement interruption, post-replacement durability uncertainty, temporary-file cleanup, missing and invalid parents, symbolic links, restrictive Unix mode, and redacted errors. They do not claim Windows ACL or Linux filesystem verification.

### Task 55 pre-replacement failure recovery check

The focused recovery test uses a private, deterministic hook after the replacement document has been written, flushed, and synchronized, and after the final destination and parent checks, immediately before `fs::rename`. It starts with valid schema-v1 settings created by `save_settings_atomic`, captures the exact on-disk bytes, injects `ReplacementFailed`, and verifies the original path, bytes, vocabulary, and successful reload remain intact. It also verifies the prepared temporary file is removed and a subsequent normal write can save and reload the replacement settings.

Run the test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  atomic_writer_pre_replacement_failure_preserves_valid_settings_and_allows_retry \
  --locked -- --nocapture
```

On the tested macOS host, the expected summary is:

```text
Outflow atomic settings writer recovery test

Initial settings write: PASS
Original settings reload: PASS
Pre-replacement failure: INJECTED
Write rejected: PASS
Original bytes preserved: PASS
Original settings reload after failure: PASS
Subsequent normal write: PASS

Schema migration performed: NO
Real user settings accessed: NO
Power-loss recovery verified: NO
Windows ACL verification: NOT RUN
Linux filesystem verification: NOT RUN
```

This proves one pre-replacement failure path on the local Unix filesystem. It does not prove power-loss or crash durability, actual disk-full recovery, actual temporary-directory creation or OS-level rename failures, Windows ACL/replacement behavior, or Linux filesystem behavior. Tasks 57 and 58 separately inject failures at temporary-file creation and synchronization callbacks; they do not reproduce OS-level failures at those operations. Task 59 separately injects a replacement-operation error without calling `fs::rename`. The writer synchronizes the parent directory after a successful rename, but this test does not simulate a crash around that metadata update. Task 54 cancellation is an in-memory review-draft operation; it is separate from this test's real filesystem preservation check and does not establish persistent migration rollback. T-018, T-017, T-038, T-039, T-053, and AC-08 remain incomplete; no schema migration is implemented. See the [Task 55 audit evidence](evidence/rust-settings-writer-recovery-task-55.md) for the coverage matrix and full verification record.

### Task 56 temporary-write failure cleanup check

Task 56 injects an I/O error from the existing private write callback after a nonempty prefix has been written to the newly created temporary replacement. The test reads back that prefix, verifies the pre-replacement hook was not reached, compares the directory entries and original destination bytes, reloads the original settings, and then confirms a normal write and reload succeed. This is a separate stage from Task 55, which injects after the temporary document is complete and synchronized, immediately before rename.

Run the focused test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings temporary_write --locked -- --nocapture
```

The test uses only synthetic schema-v1 settings in a unique temporary directory. Its injected error verifies cleanup and retry handling; it does not reproduce actual disk exhaustion, OS-level temporary-file creation or synchronization failures, an OS-level destination rename failure, or process-crash recovery. Task 59 injects separately at the replacement-operation callback. Current filesystem evidence is macOS/APFS only. Windows ACL and replacement behavior, and Linux filesystem behavior, remain unverified. See the [Task 56 evidence and recovery coverage extension](evidence/rust-settings-temporary-write-recovery-task-56.md) for the scenario matrix and complete verification record.

### Task 57 temporary-file creation failure recovery check

Task 57 injects a non-collision I/O error at the private callback used exactly where the writer normally calls `OpenOptions::create_new(true)` for its generated same-directory temporary path. The test proves the creation operation is reached once, then verifies the error is classified as `TemporaryFileCreationFailed`, no write or replacement stage is reached, the existing schema-v1 bytes and vocabulary still reload unchanged, and no temporary artifact was created. It then retries through the public writer and reloads the different valid settings. This covers the creation stage; Task 56 fails after a temporary file contains a partial write, and Task 55 fails after the file is fully prepared immediately before replacement.

Run the focused test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  temporary_creation \
  --locked -- --nocapture
```

Expected output after all assertions pass:

```text
Outflow atomic settings temporary-creation failure test

Initial settings write: PASS
Original settings reload: PASS
Temporary-file creation failure: INJECTED
Writer returned error: PASS
Replacement attempted: NO
Original destination bytes: PRESERVED
Original settings reload after failure: PASS
Unexpected temporary artifacts: NONE
Subsequent normal write: PASS
New settings reload: PASS

Schema migration performed: NO
Real user settings accessed: NO
Actual OS permission failure tested: NO
Actual disk-full failure tested: NO
```

The test uses only synthetic settings in an isolated temporary directory. It does not simulate actual temporary-directory permission denial or disk-full conditions. Actual OS-level synchronization or destination rename failures, crash or power-loss recovery, Windows ACL/replacement behavior, and Linux filesystem behavior remain unverified. See the [Task 57 evidence](evidence/rust-settings-temp-creation-recovery-task-57.md) and [writer recovery matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix). T-018, T-017, T-038, T-039, T-053, and AC-08 remain incomplete.

### Task 58 temporary-file synchronization failure recovery check

Task 58 injects a deterministic error through the existing private synchronization callback. The callback is the writer's temporary-file `sync_all` operation, after the complete JSON write and explicit flush and before destination replacement. It reads back and decodes the entire temporary file as schema-v1 settings before returning the error. The test then checks the typed synchronization error, that the replacement hook was not reached, byte-for-byte preservation and successful reload of the original settings, unchanged file permissions, and cleanup of the temporary path and any other unexpected directory entries. A subsequent write through the public writer must save and reload the replacement successfully.

This covers the synchronization stage; Task 57 injects at temporary-file creation, Task 56 injects after a verified partial write, and Task 55 injects immediately before replacement after synchronization succeeds. Run the focused Task 58 test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  atomic_writer_temporary_sync_failure_preserves_valid_settings_and_allows_retry \
  --locked -- --nocapture
```

Expected output after the assertions pass:

```text
Outflow atomic settings synchronization failure test

Initial settings write: PASS
Temporary replacement creation: PASS
Complete temporary write: VERIFIED
Temporary-file synchronization failure: INJECTED
Writer returned error: PASS
Destination replacement attempted: NO
Original destination bytes: PRESERVED
Original settings reload: PASS
Temporary artifact cleanup: PASS
Subsequent normal write: PASS
New settings reload: PASS

Schema migration performed: NO
Real user settings accessed: NO
Actual OS synchronization failure tested: NO
Crash durability verified: NO
```

The injected callback error is not an actual operating-system synchronization failure and does not establish crash or power-loss durability. Parent-directory synchronization occurs after replacement and remains a separate boundary. Task 59 covers an injected replacement-operation error but does not test an actual OS-level rename failure. Windows ACL/replacement and Linux filesystem behavior also remain unverified. See the [Task 58 evidence](evidence/rust-settings-sync-failure-recovery-task-58.md) and the [writer recovery coverage matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix). T-018, T-017, T-038, T-039, T-053, and AC-08 remain incomplete.

### Task 59 destination replacement-operation failure recovery check

Task 59 injects an I/O error from a private callback at the writer's destination replacement operation. This differs from Task 55, whose hook fails just before that operation. The Task 59 callback receives the generated temporary path and the existing destination path; it verifies the complete serialized replacement is present and decodes as schema v1 after the writer's real temporary-file `sync_all` returned successfully. Production still routes the callback to `fs::rename`.

The test verifies `ReplacementFailed`, exact preservation and reload of the original settings, unchanged Unix permissions, cleanup of the failed temporary file, no parent-directory sync after the failed replacement, and a successful retry through the public writer. It uses only synthetic settings in an isolated temporary directory.

Run the focused test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  atomic_writer_replacement_failure_preserves_valid_settings_and_allows_retry \
  --locked -- --nocapture
```

Expected output after the assertions pass:

```text
Outflow atomic settings replacement failure test

Initial settings write: PASS
Original settings reload: PASS
Temporary replacement fully written: VERIFIED
Temporary-file synchronization: COMPLETED
Replacement callback invoked: VERIFIED
Replacement-operation error: INJECTED
Writer returned replacement error: PASS
Original destination bytes: PRESERVED
Original settings reload after failure: PASS
Original permissions: PRESERVED
Temporary artifact cleanup: PASS
Subsequent public write: PASS
New settings reload: PASS

Schema migration performed: NO
Real user settings accessed: NO
Actual OS rename failure tested: NO
Post-replacement rollback verified: NO
Crash durability verified: NO
```

The injected error substitutes for the `fs::rename` syscall, so this test verifies writer handling of a replacement-operation error; it does not test an actual OS rename failure, ambiguous replacement, platform-specific atomicity, post-replacement rollback, or crash durability. Evidence currently covers the local macOS/APFS filesystem only. Windows ACL/replacement and Linux filesystem behavior remain untested. See the [Task 59 evidence](evidence/rust-settings-replacement-failure-recovery-task-59.md) and the [writer recovery coverage matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix). T-018, T-017, T-038, T-039, T-053, and AC-08 remain incomplete.

### Task 60 parent-directory synchronization failure after replacement

Task 60 reaches the parent's directory synchronization callback only after the writer's real `fs::rename` succeeds. The callback verifies that the destination contains the complete replacement bytes, decodes them as valid schema-v1 settings, and confirms the temporary path has been consumed before injecting a directory-sync error. The test establishes that the new settings are already visible. The returned `ReplacementDurabilityUncertain` error reports that directory durability could not be confirmed; it does not mean the previous settings survived or that a rollback occurred. This differs from Task 58, where temporary-file synchronization fails before rename and the old destination remains in place.

Run the focused test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  parent_directory_sync \
  --locked -- --nocapture
```

Expected output after the assertions pass:

```text
Outflow atomic settings parent-directory sync failure test

Initial settings write: PASS
Original settings reload: PASS
Temporary replacement fully written: VERIFIED
Temporary-file synchronization: COMPLETED
Destination replacement: COMPLETED
Parent-directory sync callback invoked: VERIFIED
Parent-directory sync error: INJECTED
Writer returned directory sync error: PASS
New destination bytes visible: VERIFIED
Replacement settings reload: PASS
File permissions: VERIFIED
Unexpected temporary artifacts: NONE
Subsequent public write: PASS
Third settings value reload: PASS

Old configuration rolled back: NO
Post-replacement visibility: VERIFIED
Crash durability: NOT VERIFIED
Actual OS-level directory sync failure: NOT TESTED
Real user settings accessed: NO
Schema migration performed: NO
```

This proves the observed post-error destination state and a subsequent normal write on the tested filesystem. The injected callback does not reproduce an OS-level directory-sync error, establish crash or power-loss durability, or verify rollback. Windows ACL/replacement and Linux filesystem behavior remain untested. See the [Task 60 evidence](evidence/rust-settings-directory-sync-recovery-task-60.md) and the [writer recovery coverage matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix). T-018, T-017, T-038, T-039, T-053, and AC-08 remain incomplete.

### Task 61 caller-visible settings write error contract

Both replacement-operation errors and post-replacement directory-sync errors return `Err`, but their typed variants and tested destination states differ. In the Task 59 injected replacement callback, rename is deliberately not performed, so `ReplacementFailed` accompanies the original bytes and settings remaining reloadable. In the Task 60 directory-sync callback, the real rename has already made the replacement visible; `ReplacementDurabilityUncertain` reports that durable persistence could not be confirmed. A generic `Err` therefore does not mean the destination is unchanged, and visible replacement contents do not prove crash durability.

Future callers should report a verified pre-replacement failure as a failed save and may allow a deliberate retry. For `ReplacementDurabilityUncertain`, callers must not claim success, claim restoration of the old settings, or automatically overwrite the destination with stale settings; they should reload the visible configuration before further actions when appropriate. No settings-update command or higher-level production service currently exists in this checkout, so these requirements are documented at the writer boundary. The error's `Display` and `Debug` forms retain no settings JSON or vocabulary values.

Run the focused contract test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  settings_write_error_outcome_contract \
  --locked -- --nocapture
```

The test uses synthetic schema-v1 settings and the existing private fault hooks. Its two observed outcomes apply only to those injected scenarios; actual OS rename/synchronization errors, crash or power-loss durability, Windows replacement/ACL behavior, and Linux filesystem behavior remain untested. No automatic rollback is implemented, and T-018 remains incomplete. See the [Task 61 evidence](evidence/rust-settings-write-error-contract-task-61.md) and [recovery coverage matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#recovery-coverage-matrix).

### Task 62 corrupted-settings preservation check

The public loader distinguishes a missing path from an existing file it cannot decode. A missing file returns `Ok(None)`; initialization offers validated defaults in memory without creating a file. Truncated, malformed, empty, or structurally invalid existing JSON returns the redacted `InvalidDocument` error, while a valid document with an unsupported schema version remains `InvalidSettings(UnsupportedSchemaVersion { .. })`. Rejected files are left byte-for-byte intact because they may contain recoverable preferences or vocabulary; repeated reads must not turn an error into a reset or write.

Run the focused synthetic test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  corrupted_settings \
  --locked -- --nocapture
```

The test verifies the schema-v1 baseline, all four corruption classes, missing-file behavior, unchanged bytes and Unix permissions, stable file identity, unchanged directory entries, and repeated non-destructive failures. It uses temporary test directories and synthetic values only. Corruption is introduced by direct test setup; the loader does not modify the file. No backup restoration, automatic repair, schema-v2 loading, or recovery UI exists. A future manual recovery or reset flow needs separate design and explicit user confirmation. Windows and Linux filesystem behavior, cross-platform permissions, and crash recovery remain unverified. See the [Task 62 evidence](evidence/rust-settings-corruption-preservation-task-62.md); this read-side test is separate from the write-failure injection matrix below.

### Task 63 unsupported-settings initialization preservation check

Initialization returns validated defaults in memory when the settings file is missing, loads an existing supported schema-v1 file without rewriting it, and returns `InvalidSettings(UnsupportedSchemaVersion { .. })` for an unsupported version. The decoder checks the version header before requiring schema-v1 fields, so a newer document with a different shape is still classified as unsupported. Loading and initialization do not read schema v2, downgrade, migrate, reset, or rewrite an unsupported file because an older version cannot safely interpret or discard unfamiliar settings. The separate explicit reset API can replace a selected unsupported file with defaults after the caller passes confirmation; Task 64 tests that destructive behavior only on a synthetic fixture.

Run the focused synthetic regression test with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  settings_initialization_preserves_unsupported_schema_files \
  --locked -- --nocapture
```

Expected output reports PASS for missing-file defaults without a write, unchanged supported schema-v1 settings, and typed rejection plus byte preservation for schema v2 and a minimal future schema version. It then reports `Automatic downgrade: NO`, `Automatic reset: NO`, `Schema migration: NO`, and `Real user settings accessed: NO`. The unsupported fixtures are retried and checked for exact bytes, Unix permissions, file identity, modification time, and unchanged directory entries.

This verifies only the storage and initialization boundary. In-memory defaults for a missing file are already supported; invalid or unsupported existing files remain preserved and return errors. An error is not a user-facing recovery flow. A future recovery interface must let users review safe options without silently losing unsupported settings; any downgrade or migration must be explicit and separately verified. FR-13.4 and T-017 remain incomplete. Task 62 separately covers malformed and corrupted JSON; see the [Task 63 evidence](evidence/rust-settings-initialization-preservation-task-63.md) and the [settings recovery matrix](evidence/rust-settings-temporary-write-recovery-task-56.md#separate-load-side-corruption-coverage).

Run all requested local verification commands from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
git diff --check
```

## Installation

From the project root, install the dependencies:

```sh
npm install
```

For a clean, reproducible install from the npm lockfile, use:

```sh
npm ci
```

## Development server

Start the development server on the local loopback interface:

```sh
npm run dev -- --host 127.0.0.1
```

Open [http://localhost:5173](http://localhost:5173). Vite prints the exact local URL when the server is ready.

If port 5173 is already in use, Vite selects another port unless `--strictPort` is set. Open the URL printed in the terminal.

## Foundation screen and Settings preview

The foundation screen displays:

- **Outflow** as the application name
- **Foundation ready** as the page heading
- **Your application is set up and ready for development.** as supporting text
- A **Settings** button that opens the development preview

In Settings, use the six section tabs—General, Microphone, Shortcuts, Text & Vocabulary, Privacy, and About & Diagnostics—to change the displayed section. General is selected each time the preview opens. Use **Back** to return to the foundation screen. The tabs support Up/Down arrow keys, Home, and End, with visible keyboard focus.

General contains **Launch at Login**, which defaults to **Off**. Use the switch with a mouse or focus it and press Space. The visible On/Off state and accessible switch state update immediately. The preference is kept only in React memory: it remains selected while changing sections or closing and reopening Settings in the same mounted session, and resets to Off after a full page reload. Text & Vocabulary includes a frontend-only vocabulary draft form, a development preview list, and a live synthetic example. Privacy shows a static overview of intended protections and preview limitations; Microphone and Shortcuts show their existing preview status messages, and About & Diagnostics remains a placeholder.

### Vocabulary list preview (development only)

From the repository root, install the locked frontend dependencies and start Vite:

```sh
npm ci
npm run dev
```

Open the local URL printed by Vite, then go to **Settings → Text & Vocabulary**. Enter a spoken phrase and its written replacement and select **Add to preview list**. Both fields must pass the existing validation (nonempty, valid Unicode, and at most 128 Unicode scalar values). Each accepted pair appears in the list below the form; the draft clears after adding. Select that row's **Remove** button to remove it. The accessible count shows the current total out of 500; when full, remove an entry before adding another. An accessible empty message appears when the list has no entries.

To edit an entry, select its **Edit** button. The Spoken phrase and Written replacement fields open with the saved preview values. Change either field and select **Save changes** to update that entry in place; its order and stable identity are preserved. Select **Cancel editing** to discard the unsaved changes and return focus to the row's Edit button. Only one row can be edited at a time. Empty, whitespace-only, malformed Unicode, and values over 128 Unicode scalar values show the existing inline validation and cannot be saved. Unicode and emoji that pass validation are kept as entered.

Entries and saved edits remain in React memory while moving between Settings sections or Home and Settings. A full page reload or fresh application mount clears them. This development preview does not persist entries, connect to native settings, check duplicate spoken phrases, or apply replacements to actual speech. The illustrative sentence remains synthetic.

Run the frontend verification commands from the repository root:

```sh
npm run typecheck
npm run test -- --run
npm run lint
npm run build
git diff --check
```

Expected result: each command exits successfully; the test command reports the frontend test files and passing test count, and the build prints the generated asset sizes. For the manual browser check, run `npm run dev`, open the URL Vite prints, add a pair, edit and save it, then edit again and cancel. The updated pair should remain in its original row; cancellation should restore the saved values and focus should return to Edit. Check **Remove**, the 500-entry count, and Settings/Home navigation, then refresh to confirm the list resets.

Troubleshooting:

- **Save leaves the editor open:** correct both fields; empty, whitespace-only, malformed Unicode, and values longer than 128 Unicode scalar values are rejected with inline errors.
- **Edits disappear after navigation:** keep preview entries in `App.tsx` state above the conditional Home/Settings views; entry editor drafts are intentionally discarded if editing is canceled or the view is left before saving.
- **Cancel changes the displayed entry:** keep edits in the editor's local draft until Save changes invokes the parent update callback.
- **Focus is lost after Save or Cancel:** preserve the row Edit button ref and focus it after the editor closes.
- **Data survives a full reload:** inspect `App.tsx` for accidental storage; this preview must remain in memory only.

The General section displays: **“Start Outflow automatically when you sign in to your computer.”** and **“Preview only — this setting does not change your computer's startup behavior yet.”** No operating-system login item is registered, and the preference is not written to a configuration file, localStorage, or sessionStorage. The section selection is not persisted. Microphone access, shortcut settings, diagnostics, native Tauri tray/window behavior, and all other dictation features remain unimplemented. The browser preview does not complete F-11, AC-11, T-051, or native startup integration in [`docs/TASKS.md`](docs/TASKS.md). The Rust schema contributes to T-017 but does not complete its migration behavior or its T-001 prerequisite.

The documented Base UI/Nova Switch and Field wrappers are not installed in this React/Vite scaffold (`@base-ui/react`, `components/ui/switch`, and `components/ui/field` are absent). To avoid adding a dependency, the preview uses a native button with `role="switch"`, `aria-checked`, label/description references, Space-key support, and the existing neutral and focus tokens. No component library was added.

The page uses semantic HTML, a neutral color palette, and a responsive centered layout. It adapts to the system's light or dark color preference.

## Testing and checks

The frontend checks remain separate from the Rust workspace and can be run without launching the Rust example:

Run the automated tests once:

```sh
npm run test -- --run
```

Run the tests in watch mode during development:

```sh
npm run test
```

Run the linter and TypeScript type check:

```sh
npm run lint
npm run typecheck
```

`npm run typecheck` invokes the TypeScript 7.0.2 compiler directly. This project also installs TypeScript 6.0.3 under the `typescript` package name because `typescript-eslint` currently uses its JavaScript API, which TypeScript 7 does not provide. The npm aliases follow [Microsoft's documented TypeScript 7 compatibility setup](https://devblogs.microsoft.com/typescript/announcing-typescript-7-0/): `@typescript/native` points to TypeScript 7, while `typescript` points to `@typescript/typescript6`. The explicit compiler path keeps app type checking on TypeScript 7 even if npm links a shared `tsc` executable to TypeScript 6.

## Production build

Create a production build:

```sh
npm run build
```

The command type-checks with TypeScript 7.0.2 before building the static app with Vite. Output is written to `dist/`. To serve that output locally, run:

```sh
npm run preview
```

## Initial Settings layout verification — 2026-10-30

| Command | Result |
|---|---|
| `npm run typecheck` | PASS; TypeScript 7.0.2 |
| `npm run test -- --run` | PASS; 1 test file, 11 tests, including all 4 foundation tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 production build |
| `git diff --check` | PASS; the repository has no commits and the source files are untracked in this checkout, so Git's normal diff check does not include them |

The development server was started with `npm run dev -- --host 127.0.0.1`. Port 5173 was already occupied and displayed the earlier foundation screen; the initial Settings layout preview ran at `http://localhost:5174/`.

The initial browser preview was visually inspected at **1440×900**, **768×800**, **390×844**, and **320×640**. Navigation sat beside the content at 1440px and 768px, then stacked above it at 390px and 320px. The 320px layout scrolled vertically to reveal the full panel and had no horizontal overflow. All six tabs were selected in the browser; each updated both the panel title/description and selected state. Light and dark system appearances were inspected. A keyboard-only pass opened Settings from the foundation, moved through tabs with arrows and Home/End, showed a visible focus ring, and activated Back; reopening returned to General. The browser accessibility tree exposed the navigation, tab list, selected tabs, panel, and headings. The system appearance was restored to its original Auto setting after the dark-theme check.

Text sizes use relative units and the content wraps, but direct 200%/400% browser-zoom testing was not completed. The narrow viewport checks exercise the same stacked layouts, but they do not replace a zoom test. A screen-reader session was also not run.

At that point, the checkout had no Git commits and all provided project files were untracked. `git diff --check` did not inspect those untracked files; the implementation files were reviewed directly.

## Launch at Login preview verification — 2026-10-30

The current preview was started with `npm run dev -- --host 127.0.0.1`. Ports 5173 and 5174 were occupied, so Vite served the latest source at `http://127.0.0.1:5175/`.

| Command | Result |
|---|---|
| `npm run typecheck` | PASS; TypeScript 7.0.2 |
| `npm run test -- --run` | PASS; 1 test file, 16 tests (the 11 existing cases remain) |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 production build |
| `git diff --check` | PASS; this checkout has no commits and the project files are untracked, so the command does not cover them; changed files were inspected directly |

Browser verification confirmed the setting defaults to Off; mouse and Space change it to On and back to Off; keyboard focus is visible; and the accessibility tree exposes a named switch with its checked value. The preview value stayed On after switching to Privacy and back, and after closing and reopening Settings; a page reload reset it to Off. The foundation screen and all other Settings sections remained available.

The current layout was inspected in light appearance at **1440×900**, **768×800**, **390×844**, and **320×640**, with no horizontal overflow. The navigation and panel sit side-by-side at 1440px and 768px and stack at 390px and 320px; narrow layouts scroll vertically to show the full panel. Dark appearance was inspected at 320px in macOS System Settings, and the system appearance was restored to its original **Auto** selection. The browser accessibility tree confirmed the label, description, switch role, state, and preview-only notice.

**Unverified:** 200% browser zoom could not be confirmed with the available in-app browser controls; no native screen-reader session or contrast-tool audit was run. The switch uses the documented behavior and existing semantic colors, but this preview is not an accessibility certification.

## Native settings schema v1 verification before JSON codec — 2026-10-30

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable rustc --version` | PASS; Rust 1.99.1 |
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example defaults` | PASS; printed the four lines shown above |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check -p outflow-settings --locked` | PASS; settings crate builds without external dependencies |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 17 tests total (15 settings, 2 original foundation tests) |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 1 file, 16 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 |
| `git diff --check` | PASS; no tracked changes are available in this no-commit checkout, so the new files were also inspected directly |

The current preview ran at `http://127.0.0.1:5175/` because ports 5173 and 5174 were already in use. Visual verification confirmed the foundation screen, Settings navigation with all six sections, Launch at Login changing from Off to On and back to Off, the preview-only notice in the accessibility tree, and Back returning to the foundation screen. The development server was stopped after verification. No React source changed.

At this earlier checkpoint, the settings model and validation existed before JSON support was added. The subsequent JSON codec verification below records the current serialization status. Migration, file persistence, Tauri integration, and a React connection remain out of scope, and T-001 remains unchecked.

## Native settings JSON codec verification — 2026-10-30

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo generate-lockfile --offline` | PASS; locked the approved exact Serde and `serde_json` versions and their transitive dependencies |
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example json_roundtrip` | PASS; printed the expected three lines below |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 32 tests total: 30 settings tests and 2 core tests; all 17 pre-existing workspace tests remain passing |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS; an initial derivable-default warning was corrected before the final run |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; all 16 frontend tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 |
| `git diff --check` | PASS; this checkout has no commits and project files are untracked, so the changed files were reviewed directly as well |

Expected example output:

```text
Outflow settings JSON round-trip passed
Schema version: 1
Validation: passed
```

The codec uses `serde = 1.0.229` with derive support and `serde_json = 1.0.154`. It validates on encode and decode, rejects missing and unknown fields, unsupported schema versions and invalid values, and keeps error messages free of raw JSON and vocabulary. It performs no file or network operations. No React source changed. T-017 and AC-11 remain incomplete because schema migration, persistence, the T-001 prerequisite, and broader product acceptance work remain outstanding.

## Settings schema compatibility verification — 2026-10-30

| Command | Result |
|---|---|
| `RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example version_compatibility` | PASS; printed the five expected policy lines below |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 33 tests total: 31 settings tests and 2 core tests. All 32 previously existing Rust tests remain passing. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; all 16 frontend tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 production build |
| `git status --short` | PASS; this checkout has no commits, so project files remain untracked; scoped source and new files were inspected directly |
| `git diff --check` | PASS; no tracked baseline is available in this checkout; a direct trailing-whitespace scan of the scoped files was also clean |

Example output:

```text
Outflow settings version policy ready
Schema v1: supported
Schema v0: unsupported
Schema v2: unsupported
Missing version: rejected
```

The example and its dependencies operate on synthetic in-memory JSON; source review confirms no file or network APIs are used. No configuration file was created, no network request was initiated by the example, and no React source changed. This is an in-memory compatibility check, not packet-capture evidence for the broader application. Current limitations remain: there is no historical v0 schema, migration implementation, settings persistence, Tauri integration, or frontend bridge. T-001 remains unchecked, and T-017 and AC-11 remain incomplete.

## Foundation verification (original task)

The foundation was verified on macOS with Node.js 22.23.3 and npm 10.9.9:

| Command | Result |
|---|---|
| `npm install` | PASS; dependencies installed, no reported vulnerabilities |
| `npm ci` | PASS; clean install from `package-lock.json`, no peer-dependency overrides |
| `npm run typecheck` | PASS; TypeScript 7.0.2 |
| `npm run test -- --run` | PASS; 1 test file, 4 tests |
| `npm run lint` | PASS |
| `npm run build` | PASS; Vite 8.3.5 production build |

The running page was visually inspected in the browser at **1440×900**, **390×844**, and **320×640**. The screen remained centered and readable at each size, with no visible horizontal overflow. Its accessibility tree exposed the application name, one level-one heading, and the supporting description.

## Troubleshooting

- **Rust or Cargo is missing:** install Rust 1.99.1 with rustup, then open a new shell and run `rustc --version` and `cargo --version`. The repository toolchain file also requests the required toolchain components.
- **The pinned Rust toolchain cannot be installed:** check network access to the configured Rust distribution server, then run `rustup toolchain install 1.99.1 --component rustfmt --component clippy` and retry the Cargo command.
- **Cargo reports a lockfile or workspace error:** run Cargo from the repository root, confirm `Cargo.toml`, `crates/core/Cargo.toml`, and `crates/settings/Cargo.toml` are present, then regenerate the lockfile with `cargo generate-lockfile` only when manifests have intentionally changed. Review any lockfile diff before keeping it.
- **Rust formatting or Clippy components are unavailable:** run `rustup component add rustfmt clippy --toolchain 1.99.1` and retry the relevant command.
- **`outflow-settings` rejects a settings value:** check the supported schema version, English-only language, user-initiated update policy, shortcut uniqueness, and vocabulary count/phrase limits in the validation error. JSON parse errors show only a line and column; validation and storage errors do not include phrase or shortcut contents. Schema versions other than v1 and unknown fields are rejected.
- **Settings loading reports malformed JSON or invalid UTF-8:** the file is left untouched. Check the file's encoding and JSON syntax, then compare its keys and values with the schema v1 example above. The loader does not repair the file or substitute defaults; preserve a copy before making any manual correction.
- **Settings loading reports an unsupported version or invalid values:** the loader does not migrate or overwrite the file. Version 1 is the only accepted schema. Correct the source using a known schema v1 document or preserve it for a future migration path; do not rename it to another version.
- **Settings loading reports a permission or filesystem error:** confirm the explicit file path exists, is a regular readable file, and is not a symbolic link. The loader does not print the path or operating-system error text. A missing path returns `Ok(None)`; a directory or unreadable existing path is an error.
- **Settings loading rejects a file over 1 MiB:** the loader enforces this resource bound before decoding. Some unusually large string values may exceed it even if they otherwise pass schema validation. Inspect the file size and content locally; loading will not truncate or modify it.
- **Atomic save reports permission or filesystem failure:** confirm the explicit parent directory exists and is writable, the destination is a regular file rather than a symbolic link, and the filesystem permits same-directory atomic rename and directory synchronization. An error before replacement preserves existing contents; `ReplacementDurabilityUncertain` means replacement already happened but directory durability could not be confirmed.
- **Atomic save reports unsupported platform:** current owner-only write guarantees are implemented only on Unix. Windows support requires an approved ACL implementation; the writer fails closed there.
- **Atomic writer example fails:** check that the system temporary directory is available and supports Unix mode bits and same-directory rename; the example prints no settings values or absolute paths and removes its test directory when it exits.
- **Node version warning or syntax errors:** run `node --version` and switch to Node.js 22.12.0 or newer.
- **Dependency or peer-resolution errors:** run `npm ci` from the project root. Keep both TypeScript aliases in `package.json`; do not use `--force` or `--legacy-peer-deps`.
- **Port 5173 is already in use:** Vite chooses an available port by default. Open the URL printed by Vite, or pass a free port explicitly, for example `npm run dev -- --host 127.0.0.1 --port 5174`.
- **Launch at Login returns to Off after a reload:** expected for this preview; the value is held in memory and is not persisted.
- **The computer does not start Outflow at login:** expected; this preview does not register an operating-system login item.
- **The browser cannot connect:** keep the development server running and open the local URL printed in its terminal output.
- **A stale or incomplete dependency install:** run `npm ci`, then rerun the failing command.

## Current implementation status

The starter screen and Settings layout/navigation preview are implemented, with one browser-only Launch at Login preference. The dependency-free `outflow-core` Rust foundation provides a smoke-test status message; `outflow-settings` provides schema v1 defaults, JSON serialization/deserialization, validation, explicit-path loading and safe initialization, explicit confirmed reset, and explicit-path atomic writes on Unix. The reset and settings filesystem behavior were tested on macOS 26.6.2 arm64 only; Linux and Windows were not tested. Migration, Windows ACL writing, and frontend persistence remain unimplemented. The React preference does not persist or affect computer startup, and it is not connected to the Rust model. The other Settings sections remain placeholders; no Reset button, confirmation dialog, Tauri command, Home features, reusable component library, Tauri shell, microphone capture, or in-app dictation behavior is included. Prompt 79 adds a standalone native Moonshine streaming smoke test, not an application runtime. T-001, T-017, and T-018 remain unchecked; T-017 is incomplete because migration behavior and its T-001 prerequisite remain outstanding, and T-018 is incomplete because the cross-platform writer requirements are not met. F-11, AC-11, AC-12, T-051, and native startup integration remain incomplete. The Rust crates are development foundations, not application runtime features.

## Prompt 79 — macOS native inference evidence (partial)

The complete milestone record is [`evidence/mvp/prompt-79-macos-dictation.md`](evidence/mvp/prompt-79-macos-dictation.md). The verified local-only model hashes are in [`evidence/mvp/prompt-79-model-assets.json`](evidence/mvp/prompt-79-model-assets.json). No weights, audio fixtures, native binaries, or signed URLs are stored in this repository.

### Current capability and blocker

The pinned Moonshine Small Streaming C API builds as an arm64 macOS dylib, and [`tests/native/moonshine_streaming_smoke.cpp`](tests/native/moonshine_streaming_smoke.cpp) has streamed local fixture speech through architecture `4`. Prompt 79E adds a separate, development-only AppKit/AVFoundation probe that connects a real microphone capture implementation to the same C API; see [Prompt 79E below](#prompt-79e--development-only-macos-microphone-probe). It is not the shipping application and does not add a Tauri shell, Rust Moonshine binding, cleanup pipeline, target validation, TextEdit insertion, or Copy Result panel. `npm run dev` still launches only the browser settings preview; `npm run tauri dev` is not defined. Do not use the preview as a dictation app.

Prompt 79D did not request microphone permission. Prompt 79E adds a normal user-initiated macOS microphone permission path in the isolated probe. Actual microphone permission and live speech must be reported from the separate Prompt 79E evidence; fixture success is not microphone proof. Accessibility and Automation permission are never requested by the probe. Safe insertion still requires the approved capability and real editable/protected-field checks and is not implemented.

### Prompt 79E — development-only macOS microphone probe

The implementation and its limits are documented in [`evidence/mvp/prompt-79e-detailed-implementation-handoff.md`](evidence/mvp/prompt-79e-detailed-implementation-handoff.md). Ten real-time-paced fixture runs and one sampled AppKit process-tree trace are recorded in [`evidence/mvp/prompt-79e-timed-fixture-measurements.json`](evidence/mvp/prompt-79e-timed-fixture-measurements.json). The probe is a standalone native development tool under `dev/macos-mic-probe/`; it is not linked into the browser preview, Rust workspace, production bundle, or installer.

The probe uses AVAudioEngine's default input, queries its actual sample rate/channel/sample representation, copies callback buffers into a fixed 16-slot queue, downmixes/resamples on a separate worker, and calls the pinned Small Streaming C API with the CPU provider. The worker copies native-owned transcript strings before the next native call. The window shows only volatile partial/final text; Start, Stop, Cancel, Dismiss, and window close have explicit paths. Cancel and Dismiss clear preview text. No audio or transcript is written to disk, copied to the clipboard, injected into another app, or sent over the network. A metrics file is written only when the developer explicitly supplies a new `--metrics-file` path; it contains timings and counts, not speech text.

#### Prerequisites and build

Use a logged-in macOS desktop session, Apple Clang and the macOS SDK, the pinned Moonshine source tree, the already-built and hash-verified Moonshine/ONNX Runtime libraries, and the eight locally authorized model files outside Git. The build fails closed on unexpected source-revision input, header constants, binary hashes, or architecture; it does not download dependencies or assets. Set these variables to the actual local paths (the examples shown are the paths used for this development run):

```sh
export MOONSHINE_SOURCE_DIR=/private/tmp/outflow-feasibility/moonshine-234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
export OUTFLOW_MIC_PROBE_BUILD_DIR=/private/tmp/outflow-mic-probe-build-01

scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
```

The app bundle is created outside the repository and ad-hoc signed for local development. Its `Info.plist` contains a microphone usage description. The code currently expects macOS 26.0 or newer because this is the host/SDK build used for the feasibility pass; it is not a cross-version support claim.

#### Real microphone flow

Launch through the script from a normal desktop Terminal session:

```sh
scripts/run_macos_mic_probe.sh
```

Press **Start recording**. macOS presents its normal microphone prompt on first use; allow access through that prompt. The UI then opens the default microphone and shows its discovered format while listening. Speak a short sentence and press **Stop** to drain final recognition and display the final result. Press **Cancel** to stop capture and discard/clear the result. **Dismiss result** clears the preview and returns to Idle. Closing the window cancels active work. If permission was denied earlier, use **Microphone Settings** to open the normal macOS privacy panel, change access there, and retry. No Accessibility permission is requested.

The success path on real hardware requires both a nonempty partial and a final result from actual microphone samples. Fixture runs below test the same worker/UI path but do not establish microphone permission, device availability, or real-speech quality. Do not record ambient speech from other people as a substitute for the user speaking a test sentence.

Observed on the development Mac on 2026-11-03: macOS permission reached **Granted**, the default input reported **48,000 Hz mono**, and two user-spoken test sessions produced nonempty partial and final UI text before the microphone was released. The recognized wording was imperfect; this proves live-sample path feasibility, not recognition quality. A third 47.8-second capture contained no recognizable speech and also finalized/released cleanly. The first spoken-session metrics file could not be saved because its exclusive path had already been used; the third spoken session used a fresh path and saved exact metadata-only metrics. See the measurement JSON and detailed handoff; no transcript text is stored there.

#### Timed fixture mode

The pinned local `two_cities_16k.wav` fixture is a development smoke fixture, not a human-reviewed WER corpus. Timed replay feeds 1,600-sample chunks at their 100 ms audio duration on a producer thread; inference remains on the worker thread. The app still requires the explicit Start button:

```sh
scripts/run_macos_mic_probe.sh \
  --timed-fixture "$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav" \
  --fixture-seconds 12 \
  --metrics-file /private/tmp/outflow-mic-probe-run-01.json
```

Expected output is an app window labeled **timed fixture** with partial text appearing during replay and a final result after about 12 seconds. Metrics use an exclusive create and refuse to overwrite an existing file; select a fresh path for each run. They include model load, first partial, update cadence, audio duration, stop-to-final, native process high-water RSS, CPU time, thread count, queue depth and overflow count. There is no accelerated-fixture mode in this probe. Prompt 79D's accelerated native smoke measurements remain separate and are not mixed with these time-paced observations.

#### Automated tests and RSS sampling

Run portable/native test coverage with the same pinned source and runtime variables above:

```sh
scripts/test_macos_mic_probe.sh
```

This compiles and runs core audio/state/permission/queue/WAV tests and injected Moonshine lifecycle tests, including 100 start/stop/cancel worker cycles. To additionally run real local Moonshine fixture inference at real-time pace:

```sh
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_FIXTURE_SECONDS=12
export OUTFLOW_MIC_PROBE_FIXTURE_RUNS=10
export OUTFLOW_MIC_PROBE_PROFILE_DIR=/private/tmp/outflow-mic-probe-rss-10
scripts/test_macos_mic_probe.sh --real-fixture
```

This test uses the real native model and writes one 100 ms process-tree RSS record per headless worker-harness run. It does not measure the AppKit app. For an app-window trace, launch a single configured app fixture run, find its PID with `/usr/bin/pgrep -n -x OutflowMicProbe`, then attach the sampler while the window is active:

```sh
APP_PID=$(/usr/bin/pgrep -n -x OutflowMicProbe)
python3 scripts/measure_process_tree_rss.py \
  --pid "$APP_PID" --duration-seconds 30 --interval-ms 100 \
  --output /private/tmp/outflow-mic-probe-app-rss-01.json
```

Use one app instance when attaching; the PID lookup selects the newest matching process. The sampler records process identities, RSS, sample timestamps and gaps, but not command arguments or environment. Its RSS is not macOS physical-footprint measurement and does not certify the 4 GB release gate.

#### Recovery and limits

- `launch with --model-dir`: set `OUTFLOW_MIC_PROBE_MODEL_DIR` to the local model directory and rerun the launch script.
- Model checksum failure: verify the eight files against `evidence/mvp/prompt-79-model-assets.json`; the probe will not fall back to another model.
- Permission denied/restricted: use the visible **Microphone Settings** button and normal System Settings controls; do not edit TCC databases.
- No device, busy device, disconnect, unsupported PCM format, or device-format change: the probe stops the session and reports an error; close competing capture apps or reconnect the device before trying again.
- Queue overflow: capture fails closed rather than growing memory or silently dropping speech.
- Metrics file already exists: choose a new filename; prior evidence is not replaced.
- `kLSNoExecutableErr`: rebuild the `.app` with `scripts/build_macos_mic_probe.sh` and launch it from the logged-in macOS desktop session so LaunchServices and TCC see the application bundle.

Prompt 79E does not complete T-011, T-025–T-034, AC-06, T-010, or any MVP acceptance. It does not add a production Tauri/Rust application, model packaging, injection, clipboard behavior, a Windows/Linux implementation, or release approval. The current 18 GB M3 Pro is not the approved physical 4 GB reference machine. The meaning of “4 GB” remains pending owner clarification between decimal and binary reporting; the code permits up to exactly 4 GiB for classification screening but still requires separately verified physical hardware and an approved reference class. The 950,000,000-byte app-process RSS cap is unchanged.

### Prompt 79F — cancellation verification and recovery

Prompt 79F hardens the same development probe's permission and session lifecycle. The normal **Cancel** action invalidates a pending permission request, marks the session cancelled before stopping its source, closes AVAudioEngine capture, asks the worker to cancel, joins it, clears its audio queue and releases Moonshine stream/transcriber resources. The temporary partial and final text fields are cleared. The UI remains **Cancelled** until **Dismiss result** joins any remaining terminal worker work and returns the session to **Idle**. No final result or cancellation metrics file is written after Cancel, and no clipboard or text insertion path exists. Cancel is safe to repeat; Start is enabled again only after Dismiss completes cleanup.

#### Rebuild and automate the cancellation checks

Use the locally prepared source, native libraries and model from the prerequisites above. Choose a new build directory for each build:

```sh
export OUTFLOW_MIC_PROBE_BUILD_DIR=/private/tmp/outflow-prompt79f-build
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_FIXTURE_SECONDS=12
export OUTFLOW_MIC_PROBE_FIXTURE_RUNS=1
scripts/test_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh --real-fixture
python3 -m unittest discover -s scripts/tests -v
```

The native worker fixtures use injected condition-variable gates to cancel during model loading, immediately after Ready, while samples are queued, after a partial, and during finalization. They check stale event rejection, repeated Stop/Cancel, queue sample scrubbing, and exact stream/transcriber cleanup. The native suite runs 100 paired start/stop/cancel/dismiss state and worker cycles. The Python privacy contract tests cover permission generations, current authorization, callback order, stale results, metrics privacy and join-before-Dismiss. The real-fixture command is a local model/worker integration check; it does not use the microphone or constitute a human-reviewed WER result.

#### Visible fixture cancellation

Run a new app process with timed fixture replay and an unused metadata path:

```sh
scripts/run_macos_mic_probe.sh \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 12 \
  --metrics-file /private/tmp/outflow-prompt79f-cancel-metrics.json
```

Click **Start recording** and wait until **Listening** and a nonempty temporary preview are visible. Press **Cancel**. Confirm the preview and final-result area are empty, the UI says **Cancelled**, capture/fixture and worker cleanup are complete, and no late result appears. The displayed cancellation line includes sanitized queue depth, maximum depth, overflow and (for a live microphone session) microphone-release and worker-cleanup times. Confirm the explicit metrics path does not exist after cancellation. Then press **Dismiss result**, verify **Idle**, and start another fixture session; allow it to finish with **Stop** and verify a fresh final result.

For an additional deterministic UI pass, use the app's timer to cancel during fixture replay:

```sh
scripts/run_macos_mic_probe.sh \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 12 \
  --auto-cancel-after-ms 3500 \
  --metrics-file /private/tmp/outflow-prompt79f-auto-cancel-metrics.json
```

The timer only invokes Cancel while that same session remains Listening. This mode still uses the visible app and actual Moonshine partial inference, but the samples come from the fixture rather than the microphone.

#### Physical microphone Cancel procedure

Launch without fixture options from a logged-in macOS desktop session:

```sh
scripts/run_macos_mic_probe.sh
```

Allow microphone access through the normal macOS prompt if it appears. Do not reset TCC or change permission databases. For **Test A**, press **Start recording**, speak until temporary partial text appears, then press **Cancel**. Verify the microphone format line is replaced by **Cancelled**, both text panes are empty, microphone release and worker-cleanup durations appear, and no final result or cancellation metrics file appears later. For **Test B**, press **Dismiss result**, confirm **Idle**, start a new live session, speak, and press **Stop**; verify the new session reaches **Result ready** and releases the microphone. For **Test C**, dismiss, start again and press **Cancel** as soon as the UI permits; verify cleanup and empty text even if no partial arrived. When practical, close the window during active recording and confirm the app exits after stopping capture and joining the worker.

Permission grant is accepted only for the still-current request generation and session while the window is open, state is Preparing, and AVFoundation still reports Authorized. If Cancel, Dismiss, window close or app termination invalidates that request, a delayed completion is ignored. A new session has a distinct session ID and cannot inherit old worker events.

#### Expected failures and recovery

- **Cancelled session stays visible:** this is expected until **Dismiss result**. Dismiss joins terminal cleanup, clears volatile UI text and returns to Idle.
- **Start remains unavailable after Cancel:** wait for synchronous cleanup to finish, press **Dismiss result**, then retry. If the worker reports an error, dismiss the terminal session and verify the local model hashes before restarting.
- **Permission was denied or revoked:** use **Microphone Settings** and macOS System Settings, then dismiss and retry. The app never edits TCC state. A stale grant callback cannot start capture.
- **No microphone, device disconnect, unsupported format or queue overflow:** the probe stops the source and reports an error; reconnect/select a working default input or close competing audio apps, dismiss, then retry.
- **A final result appears in an older build:** rebuild from the current branch, launch the fresh app bundle using `open -n` via the script, and repeat. The current cancellation handler gates finals on current session state and scrubs late callback text.
- **A supplied metrics path exists after a completed Stop:** metrics are exclusive-create metadata for successful final results. Cancel intentionally does not write that file; choose a fresh path for a new successful-run measurement.

This evidence applies only to the development-only AppKit/AVFoundation probe on the tested macOS machine. It does not complete T-011, T-025–T-034, AC-06, T-010, or any production/release gate. The 4 GB hardware, model quality, full app resource limits, and cross-platform behavior remain unverified.

### Prompt 79G baseline — superseded by Prompt 79H

Prompt 79G introduced a development-only **Copy Result** action to the existing AppKit probe. Its `writeObjects:`-without-clear behavior and warning below describe the pre-79H baseline only. For the current replacement sequence, warning, failure behavior, test results, and desktop verification status, use the Prompt 79H section immediately after this baseline. This prototype does not implement the production Tauri result panel, target identity checks, or insertion. **T-046, T-047, T-010, T-011, AC-10, and the MVP remain incomplete.** The final preview stays selectable and is retained in volatile memory only while its accepted session is ResultReady.

#### Build and launch

Use the already-authorized local Moonshine source, libraries, model, and fixture. These paths match the environment used for the Prompt 79G verification on the local macOS host; choose a fresh build directory if it already exists:

```sh
export MOONSHINE_SOURCE_DIR=/private/tmp/outflow-feasibility/moonshine-234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_BUILD_DIR=/private/tmp/outflow-prompt79g-build
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"
scripts/run_macos_mic_probe.sh \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 12
```

Press **Start timed fixture**, wait for temporary partial text, then press **Stop**. For microphone capture, launch without fixture options with `scripts/run_macos_mic_probe.sh`, press **Start recording**, speak, and press **Stop**. Model loading, partial recognition, Stop, automatic finalization, Cancel, Dismiss, and shutdown do not write to the clipboard.

#### Copy, failure recovery, and privacy

**Copy Result** is disabled before a result, during Preparing/Listening/Finalizing, after an empty or silent result, and after Cancel, Dismiss, or Error. It becomes enabled only when a nonempty actual final transcript belongs to the current accepted ResultReady session. A display-only “No speech was recognized in this session.” message is never eligible. Click **Copy Result** or activate it with **Return**; then paste manually into the destination field. The final preview stays visible until **Dismiss result**.

The 79G warning and writer description above are historical and superseded by Prompt 79H. The current writer converts UTF-8 before pasteboard access, then calls `clearContents` once and `writeObjects:` once after an explicit Copy Result action. Clipboard managers or OS synchronization may retain copied content. If the operating system reports a write failure, the app keeps the final text visible and selectable and suggests selecting it and pressing **⌘C**. It does not automatically retry, read the prior clipboard, restore prior contents, or claim rollback/atomicity; failure after clearing may lose the previous contents. Cancel stops capture and clears in-app text; Dismiss clears the final preview and returns to Idle. Neither action writes clipboard content.

#### Tests and observed results

Run the focused development-probe suites and full repository regressions with:

```sh
scripts/test_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh --real-fixture
python3 -m unittest discover -s scripts/tests -v
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
git diff --check
```

The focused native suite reports 13 core groups and 9 worker lifecycle groups, including 100 repeated start/stop/cancel/dismiss cycles. The real-fixture suite uses the authorized 12-second local WAV and reports nonempty partial and final recognition. The source-level privacy suite checks that writes are limited to the explicit guarded action and that no normal clipboard-content read is added. Prompt 79H test counts and the separate isolated/desktop verification layers are recorded in `evidence/mvp/prompt-79h-detailed-implementation-handoff.md`; this local AppKit work is not evidence for production AC-10 or release gates.

### Prompt 79H — macOS Copy Result replacement and manual paste

Prompt 79H corrects the development-only AppKit probe's **Copy Result** operation. The app checks current-session eligibility and converts the exact UTF-8 result before it touches the pasteboard. After an explicit click or Return activation, it calls `clearContents` once and then makes one `writeObjects:` plain-text write. Start, partial/final recognition, Stop, Cancel, Dismiss, errors, and shutdown do not write to the system clipboard. The final preview stays selectable in volatile app memory until Dismiss or shutdown. This does not implement production Tauri UI, target checks, or insertion; **T-046, T-047, T-010, T-011, AC-10, and the MVP remain incomplete.**

The warning displayed before copying is: **“Copy replaces the clipboard. If writing fails after clearing it, prior contents may be lost. Clipboard managers or sync may retain copies.”** Clear and write are separate operations, so a failed write may leave prior contents lost. The app never reads or restores previous clipboard content, retries, or claims rollback. A failed copy keeps the final result visible and selectable and suggests selecting it and pressing **⌘C**. `NSPasteboard.accessBehavior` is an access-policy value and is not treated as a diagnosis for an individual write failure. See Apple's [`clearContents`](https://developer.apple.com/documentation/appkit/nspasteboard/clearcontents%28%29) and [`writeObjects:`](https://developer.apple.com/documentation/appkit/nspasteboard/writeobjects%28_%3A%29?language=objc) documentation.

#### Build and run the development app

Export the pinned local Moonshine paths from the preceding Prompt 79G **Build and launch** block, then use a fresh Prompt 79H build directory:

```sh
export OUTFLOW_MIC_PROBE_BUILD_DIR=/private/tmp/outflow-prompt79h-build
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"
scripts/run_macos_mic_probe.sh \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 12
```

The runner launches a fresh bundle through LaunchServices (`open -n -a`). Start the timed fixture, wait for actual partial text, press Stop, and wait for a nonempty final result. No clipboard operation occurs before explicit Copy Result.

#### Isolated AppKit pasteboard test

`scripts/test_macos_mic_probe.sh` now compiles and runs `dev/macos-mic-probe/tests/pasteboard_integration_test.mm`. The test uses `+[NSPasteboard pasteboardWithUniqueName]`, not the general pasteboard. It checks the initially empty test pasteboard, four fixed synthetic ASCII/Unicode/emoji/multiline values, exact plain-text readback, replacement without stale items, and `releaseGlobally` cleanup. The existing pure policy and worker lifecycle tests still run in the same script. This test requires a macOS SDK and an available pasteboard service in the test user's session; it does not prove system general-pasteboard access.

#### Controlled manual Copy Result → TextEdit verification

Only Copy Result intentionally replaces the system clipboard. Do not inspect or preserve unrelated prior clipboard data.

1. With the timed fixture active, wait for nonempty partial text, press Stop, and wait for a nonempty final result.
2. Click **Copy Result** and confirm the app reports **“Copied. Paste manually where you want the text.”**
3. Switch to an unsaved blank TextEdit document and press **⌘V**. Compare the pasted text visually with the final preview, including punctuation, spaces, and newlines; discard the unsaved document without saving.
4. Return to the probe, dismiss the result, and confirm Copy Result is disabled. Repeat with a new fixture and activate Copy Result using **Return**.
5. Run a separate fixture Cancel flow and confirm it clears the result and leaves Copy Result disabled without copying.

Repository evidence must not contain recognized words, clipboard contents, or the TextEdit document. Record only pass/fail for visual equality and the sanitized application status. If graphical automation is unavailable or blocks the desktop layer, report **PARTIALLY VERIFIED** even when the isolated test passes.

#### Failed-write recovery and troubleshooting

- A failed `writeObjects:` result does not itself identify the cause. Check that the probe runs in an interactive graphical login session and that the current `.app` bundle was launched through the supplied LaunchServices script. The app does not infer a permission denial from `accessBehavior`.
- If LaunchServices returns `kLSNoExecutableErr` before the app window opens, record it as a launch failure; it says nothing about the pasteboard write. Do not relabel it as a clipboard permission denial or claim desktop verification.
- Run the isolated AppKit test separately. If it passes while Copy Result on the general pasteboard fails, record the precise app status and investigate the host launch/session or OS behavior. Do not reset TCC, broaden permissions, or disable system security.
- If failure follows clear, previous clipboard contents may have been lost. Keep the in-app final result visible, select it, and press **⌘C** for an explicit manual recovery attempt. There is no automatic retry or rollback.

Full validation commands are listed in the preceding Prompt 79G test block. Exact Prompt 79H results and the separate isolated/desktop verification layers are in [`evidence/mvp/prompt-79h-detailed-implementation-handoff.md`](evidence/mvp/prompt-79h-detailed-implementation-handoff.md). This work does not complete T-046, T-047, AC-10, or a production release gate.

### Prompt 79I — identify the fresh probe instance

The development-only runner accepts an optional `--verification-id <identifier>`. IDs are 1–32 ASCII letters, digits, or hyphens. Missing, repeated, empty, overlong, or malformed IDs are rejected before LaunchServices starts the app; the native app performs the same validation if launched directly. The ID is held only for that process and is not written to settings. With an ID, the title is `Outflow Mic Probe — Dev — <ID> — PID <pid>` and the window shows the same ID and PID as a noneditable diagnostic label. Without the option, the normal `Outflow Mic Probe — Development Only` title and UI remain unchanged.

Use the existing pinned Moonshine source, libraries, model, and WAV fixture environment described above. A fresh build directory is required for every app build:

```sh
export OUTFLOW_MIC_PROBE_BUILD_DIR="$(mktemp -d /private/tmp/outflow-prompt79i.XXXXXX)"
scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"

scripts/run_macos_mic_probe.sh \
  --verification-id "79I-COPY-TEST-01" \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 12
```

The runner prints the app bundle, verification ID, and a reminder that an `open` success status does not prove the target window is ready. Confirm the exact ID and PID in the visible window before interacting. To cross-check the displayed process, use the PID from the window:

```sh
PID_FROM_WINDOW=12345 # replace with the PID shown in the window
ps -p "$PID_FROM_WINDOW" -o pid=,comm=
```

The command should identify the executable inside the fresh `OutflowMicProbe.app` bundle. Older probe windows do not show the new ID; do not use them for the fixture or copy test. If automation attaches to an older window or reports a stale application ID, use Mission Control or the normal macOS window/application switcher and select the visible title containing the exact ID. If neither automation nor normal desktop selection can reach that window, record the desktop steps as blocked and stop; do not modify TCC, manipulate processes, or weaken system protections.

#### Copy Result → TextEdit desktop check

The app displays this warning before the action: **“Copy replaces the clipboard. If writing fails after clearing it, prior contents may be lost. Clipboard managers or sync may retain copies.”** Copy Result intentionally replaces the system clipboard; a later write failure may lose its previous contents. Do not inspect or restore unrelated clipboard data.

1. In the window with the expected ID and PID, click **Start timed fixture**. Wait for nonempty partial text and press **Stop**; wait for the actual nonempty final result.
2. Confirm **Copy Result** is enabled. Click it once and record only whether AppKit reports success or the displayed sanitized failure status.
3. Open a new, blank, unsaved TextEdit document in plain-text mode. Press **⌘V** manually. Compare visually with the final preview, including spaces, punctuation, Unicode, and line breaks. Do not log, screenshot, copy elsewhere, or save the recognized text; discard the document without saving.
4. Return to the same probe, choose **Dismiss result**, and confirm the result clears and Copy Result disables.
5. If the first copy succeeds, perform one new fixture session and activate Copy Result with **Return**. This is a second clipboard replacement; proceed only when the operator has authorized it. Separately start a new fixture and choose **Cancel**; verify the preview clears and the action remains disabled.

The isolated AppKit integration test uses a unique test pasteboard with fixed synthetic strings. Its PASS proves only that AppKit can replace and read that isolated board; it does not prove delivery through the system general pasteboard or a manual TextEdit paste. Conversely, a successful LaunchServices request does not prove that the fresh process's window was targeted. Keep those results separate in the handoff.

#### Prompt 79I regression and troubleshooting commands

Run the focused and repository checks with the authorized local assets already configured:

```sh
scripts/build_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh --real-fixture
python3 -m unittest discover -s scripts/tests -v
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
git diff --check
```

For each additional full app build, choose a new unused `OUTFLOW_MIC_PROBE_BUILD_DIR`. If the visible title lacks the requested ID, the wrong/old instance is selected or the runner did not forward the option; stop before Copy. If the correct window reports an AppKit failure, record the ID, PID, target-window confirmation, nonempty final/action-enabled state, callback result, and exact status. Do not infer permission denial. Run the isolated test separately; do not change the pasteboard writer absent a reproducible new defect. `kLSNoExecutableErr` or automation attachment failure is a launch/host limitation, not evidence about clipboard access.

The detailed run record is [`evidence/mvp/prompt-79i-detailed-implementation-handoff.md`](evidence/mvp/prompt-79i-detailed-implementation-handoff.md). This development probe does not complete production T-046/T-047, T-010/T-011, AC-10, or the MVP.

### Authorized local development model preparation

The pinned source commit `234f60faa0eb388b01cdf7e60aca232af37aefda` selects the English Small Streaming catalog path `https://download.moonshine.ai/model/small-streaming-en/quantized_26_08_21/`. The path is a dated catalog directory, not a separate immutable artifact revision. The eight files total 142,300,974 bytes. Their observed hashes are recorded in the evidence manifest; the existing verifier checks every byte and SHA-256:

```sh
set -eu
MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
mkdir -m 700 -p "$MODEL_DIR"
BASE=https://download.moonshine.ai/model/small-streaming-en/quantized_26_08_21
for file in adapter.ort cross_kv.ort decoder_kv.ort encoder.ort \
  frontend.model.ort frontend.weights.ort streaming_config.json tokenizer.bin; do
  curl --fail --location --show-error --output "$MODEL_DIR/$file" "$BASE/$file"
done
python3 scripts/validate_model_asset_inventory.py \
  --manifest evidence/mvp/prompt-79-model-assets.json \
  --asset-root "$MODEL_DIR"
```

This is a developer-controlled preparation step, not application behavior. The bundle was downloaded only for this local inference test under the user's explicit authorization. Keep model bytes outside Git. The manifest's artifact-specific license and redistribution status remain unresolved; the upstream English streaming model-family policy is not a substitute for artifact review or commercial approval. Do not package or distribute these files based on this local test.

### Pinned macOS native build and smoke test

Prerequisites used here: macOS 26.6.2 (25G83), arm64 Apple M3 Pro with 18 GB RAM, Xcode Command Line Tools / Apple Clang 21.0.0.21000101, CMake 4.4.5, the clean pinned Moonshine checkout, and its already-present macOS arm64 ONNX Runtime 1.23.2. The out-of-tree wrapper enables the upstream `MOONSHINE_TTS_BUILD_ONNX` target because the current C API references those symbols even when that option is off. CMake also compiles two source-checkout Git LFS inputs: pyannote community-1 data and embedded VCTK reference voices. They were materialized only in the external checkout for this local build; their notices/release rights are not approved.

```sh
MOONSHINE_SOURCE=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
WRAPPER_DIR=/private/tmp/prompt79-moonshine-wrapper
BUILD_DIR=/private/tmp/prompt79-moonshine-wrapper-build-onnx
mkdir -p "$WRAPPER_DIR"
cat > "$WRAPPER_DIR/CMakeLists.txt" <<EOF
cmake_minimum_required(VERSION 3.22.1)
project(prompt79_moonshine_wrapper LANGUAGES C CXX)
set(MOONSHINE_TTS_BUILD_ONNX ON CACHE BOOL "Build upstream C API dependencies" FORCE)
add_subdirectory(${MOONSHINE_SOURCE}/core moonshine-core)
EOF
cmake -S "$WRAPPER_DIR" -B "$BUILD_DIR" \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_ARCHITECTURES=arm64 \
  -DMOONSHINE_TTS_BUILD_ONNX=ON
cmake --build "$BUILD_DIR" --clean-first --target moonshine \
  --config Release --parallel 4
```

Compile and run the real streaming test with the same local model directory and the pinned checkout's `test-assets/two_cities_librivox_48k.wav` fixture (only its first 12 seconds are processed):

```sh
clang++ -std=c++17 -O2 -I"$MOONSHINE_SOURCE/core" \
  tests/native/moonshine_streaming_smoke.cpp \
  -L"$BUILD_DIR/moonshine-core" -lmoonshine \
  -Wl,-rpath,"$BUILD_DIR/moonshine-core" \
  -Wl,-rpath,"$MOONSHINE_SOURCE/core/third-party/onnxruntime/lib/macos/arm64" \
  -o /private/tmp/prompt79_moonshine_stream_smoke
DYLD_LIBRARY_PATH="$MOONSHINE_SOURCE/core/third-party/onnxruntime/lib/macos/arm64:$BUILD_DIR/moonshine-core" \
  /private/tmp/prompt79_moonshine_stream_smoke \
  /private/tmp/prompt79-moonshine-small-streaming \
  "$MOONSHINE_SOURCE/test-assets/two_cities_librivox_48k.wav"
```

Expected output includes `MODEL_LOAD_OK architecture=4` and two `SESSION` rows with nonempty partial/final results and complete final lines. The test deliberately does not print transcript text. The fixture is not copied into Outflow. Its upstream filename identifies it as LibriVox audio; LibriVox states its recordings are public domain, while the pinned Moonshine checkout does not include the original catalog item or narrator record. This is a local smoke fixture, not a WER corpus. See [LibriVox public-domain policy](https://librivox.org/pages/public-domain/).

### Current measurements and limits

Prompt 79's earlier three-run `getrusage` maximum was 922,386,432 bytes. Prompt 79C added a 100 ms process-tree sampler and repeated the existing two-session smoke ten times on the verified 18 GB M3 Pro: the highest sampled tree RSS was **1,023,426,560 bytes** (median 925,245,440 bytes). Four runs exceeded the 950,000,000-byte release threshold. The process tree contained only the native smoke executable; it is not an Outflow app measurement.

A separate one-fixture native batch measurement processed the full 56.269896-second smoke WAV. The sampled process tree peaked at **974,405,632 bytes**, comprising the native helper at 955,564,032 bytes and the small Python capture wrapper at 18,841,600 bytes. Recognition took 5,475.256 ms (RTF 0.097); thread-inclusive process CPU time was 30,156.661 ms and stop-to-final was 0.001 ms. This is one smoke utterance without a verified reference, so it supplies no WER and no meaningful p50/p95 corpus latency. The separate three-load probe measured 289.508 ms for its first load and a 79.125 ms median for two in-process reloads; OS file-cache state was not controlled.

The current Prompt 79 native build therefore misses the 950,000,000-byte RSS threshold even before a Tauri/webview process is present. T-010 remains **BLOCKED — INSUFFICIENT EVIDENCE** overall: there is no verified WER corpus, no physical 4 GB host, Tiny was not acquired or tested, and the native build has not isolated STT from unused TTS/diarization sources. The Small+current-runtime measurements are a resource-gate failure; do not select it for an application or claim the complete model candidate evaluation is finished. Eight local Small assets total 142,300,974 bytes. The native library is 10,164,432 bytes and the linked ONNX Runtime dylib is 27,623,992 bytes. These figures exclude the app, installer, and all remaining files; they are not installed-size evidence.

### Prompt 79C reproduction: native evaluation and process-tree RSS

These commands use the existing authorized local Small model files at `/private/tmp/prompt79-moonshine-small-streaming`; they do not download or package them. Confirm the pinned source, model hashes, and fixture readiness first:

```sh
MOONSHINE_SOURCE_DIR=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
BUILD_DIR=/private/tmp/prompt79-moonshine-wrapper-build-onnx

python3 scripts/moonshine_model_source_audit.py --source-dir "$MOONSHINE_SOURCE_DIR"
python3 scripts/validate_model_asset_inventory.py \
  --manifest evidence/mvp/prompt-79-model-assets.json --asset-root "$MODEL_DIR"
python3 scripts/validate_fixture_manifest.py
```

Build and run the existing transcript-free two-session native smoke test:

```sh
clang++ -std=c++17 -O2 -I"$MOONSHINE_SOURCE_DIR/core" \
  tests/native/moonshine_streaming_smoke.cpp \
  -L"$BUILD_DIR/moonshine-core" -lmoonshine \
  -Wl,-rpath,"$BUILD_DIR/moonshine-core" \
  -Wl,-rpath,"$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64" \
  -o /private/tmp/prompt79c_moonshine_stream_smoke
DYLD_LIBRARY_PATH="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64:$BUILD_DIR/moonshine-core" \
  /private/tmp/prompt79c_moonshine_stream_smoke \
  "$MODEL_DIR" "$MOONSHINE_SOURCE_DIR/test-assets/two_cities_librivox_48k.wav"
```

Build the development-only batch helper. Its stdout is a private JSON pipe containing raw hypotheses; **do not invoke it directly in a terminal**. The Python benchmark captures it in memory and writes only sanitized metrics.

```sh
clang++ -std=c++20 -O2 -I"$MOONSHINE_SOURCE_DIR/core" \
  tests/native/moonshine_asr_batch.cpp \
  -L"$BUILD_DIR/moonshine-core" -lmoonshine \
  -Wl,-rpath,"$BUILD_DIR/moonshine-core" \
  -Wl,-rpath,"$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64" \
  -o /private/tmp/prompt79c_moonshine_asr_batch

python3 scripts/run_native_asr_benchmark.py \
  --model-dir "$MODEL_DIR" --asset-root "$MODEL_DIR" \
  --source-dir "$MOONSHINE_SOURCE_DIR" \
  --native-runner /private/tmp/prompt79c_moonshine_asr_batch \
  --moonshine-library "$BUILD_DIR/moonshine-core/libmoonshine.dylib" \
  --onnxruntime-library "$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib" \
  --machine-label MAC-ARM64-01 --cpu-label "Apple M3 Pro" \
  --output /private/tmp/prompt79c-measured-asr.json
```

The current fixture manifest has **zero ready fixtures**, so the runner exits nonzero with `no verified ready fixtures are available for native evaluation` before model loading. It writes no measured-results file. To run the corpus evaluation, first add a lawfully usable file-level corpus and human-verified references under the fixture policy, then re-run this command with a new output path. The opt-in integration test needs the environment values listed in `scripts/tests/test_native_asr_integration.py`; it skips when the verified corpus or native artifacts are absent.

Run the sampler for one smoke process (choose a new output path each time; it refuses to overwrite evidence):

```sh
python3 scripts/measure_process_tree_rss.py --interval-ms 100 \
  --output /private/tmp/prompt79c-rss-run-01.json -- \
  /private/tmp/prompt79c_moonshine_stream_smoke \
  "$MODEL_DIR" "$MOONSHINE_SOURCE_DIR/test-assets/two_cities_librivox_48k.wav"
```

The JSON records a timestamped per-process snapshot, PIDs, process start identity, per-process RSS bytes, tree peak RSS bytes, sampling gaps, and limits. It never stores command arguments, environment variables, audio, or transcript output. On macOS it currently records no additional physical-footprint metric; the 100 ms `ps` sampling can miss short-lived helpers and RSS sums can double-count shared pages. It measures the launched tree, not the full Outflow application. Run ten times with distinct output paths before quoting the highest observed result.

Validate native notices and reproduce the available local audits with:

```sh
python3 scripts/validate_moonshine_third_party.py
python3 scripts/validate_third_party_notices.py
python3 scripts/validate_moonshine_build_scope.py
python3 -m unittest scripts.tests.test_measure_process_tree_rss scripts.tests.test_native_asr_benchmark scripts.tests.test_native_asr_integration -v
```

`PHYSICAL_4GB_GATE` remains `BLOCKED`; neither a VM limit nor this 18 GB development Mac certifies the reference machine. The macOS Accessibility probe returned `AX_NOT_TRUSTED`; no UI controls were targeted or edited. Full measured WER, Tauri process-tree RSS, CPU-only 4 GB behavior, and insertion remain unverified.

The trust-state query was read-only and did not request permission or control any UI:

```sh
CLANG_MODULE_CACHE_PATH=/private/tmp/prompt79c-clang-module-cache swift -e \
  'import ApplicationServices; print(AXIsProcessTrusted() ? "AX_TRUSTED" : "AX_NOT_TRUSTED")'
```

### Prompt 79D reproduction: feasibility corrections and paired memory runs

Prompt 79D tightened the fixture-coverage, physical-hardware, native-linkage, and measured-result checks. The current host is macOS arm64 with 19,327,352,832 installed physical bytes, so it is ineligible for the 4,000,000,000-byte reference-machine cap. The reference-class allowlist is empty. The fixture manifest has 0 ready fixtures, so no real WER result exists and the full quality gate remains `NOT_ASSESSED`.

Set the same local source, build, and model locations used by the recorded experiment. These paths are outside Git and must already exist; the commands do not download source, models, or speech data.

```sh
MOONSHINE_SOURCE_DIR=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
BUILD_DIR=/private/tmp/prompt79-moonshine-wrapper-build-onnx
MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
ORT_DIR="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64"

python3 scripts/moonshine_model_source_audit.py --source-dir "$MOONSHINE_SOURCE_DIR"
python3 scripts/validate_model_asset_inventory.py \
  --manifest evidence/mvp/prompt-79-model-assets.json --asset-root "$MODEL_DIR"
python3 scripts/validate_fixture_manifest.py
```

Rebuild the development-only native helpers against the existing Release library. The batch helper has a private stdout protocol that contains hypotheses; never run its inference mode directly in a terminal. `--runtime-identity` is sanitized, and the Python evaluator captures inference output in memory.

```sh
clang++ -std=c++17 -O2 -I"$MOONSHINE_SOURCE_DIR/core" \
  tests/native/moonshine_streaming_smoke.cpp \
  -L"$BUILD_DIR/moonshine-core" -lmoonshine \
  -Wl,-rpath,"$BUILD_DIR/moonshine-core" \
  -Wl,-rpath,"$ORT_DIR" \
  -o /private/tmp/prompt79d_moonshine_stream_smoke

clang++ -std=c++20 -O2 -I"$MOONSHINE_SOURCE_DIR/core" \
  tests/native/moonshine_asr_batch.cpp \
  -L"$BUILD_DIR/moonshine-core" -lmoonshine \
  -Wl,-rpath,"$BUILD_DIR/moonshine-core" \
  -Wl,-rpath,"$ORT_DIR" \
  -o /private/tmp/prompt79d_moonshine_asr_batch

DYLD_LIBRARY_PATH="$BUILD_DIR/moonshine-core:$ORT_DIR" \
  python3 scripts/verify_native_runtime.py \
    --native-runner /private/tmp/prompt79d_moonshine_asr_batch \
    --moonshine-library "$BUILD_DIR/moonshine-core/libmoonshine.dylib" \
    --onnxruntime-library "$ORT_DIR/libonnxruntime.1.23.2.dylib"
```

The runtime verifier asks the helper which dyld image contains `moonshine_get_version`, records all active image paths in volatile memory, and checks that the loaded helper, Moonshine library, and ONNX Runtime exactly match the supplied inputs. It checks architecture with `lipo`, dependency declarations with `otool -L`, hashes before and after the probe, and rejects unresolved/ambiguous identities or unapproved non-system images. Apple shared-cache images do not always have standalone files to hash; only documented system roots are accepted. The sanitized output includes the helper, Moonshine, and ONNX Runtime SHA-256 values without persisting local absolute paths.

The native smoke used two sequential 12-second streaming sessions over the same upstream audio slice. Reproduce both measured configurations with ten fresh process-tree runs each; use a new output directory because the sampler refuses to overwrite evidence.

```sh
RUNS_DIR=/private/tmp/prompt79d-repro-runs
mkdir -p "$RUNS_DIR"
for mode in default single_thread; do
  for run in $(seq -w 1 10); do
    output="$RUNS_DIR/${mode}-${run}.json"
    if [ "$mode" = single_thread ]; then
      python3 scripts/measure_process_tree_rss.py --interval-ms 100 \
        --output "$output" -- env MOONSHINE_ORT_SINGLE_THREAD=1 \
        /private/tmp/prompt79d_moonshine_stream_smoke "$MODEL_DIR" \
        "$MOONSHINE_SOURCE_DIR/test-assets/two_cities_librivox_48k.wav"
    else
      python3 scripts/measure_process_tree_rss.py --interval-ms 100 \
        --output "$output" -- \
        /private/tmp/prompt79d_moonshine_stream_smoke "$MODEL_DIR" \
        "$MOONSHINE_SOURCE_DIR/test-assets/two_cities_librivox_48k.wav"
    fi
  done
done
```

`MOONSHINE_ORT_SINGLE_THREAD=1` is the pinned source's ThreadSanitizer/reliability escape hatch, not a production profile. In the recorded matrix it reduced the ten-run median sampled RSS to 808,402,944 bytes and maximum to 907,902,976 bytes, while session p95 rose from 1,214.004 ms to 3,269.501 ms. Default-thread maximum was 1,010,614,272 bytes, with 2/10 over the 950,000,000-byte cap. The complete paired measurements, all run peaks, warm/cold load probes, exact artifacts, and sampler caveats are in [`prompt-79d-native-memory-measurements.json`](evidence/mvp/prompt-79d-native-memory-measurements.json). This native-only experiment does not certify the Tauri app or 4 GB hardware.

The measured evaluator and validator commands are:

```sh
DYLD_LIBRARY_PATH="$BUILD_DIR/moonshine-core:$ORT_DIR" \
  python3 scripts/run_native_asr_benchmark.py \
    --model-dir "$MODEL_DIR" --asset-root "$MODEL_DIR" \
    --source-dir "$MOONSHINE_SOURCE_DIR" \
    --native-runner /private/tmp/prompt79d_moonshine_asr_batch \
    --moonshine-library "$BUILD_DIR/moonshine-core/libmoonshine.dylib" \
    --onnxruntime-library "$ORT_DIR/libonnxruntime.1.23.2.dylib" \
    --machine-label MAC-ARM64-01 --cpu-label "Apple M3 Pro" \
    --output /private/tmp/prompt79d-measured-asr.json

python3 scripts/validate_measured_asr_results.py \
  --file /private/tmp/prompt79d-measured-asr.json
```

With the checked-in manifest, the evaluator currently exits with `no verified ready fixtures are available for native evaluation` before loading the model and writes no result. The checked-in manifest contains eight planned rows and zero ready rows. Do not use the smoke WAV as a WER fixture: it has no reviewed reference or Outflow fixture permission record. A ready fixture requires the exact audio and reference hashes, measured audio metadata, file-level provenance and permission evidence, independent coverage attributes, and a human-verified transcription. `collection_status: complete` must wait until all minimum counts and diversity requirements in [`docs/BENCHMARK_FIXTURES.md`](docs/BENCHMARK_FIXTURES.md) pass. The validator rejects duplicate IDs, malformed fields, missing/changed reference files, incomplete coverage, and unsupported attribute combinations.

After a specific recording's redistribution and commercial-evaluation rights have been approved, add its WAV and a separate human-verified UTF-8 reference under the policy-approved repository paths. Listen to the complete recording and adjudicate names, numbers, abbreviations, and punctuation before marking it ready. These commands only hash exact bytes and read WAV metadata; they do not generate or review a reference:

```sh
AUDIO=fixtures/benchmark-audio/example.wav
REFERENCE=fixtures/benchmark-references/example.txt
shasum -a 256 "$AUDIO" "$REFERENCE"
python3 - "$AUDIO" <<'PY'
import json
import sys
import wave

with wave.open(sys.argv[1], "rb") as audio:
    if audio.getcomptype() != "NONE" or audio.getsampwidth() != 2 or audio.getnchannels() != 1:
        raise SystemExit("fixture WAV must be uncompressed PCM16 mono")
    frames = audio.getnframes()
    rate = audio.getframerate()
    print(json.dumps({
        "sample_format": "PCM_S16LE",
        "sample_rate_hz": rate,
        "channels": audio.getnchannels(),
        "frame_count": frames,
        "duration_seconds": frames / rate,
    }, sort_keys=True))
PY
python3 scripts/validate_fixture_manifest.py
```

Record the resulting hashes and measured metadata in the manifest, along with source release/file ID, permission evidence, duration class, independent coverage attributes, source-reported variety if relevant, and the human-review method/status. Do not add Common Voice files without written clarification on its no-mirroring terms; do not infer file rights from a catalog page.

The measured-results writer runs `validate_measured_results` before opening the output file and refuses to overwrite an existing path. The separate synthetic schema remains `result_type: synthetic`; synthetic counts or hypotheses must not be substituted for real ASR results. Result validation checks schema, exact hardware fields, corpus coverage, WER arithmetic, timing summaries, native binary hashes, runtime identity assertions, release-gate consistency, and transcript/audio/path content exclusion.

Current dataset blockers: Mozilla's [Common Voice terms](https://commonvoice.mozilla.org/dav/terms) ask users not to mirror any dataset subset outside Mozilla Data Collective, so do not check its files into this repository without written clarification and review. [LibriSpeech SLR12](https://us.openslr.org/12/) lists CC BY 4.0; a selected file still needs exact release/file attribution, permission review, and human audio/reference adjudication. No candidate dataset was downloaded for Prompt 79D.

Run the complete requested local regression set from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
git diff --check
```

The full checked suite results and exact platform limits for this correction are recorded in [`prompt-79d-detailed-implementation-handoff.md`](evidence/mvp/prompt-79d-detailed-implementation-handoff.md). A passing native smoke, source audit, or schema test is not a WER result, physical 4 GB measurement, application RSS result, or release approval.

Run the complete applicable repository checks from the root:

```sh
python3 scripts/moonshine_model_source_audit.py \
  --source-dir /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
python3 -m unittest discover -s scripts/tests -v
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
```

Rerun on 2026-11-03 after the native-harness and documentation changes: 311 Python tests, 352 Rust unit/integration tests plus one doctest, and 136 frontend tests passed; Rust formatting/check/test/clippy/build and frontend typecheck/test/lint/build all passed. The model-source audit passes the source contract but reports its source-checkout artifact status as `NO`; local model integrity was separately verified by the evidence manifest and existing asset validator. The browser build is not a desktop app build.

### Troubleshooting and unsupported flows

- **Missing or corrupted local model:** rerun the asset validator with the exact model directory; it rejects missing/unreadable files and SHA-256 mismatches. The Prompt 79 evidence also compares observed file sizes with the pinned catalog's expected byte counts. This development check does not download anything and is not integrated into an application runtime.
- **Native library not found:** confirm `libmoonshine.dylib` and the ONNX Runtime path in `DYLD_LIBRARY_PATH`; no loader code or runtime model fetch is included in Outflow.
- **Cannot launch dictation:** there is no launchable macOS application yet. Start/Stop/Cancel, microphone permission recovery, partial preview, cleanup, focus mismatch, protected-field rejection, TextEdit insertion, and explicit Copy Result are not available.
- **Offline claim:** the smoke test used local model/audio files and the native library has a local ORT dependency, but no packaged app was run with network disabled or packet-captured. Offline first-launch acceptance remains untested.
- **Licensing and native notices:** exact model artifact terms, commercial redistribution, complete ONNX Runtime/KISS FFT notices, and the compiled TTS/diarization inputs need review. The local-only authorizations do not approve Git, packaging, or redistribution.

## Model asset inventory

The planned ASR model and native runtime asset inventory is [`docs/model-asset-inventory.json`](docs/model-asset-inventory.json). Its schema, fields, provenance rules, licensing boundaries, and steps for adding concrete assets are documented in [`docs/MODEL_ASSET_INVENTORY.md`](docs/MODEL_ASSET_INVENTORY.md).

Validate the planning inventory and run its focused tests from the repository root:

```sh
python3 scripts/validate_model_asset_inventory.py \
  --manifest docs/model-asset-inventory.json
python3 -m unittest scripts.tests.test_validate_model_asset_inventory -v
```

The validator prints a concise summary. For the initial planning inventory, the expected output is:

```text
Outflow model asset inventory valid
Manifest version: 1
Planned assets: 4
File-verified assets: 0
Redistribution approvals: 0
Bundled model status: NOT VERIFIED
T-009 licensing audit: INCOMPLETE
```

Add a planned asset by adding a unique row with one of the four supported categories and explicit `null` values for revision, file path, SHA-256, or license facts that are unknown. Set review fields to `not-reviewed`, `undetermined`, and `undetermined` until those decisions have evidence. A license identifier alone never records redistribution approval.

To check real local files later, record each exact source revision, relative file path, SHA-256, and provenance reference; then pass the directory that contains those paths:

```sh
python3 scripts/validate_model_asset_inventory.py \
  --manifest docs/model-asset-inventory.json \
  --asset-root <local-asset-directory>
```

Only rows explicitly marked `file-verified` are opened and hashed. Paths must remain within the supplied root, including symlink targets. Hashing is read-only and incremental. This command does not download assets, prove installer inclusion, validate license evidence, approve redistribution, or complete T-009. The current inventory has no model revision or checksums, no verified native dependency list, and no completed redistribution or attribution review.

Troubleshooting:

- **Unknown metadata:** keep its field as `null`; do not use placeholders or infer model/runtime licenses from a project repository license.
- **Unsafe path:** use a slash-separated relative path without `..`, absolute prefixes, or backslashes, and keep the target inside `--asset-root`.
- **Missing or mismatched file:** confirm the root and relative path identify the exact local bytes, then recompute SHA-256 after any artifact change. Do not change an expected digest without source review.
- **File integrity passes but T-009 remains incomplete:** expected. The validator does not establish commercial redistribution rights, third-party notice completeness, or installer contents.

## Moonshine licensing review

The source-backed preliminary review is [`docs/MOONSHINE_LICENSE_REVIEW.md`](docs/MOONSHINE_LICENSE_REVIEW.md), with claim-level provenance in [`docs/moonshine-license-review.json`](docs/moonshine-license-review.json). The official [Moonshine license policy](https://github.com/moonshine-ai/moonshine/blob/main/docs/license.md), [model catalog](https://github.com/moonshine-ai/moonshine/blob/main/docs/models/available-models.md), [repository LICENSE](https://github.com/moonshine-ai/moonshine/blob/main/LICENSE), [third-party source directory](https://github.com/moonshine-ai/moonshine/tree/main/core/third-party), and [C API reference](https://moonshine-voice.readthedocs.io/en/stable/api/c-api/) are listed with their reviewed scope and revision status in those records.

The catalog labels English Small Streaming and Tiny Streaming as MIT, and Moonshine's policy says all streaming STT models are MIT. The policy separately identifies a limited set of legacy, non-English, non-streaming models under the Community License. Moonshine's own repository code excludes `core/third-party`; those subdirectories follow supplier terms. The source-policy review records 6 upstream-documented claims and 3 unverified items; it does not itself identify or hash a model artifact or native runtime, assemble the complete notices, or approve redistribution. Prompt 79C separately validated eight existing local Small file hashes and recorded the macOS development library/runtime hashes and build scope in [`evidence/mvp/prompt-79c-native-license-audit.md`](evidence/mvp/prompt-79c-native-license-audit.md). The model's immutable artifact revision is unavailable, the complete notice bundle and artifact-specific rights remain unresolved, redistribution remains **undetermined**, and T-009 remains incomplete.

Validate the local record and run its focused tests with Python's standard library:

```sh
python3 scripts/validate_moonshine_license_review.py
python3 -m unittest scripts.tests.test_validate_moonshine_license_review -v
python3 -m unittest discover -s scripts/tests -v
```

The validator makes no network requests. It checks local JSON structure, source URLs, revision representation, evidence status, and the separation between documentation and artifact integrity. The synthetic validation does not verify source truth, determine legal meaning, or inspect model/runtime assets.

Before packaging, the project still needs the exact quantized model revisions and file list, per-file licenses and hashes, the complete platform runtime dependency tree and its notices, installer reconciliation, and final human/legal redistribution review.

Troubleshooting:

- **An upstream page is unavailable:** use the immutable commit-specific GitHub links in the review document where provided. The Read the Docs `stable` page and current third-party directory are mutable references; if they cannot be opened, retain their revision as unverified.
- **A source revision cannot be resolved:** leave its commit explicitly unknown in the JSON (`status: "unverified"`, `commit: null`) and explain why. Do not substitute a guessed SHA, mirror, or model artifact revision.
- **A model or runtime claim needs artifact proof:** this documentation validator cannot provide it. Only inspect an expressly authorized local asset and record its exact provenance, file path, and checksum in the asset inventory; documentation evidence alone does not verify file integrity.
- **A license appears to allow use:** keep redistribution approval undetermined until exact shipped assets, third-party obligations, notice packaging, installers, and required human review have been checked.

## Moonshine third-party source audit

The source-only inventory is [`docs/moonshine-third-party.json`](docs/moonshine-third-party.json), with per-dependency terms, pinned source/build evidence, and remaining notice questions in [`docs/MOONSHINE_THIRD_PARTY_AUDIT.md`](docs/MOONSHINE_THIRD_PARTY_AUDIT.md). It reviews the eight direct `core/third-party` entries at Moonshine commit `234f60faa0eb388b01cdf7e60aca232af37aefda`. The source review classifies five entries as confirmed build inputs, two as conditional TTS/G2P inputs, and doctest as test-only. It does not establish what Outflow will ship: redistribution approval remains **undetermined**, and no runtime binary or model has been verified.

Validate the local inventory and run its focused tests with Python's standard library:

```sh
python3 scripts/validate_moonshine_third_party.py
python3 -m unittest scripts.tests.test_validate_moonshine_third_party -v
python3 -m unittest discover -s scripts/tests -v
```

The validator performs no network requests. It checks the schema, all eight required directories, evidence URL/revision consistency, allowed classifications, unknown values, and the requirement that redistribution and binary review remain unresolved. The test suite covers malformed and incomplete records, explicit unknowns, unsupported approval claims, binary-verification scope, and stable summary output. Neither validator proves legal sufficiency or verifies binaries.

Before any redistribution decision, identify the exact per-platform ORT library provenance and transitive notices, determine whether optional TTS/G2P is included, resolve KISS FFT's referenced full BSD-3-Clause text, review Eigen's covered files and MPL source obligations, reconcile all notices with installers, and obtain human/legal review. The inventory does not approve redistribution or complete T-009.

Troubleshooting:

- **Unsupported build-use value:** use `confirmed-build-input`, `conditional`, `test-only`, or `unknown`; leave uncertain build paths as `unknown`.
- **Missing or mutable evidence link:** use an HTTPS GitHub blob/tree URL containing the recorded commit SHA and keep a license link inside that dependency's pinned directory.
- **Unknown license fact:** set `license_status` to `unknown` with an empty identifier list; do not infer a license from Moonshine's repository-level license.
- **The validator passes but release review is incomplete:** expected. It checks only local source inventory consistency; it does not fetch sources, inspect binaries, reconcile installers, or grant redistribution approval.

## Third-party notice manifest

The planned notice manifest is [`docs/third-party-notices.json`](docs/third-party-notices.json), based on the pinned source audit in [`docs/moonshine-third-party.json`](docs/moonshine-third-party.json). Read [`docs/THIRD_PARTY_NOTICES.md`](docs/THIRD_PARTY_NOTICES.md) for its fields, notice states, open questions, and release process. The manifest covers all eight audited source entries without assuming that test-only or conditional dependencies will ship.

Validate the manifest and run its focused tests with Python's standard library:

```sh
python3 scripts/validate_third_party_notices.py
python3 -m unittest scripts.tests.test_validate_third_party_notices -v
```

The current validator output is:

```text
Outflow third-party notice manifest valid
Manifest version: 1
Dependencies: 8
Packaged notices: 0
Redistribution approval: UNDETERMINED
Notice readiness: INCOMPLETE
Notice states: 4 planned, 4 blocked
```

Run all Python checks with `python3 -m unittest discover -s scripts/tests -v`. The validator reads the local audit inventory, performs no network requests or intentional file writes, and does not prove package contents or legal permission. T-009 remains incomplete.

Troubleshooting:

- **License or build-use mismatch:** copy the verified expression and classification from `docs/moonshine-third-party.json`; this planning manifest does not revise the source audit.
- **Evidence URL rejected:** use that dependency's immutable `blob/234f60faa0eb388b01cdf7e60aca232af37aefda/...` license-document URL from the inventory.
- **Unsafe destination:** leave `planned_notice_destination` as `null` until a repository-relative destination is selected; absolute and traversal paths are invalid.
- **Blocked notice or incomplete readiness:** keep the status blocked or planned until exact selected files, binaries, notices, and package placement have evidence. A passing validator is not packaging verification or redistribution approval.

## Moonshine native build scope

The source-backed build plan is [`docs/MOONSHINE_BUILD_SCOPE.md`](docs/MOONSHINE_BUILD_SCOPE.md), with its machine-readable checklist at [`docs/moonshine-build-scope.json`](docs/moonshine-build-scope.json). It describes Outflow's intended single English streaming STT runtime with CPU-only inference across the documented macOS, Windows, and Linux targets. Linux Wayland remains best-effort.

Validate the pinned CMake evidence and the existing third-party dependency/notices inventory with Python's standard library:

```sh
python3 scripts/validate_moonshine_build_scope.py
python3 -m unittest scripts.tests.test_validate_moonshine_build_scope -v
```

The manifest records 6 required features, 6 intended exclusions, 0 undetermined scope classifications, and 11 unresolved build questions. An intended exclusion is not a verified build exclusion: the pinned CMake source includes diarization and auxiliary model code in the main source list; adds the TTS subtree; and declares test, benchmark, and utility executables without a general disable option. `MOONSHINE_TTS_BUILD_ONNX` defaults to `ON`; setting it off only gates part of the TTS/G2P target graph. The exact CPU-only ONNX Runtime configuration and one English model's files/hashes are unresolved.

Task 76 configured the pinned `core/` CMake project on macOS arm64 and retained its generated cache and File API reply outside both source trees. No Moonshine target was compiled or linked, and no model was downloaded. Task 75 separately verified the local ONNX Runtime dylib's bytes and arm64 architecture; CMake generation does not prove that the library links or that its ABI is compatible. T-009 remains unchecked; T-010, T-013, AC-01, and AC-06 remain incomplete.

### Moonshine Native Dependency and License Inventory

[`scripts/moonshine_native_license_inventory.py`](scripts/moonshine_native_license_inventory.py) reads the pinned source checkout, an existing CMake cache, and its File API reply to list the selected `moonshine` target's source/link inputs. It makes only local Git metadata queries, hashes found license/notice files, and does not compile, download, or install anything. Run it with the Task 76 inputs:

```sh
python3 scripts/moonshine_native_license_inventory.py \
  --source-dir ../moonshine-upstream-234f60f \
  --build-dir /private/tmp/outflow-moonshine-task76-cmake-arm64-final
```

The report labels components `RUNTIME`, `BUILD-ONLY`, `OPTIONAL`, or `UNRESOLVED` and flags incomplete attribution with `REDISTRIBUTION_EVIDENCE_INCOMPLETE`. Moonshine's root MIT license covers Moonshine-authored repository source outside `core/third-party`; vendored third-party components use their own local license/notice evidence and are assessed separately. A license file's presence does not establish complete notices, binary/package contents, or redistribution approval. The existing model candidate inventory is not a selected production bundle: Small Streaming revision, exact model files and hashes, model-specific license evidence/approval, and the T-013 runtime manifest remain outstanding. This report supports the native dependency portion of T-009; it does not complete T-009.

Troubleshooting:

- **Mutable or malformed source URL:** use a GitHub `blob/234f60faa0eb388b01cdf7e60aca232af37aefda/...` URL with a line anchor; the validator rejects branch and tag references.
- **Invented option or changed default:** compare the recorded name, default, and conditional expression with the pinned CMake source. The validator intentionally rejects undocumented switches.
- **An excluded feature is reported as unresolved:** keep its `actual_compiled_status` at `not-tested` and add the source evidence and open question needed to resolve it; do not mark it absent without binary inspection.
- **CPU-only build feasibility is unknown:** retain the unresolved status until the selected per-platform ONNX Runtime artifact/provider is identified and tested.
- **Model asset revision is unknown:** retain it as undetermined until T-013 records the exact files and hashes. The validator does not download assets or inspect binaries.

### Pinned CMake source probe

[`docs/MOONSHINE_CMAKE_PROBE.md`](docs/MOONSHINE_CMAKE_PROBE.md) documents the read-only source probe for the six CMake files pinned to Moonshine revision `234f60faa0eb388b01cdf7e60aca232af37aefda`. It verifies Git blob IDs before checking a narrow set of source behaviors. It does not prove a minimal STT build or inspect compiled/package contents.

Verify an existing source checkout without network access:

```sh
python3 scripts/probe_moonshine_cmake.py \
  --source-dir /path/to/pinned/moonshine-source
```

Fetch only the six pinned CMake text files over HTTPS, in memory, for explicit upstream verification:

```sh
python3 scripts/probe_moonshine_cmake.py --verify-upstream
```

The upstream mode needs a trusted CA bundle available to Python. If certificate verification fails, set `SSL_CERT_FILE` to a trusted bundle; do not disable HTTPS verification. The local mode never makes network requests. A successful probe reports the TTS ONNX default and source-list relationships, then keeps minimal STT exclusions, CMake configure, native compilation, and binary contents explicitly unproven or unverified.

Troubleshooting:

- **Missing file or Git blob mismatch:** use a complete checkout at the pinned revision. The probe rejects files with changed bytes; a source tree from another commit is not interchangeable.
- **`BLOCKED: upstream source retrieval failed`:** check HTTPS access and Python's trusted CA configuration. Retry only after certificate trust is configured; the probe does not follow redirects or fetch additional files.
- **CMake contract error:** compare the active, uncommented statements in the pinned source with the exact diagnostic. A changed default or source relationship requires reviewing the source contract; do not weaken the expected blob IDs.
- **Minimal STT exclusions remain `NOT PROVEN`:** this is expected. Source analysis does not configure the project, compile target objects, or inspect binary/package contents.

## Moonshine CMake configure experiment

[`docs/MOONSHINE_CMAKE_CONFIGURE.md`](docs/MOONSHINE_CMAKE_CONFIGURE.md) records the configure runner and the original Task 25 attempt. The runner accepts only a clean Git checkout at Moonshine revision `234f60faa0eb388b01cdf7e60aca232af37aefda`, verifies the six pinned CMake files with the Task 24 probe, and checks upstream's CMake minimum. It requires a new build directory, rejects existing paths and directories inside either source tree, and retains the generated cache, File API reply, and diagnostics for inspection.

For the macOS arm64 toolchain, run the following with a clean pinned checkout. `mktemp` creates a new parent under the system temporary directory; the runner creates and retains its `build/` child:

```sh
MOONSHINE_SOURCE_DIR="/path/to/pinned/moonshine"
BUILD_PARENT="$(mktemp -d "${TMPDIR:-/tmp}/outflow-moonshine-configure.XXXXXX")"
MOONSHINE_BUILD_DIR="$BUILD_PARENT/build"

python3 scripts/try_moonshine_cmake_configure.py \
  --source-dir "$MOONSHINE_SOURCE_DIR" \
  --build-dir "$MOONSHINE_BUILD_DIR" \
  --generator "Unix Makefiles" \
  --c-compiler "$(command -v clang)" \
  --cxx-compiler "$(command -v clang++)" \
  --osx-architectures arm64
```

Task 76 used this same runner and selection with the verified local source and `/private/tmp/outflow-moonshine-task76-cmake-arm64-final`; the [Task 76 evidence](evidence/model/moonshine-native-configure-task-76.md) records the exact expanded invocation and result.

The runner supplies `CMAKE_BUILD_TYPE=Release`, `MOONSHINE_TTS_BUILD_ONNX=OFF`, `MOONSHINE_RELIABILITY=OFF`, `FETCHCONTENT_FULLY_DISCONNECTED=ON`, and `FETCHCONTENT_UPDATES_DISCONNECTED=ON`. It does not invoke a build, install, CTest, or download command. These FetchContent settings are not a general network sandbox: inspect the configured CMake entry and its included modules for custom acquisition paths. For the pinned `core/` entry, inspection found no `FetchContent`, `ExternalProject`, `file(DOWNLOAD)`, `execute_process`, or curl/wget path. The separate `micro/` entry does use FetchContent for Pico SDK and is not configured here.

Inspect the retained result with:

```sh
rg -n '^(CMAKE_BUILD_TYPE|CMAKE_C_COMPILER|CMAKE_CXX_COMPILER|CMAKE_GENERATOR|CMAKE_HOME_DIRECTORY|CMAKE_OSX_ARCHITECTURES|FETCHCONTENT_FULLY_DISCONNECTED|FETCHCONTENT_UPDATES_DISCONNECTED|MOONSHINE_RELIABILITY|MOONSHINE_TTS_BUILD_ONNX|ONNXRUNTIME_LIB_PATH):' \
  "$MOONSHINE_BUILD_DIR/CMakeCache.txt"
rg -n 'CMAKE_HOST_SYSTEM_PROCESSOR|CMAKE_SYSTEM_PROCESSOR' \
  "$MOONSHINE_BUILD_DIR/CMakeFiles" --glob 'CMakeSystem.cmake'
rg -n 'CMAKE_C_COMPILER_ID|CMAKE_CXX_COMPILER_ID|CMAKE_C_COMPILER_VERSION|CMAKE_CXX_COMPILER_VERSION' \
  "$MOONSHINE_BUILD_DIR/CMakeFiles" --glob 'CMakeCCompiler.cmake' --glob 'CMakeCXXCompiler.cmake'
find "$MOONSHINE_BUILD_DIR" -maxdepth 2 -type f -print
```

Expect `CMakeCache.txt`, the selected generator's build description, `CMakeFiles/`, and `.cmake/api/v1/reply/`. CMake may compile its small compiler-identification checks during configuration. Those checks are not Moonshine target compilation. `CONFIGURED` (exit 0) means CMake generated the build system; it does not prove a minimal STT binary, native linkage, ABI compatibility, inference, or release readiness. `BLOCKED` (exit 2) means a prerequisite or required local file is unavailable. `FAILED` (exit 1) means source integrity or an unexpected configure assumption failed. Read the runner's blocker and first actionable CMake error, preserve the build directory, and report its path; use a new directory for any later attempt.

Task 25's original configure attempt was **BLOCKED** because Git could not resolve `github.com` and CMake was not installed. Tasks 73–75 later verified CMake, acquired the pinned source, and materialized and verified the macOS arm64 ONNX Runtime artifact. Task 76 then configured the native `core/` project successfully with CMake 4.4.5, Apple Clang 21.0.0.21000101, the `Unix Makefiles` generator, and `arm64`. CMake reported the FetchContent variables as unused by this project; the configure path had no acquisition commands, and no dependency download was invoked. This is configure-only evidence, not proof of network isolation for arbitrary CMake code.

Troubleshooting:

- **`BLOCKED: Source directory is unavailable` or revision mismatch:** prepare a clean checkout at the exact commit above with `GIT_LFS_SKIP_SMUDGE=1`; do not substitute a package subdirectory, another revision, or an LFS pointer.
- **`BLOCKED: CMake executable is not available` or unsupported version:** use a CMake installation that meets the minimum read from the pinned `core/CMakeLists.txt`; do not install dependencies as part of the experiment without authorization.
- **`BLOCKED` for a missing dependency:** read the bounded CMake diagnostics and use only the exact, separately approved artifact retrieval documented under Task 75. Do not trigger LFS or model downloads from a configure attempt.
- **ONNX Runtime path is missing or an LFS pointer:** the configure result does not make that file usable. The runner reports LFS pointers as non-binaries and never treats their pointer text as a native library.
- **CMake reports `CONFIGURED`:** inspect the target list and cache, but keep binary contents and minimal STT exclusions unproven. Task 76's generated target graph included test, benchmark, TTS/G2P, and utility targets; the source provides no general disable switch for them. Continue to treat native compilation and linkage as separate steps.

### Task 76 configure result

The pinned source and Task 75 ONNX Runtime file passed the preflight. The final CMake run returned exit code 0 and generated `CMakeCache.txt`, `Makefile`, `CTestTestfile.cmake`, and CMake File API replies in a retained directory under the host's temporary directory. The cache selects `core/` as `CMAKE_HOME_DIRECTORY`, `Unix Makefiles`, `/usr/bin/clang`, `/usr/bin/clang++`, and `CMAKE_OSX_ARCHITECTURES=arm64`; the imported ONNX Runtime target resolves to the verified `libonnxruntime.1.23.2.dylib`.

The CMake 4.4.5 run completed configuration and generation without a policy compatibility error. Its only warning was that the two FetchContent settings were unused. No Moonshine target build, link, install, CTest, model download, or inference ran; the runtime model manifest is still missing. After license/model feasibility and the T-013 bundle are resolved, the next native milestone is the separately controlled build/link and ABI verification required by T-022.

## Moonshine Native Build Readiness

[`scripts/moonshine_native_preflight.py`](scripts/moonshine_native_preflight.py) checks whether the local machine has the prerequisites for attempting the existing native configure experiment. It runs offline and does not configure or build Moonshine, download source/models, or install tools. On macOS arm64 it also reads the pinned ONNX Runtime Git LFS pointer from the verified source commit and checks the materialized library's byte length, SHA-256, and Mach-O architecture; it never fetches or replaces the file. Until equivalent checks are added for other platform library paths, those hosts remain unverified and cannot report configure readiness.

### CMake toolchain prerequisite

Check for CMake and CTest with `command -v cmake`, `cmake --version`, `command -v ctest`, and `ctest --version`. On macOS, the approved installation option when CMake is missing is Homebrew's official formula: `brew install cmake`. Task 73 approved and attempted that command, but its first invocation stopped because `/opt/homebrew/Cellar` was not writable. The maintainer later reported CMake installed; the successful installation invocation was not observed. Verification found CMake and CTest 4.4.5 at `/opt/homebrew/bin/cmake` and `/opt/homebrew/bin/ctest`.

To smoke-test the local C/C++ toolchain without configuring Moonshine, create a disposable fixture:

```sh
SMOKE_DIR="$(mktemp -d)"
cat > "$SMOKE_DIR/CMakeLists.txt" <<'EOF'
cmake_minimum_required(VERSION 3.20)

project(outflow_cmake_smoke LANGUAGES C CXX)

enable_testing()

add_executable(outflow_cmake_smoke main.c)

add_test(
    NAME native_toolchain_smoke
    COMMAND outflow_cmake_smoke
)
EOF
cat > "$SMOKE_DIR/main.c" <<'EOF'
int main(void) {
    return 0;
}
EOF
cmake -S "$SMOKE_DIR" -B "$SMOKE_DIR/build"
cmake --build "$SMOKE_DIR/build"
ctest --test-dir "$SMOKE_DIR/build" --output-on-failure
```

Expect C and C++ compiler checks to succeed, the executable to build, and CTest to report `1/1` passing. Before cleanup, verify `SMOKE_DIR` is the nonempty temporary directory created above and contains the fixture; then remove only that directory with `rm -rf "$SMOKE_DIR"`. Task 73 passed this smoke test with Apple Clang 21.0.0.21000101.

Run it from the repository root:

```sh
python3 scripts/moonshine_native_preflight.py
```

The check requires a supported host, CMake at or above the minimum declared by the pinned source, C and C++ compiler executables, and a clean local Moonshine checkout at the revision already pinned in `scripts/probe_moonshine_cmake.py`. It checks for `core/CMakeLists.txt`, the pinned Git revision and clean state, and a header declaring `MOONSHINE_HEADER_VERSION` and `moonshine_load_transcriber_from_files`. Supply the local checkout explicitly:

```sh
python3 scripts/moonshine_native_preflight.py --source-dir /path/to/clean/moonshine
```

`READY_TO_ATTEMPT_CONFIGURE` means the checks for the current host passed, including verification of the selected macOS arm64 ONNX Runtime artifact when running on that host. It means only that a separate configure attempt may be tried; it does not mean CMake configuration or compilation succeeded. The configure-only runner requires `--source-dir` and a new `--build-dir`, retains generated files, and applies both FetchContent disconnected settings. It does not build or invoke download helpers. Do not run `scripts/build-ort-*`, `scripts/fetch-voice-assets.sh`, `examples/c++/download-library.sh`, or the language-binding model download helpers as part of an offline configure attempt. The separate `micro/` CMake entry can fetch the Raspberry Pi Pico SDK through `FetchContent`; it is not the native C API entry point.

### Pinned source checkout (Task 74)

The approved upstream is [`https://github.com/moonshine-ai/moonshine.git`](https://github.com/moonshine-ai/moonshine) at the full commit `234f60faa0eb388b01cdf7e60aca232af37aefda`. The local checkout is stored outside this repository in the sibling directory `moonshine-upstream-234f60f`. From the Outflow repository root, verify it with:

```sh
MOONSHINE_SOURCE_DIR="../moonshine-upstream-234f60f"
MOONSHINE_PIN="234f60faa0eb388b01cdf7e60aca232af37aefda"
git -C "$MOONSHINE_SOURCE_DIR" remote get-url origin
git -C "$MOONSHINE_SOURCE_DIR" rev-parse HEAD
git -C "$MOONSHINE_SOURCE_DIR" status --porcelain
test "$(git -C "$MOONSHINE_SOURCE_DIR" remote get-url origin)" = \
  "https://github.com/moonshine-ai/moonshine.git"
test "$(git -C "$MOONSHINE_SOURCE_DIR" rev-parse HEAD)" = "$MOONSHINE_PIN"
test -z "$(git -C "$MOONSHINE_SOURCE_DIR" status --porcelain)"
```

The expected remote is `https://github.com/moonshine-ai/moonshine.git`, `HEAD` must equal the full pin, and the status command must print nothing. The checkout is detached at that commit. Its C API header is `core/moonshine-c-api.h`; the pinned declaration is `#define MOONSHINE_HEADER_VERSION (30000)`. Check it and run the offline preflight with:

```sh
rg -n 'MOONSHINE_HEADER_VERSION|moonshine_load_transcriber_from_files|moonshine_create_stream|moonshine_start_stream|moonshine_transcribe_add_audio_to_stream|moonshine_transcribe_stream|moonshine_stop_stream|moonshine_free_stream|moonshine_free_transcriber' \
  "$MOONSHINE_SOURCE_DIR/core/moonshine-c-api.h"
python3 scripts/moonshine_native_preflight.py --source-dir "$MOONSHINE_SOURCE_DIR"
```

Task 74 verified the remote, full commit, detached state, clean tree, header declarations, CMake minimum, and local compiler/CMake prerequisites. At that time the preflight reported `READY_TO_ATTEMPT_CONFIGURE` despite the required ONNX Runtime file being an unmaterialized LFS pointer; Task 75 corrected that false-ready result. The selected quantized English streaming model, its exact files and hashes, and a T-013 runtime manifest remain unavailable. The full Git source tree includes upstream-tracked SpellingCNN/TinyVadCNN micro model files, a Tiny English tokenizer asset, and test audio; no separate Small Streaming ASR model bundle was fetched.

### Pinned ONNX Runtime artifact (Task 75)

The native macOS arm64 CMake branch in `core/third-party/onnxruntime/find-ort-library-path.cmake` selects one shared library: `core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib`. Its tracked Git object is an LFS pointer, so a regular clone can contain the pointer text instead of the native library. The pinned LFS identity is SHA-256 `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`, size `27623992` bytes. The native CMake target imports this dylib; the macOS arm64 static archive is selected by the separate Swift build path and is not required for this native C API configure.

Identify and inspect the candidate from the pinned checkout. If the working file begins with the Git LFS pointer header, use `git show` to read the tracked object metadata; do not treat the pointer text as a library:

```sh
ORT_RELATIVE_PATH="core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/$ORT_RELATIVE_PATH"
file "$ORT_LIBRARY"
ls -lh "$ORT_LIBRARY"
head -c 160 "$ORT_LIBRARY" # only when file reports ASCII text / an LFS pointer
git -C "$MOONSHINE_SOURCE_DIR" show "HEAD:$ORT_RELATIVE_PATH"
```

After a separately approved, exact-path LFS fetch, materialize and verify only that artifact:

```sh
git -C "$MOONSHINE_SOURCE_DIR" lfs fetch \
  --include="$ORT_RELATIVE_PATH" origin "$MOONSHINE_PIN"
git -C "$MOONSHINE_SOURCE_DIR" lfs checkout "$ORT_RELATIVE_PATH"
stat -f '%z bytes' "$ORT_LIBRARY"
shasum -a 256 "$ORT_LIBRARY"
file "$ORT_LIBRARY"
lipo -info "$ORT_LIBRARY"
```

The size and complete SHA-256 must match the pointer above, and `file`/`lipo` must identify an arm64 Mach-O dynamic library. If Git LFS is already installed but reports that it is not initialized, `git -C "$MOONSHINE_SOURCE_DIR" lfs install --local --skip-repo` registers filters in that checkout only and does not install hooks or change global Git settings. Use an exact `--include` path; unrestricted `git lfs pull` may download unrelated runtimes, TTS/auxiliary assets, and model data. The ONNX Runtime library checksum is not a speech-model checksum.

Rerun the offline readiness check with `python3 scripts/moonshine_native_preflight.py --source-dir "$MOONSHINE_SOURCE_DIR"`. It reports a missing library, `LFS_POINTER_NOT_MATERIALIZED`, `PRESENT_UNVERIFIED`, or `VERIFIED`. A verified result contributes to `READY_TO_ATTEMPT_CONFIGURE`; it proves only local prerequisites and the selected library's metadata/architecture match, not CMake generation, native compilation, C ABI linkage, or ASR inference.

Task 75 materialized only this 27,623,992-byte macOS arm64 library. The checkout remains at the approved commit and clean under its Git LFS filters. Speech-model assets remain a separate T-013 prerequisite.

The upstream root `LICENSE` covers Moonshine-authored source under MIT except `core/third-party`. The pinned source has per-dependency license files but no top-level third-party NOTICE file. Upstream documents the English and streaming STT model family as MIT, legacy non-English non-streaming model exceptions, and separate TTS/G2P licensing. This is only a source inventory; T-009 still requires a complete native/runtime/model redistribution review.

See the [Task 74 evidence](evidence/model/moonshine-pinned-source-task-74.md) for exact inspection results and remaining blockers.

The preflight uses only local filesystem reads, PATH lookup, bounded `cmake --version`, read-only Git revision/status/metadata queries, and local SHA-256/Mach-O inspection. A ready result does not prove CMake configuration, compilation, C ABI linkage, inference, speech-model hashes or licensing, CPU-only behavior, accuracy, installed size, or memory compliance. It does not complete T-009, T-010, T-013, T-022, or AC-06.

### Prompt 78 — Moonshine Small Streaming model-source audit

“Prompt 78” is the research prompt number, not canonical `T-078` (the signed installer clean-machine beta). This evidence relates to T-009 and does not mark T-078 complete.

The native source commit and model-artifact revision are separate pins: the commit contains the loader contract and a catalog snapshot, while the model files live at catalog URLs and are not included in the source checkout. Audit the local source with:

```sh
python3 scripts/moonshine_model_source_audit.py \
  --source-dir "$MOONSHINE_SOURCE_DIR"
```

At pinned source `234f60faa0eb388b01cdf7e60aca232af37aefda`, Small Streaming English is `MOONSHINE_MODEL_ARCH_SMALL_STREAMING` (`4`). English by itself does not select it: the dependency lookup defaults to the first catalog model, Medium Streaming, so callers must set `model_arch=4`. Quantization is represented by the catalog's dated URL directory, not a separate model variant option. The dependency function builds a local manifest from the source-bundled catalog and generated size/CRC32C registry; it does not fetch a remote manifest. The real audit lists the eight required files and the optional word-timestamp attention decoder and spelling group.

The registry still provides catalog metadata, not an immutable artifact revision. Prompt 79 subsequently acquired eight Small files for local inference, recorded their separate observed SHA-256 values in [`prompt-79-model-assets.json`](evidence/mvp/prompt-79-model-assets.json), and validated those exact local bytes. The catalog path is not an artifact revision, and the hashes are not vendor-published release identifiers. No redistribution approval has been recorded. See the [Prompt 78 evidence](evidence/model/moonshine-small-streaming-requirements-task-78.md) and [Prompt 79C native license/build audit](evidence/mvp/prompt-79c-native-license-audit.md).

## Task 26 — Home page preview

Start the React/Vite preview from the repository root:

```sh
npm run dev
```

Open the local URL printed by Vite (usually `http://localhost:5173/`). Home is the first view. Open Settings with the button in the header or **Open Settings** in the Getting started panel. Use **Home** at the top of Settings to return. These views switch inside the frontend without a full page reload. Settings values, including the Launch at Login preview switch, live only in memory and do not change operating-system behavior.

`src/app/AppHeader.tsx` contains the shared Outflow identity and Home/Settings navigation. `App.tsx` owns the active page and destination-heading focus; both Home and Settings render the same header, with the current page marked for assistive technology.

Home identifies this as a development preview and says native dictation is not connected. It explains Outflow's goal of offline desktop dictation and the planned local speech and text workflow. The preview does not request microphone access, record audio, run speech recognition, or include a model-download or transcript-history interface. Tauri integration, microphone capture, speech recognition, native shortcuts, and backend behavior are not implemented here.

Run the frontend checks from the repository root:

```sh
npm run typecheck
npm run test -- --run
npm run lint
npm run build
```

To verify navigation and layout, start the dev server, use Tab and Enter/Space to move between Home and Settings and operate the preview switch, then resize the browser to about 320px and 900px. Check a 200% zoom/reflow view when browser zoom is available, and confirm the content wraps without horizontal scrolling. Narrow viewport testing does not replace a browser-zoom check. The Home and Settings controls are native buttons with visible keyboard focus.

Troubleshooting:

- **Unsupported Node version:** the frontend requires Node.js `>=22.12.0`; check with `node --version`.
- **Dependencies are missing:** run `npm ci` from the repository root to install the versions in `package-lock.json`.
- **Port 5173 is already in use:** use the alternate local URL printed by Vite.
- **Navigation does not change views:** check `activeView` and `navigateTo` in `src/app/App.tsx`; the preview uses in-memory state and does not change the URL.
- **Focus does not move after navigation:** check the destination heading refs and focus effect in `src/app/App.tsx` and the `tabIndex={-1}` headings in both views.
- **The preview switch resets after a page refresh:** this is expected; preview settings are in-memory only.
- **The microphone or Dictate action is missing:** native recording and recognition are not part of this frontend preview.

The Home page preview does not complete Tauri-dependent acceptance criteria; `AC-11` remains unverified for the native application.

See [Task 26 frontend evidence](evidence/frontend-home-preview-task-26.md) for the exact commands and manual browser results.
See [Task 27 shared-header evidence](evidence/frontend-shared-header-task-27.md) for the extraction, verification results, and accessibility limitations.

## Task 28 — Shared page container

[`src/app/PageContainer.tsx`](src/app/PageContainer.tsx) provides the neutral wrapper shared by Home and Settings. Its `.page-container` rule in [`src/styles/global.css`](src/styles/global.css) owns the centered 56rem content maximum, responsive horizontal gutters, and page spacing. The header remains inside this wrapper, matching the existing page structure. Settings keeps its internal navigation and panel layout: one column below 48rem and a fixed navigation column beside flexible content above that breakpoint. Each active view still has one `main` landmark.

Start the application using the [development server instructions](#development-server), and run the frontend commands under [Testing and checks](#testing-and-checks). For a layout check, inspect Home and Settings at about 320px and 900px, navigate in both directions with the keyboard, and confirm the destination heading receives focus. At the narrow width, Settings sections should stack; at the wider width, its navigation and panel should sit side by side. The page should not scroll horizontally. A viewport-width check is not a substitute for a literal 200% browser zoom check.

Troubleshooting:

- **Horizontal overflow:** inspect long content inside `.page-container`, ensure flex/grid children can shrink with `min-width: 0`, and compare `document.documentElement.scrollWidth` with the viewport width. Keep page width and gutters in `.page-container` instead of adding a second page shell.
- **Settings navigation or panel crowds the content:** check the 48rem `.settings-layout` breakpoint in `src/styles/global.css`; the single-column layout is intentional below it.
- **Page spacing differs between Home and Settings:** check that both still render `PageContainer` and remove any duplicate outer width or gutter rule added to a page-specific wrapper.

See [Task 28 frontend evidence](evidence/frontend-page-container-task-28.md) for implementation details, verification results, and remaining accessibility checks. This frontend Task 28 is separate from repository task T-028 (the bounded SPSC buffer); `docs/TASKS.md` and AC-11 completion status were not changed.

## Task 29 — Reusable Settings section

[`src/app/SettingsSection.tsx`](src/app/SettingsSection.tsx) owns the active Settings panel wrapper, heading, optional description, and child content. `App.tsx` supplies the selected section's existing copy and controls; only one tab panel is mounted at a time. Panel borders, surface, padding, heading, and description styles remain in the existing `.settings-panel` and `.settings-panel-heading` rules in [`src/styles/global.css`](src/styles/global.css), so this refactor does not add new styling rules.

Run the preview and frontend checks with the existing [development server](#development-server) and [testing commands](#testing-and-checks). For visual verification, open Settings, inspect General and each tab through About & Diagnostics, and check the layout at 320px and 900px. Use the arrow keys within the section tabs, confirm visible focus, and verify the Launch at Login preview value survives Home → Settings navigation. Check literal 200% browser zoom separately when available.

Troubleshooting:

- **The panel has no accessible name:** verify `SettingsSection`'s `aria-labelledby` points to its visible `h2` ID, and that the tab's `aria-controls` still points to the active panel ID.
- **Panel spacing or surface changes:** inspect the shared `.settings-panel` and `.settings-panel-heading` rules instead of adding styles to individual section content.
- **Keyboard section navigation or focus fails:** check the tab IDs and `aria-controls` in `App.tsx`, the arrow-key handler, and the active tab's `tabIndex`; keep the panel heading hierarchy at `h2` under the Settings `h1`.
- **Home/Settings focus or preview state changes:** check `navigateTo`, destination heading focus, and the in-memory state in `App.tsx` before changing panel rendering.

See [Task 29 frontend evidence](evidence/frontend-settings-section-task-29.md) for test results, visual checks, and remaining accessibility limitations. This frontend Task 29 does not mark AC-11 complete.

## Task 30 — Reusable Settings row

[`src/app/SettingsRow.tsx`](src/app/SettingsRow.tsx) presents a setting title and optional description beside its control. General Settings uses it for Launch at Login; `App.tsx` still owns the preview boolean in React memory, so it survives view and tab navigation and resets on refresh. The row renders visible text IDs supplied by its owner; controls use `aria-labelledby` and, when applicable, `aria-describedby` to preserve their accessible name and description without wrapping a non-labelable control in `<label>`.

Run the frontend with `npm run dev` and open General Settings. Verify the title, description, switch alignment, keyboard focus and Space activation at about 320px and 900px; confirm there is no horizontal overflow. Check literal 200% browser zoom separately when available. Run the frontend commands in [Testing and checks](#testing-and-checks), including `npm run test -- --run`.

Troubleshooting:

- **Row copy or switch alignment changes:** inspect `.settings-row`, `.settings-row-copy`, and `.settings-field-control` in `src/styles/global.css` and compare them with the existing spacing tokens.
- **The switch loses its accessible name or description:** keep its `aria-labelledby` and `aria-describedby` IDs aligned with `SettingsRow`'s `titleId` and `descriptionId`.
- **Focus or switching regresses:** check the switch's native button, Space key handling, and state ownership in `src/app/App.tsx`; do not move the preview value into `SettingsRow`.

See [Task 30 frontend evidence](evidence/frontend-settings-row-task-30.md) for exact verification results and any checks not run. This task does not mark AC-11 complete.

## Reusable SettingsSwitch (frontend Task 31)

[`src/app/SettingsSwitch.tsx`](src/app/SettingsSwitch.tsx) is a controlled native-button switch for an immediate binary preference. It accepts `checked`, `onCheckedChange`, `labelledBy`, and optional `describedBy` props; it keeps no internal value and does not persist settings or call native APIs. Compose it inside [`SettingsRow`](src/app/SettingsRow.tsx): the row owns the title and description IDs, and the switch references those IDs for its accessible name and description. `App.tsx` owns the Launch at Login preview value in React memory, so it survives Home/Settings and Settings-section navigation and resets to Off after refresh.

Run the preview with `npm run dev`, open General Settings, and inspect the row and switch at about 320px and 900px. Check mouse activation, Space and Enter keyboard activation, visible focus, state across navigation, and reset after refresh. A viewport-width check is not a substitute for literal 200% browser zoom.

Run the focused component test with `npm run test -- --run src/app/SettingsSwitch.test.tsx`, or all frontend tests with `npm run test -- --run`. For accessible-name regressions, confirm `labelledBy` matches the SettingsRow `titleId`; for description regressions, confirm `describedBy` includes the row description ID; for keyboard regressions, preserve native button activation and the Space key repeat guard. For appearance regressions, inspect `.settings-switch`, `.settings-switch-track`, and `.settings-switch-thumb` in `src/styles/global.css` and retain the existing design tokens and focus ring.

See [frontend Task 31 evidence](evidence/frontend-settings-switch-task-31.md) for exact verification results and checks not run. This task does not mark AC-11 complete.

## Task 32 — Reusable Settings status message

[`src/app/SettingsStatusMessage.tsx`](src/app/SettingsStatusMessage.tsx) presents a concise title and explanatory React children in a neutral Settings message. Its props are `title: string` and `children: ReactNode`. It is static explanatory copy: it has no state, native calls, controls, alert role, or live-region announcement. The Microphone preview uses it to explain that native selection is not connected, no audio is captured, device availability and permissions have not been checked, and input selection needs future native integration.

Run the preview with `npm run dev`, open Settings → Microphone, and inspect the message at about 320px and 900px. Confirm that the copy wraps without horizontal scrolling, other sections remain available, and keyboard navigation still moves between tabs and returns focus to the destination heading when switching Home/Settings. In General, toggle Launch at Login and confirm it retains its in-memory state when navigating between Settings sections. A viewport-width check is not a substitute for literal 200% browser zoom; test screen-reader behavior separately with VoiceOver or another supported reader.

Run the frontend checks from the repository root:

```sh
npm run typecheck
npm run test -- --run
npm run lint
npm run build
```

Troubleshooting:

- **Message text overflows:** inspect `.settings-status-message` and `.settings-status-message-description` in `src/styles/global.css`; keep the message shrinkable and allow long text to wrap.
- **The title or explanation is missing from the accessibility tree:** preserve the level-three heading and paragraph markup. Keep static copy out of `role="alert"` and live regions unless it becomes a newly changing runtime status.
- **The preview implies a real microphone state:** keep the text explicit that this development preview does not connect native microphone selection or check device availability and permissions.
- **Settings focus or Launch at Login state regresses:** check the navigation and state ownership in `src/app/App.tsx` and rerun the App integration tests.

See [frontend Task 32 evidence](evidence/frontend-settings-status-task-32.md) for exact results and checks not run. This task does not mark AC-11 complete.

## Task 33 — Shortcuts status message

Settings → Shortcuts reuses [`SettingsStatusMessage`](src/app/SettingsStatusMessage.tsx), matching the Microphone preview's existing static heading, explanation, and neutral styling. It explains that global keyboard shortcuts are not connected in this development preview, that no system-wide shortcuts are registered or monitored, and that configuration needs future native integration. The preview does not record shortcuts, check keyboard permissions, or install native hooks. No shortcut-related acceptance criterion or AC-11 is complete.

Run `npm run dev` and open Settings → Shortcuts. Compare its message with Settings → Microphone, visit all six Settings sections, and use the section arrow keys plus Tab/Enter to check navigation and visible focus. In General, turn Launch at Login on, navigate away and back, and confirm it remains on. Inspect around 320px and 900px for readable wrapping and no horizontal overflow; check literal 200% zoom separately from viewport resizing. Run all frontend checks listed under [Testing and checks](#testing-and-checks), including `npm run test -- --run`.

Troubleshooting:

- **Shortcuts shows the generic placeholder:** check the `shortcuts` branch in `src/app/App.tsx` and keep the existing section heading and description.
- **Message spacing or colors differ from Microphone:** reuse the `.settings-status-message` rules in `src/styles/global.css`; do not add per-section styles.
- **The message is announced as an alert or becomes keyboard-focusable:** preserve its normal heading and paragraph in `SettingsStatusMessage`; do not add alert/live-region roles, controls, or `tabIndex`.
- **Tab focus or Launch at Login state changes:** check the existing tab handler, destination heading focus, and app-owned in-memory state in `src/app/App.tsx`.

See [frontend Task 33 evidence](evidence/frontend-shortcuts-status-task-33.md) for exact verification results and checks not run.

## Task 34 — Text cleanup mode preview

Settings → Text & Vocabulary offers **Clean** (the intended default, to remove supported filler words and apply deterministic cleanup) and **Verbatim** (intended to preserve filler words), as defined by PRD F-07. The selector is a frontend-only development preview held in React memory: it survives Settings-section and Home/Settings navigation, resets to Clean after refresh, and is not connected to dictation or text processing. The Custom Vocabulary draft form is documented in Task 35; it is not saved or applied.

Run `npm run dev`, open Settings → Text & Vocabulary, and use the mouse or radio arrow keys to choose each option. Confirm the selected radio and focus are visible, visit another Settings section and Home, then return to confirm the selection remains. Refresh and confirm Clean is selected again. Check the layout around 320px and 900px when viewport resizing is available; perform a separate literal 200% zoom check when available.

Run `npm run typecheck`, `npm run test -- --run`, `npm run lint`, and `npm run build` from the repository root. Troubleshooting: if the option label or description is missing to assistive technology, preserve the fieldset/legend, native radio labels, and description IDs in [`TextCleanupModeSelector.tsx`](src/app/TextCleanupModeSelector.tsx); if selection resets during navigation, keep its state in `App.tsx`. This preview does not complete AC-07, AC-08, AC-11, T-035, T-038, or T-053. See [Task 34 evidence](evidence/frontend-text-cleanup-mode-task-34.md) for verification and remaining checks.

## Task 35 — Custom vocabulary entry form preview

Settings → Text & Vocabulary now has **Spoken phrase** and **Written replacement** text fields after the Clean/Verbatim selector. Each field accepts Unicode text, including accented characters, emoji, and combining sequences. After a field is touched, validation reports empty or whitespace-only input, invalid unpaired UTF-16 surrogates, and values longer than 128 Unicode scalar values. Values are not normalized or truncated. The draft stays in React memory while switching Settings sections or navigating Home/Settings, and resets after a refresh or fresh app mount.

This is a development preview only: valid pairs can be added to and removed from a React-memory list capped at 500 entries. The list survives in-app navigation and clears after reload; it is not persisted, exported, duplicate-checked, connected to native settings, or applied to dictated text. This preview does not complete production T-038 or AC-08.

Run `npm run dev`, open Settings → Text & Vocabulary, and type or paste into both fields. Blur an empty field to see its error, then correct it and confirm the error clears. Check the layout around 320px and 900px, visible keyboard focus, switching sections and Home/Settings, then refresh to confirm the draft resets. A viewport-width check is not a substitute for literal 200% browser zoom.

Run the frontend checks from the repository root:

```sh
npm run typecheck
npm run test -- --run
npm run lint
npm run build
```

Troubleshooting:

- **A label or helper text is missing to assistive technology:** keep the native `<label>` associated with its input and preserve the description/error IDs in `src/app/VocabularyEntryForm.tsx`.
- **A field stays invalid after correction:** check that `App.tsx` passes the updated controlled draft back through the component's `value` prop.
- **A phrase is rejected at the boundary:** the limit counts Unicode scalar values, not UTF-16 code units or grapheme clusters; one emoji counts as one scalar, and `e` plus a combining accent counts as two.
- **Draft data survives a full refresh:** inspect `App.tsx` for accidental persistence; this preview stores no vocabulary in localStorage, sessionStorage, files, or a backend.

See [Task 35 evidence](evidence/frontend-vocabulary-form-task-35.md) for exact verification results and checks not run. This task does not mark AC-08, AC-11, T-038, T-039, or T-053 complete.

## Task 36 — Live vocabulary draft preview

[`VocabularyDraftPreview`](src/app/VocabularyDraftPreview.tsx) receives the same `VocabularyDraft` value as [`VocabularyEntryForm`](src/app/VocabularyEntryForm.tsx). It reuses the form's `validateVocabularyValue` function and has no draft state of its own. For a valid pair, it interpolates the entered phrases into the fixed synthetic sentences `I mentioned “<spoken phrase>” today.` and `I mentioned “<written replacement>” today.` React renders the values as text. The example is not produced by a replacement engine and does not read transcripts, save entries, or process speech.

Both empty fields show a neutral prompt; a missing field asks for both values. Whitespace-only, overlength, or invalid-Unicode values hide the completed example until corrected. The form continues to show its existing field validation after blur. Draft and preview state last only for the current app mount and reset after refresh.

Run `npm run dev`, open Settings → Text & Vocabulary, and enter both values. Edit each field, try an empty or invalid value, then correct it. Confirm the example updates, wraps at about 320px and 900px, renders HTML-like and Unicode text literally, and offers no Save or Apply action. Check keyboard focus and Settings/Home navigation. Test literal 200% browser zoom separately from viewport resizing.

Run the frontend checks with `npm run typecheck`, `npm run test -- --run`, `npm run lint`, and `npm run build`. Troubleshooting: if the example is hidden, check both fields against the shared validator; if it does not update, confirm `App.tsx` passes the current draft as the preview's `value`; if markup appears interpreted, keep draft text in normal React children. This display-only feature does not complete AC-08, AC-11, T-038, T-039, or T-053. See [Task 36 evidence](evidence/frontend-vocabulary-preview-task-36.md) for results and checks not run.

## Task 37 — Clear vocabulary draft

The **Clear draft** button is enabled while either vocabulary field contains any text, including whitespace or an invalid value, and disabled when both fields are empty. It resets both in-memory fields and their validation presentation, returns the synthetic preview to its empty guidance, and moves focus to **Spoken phrase**. Clean/Verbatim, Launch at Login, and other Settings state are unaffected. This form is an unsaved frontend preview with no connection to settings storage, so clearing it cannot save, modify, or delete any saved vocabulary.

Run `npm run dev`, open Settings → Text & Vocabulary, enter a draft, and activate Clear draft with a mouse, Enter, or Space. Verify the empty fields, preview guidance, and focus return; blur an invalid field first to check that its error clears. Check approximately 320px and 900px widths, then navigate between Settings sections and Home to confirm other Settings state remains intact.

Run the frontend checks with `npm run typecheck`, `npm run test -- --run`, `npm run lint`, and `npm run build`. Troubleshooting: if errors remain after clearing, confirm the form resets its touched state and App clears the controlled draft; if focus is lost, keep focus directed to the Spoken phrase input before the now-empty button is disabled; if the button state is wrong, check that it uses raw field lengths so whitespace and invalid text still enable it.

See [Task 37 evidence](evidence/frontend-vocabulary-clear-task-37.md) for test results and accessibility checks not run. AC-08, AC-11, T-038, T-039, and T-053 remain incomplete.

## Task 38 — Privacy Settings overview

[`src/app/PrivacyOverview.tsx`](src/app/PrivacyOverview.tsx) provides four read-only disclosures in Settings → Privacy: intended offline processing; content the product requirements prohibit retaining; the distinction between local settings/vocabulary and speech content; and clipboard, operating-system, and update boundaries. It reuses `SettingsStatusMessage` to state that native recording, offline recognition, clipboard behavior, network isolation, and retention safeguards are not fully implemented and verified. The disclosures describe requirements, not completed or audited protections.

The current React preview does not record audio or recognize speech. Launch at Login and Clean/Verbatim are in-memory values, the vocabulary draft is unsaved, and a refresh resets them. No privacy setting, persistence, network monitoring, clipboard operation, update check, or diagnostics export is added. The Rust settings schema and writer described above remain separate from this frontend.

Open the frontend using the [development server instructions](#development-server), then choose **Settings → Privacy**. Check all six Settings tabs, switch to another section and back, and confirm the active panel remains linked to its selected tab. Test the layout at approximately 320px and 900px, check keyboard navigation and Home/Settings heading focus, and look for horizontal overflow or clipped text. Literal 200% zoom and screen-reader checks must be performed separately; resizing the viewport does not substitute for zoom.

Run the frontend checks from the repository root:

```sh
npm run typecheck
npm run test -- --run
npm run lint
npm run build
```

Troubleshooting:

- **Privacy is still a placeholder:** check that `App.tsx` selects `PrivacyOverview` for the existing `privacy` section and keeps `SettingsSection` as the panel wrapper.
- **Text clips or overflows:** inspect the `.privacy-overview*` rules in `src/styles/global.css`; retain `min-width: 0`, wrapping, and the shared Settings panel gutter.
- **The panel heading, tab, or destination focus is wrong:** check the existing tab IDs and `aria-controls` plus the Home/Settings heading refs in `src/app/App.tsx`; Privacy must not add a second Settings heading.
- **Preview values change after visiting Privacy:** keep Launch at Login, cleanup mode, and vocabulary draft state owned by `App.tsx`; do not add storage to the informational component.

Task 38 verification passed frontend typecheck, lint, build, and all 122 tests; the 320px/900px overflow and keyboard navigation checks also passed. Literal 200% zoom and screen-reader checks are **NOT RUN**; see the evidence file below.

See [Task 38 evidence](evidence/frontend-privacy-overview-task-38.md) for commands, visual results, and accessibility checks not run. T-061, T-062, T-063, T-077, AC-11, and AC-12 remain incomplete; this frontend page does not verify native privacy safeguards.

## Task 39 — About & Diagnostics overview

[`src/app/AboutDiagnosticsOverview.tsx`](src/app/AboutDiagnosticsOverview.tsx) adds a static **About & Diagnostics** overview inside the existing Settings panel. It describes Outflow and identifies the current interface as a development preview, lists the troubleshooting cases the finished application is intended to handle, explains future diagnostic-export fields and privacy exclusions, and summarizes the planned explicit signed full-application update flow.

The page does not show a version. `package.json` contains `0.1.0`, but that is frontend package metadata, not verified native release metadata. The page does not claim a native, model, or runtime version. Troubleshooting, diagnostic collection/export, and update checking are described as planned capabilities; they are not implemented or detected by this preview. No diagnostic report can be generated here, and update checking is not implemented. The overview itself does not initiate network requests, invoke native commands, create files, or access persistent storage, and it collects no device or diagnostic data. It offers no Export diagnostics or Check for Updates control.

Open the frontend with the [development server instructions](#development-server), choose **Settings → About & Diagnostics**, and use the existing Settings tabs to return to another section. The page shares the existing neutral Settings styles and status message. All six sections and existing Settings state remain owned by `App.tsx`.

Run frontend verification from the repository root:

```sh
npm run typecheck
npm run test -- --run
npm run lint
npm run build
```

For manual verification, open the local Vite URL, navigate to About & Diagnostics, read each section, and confirm there are no export or update controls or fabricated runtime values. Use the Settings tabs and Home/Settings navigation with the keyboard; confirm the destination page heading receives focus and Launch at Login, Clean/Verbatim, and vocabulary drafts remain unchanged. Inspect about 320px and 900px widths for readable wrapping and horizontal overflow. Test literal 200% browser zoom and a screen reader separately; resizing the viewport is not a substitute.

Troubleshooting:

- **The tab still shows a placeholder:** check the `about-diagnostics` branch in `src/app/App.tsx` and keep `SettingsSection` as the only panel heading wrapper.
- **Text clips or overflows:** inspect the shared `.privacy-overview*`, `.settings-preview-note`, and `.settings-status-message` rules in `src/styles/global.css`; keep the Settings content min-width and text wrapping behavior.
- **Navigation or preview values change:** check the existing tab handler, destination-heading focus effect, and state ownership in `src/app/App.tsx`; the overview itself should remain presentational.
- **An export or update action appears:** remove it from this static overview. Diagnostic export and update checking remain native work for their own tasks.

This frontend page does not complete repository T-039 (vocabulary hints and post-correction), T-019, T-064, T-072, T-077, AC-11, or AC-13. The task ledger remains unchanged. See [frontend Task 39 evidence](evidence/frontend-about-diagnostics-task-39.md) for exact verification results, visual checks, and remaining accessibility checks.

## Task 40 — Settings UI quality verification

Start the preview using the existing [development server instructions](#development-server), then open **Settings** and inspect General, Microphone, Shortcuts, Text & Vocabulary, Privacy, and About & Diagnostics. The preview switch, cleanup mode, and vocabulary draft are in-memory only; section and Home/Settings navigation preserve them, while refreshing restores Off, Clean, and an empty draft. Microphone and global shortcuts are unavailable, Privacy describes requirements separately from verified protections, and diagnostics export/update checking are not implemented.

For a keyboard walkthrough, use Tab to reach Settings, Enter or Space to open it, and Up/Down arrows to move through the vertical Settings tabs. Tab enters the selected panel; Shift+Tab returns to the preceding control. Check that Home/Settings navigation focuses its destination heading and that keyboard focus remains visible. On Text & Vocabulary, use the radio arrow keys, enter both phrases, review the synthetic preview, and use Clear draft to check error reset and focus return.

Use the browser's responsive viewport controls to inspect Home and all six panels at **320×900** and **900×900 CSS px**. Check wrapping, controls near the viewport edges, vertical scrolling, and horizontal overflow; the browser console can compare `document.documentElement.scrollWidth` with `document.documentElement.clientWidth`. A 320px viewport check does not replace literal 200% browser zoom.

Literal 200% zoom, increased text spacing, reduced-motion and forced-colors emulation, and VoiceOver/screen-reader operation remain **NOT RUN**. No WCAG 2.2 AA claim is made; AC-11 and native/release acceptance criteria remain incomplete. See the [Task 40 QA report](evidence/frontend-settings-quality-task-40.md) for the verification matrix and exact command results. For layout issues, inspect page gutters, `min-width: 0`, and scroll widths; for panel or focus issues, check the active tab ID, `aria-controls`, panel `aria-labelledby`, and destination heading refs in `src/app/App.tsx` and `src/app/SettingsSection.tsx`.

## Focused Task 41 — Rust custom vocabulary validation

`outflow-core` exposes `outflow_core::vocabulary::validate_vocabulary_entry(spoken, written)`. It returns `Result<(), VocabularyValidationError>`; the error identifies `VocabularyField::Spoken` or `Written` and whether the value was `Empty` or `TooLong`. Each field must contain a non-whitespace Unicode scalar and may contain at most 128 scalars. The validator uses Rust `str::chars()`: a multibyte emoji counts as one scalar, while `e` plus a combining accent counts as two. This per-entry validator does not trim, normalize, case-fold, or rewrite accepted text.

Rust `&str` is valid UTF-8 and cannot contain unpaired surrogates. A JavaScript UTF-16 string with malformed surrogate code units must be rejected during conversion to Rust text, such as with `String::from_utf16`, before calling this API. FR-08.4 also calls for rejecting pathological input, but the repository defines no embedded-control policy; that rule remains unresolved and this validator adds none. The function is not connected to the React form or settings schema. Vocabulary CRUD, persistence, editing/deletion, and text replacement remain separate work. This does not complete AC-08 or repository T-038.

Run the focused and workspace Rust tests and the synthetic demo from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_validation --locked
```

The focused tests cover empty and Unicode whitespace-only fields, field/reason reporting, both-field ASCII and emoji boundaries, accented text, combining marks, unchanged accepted values, redacted errors, and deterministic results. The full workspace run passed **87 Rust tests** (15 added; all 72 previously existing tests remain passing). Frontend checks passed with 136 tests, and the Python suite passed 226 tests. See [Task 41 evidence](evidence/rust-vocabulary-validation-task-41.md) for all commands and results.

Expected Task 41 demo output (historical snapshot):

```text
Outflow vocabulary validation demo
Valid vocabulary mapping: PASS
Empty spoken phrase: REJECTED
Overlong written replacement: REJECTED
Unicode scalar boundary: PASS
Vocabulary persistence: NOT IMPLEMENTED
Text replacement: NOT IMPLEMENTED
```

If a phrase is rejected at the length boundary, count Unicode scalar values: each emoji counts once, and a base character plus combining mark counts twice. If malformed UTF-16 is involved, validate the UTF-16-to-UTF-8 conversion before calling Rust; such malformed data cannot be passed as `&str`. The repository task ledger already uses T-041 for the onboarding permission checklist, so this focused request does not change that entry or any task status. T-017 and T-018 remain incomplete, and their prerequisite work for T-038 is still outstanding.

## Vocabulary collection validation

`outflow_core::vocabulary::validate_vocabulary_collection(&[VocabularyEntryRef])` validates a borrowed, read-only collection and returns `Result<(), VocabularyCollectionError>`. It accepts zero through `MAX_VOCABULARY_ENTRIES` (500) entries and rejects a larger collection with its supplied count and the maximum. Each entry is checked by the existing `validate_vocabulary_entry`, preserving its nonempty and 128 Unicode scalar-value rules. Validation errors report the size, first invalid entry, or first duplicate pair without including phrase text. Validation order remains size, per-entry checks in input order, then duplicate checks in input order.

The `outflow-core` collection validator uses the versioned Unicode 18.0.0 canonical-caseless key documented below. Written replacements do not determine uniqueness. Original text remains unchanged. The core collection validator does not persist entries or perform transcript replacement or word-boundary matching. The existing `outflow-settings` schema validator remains lowercase-only; wiring it to this policy is a separate C-08 integration task. The initial collection work and its verification remain documented in [Task 42 evidence](evidence/rust-vocabulary-collection-task-42.md); current Unicode matching implementation and verification are in [Task 45 evidence](evidence/rust-vocabulary-icu4x-task-45.md).

Run the focused tests and synthetic example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_validation --locked
```

The example reports validation outcomes from actual calls. Its output is:

```text
Outflow vocabulary validation demo
Matching policy: Unicode 18.0.0 canonical caseless V1
Valid vocabulary mapping: PASS
Empty spoken phrase rejected: PASS
Overlong written replacement rejected: PASS
500-entry limit accepts 500: PASS
500-entry limit rejects 501: PASS
ASCII case-insensitive duplicate: PASS
German sharp-s full-fold duplicate: PASS
Canonical accent duplicate: PASS
Original vocabulary text preserved: PASS
Vocabulary persistence: NOT IMPLEMENTED
Text replacement: NOT IMPLEMENTED
```

Task 45 does not complete AC-08, T-017, T-018, T-038, or T-039; those remain incomplete.

## Vocabulary Unicode matching policy

The Rust collection validator now uses versioned Unicode 18.0.0 canonical-caseless matching: `NFD(toCasefold(NFD(input)))`, following D145. It uses ICU4X 2.4.0 compiled data with full default folding. Keys are temporary and do not change original vocabulary text. Matching does not trim whitespace, strip accents, apply NFKC, or perform transcript replacement. See the [policy decision record](docs/VOCABULARY_UNICODE_POLICY.md) for the comparison matrix, limits, and migration considerations.

Run the standard-library reference demonstration and its focused tests:

```sh
python3 scripts/check_vocabulary_unicode_policy.py --demo
python3 -m unittest scripts.tests.test_check_vocabulary_unicode_policy -v
```

The Python demonstration is a development reference only: it reports the running Python Unicode data version and does not supply the production Unicode 18.0.0 data. Task 45 replaces the former lowercase-only Rust comparator with ICU4X 2.4.0 and adds matching regression tests. This does not implement vocabulary persistence or text replacement. T-017, T-018, T-038, T-039, and AC-08 remain incomplete.

## Focused Task 45 — ICU4X vocabulary matching

The core crate pins `icu_casemap = "=2.4.0"` and `icu_normalizer = "=2.4.0"`, both with `default-features = false` and `compiled_data`. The versioned API derives keys using Unicode 18.0.0 D145: `NFD(toCasefold(NFD(input)))`. It uses full default, non-Turkic folding and preserves original values. The verified ASCII fast path is tested against ICU for every pair of ASCII scalars. The existing settings-schema validator still uses lowercase-only matching; reconciling that stored-settings path is a separate task. No vocabulary collection persistence, CRUD, transcript replacement, or frontend integration is included here.

Run the release benchmark with synthetic data:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_matching_benchmark --release --locked
```

On macOS 26.6.2 arm64 with Rust 1.99.1, the paired release benchmark measured:

| Metric | Lowercase reference | ICU4X | Change |
|---|---:|---:|---:|
| ASCII key generation | 40.20 ns | 40.26 ns | +0.15% |
| Multilingual key generation | 541.03 ns | 1,528.89 ns | +182.59% (2.83×) |
| 500-entry ASCII validation | 44,933.33 ns | 43,420.62 ns | −3.37% |
| 500-entry multilingual validation | 202,558.33 ns | 642,786.88 ns | +217.33% (3.17×) |

The pre-change `vocabulary_validation` release executable was **457,104 bytes**; the same target after ICU4X integration is **564,192 bytes** (+107,088 bytes, 23.43%). Synthetic benchmark-process peak RSS changed from **2,228,224 bytes** to **2,293,760 bytes** (+65,536 bytes, 2.94%). These are example measurements, not packaged app or aggregate process-tree release results. The Task 45 evidence records the separately captured pre-edit baseline, all benchmark workloads, test results, dependency graph, license review, and limitations. The release gates remain unmeasured.

## Focused Task 44 — Rust Unicode dependency audit

The read-only Rust dependency audit checks whether the investigated Unicode matching crates are direct `outflow-core` dependencies, appear in the lockfile, and are explicitly approved in project documentation. It distinguishes direct use from transitive or unrelated lockfile presence; a locked crate is not proof that `outflow-core` can use its API or that it conforms to D145.

Run the audit from the repository root with Python 3.11 or later (for standard-library `tomllib`):

```sh
python3 scripts/audit_vocabulary_unicode_rust_dependencies.py --repo .
```

The concise report shows the pre-Task 45 workspace/core manifest and lockfile state, lowercase-only production behavior at that time, the proposed D145 policy, and each candidate's direct dependency and approval status. Task 45 records the approved ICU4X choice, resolved feature graph, licenses, and measurements in [Rust Unicode matching dependency options](docs/VOCABULARY_UNICODE_RUST_OPTIONS.md) and [Task 45 evidence](evidence/rust-vocabulary-icu4x-task-45.md).

The Task 44 audit is a historical pre-integration snapshot: it reported that no candidate was a direct dependency and that the policy needed a decision. The maintainer's Task 45 approval supersedes that recommendation. Reproduce the read-only audit and its Python test coverage with:

```sh
python3 scripts/audit_vocabulary_unicode_rust_dependencies.py --repo .
python3 -m unittest scripts.tests.test_audit_vocabulary_unicode_rust_dependencies -v
```

If the report says a workspace/core manifest is missing or malformed, run it from the repository root and check `Cargo.toml` and `crates/core/Cargo.toml`; the audit does not create or repair them. If `Cargo.lock` is absent or malformed, restore or repair it through the normal maintainer workflow before relying on lockfile findings. The script makes no network request and does not need crate source access; source/API/version/license claims belong to the linked candidate evaluation. If a primary source is unavailable, treat that finding as UNVERIFIED and do not infer conformance from the package name. T-017, T-018, T-038, T-039, and AC-08 remain incomplete; repository T-044 remains the Windows insertion adapter task.

## Vocabulary matching migration compatibility checker

`outflow_core::vocabulary_migration::inspect_vocabulary_migration` is a pure, read-only checker for conflicts between legacy Rust `str::to_lowercase()` matching and Unicode 18.0.0 canonical-caseless V1. It validates the 500-entry and 128-scalar limits, compares entries without changing them, and reports conflict indices, classifications, and policy identifiers without printing phrase text. Pairs such as `straße` / `STRASSE`, composed `é` / decomposed `e` plus combining acute, and `σ` / `ς` are new Unicode 18 conflicts. Reviewers must resolve reported conflicts manually before adopting the newer policy for a collection; the checker never chooses or deletes an entry.

Run its synthetic compatibility example and focused tests from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_migration_check --locked
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary_migration --locked
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary --locked
```

The settings crate still validates vocabulary uniqueness with lowercase-only matching. Its current schema v1 has no matching-policy field, rejects unsupported schema versions, and has no migration implementation; schema version alone or an absent policy field is not proof of which application build last validated a file. Before persistent integration, establish trusted schema/build lineage or add an explicitly versioned policy marker through an approved schema change, define safe review and rollback behavior, and connect the checker to settings loading/saving. See [Vocabulary Matching Migration Compatibility](docs/VOCABULARY_MATCHING_MIGRATION.md) for the boundary and evidence. This checker does not complete T-017, T-018, T-038, T-039, or AC-08; repository T-046 remains the copy-only result panel task.

Task 46 verification snapshot: 148 Rust tests passed (24 migration-checker tests added; the 124-test pre-task baseline remained passing), along with the requested Rust checks/build, 136 frontend tests plus typecheck/lint/build, and 258 Python tests. Task 47 verification is recorded below and in [Task 47 evidence](evidence/rust-vocabulary-readiness-task-47.md).

## Vocabulary migration readiness assessment (Task 47)

`outflow_core::vocabulary_migration::assess_vocabulary_migration_readiness(entries, source_policy)` reuses the read-only conflict checker and returns a `VocabularyMigrationReadiness` outcome or the checker's typed size/entry validation error. The `VocabularySourcePolicy` argument is explicit provenance: use `Unknown` when evidence is absent or untrusted, and select a verified variant only when trusted evidence establishes the policy. Settings schema v1 has no matching-policy marker, so its version alone cannot prove either policy.

The outcomes distinguish an empty collection, unknown source policy, conflicts requiring manual resolution, verified legacy data compatible with an explicit future migration, and verified current data that is already current. Conflicts carry input indices and legacy-versus-Unicode-18 classifications. No outcome selects a winner, changes vocabulary text, or writes settings. In particular, compatibility does not mean a migration was applied or persisted; `EmptyCollection` also says nothing about rewriting a complete settings file.

Run the focused tests and synthetic example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core vocabulary_migration --locked
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example vocabulary_migration_check --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
```

The focused suite covers unknown provenance with and without conflicts, all readiness decisions, Unicode collisions, Turkish dotted/dotless-I and cross-script distinctions, typed validation errors, the 500-entry limit, stable conflict indices, text preservation, repeatability, and the absence of a persistence input. The example prints readiness outcomes generated by the assessment API and uses synthetic values only.

Settings load/save migration integration remains unimplemented: schema v1 has no matching-policy marker or migration path, and `crates/settings` still validates vocabulary uniqueness with lowercase-only matching. This settings validator is **not Unicode 18-compatible**. Task 48 adds a separate opt-in adapter for already-constructed settings values; it is not called by decoding, loading, initialization, validation, or saving. Before connecting readiness to those paths, establish trustworthy source provenance, define explicit conflict review, approve a versioned schema migration, and verify atomic write/rollback behavior. T-017, T-018, T-038, T-039, and AC-08 remain incomplete.

Troubleshooting:

- **`SourcePolicyUnknown`:** provide trusted policy provenance to the assessment. Schema v1 alone is insufficient, even when the checker reports no conflicts.
- **`ManualResolutionRequired`:** review both entries at each reported index and its classification. Keep both original mappings until a user explicitly resolves the conflict.
- **`TooManyEntries` or `InvalidEntry`:** check the reported count/index and typed field reason; the checker validates collection size first, then entries in input order.
- **`EmptyCollection`:** no vocabulary-key migration is needed, but this does not establish that other settings can be rewritten safely.

Task 47 adds a pure readiness assessment only; no settings files, schema fields, or persisted values are changed. Exact verification and the current settings limitations are recorded in [Task 47 evidence](evidence/rust-vocabulary-readiness-task-47.md).

## Settings vocabulary compatibility preflight (Task 48)

`outflow_settings::preflight_vocabulary_compatibility(&settings, source_policy)` is an opt-in, read-only adapter over an in-memory `Settings` DTO. Settings stores vocabulary as an ordered `Vec<VocabularyEntry>` with owned `id`, `spoken`, and `written` strings. The adapter creates temporary borrowed `outflow_core::vocabulary::VocabularyEntryRef` values from each spoken/written pair and calls `assess_vocabulary_migration_readiness`; it does not clone phrases, retain comparison keys, or alter any settings field. The existing typed core readiness, conflict, and validation result types are re-exported by `outflow-settings`.

Callers must supply trusted `VocabularySourcePolicy` provenance. Schema v1 has no matching-policy marker, so schema version alone, file timestamps, application version strings, operating system, and absence of conflicts do not establish policy provenance. For any nonempty collection, `Unknown` returns `SourcePolicyUnknown` even when there are no conflicts. `CompatibleForExplicitMigration` means only that verified legacy entries have no conflicts under Unicode 18; it does not mean a migration ran or settings were saved. Conflicts retain indices and classification and require manual resolution without automatic winner selection.

The preflight receives an already-constructed DTO and has no file or network interface. Existing `Settings::from_json`, deserialization, and load paths validate first using lowercase-only uniqueness. They can reject malformed entries or existing lowercase conflicts before the preflight can inspect them; the adapter does not bypass or recover rejected files. Schema v1 and all existing serialization, validation, initialization, load, and save behavior remain unchanged. Automatic integration still requires trusted provenance, an approved schema/migration decision, explicit conflict review and safe atomic-write/rollback behavior. T-017, T-018, T-038, T-039, and AC-08 remain incomplete.

Run the settings tests and the synthetic output demonstration from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --locked
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test vocabulary_preflight integration_summary_uses_readiness_results_from_real_preflight_calls --locked -- --nocapture
```

The integration tests exercise sharp-s, composed/decomposed accent, Greek sigma, existing lowercase conflicts, validation boundaries, ordered indices, and the 500-entry cap. They use synthetic settings only. Full implementation details and limitations are recorded in [Task 48 evidence](evidence/rust-settings-vocabulary-preflight-task-48.md).

## Vocabulary policy marker contract (Task 49)

`outflow_settings::VocabularyPolicyMarker` serializes the approved Unicode 18 canonical-caseless policy as the exact string `unicode18-canonical-caseless-v1`. The marker declares Unicode 18.0.0, default full non-Turkic folding, and NFD before and after folding. A declaration in JSON is not trusted provenance; missing or unsupported markers do not imply a legacy lowercase policy. Unsupported identifiers fail closed because their algorithm and collision behavior are unknown; silently substituting lowercase matching could reinterpret saved vocabulary.

The current persisted settings schema remains v1 and has no policy field. A JSON declaration alone cannot prove the policy under which a settings file was validated, so adding a field to v1 would overstate its provenance; a separately approved schema migration is required. The documented schema-v2 fragment is incomplete, omits required Settings fields, and is a proposal/test fixture only; the production DTO, decoder, validator, and loader do not implement it. Review [the migration contract](docs/VOCABULARY_MATCHING_MIGRATION.md) for future marker rules and required migration/rollback steps. Cross-platform writer permissions and rollback acceptance remain incomplete.

Run the focused marker tests and actual-parser summary with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings vocabulary_policy_marker --locked -- --nocapture
```

No persistent migration or automatic conflict resolution is implemented. T-017, T-018, T-038, T-039, and AC-08 remain incomplete.

## Proposed schema-v2 vocabulary contract fixture (Task 50)

[`crates/settings/tests/vocabulary_schema_v2_contract.rs`](crates/settings/tests/vocabulary_schema_v2_contract.rs) holds a **partial, test-only** envelope for exercising the proposed schema-v2 vocabulary marker and the existing read-only Unicode 18 migration-readiness APIs. Its shape contains only `schema_version`, `vocabulary_matching_policy`, and synthetic `{ "spoken", "written" }` entries; it is not a complete Settings format or a production `SettingsV2` type.

The only accepted marker spelling is the exact, case-sensitive `unicode18-canonical-caseless-v1`. The fixture rejects a missing or empty marker, wrong-case aliases, unsupported policy or Unicode versions, malformed marker JSON types, missing or malformed vocabulary fields, and unsupported schema versions. Marker recognition is only a declaration: it does not establish verified legacy or Unicode 18 provenance. Unknown provenance stays review-required even with no conflicts, and the test supplies verified provenance only as an explicit synthetic assumption.

The read-only checks cover `straße` / `STRASSE`, composed `é` / decomposed `e` plus combining acute, and Greek `σ` / `ς`. They report Unicode 18 conflicts by entry index and preserve both original mappings. The fixture does not write settings or apply a migration. Production schema v1, defaults, serialization, lowercase validation, and load/save behavior remain unchanged; production schema-v2 loading is still unsupported.

Run the focused fixture and print its asserted contract summary with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test vocabulary_schema_v2_contract \
  --locked -- --nocapture
```

Persistent migration still requires an approved complete schema and marker contract, trusted provenance handling, explicit conflict resolution, complete target validation, cross-platform atomic-write permissions, recoverable last-known-good behavior, and interruption/rollback tests. See [the migration contract](docs/VOCABULARY_MATCHING_MIGRATION.md). T-017, T-018, T-038, T-039, and AC-08 remain incomplete; repository T-050 was not changed.

## Read-only vocabulary migration planning report (Task 51)

[crates/settings/tests/vocabulary_schema_v2_migration_plan.rs](crates/settings/tests/vocabulary_schema_v2_migration_plan.rs) adds a pure, test-only planner that combines the proposed schema-v2 marker contract with the existing settings compatibility preflight. It accepts a source decoded through schema v1, a synthetic proposed-v2 JSON document, and an explicit source-policy assumption. The target marker is the exact unicode18-canonical-caseless-v1 declaration; it never determines whether the source was validated with legacy lowercase or Unicode 18 matching.

The report returns a review status and ordered conflict indices/classifications without vocabulary text. Unknown provenance remains review-required; Unicode 18 collisions require manual conflict resolution; verified legacy data without conflicts is only compatible for a future migration review; verified current data is already current for review. Empty vocabulary needs no vocabulary-key migration, but does not authorize a rewrite of the full settings schema. Conflict-free entries are not migration-approved.

Malformed content rejected by the existing schema-v1 decoder cannot reach the planner. The planner reads no user settings or files and performs no write or migration. Production loading remains schema v1 and lowercase-only. Trusted provenance, manual conflict decisions, and atomic-write/rollback verification remain future requirements.

Run the focused tests and printable summary with:

~~~sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test vocabulary_schema_v2_migration_plan \
  --locked -- --nocapture
~~~

The Task 51 fixture does not change repository T-051 (tray and settings navigation), T-017, T-018, T-038, T-039, or AC-08. See [the migration planning details](docs/VOCABULARY_MATCHING_MIGRATION.md#read-only-proposed-migration-planning-report-task-51).

## Read-only vocabulary conflict review groups

The test-only contract groups connected conflict pairs so entries that collide directly or through another entry appear in one review group. Group indices refer to the original vocabulary order; pair classifications are preserved. The report contains no vocabulary text. It does not choose a preferred mapping, delete or edit entries, resolve conflicts, or write settings.

Run the focused test and its asserted synthetic summary from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test vocabulary_conflict_review_contract \
  --locked -- --nocapture
```

Expected demonstration output:

```text
Outflow vocabulary conflict review contract

Single conflict: 1 GROUP
Three connected entries: 1 GROUP
Two independent conflicts: 2 GROUPS
Group ordering: VERIFIED
Conflict classifications: PRESERVED
Original vocabulary: UNCHANGED

Automatic winner selected: NO
Conflict resolution applied: NO
Settings files modified: NO
Production schema-v2 support: NOT IMPLEMENTED
```

Conflict reports are restricted to indices, classifications, group counts, and policy identifiers. Do not put spoken or written values, normalized keys, settings JSON, transcript data, or audio in diagnostic, logging, migration-metadata, or exported error channels. The test-only helper has no production API and does not change the schema-v1 decoder, which continues to reject schema v2.

Future user-facing review still requires a local review interface, explicit decisions, cancellation that preserves all originals, full-collection revalidation, validated vocabulary CRUD for edits or deletion, trusted provenance, an approved atomic migration, and a recoverable original configuration. Those are future requirements; this test contract implements none of those actions. See the [manual review grouping contract](docs/VOCABULARY_MATCHING_MIGRATION.md#manual-conflict-review-grouping-contract-task-52) for the full requirements.

## Prompt 79J — synthetic macOS clipboard diagnostic

The AppKit probe has a development-only `--synthetic-copy-diagnostic` mode for isolating the explicit Copy Result → NSPasteboard → destination-app path without speech. It requires a valid `--verification-id` and rejects timed-fixture or explicit microphone-capture combinations. With the flag absent, the usual microphone and timed-fixture startup paths remain in effect.

The diagnostic uses one fixed source constant, `kSyntheticCopyDiagnosticSample`, displayed in the existing selectable final-result area:

```text
OUTFLOW-SYNTHETIC-COPY-START
Café — plain-text clipboard check! 😀
OUTFLOW-SYNTHETIC-COPY-END
```

This is a test fixture, not recognized speech. Diagnostic startup does not request microphone permission, query microphone authorization, open audio capture, create a recognition worker, or load a model. It creates a session-owned `ResultReady` result through the existing state machine so the ordinary Copy Result button, Return key equivalent, eligibility policy, UTF-8 conversion, `clearContents`, `writeObjects:`, and AppKit result handling remain in use. Startup, Cancel, Dismiss, and shutdown do not write to the clipboard. Dismiss clears the synthetic result and disables Copy Result. The existing clipboard replacement warning stays visible.

### Build and launch

Use the already-authorized pinned Moonshine source/runtime paths described in [Pinned macOS native build and smoke test](#pinned-macos-native-build-and-smoke-test). A fresh output directory is required. The build still links the existing native runtime, but the diagnostic launch needs no model directory and performs no model verification or model load:

```sh
export OUTFLOW_MIC_PROBE_BUILD_DIR="$(mktemp -d /private/tmp/outflow-prompt79j.XXXXXX)"
scripts/build_macos_mic_probe.sh

export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"

scripts/run_macos_mic_probe.sh \
  --verification-id "79J-SYNTH-01" \
  --synthetic-copy-diagnostic
```

The expected window title includes `Outflow Mic Probe — Dev — 79J-SYNTH-01 — PID <pid>`. The view title is **Synthetic clipboard diagnostic**; it says the sample is not recognized speech and the microphone was not requested; there is no partial-recognition area; the fixed three-line sample appears in the selectable final-result area; Copy Result is enabled; and the clipboard-replacement warning remains visible. The verification ID and PID also appear in the window as a noneditable label. Cross-check the PID shown there against the executable inside this fresh bundle before interacting:

```sh
PID_FROM_WINDOW=12345 # replace with the PID shown in the window
ps -p "$PID_FROM_WINDOW" -o pid=,comm=
```

The executable path must resolve to the newly built `OutflowMicProbe.app`. Do not select an older probe window based on its bundle name. The launcher does not inspect the general pasteboard; startup code has no pasteboard access. Automated source-contract tests check that invariant. No automated test reads or writes the user's general clipboard.

### Controlled TextEdit comparison

Before Copy Result, tell the operator that this action replaces the system clipboard and a failed write after clearing it may lose the previous contents. The diagnostic performs no startup copy. Copy Result is the only action in this test that replaces the clipboard.

1. Confirm the exact verification ID and PID, the synthetic diagnostic label, the fixed sample, the enabled button, and the visible clipboard warning.
2. Click Copy Result **once**. Record only whether the UI reports AppKit success or its sanitized failure status.
3. Open a genuinely new, blank, unsaved TextEdit document. Switch it to plain-text mode, verify the document is empty, and put keyboard focus in its text view.
4. Press ⌘V once as a normal operator action. Compare the complete in-memory plain-text document with the fixed sample: exact characters, whitespace, both newline positions, punctuation, accented text, and emoji. Read TextEdit's in-memory value only after confirming the blank target and that TextEdit's text view received the paste. Record only equality, paste occurrence, and sanitized observations; do not log arbitrary TextEdit content.
5. If accessible-text and visual comparison disagree, verify the complete document value, target selection, focus, line-ending representation, Unicode normalization, and text-substitution behavior. Do not label an accessibility mismatch as a clipboard defect without an independent observation.
6. Discard the TextEdit document without saving. Dismiss the result in the diagnostic app and confirm the sample clears and Copy Result becomes disabled.

Classify only the observed result:

| Observation | Classification |
|---|---|
| AppKit success and exact TextEdit equality | Basic desktop clipboard path verified for the synthetic sample |
| AppKit failure | Clipboard writer or host write failure remains |
| AppKit success, no pasted text | Destination delivery, target, focus, or paste problem remains |
| TextEdit receives different synthetic text | Investigate verified transformation, stale item, or wrong target value |
| Visual text matches but accessibility comparison differs | Investigate the inspection/comparison method |
| Desktop interaction cannot be completed | Desktop verification blocked |

A passing synthetic check verifies this fixed sample through this host and does not explain the earlier real-transcript mismatch by itself. Do not change the clipboard writer unless this controlled experiment establishes a reproducible defect.

### Return-key check

Only after the mouse comparison, and only after the operator separately approves another clipboard replacement, start a fresh identified synthetic diagnostic process. Confirm the same button is available, activate the existing Return-key action, record its AppKit status separately, and paste once into a new verified blank plain-text TextEdit document. Compare against the same fixed constant, discard without saving, then Dismiss and confirm the result clears. If the mouse test cannot be compared reliably, stop without claiming keyboard verification.

### Automated checks and troubleshooting

Run the native focused suite and real-fixture regression separately. The isolated AppKit integration test uses its existing unique pasteboard and synthetic values; it does not access the general pasteboard and cannot establish TextEdit delivery. The native app also rejects invalid synthetic-mode arguments before opening AppKit.

```sh
scripts/build_macos_mic_probe.sh
scripts/run_macos_mic_probe.sh \
  --verification-id "79J-SYNTH-01" \
  --synthetic-copy-diagnostic
scripts/test_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh --real-fixture
python3 -m unittest discover -s scripts/tests -v
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
bash -n scripts/build_macos_mic_probe.sh \
  scripts/run_macos_mic_probe.sh \
  scripts/test_macos_mic_probe.sh
git diff --check
```

- **The diagnostic flag is rejected:** supply one valid 1–32-character ASCII verification ID; remove `--timed-fixture` and any explicit microphone-capture flag.
- **The runner asks for a model directory:** confirm `--synthetic-copy-diagnostic` was forwarded. Normal microphone and fixture launches still verify the configured local model.
- **The expected ID/PID is absent:** stop before copying; check the newly built `OUTFLOW_MIC_PROBE_APP` path and select the window whose title contains the exact ID.
- **Copy Result is disabled:** confirm the diagnostic reached `Result ready` and the view shows the fixed sample. Dismissed or cleared results must remain ineligible.
- **AppKit reports failure:** keep the visible synthetic sample; do not retry speculatively. The warning explains that previous clipboard contents may already be lost.
- **AppKit succeeds but TextEdit does not match:** verify blank-document state, plain-text mode, TextEdit selection and text-view focus, full-document inspection, and observed line-ending/Unicode transformations before assigning a cause.
- **TextEdit cannot be reached or compared:** record desktop verification as blocked. Automated policy and isolated-pasteboard tests are not substitutes for the desktop test.
- **A WER corpus test is unconfigured:** record it as not run, not passed. The licensed reference corpus remains a separate gate.

Prompt 79J is development-prototype evidence only. It does not complete T-046, T-047, T-010, T-011, AC-10, or the MVP. See [`evidence/mvp/prompt-79j-detailed-implementation-handoff.md`](evidence/mvp/prompt-79j-detailed-implementation-handoff.md) for exact outcomes and unresolved causes.

## Prompt 79K — real Moonshine final-result display and copy parity

Prompt 79K adds a development-only parity check for a timed-fixture session launched with a valid `--verification-id`. It compares the complete session-owned finalized UTF-8 bytes with a lossless UTF-8 encoding of the actual AppKit final-result view's `NSTextView.string` property. The check does not trim, normalize, case-fold, rewrite, hash, log, persist, or export transcript content, and does not touch the pasteboard. Ordinary microphone and synthetic diagnostic modes do not show the check.

The status appears beside Copy Result after an accepted nonempty final result:

- **MATCH** means the owned result bytes and displayed text bytes are exactly equal.
- **MISMATCH** means at least one byte differs; Copy Result is disabled and the result stays selectable.
- **UNAVAILABLE** means the displayed value cannot be converted losslessly to UTF-8, either side is invalid UTF-8, or there is no current nonempty accepted result. Copy Result is disabled for the verification session.

Copy eligibility still requires ResultReady, the accepted nonzero session ID, and nonempty finalized text. The verification session also requires MATCH. A stale, cancelled, empty, incomplete, mismatched, or unavailable result cannot reach the writer. The existing explicit Copy Result action remains the only ordinary general-pasteboard write; it converts UTF-8 before pasteboard access, calls `clearContents` once, then makes one `writeObjects:` attempt and reports AppKit's result. No automatic copy, retry, pasteboard read, or restoration is added. Cancel and Dismiss clear the volatile result and parity status.

### Build and launch the real-result verification

Use the already-authorized pinned local source, libraries, model, and WAV fixture from the Prompt 79J build environment. Export those variables first if they are not present in the shell, then build to a fresh directory and launch with the full 120-second fixture window:

```sh
export MOONSHINE_SOURCE_DIR=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
export MOONSHINE_SOURCE_REVISION=234f60faa0eb388b01cdf7e60aca232af37aefda
export MOONSHINE_LIBRARY=/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib
export MOONSHINE_ORT_LIBRARY="$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
export OUTFLOW_MIC_PROBE_MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
export OUTFLOW_MIC_PROBE_FIXTURE_WAV="$MOONSHINE_SOURCE_DIR/test-assets/two_cities_16k.wav"
export OUTFLOW_MIC_PROBE_BUILD_DIR="$(mktemp -d /private/tmp/outflow-prompt79k.XXXXXX)"

scripts/build_macos_mic_probe.sh
export OUTFLOW_MIC_PROBE_APP="$OUTFLOW_MIC_PROBE_BUILD_DIR/OutflowMicProbe.app"

scripts/run_macos_mic_probe.sh \
  --verification-id "79K-REAL-COPY-01" \
  --timed-fixture "$OUTFLOW_MIC_PROBE_FIXTURE_WAV" \
  --fixture-seconds 120
```

Before interacting, confirm the visible title includes the exact verification ID and PID, and confirm that PID's executable is the one in the fresh bundle. For example:

```sh
PID_FROM_WINDOW=12345 # replace with the PID shown in the identified window
ps -p "$PID_FROM_WINDOW" -o pid=,args=
lsof -a -p "$PID_FROM_WINDOW" -d txt
```

Press **Start timed fixture**, wait until Listening shows a nonempty partial, and click **Stop** before the 120-second auto-finalization limit. Wait for ResultReady, confirm a nonempty final and parity **MATCH**, then confirm Copy Result is enabled. The startup command does not copy anything.

### Controlled Copy Result and TextEdit comparison

Copy Result replaces the system clipboard. The warning says the prior contents may be lost if the writer fails after `clearContents`; clipboard managers or sync may retain copied text. Only proceed after the operator explicitly approves that one replacement. Then click Copy Result once and record only whether AppKit reports success or failure. Do not inspect, read, clear, retry, or restore the general pasteboard.

Open a genuinely new, empty, unsaved TextEdit document, switch it to plain-text mode, and confirm its text view has focus. Perform one normal **⌘V**. Compare the complete in-memory TextEdit value with the complete Outflow final result using exact Unicode string equality, with no normalization or rewriting. Where possible, compare the visible Outflow and TextEdit results while avoiding any screenshot or saved artifact containing recognized words. Discard the TextEdit document without saving. Dismiss the Outflow result and confirm it is cleared and Copy Result is disabled.

Record only sanitized metadata: correct app/window identity, Stop action, nonempty final, parity status, Copy action, AppKit status, whether paste was observed, exact equality status, visual comparison status, Cancel/Dismiss behavior, and whether any comparison interface was inaccessible. Never include recognized words, clipboard contents, transcript-bearing screenshots, recordings, or hashes in logs, evidence, or commits.

### Separate Cancel regression

Launch a fresh identified fixture process, then press Start, wait for a nonempty partial while Listening, and press **Cancel**. Confirm fixture production stops, no final is committed, parity is unavailable or cleared, Copy Result stays disabled, and no clipboard write occurs. Press **Dismiss result** and confirm Idle. Do not click Copy in this run.

### Regression checks and troubleshooting

Run the native focused suite, real-fixture harness, and repository regressions from the repository root:

```sh
scripts/test_macos_mic_probe.sh
scripts/test_macos_mic_probe.sh --real-fixture
python3 -m unittest discover -s scripts/tests -v
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
npm run typecheck
npm run test -- --run
npm run lint
npm run build
bash -n scripts/build_macos_mic_probe.sh \
  scripts/run_macos_mic_probe.sh \
  scripts/test_macos_mic_probe.sh
git diff --check
```

The existing synthetic diagnostic remains model- and microphone-free at runtime and continues to use the same Copy Result action. To rerun it, use a fresh build and:

```sh
scripts/run_macos_mic_probe.sh \
  --verification-id "79K-SYNTH-01" \
  --synthetic-copy-diagnostic
```

Verify its fixed sample, enabled button, and warning, but do not click Copy unless a separate clipboard replacement is explicitly approved. The isolated AppKit test uses its own unique pasteboard and synthetic values; it never reads or modifies the general clipboard.

- **Parity status is absent:** parity is shown only for a valid identified timed-fixture launch; check that both `--verification-id` and `--timed-fixture` reached the intended fresh app.
- **MISMATCH or UNAVAILABLE:** do not copy. Keep the result selectable and report the sanitized status. Do not retry or alter the writer speculatively.
- **Copy Result is disabled despite MATCH:** confirm the state is ResultReady, the displayed final is nonempty, and its session ID is still current; Dismiss/cancelled results are intentionally ineligible.
- **The desktop connector shows another process:** stop before interaction. Match the exact window ID/PID and executable path to the fresh bundle. If the correct window cannot be reached, report desktop verification blocked.
- **AppKit reports failure:** retain the visible result and do not retry; the previous clipboard may already have been cleared.
- **TextEdit receives no or different text:** verify the new blank document, plain-text mode, focus, complete document value, and exact comparison interface before assigning a failing boundary. Do not inspect the clipboard directly.
- **Licensed human-reference WER is not configured:** keep that separate quality test classified as skipped/not run, never passed.

### Prompt 79K sanitized outcome

The identified 120-second fixture run reached Listening with a nonempty partial; **Stop was clicked**, ResultReady contained a nonempty final, the source/display parity status was **MATCH**, and Copy Result was enabled. After the operator-approved single Copy Result click, AppKit reported **SUCCESS**. A new plain-text TextEdit document received one ⌘V, but exact final/document equality was **MISMATCH**. The visual comparison is **UNVERIFIED**. The first observed failing comparison is therefore at the TextEdit document-value boundary after the successful write; without reading the clipboard or an independent visual check, the cause remains unknown. The earlier Prompt 79I mismatch reproduced at this same observable boundary; it was not explained.

The separate identified Cancel run reached a nonempty partial, then Cancel stopped fixture production, discarded the result, cleared parity, and kept Copy disabled; Dismiss returned the app to Idle. A fresh synthetic diagnostic launch showed its fixed sample, existing enabled Copy Result button, and warning; it was dismissed without another clipboard write. No recognized words or clipboard contents are included in the evidence.

Prompt 79K is development-prototype evidence only. It does not complete T-046, T-047, T-010, T-011, AC-10, or release acceptance. Physical 4 GB hardware, release memory/size, licensed-corpus WER, supported-platform coverage, installer/signing, and accessibility/legal gates remain separate and unchanged. See [`evidence/mvp/prompt-79k-detailed-implementation-handoff.md`](evidence/mvp/prompt-79k-detailed-implementation-handoff.md) for the sanitized run outcome.

## Hypothetical vocabulary resolution proposals (Task 53)

[`crates/settings/tests/vocabulary_resolution_proposal.rs`](crates/settings/tests/vocabulary_resolution_proposal.rs) contains a test-only in-memory validator for explicit `Retain` and `OmitFromProposedCollection` decisions. Every original index needs exactly one decision. Retained entries stay in original order, and candidate validation reuses the core Unicode 18 collection rules. Any remaining conflict reports original indices, even when omitted entries leave gaps. The validator borrows the original strings and does not edit, delete, or persist them.

A valid proposal says only that its hypothetical retained collection passes current vocabulary validation. It does not establish trusted source provenance or authorize a migration or settings write. Manual review, trusted provenance, a separately approved settings migration, and tested atomic-write, recovery, and rollback behavior remain outstanding. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. See the [migration contract](docs/VOCABULARY_MATCHING_MIGRATION.md#hypothetical-resolution-proposal-validator-task-53).

Run the focused tests and asserted synthetic summary from the repository root:

~~~sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test vocabulary_resolution_proposal \
  --locked -- --nocapture
~~~

The summary reports proposal outcomes, index and original-value preservation, and that no resolution, deletion, settings-file modification, migration authorization, or production schema-v2 support occurred. It prints no vocabulary values.

## Vocabulary review cancellation and retry (Task 54)

The same integration-test target models a test-only review with an immutable original vocabulary and a separate editable decision draft. Incomplete or conflicting proposals leave the active draft available for correction; validation uses the existing Unicode 18 proposal validator. Cancellation discards the draft before or after validation, preserves original strings/order/indices, and is idempotent. A cancelled review cannot be edited or validated, and a fresh review starts with an empty draft. A valid proposal remains hypothetical and does not authorize settings migration or persistence.

Run the focused cancellation/retry demonstration with:

~~~sh
RUSTUP_TOOLCHAIN=stable cargo test \
  -p outflow-settings \
  --test vocabulary_resolution_proposal \
  --locked -- --nocapture
~~~

**Task 54 verifies pre-commit cancellation and in-memory recovery only; persistent filesystem rollback is NOT TESTED.** Existing atomic-writer tests do not establish schema-v2 migration rollback. Future migration work still needs real write, disk-full, permission, pre/post-replacement, interruption recovery, last-known-good preservation, permissions, Windows replacement/ACL, Linux filesystem, and unsupported-downgrade checks. T-017, T-018, T-038, T-039, T-053, and AC-08 remain incomplete. See [the migration contract](docs/VOCABULARY_MATCHING_MIGRATION.md#pre-commit-review-cancellation-and-retry-task-54) and [Task 54 evidence](evidence/rust-vocabulary-cancellation-task-54.md).

## Native in-memory Rust vocabulary CRUD (partial T-038)

`outflow-settings` now exposes `add_vocabulary_entry`, `edit_vocabulary_entry`, and `delete_vocabulary_entry` for an in-memory `Vec<VocabularyEntry>`. Add and edit reuse `outflow-core`'s current 500-entry, 128-Unicode-scalar, and Unicode 18 canonical-caseless spoken-phrase validation. Stable IDs must be unique; edit preserves the selected ID and its position, and delete removes by ID while preserving the remaining order. Each operation validates a temporary candidate before replacing the caller's vector, so failures leave the full collection unchanged. Typed errors identify duplicate IDs, duplicate spoken phrases, invalid fields, missing IDs, and a full collection without including vocabulary values or IDs.

This is native CRUD logic only. It does not write settings or connect the browser preview to Rust. Schema-v1 serialization, loading, matching behavior, and persistence are unchanged. The example optionally exercises the existing atomic writer and loader in a fresh temporary directory using synthetic values; this is a demonstration of those existing APIs, not production CRUD persistence. The current writer reports unsupported on platforms without its owner-only atomic replacement implementation.

Run the synthetic Add → Edit → Delete example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -q -p outflow-settings --example vocabulary_crud --locked
```

Expected output on a platform supported by the existing atomic writer:

```text
Add: success (entries: 1)
Edit: success (entries: 1)
Temporary save/reload: success (entries: 1)
Delete: success (entries: 0)
```

Run the focused and complete Rust test suites with:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings vocabulary_crud --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
```

The synthetic tests cover Add/Edit/Delete, stable ID and position preservation, missing and duplicate IDs, Unicode 18 `straße`/`STRASSE` and composed/decomposed collisions, Unicode and emoji, scalar/collection limits, atomic rejection, and content-free errors. No new dependency is required. The full implementation handoff and exact verification results are recorded in [`evidence/rust-vocabulary-crud-task-38.md`](evidence/rust-vocabulary-crud-task-38.md).

Troubleshooting:

- **`CollectionFull`:** remove an existing entry before adding another; the limit is 500.
- **`InvalidEntry`:** inspect the reported field and reason; each spoken and written value must contain non-whitespace text and no more than 128 Unicode scalar values.
- **`DuplicateSpoken`:** the phrase conflicts under Unicode 18 canonical-caseless matching. Error text contains only collection indices.
- **`MissingId`:** pass the stable ID already present in the in-memory collection. The error does not echo IDs.
- **Temporary save/reload unavailable:** the example still demonstrates in-memory CRUD, while the existing writer does not provide its owner-only replacement guarantee on that platform. This API does not add another persistence path.
- **Temporary directory creation fails:** check that the operating-system temporary directory is writable; the example creates and removes a uniquely named directory there and never targets the user's real settings.

This partial implementation does not complete T-038 or AC-08. Boundary-aware replacement, production settings persistence integration, frontend/native settings integration, deletion/save/load acceptance, and the T-017/T-018 prerequisites remain open. T-017, T-018, T-038, T-039, and all release gates remain unchecked.

## Native Rust whitespace normalization (partial T-035)

`outflow_core::text_cleanup::normalize_transcript_whitespace(&str) -> String` is a pure, deterministic first stage for future transcript cleanup. It converts CRLF and standalone CR to `\n`, collapses internal runs of ASCII spaces and tabs to one space, and trims those characters from each line. It preserves the number and positions of newlines, including leading/trailing newlines and blank paragraphs. Punctuation, words, capitalization, numbers, emoji, combining marks, and other Unicode text are copied unchanged; non-ASCII whitespace is not normalized. The implementation scans once and adds no dependencies.

Run the synthetic example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_whitespace_normalization --locked
```

Expected output (Rust debug formatting makes CRLF and tabs visible):

```text
Before: "  I visited   north parkade.\r\n\r\n\tI use rust\t at  ubc.  "
After: "I visited north parkade.\n\nI use rust at ubc."
Idempotent: true
```

The example asserts the exact output and idempotence, exiting unsuccessfully if either check fails. The test module has 32 table fixtures; every fixture checks exact output, idempotence, and that its original input remains unchanged. Four focused tests additionally check newline placement/count, ASCII-only trimming, character preservation, and the empty-input result.

Run the requested Rust verification from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
```

Troubleshooting:

- **CR or CRLF remains visible in debug output:** confirm the function is called on the returned string; it canonicalizes those line endings to `\n`.
- **A blank line disappeared:** this function retains newline characters; check that a caller does not split, filter, or trim lines afterward.
- **A non-breaking/em space remains:** only ASCII space and tab are normalized by design.
- **Punctuation or capitalization changed:** that behavior is outside this API; check that another cleanup stage is not being applied by the caller.
- **Cargo reports a locked dependency or toolchain issue:** run the commands at the repository root with the stable toolchain and verify the tracked `Cargo.lock`; this change does not add or update dependencies.

This is the standalone whitespace stage. The following partial T-035 slice adds opt-in dictation commands and places them before vocabulary correction; neither stage is connected to the application. T-023 remains an unmet prerequisite, and production T-035/T-036, T-038/T-039, AC-07/AC-08, and all release gates remain unchecked. See [`evidence/T-035-whitespace-normalization.md`](evidence/T-035-whitespace-normalization.md) for the whitespace-stage evidence and the following section for command-stage evidence.

## Native Rust opt-in dictation commands (partial T-035)

`outflow_core::text_cleanup::apply_dictation_commands(input, enabled)` recognizes only `new line` and `new paragraph`. Matching is ASCII case-insensitive and requires a complete phrase bounded by the start/end of input, an ASCII space, or a line boundary. The two words must be on one line with one ASCII space between them. Adjacent punctuation and quotation marks block recognition. The stage replaces a recognized command with `\n` or `\n\n`, removes its immediate ASCII separator spaces, preserves unrelated text and existing line breaks, and scans the original input once.

The `enabled` argument is the opt-in flag. The standalone function returns its input unchanged when `enabled` is `false`. The composed API `normalize_whitespace_commands_and_apply_vocabulary(input, entries, commands_enabled)` uses the documented order: normalize whitespace, optionally interpret commands, then correct vocabulary. Recognized commands take precedence over vocabulary entries. Vocabulary replacement output is not normalized afterward. The prior `normalize_whitespace_and_apply_vocabulary` API remains available and delegates with commands disabled.

The composed API normalizes source tabs, repeated ASCII spaces, CRLF, and standalone CR before matching. The standalone command function is intended for already-normalized transcript input; it requires the command phrase itself to contain one ASCII space.

Run the synthetic example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_dictation_commands --locked
```

It prints the input and outputs with Rust debug formatting and asserts both results exactly:

```text
commands enabled: "I use Rust\nat UBC\n\nNorth Parkade"
commands disabled: "I use Rust new line at UBC new paragraph North Parkade"
```

The focused tests and workspace checks are:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core text_cleanup --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
```

Troubleshooting:

- **A phrase remains literal:** pass `true` to the opt-in command flag. Commands remain disabled in the compatibility composition.
- **A phrase in quotation marks or beside punctuation remains literal:** only ASCII spaces and line boundaries count as surrounding separators; quotes and punctuation are intentionally conservative non-matches.
- **Tabs or repeated spaces inside a phrase do not match the standalone function:** use the composed API, which normalizes source whitespace before command matching.
- **An extra blank line appears around a command:** existing line breaks are preserved. A command on its own already-separated line adds its requested line break without deleting the original separators.
- **Ordinary speech becomes a line break:** exact phrases can be literal spoken content. This initial feature is opt-in and uses exact phrase matching, not a semantic intent classifier.

This is only the whitespace, command, and existing vocabulary slice. Punctuation/capitalization cleanup, Clean/Verbatim filler handling, ASR/settings integration, and the full T-035 golden corpus are not implemented. T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked. See [`evidence/T-035-dictation-commands.md`](evidence/T-035-dictation-commands.md) for exact environment, commands, results, and limitations.

## Native Rust punctuation spacing (partial T-035)

The standalone `outflow_core::text_cleanup::normalize_punctuation_spacing(input)` function removes ASCII spaces and tabs before `, . ! ? ; :` only when the previous non-whitespace character is alphabetic, the mark is followed by whitespace or the end of input, and the adjacent alphanumeric token contains no number. It leaves ambiguous spaced punctuation and quote sequences unchanged. It preserves all punctuation, words, capitalization, numeric content, Unicode, emoji, and line/paragraph breaks. It does not add punctuation or spacing after punctuation and does not perform complete punctuation restoration.

The standalone API retains its original text-only behavior. The protected-aware composition described below now runs this same spacing rule after vocabulary correction, while preserving every configured written replacement.

Run the synthetic example from the repository root after installing the Rust toolchain described under [Prerequisites](#prerequisites):

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_punctuation_spacing --locked
```

The example prints input and output with Rust debug formatting, checks the exact expected result, and checks idempotence. Its output is:

```text
Before: "Hello , world !\nI visited north parkade .\nThe café , 東京 tea 😀 .\nVersion 3 . 14 remains unchanged."
After: "Hello, world!\nI visited north parkade.\nThe café, 東京 tea 😀 .\nVersion 3 . 14 remains unchanged."
```

Run the focused fixtures and workspace checks from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core punctuation_spacing --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
```

Troubleshooting:

- **A space remains before punctuation:** only ASCII spaces and tabs are candidates. The function also preserves spacing before marks preceded by a number, emoji, combining mark, or non-alphabetic character.
- **A decimal, date, or version remains spaced:** numeric-adjacent punctuation is intentionally protected.
- **A spaced punctuation or quote sequence remains unchanged:** ambiguous sequences are preserved rather than guessed.
- **The space before a period after emoji remains:** emoji are not alphabetic, so that case is outside the rule.
- **The example output changes after editing:** inspect that only the six supported marks are normalized; punctuation insertion, capitalization, and post-punctuation spacing are not implemented here.
- **Cargo reports a toolchain or lock issue:** run from the repository root with the stable toolchain and tracked lockfile; this stage adds no dependency.

This is a partial T-035 building block. It does not change the behavior of the existing whitespace/command/vocabulary APIs. See [`evidence/T-035-punctuation-spacing.md`](evidence/T-035-punctuation-spacing.md) for the standalone stage's exact implementation, verification, and limitations.

## Native protected punctuation and vocabulary composition (partial T-035/T-039)

`outflow_core::text_cleanup::normalize_whitespace_commands_vocabulary_and_punctuation(input, entries, commands_enabled)` composes four deterministic stages in this order:

1. `normalize_transcript_whitespace`
2. `apply_dictation_commands` (only when enabled)
3. `apply_vocabulary_post_corrections_with_spans`
4. Protected-aware punctuation spacing using the existing rules from `normalize_punctuation_spacing`

The vocabulary corrector records the exact UTF-8 byte ranges of written replacements. The punctuation formatter receives those ranges immediately, before any other change to corrected text. It never deletes bytes inside a replacement. When a candidate whitespace run intersects a protected range, it retains the complete run. This preserves internal punctuation and spacing, including replacement leading/trailing spaces and line breaks. Existing eligible spacing outside a replacement can still be corrected, even when the preceding alphabetic character belongs to the replacement. Emoji, accented text, combining marks, numeric tokens, versions, abbreviations, ambiguous punctuation, quotes, and paragraph breaks retain the standalone stage's existing conservative behavior.

The new API returns final text only; protected byte offsets are internal because punctuation removal can shift later offsets. Existing composition APIs and the standalone `normalize_punctuation_spacing(input)` API keep their previous behavior.

Runnable example:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_protected_punctuation_pipeline --locked
```

Expected output (Rust debug formatting):

```text
Input: "rust , at ubc ."
Output: "Rust , Edition, at UBC."
Matches expected: true
Protected replacement preserved: true
```

The example asserts the full output and checks both ordered vocabulary replacements against their source-stage provenance ranges. The unit tests also assert exact strings for ordinary punctuation, protected punctuation and spaces, trailing/leading replacement whitespace, intersecting whitespace runs, repeated replacements, Unicode byte offsets, combining marks, newlines, paragraph breaks, numbers, abbreviations, commands, longest-match precedence, non-cascading replacements, validation errors, input immutability, and determinism.

Run the focused tests and requested workspace checks from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core text_cleanup --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
git diff --cached --check
```

Troubleshooting:

- **Spaces inside a written replacement remain:** the replacement is protected byte-for-byte; this includes spaces immediately before its punctuation.
- **Spacing outside a replacement remains:** punctuation must be one of `, . ! ? ; :`, follow the existing alphabetic and no-number rule, and not form an ambiguous punctuation or quote sequence.
- **A numeric token, version, abbreviation, combining-mark boundary, or ambiguous sequence stays spaced:** these cases remain conservative by design; the formatter does not guess.
- **A vocabulary correction is missing:** verify the collection has valid nonempty fields, no canonical-caseless duplicate spoken phrases, and that the command flag has not consumed the source phrase first.
- **The output changes on a second pipeline call:** vocabulary mappings are non-cascading within one call, but can cascade across calls. For example, `rust` → `ubc` and `ubc` → `UBC` yields `ubc` first and `UBC` on reapplication. The complete pipeline is therefore not universally idempotent.
- **A byte offset appears different after formatting:** offsets are private to the pipeline and are not returned; do not reuse ranges from the vocabulary stage against final output.

This is a partial implementation only. Automatic punctuation insertion, capitalization, the full Clean/Verbatim settings policy, ASR/settings/frontend integration, the complete golden corpus, and production prerequisites remain outstanding. Optional conservative filler removal is integrated in the following partial T-036 API. **T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.** See [`evidence/T-035-T-039-protected-punctuation-integration.md`](evidence/T-035-T-039-protected-punctuation-integration.md) for punctuation-stage evidence.

## Native line-leading filler removal (partial T-036)

`outflow_core::text_cleanup::remove_leading_isolated_fillers(input, enabled)` is a standalone, deterministic Clean/Verbatim building block. When enabled, it removes only ASCII case-insensitive `um, ` or `uh, ` at the beginning of a line, and only when the next character is alphabetic. It removes exactly that prefix, preserves the following word's original spelling and capitalization, and copies unrelated UTF-8 text and line endings unchanged. Leading spaces/tabs, numbers, quotes, punctuation, incomplete phrases, and repeated filler sequences such as `um, uh, yes` remain intact. Middle- and sentence-final fillers are not considered.

Pass `false` for Verbatim mode or when filler removal is disabled; the original text is returned unchanged. The function expects normalized transcript input with one ASCII space after the filler comma and does not normalize whitespace. It does not detect general hesitation, remove repeated words, or infer speaker intent. The standalone API remains available with its original behavior. The composed pipeline below calls the same shared recognizer after punctuation spacing and honors protected vocabulary replacement ranges. The settings crate's persisted schema is not duplicated or imported into core.

Run the synthetic example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_filler_removal --locked
```

It prints Rust debug-formatted input and outputs, then asserts exact Clean and Verbatim results, Clean idempotence, and input immutability. Expected output:

```text
Input: "um, so I visited north parkade.\nUh, I use Rust at UBC.\nI said um, not uh."
Clean: "so I visited north parkade.\nI use Rust at UBC.\nI said um, not uh."
Verbatim: "um, so I visited north parkade.\nUh, I use Rust at UBC.\nI said um, not uh."
```

Run the focused fixtures and workspace checks:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core leading_filler --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
git diff --cached --check
```

Troubleshooting:

- **The filler remains:** pass `true`, start the line with the filler without leading spaces or tabs, use its comma and exactly one ASCII space, and follow it with a letter.
- **A repeated sequence remains:** this is deliberate; `um, uh, yes` is preserved to avoid guessing which hesitation was intended.
- **A filler after punctuation, inside quotes, before a number, or in the middle/end of a sentence remains:** these positions are outside the narrowly supported rule.
- **A multi-space, tab, or non-ASCII separator does not match:** normalize transcript whitespace before calling this standalone stage.
- **Vocabulary replacement text contains `um,` or `uh,`:** use the composed optional-filler API below; it preserves a complete candidate whenever deletion would intersect protected replacement bytes.

This standalone API does not integrate Clean/Verbatim settings or the full cleanup policy. Its protected-aware composition and limits are described below. **T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.** See [`evidence/T-036-leading-filler-removal.md`](evidence/T-036-leading-filler-removal.md) for the initial standalone-stage evidence.

## Native cleanup pipeline with optional protected filler removal (partial T-036)

`outflow_core::text_cleanup::process_transcript_with_optional_fillers(input, entries, commands_enabled, fillers_enabled)` returns final text after these five stages, in order:

1. `normalize_transcript_whitespace`
2. `apply_dictation_commands` when `commands_enabled` is `true`
3. `apply_vocabulary_post_corrections_with_spans`
4. Protected-aware punctuation spacing
5. Protected-aware filler removal when `fillers_enabled` is `true`

The filler policy remains narrow: it recognizes only line-leading ASCII case-insensitive `um, ` and `uh, ` followed by an alphabetic character, while retaining repeated filler sequences. With `fillers_enabled: false`, filler removal is skipped. This flag controls only that stage; it does not yet select or persist the full Clean/Verbatim settings policy.

Vocabulary replacements remain protected byte-for-byte through punctuation formatting and filler removal. A filler-like written replacement is preserved. If an otherwise removable candidate intersects any protected replacement bytes, the entire candidate prefix is kept. An unprotected line-leading filler can still be removed before a later protected word. Internal provenance ranges are adjusted after every length-changing deletion and never escape through the public API.

Runnable example:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-core --example text_cleanup_with_fillers --locked
```

The example uses input `um, rust , at ubc .`, with `rust` → `Rust , Edition` and `ubc` → `UBC`, and prints:

```text
Input: "um, rust , at ubc ."
Fillers enabled: "Rust , Edition, at UBC."
Fillers disabled: "um, Rust , Edition, at UBC."
Protected replacement preserved: true
```

It asserts both complete outputs and verifies that a protected replacement beginning with `um, ` remains exact. The focused tests also cover protected multiline replacements, deletion overlap, commands-before-vocabulary order, longest-match/non-cascading behavior, invalid and duplicate collections, Unicode offsets, line and paragraph breaks, repeated fillers, quoted and numeric text, immutability, and deterministic output.

Run the focused suite and full Rust workspace checks from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-core optional_filler_pipeline --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
git diff --cached --check
```

Troubleshooting and limitations:

- **A filler remains:** removal must be enabled, the line must begin immediately with `um, ` or `uh, `, and the following character must be alphabetic. Indented, numeric, quoted, punctuated, sentence-internal, incomplete, or repeated filler phrases remain unchanged.
- **A protected replacement starts with a filler-like phrase:** the replacement is copied exactly. A candidate whose deletion intersects a replacement is retained whole.
- **A filler follows a protected replacement on another line:** the later unprotected candidate can be removed without changing the earlier replacement.
- **Tabs or multiple spaces prevent recognition:** the composed API normalizes source ASCII spaces/tabs before running the shared filler policy; the standalone filler function expects normalized input.
- **A command phrase becomes a line break:** command recognition is separately opt-in and runs before vocabulary matching; exact `new line`/`new paragraph` phrases can also be literal speech.
- **Spacing remains inside vocabulary text:** punctuation changes are intentionally blocked inside protected written replacements.
- **A later call changes vocabulary output:** each call is deterministic and vocabulary matching is non-cascading within that call, but the complete pipeline is not universally idempotent across calls when one replacement matches another entry.
- **Full Clean/Verbatim behavior is expected:** this core API only toggles conservative filler removal. The settings adapter below maps the existing filler preference; application integration, broader cleanup, ASR integration, and the complete acceptance corpus remain unfinished.

This remains a partial implementation. **T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.** See [`evidence/T-036-protected-filler-integration.md`](evidence/T-036-protected-filler-integration.md) for focused implementation and verification evidence.

The settings crate now provides a separate adapter for the filler preference. The core API above still accepts an explicit `fillers_enabled` flag; the adapter maps the existing settings to that flag. It does not define full Clean/Verbatim semantics or connect the application runtime.

## Native cleanup preference adapter (partial T-036)

`outflow_settings::cleanup_policy::process_transcript_with_filler_preference` accepts the existing `CleanupSettings` and `CleanupMode` values from `outflow-settings`, then delegates to `outflow-core`'s `process_transcript_with_optional_fillers`. It accepts borrowed `VocabularyEntryRef` entries and returns the core pipeline's typed `VocabularyCollectionError` when vocabulary validation fails. It performs no file access, settings migration, microphone inspection, or transcript storage.

The supported policy is filler-only:

| `CleanupMode` | `remove_fillers` | Effective filler removal |
| --- | --- | --- |
| `Clean` | `true` | Enabled |
| `Clean` | `false` | Disabled |
| `Verbatim` | `true` | Disabled |
| `Verbatim` | `false` | Disabled |

This adapter does **not** implement complete Verbatim mode. Whitespace normalization, opt-in dictation commands, vocabulary correction, and punctuation spacing continue to run in all four combinations. Any broader Clean/Verbatim stage policy needs a separate product decision before production integration.

Example setup using synthetic vocabulary:

```rust
use outflow_core::vocabulary::VocabularyCollectionError;
use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_settings::cleanup_policy::process_transcript_with_filler_preference;
use outflow_settings::{CleanupMode, CleanupSettings};

fn main() -> Result<(), VocabularyCollectionError> {
    let vocabulary = [
        VocabularyEntryRef {
            spoken: "rust",
            written: "Rust , Edition",
        },
        VocabularyEntryRef {
            spoken: "ubc",
            written: "UBC",
        },
    ];
    let cleanup = CleanupSettings {
        mode: CleanupMode::Clean,
        remove_fillers: true,
    };
    let output = process_transcript_with_filler_preference(
        "um, rust , at ubc .",
        &vocabulary,
        false,
        &cleanup,
    )?;
    assert_eq!(output, "Rust , Edition, at UBC.");
    Ok(())
}
```

Run the complete four-case example from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example cleanup_filler_preferences --locked
```

Expected output:

```text
Input: "um, rust , at ubc ."
Clean, remove_fillers=true: "Rust , Edition, at UBC."
Clean, remove_fillers=false: "um, Rust , Edition, at UBC."
Verbatim, remove_fillers=true: "um, Rust , Edition, at UBC."
Verbatim, remove_fillers=false: "um, Rust , Edition, at UBC."
```

The example asserts each full output and separately checks byte-for-byte preservation of the protected replacement `um, keep this`.

Focused and workspace verification:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings cleanup_policy --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
git diff --cached --check
```

Troubleshooting and remaining limits:

- **A leading `um, ` or `uh, ` remains:** removal requires `Clean` and `remove_fillers: true`; the existing core recognizer also requires a line-leading, isolated filler followed by an alphabetic character.
- **A repeated, quoted, numeric, or sentence-internal filler remains:** those cases are intentionally outside the existing conservative deletion rule.
- **A filler-like vocabulary replacement is present:** protected written replacements remain byte-for-byte unchanged, including `um,` or `uh,` text.
- **Commands, vocabulary, or punctuation still change Verbatim output:** this adapter controls only filler removal; other existing pipeline stages run independently of `CleanupMode`.
- **Application behavior is expected to follow these settings:** the adapter is a pure crate API and is not wired to React, Tauri, Moonshine, or the microphone. T-017/T-018 prerequisites, broader cleanup policy, the complete T-036 golden set, AC-07/AC-08, app integration, platform coverage, and all release gates remain open.

This is partial T-036 evidence only. **T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.** See [`evidence/T-036-cleanup-preference-adapter.md`](evidence/T-036-cleanup-preference-adapter.md) for the actual verification results.

### Clean/Verbatim contract example — current behavior only

[`crates/settings/examples/cleanup_mode_contract.rs`](crates/settings/examples/cleanup_mode_contract.rs) characterizes the current settings adapter for the synthetic input `  um, rust  new line  at ubc .` and vocabulary `rust → Rust`, `ubc → UBC`. It uses the existing Rust APIs and asserts every output. It does not approve complete Verbatim semantics or change production cleanup behavior. The unresolved policy alternatives and recommendation are in [docs/CLEANUP_MODE_POLICY.md](docs/CLEANUP_MODE_POLICY.md), marked **PROPOSED — NOT APPROVED**.

Run the executable and focused cross-crate regression tests from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo run -p outflow-settings --example cleanup_mode_contract --locked
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked
```

Expected output:

```text
Input: "  um, rust  new line  at ubc ."
Clean, remove_fillers=true: "Rust\nat UBC."
Clean, remove_fillers=false: "um, Rust\nat UBC."
Verbatim, remove_fillers=true: "um, Rust\nat UBC."
Verbatim, remove_fillers=false: "um, Rust\nat UBC."
Commands disabled (Verbatim): "um, Rust new line at UBC."
```

The commands-disabled line keeps the literal phrase `new line` while still correcting `rust`/`ubc` and removing the space before the final period. The focused integration test covers the public adapter contract, stage independence, formatting and Unicode preservation, typed vocabulary errors, immutability, determinism, and the existing schema-v1 JSON shape.

Run the complete Rust verification from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
git diff --cached --check
```

Troubleshooting:

- **The command phrase remains literal:** the example passes `commands_enabled: false` for that final case. Set the independent command option to `true` to recognize the supported phrase.
- **A leading filler remains:** current deletion requires Clean mode and `remove_fillers: true`; Verbatim always retains supported fillers.
- **Vocabulary or punctuation changes in Verbatim:** that is current adapter behavior. The complete Verbatim policy has not been approved; see [docs/CLEANUP_MODE_POLICY.md](docs/CLEANUP_MODE_POLICY.md).
- **A filler-like replacement changes:** vocabulary replacement spans are protected from later formatting and filler deletion; rerun the focused contract test and inspect the exact written replacement.
- **Cargo reports a toolchain or lockfile mismatch:** run from the repository root with the pinned `rust-toolchain.toml` and keep `--locked`; this task adds no dependency or lockfile change.

This is partial T-036 characterization only. **T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.** Exact commands and results are in [`evidence/T-036-cleanup-mode-contract.md`](evidence/T-036-cleanup-mode-contract.md).

## Cleanup golden corpus — current-behavior baseline

The synthetic golden fixtures are stored in [`crates/settings/tests/fixtures/cleanup_current_behavior.json`](crates/settings/tests/fixtures/cleanup_current_behavior.json). Each JSON object has eight required fields: `id`, `category`, `input`, `mode`, `remove_fillers`, `commands_enabled`, `vocabulary`, and `expected`. The optional test-only `protected_tokens` field adds manually reviewed preservation assertions without adding fixtures or changing the required schema. Unannotated fixtures remain valid. Unknown fields are rejected at fixture, vocabulary-item, and protected-token levels.

Example fixture object:

```json
{
  "id": "normalized-greeting",
  "category": "whitespace",
  "input": "Hello, world.",
  "mode": "clean",
  "remove_fillers": true,
  "commands_enabled": false,
  "vocabulary": [],
  "expected": "Hello, world."
}
```

An annotated fixture can add selected protected tokens:

```json
{
  "protected_tokens": [
    {
      "kind": "proper_noun",
      "text": "Élodie",
      "input_bytes": [0, 7],
      "output_bytes": [0, 7]
    }
  ]
}
```

`kind` is `number` or `proper_noun`. Both ranges are zero-based, half-open UTF-8 byte ranges `[start, end)`: the start byte is included and the end byte is excluded. They refer to separate strings, so punctuation cleanup can make the input and expected-output offsets differ. The checker applies the declared output range to both the independent `expected` string and the actual cleanup output. It uses the range directly and does not search for the token.

To annotate a token, choose one exact occurrence from an existing fixture, copy its exact spelling and capitalization into `text`, and manually count UTF-8 bytes from the start of `input` and `expected`. For example, the year `2026` in `numeric-comma-grouping-and-year-preserved` uses `kind: number` and `[31, 35]` for both ranges. Check both ranges against their own strings; a multibyte character occupies more than one byte. Use distinct, non-overlapping ranges for multiple annotations in a fixture. Do not annotate text intentionally rewritten by a configured vocabulary entry; vocabulary-written replacements have separate provenance tests. The metadata is test-only and does not alter persisted settings or production cleanup behavior.

To add a fixture, append a synthetic case with a unique descriptive ID and one of the eight allowed categories. Set mode, `remove_fillers`, `commands_enabled`, vocabulary, and input explicitly. Review the Rust stage order, then author the complete `expected` string from those rules; do not run cleanup and copy its output. Preserve exact punctuation, line endings, Unicode code points, and optional metadata. When extending an established corpus, use `git show <base>:<fixture-path>` and Python's `json.loads` to compare the prior ordered fixture objects with the unchanged prefix; keep this comparison one-off rather than adding a permanent comparison script. If current behavior or the unresolved Verbatim policy changes, review each affected expectation as a deliberate baseline change.

Run the focused corpus test and its expected visible summary from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture
```

The current-behavior baseline contains exactly 200 explicitly authored synthetic fixtures. The earlier expansion from 96 to 120 added four groups of six:

1. **Whitespace boundaries:** ASCII spaces/tabs-only input; outer whitespace around a sentence; mixed LF, CRLF, and standalone CR; spaced blank lines; Unicode text with ASCII separators; and whitespace around emoji beside a paragraph break. The current implementation trims leading/trailing ASCII spaces and tabs on each line, collapses interior runs to one ASCII space, maps each CR/CRLF/LF break to LF without changing the number or placement of breaks, and copies non-ASCII whitespace unchanged.
2. **Clean/Verbatim and filler boundaries:** mixed-case supported prefixes with removal on and off; Verbatim filler preservation; Verbatim vocabulary and punctuation processing; repeated fillers after a command-generated break; and a protected filler-like vocabulary line followed by a removable ordinary filler on another line. Filler recognition remains limited to line-leading ASCII `um, ` and `uh, ` prefixes followed by an alphabetic character; repeated supported fillers are retained. The adapter applies the same non-filler stages in Verbatim, while deletion remains Clean-only and controlled by `remove_fillers`.
3. **Unicode vocabulary matching:** canonical-equivalent accents; a multiword Unicode phrase; a partial-match attempt before a combining mark; emoji separators; repeated canonical-equivalent occurrences; and longest-match precedence for overlapping accented phrases. Matching uses the existing Unicode 18 canonical-caseless comparison and word-continuation rules. Written replacements are inserted byte-for-byte and are not normalized afterward.
4. **Punctuation and preservation interactions:** eligible punctuation beside a spaced decimal; spacing after a digit-bearing identifier; punctuation beside accented names; intentional leading/trailing replacement spaces; two protected replacements across a paragraph break with external punctuation; and a multiline composition of punctuation, an enabled command, vocabulary, and optional filler removal. Numeric tokens and protected replacements remain subject to exact-output assertions; eligible punctuation outside protected spans may still be normalized.

The 120-to-144 expansion added 24 more cases in four groups of six:

1. **Whitespace preservation and boundaries:** empty lines at both transcript edges; repeated horizontal whitespace around punctuation; spaces/tabs around LF and CRLF; a line containing only non-ASCII whitespace; ASCII whitespace beside combining marks; and Unicode/emoji with three consecutive blank lines. The fixtures assert exact line-break counts and preserve non-ASCII whitespace.
2. **Filler false-positive prevention:** a supported line-leading filler before a letter; a filler-like token without its comma; punctuation instead of an alphabetic continuation; mixed-case repeated fillers; a quoted line-leading phrase; and a protected vocabulary replacement beginning `um, ` beside an independently removable `uh, ` line.
3. **Vocabulary boundaries and precedence:** a phrase beside ASCII punctuation; canonical-equivalent Unicode 18 matching beside Unicode punctuation; rejection inside a larger alphabetic word; a combining-mark continuation that blocks a partial match; longest overlapping phrase precedence; and a written phrase that must not cascade to another mapping in the same call.
4. **Multi-stage safety interactions:** a numeric period left spaced beside eligible punctuation correction; an accented proper noun beside a vocabulary replacement; an enabled `new line` followed by removable filler; a disabled command phrase available for vocabulary matching; two protected replacements across punctuation and a paragraph break; and a longer whitespace/command/vocabulary/punctuation/filler composition.

The first 120 parsed objects, including all 16 existing protected-token annotations (eight numbers and eight proper nouns across the same 12 fixtures), are unchanged. The five synthetic rejection checks, strict metadata validation, and all prior mode/category coverage remain in the runner. The current-behavior label records characterization only; it does not approve a final Verbatim policy or constitute independently certified acceptance expectations.

The 144-to-168 expansion appended 24 cases in four groups of six:

1. **Ambiguous punctuation and quotation boundaries:** a supported mark beside a curly quote; spaced-period ellipsis; consecutive question/exclamation marks; punctuation between quoted phrases; abbreviation spacing beside eligible punctuation; and multiline ambiguous plus eligible spacing. Ambiguous sequences remain as spoken, while independently eligible punctuation can still tighten.
2. **Unicode matching and boundary safety:** precomposed/decomposed accents in one transcript; Unicode capital sharp-S case folding; a combining mark blocking a shorter match; vocabulary beside Japanese punctuation; emoji separating two matches; and a multiword Unicode replacement with intentional double spaces and numeric punctuation. Written replacement bytes remain exact.
3. **Repeated vocabulary and precedence:** three differently capitalized occurrences; longer phrase precedence; multiple mappings in one sentence; no same-call cascade from inserted text; a vocabulary phrase beside punctuation and numbers; and two protected replacements with punctuation spacing corrected outside their spans.
4. **Cross-stage cleanup and preservation:** Clean command, filler, and vocabulary interaction; the same command/vocabulary work with filler deletion disabled; Verbatim retaining fillers while an enabled command runs; numeric and replacement punctuation beside an eligible correction; an accented name beside a command-generated paragraph break; and multiline filler removal around two protected replacements.

A one-off comparison against base commit `f0bf356147f3fe0d63990f1365403f54130a9caa` confirms the first 144 parsed objects are unchanged, including the original 16 protected-token annotations. The 24 additions have no protected-token metadata. The five synthetic rejection cases, strict parsing, all eight categories, all four mode/filler combinations, both command settings, exact output, determinism, and immutability checks remain in the runner. These expected outputs characterize reviewed source behavior; the baseline does not approve a final Verbatim policy or constitute independently certified acceptance expectations.

The 168-to-192 expansion appends 24 cases in four ordered groups of six:

1. **Numeric and proper-noun safety:** signed values beside eligible punctuation; scientific-notation-like text; a date/version beside ambiguous punctuation; a multiword accented name; an acronym and person's name together; and multiple numeric/name tokens around a protected vocabulary replacement. Digit-bearing token punctuation remains conservatively spaced, including `2026 .` and `6.02e23 !`.
2. **Unicode whitespace and boundaries:** a nonbreaking space; thin and narrow spaces beside ASCII runs; an ideographic space in multilingual text; tabs beside a decomposed combining sequence; emoji/Japanese text across CRLF and LF; and a non-ASCII whitespace line before punctuation. ASCII spaces/tabs and CR/CRLF/LF follow the existing normalizer; other Unicode whitespace and combining sequences are copied without broader normalization.
3. **Commands and filler edges:** commands at transcript start/end and in a consecutive sequence; punctuation-interrupted command-like wording; repeated fillers after a command-generated line break; and a protected filler-like vocabulary replacement alongside ordinary filler removal after a paragraph break. Generated breaks are additive, and repeated supported fillers remain intact.
4. **Protected vocabulary and stage order:** multiple replacements separated by accented text and emoji; longest-match selection for a longer accented phrase; exact replacement punctuation spacing beside ordinary punctuation; embedded filler-like replacement lines; short and long replacements followed by punctuation edits; and a multiline commands/vocabulary/protected-punctuation/Clean-filler composition. Written replacement bytes remain exact while unprotected punctuation can still be cleaned.

A one-off comparison against base commit `71be6c46888e6e1ec68373104567e652ba87222c` confirms that the first 168 parsed fixture objects are unchanged and in order, including all 16 original protected-token annotations across the same 12 fixtures (eight numeric and eight proper-noun tokens). Exactly 24 unannotated fixtures were appended. All IDs and new inputs are unique; all eight categories, four mode/filler combinations, and both command settings remain covered. Two expectations were corrected after source review: a numeric-bearing scientific-notation-like token retains the space before `!`, and consecutive `new line`, `new paragraph`, `new line` commands emit four additive line breaks. No production cleanup/settings behavior, settings schema/defaults, dependency, lockfile, or policy changed.

The 192-to-200 milestone appends exactly one fixture in each existing category:

1. **Whitespace:** leading blank lines, ASCII horizontal whitespace, an em space, and a trailing line break, with the exact three normalized breaks preserved.
2. **Commands:** an enabled `new paragraph` between existing source line breaks and Unicode words; the two source breaks plus two generated breaks remain additive.
3. **Vocabulary:** overlapping `acme` / `acme lab` phrases select the longer match, preserve its intentional `Acme , Lab` spacing, and clean eligible punctuation outside the replacement.
4. **Punctuation:** an eligible comma is tightened while `3 . 14` and the ambiguous curly-quoted `?!”` sequence stay unchanged.
5. **Fillers and modes:** a paired Clean/Verbatim comparison uses identical input (`um, glorp , works !`), vocabulary, filler flag, and command flag. Clean removes the supported prefix; Verbatim preserves it while current vocabulary and punctuation stages still run.
6. **Unicode:** a decomposed accent is matched under the current Unicode 18 policy only at a complete boundary; a following Japanese word blocks a partial match, while Japanese punctuation separates the later complete occurrence.
7. **Safety:** multiline Clean input keeps an accented name and digit-bearing numeric punctuation intact, preserves a written `North , um, Wing` replacement byte-for-byte, removes a supported unprotected line-leading filler, and cleans eligible punctuation outside the replacement.

A one-off parsed-object comparison against base commit `ba1a7d1fb4491e628ac1950c19b78083aefa0735` confirms that the first 192 fixture objects are unchanged in order, including all 16 original protected-token annotations across 12 fixtures. Exactly eight cases were appended, one per category; the Clean/Verbatim pair is the only repeated input among the additions. All 200 IDs are unique, the new inputs are absent from the earlier corpus, and all four mode/filler combinations and both command settings remain covered. The policy remains **PROPOSED — NOT APPROVED**.

After all 200 cases pass, the test prints:

```text
Cleanup golden corpus: 200 fixtures passed
Categories covered: 8
Protected numeric tokens: 8 (0 altered)
Protected proper nouns: 8 (0 altered)
Current behavior baseline: true
AC-07 acceptance: NOT EVALUATED
```

The current annotations are selected examples, not a universal guarantee that every number or proper noun will be preserved in arbitrary speech. They are additional checks on top of full exact-output comparisons and do not expand beyond the same 16 selected tokens.

AC-07 status after this numerical milestone:

| Requirement | Status after this task |
|---|---|
| At least 200 current-behavior fixtures | Met numerically, if all 200 pass |
| Exact outputs on those 200 fixtures | Verified for the characterization baseline only |
| Approved final acceptance corpus | Not established |
| ≥99% exact output against final acceptance expectations | Not evaluated |
| Numeric/proper-noun preservation | Verified for the existing 16 selected annotations only |
| Final Clean/Verbatim semantics | Not approved |
| Production ASR/application integration | Not completed |
| AC-07 overall | Open |

Run the focused corpus and Clean/Verbatim contract tests, then the requested workspace checks from the repository root:

```sh
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_golden_corpus --locked -- --nocapture
RUSTUP_TOOLCHAIN=stable cargo test -p outflow-settings --test cleanup_mode_contract --locked
RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check
RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked
RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked
git diff --check
# Run after staging only the intended task files.
git diff --cached --check
```

Troubleshooting:

- **JSON or schema error:** check commas and quoting, keep the top-level value an array, include every required field, use an allowed category and mode, and remove unknown fields. Each vocabulary object needs both `spoken` and `written`.
- **Fixture vocabulary is rejected:** use nonempty valid spoken and written phrases and avoid duplicate spoken phrases under the existing matching policy.
- **Protected range is out of bounds or selects the wrong text:** recount bytes from the beginning of the exact input or expected-output UTF-8 string. The end offset is exclusive; do not reuse an input range when cleanup changes the preceding byte count. UTF-8 offsets count encoded bytes, not visible characters; combining marks and emoji can occupy multiple bytes, and both endpoints must be character boundaries.
- **Range splits a Unicode character:** UTF-8 ranges must begin and end at character boundaries. Count the encoded bytes, not displayed characters; for example, `É` uses two UTF-8 bytes.
- **Protected token was altered or does not match its declared spelling:** compare the range-selected input, expected-output, and actual-output slices byte-for-byte. Preserve capitalization and Unicode composition exactly; do not normalize or copy a replacement produced by vocabulary mapping.
- **Unicode vocabulary mismatch:** inspect the exact code points in the spoken phrase and transcript (precomposed accents and base-plus-combining-mark spellings may compare canonically). Check that no letter, number, combining mark, joiner, or other recognized continuation extends the match boundary. Keep the written replacement exactly as configured; do not normalize it.
- **Unicode whitespace differs from expectations:** only ASCII space and tab runs are trimmed/collapsed. Nonbreaking, thin, narrow, em, and ideographic spaces are copied unchanged; CRLF and standalone CR become LF. Compare exact code points and line endings instead of applying broader Unicode normalization.
- **A filler-looking phrase is not removed:** the current rule recognizes only line-leading ASCII `um, ` or `uh, ` followed by an alphabetic character. A missing comma, a tab/multiple separator before normalization, punctuation or a quote before the word, indentation, numbers, and repeated supported fillers are intentionally retained to avoid false-positive deletion.
- **A vocabulary phrase partly matches before a combining mark:** combining marks count as word continuations. The entire candidate should remain unchanged when a configured phrase would stop before that mark; inspect the Unicode 18 boundary rule and the exact input code points instead of widening the match.
- **Whitespace mismatch:** check each line independently. Leading/trailing ASCII spaces and tabs are trimmed, interior ASCII runs collapse to one space, and each LF, CRLF, or standalone CR remains one LF. Non-ASCII whitespace is copied unchanged. Count source breaks separately from any newlines inserted by commands.
- **Punctuation mismatch:** only `, . ! ? ; :` can be tightened, only when the prior character is alphabetic and the adjacent token contains no number, and only when the mark is followed by whitespace or the end. Ambiguous punctuation sequences—including spaced ellipses and punctuation before quote delimiters—remain unchanged, as do candidate spaces intersecting protected replacement bytes.
- **Expected output mismatch:** inspect the named fixture ID and category against the current ordered stages. Correct the explicitly reviewed expected value only when evidence shows the baseline value is wrong; do not silently regenerate it from the implementation.
- **Unexpected newline count:** whitespace normalization preserves every existing LF/CR/CRLF break, including breaks on otherwise blank lines. A recognized `new paragraph` also inserts two newlines and leaves surrounding existing breaks in place, so count the command output and source breaks independently.
- **Canonical-equivalent vocabulary output contains a repeated suffix:** the mapper replaces only the matched spoken span and copies unmatched input bytes. If the written phrase already includes a following source word, include that whole spoken phrase in the mapping when that is the intended characterization.
- **Written replacement spaces or punctuation look unusual:** replacements are copied exactly and are not normalized afterward. A punctuation-spacing candidate that touches protected replacement bytes remains unchanged; inspect whether the separator belongs to the replacement or the transcript.
- **A replacement after earlier text edits appears to lose protection:** punctuation and filler stages carry and adjust replacement byte ranges after earlier deletions. Check whether the affected bytes are actually inside the written replacement; bytes after its end remain eligible for their ordinary cleanup rules.
- **Punctuation beside a number stays spaced:** the current rule trims punctuation spacing only when the preceding character is alphabetic and its adjacent alphanumeric token contains no number. Existing fixtures record `2026 .`; the new protected Unicode replacement also records `0 ,` after its numeric written suffix. Preserve these outputs and document any proposed behavior change separately.
- **A policy change is proposed:** the corpus records current behavior. Review affected expectations alongside an approved product decision; the recommendation in [docs/CLEANUP_MODE_POLICY.md](docs/CLEANUP_MODE_POLICY.md) remains **PROPOSED — NOT APPROVED**.

These 200 cases remain a current-behavior characterization only; their exact-output passes do not establish an approved acceptance corpus or prove the ≥99% production threshold. They do not complete T-035/T-036 or AC-07: T-035 prerequisites, full idempotence requirements, final Clean/Verbatim semantics, the acceptance fixture set, and production ASR/application integration remain open. The 16 protected annotations are selected examples, not a universal preservation guarantee. Numeric-adjacent punctuation such as `2026 .`, `6.02e23 !`, and replacement-adjacent numeric punctuation remains spaced under existing conservative guards. Command-generated breaks preserve source breaks, and protected replacements preserve intentional whitespace. The Clean/Verbatim proposal remains **PROPOSED — NOT APPROVED**. T-017, T-018, T-023, T-035, T-036, T-038, T-039, AC-07, AC-08, and all release gates remain unchecked.

## Independent AC-07 exact-output scorer

[`scripts/score_cleanup_ac07.py`](scripts/score_cleanup_ac07.py) compares independently supplied expected and actual strings in memory. It counts byte-for-byte exact string matches as represented by Python strings: punctuation, capitalization, whitespace, and Unicode code points must all match, with no normalization. It reports the corpus size, exact matches, mismatches, percentage, the 200-case minimum, and the 99% exact-output threshold. The threshold is decided with integer arithmetic, so a displayed percentage cannot round a failing score into a pass.

Run the synthetic demonstration from the repository root:

```sh
python3 scripts/score_cleanup_ac07.py --demo
```

Expected output:

```text
AC-07 exact-output scorer: SYNTHETIC DEMO
Cases: 200
Exact matches: 198
Mismatches: 2
Exact-output rate: 99.00%
Minimum corpus size: PASS
Exact-output threshold: PASS
Protected-token acceptance: NOT EVALUATED
Final Clean/Verbatim policy: NOT APPROVED
AC-07 acceptance: NOT EVALUATED
```

In application code, call `scripts.score_cleanup_ac07.score_cleanup_cases()` with iterable records containing unique nonempty string IDs and string `expected` and `actual` fields. Empty output strings are valid. A corpus can pass the percentage check while failing the minimum count (for example, 199/199); both checks must pass for numerical eligibility. The 99% check uses `exact_matches * 100 >= total_cases * 99`; for example, 198/201 fails even though rounding the rate to a whole percentage would show 99%.

Run the focused tests and the complete standard-library script suite:

```sh
python3 -m unittest scripts.tests.test_score_cleanup_ac07 -v
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
```

Troubleshooting:

- **At least one case is required:** pass a nonempty iterable of records.
- **Invalid record or field error:** each record must be a mapping with `id`, `expected`, and `actual`; IDs must be nonempty unique strings, and both outputs must be strings. Empty strings are permitted for outputs.
- **Unexpected mismatch:** compare exact strings, including capitalization, punctuation, whitespace, and Unicode composition. The scorer deliberately does not normalize them.
- **Demo passes:** this verifies only synthetic arithmetic. It does not run Rust cleanup, read `cleanup_current_behavior.json`, evaluate protected tokens, or establish AC-07 acceptance. See the proposed [AC-07 evaluation plan](docs/AC07_EVALUATION_PLAN.md) for the remaining evidence gates.

## Independent AC-07 protected-token evaluator

[`scripts/score_cleanup_protected_ac07.py`](scripts/score_cleanup_protected_ac07.py)
checks exact UTF-8 byte preservation for explicitly annotated numeric tokens
and proper nouns. It is independent of
[`score_cleanup_ac07.py`](scripts/score_cleanup_ac07.py), which measures
corpus size and the exact-output percentage. The protected-token evaluator
does not invoke Rust cleanup, read the characterization corpus, derive token
ranges or expected text from actual output, or establish AC-07 acceptance.

Run its two-case synthetic demonstration and focused tests from the repository
root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/score_cleanup_protected_ac07.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_score_cleanup_protected_ac07 -v
```

Expected demonstration summary:

```text
AC-07 protected-token scorer: SYNTHETIC DEMO
Cases: 2
Annotated cases: 2
Protected numeric tokens: 1 (0 altered)
Protected proper nouns: 1 (0 altered)
Both token kinds represented: PASS
Protected-token gate: PASS (synthetic only)
AC-07 acceptance: NOT EVALUATED
```

Supply records in memory to `score_protected_cleanup_cases()`:

```python
{
    "id": "case-001",
    "input": "Dr . Élodie",
    "expected": "Dr. Élodie",
    "actual": "Dr. Élodie",
    "protected_tokens": [
        {
            "kind": "proper_noun",
            "text": "Élodie",
            "input_bytes": [5, 12],
            "output_bytes": [4, 11],
        }
    ],
}
```

Supported kinds are `number` and `proper_noun`. Ranges are zero-based,
half-open UTF-8 **byte** offsets, not character indexes. The input range must
select the declared token in `input`; the output range must independently
select it in `expected`. Both ends must lie on UTF-8 character boundaries.
The evaluator checks `actual` only at the declared expected-output range and
does not search for or relocate a token.

The returned immutable summary includes case counts, per-kind protected and
altered counts, kind coverage, zero-alteration status, and the protected-token
gate. That gate passes only if both kinds are annotated and neither kind has
alterations. A preceding insertion or deletion in `actual` can shift a token
away from its expected byte range and therefore count conservatively as an
alteration. This behavior is intentional.

Troubleshooting:

- **Invalid range or token mismatch:** count UTF-8 encoded bytes independently
  in `input` and `expected`. A punctuation edit can make their ranges differ.
  End offsets are exclusive, and neither range may split a multibyte character.
- **Altered token:** inspect `actual` at the exact declared `output_bytes`
  offset. The scorer intentionally does not relocate tokens after insertions,
  deletions, or other output changes.
- **Gate does not pass with zero alterations:** at least one valid annotation
  of both `number` and `proper_noun` is required for gate completeness.
- **Validation error:** fix the case or annotation schema. Invalid reference
  metadata rejects evaluation; an actual-output mismatch is reported as an
  alteration count instead.

The exact-output and protected-token tools have separate inputs, validations,
and results. For formal AC-07 evaluation, both must ultimately use the same
independently approved corpus and its corresponding production outputs. The
synthetic unit tests and demo validate evaluator behavior only; they do not
certify cleanup quality, comprehensive preservation, or AC-07 acceptance.
See the [AC-07 evaluation plan](docs/AC07_EVALUATION_PLAN.md) for the
remaining evidence requirements.

## Shared AC-07 evaluation coordinator

[`scripts/evaluate_cleanup_ac07.py`](scripts/evaluate_cleanup_ac07.py)
coordinates the exact-output and protected-token scorers using one in-memory
case collection. It validates each complete record and its token annotations
with the existing protected-token evaluator, then projects the same cases to
the exact-output scorer. It returns both independent score objects and fails
if their case counts differ. It does not calculate a combined percentage or
return an AC-07 acceptance decision.

Run the synthetic demonstration and focused tests from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/evaluate_cleanup_ac07.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_evaluate_cleanup_ac07 -v
```

Expected demo output:

```text
AC-07 combined evaluator: SYNTHETIC DEMO
Cases: 200
Exact matches: 198
Mismatches: 2
Exact-output rate: 99.00%
Minimum corpus size: PASS
Exact-output threshold: PASS
Protected numeric tokens: 1 (0 altered)
Protected proper nouns: 1 (0 altered)
Protected-token gate: PASS (synthetic only)
Shared case count: PASS
Final Clean/Verbatim policy: NOT APPROVED
AC-07 acceptance: NOT EVALUATED
```

Call `evaluate_cleanup_ac07_cases(cases)` with records in this shared format:

```python
{
    "id": "case-001",
    "input": "Dr . Élodie met 2026",
    "expected": "Dr. Élodie met 2026",
    "actual": "Dr. Élodie met 2026",
    "protected_tokens": [
        {
            "kind": "proper_noun",
            "text": "Élodie",
            "input_bytes": [5, 12],
            "output_bytes": [4, 11],
        }
    ],
}
```

`protected_tokens` is required but may be empty on individual records. Across
the evaluated corpus, the protected-token gate requires valid annotations for
both `number` and `proper_noun`, with zero altered tokens. Offsets are
zero-based, half-open UTF-8 byte ranges: `input_bytes` selects the token in
`input`, and `output_bytes` selects it in `expected`.

Troubleshooting:

- **Missing field:** include `id`, `input`, `expected`, `actual`, and
  `protected_tokens` on every record. Each annotation needs `kind`, `text`,
  `input_bytes`, and `output_bytes`.
- **Invalid annotation or UTF-8 offset:** verify that the ranges select the
  declared token in the corresponding input and expected strings, stay within
  bounds, and start and end on UTF-8 character boundaries. Fix the reference
  metadata; invalid annotations fail the whole evaluation.
- **Duplicate ID:** assign each record a unique, nonempty string `id`.
- **Protected-token gate fails with no alterations:** annotate at least one
  numeric token and one proper noun. Empty annotation lists remain valid but
  do not provide token-kind coverage.
- **Exact-output threshold and protected-token gate differ:** the gates answer
  separate questions. An unprotected punctuation mismatch can reduce exact
  accuracy while preserving every annotated token; a protected-token change
  can fail preservation even when exact accuracy remains at or above 99%.

Passing both synthetic gates is not formal acceptance. AC-07 still requires
an independently reviewed reference corpus, approved Clean/Verbatim semantics,
production cleanup results, and integration evidence. The existing 200
characterization fixtures are not designated as acceptance data, and the PRD
thresholds remain unchanged.

## AC-07 corpus metadata manifest

The versioned metadata-only manifest records a corpus identity and revision,
case IDs, the eight cleanup fixture categories, modes and settings, provenance
references, review declarations, planned protected-token kinds, and lowercase
SHA-256 digests for future input and expected-reference bytes. The manifest
does not contain transcripts or outputs. The standard-library validator checks
the schema and digest format; it does not load case data, authenticate review
evidence, or establish corpus approval or AC-07 acceptance. See the
[manifest contract](docs/AC07_CORPUS_MANIFEST.md).

Run the synthetic draft demonstration and focused tests from the repository
root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_manifest.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_manifest -v
```

Expected demo output:

```text
AC-07 corpus manifest: SYNTHETIC DRAFT
Schema version: 1
Cases indexed: 3
Unique case IDs: PASS
Categories represented: 3/8
Clean and Verbatim represented: PASS
Content hashes: FORMAT VALIDATED ONLY
Reference review: PENDING
Corpus approval: NOT ESTABLISHED
AC-07 acceptance: NOT EVALUATED
```

Each digest must be exactly 64 lowercase hexadecimal characters. The synthetic
draft's digests are checked in tests against explicitly constructed synthetic
source strings; those strings are not stored in the metadata file. The
metadata-only validator verifies hash syntax only. The separate synthetic
bundle and binding validator below verify content-to-manifest correspondence.
A future evaluation must also independently approve references and production
outputs before it sends the same records to both AC-07 scorers.

Troubleshooting:

- **Malformed hash:** compute SHA-256 over the exact UTF-8 bytes intended for
  the separately supplied case data; encode it as 64 lowercase hex digits.
- **Duplicate JSON key or field:** keep one instance of each required key and
  remove all unknown keys. Both root and case objects use strict schemas.
- **Reviewed candidate is rejected:** include a nonempty `review_evidence_ref`
  when declaring the candidate reviewed. The validator checks only that the
  reference is present; a separate reviewer must verify it.
- **Demo says hashes were format-validated only:** this is expected. No case
  data is supplied to the manifest validator, so it cannot confirm digest-to-
  content correspondence.
- **Demo says corpus approval is not established:** this is expected for the
  pending synthetic draft. Metadata validation never designates or approves
  acceptance data.

## AC-07 synthetic reference-bundle binding

[`scripts/validate_cleanup_ac07_reference_bundle.py`](scripts/validate_cleanup_ac07_reference_bundle.py)
checks that the separate reference bundle's corpus ID, revision, and exact
case IDs match the manifest, then hashes each decoded input and expected string
as exact UTF-8 with SHA-256. The bundle is matched by case ID, so its array may
be reordered. No Unicode normalization, whitespace trimming, line-ending
replacement, or hash repair occurs. Strict schemas reject missing/unknown
fields, duplicate JSON keys and IDs, unsupported versions, and invalid
strings. Errors and successful output do not expose case content.

Run the synthetic demonstration and focused tests from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_reference_bundle.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_reference_bundle -v
```

Expected demo output:

```text
AC-07 reference-bundle binding: SYNTHETIC DEMO
Corpus ID: ac07-synthetic-manifest-demo
Revision: draft-001
Manifest cases: 3
Bundle cases: 3
Case-ID correspondence: PASS
Input SHA-256 hashes verified: 3/3
Expected SHA-256 hashes verified: 3/3
Reference review: PENDING
Corpus approval: NOT ESTABLISHED
AC-07 acceptance: NOT EVALUATED
```

The manifest's hashes are checked as declared; they are never recalculated or
updated by the validator. Matching proves only that these exact bundle strings
correspond to those manifest digests. It does not establish that expected
outputs are correct or independently approved, and it does not run production
cleanup or either scorer. The checked-in three-case bundle is synthetic test
data, not acceptance data.

Troubleshooting:

- **Corpus ID or revision mismatch:** use the exact identity declared in the
  manifest; the validator does not rewrite either value.
- **Case IDs do not correspond:** include each manifest ID exactly once and no
  additional IDs. Array order does not matter.
- **Input or expected hash mismatch:** restore the exact intended string, or
  author a separately reviewed manifest revision. Do not normalize, trim, or
  automatically update the digest to make a bundle pass.
- **Duplicate field or ID, missing field, or unsupported field:** keep one
  instance of every required field and remove all unknown fields.
- **Invalid UTF-8 string:** remove unpaired surrogate code points; the error
  reports only the case position and field.
- **Demo passes:** this is synthetic reference-integrity verification only.
  Reference review remains pending, corpus approval is not established, and
  AC-07 acceptance is not evaluated.

## AC-07 synthetic protected-reference annotation binding

[`scripts/validate_cleanup_ac07_protected_references.py`](scripts/validate_cleanup_ac07_protected_references.py)
binds a separate protected-token annotation bundle to the existing manifest and
SHA-256-verified reference bundle. It requires the same corpus ID, revision,
and exact case-ID set; annotation kinds must match each case's declared
`protected_kinds`. It uses the existing protected-token scorer to check that
each token's independent input and expected ranges select the declared text at
valid, nonoverlapping UTF-8 byte boundaries. No token text or transcript is
printed.

Run the synthetic demonstration and focused tests from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_protected_references.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_protected_references -v
```

The tool validates annotation-to-reference correspondence only. Manifest
`protected_kinds` are declarations, reference hashes bind supplied strings to
those declarations, and successful annotation ranges show that the selected
bytes match the declared tokens. Those checks do not authenticate independent
review, prove that production cleanup preserves the tokens, or evaluate
acceptance. The scorer is called with `actual=expected` only so its existing
reference-range checks can be reused; all alteration and gate results are
ignored. Actual-output preservation remains a later evaluation step after
independent review and production execution.

In the checked-in demonstration, `demo-001` uses the numeric substring `001`
with different input and expected offsets. `demo-003` labels the `AC-07`
identifier as a synthetic `proper_noun` only to demonstrate the contract; this
does not show natural-language proper-noun coverage. The expected summary ends
with `Corpus approval: NOT ESTABLISHED` and `AC-07 acceptance: NOT EVALUATED`.

## AC-07 synthetic actual-output identity binding

The four-artifact relationship is **manifest → hash-bound reference bundle →
validated protected annotations → separately supplied actual outputs**.
[`scripts/validate_cleanup_ac07_actual_outputs.py`](scripts/validate_cleanup_ac07_actual_outputs.py)
first calls the existing protected-reference validator, then validates an
independently supplied output bundle against that verified identity. It
associates cases by exact, case-sensitive ID, independent of array order, and
checks exact corpus ID and revision, explicit source declarations, complete
case correspondence, and UTF-8 actual strings. Empty actual strings are
retained. The binder does not generate actual values from expected strings,
normalize output, compare expected and actual values, inspect protected-token
preservation, or call the shared scorer/coordinator.

`synthetic_demo` outputs require a null `execution_evidence_ref`.
`production_declared` outputs require a nonempty reference string, but that
reference is not proof that production code ran. A synthetic-purpose manifest
rejects a production declaration. Even for an acceptance candidate, the
summary reports production execution as unverified.

Run the demonstration and focused tests from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_actual_outputs.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_actual_outputs -v
```

Expected demonstration output:

```text
AC-07 actual-output binding: SYNTHETIC DEMO
Corpus ID: ac07-synthetic-manifest-demo
Revision: draft-001
Run ID: synthetic-run-001
Reference cases: 3
Actual-output cases: 3
Case-ID correspondence: PASS
Actual UTF-8 strings validated: 3/3
Output source: SYNTHETIC DEMO
Production execution: NOT VERIFIED
Reference review: PENDING
Corpus approval: NOT ESTABLISHED
AC-07 acceptance: NOT EVALUATED
```

Output identity validation does not establish cleanup correctness. The
existing shared coordinator may eventually score verified references against
corresponding actual outputs after approved references, approved Clean/Verbatim
semantics, authenticated production execution, and integration evidence are
available. This task does not invoke it. The authoritative AC-07 thresholds
and all task/release status remain unchanged; the Clean/Verbatim policy is
still **PROPOSED — NOT APPROVED**.
