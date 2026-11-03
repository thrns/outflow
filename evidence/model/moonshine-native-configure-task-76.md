# Task 76 — First Reproducible Moonshine Native CMake Configuration

```yaml
task_id: Task 76 (external configure sequence; distinct from backlog T-076 license review)
feature_ids: [F-06]
flows: [UF-05]
acceptance: [AC-06]
components: [C-05, C-09]
stack: [TS-05, TS-09]
owner: Codex coordinator in codex/task23-moonshine-build-scope
base_commit: f4dd938bd7220395b39dcea7538fdcadec14d71
result_commit: uncommitted
platform: macOS 26.6.2 (25G83), arm64; not reference 4 GB hardware
implementation: scripts/try_moonshine_cmake_configure.py; scripts/tests/test_try_moonshine_cmake_configure.py; README.md; docs/MOONSHINE_CMAKE_CONFIGURE.md; docs/MOONSHINE_BUILD_SCOPE.md; this evidence
metrics: NOT_MEASURED
doc_updates: Configure workflow and result documented; docs/TASKS.md completion markers unchanged
risks: Model selection/bundle, license signoff, native build/link, ABI, WER, latency, RSS, installed size and release qualification remain open
review: PENDING
```

## Result

```text
Outflow Moonshine native configuration

Pinned source: VERIFIED
CMake toolchain: VERIFIED
ONNX Runtime ARM64: VERIFIED
Offline configuration: PASS (inspected core path; no acquisition command invoked)
CMake cache: GENERATED
Upstream source cleanliness: PRESERVED

Moonshine compilation: NOT RUN
Native ABI linkage: NOT VERIFIED
Model bundle: UNAVAILABLE
Runtime model manifest: MISSING
ASR inference: NOT RUN
Hardware resource and WER gates: NOT TESTED
```

The offline result applies to the inspected `core/` configuration path and this configure attempt. The two FetchContent settings were passed but CMake reported both unused by this project. These settings do not provide a general network sandbox.

## Source and dependency verification

Source checkout: `/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f`.

| Check | Result |
|---|---|
| `git -C "$MOONSHINE_SOURCE_DIR" rev-parse HEAD` | `234f60faa0eb388b01cdf7e60aca232af37aefda`; exact pin |
| `git -C "$MOONSHINE_SOURCE_DIR" status --short` before configure | Empty; clean |
| `git -C "$MOONSHINE_SOURCE_DIR" status --short` after configure | Empty; clean |
| `python3 scripts/moonshine_native_preflight.py --source-dir "$MOONSHINE_SOURCE_DIR"` | `READY_TO_ATTEMPT_CONFIGURE` |
| CMake minimum in `core/CMakeLists.txt` | 3.22.1 |

The native C API CMake source directory is `core/`. `core/CMakeLists.txt` sets C++20, enables testing unconditionally, and adds `ort-utils`, `bin-tokenizer`, and `moonshine-tts`. `MOONSHINE_TTS_BUILD_ONNX=OFF` disables the ONNX TTS branch, and `MOONSHINE_RELIABILITY=OFF` disables reliability instrumentation; neither option removes every test, benchmark, TTS/G2P, or utility target. No general test/example/auxiliary-target disable switch was found in the inspected CMake files.

The macOS arm64 CMake path in `core/third-party/onnxruntime/find-ort-library-path.cmake` selects:

```text
core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib
```

`core/third-party/onnxruntime/CMakeLists.txt` declares the native library as an imported shared target. The verified local file was checked again before configuration:

| Check | Result |
|---|---|
| `file "$ORT_LIBRARY"` | Mach-O 64-bit dynamically linked shared library, arm64 |
| `stat -f '%z bytes' "$ORT_LIBRARY"` | `27623992 bytes` |
| `shasum -a 256 "$ORT_LIBRARY"` | `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4` |
| `lipo -info "$ORT_LIBRARY"` | Non-fat Mach-O containing `arm64` |

The complete SHA-256 matches Task 75's pinned Git LFS object. The checkout's LFS object store still contains only that one verified ORT object. No additional native dependency or model asset was fetched during Task 76.

## Configure-time acquisition and target controls inspected

The configured `core/` CMake entry and its included modules contain no `FetchContent`, `ExternalProject`, `execute_process`, `file(DOWNLOAD)`, curl, or wget acquisition path. `find_package(Threads REQUIRED)` resolved to the host toolchain; Python3 is queried optionally for an offline test. The separate `micro/` CMake entry contains a Pico SDK `FetchContent` path and was not used. No submodule, LFS, model-asset, dependency-installation, or upstream download helper ran.

The runner passed both `FETCHCONTENT_FULLY_DISCONNECTED=ON` and `FETCHCONTENT_UPDATES_DISCONNECTED=ON`. CMake 4.4.5 recorded each value as `ON` and warned that both command-line variables were unused by this `core/` project. The inspected source path had no configure-time retrieval command, but the flags alone are not a network sandbox.

## Toolchain and exact configure attempt

Host: macOS 26.6.2 (Darwin 25.6.0), arm64. CMake was 4.4.5. Apple Clang was 21.0.0.21000101 for C and C++; `/usr/bin/make` was available. The generator selected was `Unix Makefiles`.

The runner now requires a new `--build-dir`, retains it on success or failure, rejects any existing path/cache, and rejects paths inside or enclosing the Moonshine or Outflow source trees. Its explicit toolchain options support generator, C/C++ compiler, and macOS architecture selection. The focused regression tests cover missing/invalid build directory, pre-existing cache preservation, source/revision failures, invalid or LFS-pointer ONNX paths, configure failure propagation, offline settings, and absence of build/download commands.

Exact final command:

```sh
python3 scripts/try_moonshine_cmake_configure.py \
  --source-dir /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f \
  --build-dir /private/tmp/outflow-moonshine-task76-cmake-arm64-final \
  --generator 'Unix Makefiles' \
  --c-compiler /usr/bin/clang \
  --cxx-compiler /usr/bin/clang++ \
  --osx-architectures arm64
```

The runner's CMake definitions were:

```text
-DCMAKE_BUILD_TYPE=Release
-DMOONSHINE_TTS_BUILD_ONNX=OFF
-DMOONSHINE_RELIABILITY=OFF
-DFETCHCONTENT_FULLY_DISCONNECTED=ON
-DFETCHCONTENT_UPDATES_DISCONNECTED=ON
```

The runner exited `0`. Its final status was `CONFIGURED`; source integrity and source cleanliness both reported `VERIFIED`. There was no CMake 4.4.5 policy-compatibility error.

Relevant CMake output:

```text
-- The C compiler identification is AppleClang 21.0.0.21000101
-- The CXX compiler identification is AppleClang 21.0.0.21000101
-- Detecting C compiler ABI info - done
-- Detecting CXX compiler ABI info - done
-- Found Threads: TRUE
-- Configuring done (1.7s)
-- Generating done (0.2s)
-- Build files have been written to: /private/tmp/outflow-moonshine-task76-cmake-arm64-final
CMake Warning (unused-cli):
  Manually-specified variables were not used by the project:
    FETCHCONTENT_FULLY_DISCONNECTED
    FETCHCONTENT_UPDATES_DISCONNECTED
```

## Cache and generated artifacts

Retained build directory: `/private/tmp/outflow-moonshine-task76-cmake-arm64-final`.

The generated `CMakeCache.txt` records:

```text
CMAKE_HOME_DIRECTORY=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core
CMAKE_GENERATOR=Unix Makefiles
CMAKE_BUILD_TYPE=Release
CMAKE_C_COMPILER=/usr/bin/clang
CMAKE_CXX_COMPILER=/usr/bin/clang++
CMAKE_OSX_ARCHITECTURES=arm64
FETCHCONTENT_FULLY_DISCONNECTED=ON
FETCHCONTENT_UPDATES_DISCONNECTED=ON
MOONSHINE_TTS_BUILD_ONNX=OFF
MOONSHINE_RELIABILITY=OFF
ONNXRUNTIME_LIB_PATH=/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib
```

`CMakeFiles/4.4.5/CMakeSystem.cmake` records host and target processors as `arm64`. The C and C++ compiler metadata files identify AppleClang 21.0.0.21000101. The File API codemodel identifies `moonshine` as `SHARED_LIBRARY`; it also contains test executables, benchmark targets, TTS/G2P targets, and utilities. CMake generated the root `Makefile`, `CMakeCache.txt`, `CMakeFiles/`, per-subdirectory makefiles and CTest listings, and `.cmake/api/v1/reply/` JSON files.

No Moonshine library artifact or target object was generated. The compiler identification and compiler ABI probes (including small identification executables) are part of CMake configuration and are not Moonshine compilation or Moonshine ABI linkage. The generated build directory is not proof that any declared target can compile or link.

An initial configure run before the File API target-type reader was corrected also succeeded and remains retained at `/private/tmp/outflow-moonshine-task76-cmake-arm64`. The final run above used a fresh directory and verified the runner's corrected target-type output. Neither directory is inside the Moonshine or Outflow checkout.

## Automated verification

| Command | Result |
|---|---|
| `python3 -m unittest scripts.tests.test_try_moonshine_cmake_configure -v` | PASS; 23 tests |
| `python3 -m unittest scripts.tests.test_moonshine_native_preflight -v` | PASS; 25 tests |
| `python3 -m unittest discover -s scripts/tests -v` | PASS; 287 tests |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; existing suite baseline is 352 unit/integration tests plus 1 doc test |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 136 tests across 10 files |
| `npm run lint` | PASS |
| `npm run build` | PASS; typecheck and Vite production build |

The Rust and frontend commands verify the Outflow workspace only. They did not compile Moonshine. No Cargo or npm lockfile changes were introduced.

## Handoff and remaining gates

Task 76 completes only the configure-only attempt. T-009 license/redistribution review, T-010 model feasibility and benchmark decision, T-013 exact model bundle/manifest, and T-022 native compilation/linkage and ABI verification remain incomplete. No completion marker in `docs/TASKS.md` was changed.

Native configure: **PASS**. Native compilation: **NOT RUN**. ABI linkage: **NOT VERIFIED**. Model bundle: **UNAVAILABLE**. ASR inference: **NOT RUN**. Hardware resource and WER gates: **NOT TESTED**. No CPU-only inference, recognition quality, latency, installed-size, RSS, or release-readiness claim is made.

Next step: complete the T-009/T-010/T-013 prerequisites, then run a separately scoped native compile/link and ABI inspection for T-022. Keep CMake's generated targets and the current model gaps in view; successful configuration alone does not validate an Outflow-minimal binary.
