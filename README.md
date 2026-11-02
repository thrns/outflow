# Outflow

Outflow is an offline desktop dictation application in development. This repository contains a browser-based React foundation screen and Settings preview, a Rust workspace foundation, and a native Rust settings schema v1 model with JSON serialization, validated decoding, explicit-path loading and initialization, an explicitly confirmed reset, and an atomic JSON writer on Unix. The Tauri desktop shell, microphone and dictation flows, Windows owner-only ACL writing, and release packaging have not been implemented.

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

The Cargo workspace is separate from the React/Vite app. `outflow-core` remains dependency-free; `outflow-settings` uses pinned Serde and `serde_json` dependencies for schema v1 JSON:

```text
Cargo.toml
rust-toolchain.toml
crates/
├── core/
│   ├── Cargo.toml
│   ├── src/lib.rs
│   └── examples/foundation.rs
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

The `outflow-core` library currently provides a development-only status function. Run its example from the project root:

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
- **Reset returns `ReplacementFailed`:** replacement did not occur and the prior file remains. Check that the parent directory is accessible and writable, then resolve the filesystem issue before invoking reset again.
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

This proves one pre-replacement failure path on the local Unix filesystem. It does not prove power-loss or crash durability, actual disk-full recovery, temporary-file creation or rename failure handling, Windows ACL/replacement behavior, or Linux filesystem behavior. The writer synchronizes the parent directory after a successful rename, but this test does not simulate a crash around that metadata update. Task 54 cancellation is an in-memory review-draft operation; it is separate from this test's real filesystem preservation check and does not establish persistent migration rollback. T-018, T-017, T-038, T-039, T-053, and AC-08 remain incomplete; no schema migration is implemented. See the [Task 55 audit evidence](evidence/rust-settings-writer-recovery-task-55.md) for the coverage matrix and full verification record.

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

General contains **Launch at Login**, which defaults to **Off**. Use the switch with a mouse or focus it and press Space. The visible On/Off state and accessible switch state update immediately. The preference is kept only in React memory: it remains selected while changing sections or closing and reopening Settings in the same mounted session, and resets to Off after a full page reload. Text & Vocabulary includes a frontend-only vocabulary draft form and live synthetic example. Privacy shows a static overview of intended protections and preview limitations; Microphone and Shortcuts show their existing preview status messages, and About & Diagnostics remains a placeholder.

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

The starter screen and Settings layout/navigation preview are implemented, with one browser-only Launch at Login preference. The dependency-free `outflow-core` Rust foundation provides a smoke-test status message; `outflow-settings` provides schema v1 defaults, JSON serialization/deserialization, validation, explicit-path loading and safe initialization, explicit confirmed reset, and explicit-path atomic writes on Unix. The reset and settings filesystem behavior were tested on macOS 26.6.2 arm64 only; Linux and Windows were not tested. Migration, Windows ACL writing, and frontend persistence remain unimplemented. The React preference does not persist or affect computer startup, and it is not connected to the Rust model. The other Settings sections remain placeholders; no Reset button, confirmation dialog, Tauri command, Home features, reusable component library, Tauri shell, microphone capture, or dictation behavior is included yet. T-001, T-017, and T-018 remain unchecked; T-017 is incomplete because migration behavior and its T-001 prerequisite remain outstanding, and T-018 is incomplete because the cross-platform writer requirements are not met. F-11, AC-11, AC-12, T-051, and native startup integration remain incomplete. The Rust crates are development foundations, not application runtime features.

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

The catalog labels English Small Streaming and Tiny Streaming as MIT, and Moonshine's policy says all streaming STT models are MIT. The policy separately identifies a limited set of legacy, non-English, non-streaming models under the Community License. Moonshine's own repository code excludes `core/third-party`; those subdirectories follow supplier terms. The review records 6 upstream-documented claims and 3 unverified items. It does not identify or hash a model artifact or native runtime, assemble the complete notices, or approve redistribution. Redistribution remains **undetermined**, and T-009 remains incomplete.

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

Moonshine has not been configured, compiled, linked, measured, or runtime-tested for this plan. No model or native library was downloaded, and no binary contents are verified. T-009 remains unchecked; T-010, T-013, AC-01, and AC-06 remain incomplete.

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

[`docs/MOONSHINE_CMAKE_CONFIGURE.md`](docs/MOONSHINE_CMAKE_CONFIGURE.md) records the isolated configure runner and the actual Task 25 attempt. The runner accepts only a clean Git checkout at Moonshine revision `234f60faa0eb388b01cdf7e60aca232af37aefda`, verifies the six pinned CMake files with the Task 24 probe, checks upstream's CMake minimum, and configures into a temporary out-of-source directory. It uses Python's standard library and removes the temporary build tree after inspecting the cache, target graph, compiler selections, and ONNX Runtime library path.

When the exact checkout and compatible CMake are available, run:

```sh
python3 scripts/try_moonshine_cmake_configure.py \
  --source-dir /path/to/pinned/moonshine
```

The runner supplies `CMAKE_BUILD_TYPE=Release`, `MOONSHINE_TTS_BUILD_ONNX=OFF`, `MOONSHINE_RELIABILITY=OFF`, and `FETCHCONTENT_FULLY_DISCONNECTED=ON`. It does not build, install, or test Moonshine targets. `CONFIGURED` (exit 0) reports CMake generation only; it does not prove a minimal STT binary, working native linkage, runtime compatibility, or release readiness. `BLOCKED` (exit 2) means the pinned source, CMake, a required dependency, or an environmental capability is unavailable. `FAILED` (exit 1) means source integrity or an unexpected configure assumption failed.

Task 25 is currently **BLOCKED**: Git could not resolve `github.com` while preparing the source-only checkout, and CMake is not installed. No source revision, target graph, or ONNX Runtime library was verified during this attempt. Network isolation also remains unverified because `FETCHCONTENT_FULLY_DISCONNECTED=ON` does not disable every possible configure-time network request.

Troubleshooting:

- **`BLOCKED: Source directory is unavailable` or revision mismatch:** prepare a clean checkout at the exact commit above with `GIT_LFS_SKIP_SMUDGE=1`; do not substitute a package subdirectory, another revision, or an LFS pointer.
- **`BLOCKED: CMake executable is not available` or unsupported version:** use a CMake installation that meets the minimum read from the pinned `core/CMakeLists.txt`; do not install dependencies as part of the experiment without authorization.
- **`BLOCKED` for a missing dependency:** read the bounded CMake diagnostics. Supply only already-approved source dependencies; do not fetch ONNX Runtime binaries or model weights for this task.
- **ONNX Runtime path is missing or an LFS pointer:** the configure result does not make that file usable. The runner reports LFS pointers as non-binaries and never treats their pointer text as a native library.
- **CMake reports `CONFIGURED`:** inspect the target list and cache, but keep binary contents and minimal STT exclusions unproven. The TTS subtree, tests, benchmarks, utilities, diarization, and auxiliary sources have no verified general exclusion control in the reviewed source.

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

This is a preview only: entries are not saved, edited, deleted, exported, or applied to dictated text. There is no vocabulary list, duplicate detection, 500-entry enforcement, or replacement processing.

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
