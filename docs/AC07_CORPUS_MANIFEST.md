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

The SHA-256 fields are intended to bind case data to fixed input and expected-reference bytes. Digests operate on exact byte sequences; the synthetic test strings are UTF-8 encoded. The metadata-only manifest validator checks hash syntax only; it cannot establish that a digest corresponds to case bytes or an independently approved reference. The separate development-only binding validator now checks the supplied synthetic bundle's content against the declared hashes. Hash syntax validation alone is not a content-integrity check.

## Separate reference-bundle format and binding

The development-only reference bundle is a separate JSON file with exactly
these top-level fields: `schema_version` (integer `1`), `corpus_id`,
`revision`, and a nonempty `cases` array. Each case has exactly `id`, `input`,
and `expected`. Corpus ID, revision, and case IDs must be nonempty strings;
input and expected must be strings, but may be empty. Unknown or missing
fields, unsupported versions, duplicate object keys, duplicate case IDs,
invalid UTF-8 strings, and malformed collections are rejected.

```json
{
  "schema_version": 1,
  "corpus_id": "ac07-synthetic-manifest-demo",
  "revision": "draft-001",
  "cases": [
    {
      "id": "demo-001",
      "input": "exact input string",
      "expected": "exact expected string"
    }
  ]
}
```

The bundle's corpus ID and revision must exactly equal the manifest values.
Case identity is matched by exact, case-sensitive ID, independent of array
position. Every manifest ID must occur once in the bundle, with no additional
IDs. For each matched case the binding validator hashes the exact UTF-8
encoding of the decoded `input` and `expected` strings with SHA-256 and
compares the results to that case's `input_sha256` and `expected_sha256` in
the manifest. It does not normalize Unicode, alter line endings, trim
whitespace, or repair hashes.

The metadata validator's digest-format check establishes only that a digest
looks like 64 lowercase hexadecimal characters. The separate binding check
establishes that the supplied strings hash to the digests already declared by
the manifest. Matching hashes prove correspondence to that declared manifest;
they do **not** prove expected outputs are correct, independently reviewed, or
approved. The checked-in three-case bundle is synthetic test data and cannot
be used as AC-07 acceptance data.

The manifest records planned categories, modes, settings, and declared protected kinds. It does not score exact outputs or count altered protected tokens; those remain the responsibility of the existing exact-output and protected-token evaluators.

## Separate protected-token annotation bundle

Protected-token annotations live in a third, separate JSON artifact. Its
version 1 schema has exactly `schema_version`, `corpus_id`, `revision`, and
`cases`. Each case has exactly `id` and `protected_tokens`; each token has
exactly `kind`, `text`, `input_bytes`, and `output_bytes`. The annotation
bundle's corpus ID, revision, and exact case-ID set must match the manifest and
the hash-verified reference bundle. Cases may be reordered. Token kinds are
`number` or `proper_noun`; the set of kinds on each case must equal that case's
manifest `protected_kinds`. Repeated nonoverlapping tokens of one kind are
allowed.

[`scripts/validate_cleanup_ac07_protected_references.py`](../scripts/validate_cleanup_ac07_protected_references.py)
first runs the existing manifest/reference binding validator, then checks the
separate annotation bundle. It reuses the protected-token scorer to validate
that each half-open UTF-8 byte range selects the declared token in the exact
reference input and expected strings, with no overlap or split character.
During this reference-only check the scorer receives `actual=expected`; its
alteration counts and acceptance-gate result are discarded. This is not a
production cleanup result or protected-token acceptance evaluation.

The manifest's `protected_kinds` are declarations only. Matching those kinds
with annotations does not authenticate independent review of the token
selections. Matching reference hashes proves correspondence to the manifest's
declared bytes, not reference correctness or approval. Valid annotation ranges
prove only that those annotations point to the declared tokens in the supplied
references; they do not prove that production cleanup preserves those tokens.
Only later actual-output scoring can measure preservation, and that scoring
still does not establish AC-07 acceptance by itself.

The synthetic annotation for case `demo-003` treats the literal `AC-07`
identifier as a `proper_noun` solely to demonstrate the contract. It is not
evidence of natural-language proper-noun coverage. The numeric `001` in
`demo-001` demonstrates input and expected byte ranges shifted by different
prefix lengths. All three artifacts are synthetic and are not acceptance data.

Run the synthetic protected-reference demonstration and focused tests from
the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_protected_references.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_protected_references -v
```

The summary prints only identity and counts after all three artifacts bind. It
reports reference review as pending, corpus approval as not established, and
AC-07 acceptance as not evaluated; it does not print annotation or transcript
text.

## Separate actual-output bundle and identity binding

Actual outputs are supplied in a fourth, separate JSON artifact. Its version 1
schema contains exactly `schema_version`, `corpus_id`, `revision`, `run_id`,
`source_kind`, `execution_evidence_ref`, and a nonempty `cases` array. Each
case contains exactly `id` and `actual`. The output corpus ID, revision, and
case IDs must match the identity already verified across the manifest,
hash-bound reference bundle, and protected-token annotation bundle. Cases are
matched by exact, case-sensitive ID rather than array position, so a reordered
output bundle is valid when the complete ID set matches.

The `source_kind` declaration is `synthetic_demo` or `production_declared`.
Synthetic outputs require `execution_evidence_ref: null`; a production
declaration requires a nonempty string. A `synthetic_demo` manifest rejects a
`production_declared` source. An `acceptance_candidate` manifest may accept
either declaration. An execution-evidence reference is only a declaration and
is not proof that production code ran; output identity validation never
independently verifies production provenance.

[`scripts/validate_cleanup_ac07_actual_outputs.py`](../scripts/validate_cleanup_ac07_actual_outputs.py)
first calls the protected-reference validator, then validates output identity,
strict fields, source declarations, complete case-ID correspondence, and
UTF-8 encodability. Empty actual strings are retained as valid outputs. The
binder does not normalize or alter actual strings, compare them to expected
strings, inspect protected-token preservation, or generate outputs from
expected values. The checked-in three-case output file is independently
authored synthetic test data; it contains no input, expected-output, or
protected-token text.

The four-artifact relationship is:

**Manifest → hash-bound reference bundle → validated protected annotations → separately supplied actual outputs.**

Run the synthetic demonstration and focused tests from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_actual_outputs.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_actual_outputs -v
```

The summary is printed only after every binding succeeds. It contains identity
and counts only, and explicitly says `Production execution: NOT VERIFIED` and
`AC-07 acceptance: NOT EVALUATED`. The existing shared coordinator may later
score verified references against corresponding actual outputs, but this
validator does not invoke it. Output identity and a source declaration do not
establish cleanup correctness.

Formal AC-07 evaluation still requires an approved acceptance corpus, approved
Clean/Verbatim semantics, authenticated production execution, and integration
evidence. The authoritative case-count, exact-output, and protected-token
thresholds remain in the PRD and are unchanged.

## Validator and synthetic draft

From the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_manifest.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_manifest -v
```

The demo validates only [`scripts/testdata/synthetic_ac07_manifest.json`](../scripts/testdata/synthetic_ac07_manifest.json), which has three synthetic metadata records, three categories, both modes, one numeric-protection declaration, and one proper-noun-protection declaration. Test-only synthetic source strings verify its SHA-256 values. No case transcript or expected output is included in the manifest.

`validate_cleanup_manifest(data)` returns a frozen summary with identity, declared review state, indexed and unique case counts, category and mode counts, declared protection-kind counts, pending reference-review count, and a `syntactically_valid` result. Invalid manifests raise `CleanupManifestValidationError`; messages identify the field or case position and do not print case content.

The optional `--manifest PATH` form validates a caller-supplied metadata-only file. It never treats a declared review status as approval and always reports `AC-07 acceptance: NOT EVALUATED`.

The separate binding demonstration reads only the checked-in synthetic
manifest and bundle:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/validate_cleanup_ac07_reference_bundle.py --demo
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest scripts.tests.test_validate_cleanup_ac07_reference_bundle -v
```

The binding summary prints verified counts only after every ID and digest
matches. It reports reference review as pending, corpus approval as not
established, and AC-07 acceptance as not evaluated. No input or expected
content is printed.
