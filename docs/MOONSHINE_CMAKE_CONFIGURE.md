# Moonshine CMake Configure Experiment

**Status:** Runner implemented and tested; the real upstream configure attempt is **BLOCKED** because a complete source checkout and CMake are unavailable in this environment. No Moonshine target was built. T-009 remains unchecked.

## Pinned source and preparation

The only accepted source revision is `234f60faa0eb388b01cdf7e60aca232af37aefda`. The runner checks `git rev-parse HEAD`, requires a clean checkout, and reuses [`probe_moonshine_cmake.py`](../scripts/probe_moonshine_cmake.py) to verify the six Task 24 CMake source files before configuration.

Prepare a temporary source-only checkout with Git LFS smudging disabled:

```sh
SOURCE_PARENT="$(mktemp -d)"
export GIT_LFS_SKIP_SMUDGE=1
git clone --filter=blob:none --no-checkout \
  https://github.com/moonshine-ai/moonshine.git \
  "$SOURCE_PARENT/moonshine"
git -C "$SOURCE_PARENT/moonshine" checkout --detach \
  234f60faa0eb388b01cdf7e60aca232af37aefda
git -C "$SOURCE_PARENT/moonshine" rev-parse HEAD
python3 scripts/probe_moonshine_cmake.py \
  --source-dir "$SOURCE_PARENT/moonshine"
```

Do not run `git lfs pull`, `git lfs fetch`, or `git submodule update`. A Git LFS pointer is metadata and is not a native library or model asset.

## Host and observed result

Run date: 2026-10-31. Host: macOS 26.6.2, Darwin 25.6.0, arm64. Apple Clang 21.0.0.101 is available as both the C and C++ driver. CMake is not installed (`cmake --version` reports command not found); Git LFS is also unavailable.

The source-only clone was attempted with `GIT_LFS_SKIP_SMUDGE=1`, `--filter=blob:none`, and `--no-checkout`. Git exited 128 with `Could not resolve host: github.com`. The failed clone did not leave a complete checkout. A broader local search found only Transformers package subdirectories named `moonshine`, not an upstream repository. Therefore the source revision and six-file integrity could not be reverified for this experiment.

The runner was invoked with the attempted checkout path and returned `BLOCKED` (exit code 2) because that source directory was unavailable. The existing Task 24 source probe also returned an unavailable-source error. Since source verification stopped before the CMake prerequisite check, no upstream minimum CMake version was read and no configure command was run. CMake version, configured target graph, selected C/C++ compiler cache entries, actual option values, and ONNX Runtime path are **not available**. No `CONFIGURED` or target-definition claim is made.

The host compiler is present, but that does not substitute for CMake's compiler selection and configure checks. The ONNX Runtime library was not selected or inspected. Its presence, binary format, and any Git LFS pointer state remain unverified. No model weights or native-library binaries were fetched.

## Configuration requested by the runner

When the pinned source, its dependencies, and a suitable CMake are available, the runner uses `core` as the source root, a temporary out-of-source build directory, and these exact project definitions:

```text
-DCMAKE_BUILD_TYPE=Release
-DMOONSHINE_TTS_BUILD_ONNX=OFF
-DMOONSHINE_RELIABILITY=OFF
-DFETCHCONTENT_FULLY_DISCONNECTED=ON
```

It does not set `MOONSHINE_BUILD_SHARED` or `MOONSHINE_BUILD_SWIFT`. The host's native architecture is used. `FETCHCONTENT_FULLY_DISCONNECTED=ON` only controls CMake FetchContent behavior; it does not prove the full configure is network-isolated. Network isolation was not available for verification here.

## Target graph and remaining scope questions

No configure-generated target graph exists for this run. The source-level findings previously recorded in [`MOONSHINE_CMAKE_PROBE.md`](MOONSHINE_CMAKE_PROBE.md) and [`MOONSHINE_BUILD_SCOPE.md`](MOONSHINE_BUILD_SCOPE.md) remain source evidence only. They do not establish configured targets or compiled contents.

The known optional-feature limits remain unresolved at the binary level:

- `MOONSHINE_TTS_BUILD_ONNX=OFF` gates the ONNX TTS branch and link, but not the unconditional TTS subtree, `moonshine_tts_core`, utf8proc, or all TTS/G2P utilities and tests.
- `MOONSHINE_RELIABILITY=OFF` does not disable ordinary tests or benchmarks.
- No global test, benchmark, utility, diarization, or auxiliary-source exclusion was verified. The main source list includes diarization and auxiliary model/voice code.
- The upstream source probe did not identify a verified CPU-only ONNX Runtime provider selection.
- The actual host evaluation of upstream platform conditions could not be observed because configuration did not run.

## Verification stages

`CONFIGURED` means CMake completed configuration and generation. It does not mean the `moonshine` target or any other target was compiled. A build would produce object files; linking would establish the selected binary's actual link composition. Tests would exercise compiled behavior. Release verification additionally requires per-platform assets, notices, signatures, offline installation, model quality, latency, installed size, process memory, and supported-platform checks. None of those later stages is implied by configuration.

The runner never invokes `cmake --build`, `cmake --install`, or `ctest`. It removes its temporary build directory after inspection. Its summary always keeps compilation `NOT RUN`, binary contents `NOT VERIFIED`, minimal STT build `NOT PROVEN`, and runtime compatibility `NOT ASSESSED`.

## Reproduce

With a complete, clean checkout at the pinned commit and a compatible CMake already installed:

```sh
python3 scripts/try_moonshine_cmake_configure.py \
  --source-dir "$SOURCE_PARENT/moonshine"
```

Exit codes are `0` for `CONFIGURED`, `2` for `BLOCKED` prerequisites or environmental capability, and `1` for `FAILED` integrity or unexpected configuration errors. The runner reports compiler/cache values, target names from CMake's generated File API codemodel, target categories, the selected ONNX Runtime path, and whether the selected file is absent, a recognized binary, or an LFS pointer when configuration succeeds.

The configure timeout is five minutes. A timeout is `BLOCKED`. A missing dependency reported by CMake is also `BLOCKED`; another nonzero configure result is `FAILED`. The output includes bounded diagnostics. A missing or pointer-valued ONNX Runtime library is called out even if CMake itself generated successfully.

T-009 remains unchecked. This experiment does not complete T-010, T-013, AC-01, or AC-06.
