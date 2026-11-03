# Moonshine CMake Configure Experiment

**Status:** Task 76 configured the pinned upstream `core/` project successfully on macOS arm64. The retained CMake build tree is inspectable; no Moonshine target was compiled or linked. T-009 remains unchecked.

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

The original Task 25 attempt below was blocked before configuration because the pinned checkout and CMake were unavailable. Task 76 reran the experiment on 2026-11-03 with the exact pinned source and the Task 75 ONNX Runtime artifact.

**Task 76 host:** macOS 26.6.2, Darwin 25.6.0, arm64; CMake 4.4.5; Apple Clang 21.0.0.21000101 for both C and C++; `Unix Makefiles` generator; `/usr/bin/make` available.

Pinned source revision `234f60faa0eb388b01cdf7e60aca232af37aefda` and clean checkout were verified before and after configuration. The preflight reported `READY_TO_ATTEMPT_CONFIGURE`. CMake's minimum is 3.22.1. The configure-only runner returned exit code 0 (`CONFIGURED`) and retained its generated build tree at `/private/tmp/outflow-moonshine-task76-cmake-arm64-final`.

The generated cache selects `core/` as `CMAKE_HOME_DIRECTORY`, `Unix Makefiles`, `/usr/bin/clang`, `/usr/bin/clang++`, and `CMAKE_OSX_ARCHITECTURES=arm64`. CMake detected Apple Clang 21.0.0.21000101 for C and C++, and its generated `CMakeSystem.cmake` records both host and target processors as `arm64`. The configured ONNX Runtime path is the pinned local `libonnxruntime.1.23.2.dylib`; its 27,623,992-byte file was reverified as arm64 Mach-O with SHA-256 `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`.

CMake completed configuration and generation without a policy compatibility error. Its only warning was that the two FetchContent cache variables were unused. No Moonshine library or target was built; CMake's compiler identification and compiler ABI probes are allowed configure-time checks and do not verify Moonshine's native ABI linkage. No model asset or dependency download was invoked.

## Configuration requested by the runner

The runner uses `core` as the source root and requires a new, retained out-of-source build directory. Its project definitions are:

```text
-DCMAKE_BUILD_TYPE=Release
-DMOONSHINE_TTS_BUILD_ONNX=OFF
-DMOONSHINE_RELIABILITY=OFF
-DFETCHCONTENT_FULLY_DISCONNECTED=ON
-DFETCHCONTENT_UPDATES_DISCONNECTED=ON
```

For the recorded macOS run the invocation also selected `Unix Makefiles`, `/usr/bin/clang`, `/usr/bin/clang++`, and `CMAKE_OSX_ARCHITECTURES=arm64`. It does not set `MOONSHINE_BUILD_SHARED` or `MOONSHINE_BUILD_SWIFT`. Both FetchContent variables were present in the cache but reported unused by this `core/` project. They are not a general network sandbox. Inspection found no `FetchContent`, `ExternalProject`, `execute_process`, `file(DOWNLOAD)`, curl, or wget acquisition path in the configured `core/` CMake modules. The separate `micro/` tree includes FetchContent for the Pico SDK and was not configured.

## Target graph and remaining scope questions

The earlier Task 25 attempt generated no target graph. Task 76's File API reply identifies `moonshine` as a `SHARED_LIBRARY` and also declares test executables, benchmarks, TTS/G2P targets, and utilities. Target declarations are not compiled contents. The source-level limits in [`MOONSHINE_CMAKE_PROBE.md`](MOONSHINE_CMAKE_PROBE.md) and [`MOONSHINE_BUILD_SCOPE.md`](MOONSHINE_BUILD_SCOPE.md) remain unresolved at the binary level.

The known optional-feature limits remain unresolved at the binary level:

- `MOONSHINE_TTS_BUILD_ONNX=OFF` gates the ONNX TTS branch and link, but not the unconditional TTS subtree, `moonshine_tts_core`, utf8proc, or all TTS/G2P utilities and tests.
- `MOONSHINE_RELIABILITY=OFF` does not disable ordinary tests or benchmarks.
- No global test, benchmark, utility, diarization, or auxiliary-source exclusion was verified. The main source list includes diarization and auxiliary model/voice code.
- The upstream source probe did not identify a verified CPU-only ONNX Runtime provider selection.
- The actual host evaluation of upstream platform conditions could not be observed because configuration did not run.

## Verification stages

`CONFIGURED` means CMake completed configuration and generation. It does not mean the `moonshine` target or any other target was compiled. A build would produce object files; linking would establish the selected binary's actual link composition. Tests would exercise compiled behavior. Release verification additionally requires per-platform assets, notices, signatures, offline installation, model quality, latency, installed size, process memory, and supported-platform checks. None of those later stages is implied by configuration.

The runner never invokes `cmake --build`, `cmake --install`, or `ctest`. It requires a new build directory outside both source trees, refuses to reuse an existing path or cache, and retains that directory after success or failure. Its summary keeps compilation `NOT RUN`, binary contents `NOT VERIFIED`, minimal STT build `NOT PROVEN`, and runtime compatibility `NOT ASSESSED`.

## Reproduce

With a complete, clean checkout at the pinned commit and a compatible CMake already installed:

```sh
python3 scripts/try_moonshine_cmake_configure.py \
  --source-dir "$MOONSHINE_SOURCE_DIR" \
  --build-dir "$MOONSHINE_BUILD_DIR" \
  --generator "Unix Makefiles" \
  --c-compiler /usr/bin/clang \
  --cxx-compiler /usr/bin/clang++ \
  --osx-architectures arm64
```

Exit codes are `0` for `CONFIGURED`, `2` for `BLOCKED` prerequisites or environmental capability, and `1` for `FAILED` integrity or unexpected configuration errors. Inspect `CMakeCache.txt`, `CMakeFiles/<version>/CMakeSystem.cmake`, compiler metadata under `CMakeFiles/<version>/`, and `.cmake/api/v1/reply/`. The runner reports compiler/cache values, target names and types from CMake's File API codemodel, target categories, the selected ONNX Runtime path, and whether the selected file is absent, a recognized binary, or an LFS pointer when configuration succeeds.

The configure timeout is five minutes. A timeout is `BLOCKED`. A missing dependency reported by CMake is also `BLOCKED`; another nonzero configure result is `FAILED`. The output includes bounded diagnostics. A missing or pointer-valued ONNX Runtime library is called out even if CMake itself generated successfully.

Task 76 is configuration evidence only. T-009 remains unchecked, and this experiment does not complete T-010, T-013, T-022, AC-01, or AC-06. The next native step follows the T-009/T-010/T-013 prerequisites and is a separately controlled compile/link and ABI verification for T-022.
