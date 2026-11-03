# Task 74 — Acquire and Verify Pinned Moonshine Source

```yaml
task_id: T-074
feature_ids: [F-06]
flows: [UF-05]
acceptance: [AC-06]
components: [C-05, C-09]
stack: [TS-05, TS-09]
owner: Codex coordinator in the existing codex/task23-moonshine-build-scope worktree
base_commit: f4dd938bd7220395b39dcea7538fdcadec14d71d
result_commit: uncommitted
platform: macOS 26.6.2 arm64; not physical 4 GB release evidence
implementation: README.md; scripts/moonshine_native_preflight.py; scripts/tests/test_moonshine_native_preflight.py; evidence/model/moonshine-pinned-source-task-74.md
review: PENDING
```

## Scope and acquisition

Task 74 continued the source-readiness work recorded in Tasks 72 and 73. The Outflow worktree was already dirty at the start of this task. Its existing README, settings evidence/tests, Task 72/73 preflight code and evidence were preserved. The repository task ledger was not edited.

No suitable local Moonshine checkout existed in the project locations inspected, and the Task 72/73 evidence also recorded the source as missing. The user explicitly approved fetching only the public Moonshine Git repository at the pinned commit. No approval was inferred from Task 73's separate Homebrew operation.

The source checkout is outside Outflow at:

```text
/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
```

Acquisition used a new directory and fetched only the approved commit:

```sh
git init /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
git -C /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f remote add origin \
  https://github.com/moonshine-ai/moonshine.git
git -C /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f fetch \
  --depth 1 --no-tags origin 234f60faa0eb388b01cdf7e60aca232af37aefda
git -C /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f \
  checkout --detach FETCH_HEAD
```

Verification results:

| Check | Result |
|---|---|
| `git remote get-url origin` | `https://github.com/moonshine-ai/moonshine.git` |
| `git rev-parse HEAD` | `234f60faa0eb388b01cdf7e60aca232af37aefda` (full SHA, exact match) |
| `git symbolic-ref -q HEAD` | No branch reference; checkout is detached. |
| `git status --porcelain` | Empty; checkout is clean. |
| Submodule entries in commit tree | None. No submodule command was run. |
| Git LFS | Git LFS executable unavailable. LFS-managed paths remain pointer text; no LFS payload was fetched or installed. |

The full Git checkout includes upstream-tracked auxiliary SpellingCNN/TinyVadCNN micro model files under `micro/models/`, a Tiny English tokenizer file in the Python binding assets, and test audio files. These arrived as ordinary Git objects in the approved source checkout; they are not the selected quantized English streaming ASR model bundle. No separate Small Streaming ASR weight bundle was fetched, and no model or audio download helper was run.

## Pinned C API header

The header is `core/moonshine-c-api.h`. Its actual declaration is:

```c
#define MOONSHINE_HEADER_VERSION (30000)
```

The header contains declarations for all requested APIs: `moonshine_load_transcriber_from_files`, `moonshine_create_stream`, `moonshine_start_stream`, `moonshine_transcribe_add_audio_to_stream`, `moonshine_transcribe_stream`, `moonshine_stop_stream`, `moonshine_free_stream`, and `moonshine_free_transcriber`. This establishes that the declarations are present at the pinned source commit; it does not establish ABI linkage with a library.

## Native build entry points and fetch risks

The native C/C++ entry point is `core/CMakeLists.txt` (minimum CMake 3.22.1; C++20). The separate `micro/CMakeLists.txt` entry imports `pico_sdk_import.cmake`, which can use CMake `FetchContent` to retrieve the Raspberry Pi Pico SDK. The native `core/` CMake source does not declare a `FetchContent` or `ExternalProject` download path in the inspected files.

The native core imports ONNX Runtime through `core/third-party/onnxruntime/find-ort-library-path.cmake`. On this macOS arm64 host it selects `core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib`. That path is a Git LFS pointer, not a usable native library. Other platform ORT files are also LFS-managed. The vendored ORT headers are present, but the selected native runtime binary is not verified. This dependency is a remaining native configure/build blocker even though the current preflight does not inspect ORT binary contents.

Inspected fetch-producing commands and how to avoid them in a future offline configure-only task:

| Entry point | Fetch behavior | Offline handling |
|---|---|---|
| `scripts/ort-build-common.sh`, used by `scripts/build-ort-wasm.sh`, `build-ort-android.sh`, and `build-ort-ios.sh` | Clones ONNX Runtime v1.23.2 recursively when absent; otherwise initializes/updates recursive submodules. | Do not run these scripts in an offline configure task. Provide an independently approved, verified local ORT binary if needed. |
| `scripts/fetch-voice-assets.sh` | Retrieves model, TTS/G2P, and test assets from the Moonshine CDN and/or Hugging Face. | Do not invoke; selected streaming model assets remain separate work. |
| `examples/c++/download-library.sh` | Downloads a published native archive, Medium Streaming model files, and sample audio. | Do not invoke. |
| Python binding download helpers | Can retrieve model and other inference assets when called. | Do not execute them. |
| `micro/pico_sdk_import.cmake` | Uses `FetchContent` to obtain the Pico SDK when the micro project is configured. | Avoid the `micro/` entry point; configure only `core/`. |

The existing Outflow configure-only runner sets `FETCHCONTENT_FULLY_DISCONNECTED=ON`, and configures `core/` in a temporary directory. This CMake option is not a general network sandbox. A future configure-only attempt should use that runner only after the required local ORT binary is verified and after reviewing any changed source fetch paths. No Moonshine configure, build, installation, submodule update, or native dependency script ran in Task 74.

## Preliminary license inventory

The upstream root `LICENSE` is MIT for Moonshine-authored source except `core/third-party`. `docs/license.md` says English and streaming STT models default to MIT, while enumerated legacy non-English non-streaming models remain under the Moonshine Community License. TTS/G2P data and models have separate license terms in their own source documentation.

Per-component license files are present under `core/third-party` for Eigen, doctest, kaldi-native-fbank, kissfft, nlohmann/json, ONNX Runtime, UTF8-CPP, and utf8proc. No top-level `NOTICE` or `ThirdPartyNotices` file was found. This is a preliminary source inventory only. It does not establish complete native binary, model artifact, or installer redistribution clearance; T-009 remains incomplete.

## Preflight and regression checks

The first real-source preflight reported the header as missing even though it exists: the upstream macro uses parentheses around `30000`, while the parser accepted only an unparenthesized integer. The preflight parser now accepts both forms, and a synthetic regression test covers the upstream form. No other preflight behavior changed.

Final preflight command and result:

```sh
python3 scripts/moonshine_native_preflight.py \
  --source-dir /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
```

Exit code 0; status `READY_TO_ATTEMPT_CONFIGURE`. It detected CMake 4.4.5, pinned source minimum 3.22.1, C and C++ compiler executables, the header/version 30000, the exact clean source revision, and a supported macOS arm64 host. It separately reported no T-013 runtime model manifest and two model/tokenizer inventory entries without local file paths. The readiness result does not check the LFS-backed ORT library and is not proof that CMake can configure successfully.

| Command | Result |
|---|---|
| `python3 -m unittest scripts.tests.test_moonshine_native_preflight -v` | PASS; 17 tests, including the parenthesized header-version regression; output captured in `/private/tmp/outflow-task74-focused-python.log`. |
| `python3 -m unittest discover -s scripts/tests -v` | PASS; 275 tests; output captured in `/private/tmp/outflow-task74-python-suite.log`. |
| `RUSTUP_TOOLCHAIN=stable CARGO_NET_OFFLINE=true cargo fmt --all -- --check` | PASS. |
| `RUSTUP_TOOLCHAIN=stable CARGO_NET_OFFLINE=true cargo check --workspace --locked` | PASS. |
| `RUSTUP_TOOLCHAIN=stable CARGO_NET_OFFLINE=true cargo test --workspace --locked` | PASS; 352 unit/integration tests plus one doc test. |
| `RUSTUP_TOOLCHAIN=stable CARGO_NET_OFFLINE=true cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS. |
| `RUSTUP_TOOLCHAIN=stable CARGO_NET_OFFLINE=true cargo build --workspace --locked` | PASS. |
| `npm_config_offline=true npm run typecheck` | PASS. |
| `npm_config_offline=true npm run test -- --run` | PASS; 136 tests in 10 files. |
| `npm_config_offline=true npm run lint` | PASS. |
| `npm_config_offline=true npm run build` | PASS. |
| `git diff --check` | PASS after documentation and evidence updates. |

These are Outflow regressions only. No Moonshine CMake configuration or native compilation was run.

## Remaining blockers and handoff

- The selected Small Streaming model files, tokenizer, exact hashes, and T-013 runtime manifest remain unavailable.
- The pinned checkout's macOS arm64 ONNX Runtime library is an LFS pointer; Git LFS was not available or invoked.
- C ABI linkage, native compilation, model compatibility, runtime inference, WER, latency, CPU-only behavior, installed size, and peak RSS remain unverified.
- Redistribution clearance remains incomplete under T-009; T-010, T-013, and T-022 remain incomplete.
- No centralized task completion markers were changed.

The next safe configure-only step is a separate task: first obtain approval for any required ORT runtime asset, verify its exact provenance and native file contents, then use the existing Outflow configure-only runner with the pinned source. Do not run upstream asset/build scripts as part of that step without separate approval.
