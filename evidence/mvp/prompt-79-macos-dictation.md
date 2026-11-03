# Prompt 79 — macOS offline dictation evidence

```yaml
prompt_id: 79
status: PARTIALLY VERIFIED / BLOCKED
feature_ids: [F-01, F-03, F-04, F-05, F-06, F-07, F-09, F-10, F-12, F-13]
flows: [UF-03, UF-04, UF-05, UF-07, UF-08, UF-10, UF-11]
acceptance: [AC-01, AC-03, AC-04, AC-05, AC-06, AC-07, AC-09, AC-10, AC-12, AC-13]
owner: Codex coordinator
branch: codex/prompt79-macos-offline-dictation
base_commit: efb3ba8a7b48f608a0eb5aaf29a3f656f57f893f
result_commit: a7c037030ec99731f183e25b45f88f6a7113b012
platform: macOS 26.6.2 (25G83), Apple M3 Pro arm64, 18 GB physical RAM
implementation: Added native C API inference smoke harness, local artifact-hash manifest, task evidence, and development instructions. No Outflow runtime/app integration.
metrics: Native smoke process only; app footprint, app-process RSS, 4 GB, WER and offline app operation NOT TESTED.
review: PENDING
```

## Outcome

**Prompt 79 is not functionally complete.** Real Moonshine Small Streaming inference was built and exercised through the pinned C API with local model artifacts and a speech fixture. Two fixture-fed streaming sessions produced nonempty partial and final transcripts. The test is an external native smoke harness, not the Outflow application.

The repository has no Tauri application shell, microphone capture service, Rust Moonshine binding, recording/session integration, cleanup pipeline connected to recognition, macOS target validation/insertion adapter, Copy Result panel, or launchable dictation app. `npm run dev` launches the browser Settings preview only; the repository does not define `npm run tauri dev`. No mic, Accessibility, or Automation permission flow was exercised. Start/Stop/Cancel, real microphone-to-ASR, deterministic cleanup, TextEdit insertion, focus mismatch, protected-field rejection, and explicit clipboard fallback remain unimplemented and untested.

### Implementation files

- `tests/native/moonshine_streaming_smoke.cpp` — new standalone C++ C API smoke test. It reads caller-supplied PCM16 mono WAV, loads the explicit Small Streaming architecture, streams 300 ms chunks, polls partials, stops/drains, checks nonempty partial/final lines, reuses the transcriber for a second session, and prints status/timing/RSS only (never transcript content).
- `evidence/mvp/prompt-79-model-assets.json` — new local artifact hash manifest. It records source URLs, observed SHA-256, and unresolved licensing/redistribution status. The repository's validator is a development integrity check, not an app runtime verifier or signed production manifest.
- `README.md` — development build, authorized local acquisition, smoke test, limitations, and troubleshooting instructions.
- `docs/TASKS.md` — Prompt 79 partial evidence and updated milestone progress; no task checkbox was checked.
- `evidence/mvp/prompt-79-macos-dictation.md` — this report.

No model weights, audio, native binaries, signing material, or user transcripts are included in Git. Existing Rust crates and frontend were not changed. No reusable app shell/audio/session/insertion implementation was present to connect.

## Repository and upstream provenance

- Repository base commit: `efb3ba8a7b48f608a0eb5aaf29a3f656f57f893f` on `main`; work was carried on `codex/prompt79-macos-offline-dictation`.
- Moonshine source: `https://github.com/moonshine-ai/moonshine`, checked out at `234f60faa0eb388b01cdf7e60aca232af37aefda`.
- The external Moonshine checkout was verified detached at that commit; `git status --short --branch` and `git lfs status` showed no source modifications or pending LFS objects after the local-only fetches.
- Source audit confirms `MOONSHINE_HEADER_VERSION = 30000`, `MOONSHINE_MODEL_ARCH_SMALL_STREAMING = 4`, explicit architecture loading, and the catalog path `/quantized_26_08_21`.
- That dated catalog path does not provide an immutable artifact revision. Each acquired file is identified by its observed SHA-256 below; model-specific artifact metadata/revision is still unavailable.
- The source audit command passes its source-contract checks but reports `MODEL_ARTIFACTS_VERIFIED: NO` because it examines the external source checkout, not the separately acquired local model directory. The separate local validator verifies the observed file hashes.

## Local model artifacts

The user explicitly authorized downloading the eight files from the official catalog for local development inference only. They are stored under `/private/tmp/prompt79-moonshine-small-streaming`, outside Git. The source-derived expected byte counts matched the observed local file sizes. The pinned catalog's CRC32C values are upstream metadata; SHA-256 values were computed locally and are separately recorded in `prompt-79-model-assets.json`.

| File | Observed bytes | Observed SHA-256 |
|---|---:|---|
| `adapter.ort` | 2,870,368 | `c665f742364febad597cc9ac1e0b341ffbee0e24a1466e2f3bde95e6e4771762` |
| `cross_kv.ort` | 5,356,536 | `e2d3417144e9514055ebfefe8dcc4c0a55a55adcb8530435844c75c53e352bf6` |
| `decoder_kv.ort` | 81,878,600 | `1a05465b1dd955858dfcbee039c0020fb5dd982b0f5094c34e61735d518d771b` |
| `encoder.ort` | 44,148,576 | `2d4d973e91e8aca08c51e7e7efa28a46ab265b63d809d5294d18b86bcd85b993` |
| `frontend.model.ort` | 26,944 | `09b1210ae30dc5f0f3e45f0ebab914c254741323114f53fbbe5ae62cca35058f` |
| `frontend.weights.ort` | 7,769,464 | `7ef97521bd4bad3928f5bb6808586f4fcc6e92bd5990394112eed7d4052ec338` |
| `streaming_config.json` | 512 | `26f02b6afb22d60871a5efd85c3d38e569cc0ddb6c5eb6e93d3260152ae8a47a` |
| `tokenizer.bin` | 249,974 | `6884b35fd6377d4c4d32336a0bc152f36b64d1e45b6503683cdc238250a8472d` |
| **Total** | **142,300,974** | — |

Verification command:

```sh
python3 scripts/validate_model_asset_inventory.py \
  --manifest evidence/mvp/prompt-79-model-assets.json \
  --asset-root /private/tmp/prompt79-moonshine-small-streaming
```

Result: valid manifest, eight file hashes checked, zero redistribution approvals, bundled model `NOT VERIFIED`, T-009 `INCOMPLETE`. The manifest validator checks file presence and SHA-256; expected byte sizes were independently compared against the source catalog and observed `stat` output. This is local artifact integrity evidence, not proof of installer inclusion, an app startup verifier, an immutable vendor revision, or legal approval.

### License and redistribution state

- Pinned Moonshine source docs identify the English Small Streaming model family as MIT. Exact acquired artifact/tokenizer/support-file terms and an immutable artifact revision are not provided by the dated catalog entry; artifact-specific review remains pending.
- No model weights were committed, packaged, or redistributed. The local-only acquisition authorization does not permit those actions.
- ONNX Runtime 1.23.2's local arm64 dylib was present in the external checkout (27,623,992 bytes; SHA-256 `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`). The pinned checkout's ONNX Runtime `ThirdPartyNotices.txt`/`NOTICE` review is incomplete. KISS FFT's `COPYING` references a missing `LICENSES/BSD-3-Clause` text; notice reconciliation is incomplete.
- The CMake dependency graph required `MOONSHINE_TTS_BUILD_ONNX=ON` to compile/link the pinned C API. The build consequently includes broader TTS and diarization code. Two Git LFS inputs were fetched into the external checkout under separate local-only user authorizations, never into Outflow Git: `core/cpp-annote/src/community1_cpp_annote_embedded.cpp` (2,535,244 bytes; SHA-256 `9424da4176b33e67e4000ea2a776d64b6a78ee9bbf72d40405fb6805d12758c4`; source identifies pyannote community-1 data, CC BY 4.0) and `core/moonshine-tts/src/zipvoice-voices-data.cpp` (11,695,724 bytes; SHA-256 `f3e4d62cae93c465e1de8521bc5706b9a20f834cbbf37404bf03f0d35dffa012`; VCTK reference clips, artifact-specific license evidence absent in the checkout).
- Native redistribution, exact notices for the actual linked build, and legal review remain unresolved. T-009 is not complete.

## Native build and ABI evidence

Environment: macOS 26.6.2 (25G83), arm64 Apple M3 Pro, 18 GB physical RAM; Apple Clang 21.0.0.21000101, CMake 4.4.5. The upstream source was not modified. The wrapper and build directory were outside the repository at `/private/tmp/prompt79-moonshine-wrapper` and `/private/tmp/prompt79-moonshine-wrapper-build-onnx`.

An initial build with `MOONSHINE_TTS_BUILD_ONNX=OFF` failed because the C API still includes `moonshine-asset-catalog.h` and references TTS symbols. The first corrected wrapper attempt then stopped on the two Git LFS pointer files above. With the authorized LFS objects materialized and `MOONSHINE_TTS_BUILD_ONNX=ON`, the pinned source built successfully:

```sh
cmake -S /private/tmp/prompt79-moonshine-wrapper \
  -B /private/tmp/prompt79-moonshine-wrapper-build-onnx \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_ARCHITECTURES=arm64 \
  -DMOONSHINE_TTS_BUILD_ONNX=ON
cmake --build /private/tmp/prompt79-moonshine-wrapper-build-onnx \
  --clean-first --target moonshine --config Release --parallel 4
```

Final build exit code: 0. The clean build log is `/private/tmp/prompt79-native-build.log` (outside Git). Output: `/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib`, arm64 Mach-O, 10,164,432 bytes, SHA-256 `42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5`. `otool -L` shows `@rpath/libonnxruntime.1.23.2.dylib` plus system libraries. `nm -gU` confirmed these exported C ABI symbols:

```text
moonshine_load_transcriber_from_files
moonshine_free_transcriber
moonshine_create_stream
moonshine_start_stream
moonshine_transcribe_add_audio_to_stream
moonshine_transcribe_stream
moonshine_stop_stream
moonshine_free_stream
```

This proves one local arm64 upstream C API build and symbol presence. It does not integrate the ABI into Rust, validate other target platforms, complete the production dependency/notice audit, or satisfy T-022.

## Real native inference test

Fixture: pinned checkout `test-assets/two_cities_librivox_48k.wav`; SHA-256 `0c7c127dc1ccabb1384de70b2d4591fd37c4e843b6b395efe876846ccd30b1de`; 5,402,230 bytes; PCM16 mono 48 kHz, 56.269896 seconds. The harness processes only the first 12 seconds. The filename identifies LibriVox audio; LibriVox's [public-domain policy](https://librivox.org/pages/public-domain/) says its recordings are public domain in the US. The pinned checkout lacks the original catalog item and narrator record; territorial rights outside the US are not independently established here. This is a local smoke fixture, not a benchmark corpus.

Exact compile and run command:

```sh
clang++ -std=c++17 -O2 \
  -I/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core \
  tests/native/moonshine_streaming_smoke.cpp \
  -L/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core \
  -lmoonshine \
  -Wl,-rpath,/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core \
  -Wl,-rpath,/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core/third-party/onnxruntime/lib/macos/arm64 \
  -o /private/tmp/prompt79_moonshine_stream_smoke
DYLD_LIBRARY_PATH=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core/third-party/onnxruntime/lib/macos/arm64:/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core \
  /private/tmp/prompt79_moonshine_stream_smoke \
  /private/tmp/prompt79-moonshine-small-streaming \
  /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/test-assets/two_cities_librivox_48k.wav
```

Latest run (exit 0):

```text
MODEL_LOAD_OK architecture=4 sample_rate=48000 sample_count=2700955 model_load_ms=599.8
SESSION 1 partial_calls=10 partial_nonempty=yes final_lines=3 final_nonempty=yes all_final_lines_complete=yes session_ms=1626.626 stop_to_final_ms=0.014 rtf=0.136
SESSION 2 partial_calls=10 partial_nonempty=yes final_lines=3 final_nonempty=yes all_final_lines_complete=yes session_ms=992.985 stop_to_final_ms=0.013 rtf=0.083
SMOKE_PROCESS_MAX_RSS_BYTES=886587392
```

This confirms the real native inference function ran on fixture audio, returned at least one nonempty partial and nonempty final transcript in both sessions, and finalized complete lines. The harness intentionally does not print transcript text; there is no semantic/reference comparison, WER result, Rust FFI ownership test, cancellation test, or microphone integration. Two consecutive sessions show basic stream reuse only.

Negative load check: running the harness with `/private/tmp/prompt79-empty-model` returned exit code 1 and the native API reported that `tokenizer.bin` was missing; the process exited without a crash. This checks native load failure only, not Outflow's runtime recovery UI. Existing Python validator tests cover missing-file and hash-mismatch cases; no app verifier is integrated.

## Verification commands and results

Run from repository root `/Users/bixxie/Desktop/Projects/outflow`:

| Exact command | Result |
|---|---|
| `python3 scripts/moonshine_model_source_audit.py --source-dir /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f` | PASS source contract; source checkout artifacts not verified by this audit; third-party notice review incomplete |
| `python3 scripts/validate_model_asset_inventory.py --manifest evidence/mvp/prompt-79-model-assets.json --asset-root /private/tmp/prompt79-moonshine-small-streaming` | PASS; eight local hashes; no redistribution approvals |
| `python3 -m unittest discover -s scripts/tests -v` | PASS; 311 Python tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 352 unit/integration tests and 1 documentation test |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 136 frontend tests in 10 files |
| `npm run lint` | PASS |
| `npm run build` | PASS; browser-only Vite output, not a desktop app |
| Native C++ compile/run command above | PASS; real model, two fixture sessions, nonempty partial/final lines |

All repository checks above were rerun after adding the smoke harness and docs. No app-level or UI acceptance check ran because no launchable desktop app exists. No test results here certify real macOS permissions, capture, insertion, accessibility, or offline first launch.

## End-to-end status

| Required path | Status | Evidence/limitation |
|---|---|---|
| Real Moonshine inference | **PASS (native harness only)** | Actual local model and pinned C API; two fixture sessions |
| Missing model native-load failure | **PASS (native harness only)** | Empty model directory returns a native error and nonzero status without crashing |
| Real microphone capture | **NOT RUN / BLOCKED** | No microphone service or app |
| Live partial transcription | **PASS (fixture-fed native stream only)** | 10 nonempty partial polls per 12-second session; not live mic |
| Final transcript cleanup | **NOT RUN** | No cleanup-to-recognition connection |
| macOS TextEdit insertion | **NOT RUN** | No target adapter or app |
| Protected field and changed-focus handling | **NOT RUN** | No macOS target validation |
| Explicit Copy Result fallback | **NOT RUN** | No result panel/clipboard action |
| Offline application operation | **NOT RUN** | Native smoke used local files, but no app/network-disabled run or packet capture |
| Cancel, mic release, permission denial, repeated app cycles | **NOT RUN** | No app/session/audio implementation |
| Corrupt bundle app recovery | **NOT RUN** | Existing development validator and its tests exist; no runtime verifier integrated |

No OS permission was granted or bypassed. The app was not launched because no app target or verified Tauri development command exists.

## Resource and release gates

| Measurement | Observed | Scope |
|---|---:|---|
| Model files | 142,300,974 bytes | Eight local ASR assets, no app or notices |
| Moonshine dylib | 10,164,432 bytes | Local arm64 build, includes broader dependency graph |
| ONNX Runtime dylib | 27,623,992 bytes | Existing arm64 dependency |
| Model load | 599.8 ms | Latest smoke run; cache/warmth not controlled |
| Session time / RTF | 1,626.6 ms / 0.136; 993.0 ms / 0.083 | Each session processes 12 s fixture audio |
| Stop-to-final | 0.014 ms / 0.013 ms | Native test only, not app insertion latency |
| Native smoke process peak RSS | 886,587,392 bytes latest; 922,386,432 bytes maximum observed across three runs | `getrusage(RUSAGE_SELF)`; not aggregate Outflow app RSS |
| Idle/app peak RSS, installed app bytes, CPU/battery | **NOT MEASURED** | No application build/run |

The native smoke RSS approaches the 950,000,000-byte limit on an 18 GB M3 Pro, so current evidence does not establish the release cap and suggests substantial model/runtime memory optimization or a measured candidate review may be necessary. It does not certify or disqualify a packaged app by itself. Physical 4 GB CPU-only behavior, benchmark WER/quality, broad latency, and AC-06 requirements are **NOT TESTED**. No Tiny fallback was evaluated or selected.

## Task ledger and blockers

`docs/TASKS.md` records M0 as in progress, M1 in progress, M2 not started, and this evidence without checking any boxes. T-009, T-010, T-013, T-022, and T-029 received partial feasibility evidence only and remain incomplete. T-003 still lacks the required 100-phrase corpus manifest; T-004 lacks a process-tree sampler; T-006 and all real insertion tests remain unrun. T-011 is gated by incomplete T-010. Audio/session/cleanup/insertion/UI tasks downstream remain blocked or not started.

The existing `evidence/model/moonshine-small-streaming-requirements-task-78.md` labels itself `T-078`, while canonical `docs/TASKS.md` uses T-078 for signed installer beta. This report does not alter or mark either task complete; coordinator review must resolve the ID collision before future ledger updates.

Outstanding blockers before a true Prompt 79 app slice:

1. Complete T-009 source/artifact/runtime license and notice review; obtain redistribution decisions for any release artifacts.
2. Complete T-003 benchmark corpus and T-004 process-tree sampler, identify physical 4 GB target hardware, then run the T-010 Small/Tiny quality, latency, size, and memory go/no-go without relaxing gates.
3. Resolve T-011/T-012/T-013 prerequisites before introducing the approved Tauri shell and production model bundle.
4. Implement and test Rust C ABI ownership, microphone capture/queue, explicit session controls, deterministic cleanup, target validation, and Copy Result; then run real microphone and two-application macOS tests with user-granted permissions.
5. Measure full app size/RSS and offline behavior. Keep all unverified acceptance/release gates marked NOT TESTED.

No screenshots were captured. No personal audio/transcript was used or committed. Native/model/audio artifacts and temporary build logs remain outside the repository.

## GitHub delivery

Repository: `https://github.com/thrns/outflow`. Branch: `codex/prompt79-macos-offline-dictation`. The partial implementation/evidence commit `a7c037030ec99731f183e25b45f88f6a7113b012` was pushed to that branch at [the GitHub commit page](https://github.com/thrns/outflow/commit/a7c037030ec99731f183e25b45f88f6a7113b012). The first push was verified with `git ls-remote origin refs/heads/codex/prompt79-macos-offline-dictation`, which returned that same SHA. A documentation-only follow-up records this push evidence; the final branch head is reported in the completion response. Model weights, native binaries, LFS inputs, temporary logs, and fixture audio are excluded from Git.
