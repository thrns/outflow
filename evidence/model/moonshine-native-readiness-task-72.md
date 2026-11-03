# Task 72 — Offline Moonshine Native Build Readiness

```yaml
task_id: T-072
feature_ids: [F-06]
flows: [UF-05]
acceptance: [AC-06]
components: [C-05, C-09]
stack: [TS-05, TS-09]
owner: Codex coordinator in the existing codex/task23-moonshine-build-scope worktree
base_commit: f4dd938bd7220395b39dcea7538fdcadec14d71d
result_commit: uncommitted
platform: macOS 26.6.2 (25G83), arm64
implementation: scripts/moonshine_native_preflight.py and scripts/tests/test_moonshine_native_preflight.py; README.md section added
review: PENDING
```

## Scope and prior state

This task adds a read-only offline preflight for a future Moonshine CMake configuration. It does not integrate ASR into Tauri, configure or compile Moonshine, change the model selection, or change the centralized task ledger. T-010, T-013, T-022, T-029, and AC-06 remain incomplete.

Existing build infrastructure included the Task 24 source-only CMake probe in `scripts/probe_moonshine_cmake.py` and the Task 25 isolated configure runner in `scripts/try_moonshine_cmake_configure.py`. The readiness command is new because those tools verify source behavior and attempt configuration, respectively; neither gives the requested consolidated local tool/source/header/model prerequisite report. The runner requires the pinned revision, a clean checkout, CMake, and a CMake-compatible source tree.

The previous Task 25 configure attempt is documented in `docs/MOONSHINE_CMAKE_CONFIGURE.md` and `evidence/T-009.md`: it was blocked before CMake configuration because no complete local Moonshine checkout was available. The prior source-only retrieval attempt could not resolve `github.com`. This task made no network request and did not retry source retrieval.

## Implementation

Added `scripts/moonshine_native_preflight.py` using Python's standard library. It reports host support, CMake executable and version, C/C++ compiler availability, local source directory, a C API header identified by the documented version macro and loader declaration, the pinned clean Git revision, model-manifest configuration, model/tokenizer file presence, and overall readiness in stable order.

The expected source revision is read from the existing `SOURCE_REVISION` in `scripts/probe_moonshine_cmake.py`, avoiding a second source pin. Source Git inspection is local and read-only. CMake is invoked only as `cmake --version`; compiler checks use PATH lookup. Git commands are limited to `rev-parse` and `status`. The command has no configure/build/install/download path and applies bounded subprocess timeouts.

The script reuses `docs/model-asset-inventory.json` only as a development inventory. It does not describe that file as a runtime model manifest. Model files are checked only for declared local path presence; the command does not hash or approve them. Model availability does not block native library configuration, but it remains required for later model inference and packaging.

The 16 synthetic tests cover: missing CMake; valid, unreadable, and below-minimum CMake version output; missing compiler; missing source checkout; a Moonshine-named directory without `core/CMakeLists.txt`; source tree without the C API header; a valid source/header fixture; mismatched/unverified revision; missing inventory/runtime manifest; declared model files missing; complete synthetic prerequisites; deterministic output order; allowed local-only subprocess calls; and unchanged source/model files. Fixtures inject PATH and Git answers and use temporary files; they do not establish actual upstream build or model provenance.

## Observed prerequisites and provenance

| Check | Actual result |
|---|---|
| Host | macOS 26.6.2, build 25G83, arm64; detected in the documented project build target family. Platform certification remains unverified. |
| CMake | Executable missing on PATH. Version cannot be determined. |
| C compiler | `cc` found at `/usr/bin/cc`; executable presence only. Compiler compatibility and CMake selection are unverified. |
| C++ compiler | `c++` found at `/usr/bin/c++`; executable presence only. Compiler compatibility and CMake selection are unverified. |
| Moonshine source checkout | No local checkout path is configured or supplied. The repository scan found no `moonshine.h`, `moonshine.hpp`, or Moonshine `CMakeLists.txt`. |
| C API header | Not verified because a source checkout is unavailable. Header version/API compatibility with a native binary is not established. |
| Source revision | Unverified locally. The existing project pin is `234f60faa0eb388b01cdf7e60aca232af37aefda`; no local checkout HEAD was available to compare. |
| Local runtime model manifest | Not configured. T-013 has not created a production runtime manifest. |
| Model/tokenizer assets | Not assessed: both model and tokenizer inventory entries have null `file_path`; all four inventory entries remain `planned`, with null hashes. |

The repository inventory validator reports 4 planned assets, 0 file-verified assets, and 0 redistribution approvals. No model SHA-256 is claimed or inferred. The existing source probe and Task 24 evidence record the project pin and earlier source-file review; they do not establish a currently available local checkout. T-009 and its licensing/redistribution review remain incomplete.

## Commands and results

| Command | Result |
|---|---|
| `python3 scripts/moonshine_native_preflight.py --help` | PASS; options include `--source-dir`, `--asset-inventory`, and `--asset-root`. |
| `python3 scripts/moonshine_native_preflight.py` | BLOCKED, exit 2; CMake missing, source path not configured, header not verified, source revision unverified. Compiler executables found. Model assets reported separately as not assessed. |
| `python3 -m unittest scripts.tests.test_moonshine_native_preflight -v` | PASS; 16 tests. |
| `python3 -m unittest discover -s scripts/tests -v` | PASS; 274 tests total (258 existing baseline + 16 new tests). |
| `python3 -m py_compile scripts/moonshine_native_preflight.py scripts/tests/test_moonshine_native_preflight.py` | PASS. Test-generated `__pycache__` directories were removed. |
| `python3 scripts/validate_model_asset_inventory.py --manifest docs/model-asset-inventory.json` | PASS; 4 planned, 0 file-verified, 0 redistribution approvals; bundled model not verified. |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS; 352 unit/integration tests and 1 doc test. |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS. |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS; this builds only the existing Rust workspace, not Moonshine. |
| `npm run typecheck` | PASS. |
| `npm run test -- --run` | PASS; 136 tests. |
| `npm run lint` | PASS. |
| `npm run build` | PASS. |
| `git diff --check` | PASS. |

No Moonshine CMake configure, native compilation, ASR inference, model hash verification, or package inspection was run.

## Readiness and next step

Current classification: **BLOCKED**. A supported host and C/C++ compiler executables are present. The actionable native configuration blockers are the missing CMake executable/version and the absence of a supplied clean checkout at the existing pinned revision. The C API header cannot be checked until that checkout exists.

After an already available, clean checkout at the pinned revision and a compatible CMake are present, rerun:

```sh
python3 scripts/moonshine_native_preflight.py --source-dir <clean-pinned-checkout>
```

If it reports `READY_TO_ATTEMPT_CONFIGURE`, the next separate configure-only command is:

```sh
python3 scripts/try_moonshine_cmake_configure.py --source-dir <clean-pinned-checkout>
```

That command was not run in Task 72. Model integrity and license clearance, successful compilation/linkage, inference, CPU-only behavior, model quality, latency, installed size, and RAM compliance remain **NOT VERIFIED**.

## Repository state

The worktree was already dirty before this task. Pre-existing changes to `README.md`, settings evidence, and Task 70/71 tests/evidence were preserved. This task added the readiness section to that already modified README and did not change those other files. No Cargo manifest/lockfile, npm manifest/lockfile, model file, upstream source, frontend source, schema, or `docs/TASKS.md` file was changed. No files were staged or committed.
