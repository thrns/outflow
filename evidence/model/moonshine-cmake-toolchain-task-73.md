# Task 73 — CMake Toolchain Verification

```yaml
task_id: T-073
feature_ids: [F-06]
flows: [UF-05]
acceptance: [AC-06]
components: [C-05, C-09]
stack: [TS-05, TS-09]
owner: Codex coordinator in codex/task23-moonshine-build-scope
base_commit: f4dd938bd7220395b39dcea7538fdcadec14d71d
result_commit: uncommitted
platform: macOS, Darwin, arm64
implementation: README.md Moonshine Native Build Readiness section; this evidence record
review: PENDING
```

## Scope and installation record

This task verifies a local CMake/CTest toolchain with an isolated C/C++ smoke fixture. It does not acquire Moonshine source, configure or compile Moonshine, fetch model weights, or implement speech recognition. T-010, T-013, T-022, and AC-06 remain incomplete.

At initial inspection, `command -v cmake` and `cmake --version` found no executable; the common Homebrew, system, and CMake.app locations checked were also absent. Homebrew 7.0.6 and Apple Clang/Clang++ 21.0.0 were available. The user explicitly approved `brew install cmake`. The first invocation failed before installation because `/opt/homebrew/Cellar` was not writable. The user subsequently reported CMake installed. The successful installation action was not observed; no second installation was attempted. Verification then found CMake at `/opt/homebrew/bin/cmake`, version 4.4.5, and CTest at `/opt/homebrew/bin/ctest`, version 4.4.5.

An earlier read-only `brew list --versions cmake` query attempted to refresh Homebrew API metadata and failed because the Homebrew API cache path was not writable. It did not establish installation state. The approved Homebrew formula remains the documented installation option; the successful installation invocation is unverified.

The CMake version is recorded only as installed and usable for this smoke fixture. Compatibility with the pinned Moonshine source minimum remains unverified because no source checkout was supplied.

## Toolchain and smoke test

Host: `uname -s` returned `Darwin`; `uname -m` returned `arm64`. `command -v clang` and `command -v clang++` returned `/usr/bin/clang` and `/usr/bin/clang++`; `xcrun --find` resolved both to `/Library/Developer/CommandLineTools/usr/bin`. Apple Clang and Clang++ reported `Apple clang version 21.0.0 (clang-2100.1.1.101)`.

The required disposable fixture was created at `/var/folders/4k/wvp_db_57gs6r0c2v4l_fg4h0000gn/T/tmp.Rt2BIG4atg`. Before removal, its canonical path was verified as a child of the system temporary directory, and `CMakeLists.txt`, `main.c`, and the generated `build` directory were present. Cleanup of that exact fixture returned exit 0.

| Command/check | Result |
|---|---|
| `cmake -S "$SMOKE_DIR" -B "$SMOKE_DIR/build"` | PASS, exit 0; C and C++ compiler identification and ABI checks succeeded with AppleClang 21.0.0.21000101. |
| `cmake --build "$SMOKE_DIR/build"` | PASS, exit 0; `outflow_cmake_smoke` compiled and linked. |
| `ctest --test-dir "$SMOKE_DIR/build" --output-on-failure` | PASS, exit 0; `1/1` test passed. |
| Fixture-path/content check and `rm -rf "$SMOKE_DIR"` | PASS; checked under the system temp directory before removal; cleanup exit 0. |

The smoke test verifies a local minimal project only. It says nothing about Moonshine's source configuration, native dependency graph, ABI, model integration, or runtime behavior.

## Moonshine readiness after CMake verification

| Check | Result |
|---|---|
| CMake | AVAILABLE: `/opt/homebrew/bin/cmake`, 4.4.5. |
| CTest | AVAILABLE: `/opt/homebrew/bin/ctest`, 4.4.5. |
| C/C++ compilers | AVAILABLE: `cc` at `/usr/bin/cc`, `c++` at `/usr/bin/c++`; the smoke configure selected AppleClang for both. |
| Moonshine checkout | MISSING; no local source directory was supplied. |
| C API header | NOT VERIFIED because source is absent. |
| Source revision | UNVERIFIED; expected pin `234f60faa0eb388b01cdf7e60aca232af37aefda` was not compared with a checkout. |
| Runtime model manifest | NOT CONFIGURED; T-013 has not supplied one. |
| Model/tokenizer files | NOT ASSESSED; the development inventory has two model/tokenizer entries without local file paths. |
| Overall preflight | BLOCKED, exit 2, due to missing/unverified source prerequisites. |

Exact diagnostic commands:

- `python3 scripts/moonshine_native_preflight.py --help` — PASS, exit 0.
- `python3 scripts/moonshine_native_preflight.py` — BLOCKED, exit 2; CMake now passes, while source, header, and revision remain missing or unverified. Model readiness is reported separately.

The actual readiness report included these results:

```text
CMake executable: AVAILABLE
  Found on PATH: /opt/homebrew/bin/cmake
CMake version: DETECTED
  Detected: CMake 4.4.5 at /opt/homebrew/bin/cmake.
  Pinned source minimum cannot be assessed until the exact clean checkout is verified.
C/C++ compiler: AVAILABLE
  C compiler: cc (/usr/bin/cc).
  C++ compiler: c++ (/usr/bin/c++).
Moonshine source directory: MISSING
Moonshine C API header: NOT VERIFIED
Source revision: UNVERIFIED
  No source checkout is available to inspect.
Local model manifest: NOT CONFIGURED
Model artifacts: NOT ASSESSED
Offline configuration readiness: BLOCKED
  One or more prerequisites needed for native configuration are missing or unverifiable.
preflight_exit=2
```

The preflight did not configure Moonshine. No Moonshine source or model was downloaded.

## Regression checks

| Command | Result |
|---|---|
| `python3 -m unittest scripts.tests.test_moonshine_native_preflight -v` | PASS; 16 tests. |
| `python3 -m unittest discover -s scripts/tests -v` | PASS; 274 tests. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; workspace suites passed, including 352 unit/integration tests and 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS; builds the existing Rust workspace, not Moonshine. |
| `npm run typecheck` | PASS. |
| `npm run test -- --run` | PASS; 136 tests. |
| `npm run lint` | PASS. |
| `npm run build` | PASS. |
| `git diff --check` | PASS after documentation updates. |

## Repository and remaining limits

Only `README.md` and this evidence file were edited by Task 73. No Cargo/npm manifests or lockfiles, production source, Moonshine source, model assets, or `docs/TASKS.md` were changed. No user settings, audio, or transcripts were accessed. Existing uncommitted changes were preserved; nothing was staged or committed.

CMake availability does not complete the Moonshine native build task. A later task still needs an existing clean checkout at the expected revision, verification of the actual C API header/revision, and an authorized native configure/build workflow. T-010, T-013, and T-022 remain unchecked. No hardware, ASR quality, memory, installed-size, or release metrics were measured.
