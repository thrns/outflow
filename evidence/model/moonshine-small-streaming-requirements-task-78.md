# Prompt 78 — Moonshine Small Streaming Model Source Requirements

> **Identifier correction:** “78” is the research prompt number, not canonical task `T-078`. Canonical `T-078` is the signed installer clean-machine beta. This source-requirements research is related to `T-009`; it does not complete or change `T-078`.

```yaml
prompt_id: Prompt 78
task_id: null
related_task_ids: [T-009]
feature_ids: [F-06]
flows: [UF-05]
acceptance: [AC-06]
components: [C-05, C-09]
stack: [TS-05, TS-09]
owner: Codex coordinator in the existing codex/task23-moonshine-build-scope worktree
base_commit: f4dd938bd7220395b39dcea7538fdcadec14d71d
result_commit: uncommitted
platform: macOS 26.6.2 arm64; Apple M3 Pro, 18 GB host; not physical 4 GB release evidence
implementation: scripts/moonshine_model_source_audit.py; scripts/tests/test_moonshine_model_source_audit.py; README.md; this evidence
metrics: Model bytes, installed size, process RSS, latency, CPU real-time factor, WER, and quality gates NOT_MEASURED
doc_updates: README.md Prompt 78 subsection; this evidence; docs/TASKS.md unchanged
risks: Exact artifact revision and packaged SHA-256 are unavailable; no model files are present; redistribution evidence is incomplete
review: PENDING
```

## Scope and result

Prompt 78 adds a standard-library-only, offline audit of the model-selection and dependency-manifest contract in the already pinned Moonshine source. It does not invoke the C API, compile Moonshine, call model-download helpers, access a network, modify the source checkout, obtain model files, create a production manifest, or implement inference.

The source checkout identity was verified locally:

| Check | Observation |
|---|---|
| Origin | `https://github.com/moonshine-ai/moonshine.git` |
| `git rev-parse HEAD` | `234f60faa0eb388b01cdf7e60aca232af37aefda` |
| `git status --short` | Empty; clean checkout |
| C API header | `core/moonshine-c-api.h`, `MOONSHINE_HEADER_VERSION (30000)` |

The exact source contract is established: **English Small Streaming uses `MOONSHINE_MODEL_ARCH_SMALL_STREAMING` = `4`**. The pinned source catalog and generated registry establish the eight default required file names and vendor expected size/CRC32C metadata. No weights are in the checkout, and the source manifest provides no artifact revision or SHA-256. Thus source requirements are verified while the model artifacts remain unverified.

## Source trace and line evidence

All paths in this section are relative to the pinned Moonshine checkout at the commit above.

| Finding | Pinned evidence |
|---|---|
| Header version and architecture identifiers | `core/moonshine-c-api.h:95,106-114`: header `30000`; `TINY=0`, `BASE=1`, `TINY_STREAMING=2`, `BASE_STREAMING=3`, `SMALL_STREAMING=4`, `MEDIUM_STREAMING=5`. No non-streaming `SMALL` identifier is defined. |
| Loader architecture contract | `core/moonshine-c-api.h:471-480` declares `moonshine_load_transcriber_from_files(..., uint32_t model_arch, ...)`; `core/moonshine-c-api.cpp:261-283` copies the parameter to `TranscriberOptions.model_arch`. The loader takes an explicit architecture value. |
| English catalog selection and default | `core/moonshine-model-catalog.cpp:100-103,149-165` lists Medium Streaming first and Small Streaming next for `en`; `:358-394` uses the requested `model_arch`, or `lang->models.front()` when omitted. Therefore English dependency lookup without `model_arch` defaults to Medium Streaming (`5`); language selection alone does not select Small. Tiny Streaming (`2`) remains only a possible later fallback. |
| Streaming support and Small Streaming URL | `core/moonshine-model-catalog.cpp:93-98` recognizes `MOONSHINE_MODEL_ARCH_SMALL_STREAMING` as streaming; `:149-165` registers it for English under `small-streaming-en`. |
| Quantization selection | `core/moonshine-model-catalog.cpp:33-45,155-157` selects the dated catalog path `/quantized_26_08_21`. This is path/catalog selection, not a separate STT variant option. The dated path label is not an artifact hash or independent revision proof. |
| Required and optional files | `core/moonshine-model-catalog.cpp:288-314` returns eight streaming files by default. It adds `decoder_kv_with_attention.ort` only for English when `word_timestamps` is enabled. The spelling catalog at `:241-250` adds `spelling_cnn.ort` and `spelling_cnn_meta.json` as a separate optional group when spelling is requested. No separate timestamp model file is listed; the attention decoder supports word timestamps. |
| Dependency selection and local computation | `core/moonshine-c-api.h:1074-1119` documents language, options, manifest shape, and `moonshine_get_stt_dependencies`. `core/moonshine-c-api.cpp:2441-2508` parses `model_arch`, spelling, and word-timestamp options, then calls `moonshine::stt_model_dependencies`; it does not fetch a remote manifest. `core/moonshine-model-catalog.cpp:15-29,358-394` builds groups and adds local metadata from `find_model_file_metadata`. |
| Source-bundled metadata registry | `core/moonshine-model-file-metadata.h:9-25` describes size/checksum lookup; `core/moonshine-model-file-metadata.generated.cpp:1-9,158-184` contains generated URL-keyed expected sizes and base64 CRC32C values for Small Streaming English. The comment says the registry is generated from CDN objects and baked into the source so runtime lookup needs no network. |
| Returned JSON schema and buffer release | `core/moonshine-c-api.cpp:1791-1838` serializes `groups`, `base_url`, optional `role`, and file `name`, `url`, `size`, `checksum`, and `checksum_type`. `core/moonshine-c-api.h:319-333` and `core/moonshine-c-api.cpp:254-259` document/provide `moonshine_free_buffer` for the library-allocated buffer. No model revision or architecture identifier is emitted in the returned dependency manifest. |

For the default Small Streaming dependency lookup, the eight required source-catalog files are:

| Relative artifact name | Source expected size (bytes) | Vendor checksum type/value |
|---|---:|---|
| `adapter.ort` | 2,870,368 | CRC32C `XlWjTg==` |
| `cross_kv.ort` | 5,356,536 | CRC32C `5ySK8g==` |
| `decoder_kv.ort` | 81,878,600 | CRC32C `Rn/VUA==` |
| `encoder.ort` | 44,148,576 | CRC32C `41D2jQ==` |
| `frontend.model.ort` | 26,944 | CRC32C `WW2/2g==` |
| `frontend.weights.ort` | 7,769,464 | CRC32C `9pROuQ==` |
| `streaming_config.json` | 512 | CRC32C `dPbFiw==` |
| `tokenizer.bin` | 249,974 | CRC32C `B7s10Q==` |

The optional word-timestamp file has source expected size `81,766,608` bytes and vendor CRC32C `IzJXVA==`. Optional spelling files are a separate group and are not part of the default eight-file model list.

The registry's expected `size` values are available and tied to the pinned source snapshot. They do not prove the size of a local artifact because no model artifact was acquired. The registry supplies CRC32C, not Outflow's packaged SHA-256. After a separately approved artifact acquisition, Outflow must independently hash the exact acquired bytes with SHA-256 and retain artifact-specific provenance. No SHA-256 was inferred from CRC32C.

## Licensing and redistribution evidence

| Area | Evidence and limit |
|---|---|
| Moonshine native source | Upstream `LICENSE` and `docs/license.md:3` identify Moonshine-authored source as MIT except `core/third-party`. This is source licensing evidence, not a blanket license for every model/runtime asset. |
| English Small Streaming weights | Pinned `docs/license.md:5,7-19` says streaming STT models and all English models are MIT by default and enumerates the non-MIT legacy non-English non-streaming exceptions. Pinned `docs/models/available-models.md:14-18` specifically lists English Small Streaming as MIT. This is model-family/source-document evidence; it does not identify an acquired artifact revision or establish final redistribution clearance. |
| Native third-party runtime | Task 77 evidence at [`moonshine-native-license-inventory-task-77.md`](moonshine-native-license-inventory-task-77.md) records distinct third-party terms. KISS FFT's `COPYING` refers to a missing `LICENSES/BSD-3-Clause` text; ONNX Runtime has a local MIT license file but no local `ThirdPartyNotices.txt`/`NOTICE` evidence. Notice reconciliation, package inclusion, and human/legal review remain outstanding. |
| Download-host terms | No artifact-specific host terms were found in the pinned license/model documents inspected for this task. Their status is unverified. |

The audit reports model license evidence as **Present** and runtime notices as **Incomplete**. Neither result authorizes use or redistribution. **T-009 remains incomplete.**

## Audit output

Commands were run from the Outflow repository root. The help command exited `0`. The real audit exited `0`, meaning the local audit completed; acquisition readiness is still explicitly **Blocked**.

```text
Moonshine model-source audit (local source only)
Pinned source revision: 234f60faa0eb388b01cdf7e60aca232af37aefda
Origin: https://github.com/moonshine-ai/moonshine.git
SOURCE_CONTRACT_VERIFIED: YES
MODEL_ARTIFACTS_VERIFIED: NO

Check | Result
--- | ---
Pinned source revision | Verified
C API header | Verified
Small Streaming architecture | Verified
Model loader contract | Verified
Dependency manifest discovery | Traced
Exact model artifact list | Verified
Artifact revision | Not verified
Artifact byte sizes | Not verified (source expected sizes present; model files absent)
Artifact SHA-256 values | Not verified (source registry supplies CRC32C, not SHA-256)
Model license evidence | Present
Runtime third-party notices | Incomplete
Model acquisition readiness | Blocked

C API header version: 30000
Selected architecture: MOONSHINE_MODEL_ARCH_SMALL_STREAMING = 4
Related architecture identifiers: TINY=0; BASE=1; TINY_STREAMING=2; BASE_STREAMING=3; SMALL_STREAMING=4; SMALL (non-streaming)=not defined
English default when model_arch is omitted: MEDIUM_STREAMING (5)
Loader contract: explicit uint32_t model_arch argument is copied to TranscriberOptions.model_arch.
Quantized source path: /quantized_26_08_21
Quantization selection: catalog URL path, not a separate STT variant option.
Model base URL in the local catalog: https://download.moonshine.ai/model/small-streaming-en/quantized_26_08_21/
Required source-catalog files:
- adapter.ort | source expected bytes: 2870368 | vendor checksum: crc32c:XlWjTg==
- cross_kv.ort | source expected bytes: 5356536 | vendor checksum: crc32c:5ySK8g==
- decoder_kv.ort | source expected bytes: 81878600 | vendor checksum: crc32c:Rn/VUA==
- encoder.ort | source expected bytes: 44148576 | vendor checksum: crc32c:41D2jQ==
- frontend.model.ort | source expected bytes: 26944 | vendor checksum: crc32c:WW2/2g==
- frontend.weights.ort | source expected bytes: 7769464 | vendor checksum: crc32c:9pROuQ==
- streaming_config.json | source expected bytes: 512 | vendor checksum: crc32c:dPbFiw==
- tokenizer.bin | source expected bytes: 249974 | vendor checksum: crc32c:B7s10Q==
Optional source-catalog files:
- word_timestamps: decoder_kv_with_attention.ort | source expected bytes: 81766608 | vendor checksum: crc32c:IzJXVA==
- include_spelling: spelling_cnn.ort, spelling_cnn_meta.json

Manifest discovery: source-bundled catalog implementation and generated metadata registry; no remote manifest read.
Manifest metadata fields: groups; group base_url and optional role; file name, URL, expected byte size (null if unknown), checksum, checksum_type.
Selection options: model_arch; include_spelling/spelling/spelling_model_path; word_timestamps.
Model architecture and artifact revision are not emitted as returned manifest fields.
Model revision field: absent from the dependency manifest; the dated catalog directory is not artifact verification.
Buffer release: moonshine_free_buffer (library-owned malloc/free boundary).

Reason codes:
- ARTIFACT_REVISION_UNKNOWN
- LOCAL_MODEL_FILES_ABSENT
- PACKAGED_SHA256_REQUIRED
- RUNTIME_THIRD_PARTY_NOTICES_INCOMPLETE
Exit code: 0
```

## Verification

The twelve new tests use synthetic source text and mocked local Git metadata only. They do not treat their synthetic metadata as authentic upstream artifacts. Coverage includes missing source/header/architecture/dependency implementation, revision mismatch, recognized architecture and local catalog, incomplete metadata, CRC32C versus SHA-256, missing model-specific license evidence, stable output ordering, and no network access or source-tree writes.

| Command | Result |
|---|---|
| `python3 scripts/moonshine_model_source_audit.py --help` | PASS, exit 0 |
| `python3 scripts/moonshine_model_source_audit.py --source-dir /Users/bixxie/Desktop/Projects/moonshine-upstream-234f60f` | PASS, audit exit 0; readiness Blocked |
| `python3 -m unittest scripts.tests.test_moonshine_model_source_audit -v` | PASS, 12 tests |
| `python3 -m unittest discover -s scripts/tests -v` | PASS, 311 tests (299 existing + 12 new) |
| `RUSTUP_TOOLCHAIN=stable cargo fmt --all -- --check` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo check --workspace --locked` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` | PASS, existing 352 unit/integration tests plus 1 doc test |
| `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `RUSTUP_TOOLCHAIN=stable cargo build --workspace --locked` | PASS |
| `npm run typecheck` | PASS |
| `npm run test -- --run` | PASS, 136 tests in 10 files |
| `npm run lint` | PASS |
| `npm run build` | PASS |

The Rust workspace commands validate Outflow only. They do not configure or compile Moonshine.

## State, remaining prerequisite, and handoff

```text
Native source: VERIFIED
Native configure: PASSED (Task 76 evidence; not rerun by Prompt 78)
Native compilation: NOT RUN
Model architecture: VERIFIED — MOONSHINE_MODEL_ARCH_SMALL_STREAMING (4)
Model artifact list: VERIFIED from pinned source catalog
Model revision: UNVERIFIED
Model SHA-256 values: UNVERIFIED
Model weights locally available: NO
Production model manifest: NOT CREATED
ASR inference: NOT RUN
Hardware and WER gates: NOT TESTED
Redistribution clearance: INCOMPLETE
```

The next prerequisite is artifact-specific provenance/revision and distribution review, together with resolution of the KISS FFT and ONNX Runtime notice gaps. Only after that review and a separate authorization should a controlled acquisition of the catalog-listed files be considered. That later acquisition must verify each byte length and vendor CRC32C from this pinned registry, compute a separate SHA-256 for every exact file, and retain immutable provenance before any production manifest is written. Prompt 78 did not perform that retrieval.

T-009, T-010, T-013, T-022, and AC-06 remain incomplete. No task-ledger completion markers were changed. The README was already modified before Prompt 78; its prior content was preserved and a Prompt 78 subsection was added. No dependency, Cargo/npm lockfile, model file, Moonshine binary, or upstream source change was introduced. No files were staged or committed.
