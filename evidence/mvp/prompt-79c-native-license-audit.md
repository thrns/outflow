# Prompt 79C Native Build and License Evidence

**Date:** 2026-11-03
**Status:** Development-build scope established; T-009 incomplete; redistribution approval undetermined.
**Upstream:** `moonshine-ai/moonshine`, commit `234f60faa0eb388b01cdf7e60aca232af37aefda`.
**Platform:** macOS 26.6.2 (25G83), Apple M3 Pro arm64, 18 GB physical RAM.
**Build owner:** Codex coordinator.
**Scope:** Exact local Moonshine macOS native build, direct runtime link, model-source split, CMake TTS-off feasibility, and third-party inputs. No installer or redistribution was produced.

## Identity: source, model catalog, artifacts, approval

These are separate facts and must remain separate in future manifests:

| Identity | Observed value | What it establishes |
|---|---|---|
| Moonshine source revision | `234f60faa0eb388b01cdf7e60aca232af37aefda` | Exact clean upstream source checkout used by the local build. |
| C API header | `MOONSHINE_HEADER_VERSION=30000` | Header ABI expected by the native helper. |
| Model architecture | `MOONSHINE_MODEL_ARCH_SMALL_STREAMING=4` | Streaming Small architecture selected by the C API helper. |
| Vendor catalog path | `small-streaming-en/quantized_26_08_21` | Dated registry/catalog location for the English quantized Small files. |
| Immutable model artifact revision | **Unavailable** | The catalog path does not expose a separate immutable artifact revision. Do not substitute a source commit or file hash. |
| Local artifact SHA-256 | Eight per-file hashes in [`prompt-79-model-assets.json`](prompt-79-model-assets.json) | Identifies the exact local bytes. Eight files total 142,300,974 bytes. |
| Model-family license evidence | Pinned Moonshine docs identify English streaming STT model families as MIT | Family-level license statement only; it does not prove artifact-specific rights for these quantized files. |
| Redistribution status | `undetermined` | No package or production model manifest is authorized. |

The inventory validator accepts `source_revision: null` only because each verified file explicitly says the vendor artifact revision is unavailable. It separately verifies the observed local SHA-256. This does not certify an upstream artifact version or permit redistribution.

## Successful native development build

The external wrapper at `/private/tmp/prompt79-moonshine-wrapper/CMakeLists.txt` adds the pinned `core/` tree and force-selects `MOONSHINE_TTS_BUILD_ONNX=ON`. No file in the pinned source checkout was modified. The build used Apple Clang 21.0.0.21000101, CMake 4.4.5, Release, arm64, and target `moonshine`:

```sh
cmake -S /private/tmp/prompt79-moonshine-wrapper \
  -B /private/tmp/prompt79-moonshine-wrapper-build-onnx \
  -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_ARCHITECTURES=arm64 \
  -DMOONSHINE_TTS_BUILD_ONNX=ON
cmake --build /private/tmp/prompt79-moonshine-wrapper-build-onnx \
  --clean-first --target moonshine --config Release --parallel 4
```

Both commands completed successfully. The native library output was:

| Artifact | Local path | Bytes | SHA-256 |
|---|---|---:|---|
| Moonshine dylib | `/private/tmp/prompt79-moonshine-wrapper-build-onnx/moonshine-core/libmoonshine.dylib` | 10,164,432 | `42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5` |
| ONNX Runtime dylib | `<source>/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib` | 27,623,992 | `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4` |

`otool -L` confirms dynamic linkage to `@rpath/libonnxruntime.1.23.2.dylib`, `/usr/lib/libc++.1.dylib`, and `/usr/lib/libSystem.B.dylib`. `nm -gU` found exported TTS/G2P symbols (including ZipVoice and nlohmann JSON types) and `cppannote`/embedded community-1 symbols. This is one local macOS development artifact, not an Outflow app runtime or release artifact. The binary+model bytes total 180,089,398 bytes, but that sum omits all application, packaging, and installer assets and is not installed-size evidence.

## Why the current revision does not produce an STT-only C API library

The CMake option exists, but it does not cleanly sever TTS from this revision's C API:

1. The first external wrapper set `MOONSHINE_TTS_BUILD_ONNX=OFF` and added the pinned `core/` tree without extra TTS include paths. CMake configuration passed. Building target `moonshine` failed while compiling `core/moonshine-c-api.cpp`: `moonshine-asset-catalog.h` was not found. The C API source includes that TTS asset-catalog header unconditionally, while the main target adds the TTS source include directory only under the ON condition.
2. A second temporary wrapper added the TTS source include directory after `add_subdirectory`. Compilation then reached link and failed with undefined references from `moonshine-c-api.cpp.o` to `MoonshineTTS`, `MoonshineG2P`, `split_sentences`, and asset-catalog functions such as `moonshine_asset_catalog_all_registered_language_tags` and `moonshine_asset_catalog_g2p_dependency_keys`. The OFF configuration does not link the implementation target that defines those symbols.
3. Both ON and OFF source lists append `CPP_ANNOTE_SRC_FILES` to `MOONSHINE_SRC_FILES` unconditionally. The OFF build log compiled `cpp-annote`, `community1_cpp_annote_embedded.cpp`, KISS FFT `kiss_fft.c`/`kiss_fftr.c`, and kaldi-native-fbank feature sources before link failure.

Therefore an upstream-supported option alone did not yield an STT-only C API library. A target-specific wrapper that edits `MOONSHINE_SRC_FILES` or supplies alternate C API stubs would alter the pinned source contract and require separate review/approval. The safe next choices are an upstream revision with a defined STT-only C API boundary or a separately approved maintained wrapper/interface. No such change is made here.

The current evidence proves TTS and diarization code/config are in the built library; it does not quantify their individual contribution to RSS. The 100 ms native RSS measurements include the complete loaded library and ONNX runtime. No matched before/after isolation experiment succeeded.

## Third-party and data review

| Component or data | Actual Prompt 79 build evidence | License/notice status and remaining work |
|---|---|---|
| Moonshine-authored C++ | Pinned source built into `libmoonshine.dylib` | Source repo MIT notice is available. Preserve applicable notice and review human redistribution terms. |
| Small Streaming model files | Eight local files loaded by native helper; 142,300,974 bytes | Family-level MIT evidence exists; exact artifact revision/terms absent; no redistribution approval. |
| ONNX Runtime | Local macOS arm64 shared dylib linked; hash above | Local source `LICENSE.txt` is Microsoft's MIT text. The official [v1.23.2 ThirdPartyNotices.txt](https://raw.githubusercontent.com/microsoft/onnxruntime/v1.23.2/ThirdPartyNotices.txt) was inspected and lists a separate transitive notice set (including ONNX, protobuf, zlib, pybind11 and Intel MKL). The local dylib's exact source provenance and match to that file remain unproven; no notice bundle was assembled. |
| KISS FFT | Two C sources compiled into `moonshine` through cpp-annote, including under TTS OFF | `COPYING` identifies BSD-3-Clause but references missing `LICENSES/BSD-3-Clause`. The [canonical SPDX text](https://raw.githubusercontent.com/spdx/license-list-data/main/text/BSD-3-Clause.txt) was checked; the exact full notice is not in the pinned checkout or Outflow package. Preserve Mark Borgerding's copyright/conditions in a reviewed notice destination. |
| kaldi-native-fbank | Feature-extraction C++ sources compiled into `moonshine` through cpp-annote | Pinned source includes Apache-2.0 `LICENSE`; review applicable NOTICE materials and package attribution. |
| Eigen / cpp-annote | Eigen-backed `cppannote` sources compiled; `EIGEN_MPL2_ONLY=1` is applied to the listed sources | MPL-2.0 subset indicated; exact used-file/source availability and notice obligations still require review. cpp-annote source itself has MIT license. |
| Community-1 diarization embedded data | `community1_cpp_annote_embedded.cpp` is 2,535,244 source bytes, SHA-256 `9424da4176b33e67e4000ea2a776d64b6a78ee9bbf72d40405fb6805d12758c4`; symbols are present | Source README identifies pyannote.ai community-1 models under CC BY 4.0. This file holds compiled PLDA/config/geometry, not the removed 8.2 MB segmentation/embedding weights. Exact attribution and distribution rights for the embedded data need human review. Runtime smoke used `identify_speakers=false`; it did not perform diarization. |
| TTS/G2P code and nlohmann/json | TTS ONNX target linked; nlohmann JSON symbols in dylib | nlohmann's vendored MIT notice applies where code/header copies ship. Product decision to exclude TTS/G2P unresolved. |
| utf8proc | TTS/G2P branch built from source and linked | Vendored license file contains MIT code plus separate Unicode data terms for `utf8proc_data.c`. Exact selected data object and notice in final binary were not resolved. |
| ZipVoice embedded VCTK reference voices | TTS ONNX source includes `zipvoice-voices-data.cpp` (11,695,724 bytes); ZipVoice TTS symbols present in dylib. Source SHA-256 `f3e4d62cae93c465e1de8521bc5706b9a20f834cbbf37404bf03f0d35dffa012`. | Pinned `zipvoice/README.md` says the voices are VCTK clips compiled into `libmoonshine`; upstream acknowledgements cite the VCTK dataset but this checkout provides no artifact-specific license/redistribution terms for these exact voice clips. Human review is required. |
| doctest and developer executables | Test targets are declared by CMake; no release package was produced | Do not infer they ship from source-tree declarations. Verify exact packaging later. |

The official notice URLs establish available license text only; a license identifier or notice file is not itself legal clearance. `docs/third-party-notices.json` remains conservative: 0 packaged notices, 4 planned, 4 blocked, and redistribution approval `undetermined`. T-009 cannot be marked complete until the exact release binaries, notices, model terms, embedded-data terms, and human/legal review are resolved.

## External verification outputs

The source checkout remained clean and pinned throughout these experiments. Build logs, temporary wrappers, local runtime artifacts, model files, fixture audio, and process snapshots remain outside the Outflow repository under `/private/tmp` or in the separate Moonshine checkout. The relevant exact paths and hashes are above; no model, audio fixture, native binary, or raw transcript was committed.
