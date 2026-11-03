# Task 75 — Acquire and Verify the Pinned Moonshine ONNX Runtime Binary

```yaml
task_id: T-075
feature_ids: [F-06]
flows: [UF-05]
acceptance: [AC-06]
components: [C-05, C-09]
stack: [TS-05, TS-09]
owner: Codex coordinator in codex/task23-moonshine-build-scope
base_commit: f4dd938bd7220395b39dcea7538fdcadec14d71
result_commit: uncommitted
platform: macOS 26.6.2 arm64; not physical 4 GB release evidence
implementation: README.md; scripts/moonshine_native_preflight.py; scripts/tests/test_moonshine_native_preflight.py; this evidence file
review: PENDING
```

## Pinned artifact identity

The source checkout is `/Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f`, remote `https://github.com/moonshine-ai/moonshine.git`, pinned commit `234f60faa0eb388b01cdf7e60aca232af37aefda`.

Inspection of `core/third-party/onnxruntime/find-ort-library-path.cmake` shows that the native Apple arm64 branch selects:

```text
core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib
```

`core/third-party/onnxruntime/CMakeLists.txt` declares the native macOS ORT target as `SHARED IMPORTED` and points it at `ONNXRUNTIME_LIB_PATH`. This native C API configuration selects this one ONNX Runtime dylib plus the existing headers under `core/third-party/onnxruntime/include`; it does not select an additional ORT binary. The macOS arm64 `.a` path is selected for the separate Swift build, not this native CMake path.

The working file initially contained a 133-byte ASCII Git LFS pointer. Its tracked `HEAD` representation and `.gitattributes` identify the same LFS object:

| Field | Value |
|---|---|
| Repository-relative path | `core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib` |
| LFS object SHA-256 | `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4` |
| Declared size | `27623992` bytes |
| Source commit | `234f60faa0eb388b01cdf7e60aca232af37aefda` |
| LFS attribute | `filter=lfs` |

The digest above identifies the ONNX Runtime binary. It is not a Moonshine speech-model checksum.

## Approval and targeted acquisition

Git LFS was initially unavailable (`git: 'lfs' is not a git command`). The user explicitly approved both `brew install git-lfs` and network retrieval of only the path and LFS object above. No ASR model weights were included in that approval or fetched.

Git LFS 3.8.1 was installed with `brew install git-lfs`. Its documented options were inspected before use. Git LFS was initialized locally with:

```sh
git -C /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f lfs install --local --skip-repo
```

This registers LFS filters only in the Moonshine checkout's `.git/config`; it does not alter global Git configuration or install hooks.

An initial dry run combining the exact include path with `--exclude='*'` returned zero transfers; the exclusion pattern also excludes the requested path. The actual approved request used an exact include without that conflicting exclusion:

```sh
git -C /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f lfs fetch \
  --include=core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib \
  origin 234f60faa0eb388b01cdf7e60aca232af37aefda
git -C /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f lfs checkout \
  core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib
```

The include filter limits retrieval to the one approved path. A sandboxed dry run could not resolve `github.com`; the already approved exact include-only retrieval was then run with network access. The targeted checkout reported one object checked out (28 MB). The LFS object store contains exactly one file, with the expected object ID. Git LFS did not print an independent network-byte counter; the materialized file and stored object have the exact metadata-declared size of 27,623,992 bytes.

## Integrity and source verification

Commands and results:

| Command | Result |
|---|---|
| `file /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib` | `Mach-O 64-bit dynamically linked shared library arm64` |
| `stat -f '%z bytes' /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib` | `27623992 bytes`; exact LFS size match |
| `shasum -a 256 /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib` | `480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4`; exact full digest match |
| `lipo -info /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib` | Non-fat file; architecture `arm64` |
| `git -C /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f rev-parse HEAD` | `234f60faa0eb388b01cdf7e60aca232af37aefda`; unchanged |
| `git -C /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f status --short` | Empty; clean under the installed LFS filters |
| `find /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f/.git/lfs/objects -type f -print` | Exactly the one object with SHA-256 `480790a9…17364cf4` |

No source revision, native source, model asset, or other LFS object was changed or fetched. The preflight confirms its Git revision check and LFS pointer metadata independently.

## Preflight behavior

Task 74's preflight output was `READY_TO_ATTEMPT_CONFIGURE` while the required file was still a Git LFS pointer; that result did not inspect the runtime binary. Task 75 adds an offline check that reads the pointer from `git show HEAD:<path>`, then classifies the local file as missing, `LFS_POINTER_NOT_MATERIALIZED`, `PRESENT_UNVERIFIED`, or `VERIFIED`. Size, SHA-256, and 64-bit Mach-O dylib architecture must all match before verification succeeds. The check does not download or replace files. Hosts outside macOS arm64 remain conservatively unverified by this task's platform-specific artifact check.

Command:

```sh
python3 scripts/moonshine_native_preflight.py \
  --source-dir /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f
```

Result: exit 0; `ONNX Runtime native library: VERIFIED`; overall `READY_TO_ATTEMPT_CONFIGURE`. CMake 4.4.5 meets the pinned source minimum 3.22.1, the C/C++ compiler executables are present, the pinned source and C API header checks pass. The preflight still reports `Local model manifest: NOT CONFIGURED` and `Model artifacts: NOT ASSESSED`; model availability is a separate T-013 prerequisite.

`READY_TO_ATTEMPT_CONFIGURE` means only that a separate configure attempt may be made. No Moonshine CMake configure, native compilation, C ABI linkage, or inference was run or claimed.

## Automated verification

| Command | Result |
|---|---|
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_moonshine_native_preflight -v` | PASS; 25 focused tests, including missing artifact, LFS pointer, size/checksum mismatch, synthetic ARM64 verification, conservative architecture handling, no download/replacement, and existing source/header checks |
| `PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts/tests -v` | PASS; 283 tests. Full output: `/private/tmp/outflow-task75-python-suite.log` |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS; 136 tests in 10 files |
| `npm run lint` | PASS |
| `npm run build` | PASS |
| `git diff --check` | PASS |

These Rust and frontend checks validate the Outflow workspace only; none configured or compiled Moonshine.

## Scope and remaining blockers

Updated `README.md`, the offline preflight, its focused tests, and this evidence file. `docs/TASKS.md` was not changed. The existing Outflow uncommitted work was preserved; Cargo and npm lockfiles were not modified. Final status remained on `codex/task23-moonshine-build-scope` with no staged or committed changes. The separate Moonshine checkout reported `## HEAD (no branch)` and remained clean at the pinned commit.

T-009, T-010, T-013, and T-022 remain incomplete. Speech-model selection and exact model files/checksums, runtime manifest, full redistribution review, 4 GB CPU-only measurements, quality/latency evidence, native C ABI linkage, and inference remain open. No release readiness is claimed.
