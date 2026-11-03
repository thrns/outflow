# Third-Party Notice Manifest

The planned notice record is [`third-party-notices.json`](third-party-notices.json). It is a schema-version-1 manifest for the eight direct Moonshine source dependencies recorded in [`moonshine-third-party.json`](moonshine-third-party.json), pinned to upstream commit `234f60faa0eb388b01cdf7e60aca232af37aefda`. The audit inventory remains the source of truth for dependency IDs, component names, license identifiers and evidence URLs, build-use classifications, and the questions already found during source review. The narrative audit is [`MOONSHINE_THIRD_PARTY_AUDIT.md`](MOONSHINE_THIRD_PARTY_AUDIT.md).

## Manifest fields

Each entry copies the audited dependency ID, component name, license expression and build-use classification. A compound expression joins the inventory's verified license identifiers with `AND` in their recorded order. `license_evidence_url` must be the dependency's immutable, commit-specific license-document URL from the inventory. `notice_required_if_shipped` reflects whether the inventory records an attribution or notice requirement; it does not say that the dependency will ship.

`planned_notice_destination` is either `null` or a clean repository-relative path. A non-null path would describe a proposed future destination, not prove that a file exists. The current manifest leaves every destination `null` because no release package layout is verified. `notice_verification_status` is `planned` when the applicable notice obligation is identified but package contents are unverified, or `blocked` when missing or ambiguous licensing material needs resolution. Blocked entries include a status explanation. Every entry and the manifest retain `redistribution_approval: "undetermined"`.

## Determining notice requirements

Requirements come from each dependency's license evidence and attribution/notice summary in the source inventory. The validator compares the manifest's license expression and build-use classification to that inventory rather than maintaining another license rules table. This is a planning record; it does not decide legal interpretation, determine selected build flags, or make all eight audited dependencies mandatory release artifacts.

The existing audit classifies doctest as `test-only`, so it is not automatically a production dependency. nlohmann/json and utf8proc remain `conditional` in the pinned source inventory, but the Prompt 79 development build explicitly set `MOONSHINE_TTS_BUILD_ONNX=ON` and linked the TTS/G2P target into `libmoonshine.dylib`. Those dependencies were therefore in that development build's source/link scope. The build does not establish product installer contents or redistribution approval. `confirmed-build-input` records source-level CMake evidence; the exact development artifact and its limitations are documented in [`prompt-79c-native-license-audit.md`](../evidence/mvp/prompt-79c-native-license-audit.md).

## Validate

From the repository root, run the manifest validator and its unit tests:

```sh
python3 scripts/validate_third_party_notices.py
python3 -m unittest scripts.tests.test_validate_third_party_notices -v
```

The validator uses Python's standard library, reads the local manifest and inventory, makes no network calls, and writes no files. A valid plan currently reports 8 dependencies, 4 planned notices, 4 blocked notices, 0 packaged notices, redistribution approval `UNDETERMINED`, and readiness `INCOMPLETE`.

## Planned versus packaged

A planned notice means only that a notice requirement has been identified and a package destination is not yet verified. A blocked notice cannot be treated as release-ready until its question is resolved. Neither state is a packaged notice, a completed verification, or redistribution approval. The schema accepts only `planned` and `blocked`; it rejects packaged/verified states and approval claims. A future release process must inspect the exact selected source and binaries, collect the complete verified license and third-party notice texts, reconcile each platform installer and destination, and record reproducible artifact evidence before incorporating those files into release artifacts. Legal or licensing review remains a separate required decision.

## Known open questions

- KISS FFT's `COPYING` identifies BSD-3-Clause but references a `LICENSES/BSD-3-Clause` file absent from the pinned checkout. Prompt 79D added the standard text with Mark Borgerding's copyright to [`docs/licenses/KISS-FFT-BSD-3-Clause.txt`](licenses/KISS-FFT-BSD-3-Clause.txt), cross-checked against the [SPDX License List](https://spdx.org/licenses/BSD-3-Clause.html). Its inclusion in any actual source or binary distribution is not yet verified, so the planned notice remains blocked.
- Eigen's exact covered files and applicable MPL-2.0 source and notice obligations need confirmation.
- The Prompt 79 macOS arm64 build linked the local `libonnxruntime.1.23.2.dylib` (SHA-256 recorded in the evidence file). Its filename and matching runtime version do not prove build provenance. The official [ONNX Runtime v1.23.2 ThirdPartyNotices.txt](https://raw.githubusercontent.com/microsoft/onnxruntime/v1.23.2/ThirdPartyNotices.txt) is available, but the exact binary's full notice set has not been reconciled or packaged.
- utf8proc's Unicode-related terms need assessment against the actual selected files, including whether `utf8proc_data.c` is present.
- The development build used TTS/G2P; the future product build remains unselected. The pinned CMake option did not yield an STT-only C API library when turned off.
- doctest is test-only in the audit and must not be treated as a production dependency without package evidence.
- Exact package contents, notice destinations, installer inclusion, and redistribution approval remain undetermined for all entries.

Troubleshooting:

- **License expression or build-use mismatch:** reconcile the entry with `docs/moonshine-third-party.json`; do not change the audit classification based on an unselected build.
- **Evidence URL rejected:** use the immutable `blob/<commit>/...` license-document URL already recorded for that dependency, not a mutable branch or tag URL.
- **Unsafe destination rejected:** use `null` until a repository-relative destination is chosen; absolute paths, drive paths, backslashes, and `..` are not accepted.
- **Blocked notice needs resolution:** update the source evidence and open questions only after the relevant files or terms have been inspected. Do not promote the state by assumption.
- **Manifest passes but readiness is incomplete:** expected. Validation confirms metadata consistency; it does not inspect binaries, installers, or legal permissions.

### Prompt 79D notice update (2026-11-03)

Runtime identity is now checked against the libraries actually loaded by the macOS native helper, and the KISS FFT BSD-3-Clause text is present in the repository. The ONNX Runtime binary still lacks verified source/build provenance, the full transitive notice set has not been matched to that binary, and neither notice has a verified installer destination. The notice manifest therefore remains structurally valid but incomplete, with redistribution approval `undetermined`.
