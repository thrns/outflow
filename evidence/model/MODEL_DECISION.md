# Prompt 79C Model Feasibility Decision

**Decision:** `BLOCKED — INSUFFICIENT EVIDENCE`

**Scope:** T-010, F-06, AC-06. This is a development evidence record for the pinned Moonshine Small Streaming build. It is not approval to redistribute model assets or a production model manifest.

## Decision summary

The Small Streaming files load and perform real native inference on the verified macOS development machine. The current Prompt 79 native library and harness exceed the project's 950,000,000-byte sampled RSS release threshold. That is a resource failure for the measured native build, before a Tauri shell or webview is included. It does not establish the intrinsic memory requirement of an STT-only build because the pinned build also contains TTS and diarization code and no approved isolated build was available.

There is no verified WER corpus, physical 4 GB reference machine, Tiny candidate artifact, artifact-specific model redistribution approval, or signed decision. Therefore the model selection gate remains blocked. Do not proceed to the production Tauri application or claim T-011/T-013 prerequisites are satisfied.

## Candidate gate record

| Gate | Small Streaming + Prompt 79 native build | Status | Evidence and limits |
|---|---:|---|---|
| Source/API identity | Moonshine `234f60faa0eb388b01cdf7e60aca232af37aefda`, header 30000, architecture 4 | PASS for local dev build | Source checkout and native smoke verified. Not a production source lock. |
| Local model file integrity | 8 files, 142,300,974 bytes | PASS for local files only | File hashes in [`prompt-79-model-assets.json`](../mvp/prompt-79-model-assets.json). The dated catalog path is not an immutable model artifact revision. |
| Native speech inference | Real partial and final output on two-session smoke and one full 56.269896-second WAV | PASS for smoke only | No transcript is stored. The smoke WAV has no human-verified benchmark reference. |
| Native process-tree RSS | Ten 100 ms smoke runs: highest 1,023,426,560 bytes; four of ten >950,000,000 bytes | **FAIL for current measured native build** | Native smoke process only. A separate batch run including a small Python capture wrapper peaked at 974,405,632 bytes (native child 955,564,032 bytes). This is not full Outflow application RSS. |
| Installed-size gate | Model 142,300,974 bytes; `libmoonshine.dylib` 10,164,432 bytes; ONNX Runtime 27,623,992 bytes | NOT ASSESSED | These development files are not an installer or complete package. Combined figures omit all other runtime and installer files. |
| Cold/warm model load | First load probe 289.508 ms; two warm reloads 79.094 and 79.125 ms | OBSERVED, not a release gate pass | One local 18 GB M3 Pro; OS cache was not controlled. One inference process load was 94.794 ms. |
| Recognition timing | 5,475.256 ms for 56.269896 sec audio; RTF 0.097; stop-to-final 0.001 ms | OBSERVED, not a release gate pass | One smoke utterance, no distribution of utterances, and no 4 GB hardware. Process CPU time was 30,156.661 ms across native threads. |
| WER / reference quality | 0 ready fixtures, no scored references | NOT RUN | AC-06 requires clean-speech WER ≤12% on the frozen diverse reference set. |
| Physical 4 GB CPU-only | Development Mac is Apple M3 Pro, 18 GB RAM, macOS 26.6.2 | `PHYSICAL_4GB_GATE: BLOCKED` | The host does not represent the required physical 4 GB system. No VM limit is substituted. |
| Redistribution / notices | Model review pending; runtime notices incomplete | BLOCKED | See [`prompt-79c-native-license-audit.md`](../mvp/prompt-79c-native-license-audit.md). No legal or owner approval. |
| Tiny Streaming fallback | No local verified Tiny files found in the authorized development asset location; no Tiny run | NOT EVALUATED | User authorization covered local Small assets only. Any Tiny acquisition requires separate provenance, hash, license, and acquisition approval. |

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

## Required evidence to unblock T-010

1. Obtain a lawful, file-level corpus meeting at least 100 short and 50 long English phrases, required noise/accent/name/number/silence coverage, and human-verified references. Freeze hashes and normalization policy. The current schema, validator, scorer, native helper, and result validator are ready for that input.
2. Run the measured native evaluator on the frozen fixtures and report aggregate/per-fixture WER, p50/p95 recognition and stop-to-final timings, RTF, and repeated-session stability. AC-06 requires WER ≤12%, CPU-only p95 RTF <1.0, p95 stop-to-final <1,500 ms, and end-to-end p95 ≤2,000 ms where insertion is available.
3. Run the candidate on a real physical 4 GB CPU-only reference system with complete process-tree measurements.
4. Resolve whether an upstream-approved STT-only C API build can exclude optional TTS and diarization code, or measure and accept the current broad runtime if every hard cap and licensing requirement can pass. Do not edit the pinned source without approval.
5. Complete artifact-specific model/runtime redistribution review. If Small remains over memory, obtain explicit authorization for Tiny assets, then repeat hash, licensing, quality, latency, disk, and physical memory gates for Tiny.
6. Record the human-signed candidate decision. If neither model meets every required quality, latency, resource, and license gate, stop the release.

## Reproduction references

The exact commands and detailed implementation/test walkthrough are in [`prompt-79c-detailed-implementation-handoff.md`](../mvp/prompt-79c-detailed-implementation-handoff.md) and [`README.md`](../../README.md#prompt-79c-reproduction-native-evaluation-and-process-tree-rss). Raw process-tree snapshots and local model files remain outside Git under `/private/tmp`.
