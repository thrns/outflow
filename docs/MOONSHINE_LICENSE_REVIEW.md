# Moonshine Streaming Model Licensing Review

**Review date:** 2026-10-31

**Status:** Preliminary source review; redistribution approval is **undetermined**.

**Scope:** English Small Streaming and English Tiny Streaming speech-to-text model families only.

This is a source-backed engineering record, not legal advice or final approval to package or sell any model, runtime, or application. The companion [machine-readable review record](moonshine-license-review.json) contains claim IDs, source paths, reviewed revisions, evidence statuses, and limitations.

## Scope and review method

The review covered Moonshine AI's license policy, English model catalog entries, repository license, the `core/third-party` source directory listing, and the Moonshine Voice C API reference. Only text documentation and repository directory listings were accessed. No model weights, native binaries, or runtime packages were downloaded, opened, or compiled.

For the repository documents, an immutable source snapshot was resolved and inspected at commit `234f60faa0eb388b01cdf7e60aca232af37aefda`. This identifies the reviewed **documentation/source tree only**. It does not identify a separately distributed or quantized model artifact, a native runtime binary, or any asset checksum. The C API reference uses a mutable Read the Docs `stable` URL, and the third-party directory listing was inspected at mutable `main`; their exact source revisions are unverified.

## Verified upstream statements

### English Small Streaming and Tiny Streaming

The upstream model catalog has separate current entries for **English / Small Streaming / MIT** and **English / Tiny Streaming / MIT**. The license policy says all streaming speech-to-text models are MIT. These two sources support the license statement for the named model families. They do not identify Outflow's exact quantized files, complete file list, artifact-specific license metadata, revision, or SHA-256 values. [Model catalog at reviewed commit](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/docs/models/available-models.md) · [License policy at reviewed commit](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/docs/license.md)

### Streaming versus legacy non-streaming models

The policy distinguishes streaming STT from legacy non-streaming models. It describes the Community License exception as an exhaustive list of legacy **non-English** non-streaming models, and says other STT models are MIT. The catalog separately lists English Base and English Tiny (non-streaming architectures) as MIT. Do not confuse those English legacy architecture names with English Small Streaming or Tiny Streaming; this review's scope is limited to the streaming model families above. [License policy](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/docs/license.md) · [Model catalog](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/docs/models/available-models.md)

### Moonshine-authored source code and third-party source

Moonshine's policy and repository `LICENSE` state that repository code **apart from `core/third-party`** is MIT-licensed. The `core/third-party` policy says the imported code is governed by the originating open-source projects' terms and points to a `LICENSE` file in each subdirectory. The policy also says the compiled Eigen library contains only the MPL-2.0 subset, with other-licensed Eigen files removed. The directory listing inspected during this review showed `Eigen`, `doctest`, `kaldi-native-fbank`, `kissfft`, `nlohmann`, `onnxruntime`, `utf-8`, and `utf8proc` directories. That listing is not a dependency bill of materials and does not prove which sources or binaries a particular platform build includes. [Repository LICENSE at reviewed commit](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/LICENSE) · [License policy at reviewed commit](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/docs/license.md) · [Third-party directory at mutable `main`](https://github.com/moonshine-ai/moonshine/tree/main/core/third-party)

### Copyright and permission notice

Section 1 of the repository `LICENSE` contains the MIT License and names `Copyright (c) 2025 Useful Sensors, Inc. (dba Moonshine AI)`. Its MIT condition requires the copyright notice and permission notice to be included in copies or substantial portions of the licensed Software. Before distribution, preserve the applicable MIT copyright and permission notices with the copied material. This is the standard MIT notice condition stated in that file; it does not replace other suppliers' license and attribution requirements. [Repository LICENSE at reviewed commit](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/LICENSE)

### Native C API documentation

The C API reference describes the portable ABI, header version `30000` (3.0.0), model loader functions, streaming calls, handles, and buffer ownership. It is API documentation, not a license grant for a particular runtime package or binary. The page does not establish the licenses or complete contents of native runtime distributions. Its `stable` documentation URL did not provide a verified immutable revision during this review. [Moonshine Voice C API](https://moonshine-voice.readthedocs.io/en/stable/api/c-api/)

## Licensing boundaries

| Material | What this review establishes | What remains outside the evidence |
|---|---|---|
| Moonshine-authored source code | Upstream says repository code outside `core/third-party` is MIT-licensed. | Whether a selected build includes each source file; other components in the build. |
| English Small Streaming weights | The model family is listed as MIT, and upstream says streaming STT models are MIT. | Exact quantized artifact and file identity, artifact-specific license metadata, revision, hashes, or local integrity. |
| English Tiny Streaming weights | The model family is listed as MIT, and upstream says streaming STT models are MIT. | Exact quantized artifact and file identity, artifact-specific license metadata, revision, hashes, or local integrity. |
| Tokenizer and supporting model data | The catalog documents model formats and architectures. | Exact tokenizer/config/support file set, suppliers, artifact-specific licenses, revisions, and hashes. No separate rights claim is made here. |
| Native runtime and third-party libraries | Upstream says `core/third-party` follows originating project terms and identifies per-subdirectory license files; the reviewed C API page describes an interface. | Complete per-platform runtime inventory, exact binaries, transitive dependency versions, inclusion in Outflow packages, and each supplier's obligations. |
| License and attribution materials | The Moonshine MIT text requires retaining the stated copyright and permission notice for copies or substantial portions of MIT-licensed Software. | Whether Outflow has assembled all applicable notices for the exact model/runtime files and each target installer. |

The repository-wide MIT statement must not be treated as blanket evidence for every generated model file, tokenizer, runtime library, or redistributed binary. The exact shipped artifact set still needs its own provenance and license review.

## Redistribution obligations to check before packaging

1. For each exact Moonshine MIT-licensed item copied or distributed, include the applicable copyright and permission notice required by its MIT terms. Use the upstream `LICENSE` text and do not assume a repository-level file covers unrelated suppliers.
2. Review every included `core/third-party` component against the license file for that subdirectory and the exact version/source actually built. Carry forward that component's required notices, attribution, source or other conditions as applicable.
3. Identify dependencies embedded in each macOS, Windows, and Linux runtime binary. A source directory listing or API reference alone does not show the dependency set in a release binary.
4. Reconcile the complete asset and notice list with actual installer contents. Keep license review, file integrity, package inclusion, and human redistribution approval as separate records.
5. Obtain the project's required final redistribution review/sign-off before claiming that commercial packaging is approved.

The first item describes the MIT condition stated in Moonshine's repository license. Items for other components must be derived from those suppliers' own licenses; no obligations for unreviewed third-party components are inferred here.

PRD §8 calls for a signed and notarized Mac DMG, signed Windows MSI/NSIS, and signed Linux packages/checksums, with an optional opt-in Tauri updater. The asset and notice review must be reconciled against each actual release package before distribution.

## Outstanding questions

- Exact immutable revisions of the quantized English Small Streaming and Tiny Streaming artifacts.
- Complete per-model file list, including tokenizer, configuration, and supporting data.
- Artifact-specific license metadata and provenance linking each distributable file to its source.
- SHA-256 hashes and local integrity verification for every file.
- Complete Moonshine native runtime and transitive dependency inventory for each platform.
- Exact platform-specific binary contents and the licenses/notices for bundled dependencies.
- Inclusion and correctness of Moonshine copyright/permission text and all third-party attribution materials in each installer.
- Final human/legal redistribution review and approval.

Until these points are answered, the review records **redistribution approval: undetermined**. No model or runtime artifact is marked verified, and this review does not complete T-009, T-010, T-013, AC-01, or commercial distribution requirements.

## Provenance and revision record

All repository sources below are from organization/repository **Moonshine AI / `moonshine-ai/moonshine`**. Review date: **2026-10-31**.

| Source path | Immutable revision and reviewed location | Statement/evidence covered |
|---|---|---|
| `docs/license.md` | [`234f60faa0eb388b01cdf7e60aca232af37aefda`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/docs/license.md) | MIT model policy, legacy exceptions, source-code scope, `core/third-party` boundary, and Eigen statement. |
| `docs/models/available-models.md` | [`234f60faa0eb388b01cdf7e60aca232af37aefda`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/docs/models/available-models.md) | English Small Streaming and Tiny Streaming entries; model format/catalog context; English Base/Tiny comparison. |
| `LICENSE` | [`234f60faa0eb388b01cdf7e60aca232af37aefda`](https://github.com/moonshine-ai/moonshine/blob/234f60faa0eb388b01cdf7e60aca232af37aefda/LICENSE) | Repository scope, MIT Section 1 text, copyright, notice condition, and documented model exception policy. |
| `core/third-party` | [Mutable `main` directory reference](https://github.com/moonshine-ai/moonshine/tree/main/core/third-party) | Directory names were inspected; exact directory revision was not verified. No individual supplier license files were reviewed. |
| Moonshine Voice `api/c-api/` | [Read the Docs `stable` page](https://moonshine-voice.readthedocs.io/en/stable/api/c-api/) | C ABI/API behavior and version details; immutable documentation revision not verified; page does not provide runtime license terms. |

The repository commit is provenance for the reviewed upstream documentation only. It does not identify or checksum a separately distributed model artifact or native runtime. See the JSON record for per-claim revision status and limitations.

### Prompt 79C artifact-evidence supplement (2026-11-03)

This source-policy review remains limited to upstream documentation and does not itself establish artifact terms or redistribution rights. Separate Prompt 79C evidence now records eight locally observed Small Streaming file hashes and local macOS development-binary hashes, plus the actual native link/build scope; the local file-integrity validator passes for those eight model files. The model inventory still has zero redistribution approvals, the model catalog path has no immutable artifact revision, the native dependency notice bundle is incomplete, and no installer was inspected. This supplemental artifact evidence does not change this review's family-level license conclusion or complete T-009. See [`prompt-79c-native-license-audit.md`](../evidence/mvp/prompt-79c-native-license-audit.md), [`prompt-79-model-assets.json`](../evidence/mvp/prompt-79-model-assets.json), and [`MODEL_DECISION.md`](../evidence/model/MODEL_DECISION.md).
