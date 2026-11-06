# AC-07 Corpus Manifest

**Status:** Metadata contract for tracking corpus identity and review declarations. This document does not create or approve acceptance data, approve the Clean/Verbatim policy, or establish AC-07 acceptance.

The version 1 JSON manifest contains **metadata only**. Never put input transcripts, expected outputs, actual outputs, or full protected-token text in a manifest. Case bytes, when supplied later, must live in a separately governed data bundle.

## Schema version 1

The top-level object must contain exactly these fields:

| Field | Type and rule |
|---|---|
| `schema_version` | JSON integer exactly `1` (a boolean is not an integer for this contract). |
| `corpus_id` | Nonempty string. |
| `revision` | Nonempty string. |
| `purpose` | `synthetic_demo` or `acceptance_candidate`. |
| `review_status` | `pending` or `reviewed`. This is a declaration, not authenticated approval. |
| `review_evidence_ref` | Must be `null` when `review_status` is `pending`; must be a nonempty reference string when it is `reviewed`. |
| `cases` | Nonempty array of case metadata objects. |

Every case object must contain exactly these fields:

| Field | Type and rule |
|---|---|
| `id` | Nonempty string, unique within the manifest. IDs are compared exactly and are case-sensitive. |
| `category` | One of `whitespace`, `commands`, `vocabulary`, `punctuation`, `fillers`, `modes`, `unicode`, or `safety`, matching the existing cleanup fixture categories. |
| `mode` | `clean` or `verbatim`. |
| `remove_fillers` | Strict JSON boolean. |
| `commands_enabled` | Strict JSON boolean. |
| `input_sha256` | Exactly 64 lowercase hexadecimal characters. |
| `expected_sha256` | Exactly 64 lowercase hexadecimal characters. |
| `provenance_ref` | Nonempty reference describing the independently authored source. |
| `reference_review_status` | `pending` or `reviewed`. |
| `protected_kinds` | Array of unique values drawn from `number` and `proper_noun`; an empty array is valid. |

Objects reject missing and unknown fields. JSON files also reject duplicate object keys. Nonempty strings must contain at least one non-whitespace character.

## Review declarations and hash limits

A `synthetic_demo` manifest must have `review_status: "pending"`, a null `review_evidence_ref`, and `reference_review_status: "pending"` on every case. The validator rejects any synthetic demo that claims corpus or reference review.

An `acceptance_candidate` may declare `pending` or `reviewed` status. A reviewed declaration requires a nonempty `review_evidence_ref`; that reference only points to separate evidence. **The validator cannot authenticate review evidence, determine whether the named review occurred, or approve a candidate.** It does not provide a formal approval or acceptance flag.

The SHA-256 fields are intended to bind future case data to fixed input and expected-reference bytes. Digests operate on exact byte sequences; the synthetic test strings are UTF-8 encoded. A future case-data bundle must specify and verify the exact bytes being hashed. The checked-in demonstration contains only digests and synthetic provenance references. Until a separate case-data bundle is supplied, the validator verifies hash syntax only; it cannot establish that a digest corresponds to any case bytes or independently approved reference. Hash syntax validation is not a content-integrity check.

The manifest records planned categories, modes, settings, and declared protected kinds. It does not score exact outputs or count altered protected tokens; those remain the responsibility of the existing exact-output and protected-token evaluators.

## Validator and synthetic draft

From the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_manifest.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_manifest -v
```

The demo validates only [`scripts/testdata/synthetic_ac07_manifest.json`](../scripts/testdata/synthetic_ac07_manifest.json), which has three synthetic metadata records, three categories, both modes, one numeric-protection declaration, and one proper-noun-protection declaration. Test-only synthetic source strings verify its SHA-256 values. No case transcript or expected output is included in the manifest.

`validate_cleanup_manifest(data)` returns a frozen summary with identity, declared review state, indexed and unique case counts, category and mode counts, declared protection-kind counts, pending reference-review count, and a `syntactically_valid` result. Invalid manifests raise `CleanupManifestValidationError`; messages identify the field or case position and do not print case content.

The optional `--manifest PATH` form validates a caller-supplied metadata-only file. It never treats a declared review status as approval and always reports `AC-07 acceptance: NOT EVALUATED`.
