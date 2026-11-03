# Task 77 — Moonshine Native Dependency and License Inventory

```yaml
task_id: T-077
feature_ids: [F-01, F-06, F-12]
flows: [UF-01, UF-05, UF-10]
acceptance: [AC-01, AC-06, AC-12]
components: [C-05, C-09]
stack: [TS-05, TS-09]
owner: Codex coordinator in the existing codex/task23-moonshine-build-scope worktree
base_commit: f4dd938bd7220395b39dcea7538fdcadec14d71d
result_commit: uncommitted
platform: macOS 26.6.2 arm64; not physical 4 GB release evidence
implementation: scripts/moonshine_native_license_inventory.py; scripts/tests/test_moonshine_native_license_inventory.py; README.md; this evidence
review: PENDING
```

## Scope and verification

This is a read-only source and CMake target inventory for T-009. Existing license tools were inspected first. `validate_moonshine_third_party.py` validates the recorded eight-entry source audit, and `validate_moonshine_license_review.py` validates recorded review claims. Neither accepts a source/build directory pair, derives the configured target graph, or hashes the current local license files, so the focused command below fills that gap without replacing either validator.

The Moonshine checkout was clean at revision `234f60faa0eb388b01cdf7e60aca232af37aefda`. The Task 76 cache and File API reply matched `source-dir/core` and the retained build directory. The generated platform was Darwin arm64, `CMAKE_OSX_ARCHITECTURES=arm64`, build type `Release`, and `MOONSHINE_TTS_BUILD_ONNX=OFF`. The File API identifies `moonshine` as a `SHARED_LIBRARY`; the target was not built.

The selected target has 45 C/C++ source entries: 21 Moonshine core files, 10 `cpp-annote` files, 12 kaldi-native-fbank files, and 2 KISS FFT files. Its configured CMake target dependencies are `bin-tokenizer`, `moonshine-utils`, and `ort-utils`; its link fragments include the verified ONNX Runtime dylib. The compile groups also include Eigen (with `EIGEN_MPL2_ONLY=1`) and UTF8-CPP headers. This is the configured target graph, not proof that every included feature is called by Outflow or present in a released binary.

## Native component inventory

| Component | Role | Source / configuration evidence | Local license evidence | Status |
|---|---|---|---|---|
| Moonshine streaming C API target (`moonshine`) | Runtime | `core/`, shared target; 45 source entries | `LICENSE` and `docs/license.md` (MIT source scope outside `core/third-party`) | Evidence present |
| `bin-tokenizer`, `moonshine-utils`, `ort-utils` | Runtime | CMake dependencies of `moonshine`; static archives in generated link fragments | `LICENSE` (same upstream source scope) | Evidence present |
| `cpp-annote` | Runtime source included in target | `core/cpp-annote/src/` | `core/cpp-annote/LICENSE` (MIT) | Evidence present |
| Eigen | Runtime header input to `cpp-annote` | `core/third-party/Eigen/`; `EIGEN_MPL2_ONLY=1` on the cpp-annote source list | `core/third-party/Eigen/COPYING.MPL2` (MPL-2.0) | Evidence present; file-level MPL obligations need human review |
| kaldi-native-fbank | Runtime source included in target | `core/third-party/kaldi-native-fbank/csrc/` | `core/third-party/kaldi-native-fbank/LICENSE` (Apache-2.0) | Evidence present; package notices not reconciled |
| KISS FFT | Runtime source included in target | `core/third-party/kissfft/` | `core/third-party/kissfft/COPYING` (BSD-3-Clause identifier) | **Unresolved:** COPYING points to missing `core/third-party/kissfft/LICENSES/BSD-3-Clause` |
| UTF8-CPP | Runtime header input to `core/transcriber.cpp` | `core/third-party/utf-8/` include path | `core/third-party/utf-8/LICENSE.txt` (BSL-1.0) | Evidence present |
| ONNX Runtime 1.23.2 | Runtime shared library | `core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib`; File API link fragment | `core/third-party/onnxruntime/LICENSE.txt` (MIT); local `README.md` points to the upstream project | **Unresolved:** no local `ThirdPartyNotices.txt`/`NOTICE` evidence found |
| doctest | Build-only, test targets | Third-party include used by test targets; not a `moonshine` dependency | `core/third-party/doctest/LICENSE.txt` (MIT) | Evidence present |
| nlohmann/json | Optional TTS/G2P | Not linked by `moonshine` with the recorded TTS option off | `core/third-party/nlohmann/LICENSE.MIT` (MIT) | Evidence present; optional path |
| utf8proc | Optional TTS/G2P | `moonshine_tts_utf8proc` target is outside the `moonshine` dependency closure | `core/third-party/utf8proc/LICENSE.md` (MIT and Unicode data terms) | Evidence present; optional path |
| Moonshine TTS/G2P target family | Optional | Configured separately; not a dependency of `moonshine`; TTS option is off | Root `LICENSE` and `docs/license.md` for source scope; model/data terms are separate | Evidence present for source scope; not selected |
| macOS system library/framework closure | Unresolved | No explicit `-framework` fragment in the generated `moonshine` link fragments; no built dylib exists for dependency inspection | Not established | **Unresolved** |

The generated target graph also contains 58 test, 2 benchmark, 29 TTS/G2P, and 4 smoke/batch utility targets not linked by the selected `moonshine` target. These family counts overlap. CMake and AppleClang are host build tools, not app runtime components. No additional system framework was inferred from the CMake graph. Actual transitive macOS dynamic libraries remain unresolved until a separate build and binary inspection.

For vendored entries, the verified revision is the enclosing Moonshine source commit; the individual upstream release revision is not separately stated by this local inventory.

### SHA-256 of inspected license evidence

| Repository-relative path | SHA-256 |
|---|---|
| `LICENSE` | `fa7d1174dd8af6a7cd280be20b80d10095ed4c19b5b20b61a7715c3ad790dc5f` |
| `docs/license.md` | `082542e55e7d8556182dedfbdc7df936792ba8d3ef57821fc68dcd371de4312a` |
| `core/cpp-annote/LICENSE` | `70b76093f6fb38e8d76e5cf0045ce4c3ae12f7f93b81f143ad6b389eaf9e0e16` |
| `core/third-party/Eigen/COPYING.MPL2` | `fab3dd6bdab226f1c08630b1dd917e11fcb4ec5e1e020e2c16f83a0a13863e85` |
| `core/third-party/doctest/LICENSE.txt` | `0fe0b331fa1513dcce8604ff1fa925f32d1cea17d8aeb1c2471fad40d291adc5` |
| `core/third-party/kaldi-native-fbank/LICENSE` | `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30` |
| `core/third-party/kissfft/COPYING` | `a2840585f8411be8e6826a31ef15ae65c950bd74a2437a73b013398a934ad0c6` |
| `core/third-party/nlohmann/LICENSE.MIT` | `86b998c792894ccb911a1cb7994f7a9652894e7a094c0b5e45be2f553f45cf14` |
| `core/third-party/onnxruntime/LICENSE.txt` | `2f07c72751aed99790b8a4869cf2311df85a860b22ded05fa22803587a48922c` |
| `core/third-party/utf-8/LICENSE.txt` | `c9bff75738922193e67fa726fa225535870d2aa1059f91452c411736284ad566` |
| `core/third-party/utf8proc/LICENSE.md` | `3b510150d34f248a221bb88e1d811238d6c6c18b51231822c42974c39bb07256` |

The pinned `docs/license.md` confirms the root MIT boundary and says third-party code under `core/third-party` follows per-project terms. It also describes the default MIT model policy, the enumerated legacy non-English non-streaming model exceptions, and separate TTS/G2P data terms. Those model statements do not identify or approve an Outflow production model.

The ONNX Runtime binary's local SHA-256 was `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`, matching Task 75. Task 75 also verified its 27,623,992-byte arm64 Mach-O dylib architecture. The binary hash does not establish the complete ONNX Runtime third-party notice set.

## Command results

Commands were run from the Outflow repository root. The inventory command reads the pinned checkout and existing Task 76 cache/File API data; it performs only local Git revision/status queries, hashes evidence files, and prints relative source paths.

| Command | Result |
|---|---|
| `python3 scripts/moonshine_native_license_inventory.py --help` | PASS; required `--source-dir` and `--build-dir` options shown |
| `python3 scripts/moonshine_native_license_inventory.py --source-dir ../moonshine-upstream-234f60f --build-dir /private/tmp/outflow-moonshine-task76-cmake-arm64-final` | PASS; exit 0; reports `Native source inventory: VERIFIED`, `REDISTRIBUTION_EVIDENCE_INCOMPLETE`, KISS FFT/ORT notice/system-library gaps, and the verified ORT hash |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_moonshine_native_license_inventory -v` | PASS; 12 tests |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v` | PASS; 299 tests (287 existing baseline plus 12 added) |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; workspace suite and 1 doc test; existing unit/integration baseline 352 |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 136 tests in 10 files |
| `npm run lint` | PASS |
| `npm run build` | PASS |

The focused tests use synthetic source/build fixtures and mocked local Git queries. They cover missing checkout/revision/cache, mismatched source path, discovered and missing license evidence, SHA calculation, runtime versus optional/build-only roles, conservative unresolved handling, stable output, local-only subprocess calls, and unchanged inspected fixture files.

## Remaining review and release prerequisites

- **Native source inventory: VERIFIED** at the configured source/link target level only. Native compilation and linkage were **NOT RUN**.
- **Redistribution approval: NOT YET ESTABLISHED.** The report is `REDISTRIBUTION_EVIDENCE_INCOMPLETE`; resolve the KISS FFT full notice, obtain/review the exact ONNX Runtime third-party notices, inspect the eventual macOS binary dependency closure, reconcile notices with installers, and obtain human/legal review. Review Apache-2.0, MPL-2.0, Boost, MIT, and Unicode obligations for exact shipped content.
- **Model license and manifest: INCOMPLETE.** The earlier four model candidates are not the production bundle. Exact Small Streaming revision, quantized files and SHA-256 values, model-specific license evidence and redistribution approval, and the T-013 runtime manifest remain outstanding. No placeholder manifest was created.
- **ASR model selection: NOT COMPLETE.** No weights were acquired and no inference was run.
- **Hardware resource and WER gates: NOT TESTED.** No 4 GB device, RSS/size, latency, CPU real-time factor, or WER result is claimed.
- T-009, T-010, T-013, and T-022 remain unchecked in `docs/TASKS.md`.

No Moonshine source, Task 76 cache, or generated CMake File API file was modified. No Moonshine compilation, model inference/download, dependency installation/download, or runtime dependency/lockfile change occurred. The existing dirty Outflow worktree was preserved; this task added the inventory tool, its tests, this evidence file, and one README subsection. No Git stage, commit, push, merge, or history rewrite was performed.
