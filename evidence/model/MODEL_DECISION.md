# Prompt 79D Model Feasibility Decision

**Decision:** `BLOCKED — INSUFFICIENT EVIDENCE`

**Scope:** T-010, F-06, AC-06. This is a development evidence record for the pinned Moonshine Small Streaming build. It is not approval to redistribute model assets or a production model manifest.

## Decision summary

The Small Streaming files load and perform real CPU-only native inference on the 18 GB Apple M3 Pro. Ten paired two-session runs with default thread settings reached a sampled process-tree peak of 1,010,614,272 bytes; two runs exceeded the 950,000,000-byte limit. The upstream single-thread reliability switch reduced the paired maximum to 907,902,976 bytes, but raised p95 session latency from 1,214 ms to 3,270 ms per 12-second smoke slice. The switch is documented upstream for reliability/ThreadSanitizer use, not as a production performance setting, and its remaining headroom does not prove that the full application fits.

There is no ready human-reviewed WER fixture, approved physical 4 GB reference class, Tiny artifact or run, artifact-specific model redistribution approval, or signed decision. Therefore the model selection gate remains blocked. Do not proceed to the production Tauri application or claim T-011/T-013 prerequisites are satisfied.

## Candidate gate record

| Gate | Small Streaming + Prompt 79 native build | Status | Evidence and limits |
|---|---:|---|---|
| Source/API identity | Moonshine `234f60faa0eb388b01cdf7e60aca232af37aefda`, header 30000, architecture 4 | PASS for local dev build | Source checkout and native smoke verified. Not a production source lock. |
| Local model file integrity | 8 files, 142,300,974 bytes | PASS for local files only | File hashes in [`prompt-79-model-assets.json`](../mvp/prompt-79-model-assets.json). The dated catalog path is not an immutable model artifact revision. |
| Native speech inference | Real partial and final output on two-session smoke and one full 56.269896-second WAV | PASS for smoke only | No transcript is stored. The smoke WAV has no human-verified benchmark reference. |
| Native process-tree RSS | Prompt 79D: 10 default-thread CPU runs, max 1,010,614,272 bytes (2/10 ≥950,000,000); 10 upstream single-thread runs, max 907,902,976 bytes (0/10 ≥950,000,000) | **Default configuration FAIL; single-thread mode is experimental only** | Identical arm64 native helper, model, audio and two sequential sessions; 100 ms sampler. The single-thread switch is reliability-only upstream and does not establish full-app headroom or release eligibility. |
| Installed-size gate | Model 142,300,974 bytes; `libmoonshine.dylib` 10,164,432 bytes; ONNX Runtime 27,623,992 bytes | NOT ASSESSED | These development files are not an installer or complete package. Combined figures omit all other runtime and installer files. |
| Cold/warm model load | 10 fresh-process probes per mode, 10 loads each; default first-load p50/p95 122.025/326.166 ms, within-process reload p50/p95 82.827/92.754 ms; single-thread 145.115/160.387 ms and 100.631/106.899 ms | OBSERVED, not a release gate pass | OS file-cache state was not controlled. The process-load probe peaked at 508,903,424 bytes default and 496,533,504 bytes single-thread; this excludes streaming inference buffers. |
| Recognition timing | Default session p50/p95 1,002.713/1,214.004 ms, RTF p50/p95 0.0835/0.1011; single-thread 3,170.911/3,269.501 ms, RTF 0.264/0.2721 | OBSERVED, not a release gate pass | Twenty 12-second smoke sessions per configuration over repeated copies of the same audio slice. This is not a distribution across the required corpus or 4 GB machine. Stop-to-final p95 was 0.065 ms default and 0.032 ms single-thread. |
| WER / reference quality | 0 ready fixtures, no scored references | NOT RUN | AC-06 requires clean-speech WER ≤12% on the frozen diverse reference set. |
| Physical 4 GB CPU-only | macOS reports exactly 19,327,352,832 installed bytes on the Apple M3 Pro | Eligibility `INELIGIBLE`; gate `BLOCKED` | Prompt 79D uses a strict 4,000,000,000-byte cap and an explicit approved-class allowlist, currently empty. The native result validator keeps an eligible physical class `NOT_ASSESSED` until a benchmark runs. |
| Redistribution / notices | Model review pending; runtime notices incomplete | BLOCKED | See [`prompt-79c-native-license-audit.md`](../mvp/prompt-79c-native-license-audit.md). No legal or owner approval. |
| Tiny Streaming fallback | Source architecture `MOONSHINE_MODEL_ARCH_TINY_STREAMING=2`; English catalog lists 34M parameters, dated `quantized_26_08_21` path, and source-declared files totaling 77,748,675 bytes including optional word-timestamp decoder (45,233,659 bytes without it) | NOT EVALUATED | These are pinned-source metadata, not downloaded or hashed Tiny artifacts. The catalog CRC32C values are not SHA-256. Upstream reports 12.00% WER for Tiny Streaming, exactly at Outflow's limit with no demonstrated headroom. Tiny acquisition requires separate authorization; no quality/resource conclusion follows from parameter count or catalog size. |

## Resource interpretation

The ten-run sampled RSS results for the two-session smoke were:

| Run | Peak process-tree RSS (bytes) |
|---:|---:|
| 1 | 908,558,336 |
| 2 | 1,017,888,768 |
| 3 | 1,023,426,560 |
| 4 | 918,896,640 |
| 5 | 922,861,568 |
| 6 | 1,011,761,152 |
| 7 | 920,649,728 |
| 8 | 980,975,616 |
| 9 | 908,361,728 |
| 10 | 927,629,312 |

These are 100 ms RSS sums, not macOS physical footprint/PSS. The single batch run observed a 974,405,632-byte tree peak over 57 snapshots with a maximum 105 ms gap. RSS may double-count shared pages; a 100 ms poll may miss short-lived processes. The sampler captures no arguments, environment values, audio, or transcripts.

The current native library contains the optional TTS/G2P and diarization source paths. A TTS-off attempt did not produce an STT-only C API library: the unmodified C API requires TTS headers and link symbols, and diarization sources are appended unconditionally. No before/after binary or runtime comparison isolates those components' RSS contribution. The large observed memory use is therefore attributed to the complete measured native execution, not to the model weights alone.

## Prompt 79D paired memory and performance experiment

The raw sanitized per-run records, exact native/model/audio identities, sampling details, cold/warm load-probe results, and all session timings are in [`prompt-79d-native-memory-measurements.json`](../mvp/prompt-79d-native-memory-measurements.json). All 20 paired smoke measurements and all 20 model-load processes completed successfully. The audio was the pinned upstream `two_cities_librivox_48k.wav`, truncated to the same 12-second speech slice for each of the two sequential streaming sessions. The native helper explicitly selected `CPUExecutionProvider`.

| Configuration | 10 tree peaks (bytes) | Median / p95 / maximum RSS | Exceedances | Session p50 / p95 | RTF p50 / p95 | Native CPU p50 / p95 |
|---|---|---:|---:|---:|---:|---:|
| Default ONNX Runtime thread settings | 829,472,768; 949,501,952; 931,577,856; 924,762,112; 930,217,984; 1,010,614,272; 897,400,832; 963,952,640; 938,295,296; 928,088,064 | 930,897,920 / 989,616,538 / 1,010,614,272 | 2/10 | 1,002.713 / 1,214.004 ms | 0.0835 / 0.1011 | 5,428.504 / 6,160.906 ms |
| `MOONSHINE_ORT_SINGLE_THREAD=1` | 791,756,800; 801,144,832; 795,459,584; 907,902,976; 800,456,704; 795,459,584; 829,095,936; 863,059,968; 815,661,056; 889,585,664 | 808,402,944 / 899,660,186 / 907,902,976 | 0/10 | 3,170.911 / 3,269.501 ms | 0.2640 / 0.2721 | 3,169.251 / 3,260.373 ms |

The single-thread test mode lowered median process-tree RSS by 13.16% and the maximum by 10.16%, while increasing p95 recognition time about 2.69×. The pinned `core/ort-utils/ort-utils.cpp` applies intra-op/inter-op thread counts of one and sequential execution when this variable is set; `ort-utils.h` explicitly describes it as a ThreadSanitizer/reliability escape hatch and says production builds do not set it. The experiment is therefore useful evidence that the current ONNX session/thread configuration affects memory, but it is not an approved production optimization.

The default run's single-process `getrusage` high-water mark reached 1,020,248,064 bytes, higher than the 100 ms sampled tree maximum; the one-thread maximum was 907,902,976 bytes. This demonstrates that polling can miss short peaks. Load-only probes peaked at 508,903,424 bytes (default) and 496,533,504 bytes (single-thread), below inference peaks. This points to inference/session work as a major source of transient memory, but the experiment did not isolate individual allocations. It did not collect `vmmap` region accounting, phase-exact current RSS, or per-thread counts, so it cannot separate ORT pools, decoder buffers, mapped model pages, allocator caches, and fragmentation or determine whether any retained allocation is a leak. Two streams per run are insufficient for a leak claim. No before/after test successfully isolates TTS or diarization RSS.

The runtime constructs five streaming ONNX sessions (frontend, encoder, adapter, cross-KV, and decoder-KV) and already uses direct ORT-model bytes, disables prepacking and the CPU memory arena, and shares environment allocators. No additional memory option was adopted. A valid production path needs either a supported, measured thread/session control with sufficient p95 throughput and complete-app memory headroom, or an approved upstream STT-only C API change. Current default Small exceeds the hard cap, and the reliability mode leaves only about 42 MB below it at its maximum before adding the Tauri app.

The runtime identity verification for the tested helper is summarized in [`prompt-79d-native-runtime-identity.json`](../mvp/prompt-79d-native-runtime-identity.json). The Tiny source fallback notes above are preparation only; Tiny files were not acquired or tested.

## Required evidence to unblock T-010

1. Obtain a lawful, file-level corpus meeting at least 100 short and 50 long English phrases, required noise/accent/name/number/silence coverage, and human-verified references. Freeze hashes and normalization policy. The current schema, validator, scorer, native helper, and result validator are ready for that input.
2. Run the measured native evaluator on the frozen fixtures and report aggregate/per-fixture WER, p50/p95 recognition and stop-to-final timings, RTF, and repeated-session stability. AC-06 requires WER ≤12%, CPU-only p95 RTF <1.0, p95 stop-to-final <1,500 ms, and end-to-end p95 ≤2,000 ms where insertion is available.
3. Run the candidate on a real physical 4 GB CPU-only reference system with complete process-tree measurements.
4. Resolve whether an upstream-approved STT-only C API build can exclude optional TTS and diarization code, or measure and accept the current broad runtime if every hard cap and licensing requirement can pass. Do not edit the pinned source without approval.
5. Complete artifact-specific model/runtime redistribution review. If Small remains over memory, obtain explicit authorization for Tiny assets, then repeat hash, licensing, quality, latency, disk, and physical memory gates for Tiny.
6. Record the human-signed candidate decision. If neither model meets every required quality, latency, resource, and license gate, stop the release.

## Reproduction references

The exact commands and detailed implementation/test walkthrough are in [`prompt-79d-detailed-implementation-handoff.md`](../mvp/prompt-79d-detailed-implementation-handoff.md) and [`README.md`](../../README.md#prompt-79d-reproduction-feasibility-corrections-and-paired-memory-runs). Raw process-tree snapshots and local model files remain outside Git under `/private/tmp`.
