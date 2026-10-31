# Model Asset Inventory

## Purpose

[`model-asset-inventory.json`](model-asset-inventory.json) is a development-time inventory for the planned ASR model, supporting model assets, native runtime libraries, and license/notice materials. It supports C-05/C-09 and TS-09 by recording source provenance and checking local hashes for assets explicitly marked `file-verified`.

The inventory is not a downloader, package manager, generic SBOM generator, runtime model manifest, installer-content proof, or legal opinion. A structurally valid inventory is not an approved distribution bill of materials. The project still needs per-file source, license, notice, and installer redistribution review before T-009 or AC-01 can be completed.

## Schema version 1

The root object contains `schema_version` and `assets`. Every asset row contains the fields below; unknown information is represented by JSON `null`, not a guessed value or placeholder string.

| Field | Meaning |
|---|---|
| `asset_id` | Stable, unique identifier for this planned item or file. |
| `category` | One of `model`, `tokenizer`, `native-runtime`, or `license-notice`. |
| `status` | `planned` or `file-verified`; the latter means local file and provenance checks pass. |
| `source` | Project or supplier name. For transitive runtime items, identify each supplier in its own eventual row. |
| `source_url` | HTTP(S) source page when known, otherwise `null`. The validator never opens it. |
| `source_revision` | Exact source/model revision when verified, otherwise `null`. Avoid moving branch names such as `main` for verified rows. |
| `file_path` | Canonical slash-separated path relative to `--asset-root`, otherwise `null`. |
| `sha256` | SHA-256 of the exact local file bytes, otherwise `null`. The validator recomputes it incrementally. |
| `provenance_reference` | Durable reference to evidence identifying the source and revision; required for `file-verified`. |
| `license_identifier` | License identifier only when reviewed for this individual asset; otherwise `null`. A repository-level license must not be copied onto model weights or transitive libraries without per-asset evidence. |
| `license_evidence_reference` | Reference to the license text or recorded human review. Required when `license_review_status` is `reviewed`. |
| `license_review_status` | `not-reviewed`, `review-pending`, or `reviewed`. This records a human review state; the validator does not determine legal meaning. |
| `redistribution_status` | `undetermined`, `approved`, or `blocked`. Approval is separate from the license name and requires an evidence reference plus a reviewed license state. |
| `redistribution_evidence_reference` | Reference to the recorded redistribution decision; required for `approved` and `blocked`. |
| `notice_attribution_status` | `undetermined`, `required`, `not-required`, or `prepared`. A definite state requires supporting evidence. |
| `notice_attribution_evidence_reference` | Reference to the notice/attribution requirement or prepared notice material. |

The initial manifest uses four **planning categories**, not an assertion that these are the final or complete artifact rows:

1. Moonshine Small Streaming candidate (`model`).
2. Tokenizer or other supporting model assets (`tokenizer`).
3. Moonshine native runtime and transitive libraries (`native-runtime`).
4. License and third-party notice materials (`license-notice`).

All four are `planned`; their exact revisions, local paths, hashes, and licenses remain `null`. In particular, the repository's general Moonshine license statements do not establish the license for the exact model files or any native transitive dependency.

### Planned entry example

This example shows the explicit unknown values used before an artifact is acquired and inspected:

```json
{
  "asset_id": "example-model-candidate",
  "category": "model",
  "status": "planned",
  "source": "Supplier name",
  "source_url": null,
  "source_revision": null,
  "file_path": null,
  "sha256": null,
  "provenance_reference": null,
  "license_identifier": null,
  "license_evidence_reference": null,
  "license_review_status": "not-reviewed",
  "redistribution_status": "undetermined",
  "redistribution_evidence_reference": null,
  "notice_attribution_status": "undetermined",
  "notice_attribution_evidence_reference": null
}
```

## Integrity and provenance requirements

To mark an asset `file-verified`, record its exact non-placeholder source revision, repository-relative file path, SHA-256, and a provenance evidence reference. Run the validator with the directory that contains those paths as `--asset-root`. It requires a regular local file, resolves symlinks only when their target remains inside that root, and hashes file data in 1 MiB chunks. It does not alter files or make network requests.

`file-verified` means the local file matched the declared digest and has revision/provenance metadata. It does not mean the file is bundled in any installer, the list is complete, or redistribution is allowed. Actual packaging must independently show every required file is present and the release's installed-byte checks include it.

## Licensing and notices

Review each model file, tokenizer/config/support file, native library, transitive library, and notice as its own asset. Record evidence for license text and source/version, and a separate documented human decision for commercial redistribution. Include copied required attribution and third-party notice files in the package inventory. A name such as `MIT` never changes `redistribution_status`; approval requires an explicit recorded state and evidence reference. The validator checks for that metadata but does not validate the cited evidence or make a legal determination.

Moonshine Streaming is the selected project candidate described by the PRD and TECH_STACK. The exact quantized Small Streaming files, file list, and revision are not yet verified. The project documents distinguish Streaming weights from certain legacy model families. Native runtime and transitive licenses remain independent and require supplier-by-supplier review.

## Adding verified assets

1. Keep an item `planned` and use `null` for facts not yet established.
2. Obtain an artifact only through the separately authorized build/research workflow; this validator does not download assets.
3. Record the exact supplier, immutable source revision, and evidence reference that ties the local file to that revision.
4. Add one row per concrete file or notice, use a unique `asset_id`, and record the local relative path and expected SHA-256.
5. Run the validator with the correct local asset root. Do not label an item `file-verified` until checksum and provenance validation pass.
6. Record license review, redistribution decision, and notice/attribution state separately with evidence. These are human decisions, not checksum results.
7. Independently reconcile the inventory with the complete transitive dependency tree, copied notices, installer contents, and the product's distribution terms before preparing the real license BOM.

## Remaining work before T-009 can close

- Pin the actual Moonshine source commit and exact quantized Small Streaming artifact revision and complete file list.
- Verify each local model/tokenizer/support file and its checksum against source provenance.
- Identify and pin the native runtime build and every transitive native library.
- Review per-asset license texts and commercial redistribution rights, including legacy-model exclusions where relevant.
- Prepare and verify required third-party notices and attributions.
- Reconcile the reviewed inventory against each installer and obtain the required human/legal sign-off.

Until that work is complete, the inventory is a planning aid and T-009 remains incomplete.
