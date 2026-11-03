# Prompt 79D — Detailed Implementation Handoff

**Status:** Engineering corrections and native feasibility measurements completed; ASR selection remains `BLOCKED — INSUFFICIENT EVIDENCE`.
**Date:** 2026-11-03
**Scope:** T-002, T-003, T-004, T-006, T-009, T-010; F-06 / AC-06 and runtime/fixture evidence.
**Base commit:** `4be4e7b226f5a16d1c2104a66aa41d591d6adbca` (Prompt 79C reviewed starting commit).
**Branch:** `codex/prompt79d-feasibility`.
**Repository:** `thrns/outflow`.

This record distinguishes checked-in implementation, commands that ran in this environment, source/evidence inspection, and external requirements that remain open. It contains no audio, transcript, reference text, model weights, native binary, hostname, serial number, or absolute paths to the user's external assets.

## A. Executive summary

Before Prompt 79D, the repository had a native Moonshine Small Streaming evaluator and synthetic WER validator, an eight-row planned-only fixture manifest, a 100 ms process-tree RSS sampler, local Small model hash evidence, and a Prompt 79C native smoke whose ten-run maximum sampled RSS was `1,023,426,560` bytes. The measured evaluator could not calculate WER because no ready fixture had a human-verified reference. Hardware classification accepted an approximate installed-memory boundary, measured results did not carry actual loaded-library identity, and corpus coverage could pass from only short/long labels.

Prompt 79D makes corpus diversity explicit, separates installed RAM facts from hardware certification, verifies the actual macOS dyld-loaded helper/Moonshine/ONNX Runtime files, upgrades measured native results to schema version 2, and validates every result in memory before an output file can be created. New direct tests cover the identified verification gaps. A real paired memory/timing/load experiment on the existing Small Streaming build found a configuration effect: default-thread RSS still exceeded the hard cap, while an upstream single-thread reliability switch lowered RSS at a substantial latency cost and is not selected for production.

What now runs: fixture, asset, pinned-source, build-scope, notice-inventory, native build, native streaming smoke, runtime identity, and local regression checks. What does not run to completion: measured corpus WER, a real 4 GB reference benchmark, Tiny evaluation, a Tauri process-tree measurement, and redistribution approval. The fixture manifest has zero ready rows. `evidence/model/MODEL_DECISION.md` remains `BLOCKED — INSUFFICIENT EVIDENCE`; no production app or model selection is authorized by these measurements.

## B. File-by-file implementation

### Runtime and scoring implementation

- [`scripts/validate_fixture_manifest.py`](../../scripts/validate_fixture_manifest.py#L131) — `assess_corpus_coverage(fixtures)` counts independent `duration_class`, `coverage_attributes`, and `speaker_variety` fields. Coverage requires 100 short speech rows, 50 long speech rows, quiet and noisy speech, two distinct source-reported English varieties, proper nouns, technical vocabulary, numbers, abbreviations, a silence control, and verified provenance and human reference status for every ready row. `validate_manifest` remains the entry point and accepts old planned rows without new optional fields; ready rows must supply them. It reports validation errors without creating audio or references and refuses `collection_status: complete` when any minimum is missing. These rules support C-05 / AC-06 and run before the evaluator reads audio. The criteria are documented as minimum inclusion criteria, not statistical representativeness. Tests are in `scripts/tests/test_validate_fixture_manifest.py`.

- [`scripts/run_native_asr_benchmark.py`](../../scripts/run_native_asr_benchmark.py#L288) — `verify_native_runtime` canonicalizes the selected files, runs the helper's `--runtime-identity` mode, verifies active architecture and C API version, checks the resolved path containing `moonshine_get_version`, requires the exact helper/Moonshine/ORT images in dyld's active image list, checks `lipo -archs` and `otool -L`, rejects unexpected non-system images and ambiguous dependencies, and compares SHA-256 before/after inspection. On macOS, system shared-cache paths can lack a separately hashable file; only documented Apple system roots are accepted, and the sanitized report records counts and hashes rather than local paths.

- [`scripts/run_native_asr_benchmark.py`](../../scripts/run_native_asr_benchmark.py#L397) — `verify_source` checks the pinned clean Moonshine source revision and C API/model architecture constants; `verify_model_assets` validates the local eight-file Small inventory; `load_verified_fixtures` validates exact fixture bytes, metadata, hashes, permissions/provenance, and human references before inference. `machine_record` obtains exact installed RAM bytes from `sysctl hw.memsize` on macOS but always marks hardware status unknown; a RAM query cannot certify physical/reference status. `assess_physical_4gb_eligibility` requires at most `4,000,000,000` bytes, independently verified physical status, and an allowlisted hardware class. The current class allowlist is empty.

- [`scripts/run_native_asr_benchmark.py`](../../scripts/run_native_asr_benchmark.py#L694) — `build_measured_result` holds hypotheses and references only while scoring, includes the detailed corpus coverage record and per-fixture metrics, and excludes transcript/audio content. CPU provider, physical eligibility, actual hardware measurements, resource gates, and quality gates are separate fields. `run_benchmark` checks the runtime before loading or scoring and selects CPUExecutionProvider through the native helper. `_write_result` at line 867 validates the whole schema before opening a new file in exclusive-create mode; it does not replace existing evidence. Empty or invalid fixtures fail before native inference.

- [`scripts/validate_measured_asr_results.py`](../../scripts/validate_measured_asr_results.py#L135) — `_validate_machine` checks exact physical-memory bytes/source, physical status, class, CPU-only status, and whether real-hardware measurements were completed. `validate_measured_results` at line 182 verifies the schema, source/model/binary identity, runtime assertions, per-fixture and aggregate WER arithmetic, corpus coverage, timing percentiles, release-gate consistency, duplicate JSON keys, and forbidden transcript/audio/path/identity keys. It cannot independently establish truth of a run; provenance is tied to the runner's hashes and the recorded measurement method. Tests are in `scripts/tests/test_validate_measured_asr_results.py`.

- [`scripts/verify_native_runtime.py`](../../scripts/verify_native_runtime.py#L18) — standalone CLI for actual macOS runtime-linkage verification when no ready fixtures exist. Its stdout is sanitized JSON; errors go to stderr without dumping loaded image paths or captured native output. It fails closed on unsupported platforms or incomplete identity evidence.

- [`tests/native/moonshine_asr_batch.cpp`](../../tests/native/moonshine_asr_batch.cpp#L146) — development-only native batch helper. `load_transcriber` explicitly requests `CPUExecutionProvider`; `run_session` streams bounded 300 ms PCM chunks, polls partials, stops/drains final output, and measures wall/CPU/RTF/stop-to-final timings. Hypotheses exist only in the helper's private stdout protocol for the Python runner to capture in memory. `--runtime-identity` reports the executable path, the resolved image containing the Moonshine C API symbol, architecture, and dyld paths to the verifier; `--load-probe` reports cold/warm model-load timings. Do not invoke the normal inference mode directly in a terminal.

- [`tests/native/moonshine_streaming_smoke.cpp`](../../tests/native/moonshine_streaming_smoke.cpp#L87) — transcript-free smoke helper that loads Small with `ort_providers=CPU`, runs two sequential 12-second sessions, requires nonempty partial/final output and completed final lines, reports timings and `getrusage` maximum RSS, then frees streams/transcriber. It prints no recognition text. It does not report stage-exact current RSS or allocator mappings.

### Tests added or extended

- [`scripts/tests/test_validate_fixture_manifest.py`](../../scripts/tests/test_validate_fixture_manifest.py#L87) covers short/long-only false completeness, full independent coverage, duplicate IDs, malformed metadata, unverified inputs, missing references, and unsupported category/attribute combinations.
- [`scripts/tests/test_native_runtime_identity.py`](../../scripts/tests/test_native_runtime_identity.py#L65) covers exact image acceptance, symbol from mismatched Moonshine, absent loaded libraries, wrong architecture, changed hashes, ambiguous ORT dependencies, unexpected non-system images, sanitized CLI output, and conservative CLI failure.
- [`scripts/tests/test_validate_measured_asr_results.py`](../../scripts/tests/test_validate_measured_asr_results.py#L165) covers malformed/unsupported records, invalid WER arithmetic, impossible audio/timing metadata, forbidden content, invalid runtime identity, incomplete corpus coverage, malformed hardware facts, more than 4 GB, simulated/unknown status, unverified measurements, and duplicate JSON keys.
- [`scripts/tests/test_native_asr_benchmark.py`](../../scripts/tests/test_native_asr_benchmark.py#L209) was extended for the stricter source/runtime/hardware schema and result validation before persistence. It verifies a corrupt measured record is rejected before a file is opened.

### Documentation and evidence

The fixture policy/schema are synchronized in `docs/BENCHMARK_FIXTURES.md` and `docs/benchmark-fixtures.json`; measured-v2 and runtime/hardware rules are in `docs/BENCHMARK_RESULTS.md`. `docs/TASKS.md` retains all original feasibility task checkboxes as open and records this milestone without marking AC-06 complete. `evidence/model/MODEL_DECISION.md` contains the blocked Small/Tiny decision and the paired measurements. Licensing supplements are in `docs/MOONSHINE_LICENSE_REVIEW.md`, `docs/MOONSHINE_THIRD_PARTY_AUDIT.md`, `docs/MOONSHINE_BUILD_SCOPE.md`, `docs/THIRD_PARTY_NOTICES.md`, and `evidence/mvp/prompt-79d-licensing-review.md`. The standard KISS FFT BSD-3-Clause text is now present at `docs/licenses/KISS-FFT-BSD-3-Clause.txt`; its product packaging and legal review remain open.

## C. Execution/data flow

1. **Fixture validation:** `validate_fixture_manifest.py` parses schema v1, keeps legacy planned rows valid, validates required metadata/hashes for ready rows, and independently computes coverage. `collection_status: complete` is rejected unless the declared minimums pass. The checked-in eight rows remain planned.
2. **Fixture bytes and source/model validation:** `run_benchmark` calls `load_verified_fixtures`, which resolves paths inside the repository and verifies audio/reference hashes, PCM metadata, permission, provenance, and human review. It then verifies the exact upstream source and eight local model asset hashes. No download path is called.
3. **Actual native identity:** the runner executes the compiled helper's identity-only mode. dyld reports the active image list and `dladdr` reports the library containing the C API symbol. Python resolves and compares paths, checks `lipo`/`otool`, hashes the three selected binaries around the probe, and emits sanitized identity fields.
4. **Model load and PCM streaming:** the helper's load probe runs explicitly on CPU. For evaluation, the C++ helper parses verified PCM16 mono WAV, loads Small Streaming with CPU provider selection, pushes 300 ms sample chunks to a C API stream, polls partials, stops and drains final text, and captures wall time, CPU time, RTF, partial/final status, and stop-to-final latency.
5. **Transcript collection and WER:** helper stdout is captured in memory by `_run_json_command`. Python parses it without echoing it, compares each speech hypothesis against the separate human reference using the frozen scorer, and aggregates substitution/deletion/insertion counts and WER. Silence controls are checked for no-speech behavior. Hypotheses, references, audio, and absolute paths are deleted from local result collections before a sanitized result is returned.
6. **Result validation:** the runner calls `validate_measured_results` before persistence, and `_write_result` repeats validation immediately before exclusive-create. The validator recomputes coverage, WER arithmetic, timing, identity-hash matching, and all release-gate statuses. Synthetic numerical WER stays on the separate schema-v1 path.
7. **Memory measurement:** process-tree RSS is a separate measurement branch using `scripts/measure_process_tree_rss.py` at 100 ms. It captures the launched native process and currently visible descendants, with PID/start identity and summed RSS; it does not record args, env, audio, or transcript output. The 20-run paired evidence is `evidence/mvp/prompt-79d-native-memory-measurements.json`. It is not inserted into a measured WER result and does not include a Tauri process tree.

## D. Model and memory investigation

### Controlled native configuration

- Source revision: `234f60faa0eb388b01cdf7e60aca232af37aefda`, clean pinned Moonshine checkout.
- Host: macOS `26.6.2`, arm64 Apple M3 Pro, installed RAM `19,327,352,832` bytes; not an eligible 4 GB reference system.
- Native library build: out-of-tree CMake Release target `moonshine`, arm64, `MOONSHINE_TTS_BUILD_ONNX=ON`; same shared library for paired runs. No upstream source patch was applied. The CMake source still includes optional TTS/G2P and diarization paths.
- Model: locally verified English Small Streaming, 8 assets totaling `142,300,974` bytes; exact observed per-file hashes are in `evidence/mvp/prompt-79-model-assets.json`. The dated upstream catalog path is not an immutable artifact revision or redistribution approval.
- Provider/audio/sessions: explicit `CPUExecutionProvider`; same pinned `two_cities_librivox_48k.wav` 12-second speech slice, two sequential streaming sessions per process; 100 ms process-tree RSS sampler. This smoke audio has no verified Outflow reference.
- Native helper: Release arm64 executable, `39,792` bytes, SHA-256 `0eefb71563a9a29bc04da5396e977d1d94feebd72fa90c2a2038ed5a50cabeb6`. `libmoonshine.dylib`: `10,164,432` bytes, SHA-256 `42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5`. ONNX Runtime 1.23.2 dylib: `27,623,992` bytes, SHA-256 `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`. Helper/source/model identities and all per-run values are captured in the memory evidence JSON.
- Repeated-memory runs: 10 default-thread processes and 10 processes with the pinned source's `MOONSHINE_ORT_SINGLE_THREAD=1`; every process ran two sessions. No failed or unfavorable run was discarded. Sampler max gaps were 110 ms default and 123 ms single-thread. RSS sums can double-count shared pages; polling can miss short peaks.

### Paired measurements

| Metric | Default ONNX Runtime threads | `MOONSHINE_ORT_SINGLE_THREAD=1` |
|---|---:|---:|
| Ten sampled process-tree peaks (bytes) | 829,472,768; 949,501,952; 931,577,856; 924,762,112; 930,217,984; 1,010,614,272; 897,400,832; 963,952,640; 938,295,296; 928,088,064 | 791,756,800; 801,144,832; 795,459,584; 907,902,976; 800,456,704; 795,459,584; 829,095,936; 863,059,968; 815,661,056; 889,585,664 |
| Median / p95 / max sampled RSS | 930,897,920 / 989,616,538 / 1,010,614,272 | 808,402,944 / 899,660,186 / 907,902,976 |
| At or above 950,000,000 bytes | 2/10 | 0/10 |
| Session p50 / p95 | 1,002.713 / 1,214.004 ms | 3,170.911 / 3,269.501 ms |
| RTF p50 / p95 | 0.0835 / 0.1011 | 0.2640 / 0.2721 |
| Native CPU ms p50 / p95 | 5,428.504 / 6,160.906 | 3,169.251 / 3,260.373 |
| Stop-to-final p50 / p95 | 0.010 / 0.065 ms | 0.027 / 0.032 ms |
| `getrusage` maximum RSS | 1,020,248,064 bytes | 907,902,976 bytes |

The single-thread experiment reduces median sampled RSS 13.16% and the maximum 10.16%, but increases p95 session wall time about 2.69×. In the pinned source, it sets intra/inter-op thread counts to one and sequential execution. `ort-utils.h` describes it as a TSan/reliability escape hatch; production builds do not set it. It is evidence that the current thread/session configuration contributes to memory use, not an approved production optimization. Its 907,902,976-byte max leaves only `42,097,024` bytes under the native threshold before adding the desktop app.

The separate ten-process-per-mode model-load probes performed ten loads per process. Default process-cold p50/p95 was `122.025/326.166 ms`, with same-process warm reload p50/p95 `82.827/92.754 ms`; single-thread cold p50/p95 was `145.115/160.387 ms`, warm reload p50/p95 `100.631/106.899 ms`. Their max process-tree RSS was `508,903,424` bytes and `496,533,504` bytes respectively. OS file-cache state was uncontrolled.

One extra final native smoke run after the build check succeeded on CPU and reported partial/final output in both sessions; native `getrusage` maximum was `969,637,888` bytes. This is an additional single-run smoke observation, not part of the paired 10-run sampled-RSS statistics.

### Memory attribution and rejected alternatives

The observed model-load peak is materially below the streaming peak, pointing to inference/session work as a major transient contributor. The runtime creates five ORT sessions (frontend, encoder, adapter, cross-KV, decoder-KV) and source inspection shows direct ORT model bytes, environment allocators, disabled prepacking, disabled CPU arena, and graph optimizations already configured. There are no `vmmap` region reports, phase-exact current RSS samples, native thread-count snapshots, or memory samples after each stream/transcriber release. The evidence cannot distinguish ORT thread-pool reservations, streaming/decoder buffers, mappings, cache retention, allocator fragmentation, or leaks. Ten fresh-process runs and two sequential sessions per process are not an endurance/leak test.

The TTS-off CMake experiment did not produce an STT-only C API library: the C API still references TTS headers/symbols and asset-catalog functions, and diarization sources are appended unconditionally. No safe supported configuration change was adopted. The smallest approval-dependent alternative is an upstream-supported, ABI-compatible target/interface gate that excludes unneeded TTS and diarization while preserving the pinned streaming C ABI, followed by equivalent real inference, license, memory, and quality remeasurement. Do not edit the pinned Moonshine source in Outflow without architecture approval.

Tiny was not acquired or tested. The pinned source identifies `MOONSHINE_MODEL_ARCH_TINY_STREAMING=2`, a dated English quantized catalog path, 34M parameters, and source-declared lengths totaling `77,748,675` bytes including the optional word-timestamp decoder (`45,233,659` without it). These are not verified files or installed-byte evidence. The upstream-reported Tiny WER is 12.00%, exactly Outflow's current threshold with no demonstrated headroom. Acquiring those assets requires separate explicit authorization, exact file/license/hash review, and the same tests as Small.

## E. Test verification

### Automated tests and builds

| Command | Result | What it proves / limits |
|---|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v` | **PASS:** 369 tests; 1 skipped | All Python tests pass. The skipped integration test requires a verified licensed corpus and configured external paths; its skip is not a pass for real corpus evaluation. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | **PASS** | Rust formatting only. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | **PASS** | Existing Rust workspace type/check build; no native Tauri application is built. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | **PASS:** 353 listed unit/integration test names and 1 doctest | Existing Rust behavior; no Prompt 79D Rust runtime changes. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | **PASS** | Existing Rust targets are warning-free. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | **PASS** | Existing Rust workspace build only. |
| `npm run typecheck` | **PASS** | Existing frontend TypeScript check. |
| `npm run test -- --run` | **PASS:** 10 test files, 136 tests | Existing web UI tests. |
| `npm run lint` | **PASS** | Existing frontend lint. |
| `npm run build` | **PASS** | Typecheck plus Vite browser bundle; not a Tauri installer/app build. |
| `git diff --check` | Run again after final documentation edits before staging | Whitespace/conflict-marker check. |

The repository's focused correction suite was run separately: fixture manifest, native ASR benchmark, native runtime identity, and measured-result validator tests: **52 passed**. New runtime identity tests directly simulate missing libraries, mismatched symbol path, wrong architecture, changing hash, ambiguous ONNX dependency, unexpected image, and verifier failure. New results tests directly challenge malformed schema, impossible metrics, unauthorized physical-memory claims, incomplete coverage, and forbidden content. Tests do not make WER or physical 4 GB claims.

### Native and data checks

| Command/check | Result | Scope |
|---|---|---|
| `python3 scripts/moonshine_model_source_audit.py --source-dir <pinned source>` | **PASS source contract / artifact incomplete** | Exact revision/header/architecture and catalog contract; reports artifact revision/rights and runtime notices as incomplete. |
| `python3 scripts/validate_model_asset_inventory.py --manifest evidence/mvp/prompt-79-model-assets.json --asset-root <local Small directory>` | **PASS file integrity** | Eight local files, 142,300,974 bytes; zero redistribution approvals; not immutable vendor identity. |
| `python3 scripts/validate_fixture_manifest.py` | **PASS schema / NOT READY** | Eight planned rows, 0 ready, every coverage count 0. |
| `python3 scripts/validate_moonshine_third_party.py` | **PASS inventory / approval undetermined** | Structural source/binary inventory only. |
| `python3 scripts/validate_third_party_notices.py` | **PASS structure / notice readiness incomplete** | Eight dependencies, zero packaged notices, four blocked notices. |
| `python3 scripts/validate_moonshine_build_scope.py` | **PASS source record / minimal build not assessed** | Six required and six excluded scope entries; 11 unresolved build questions. |
| `cmake --build <local Release wrapper> --config Release --target moonshine --parallel 4` | **PASS** | Pinned native target builds on this macOS arm64 host with TTS ON. |
| `clang++ -std=c++17 ... moonshine_streaming_smoke.cpp ...` plus run | **PASS native smoke** | CPU provider, real partial/final recognition in two sessions; transcript-free output. Most recent single run showed 969,637,888-byte `getrusage` max. |
| `clang++ -std=c++20 ... moonshine_asr_batch.cpp ...`; `scripts/verify_native_runtime.py ...` | **PASS runtime identity** | One arm64 helper/runtime set. Sanitized record: helper SHA `e4a9bed5daf5ed0f7d7bc0979111a75ea327e6dda254c569a486c4d541846685`, Moonshine SHA `42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5`, ONNX Runtime SHA `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`; active image count 365; no unexpected non-system image. |
| `scripts/run_native_asr_benchmark.py ... --output <new file>` | **EXPECTED BLOCK / exit 2** | Actual invocation confirmed `no verified ready fixtures are available for native evaluation`; asserted no output file was created. |
| `scripts/validate_measured_asr_results.py --file <measured result>` | **NOT RUN against a real measured record** | No real result exists because there are 0 ready fixtures. Direct validator tests pass on controlled in-memory fixtures. |

## F. Performance and quality evidence

- **Ready fixtures:** 0 of the required frozen corpus. Eight manifest entries are planning rows. No reference file is human-verified; no audio/reference hashes are ready.
- **WER:** `NOT RUN`; no aggregate/per-fixture scores, word counts, substitutions, deletions, or insertions exist. The upstream smoke WAV is not a WER fixture.
- **Recognition:** 20 repeated smoke sessions per configuration across 10 processes; default and one-thread metrics are in section D and the raw sanitized measurement record. The same 12-second speech slice was reused. These samples are not p50/p95 over diverse utterances.
- **Process-tree RSS:** default max `1,010,614,272` bytes; 2/10 at or above the strict `950,000,000` byte threshold. One-thread max `907,902,976`; 0/10 at or above threshold. `getrusage` max in default mode `1,020,248,064` shows that 100 ms polling missed a short peak.
- **Load timing:** cold/warm probes in section D, ten fresh helper processes per mode, ten loads per process; disk cache state uncontrolled.
- **Native library sizes:** Small model `142,300,974` bytes; Moonshine `10,164,432`; ONNX Runtime `27,623,992`; smoke helper `39,792`. These are selected dev inputs, not complete installed application size.
- **Host:** macOS 26.6.2 on Apple M3 Pro, arm64, `19,327,352,832` physical RAM bytes. Not the 4 GB reference class; no physical 4 GB CPU-only benchmark occurred.
- **Complete app resources:** not measured. No Tauri shell or helper tree was built. The experimental single-thread max is too close to the cap to establish sufficient app headroom.
- **AC-06:** `NOT_ASSESSED`. Corpus, human-reviewed references, hardware, full-app memory, and human sign-off are absent, regardless of smoke RTF.

## G. Incomplete work

| Task / acceptance | Completed implementation/evidence | Missing proof, dependency, and next action |
|---|---|---|
| **T-002 / AC-01** physical hardware matrix | Exact host RAM source and strict physical/reference-class model are represented in result schema; >4,000,000,000 bytes is ineligible. | No approved reference-class allowlist or physical 4 GB host evidence. Identify a real target class and document OS/build/CPU/RAM, then run the full CPU-only benchmark there. No VM limit can replace this. |
| **T-003 / AC-06** corpus | Independent duration and diversity attributes, coverage assessment, provenance and human-reference readiness gates are implemented. | 0 ready fixtures. Common Voice's current [terms](https://commonvoice.mozilla.org/dav/terms) ask not to mirror any dataset subset outside Mozilla Data Collective; written clarification and legal review are needed before repository copies. [LibriSpeech SLR12](https://us.openslr.org/12/) lists CC BY 4.0, but exact files, attribution, file-level permission/provenance and human audio/reference review remain to be completed. Freeze at least 100 short + 50 long and all diversity gates. |
| **T-004 / AC-01** RSS | Existing sampler and tests retained; 20 paired native tree runs captured, including unfavorable runs; native `getrusage` maxima included. | No physical-footprint/PSS or mapped-region profile; no phase current RSS/thread counts; no full Tauri helper tree or endurance run. Integrate stage memory/thread snapshots if useful, then profile the complete app on reference hardware. |
| **T-005** installed-byte utility | Previously completed by the repository with its own evidence. | The complete installer still has no measurement; this milestone adds none. |
| **T-006 / AC-09** insertion | Prior read-only macOS AX probe returned `AX_NOT_TRUSTED`; this milestone did not request or bypass permissions. | No editable/protected controls were tested. Requires user-granted Accessibility and a separately scoped safe insertion experiment after prerequisites. |
| **T-009 / AC-01** licensing | Verifies one local runtime image set; adds the KISS FFT BSD-3-Clause text; checks source and notice inventories. | ONNX Runtime binary provenance and matching full notices; TTS/G2P, VCTK, diarization/community-1 and other transitive terms; exact Small artifact rights/revision; package destinations and legal/human sign-off. Keep `redistribution_approval: undetermined`. |
| **T-010 / AC-06** candidate decision | Paired Small memory/timing/load measurements; source-level Tiny fallback requirements; model decision updated. | No WER corpus, eligible hardware, approved production memory setting, Tiny artifacts/run, complete app RSS/size, package license clearance, or signed human decision. Small default misses RSS; one-thread is not a production setting. Do not mark T-010 complete. |
| **T-011/T-013** app and model packaging | No application shell or bundle change was started. | Remain gated on T-009/T-010; exact artifact identity/rights and native ABI/build contract are unresolved. |

The central task ledger keeps T-002, T-003, T-004, T-006, T-009, and T-010 unchecked. AC-06 and model selection remain blocked. No desktop application task is complete due to native smoke evidence.

## H. Reproduction guide

The full command sequence is in [`README.md`](../../README.md#prompt-79d-reproduction-feasibility-corrections-and-paired-memory-runs). External local paths used for the observed run were a clean Moonshine source checkout, a CMake Release wrapper/build, and the locally authorized eight-file Small model directory, all outside Git. The source checkout, build directory, model directory, and upstream smoke WAV must exist before running; this handoff does not assume they are vendored.

Source/model/fixture checks:

```sh
python3 scripts/moonshine_model_source_audit.py --source-dir "$MOONSHINE_SOURCE_DIR"
python3 scripts/validate_model_asset_inventory.py \
  --manifest evidence/mvp/prompt-79-model-assets.json --asset-root "$MODEL_DIR"
python3 scripts/validate_fixture_manifest.py
```

Build the already-configured pinned source target:

```sh
cmake --build "$BUILD_DIR" --clean-first --target moonshine \
  --config Release --parallel 4
```

Build the two Outflow development helpers, run standalone runtime identity and the two-session smoke using the same commands in the README. The smoke WAV is `test-assets/two_cities_librivox_48k.wav` inside the external pinned checkout; the Small model assets live outside the repository. The batch helper's normal stdout contains private hypotheses; invoke only through `run_native_asr_benchmark.py`, which captures them in memory.

The README contains a ten-run loop for each exact tested mode. It uses `measure_process_tree_rss.py --interval-ms 100`, two sessions per process, the same WAV slice/model/library/build, and separate fresh output files. Use the exact `MOONSHINE_ORT_SINGLE_THREAD=1` environment variable for the experimental comparison. Do not treat this upstream reliability escape hatch as a selected app configuration.

The real evaluator invocation is also in the README. Against the present planned-only manifest, expect exit code 2 and no results file. Once ready fixture files are lawfully available and human-verified, run `python3 scripts/validate_measured_asr_results.py --file <new measured JSON>` only on the emitted measured schema-v2 result. The synthetic `scripts/validate_asr_results.py` file remains a separate, non-ASR numerical example.

Run all Python/Rust/frontend checks with the exact commands recorded in README. The native runtime identity CLI is macOS-specific and fails closed elsewhere; other OSes need their own verified loader-inspection evidence before measured result identity can be claimed.

## I. GitHub delivery

- Starting commit: `4be4e7b226f5a16d1c2104a66aa41d591d6adbca`.
- Working branch: `codex/prompt79d-feasibility`.
- Changed code: fixture manifest evaluator, measured ASR runner/result validator, standalone runtime verifier, native batch/smoke helpers, focused Python tests.
- Changed docs/evidence: README, benchmark fixture/results guidance, task ledger, model decision, Moonshine build/license/third-party/notice records, KISS FFT text, runtime identity evidence, paired native memory evidence, licensing review, and this handoff.
- Excluded from Git: model weights, upstream smoke audio, generated native binaries, raw process output and private WER hypotheses. The committed memory/identity JSON contains only sanitized machine/build hashes and numeric evidence.
- Final commit reference: `origin/codex/prompt79d-feasibility` is the branch ref containing this handoff. Resolve its current SHA with `git rev-parse HEAD` and verify it against `git ls-remote origin refs/heads/codex/prompt79d-feasibility`; the delivery response records the exact verified SHA and GitHub commit URL because a commit cannot embed its own hash.
- Worktree state at delivery: clean; local `codex/prompt79d-feasibility` tracks `origin/codex/prompt79d-feasibility`. The exact final SHA is confirmed in the delivery response.

## J. Next engineering milestone

Production app prerequisites are **not satisfied**. The shortest next proof sequence is:

1. Obtain a lawful file-level English corpus path and a human transcription reviewer; create, hash, and verify the ready fixtures until all independent coverage gates pass.
2. Identify and verify an approved physical 4 GB reference machine, then run Small on CPU with complete process-tree memory and the frozen corpus. Keep the exact installed byte and class evidence.
3. Resolve a supported production memory configuration or an approved upstream STT-only C API design, then rerun measured CPU quality/latency/RSS tests. The single-thread reliability escape hatch cannot be adopted based on this experiment.
4. Finish artifact-specific Small/runtime license and notice review. If Small still fails resources or quality, obtain explicit authorization before acquiring Tiny files and repeat the exact asset, licensing, quality, latency, and resource gates.
5. Record the maintainer's signed model decision. If no candidate satisfies all requirements, retain the no-go and stop.

Only after T-010 and its bound quality, resource, and licensing gates pass should a later milestone begin Tauri shell work. Prompt 79D does not start Prompt 80.
