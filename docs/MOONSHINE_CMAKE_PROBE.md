# Moonshine CMake Source Probe

**Status:** Read-only source verification for the Task 23 build-scope checklist. This probe does not configure or compile Moonshine, fetch model weights or native libraries, change upstream files, or inspect a binary. It supports T-009 evidence only; T-009 remains unchecked.

## Pinned source inputs

The probe accepts only revision [`234f60faa0eb388b01cdf7e60aca232af37aefda`](https://github.com/moonshine-ai/moonshine/tree/234f60faa0eb388b01cdf7e60aca232af37aefda). It computes Git blob object IDs as `SHA-1("blob " + byte_length + NUL + file_bytes)` and compares each file with this exact list:

| Pinned source file | Verified Git blob SHA |
|---|---|
| [`core/CMakeLists.txt`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/CMakeLists.txt) | `30c472029b6f219afff9df11faa03d413064834b` |
| [`core/moonshine-tts/CMakeLists.txt`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/moonshine-tts/CMakeLists.txt) | `af834942deba237ec3e6d46cca1be4983a839fe1` |
| [`core/ort-utils/CMakeLists.txt`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/ort-utils/CMakeLists.txt) | `dddcabd53385365dfdef9592395498616f95dc7f` |
| [`core/bin-tokenizer/CMakeLists.txt`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/bin-tokenizer/CMakeLists.txt) | `55c1dc34f03a136a8243b589b9d03b1c8c6efb77` |
| [`core/third-party/onnxruntime/CMakeLists.txt`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/third-party/onnxruntime/CMakeLists.txt) | `a07e58a6b793771046da1fd9a93fde4dc8bc77e2` |
| [`core/third-party/onnxruntime/find-ort-library-path.cmake`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/third-party/onnxruntime/find-ort-library-path.cmake) | `9699d99ca60471963e1d0e12017048b62087c24d` |

These are Git source-blob identifiers, not model or binary checksums. A missing required file, changed bytes, invalid text, unexpected redirect, or upstream retrieval error fails the probe. The CMake matcher handles the observed command syntax, quoted strings, multiline calls, and `#` comments. It is intentionally not a complete CMake parser.

## Source findings

After all six Git blob IDs match, the probe verifies these narrow behaviors:

- `MOONSHINE_TTS_BUILD_ONNX` is declared with default `ON`. The TTS subdirectory is added unconditionally by the root CMake file. The option guards the ONNX TTS library/link section; `moonshine_tts_core` and `moonshine_tts_utf8proc` are defined outside that guard. Turning the option off is not a verified way to omit the TTS subtree.
- `speaker-diarizer.cpp` is in `MOONSHINE_SRC_FILES`, and `CPP_ANNOTE_SRC_FILES` is appended to that main list. This proves source-list inclusion only; the probe makes no object or binary-presence claim.
- The main `moonshine` target references `bin-tokenizer` and `ort-utils`. The `ort-utils` CMake file links the configured ONNX Runtime library path, and the ONNX Runtime CMake files declare the imported runtime target and path resolver.
- Test executables, test-builder behavior, benchmarks, and build utilities are declared in source. Those declarations do not prove which targets a selected build command compiles or what is packaged.

The other reviewed option findings are:

| Variable | Source finding |
|---|---|
| `MOONSHINE_RELIABILITY` | `option()` default is `OFF`; this is a reliability/sanitizer control, not a general test-disable option. |
| `MOONSHINE_SANITIZER` | Cache default is `address,undefined`; the probe does not infer a CPU-only runtime setting from it. |
| `MOONSHINE_BUILD_SHARED` | Referenced in the main library kind condition; no active `option()` or `set()` assignment/default is present in the inspected files. Default remains unknown. |
| `MOONSHINE_BUILD_SWIFT` | Referenced in the main library kind condition; no active `option()` or `set()` assignment/default is present in the inspected files. Default remains unknown. |

No complete minimal-STT configuration has been proven. In particular, this source review does not show that diarization, TTS, tests, benchmarks, utilities, or auxiliary sources are excluded. It also does not prove a CPU-only provider selection, selected model assets, or production packaging.

## What each verification stage proves

| Stage | What it can establish | Status for this probe |
|---|---|---|
| Source integrity | Exact bytes of the six pinned CMake text files match the listed Git blob IDs. | Verified by this probe when it passes. |
| CMake source analysis | The listed declarations and target/source relationships appear in active, non-comment CMake commands. | Verified by this probe when it passes. |
| Intended minimal STT scope | Outflow's intended scope remains documented in [`MOONSHINE_BUILD_SCOPE.md`](MOONSHINE_BUILD_SCOPE.md). | Documented; no configuration is proven. |
| CMake configure | Generator-specific target graph and configure-time inputs. | Not run. |
| Compilation | Actual object generation and compiler behavior. | Not run. |
| Linking | Final library/executable composition and transitive link behavior. | Not run. |
| Binary/package inspection | Actual symbols, objects, assets, and installed package contents. | Not verified. |

The probe must not output a CMake command that claims to remove unverified components. A successful result means **minimal STT exclusions: NOT PROVEN**.

## Run the probe

Offline verification reads only the six files under an existing pinned source checkout and never opens a network connection:

```sh
python3 scripts/probe_moonshine_cmake.py \
  --source-dir /path/to/pinned/moonshine-source
```

The explicit upstream mode retrieves only the six commit-specific CMake text files over HTTPS. It retains bytes in memory and does not persist them to the repository:

```sh
python3 scripts/probe_moonshine_cmake.py --verify-upstream
```

Both modes use Python's standard library. No CMake configure, compilation, model download, or native-library download is performed. If Python cannot validate the HTTPS certificate because its CA bundle is unavailable, configure `SSL_CERT_FILE` to a trusted CA bundle and rerun; do not disable TLS verification. An upstream network, TLS, HTTP, redirect, or source-integrity failure is reported as `BLOCKED` or `ERROR`, never as a pass.

On successful source verification, the summary includes:

```text
Outflow Moonshine CMake source probe passed
Source revision: 234f60faa0eb388b01cdf7e60aca232af37aefda
Pinned source files: 6 verified
TTS ONNX default: ON
TTS subtree: included
Diarization sources: included in main target
Minimal STT exclusions: NOT PROVEN
CMake configure: NOT RUN
Native compilation: NOT RUN
Binary contents: NOT VERIFIED
```

The command then prints source-integrity and source-analysis states, unknown variable defaults, and all six verified blob IDs. It reports target declarations separately from production packaging.

## Before a native configure experiment

The next step is not a configure command. Before an approved native configure experiment, record the intended target and platform/generator, obtain the complete source checkout at this exact revision, and verify every configure-time dependency is already present locally. Resolve the ONNX Runtime artifact, ABI, link mode, provider behavior, and notices for each target. Decide how the intended minimal STT scope will be measured because the inspected source has no verified global TTS, test, benchmark, or diarization exclusion. Keep configuration output in an isolated build directory, and inspect the generated target graph before any compilation. Do not interpret target declarations as installer contents.

T-009 remains unchecked. T-010, T-013, AC-01, and AC-06 remain incomplete; no hardware, model-quality, size, latency, or runtime gate is satisfied by this source probe.
