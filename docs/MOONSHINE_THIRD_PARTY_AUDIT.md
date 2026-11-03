# Moonshine Third-Party Source Dependency Audit

- **Review date:** 2026-10-31
- **Reviewed source commit:** [`234f60faa0eb388b01cdf7e60aca232af37aefda`](https://github.com/moonshine-ai/moonshine/tree/234f60faa0eb388b01cdf7e60aca232af37aefda)
- **Review status:** Source inventory supplemented with Prompt 79 development-build evidence; redistribution approval **undetermined**.

**Scope:** The eight direct source entries in `core/third-party`, their bundled license documents, the pinned CMake references described below, and the explicitly bounded Prompt 79 macOS arm64 development build. This is not a legal opinion, installer review, or redistribution clearance. Prompt 79's compiled library is not a production Outflow artifact.

The machine-readable inventory is [`moonshine-third-party.json`](moonshine-third-party.json). Its classifications describe the pinned source tree. The original source-only review did not run a Moonshine build; Prompt 79C adds a separate macOS arm64 development-build supplement below. Model bytes and native binaries remain outside this repository.

## Findings at a glance

| Pinned directory | License evidence | Build-use classification | Selected English streaming runtime assessment |
|---|---|---|---|
| `Eigen` | `COPYING.MPL2` — MPL-2.0 | `confirmed-build-input` | Eigen is used by the cpp-annote source list included in the core target. `EIGEN_MPL2_ONLY=1` is set for that list. This does not verify all Eigen consumers or a binary. |
| `doctest` | `LICENSE.txt` — MIT | `test-only` | CMake references it from test targets; no production target use was identified. |
| `kaldi-native-fbank` | `LICENSE` — Apache-2.0 | `confirmed-build-input` | Feature-extraction source files are compiled through cpp-annote, which is added to the core target. Speaker diarization is optional behavior, but these sources are in the inspected target source list. |
| `kissfft` | `COPYING` — BSD-3-Clause | `confirmed-build-input` | `kiss_fft.c` and `kiss_fftr.c` are compiled through cpp-annote. The short `COPYING` points to a BSD-3-Clause text; its release notice placement still needs reconciliation. |
| `nlohmann` | `LICENSE.MIT` — MIT | `conditional` | JSON configuration is used in the TTS/G2P core. The root CMake target links its ONNX TTS target only when the TTS option is enabled; that option defaults to ON in the TTS CMake file. Outflow's build flag is not selected. |
| `onnxruntime` | `LICENSE.txt` — MIT | `confirmed-build-input` | The core links `ort-utils`, which links the imported ONNX Runtime library. Platform binary provenance and notices are not verified. |
| `utf-8` | `LICENSE.txt` — Boost Software License 1.0 (`BSL-1.0`) | `confirmed-build-input` | The transcriber includes `utf8.h`; the core target includes this directory. |
| `utf8proc` | `LICENSE.md` — MIT library code plus Unicode data license | `conditional` | The TTS/G2P core compiles `utf8proc.c`; linking TTS into Moonshine is option-controlled. The separate Unicode data notice applies to `utf8proc_data.c`. |

The classifications describe source-level CMake evidence at this commit. They do not establish which targets an Outflow release will build or which files an installer will distribute. A classification of `conditional` means the dependency belongs to an option-controlled path, not that the option is disabled. A classification of `test-only` means no production use was found in the inspected CMake references, not that package contents were inspected.

## Per-dependency review

### Eigen

The pinned tree contains `core/third-party/Eigen/COPYING.MPL2`; the license text identifies Mozilla Public License 2.0. Moonshine's pinned [license policy](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/docs/license.md) says the compiled Eigen subset is MPL-2.0 only. The pinned [core CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/CMakeLists.txt) adds cpp-annote's sources to the `moonshine` target and applies `EIGEN_MPL2_ONLY=1` to `CPP_ANNOTE_SRC_FILES`.

MPL-2.0 is not MIT: it has file-level source obligations for covered software. Its executable-form terms require making the covered source available and explaining how recipients can obtain it; notices in covered source must be retained. The macro is relevant to the included cpp-annote translation units, but the CMake setting is scoped to that list. It does not prove every Eigen include in every first-party translation unit is constrained, nor does it prove which objects are present in a released binary. Exact file-level scope, source availability, and notice packaging need review against the eventual build.

### doctest

The bundled `core/third-party/doctest/LICENSE.txt` identifies MIT and names Viktor Kirilov. Pinned references in the [core](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/CMakeLists.txt), [ort-utils](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/ort-utils/CMakeLists.txt), and [bin-tokenizer](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/bin-tokenizer/CMakeLists.txt) CMake files attach doctest include paths to test executables. The reviewed references support `test-only`; they do not show doctest linked into the production core target.

If doctest source or substantial portions are distributed, retain its copyright and MIT permission notice. Confirm release build and installer rules exclude test binaries and test headers before treating it as absent from a product package.

### kaldi-native-fbank

The bundled `core/third-party/kaldi-native-fbank/LICENSE` contains Apache License 2.0. The [pinned core CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/CMakeLists.txt) compiles its feature-fbank, feature-functions, feature-mfcc, feature-window, STFT/RFFT, and related sources as part of cpp-annote, which is added to `moonshine`.

Apache-2.0 requires a copy of the license, prominent notices on modified files, preservation of relevant source notices, and preservation of applicable notices if the distributed work includes a `NOTICE` file. It also contains an express patent grant and patent-litigation termination terms absent from the short MIT/BSD/Boost forms reviewed here. The selected app may not request speaker identification, but source inclusion is still confirmed by CMake. The exact upstream revision, modifications, and any additional NOTICE materials remain to be reconciled with the package.

### KISS FFT

The bundled `core/third-party/kissfft/COPYING` records copyright 2003–2010 Mark Borgerding, identifies `BSD-3-Clause`, and refers to `LICENSES/BSD-3-Clause`. The [pinned core CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/CMakeLists.txt) compiles `kiss_fft.c` and `kiss_fftr.c` through cpp-annote. The referenced full license text was not found at the expected `core/third-party/kissfft/LICENSES/BSD-3-Clause` path during this review, so the final notice location must be checked.

BSD-3-Clause requires retention of the copyright notice, license conditions, and disclaimer in source and binary redistribution materials, and prohibits using contributor names to endorse products without permission. It is permissive like MIT in broad use, but has an explicit non-endorsement clause and a different notice text. Redistribution remains undetermined until the complete notice is located and included as applicable.

### nlohmann/json

The pinned vendored directory contains `core/third-party/nlohmann/LICENSE.MIT`; its license identifier is MIT. The TTS CMake file lists JSON configuration source in the TTS/G2P core and exposes the `core/third-party` include root. The [core CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/CMakeLists.txt) links the ONNX TTS target only when `MOONSHINE_TTS_BUILD_ONNX` is enabled. The [TTS CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/moonshine-tts/CMakeLists.txt) declares that option with a default of `ON`.

The English STT path does not require JSON configuration from TTS by itself, but the source tree's default option can build/link optional TTS/G2P. This review therefore records `conditional`, not `not used`. Prompt 79's development build set the option to ON and linked the TTS/G2P target; the product build remains unselected. If included, preserve the copyright and MIT permission notice in copies or substantial portions.

### ONNX Runtime

The pinned vendor directory includes `LICENSE.txt` with Microsoft's MIT notice and a `README.md` that directs readers to the ONNX Runtime project. The [ort-utils CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/ort-utils/CMakeLists.txt) includes the vendored ORT project and links `ONNXRUNTIME_LIB_PATH`. The [vendor CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/third-party/onnxruntime/CMakeLists.txt) imports it as a platform library. The [path selection file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/third-party/onnxruntime/find-ort-library-path.cmake) chooses platform-specific files; the pinned [core CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/CMakeLists.txt) links `ort-utils` into the core target.

The inspected CMake distinguishes these source-level linking paths:

- Android, macOS, Linux, and Windows use imported shared-library targets. Windows uses an import library and has a helper that copies the corresponding DLL next to an executable.
- iOS and Swift use a static ORT archive. Static framework packaging merges the ORT archive into the Moonshine framework.
- WebAssembly uses a static archive produced by `scripts/build-ort-wasm.sh` rather than a Microsoft-provided prebuilt archive.

Static linking or archive merging does not remove the need to review and satisfy the relevant license and notice conditions. Dynamic linking still requires identifying and packaging the exact shared library and its notices. Prompt 79 linked and hashed a local macOS arm64 ORT dylib; the exact binary provenance and full notice set remain unverified.

ONNX Runtime's upstream notice set is separate from the short MIT license. The official [ONNX Runtime v1.23.2 ThirdPartyNotices.txt](https://raw.githubusercontent.com/microsoft/onnxruntime/v1.23.2/ThirdPartyNotices.txt) lists additional components and terms, including notices for ONNX, protobuf, zlib, pybind11, and Intel MKL. The local Moonshine dylib links a library whose filename contains `1.23.2`, but that alone does not prove all platform libraries were built from that exact release or carry that same notice set. Obtain notices for each exact binary or source build selected for Outflow, reconcile all third-party components and licenses, and include the required materials. The development-binary hash and link evidence are in the supplement.

### UTF8-CPP (`utf-8`)

The bundled `core/third-party/utf-8/LICENSE.txt` is the Boost Software License Version 1.0, SPDX identifier `BSL-1.0`; the source is acknowledged as UTF8-CPP. The [pinned `transcriber.cpp`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/transcriber.cpp) includes `utf8.h`, and the [core CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/CMakeLists.txt) adds the vendored directory to the core target's include paths. This is a confirmed transcription source input.

BSL-1.0 is not MIT. It requires copyright notices and the license statement with copies and derivatives, with a specific exception for copies solely in machine-executable object-code form generated by a source language processor. The distribution still needs a deliberate source/notice review; do not replace this license with the project's MIT notice.

### utf8proc

The bundled `core/third-party/utf8proc/LICENSE.md` contains separate terms: MIT for utf8proc library code and a Unicode license notice for the derived `utf8proc_data.c` data. The inventory records both identifiers (`MIT` and `Unicode-3.0`) because the data notice is not subsumed by the library's MIT grant. MIT requires copyright and permission notice retention. The Unicode data terms require the copyright and permission notice with copies and in associated documentation, a clear notice if the data is modified, and restrict promotional use of Unicode names or marks without authorization.

The [TTS CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/moonshine-tts/CMakeLists.txt) compiles `utf8proc.c` and links it into the TTS/G2P core. The main [core CMake file](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/core/CMakeLists.txt) adds TTS to Moonshine only when its build option is enabled; the option defaults to ON. Prompt 79's development build used ON; the future product build remains unselected, so source inventory use remains conditional.

## License differences and practical obligations

- **MIT:** broad permission to use, modify, distribute, sublicense, and sell; retain the copyright and permission notice in copies or substantial portions. Applies to doctest, nlohmann/json, ONNX Runtime's own source license, and utf8proc library code.
- **Apache-2.0:** requires license and applicable notices, marks changed files, has an express patent license and patent-litigation termination provision, and has rules for any distributed `NOTICE` file. It applies to kaldi-native-fbank; it is not interchangeable with MIT.
- **MPL-2.0:** file-level copyleft with source-form and executable-form obligations for covered files, plus notice preservation. It applies to the inspected Eigen subset; the `EIGEN_MPL2_ONLY` compile definition is scoped to cpp-annote sources.
- **BSD-3-Clause:** requires retention of copyright, conditions, and disclaimer, and prohibits endorsement using contributor names without permission. KISS FFT's short `COPYING` identifies this license and points to a full BSD text whose final package location must be checked.
- **Boost Software License 1.0:** permissive terms with an express copyright/license notice rule and a generated-object-code exception. It covers UTF8-CPP and must not be mislabeled as MIT.
- **Unicode data terms:** utf8proc's Unicode-derived data carries a separate copyright and permission notice, associated-documentation, modification-notice, and name-use condition. A single MIT label would omit that material obligation.

These summaries identify issues for engineering review; they do not decide legal interpretation or establish redistribution rights.

## Open requirements before redistribution review

1. Fix the actual Moonshine source commit, build configuration, exact selected targets, and Outflow's per-platform runtime artifacts.
2. Verify each selected library's source/binary provenance, version, hash, platform architecture, and static/dynamic linkage. The pinned source CMake is not binary evidence.
3. Obtain the exact ONNX Runtime `ThirdPartyNotices.txt` and component licenses for every selected binary or source build; reconcile the full transitive notice set.
4. Resolve the kissfft full BSD-3-Clause text and confirm notice packaging. Confirm Apache `NOTICE` applicability and required MPL source availability for the selected files.
5. Decide whether optional TTS/G2P is included; if so, include nlohmann, utf8proc, Unicode data, TTS models/data, and their own licensing review as applicable.
6. Reconcile notices against each installer and obtain final human/legal redistribution review. Until then, redistribution approval remains **undetermined**.

The audit's Python validator checks local inventory consistency and immutable references only. It does not contact the network or test whether the source evidence is legally sufficient.

## Prompt 79 macOS development-build supplement

Prompt 79 built the pinned commit on macOS 26.6.2 arm64 with Apple Clang 21.0.0.21000101 and `MOONSHINE_TTS_BUILD_ONNX=ON`. The external development-only `libmoonshine.dylib` is 10,164,432 bytes, SHA-256 `42128afcc4ad6f71d8f6c79012aa81d0b8755c3c6b393c8f9c38029a7de946f5`. `otool -L` shows it dynamically links `libonnxruntime.1.23.2.dylib` (27,623,992 bytes, SHA-256 `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`), plus system `libc++` and `libSystem`. The filenames/architecture and hashes identify local bytes; they do not establish full binary build provenance or product package contents.

The successful TTS-enabled build compiled and exported TTS/G2P and `cppannote` symbols. Source and link evidence confirms:

- `core/moonshine-tts/CMakeLists.txt` adds `moonshine_tts_ort` and links it into `moonshine` when `MOONSHINE_TTS_BUILD_ONNX=ON`. The compiled branch includes ZipVoice/TTS code and `zipvoice-voices-data.cpp`; that source is 11,695,724 bytes and contains embedded VCTK reference clips. The source acknowledgements identify VCTK, but no artifact-specific license/redistribution terms for the embedded clips were established. Do not redistribute this binary until those terms are reviewed.
- `core/CMakeLists.txt` unconditionally appends cpp-annote, KISS FFT, and kaldi-native-fbank sources to the `moonshine` target, regardless of the TTS flag. Exported `cppannote`/embedded community-1 symbols were found in the built dylib. `community1_cpp_annote_embedded.cpp` is 2,535,244 bytes (SHA-256 `9424da4176b33e67e4000ea2a776d64b6a78ee9bbf72d40405fb6805d12758c4`) and contains embedded PLDA/config/geometry data. The README attributes the community-1 diarization model family to pyannote.ai under CC BY 4.0; the larger segmentation/embedding weights were deleted from this vendored file and are external model inputs. Prompt 79 used `identify_speakers=false`; that means the smoke did not request diarization inference, not that its code/config data were absent.
- With `MOONSHINE_TTS_BUILD_ONNX=OFF`, upstream CMake configured, but building `moonshine` first failed because `moonshine-c-api.cpp` still includes `moonshine-asset-catalog.h` while the TTS source include directory is conditional. An external wrapper adding that include directory let compilation continue, but linking failed on TTS/G2P APIs and asset-catalog functions referenced by the same C API. The same OFF build compiled cpp-annote, KISS FFT, and kaldi-native-fbank. No pinned source file was edited. An STT-only C API target requires an upstream change or a separately reviewed wrapper/interface contract.
- The local ONNX Runtime directory contains Microsoft's MIT `LICENSE.txt`. The official [v1.23.2 ThirdPartyNotices.txt](https://raw.githubusercontent.com/microsoft/onnxruntime/v1.23.2/ThirdPartyNotices.txt) was checked and includes notices for additional components such as ONNX/Apache-2.0, protobuf/BSD-3-Clause, zlib, pybind11, and Intel MKL. This notice file has not been matched to the local dylib's exact source/build provenance or reconciled into a product package.
- The pinned KISS FFT `COPYING` identifies BSD-3-Clause and points to `LICENSES/BSD-3-Clause`, which is absent from the checkout. The canonical text was checked against the [SPDX BSD-3-Clause License List](https://raw.githubusercontent.com/spdx/license-list-data/main/text/BSD-3-Clause.txt); no release notice file was created. Notice delivery remains blocked.

Detailed file identities, build commands, runtime measurements, and exact open review questions are in [`evidence/mvp/prompt-79c-native-license-audit.md`](../evidence/mvp/prompt-79c-native-license-audit.md). The local model inventory separates Moonshine source commit, vendor catalog path, absent immutable artifact revision, and observed per-file SHA-256. The pinned source calls English streaming STT MIT at the family level, but it does not give an immutable revision or artifact-specific redistribution approval for the eight local quantized assets. T-009 remains incomplete.
