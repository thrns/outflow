# Prompt 79D — Native Runtime and Corpus Licensing Review

**Date:** 2026-11-03
**Scope:** T-003, T-009, T-010, F-06, AC-06
**Result:** Partial evidence only; redistribution approval remains `undetermined`.
**Pinned Moonshine source:** `234f60faa0eb388b01cdf7e60aca232af37aefda`
**Tested native build:** local macOS arm64 development build with `MOONSHINE_TTS_BUILD_ONNX=ON`; not an Outflow application or release package.

## Findings by evidence type

| Item | Evidence established | Remaining review or approval |
|---|---|---|
| Moonshine source code | The exact upstream source checkout is pinned and built locally. Its repository-level MIT license does not cover `core/third-party`. | Reconcile notices for every selected source/runtime and actual installer. |
| Small Streaming model assets | Eight local file hashes validate against the observed manifest; total `142,300,974` bytes. Upstream calls the English streaming model family MIT. | No immutable artifact revision or artifact-specific rights record; no redistribution approval. |
| Tiny Streaming | No verified local Tiny assets or benchmark run. | Separate acquisition authorization plus hashes, exact source metadata, artifact terms, and the same quality/resource evaluation. |
| ONNX Runtime | The running helper loaded the exact selected macOS arm64 `libonnxruntime.1.23.2.dylib`; observed SHA-256 is recorded in the runtime identity evidence. The local source directory has Microsoft's MIT license text. | The shared-library build provenance is not established. The official 1.23.2 transitive notice list is not proven to match this binary or reconciled into a package. Other platforms are unreviewed. |
| KISS FFT | `kiss_fft.c` and `kiss_fftr.c` are compiled through cpp-annote into the tested Moonshine library. The pinned `COPYING` identifies BSD-3-Clause and Mark Borgerding. Prompt 79D added the full standard text at [`docs/licenses/KISS-FFT-BSD-3-Clause.txt`](../../docs/licenses/KISS-FFT-BSD-3-Clause.txt), checked against the [SPDX BSD-3-Clause text](https://spdx.org/licenses/BSD-3-Clause.html). | Confirm inclusion in every actual source/binary distribution. This documentation file does not establish product packaging or legal approval. |
| TTS/G2P, nlohmann/json, utf8proc | The tested development library was built with TTS ON; TTS/G2P symbols are present. The pinned TTS-off C API build failed at compile/link. | Decide product scope through approved source/interface work. Review utf8proc's separate Unicode data terms against exact object use and include required notices. |
| Diarization and embedded data | `cppannote` symbols and community-1 embedded configuration data are present in the library; speaker identification was disabled in smoke runs. | Resolve community-1 attribution/rights and the exact pyannote data provenance. No per-feature RSS isolation was measured. |
| Embedded VCTK reference voices | The TTS-enabled branch includes `zipvoice-voices-data.cpp` with compiled voice clips. | The pinned checkout did not establish artifact-specific VCTK distribution rights. Do not redistribute the tested binary until reviewed. |
| kaldi-native-fbank / Eigen | kaldi-native-fbank, cpp-annote and Eigen-backed sources were compiled. | Confirm Apache notice applicability, exact MPL-2.0 covered files/source obligations, and the complete package notice set. |

The standalone verifier checks the active process, not merely paths supplied on a command line. It checks arm64 architecture, C API version 30000, the image containing `moonshine_get_version`, exact loaded Moonshine and ONNX Runtime paths, `otool -L` dependency chain, `lipo` architecture, hashes before and after probing, and rejects unexpected non-system images. On this macOS version, Apple system libraries may be listed from the dyld shared cache without standalone hashable files; the verifier permits only the documented system roots and records paths/counts rather than hashes for those images. This verifies the development process identity, not the supplier provenance or notices of system components.

The native library also maps Apple system frameworks that ONNX Runtime ships alongside its providers. The inference harness explicitly selected `CPUExecutionProvider` and produced real partial/final results; a mapped framework is not evidence that its execution provider performed inference. The user's CPU-only product requirement remains unverified on release hardware and on other operating systems.

## Benchmark-source review

- Mozilla's current [Common Voice terms](https://commonvoice.mozilla.org/dav/terms) make the dataset available under CC0 but ask users not to post, distribute, or mirror any Common Voice dataset, in whole or part, outside Mozilla Data Collective. That conflicts with placing a reusable subset in this repository. No data was downloaded. The project needs written Mozilla clarification plus human/legal review before considering it as a checked-in corpus source.
- [OpenSLR LibriSpeech SLR12](https://us.openslr.org/12/) lists CC BY 4.0. A small, attributable subset may be viable, but this milestone did not select or download any exact file: the archives are hundreds of megabytes, individual file identity and transcription provenance must be reviewed, and no human reviewer checked a complete clip/reference pair. It also cannot alone meet noise and accent-diversity requirements.
- The pinned Moonshine checkout's `two_cities_librivox_48k.wav` is a development smoke input only. No human-verified Outflow reference, file-level permission review, or frozen manifest entry exists for it. No WER was calculated.

## Current notice state

`python3 scripts/validate_moonshine_third_party.py` and `python3 scripts/validate_third_party_notices.py` validate the local inventory structure, not legal sufficiency. The KISS FFT text gap is improved, but the existing notice manifest remains `INCOMPLETE`, has no packaged notices, and retains `redistribution_approval: "undetermined"`. Open items include exact ONNX Runtime provenance/notices, TTS/G2P and Unicode data scope, pyannote/VCTK data rights, platform-specific runtime graphs, and human/legal packaging review. T-009 is not complete.

See [`docs/MOONSHINE_LICENSE_REVIEW.md`](../../docs/MOONSHINE_LICENSE_REVIEW.md), [`docs/MOONSHINE_THIRD_PARTY_AUDIT.md`](../../docs/MOONSHINE_THIRD_PARTY_AUDIT.md), [`docs/THIRD_PARTY_NOTICES.md`](../../docs/THIRD_PARTY_NOTICES.md), and [`evidence/mvp/prompt-79c-native-license-audit.md`](prompt-79c-native-license-audit.md) for the underlying source and build evidence.
