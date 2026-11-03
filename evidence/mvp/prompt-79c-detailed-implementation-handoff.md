# Prompt 79C Detailed Implementation Handoff

**Date:** 2026-11-03
**Milestone:** Prompt 79C correction; this is not Prompt 80.
**Repository:** `thrns/outflow`
**Branch:** `codex/prompt79c-feasibility`
**Base commit:** [`2303916c2066ade0fbffb493b1d09ed1b5f86a94`](https://github.com/thrns/outflow/commit/2303916c2066ade0fbffb493b1d09ed1b5f86a94)
**Implementation commit:** [`b2a1e5b51c66ab9aabae1bb46a9f5d129486e79e`](https://github.com/thrns/outflow/commit/b2a1e5b51c66ab9aabae1bb46a9f5d129486e79e). The final branch head after this handoff metadata update is reported by the delivery message; a commit cannot contain its own hash without changing that hash.
**Status:** Partial engineering progress committed for review; T-002/T-003/T-004/T-006/T-009/T-010 remain incomplete.
**Platform used:** macOS 26.6.2 (25G83), Darwin 25.6.0, Apple M3 Pro arm64, 18 GB RAM.
**No release decision:** `BLOCKED — INSUFFICIENT EVIDENCE`.

## A. Executive implementation summary

### Before this milestone

Prompt 79 supplied an out-of-process native Moonshine C API smoke harness and a local macOS arm64 build. The pinned source was `234f60faa0eb388b01cdf7e60aca232af37aefda`, with C API header version `30000` and `MOONSHINE_MODEL_ARCH_SMALL_STREAMING = 4`. Eight Small Streaming files were locally present and their observed SHA-256 values were recorded, totaling 142,300,974 bytes. The smoke test had produced nonempty partial and final recognition in two sessions, but Outflow itself still had no Tauri shell, Rust ASR binding, microphone capture, cleanup path, insertion adapter, or Copy Result interaction. Prompt 79 also had a synthetic WER schema, planned-only fixture rows, preliminary license inventories, and an RSS observation from the native harness.

### Added in Prompt 79C

- `scripts/measure_process_tree_rss.py` samples the target and currently visible descendants every 100 ms by default, preserves timestamped per-process RSS in bytes, tracks process start identities and sampler gaps, and refuses measurements it cannot trust. It supports macOS `ps` and Linux `/proc`; the operating-system-specific process discovery is isolated from deterministic aggregation logic.
- `tests/native/moonshine_asr_batch.cpp` is a development-only native bridge for bounded PCM16 mono WAV input, the pinned Small Streaming C API, load probes, final hypotheses, and recognition/CPU timings. Raw hypotheses travel only over captured stdout to the Python caller.
- `scripts/run_native_asr_benchmark.py` validates the frozen fixture manifest, exact audio/reference hashes and WAV metadata, pinned source/header/architecture, all eight model hashes, and native binary identities before it runs inference. It scores in-memory hypotheses with the frozen WER scorer and emits only sanitized measured metrics. `scripts/validate_measured_asr_results.py` keeps measured records separate from the existing synthetic schema and rejects transcript/path/identity fields.
- `scripts/score_wer.py` and `scripts/score_corpus_wer.py` now use the explicit Unicode lexical policy `lexical-wer-unicode-boundary-v1`, with deterministic alignment and aggregate counts. `scripts/validate_fixture_manifest.py` now requires file-level hashes, measured audio format/duration, provenance and human-review evidence before a row can become ready. `scripts/validate_model_asset_inventory.py` distinguishes the model catalog path, unavailable upstream artifact revision, and locally observed SHA-256 values.
- A native build and notice audit records the exact local Moonshine/ONNX build, the upstream CMake TTS-off failures, compiled optional code/data, and remaining attribution and human-review gaps. The earlier Prompt 78 research label is explicitly distinguished from canonical T-078.
- README reproduction commands, task progress, fixture policy, benchmark-result policy, licensing inventories, and the model decision record were updated. No dependency, Cargo lockfile, npm lockfile, pinned upstream source, model asset, audio fixture, or native binary was added to the repository.

### What is executable now

The native smoke can be rebuilt and run; a new native batch helper can load the pinned model and measure real inference on valid mono PCM16 WAV fixtures; the benchmark runner and validators can process a lawful frozen corpus; the process-tree sampler can measure standalone native processes; and focused deterministic tests exercise error handling, privacy boundaries, process ancestry, byte units, and result arithmetic. The local model asset validator checks all eight existing files against their recorded hashes.

### What remains conceptual or blocked

The corpus manifest has 8 planned rows and **0 ready fixtures**, so no corpus WER or p50/p95 corpus latency was measured and the real integration test is skipped. The current native build exceeds the 950,000,000-byte sampled RSS limit in 4 of 10 repeated smoke runs. The only physical host was an 18 GB M3 Pro. No verified 4 GB machine, Tiny artifacts, full installed app, artifact-specific redistribution approval, approved complete notice bundle, or Accessibility trust was available. No real text controls were tested. The measured native helper is not an application interface or production runtime.

## B. File-by-file implementation inventory

### Source and tests

| File | Responsibility, interface, ownership, and validation |
|---|---|
| [`scripts/measure_process_tree_rss.py`](../../scripts/measure_process_tree_rss.py#L41) | New standard-library profiler for C-10 evidence. `ProcessInfo` and `Sample` hold process PID/PPID, start identity, executable basename, RSS bytes, timestamp and total. `parse_macos_ps_snapshot`, `_linux_process_snapshot`, `get_process_snapshot`, `select_process_tree`, `make_sample`, `_sample_until`, `build_measurement`, and `main` separate OS reads from tree aggregation, process lifecycle, validation, and JSON output. CLI can launch after `--` or attach to a PID for a fixed duration. It never stores command arguments, environment values, audio, transcripts, or complete executable paths. Tests cover ancestry, startup/termination, duplicate PID prevention, PID reuse, permission/process disappearance, missed short-lived processes, max-gap rejection, peak detection, byte units, and sanitized output. |
| [`scripts/tests/test_measure_process_tree_rss.py`](../../scripts/tests/test_measure_process_tree_rss.py#L34) | New deterministic process-snapshot tests for the profiler. It tests injected snapshots and CLI plumbing rather than depending on nondeterministic process-tree memory values. |
| [`tests/native/moonshine_asr_batch.cpp`](../../tests/native/moonshine_asr_batch.cpp#L48) | New test-only native bridge; it is not linked into Outflow. `read_pcm16_mono_wav` bounds input to 64 MiB/120 seconds, validates RIFF/WAVE, PCM16, mono and sample rate, then converts signed samples into `float` in `[-1,1)`. `run_session` streams 300 ms chunks, periodically polls partial output, stops once, obtains final output, records recognition/stop-to-final/RTF and process CPU time, then frees stream/transcriber handles. `collect_final_text` copies the native transcript before the next C API call invalidates it. `--load-probe` measures repeated model loads. Stdout is a private JSON protocol containing hypotheses; the caller must capture it and must not print or persist it. |
| [`scripts/run_native_asr_benchmark.py`](../../scripts/run_native_asr_benchmark.py#L88) | New Python orchestration for T-003/T-010. `load_verified_fixtures` enforces ready status, rooted paths, SHA-256, PCM metadata and reference constraints; `verify_source` requires the exact clean Moonshine revision and C API constants; `verify_model_assets` hashes all required model assets; `parse_load_probe` and `parse_native_output` reject malformed native results; `build_measured_result` scores volatile hypotheses and creates transcript-free per-fixture and aggregate metrics; `_write_result` uses exclusive creation to avoid overwriting prior evidence; `run_benchmark` validates before executing the native helper and removes transcript-bearing intermediate objects before returning. It fails closed if there are no ready fixtures or native inference fails. |
| [`scripts/tests/test_native_asr_benchmark.py`](../../scripts/tests/test_native_asr_benchmark.py#L71) | New unit tests for JSON parsing, WER normalization/alignment, aggregate counts, deterministic output, native failure redaction, load/inference protocol validation, unsupported audio, and the current zero-ready-fixture block. |
| [`scripts/tests/test_native_asr_integration.py`](../../scripts/tests/test_native_asr_integration.py#L27) | New opt-in fixture-backed native integration test. It requires eight `OUTFLOW_BENCHMARK_*` paths/values, then invokes the real runner, validates the output, and checks nonzero measured fixture count. It skips if required local binaries/corpus are missing or no fixture is ready. It skipped in this environment. |
| [`scripts/validate_measured_asr_results.py`](../../scripts/validate_measured_asr_results.py#L80) | New separate measured-results validator. It checks strict root and nested fields, pinned source/model identity, hashes, model and binary byte counts, timestamps, hardware fields, per-fixture metrics and aggregate arithmetic; recursively rejects transcript, reference, path, command-line, environment, hostname and identity fields. It does not accept the synthetic schema as measured evidence. |
| [`scripts/score_wer.py`](../../scripts/score_wer.py#L40) | Modified lexical tokenization and Levenshtein alignment. NFC, lowercase, collapsed whitespace and Unicode letter/number/mark tokens are used; punctuation is a token boundary. Number spelling, fillers and vocabulary are not rewritten. Stable diagonal/deletion/insertion tie order feeds per-fixture counts. Existing scorer tests and the new evaluator tests cover Unicode and deterministic results. |
| [`scripts/score_corpus_wer.py`](../../scripts/score_corpus_wer.py#L43) | Modified to reuse the same versioned normalization and sum substitutions, deletions, insertions, reference words and utterances for corpus WER. Synthetic arithmetic remains distinct from native measured result files. |
| [`scripts/validate_fixture_manifest.py`](../../scripts/validate_fixture_manifest.py#L121) | Modified fixture readiness checks. In addition to containment and provenance, ready entries require audio and reference files/hashes, supported measured PCM metadata, nonempty human-verified references and a documented review method. Planned rows remain valid but do not count as ready. |
| [`scripts/tests/test_validate_fixture_manifest.py`](../../scripts/tests/test_validate_fixture_manifest.py) | Modified tests add coverage for measured audio metadata, reference checksums, and required human review/provenance conditions. |
| [`scripts/validate_model_asset_inventory.py`](../../scripts/validate_model_asset_inventory.py#L190) | Modified model inventory validation to allow an unavailable artifact revision only when the provenance note states it is unavailable, while independently verifying each local artifact SHA-256. The source commit, dated catalog path, local hashes and redistribution approval remain distinct facts. |
| [`scripts/tests/test_validate_model_asset_inventory.py`](../../scripts/tests/test_validate_model_asset_inventory.py) | Modified tests ensure a vendor catalog path or observed file hash is not misrepresented as an immutable model artifact revision or redistribution approval. |

### Documentation and evidence

| File | Change and status |
|---|---|
| [`README.md`](../../README.md#L1200) | Added exact model/source checks, native build/smoke and evaluator commands, profiler use, regression suite commands, safe failure expectations, and current result labels. The existing Small model preparation instructions remain development-only and do not authorize distribution. |
| [`docs/TASKS.md`](../../docs/TASKS.md#L30) | Updated progress accurately; T-002, T-003, T-004, T-006, T-009, and T-010 stay unchecked. T-005 remains complete only for its utility; no application or release gate is marked done. |
| [`docs/BENCHMARK_FIXTURES.md`](../../docs/BENCHMARK_FIXTURES.md#L1) | Added schema behavior and two public corpus research leads checked 2026-11-03. Common Voice and LibriSpeech are not selected or ready; neither has been acquired or hashed into this repo. |
| [`docs/BENCHMARK_RESULTS.md`](../../docs/BENCHMARK_RESULTS.md#L1) | Clarified that synthetic arithmetic schema cannot certify ASR and that measured native results have their own runner/schema; current zero ready fixtures block a result file. |
| [`docs/benchmark-fixtures.json`](../../docs/benchmark-fixtures.json) | Extended the existing v1 planned-only rows with source release, reference hash, WAV metadata, human review method, and frozen transcription/normalization policy versions. Eight planned entries; zero ready. |
| [`docs/MOONSHINE_BUILD_SCOPE.md`](../../docs/MOONSHINE_BUILD_SCOPE.md) | Reconciled source-only build claims with actual Prompt 79 native build evidence; no false minimal-STT compile pass. |
| [`docs/MOONSHINE_LICENSE_REVIEW.md`](../../docs/MOONSHINE_LICENSE_REVIEW.md) | Added a Prompt 79C supplement distinguishing this historical source-policy review from the separate local model/binary hash and build-scope evidence. |
| [`docs/MOONSHINE_THIRD_PARTY_AUDIT.md`](../../docs/MOONSHINE_THIRD_PARTY_AUDIT.md) and [`docs/moonshine-third-party.json`](../../docs/moonshine-third-party.json) | Added actual runtime/build-only/optional data scope and outstanding attribution/provenance questions. License identifiers remain separate from clearance. |
| [`docs/THIRD_PARTY_NOTICES.md`](../../docs/THIRD_PARTY_NOTICES.md) and [`docs/third-party-notices.json`](../../docs/third-party-notices.json) | Aligned the planned notice inventory with the audit questions. Result remains 0 packaged, 4 planned, 4 blocked, redistribution undetermined. |
| [`evidence/mvp/prompt-79-model-assets.json`](prompt-79-model-assets.json) | Preserved all eight observed SHA-256 values and file-verified state; set unavailable upstream artifact revision to null and documented the vendor catalog path separately. No production manifest is created. |
| [`evidence/mvp/prompt-79c-native-license-audit.md`](prompt-79c-native-license-audit.md) | New exact source, build, binary hash, dynamic link, TTS-off failure, compiled optional code/data, notices and human-review evidence. T-009 remains incomplete. |
| [`evidence/model/MODEL_DECISION.md`](../model/MODEL_DECISION.md) | New candidate record: `BLOCKED — INSUFFICIENT EVIDENCE`; current native RSS subgate fails; WER, physical 4 GB, Tiny, installer size and redistribution remain unverified. |
| [`evidence/model/moonshine-small-streaming-requirements-task-78.md`](../model/moonshine-small-streaming-requirements-task-78.md) | Preserved historical research evidence while clarifying “Prompt 78” is not canonical T-078, the signed installer beta. |

## C. Actual execution and data flow

### Native smoke

`tests/native/moonshine_streaming_smoke.cpp` remains the two-session smoke harness. It loads the local Small files at architecture 4 and the pinned C API, reads the upstream LibriVox WAV excerpt, feeds real audio through Moonshine, checks nonempty partial and final results, and prints only counts/booleans and timing rather than transcript text. The final rebuilt run on this machine reported two sessions with partial and final output, RTF 0.095 and 0.079, and a sampled native-process max RSS of 904,904,704 bytes. This is a short smoke, not WER evidence.

### Corpus benchmark design

The intended executable path is:

```text
frozen manifest + exact audio/reference files
  → validate permission/review state, root-contained paths, SHA-256, and WAV metadata
  → native helper converts bounded PCM16 mono samples to float32
  → pinned Moonshine C API streams 300 ms chunks, polls partials, stops, and returns finals
  → Python captures raw native JSON in memory, pairs each final with its human reference
  → Unicode lexical alignment computes per-fixture counts and aggregate corpus WER
  → timings/RTF/CPU/model/source/machine/hash data are sanitized and written once
  → measured-results validator checks schema, identity, arithmetic, and privacy fields
```

`run_native_asr_benchmark.py` owns the validation/scoring/result boundary. The C++ helper owns the native transcriber and stream handles, PCM sample buffer, and volatile hypothesis strings; it frees each stream and the transcriber on success or failure. The native transcript pointer is copied before another C API transcript call because the pinned API owns that buffer. Native stdout contains hypotheses and must only be captured into Python memory. The Python code does not echo captured stdout/stderr in errors, and its persisted result does not include hypotheses, references, audio paths, raw commands, environment values, hostnames, usernames or serial numbers. The result is created with exclusive mode, so it cannot silently overwrite earlier evidence.

This end-to-end corpus flow is implemented but **not yet completed on a real verified fixture**: corpus validation stops with `no verified ready fixtures are available for native evaluation`. The actual native helper was separately built and run on the upstream smoke WAV; those smoke measurements are not presented as corpus WER or as measured-result JSON.

## D. Detailed test evidence

| Test or command | What it exercises | Category | Observed result | What it does not prove |
|---|---|---|---|---|
| `python3 -m unittest discover -s scripts/tests -v` | Existing and new Python validators, WER, profiler, evaluator, inventories | Unit/integration | **343 tests; 342 passed, 1 skipped; overall PASS.** Skip is the real corpus integration because required artifact/corpus env is absent and there are zero ready fixtures. | No corpus WER or real fixture-backed evaluator integration. |
| `python3 -m unittest scripts.tests.test_native_asr_benchmark scripts.tests.test_measure_process_tree_rss scripts.tests.test_validate_fixture_manifest scripts.tests.test_validate_model_asset_inventory -v` | Focused parser, scorer, process snapshots, fixture readiness and asset provenance | Unit | **67 tests passed.** | Does not run a licensed audio fixture. |
| `python3 -m unittest scripts.tests.test_measure_process_tree_rss scripts.tests.test_native_asr_benchmark scripts.tests.test_native_asr_integration -v` | Profiler, measured-result behavior and integration gate | Unit/integration | **28 tests, 27 passed, 1 skipped.** | The skipped real integration is not a pass. |
| `python3 scripts/validate_fixture_manifest.py` | Actual fixture manifest readiness | Data validation | **PASS as validator; 8 planned, 0 ready; benchmark NOT READY.** | No fixture content or references exist to score. |
| `python3 scripts/validate_model_asset_inventory.py --manifest evidence/mvp/prompt-79-model-assets.json --asset-root /private/tmp/prompt79-moonshine-small-streaming` | SHA-256 and byte counts for the eight existing local Small files | Artifact integrity | **PASS; 8 file-verified, 142,300,974 bytes; 0 redistribution approvals.** | No immutable upstream artifact revision, installer inclusion, license clearance or production bundle. |
| `python3 scripts/moonshine_model_source_audit.py --source-dir /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f` | Pinned source/header/architecture/catalog contract | Source audit | **Exit 0; source contract verified; header 30000; architecture 4.** The audit explicitly says model artifacts are not verified by this source-only command. | Local file bytes are checked by the separate asset validator; does not certify app ABI or notices. |
| `python3 scripts/validate_moonshine_third_party.py`; `python3 scripts/validate_third_party_notices.py`; `python3 scripts/validate_moonshine_build_scope.py`; `python3 scripts/validate_moonshine_license_review.py` | License inventory schemas and scope statements | Documentation/data validation | **PASS structurally.** License validator reports 8 dependencies, redistribution undetermined, binary not verified; notice validator reports 0 packaged, 4 planned, 4 blocked; build-scope validator says compilation not run for the source-only plan. | No human/legal approval, notice bundle, model-file rights, exact ONNX build provenance, or installer review. |
| Native C++ smoke compile and `measure_process_tree_rss.py ... -- /private/tmp/prompt79c_moonshine_stream_smoke ...` | Real C API load and two real streaming sessions | Native integration/performance | **PASS for smoke:** two sessions, partial and final nonempty; final sample max RSS 904,904,704 bytes. Ten independent repeated runs all exited successfully. | No human reference, WER, app integration, 4 GB certification, or model candidate pass. |
| Ten repeated smoke invocations via 100 ms sampler | Process-tree sampling, peak and repeated native runtime | Performance | **10/10 exited 0; max 1,023,426,560 bytes; 4/10 exceeded 950,000,000 bytes.** | Only standalone native smoke process; no Tauri/webview/helper processes. |
| Native batch helper on full 56.269896-second smoke WAV | Real native hypothesis, load, recognition, CPU/RTF/stop-to-final metrics | Native performance | One wrapper-captured run: recognition 5,475.256 ms, RTF 0.097, process CPU 30,156.661 ms, stop-to-final 0.001 ms, process-tree peak 974,405,632 bytes. A separate probe observed 5,161.664 ms / RTF 0.092 / CPU 28,697.753 ms. | One smoke utterance without a reference; not p50/p95 or WER. |
| `python3 scripts/run_native_asr_benchmark.py ...` with local Small/source/native artifacts | Evaluator's actual fail-closed entry path | Integration gate | **Expected BLOCKED exit 2:** `no verified ready fixtures are available for native evaluation`; no measured output created. | It does not reach model load/inference until the fixture corpus is ready. |
| `/private/tmp/prompt79c_moonshine_asr_batch --load-probe /private/tmp/outflow-no-such-model 2` through a stdout/stderr-capturing Python subprocess | Actual native model-load failure propagation | Native negative check | **Exit 1; stderr nonempty; output captured and summarized without exposing it.** | Confirms the failure path only; not a valid model load or benchmark result. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | Rust formatting | Format | **PASS.** | No Rust code changed. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | Rust workspace compilation/check | Build | **PASS.** | No native Moonshine/Tauri app integration. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | Existing Rust tests | Unit/doc | **352 unit/integration tests and 1 doctest passed.** `-- --list` confirmed 352 non-doctest cases. | No new Rust ASR binding or app path exists. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | Rust lint | Static | **PASS.** | No Rust source change. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | Rust workspace build | Build | **PASS.** | Does not build a Tauri application; none exists. |
| `npm run typecheck`; `npm run test -- --run`; `npm run lint`; `npm run build` | Existing frontend | Type/lint/unit/build | **PASS; 136 tests across 10 files.** | Does not create or test dictation UI; no UI source was changed. |
| `AXIsProcessTrusted()` read-only probe | Current macOS Accessibility state | Platform feasibility | **`AX_NOT_TRUSTED`.** No prompt was requested, no target was controlled. | No editable/protected control, target identity or insertion behavior was tested. |
| `git diff --check` | Whitespace/patch integrity | Repository | **PASS at final review.** | No behavioral proof. |

## E. Quantitative results

| Measurement | Observed value | Scope/qualification |
|---|---:|---|
| Small model files | 142,300,974 bytes, 8 SHA-256-verified files | Local developer copy only. |
| Immutable model artifact revision | Unavailable | Catalog path `small-streaming-en/quantized_26_08_21` is not an immutable revision. |
| Moonshine native dylib | 10,164,432 bytes; SHA-256 `42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5` | macOS arm64 dev build. |
| ONNX Runtime dylib | 27,623,992 bytes; SHA-256 `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4` | macOS arm64 linked runtime; exact external build provenance/notice match unresolved. |
| Native helper binary | 40,352 bytes; SHA-256 `e47798a900b2a2f285e42c287414b1c3631eedd58830fe39fd7ea42fdcb1021c` | Temporary development binary outside Git. |
| Existing streaming smoke helper | 39,792 bytes; SHA-256 `c4c00a2c64aef9a065f476a7222319269d61405403521ea01e60a055c588b987` | Temporary development binary outside Git. |
| Model + Moonshine dylib + ONNX Runtime | 180,089,398 bytes | Subset sum only; **not** installed application or installer size. |
| Repeated native smoke process-tree RSS | Highest 1,023,426,560 bytes; p95 nearest-rank 1,017,888,768; median 925,245,440; min 908,361,728; 4/10 >950,000,000 | 100 ms RSS sums on the 18 GB M3 Pro. This is the current broad native build, not the Outflow application. |
| Final short smoke run | 904,904,704 bytes peak RSS; 2 sessions | Single 100 ms native process sample. The lower single result does not overrule the ten-run maximum. |
| Full smoke WAV native batch | One observed 5,475.256 ms recognition; RTF .097; stop-to-final .001 ms; CPU 30,156.661 ms | One 56.269896 sec utterance, no verified reference. Separate run: 5,161.664 ms, RTF .092, CPU 28,697.753 ms. |
| Load probe | First process load 289.508 ms; warm reloads 79.094 and 79.125 ms (median 79.125 ms) | One repeated-load probe with uncontrolled OS cache. Separate inference process load measured 94.794 ms; final short smoke load 439.7 ms. |
| WER and ready fixtures | NOT MEASURED; 0 verified fixtures | No valid reference corpus. |
| Corpus p50/p95 latency and p50/p95 RTF | NOT MEASURED | One smoke WAV is not a corpus distribution. |
| Physical 4 GB gate | `PHYSICAL_4GB_GATE: BLOCKED` | Machine is Apple M3 Pro with 18 GB RAM; no VM substitute. |
| Full installed footprint / installer | NOT MEASURED | No complete Outflow application or installer exists. |
| macOS private/physical footprint | NOT COLLECTED | Profiler records RSS; shared pages can be double-counted. |
| CPU/thread profile | Partial only | Native `getrusage` captures process user+system CPU across threads; no sampled CPU utilization or per-thread profile. |

The current native build's repeated RSS is a hard resource failure for that measured build against 950,000,000 bytes. It does not determine the intrinsic memory footprint of an isolated STT-only build because the pinned revision's TTS and diarization inputs are compiled in and no supported isolated build was obtained.

## F. Architecture and integration readiness

### Available for later integration

- Pinned C API facts: source commit, header version, model architecture, and explicit model file hashes.
- A native C++ smoke pattern for model load, create/start/add-audio/poll/stop/final/free lifecycle and a temporary batch test helper.
- A bounded sample WAV parser and float PCM conversion used only by the test helper.
- A deterministic lexical WER scorer, corpus aggregation, a strict ready-fixture manifest validator, and a separate privacy-checked measured result format.
- A process-tree RSS sampler with real standalone-native measurements and deterministic aggregation tests.
- Existing Rust `outflow-core`/settings code and frontend preview remain as they were; they do not yet own audio capture, Moonshine, text cleanup, insertion, or Copy Result.

### Not available as app interfaces

No repository code yet connects Rust FFI → microphone worker → session reducer → cleanup → macOS insertion → Copy Result UI. The test helper cannot be lifted into production directly: it reads files, collects hypotheses, emits a private JSON stdout protocol, and does not implement microphone callback constraints, bounded audio queues, app cancellation, focus snapshots, target identity checks, accessibility capability handling, or user-visible copy. No Tauri shell, native Moonshine Rust binding, audio capture module, cleanup module, insertion adapter, clipboard flow, or result panel exists.

### Future integration map and dependencies

1. **Rust FFI** — establish a reviewed, typed wrapper around the pinned C API, including ABI/header/build hashes, lifetime rules, bounded error mapping, and a validated upstream or separately approved STT-only boundary. Do not call the C++ test helper from the app.
2. **Microphone worker** — add user-initiated capture through the approved audio backend after model/hardware/licensing gates; keep callbacks nonblocking/nonallocating and transfer bounded PCM to a worker thread.
3. **Session reducer** — add a pure Rust state machine to own press/release, cancellation, focus snapshot, idempotency and audio/model lifecycle.
4. **Cleanup** — add deterministic cleanup only after raw ASR behavior and vocabulary policy have golden tests; don't fold punctuation or vocabulary transformations into the WER scorer.
5. **macOS insertion** — complete T-006 with granted Accessibility permission, identity capture/recheck immediately before insertion, and five real editable/protected controls; implement safe fallback only after the contract is tested.
6. **Copy Result UI** — implement only under `docs/DESIGN.md` and `docs/USER_FLOWS.md` after the production shell and typed command/event boundary exist; explicit Copy must be user-triggered and in-memory.

T-010 remains an upstream gate for T-011/T-013. Do not start the production app slice while candidate quality/resource/licensing and physical 4 GB evidence are blocked.

## G. Incomplete work and exact blockers

| Task | Status | Work done | Missing evidence / next operation | External requirement / dependency |
|---|---|---|---|---|
| T-002 supported OS/CPU matrix | INCOMPLETE; physical 4 GB gate blocked | Current machine identity recorded: macOS 26.6.2, M3 Pro arm64, 18 GB. | Identify and test the project-approved physical 4 GB CPU-only reference host; record exact OS/build, CPU, RAM and platform. | Physical machine required; 18 GB host and VM limit do not substitute. |
| T-003 benchmark fixture manifest | INCOMPLETE; 0 ready fixtures | Existing manifest schema extended, validator and real evaluator implemented; public candidate leads recorded: Mozilla Common Voice spontaneous English listing (CC0-1.0) and OpenSLR LibriSpeech SLR12 (CC BY 4.0). | Select a privacy-appropriate exact release; acquire permitted files if approved; verify file-level provenance/terms, review every transcription against audio, convert to supported WAV, hash, and meet ≥100 short/≥50 long plus accents/noise/quiet/names/numbers/silence. Then rerun validators and real integration. | Common Voice download access may require email; CC BY attribution for LibriSpeech. Candidate listing is not a selected corpus or file-level approval. Require a privacy/license review and non-sensitive subset. |
| T-004 process-tree RSS | INCOMPLETE overall | Reusable sampler and deterministic unit tests; ten real native smoke measurements. | Profile complete app/helper tree after T-011 exists; verify macOS physical footprint metric if needed and run on physical 4 GB host. | Depends on T-002 and app process tree; no Tauri helper process exists yet. |
| T-005 installed-byte utility | COMPLETE as utility only; release-size gate incomplete | Existing `scripts/count_installed_bytes.py` and accepted `evidence/T-005.md` remain; partial model/library byte counts are reported above. | Run counter against complete unpacked release/install containing app, model, native libraries, support files and notices. | Complete release package and model/licensing selection do not exist; no AC-01 installed-size pass. |
| T-006 macOS insertion feasibility | INCOMPLETE/BLOCKED | Read-only `AXIsProcessTrusted()` returned `AX_NOT_TRUSTED`; no prompt or control was triggered. Reproduce the trust-state query with `CLANG_MODULE_CACHE_PATH=/private/tmp/prompt79c-clang-module-cache swift -e 'import ApplicationServices; print(AXIsProcessTrusted() ? "AX_TRUSTED" : "AX_NOT_TRUSTED")'`. | After user grants Accessibility, run development probe against TextEdit, one other ordinary editor, browser input, changed-focus target and password/protected field; record target identity/focus checks and safe failures. | User-granted macOS Accessibility, five real controls, suitable probe; T-002 platform record still incomplete. |
| T-009 model/runtime redistribution | INCOMPLETE/BLOCKED on approvals | Exact local build, direct linked libraries, upstream source, source notices, model asset hashes and CMake optional dependencies reviewed; TTS-off failure reproduced without modifying upstream. | Resolve model artifact-specific terms/revision; match ONNX binary provenance to exact 1.23.2 third-party notices; add full KISS FFT BSD-3-Clause text and required notices; determine VCTK/pyannote embedded data obligations; obtain human/legal review of actual release package. | Human/legal redistribution approval. No dependency/source update or production manifest is authorized here. |
| T-010 candidate decision | BLOCKED — INSUFFICIENT EVIDENCE | Real Small smoke, ten RSS runs, one full WAV batch timing, load probe, model bytes, candidate record. Small+current native build RSS subgate fails. | Once corpus/license/build scope and 4 GB host are solved, rerun full Small gates. If Small still fails and authorization is granted, acquire Tiny with file provenance/hashes/license and independently test WER, memory, latency and disk. Require signed decision. | Verified corpus, physical 4 GB, complete notices/license approval, Tiny acquisition authorization if needed, and full app resource measurement. |

### Corpus candidates identified, not approved

The Mozilla Common Voice Data Collective listing was checked 2026-11-03; it lists English spontaneous-speech ASR data under CC0-1.0 and Common Voice release metadata can include contributor-reported accent fields. OpenSLR lists the LibriSpeech English read-speech corpus under CC BY 4.0 with clean and more challenging subsets. These provide possible material for accent/spontaneous and longer read-speech coverage, but neither source alone fulfills the required coverage. No corpus was downloaded, selected or file-hashed, and neither candidate is counted as ready. The manifest remains 8 planned / 0 ready.

## H. Reproduction guide

### Checkout and verify identity

```sh
git fetch origin
git switch --track origin/codex/prompt79c-feasibility
git rev-parse HEAD
git status --short --branch

MOONSHINE_SOURCE_DIR=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
BUILD_DIR=/private/tmp/prompt79-moonshine-wrapper-build-onnx

python3 scripts/moonshine_model_source_audit.py --source-dir "$MOONSHINE_SOURCE_DIR"
python3 scripts/validate_model_asset_inventory.py \
  --manifest evidence/mvp/prompt-79-model-assets.json --asset-root "$MODEL_DIR"
python3 scripts/validate_fixture_manifest.py
```

Expected model output is source contract verified, C API `30000`, architecture `4`; the source-only audit may say model artifacts are not checked by that command. The separate asset inventory must say 8 file-verified and 142,300,974 bytes. Fixture validation must say 0 ready and `NOT READY` until lawful data is added.

### Rebuild and execute the existing native smoke

The exact wrapper build is documented in [`prompt-79c-native-license-audit.md`](prompt-79c-native-license-audit.md) and the original native procedure in [`prompt-79-macos-dictation.md`](prompt-79-macos-dictation.md). Rebuild the pinned Moonshine target from the existing external wrapper, then compile and run:

```sh
MOONSHINE_SOURCE_DIR=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
MODEL_DIR=/private/tmp/prompt79-moonshine-small-streaming
BUILD_DIR=/private/tmp/prompt79-moonshine-wrapper-build-onnx
WRAPPER_DIR=/private/tmp/prompt79-moonshine-wrapper
mkdir -p "$WRAPPER_DIR"
cat > "$WRAPPER_DIR/CMakeLists.txt" <<EOF
cmake_minimum_required(VERSION 3.22.1)
project(prompt79_moonshine_wrapper LANGUAGES C CXX)
set(MOONSHINE_TTS_BUILD_ONNX ON CACHE BOOL "Build upstream C API dependencies" FORCE)
add_subdirectory(${MOONSHINE_SOURCE_DIR}/core moonshine-core)
EOF
cmake -S "$WRAPPER_DIR" -B "$BUILD_DIR" \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_ARCHITECTURES=arm64 \
  -DMOONSHINE_TTS_BUILD_ONNX=ON
cmake --build "$BUILD_DIR" --clean-first --target moonshine \
  --config Release --parallel 4

clang++ -std=c++17 -O2 -I"$MOONSHINE_SOURCE_DIR/core" \
  tests/native/moonshine_streaming_smoke.cpp \
  -L"$BUILD_DIR/moonshine-core" -lmoonshine \
  -Wl,-rpath,"$BUILD_DIR/moonshine-core" \
  -Wl,-rpath,"$MOONSHINE_SOURCE_DIR/core/third-party/onnxruntime/lib/macos/arm64" \
  -o /private/tmp/prompt79c_moonshine_stream_smoke
python3 scripts/measure_process_tree_rss.py --interval-ms 100 \
  --output /private/tmp/prompt79c-rss-new-run.json -- \
  /private/tmp/prompt79c_moonshine_stream_smoke "$MODEL_DIR" \
  "$MOONSHINE_SOURCE_DIR/test-assets/two_cities_librivox_48k.wav"
```

The profiler requires permission to read the local macOS process table. It writes sanitized JSON, preserves a 100 ms sample limitation, and never stores the command arguments. Do not commit model/audio/runtime/build artifacts or raw stdout containing a future hypothesis.

### Build and run the batch evaluator

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
python3 scripts/validate_measured_asr_results.py \
  --file /private/tmp/prompt79c-measured-asr.json
```

With the committed manifest, the runner exits `2` with `no verified ready fixtures are available for native evaluation` and writes no result. Do not replace its failure with a synthetic result. After lawful ready fixtures are frozen, the opt-in Python integration test accepts the `OUTFLOW_BENCHMARK_NATIVE_RUNNER`, `OUTFLOW_BENCHMARK_MODEL_DIR`, `OUTFLOW_BENCHMARK_SOURCE_DIR`, `OUTFLOW_BENCHMARK_MOONSHINE_LIBRARY`, `OUTFLOW_BENCHMARK_ONNXRUNTIME_LIBRARY`, `OUTFLOW_BENCHMARK_ASSET_ROOT`, `OUTFLOW_BENCHMARK_FIXTURE_MANIFEST`, and `OUTFLOW_BENCHMARK_FIXTURE_ROOT` environment values.

### Automated regression commands

```sh
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

The expected baseline for this handoff is 343 Python tests with one corpus-dependent skip, 352 Rust unit/integration tests plus one doctest, and 136 frontend tests. Source and artifact audits report current actual status and are not release approvals.

## I. GitHub delivery and review context

- Repository: `https://github.com/thrns/outflow`
- Development branch: `codex/prompt79c-feasibility`, based directly on verified Prompt 79 commit `2303916c2066ade0fbffb493b1d09ed1b5f86a94`.
- Implementation commit [`b2a1e5b51c66ab9aabae1bb46a9f5d129486e79e`](https://github.com/thrns/outflow/commit/b2a1e5b51c66ab9aabae1bb46a9f5d129486e79e) was pushed normally and verified with `git ls-remote` at that SHA before this documentation-only handoff update.
- No history was rewritten, no branch was force-pushed, and no merge/release operation was performed.
- The final branch-head SHA and its commit URL are reported in the final delivery message after this evidence-only update is pushed. Local build outputs, native executables, model files, audio, process snapshots, wrapper directories, and test artifacts remain outside Git. `dist/` and Python cache directories are generated/ignored and are not staged.
- No dependency, Cargo/npm lockfile, or GitHub Actions changes were made.
- Required external reviews remain: corpus/license and privacy selection; artifact-specific model redistribution; exact native notice bundle; and legal/maintainer decision on TTS/diarization scope.

### Changed repository files

```text
README.md
docs/BENCHMARK_FIXTURES.md
docs/BENCHMARK_RESULTS.md
docs/MOONSHINE_BUILD_SCOPE.md
docs/MOONSHINE_LICENSE_REVIEW.md
docs/MOONSHINE_THIRD_PARTY_AUDIT.md
docs/TASKS.md
docs/THIRD_PARTY_NOTICES.md
docs/benchmark-fixtures.json
docs/moonshine-third-party.json
docs/third-party-notices.json
evidence/model/MODEL_DECISION.md
evidence/model/moonshine-small-streaming-requirements-task-78.md
evidence/mvp/prompt-79-model-assets.json
evidence/mvp/prompt-79c-detailed-implementation-handoff.md
evidence/mvp/prompt-79c-native-license-audit.md
scripts/measure_process_tree_rss.py
scripts/run_native_asr_benchmark.py
scripts/score_corpus_wer.py
scripts/score_wer.py
scripts/tests/test_measure_process_tree_rss.py
scripts/tests/test_native_asr_benchmark.py
scripts/tests/test_native_asr_integration.py
scripts/tests/test_validate_fixture_manifest.py
scripts/tests/test_validate_model_asset_inventory.py
scripts/validate_fixture_manifest.py
scripts/validate_measured_asr_results.py
scripts/validate_model_asset_inventory.py
tests/native/moonshine_asr_batch.cpp
```

## J. Recommended next engineering action

Do not start T-011 or Prompt 80. The smallest next proof that can advance the decision is a **frozen, lawful, file-level benchmark fixture set with human-verified references**. The current schema and runner cannot score even one fixture until this exists. Select a specific non-sensitive corpus release from the leads in `docs/BENCHMARK_FIXTURES.md`, establish access and license evidence for exact audio/reference files, measure/convert/hash each file, and human-review the reference text. Then run the new native evaluator and inspect its sanitized WER/latency results. In parallel, secure a physical 4 GB CPU-only host and complete model/runtime redistribution review. If the Small native runtime still exceeds the hard resource cap and a physical 4 GB gate, Tiny evaluation needs separately authorized acquisition and the same independent quality/latency/resource/license checks. If no candidate meets every hard gate, record a no-go; do not relax thresholds.
