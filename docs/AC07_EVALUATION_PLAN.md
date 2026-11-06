# AC-07 Evaluation Plan

**Status:** Proposed evaluation procedure only. This document does not approve
the cleanup policy, establish an acceptance corpus, or declare AC-07 complete.
The authoritative AC-07 threshold remains in [PRD §5](PRD.md#5-formal-measurable-acceptance-criteria).

## A. Numerical scoring

[`scripts/score_cleanup_ac07.py`](../scripts/score_cleanup_ac07.py) accepts
caller-supplied `id`, `expected`, and `actual` strings in memory. It validates
that there is at least one case, IDs are nonempty and unique strings, and both
output fields are strings. Empty expected and actual strings are valid.

Expected and actual values are compared exactly as supplied. The scorer does
not normalize Unicode, punctuation, capitalization, or whitespace. It reports
the case count, exact matches, mismatches, exact-output percentage, the
200-case minimum, and the 99% threshold. The threshold decision uses integer
arithmetic:

```text
exact_matches * 100 >= total_cases * 99
```

Displayed percentages are informational and are not rounded before the pass
decision. The synthetic `--demo` creates 200 in-memory pairs with 198 exact
matches and two mismatches. It reads no current-behavior fixture and writes no
case content to logs or disk. It verifies arithmetic only.

## B. Protected-token scoring

[`scripts/score_cleanup_protected_ac07.py`](../scripts/score_cleanup_protected_ac07.py)
is a separate in-memory evaluator for explicitly reviewed token annotations.
Each record supplies `id`, `input`, `expected`, `actual`, and a
`protected_tokens` list. Every annotation supplies a supported `kind`
(`number` or `proper_noun`), nonempty token `text`, `input_bytes`, and
`output_bytes`. Both ranges are zero-based, half-open UTF-8 byte offsets: the
input range selects the token in `input`; the output range independently
selects the same token in reviewed `expected` text.

It rejects empty collections, malformed or unknown fields, invalid strings,
duplicate/empty IDs, invalid or unsupported annotations, out-of-bounds or
overlapping reference ranges, non-boundary UTF-8 offsets, and ranges whose
input or expected bytes differ from the declared token. Errors identify the
case position, token kind when valid, and field without printing transcript
contents. Invalid reference metadata rejects the evaluation; mutations in
`actual` are counted as altered tokens.

For actual-output scoring, the evaluator checks the same `output_bytes` range
against `actual`. It does not search elsewhere for the token or shift offsets
based on actual text. Therefore, an insertion or deletion before a protected
token can move that token and conservatively produce an alteration even when
the token's spelling is unchanged. The result reports case and token counts,
alteration counts, whether both token kinds are represented, whether zero
alterations were observed, and whether the protected-token gate passes. That
gate requires at least one annotation for each kind and zero alterations.
Kind coverage is an evaluation-completeness safeguard; it does not establish
comprehensive protection from a small sample.

The `--demo` uses two manually annotated synthetic examples, including a
punctuation correction that shifts the expected numeric token's byte offset
and an accented proper noun. It reads no characterization corpus and prints
counts only. A passing synthetic gate is verification of scoring logic, not
AC-07 acceptance evidence.

## C. Shared evaluation coordinator

The evaluation now has five distinct pieces:

1. **Exact-output scorer:** checks corpus count and byte-for-byte exact-match
   percentage using the unchanged 200-case minimum and 99% threshold.
2. **Protected-token scorer:** checks annotated numeric and proper-noun
   preservation at independently reviewed UTF-8 byte ranges.
3. **Shared coordinator:** [`scripts/evaluate_cleanup_ac07.py`](../scripts/evaluate_cleanup_ac07.py)
   accepts one in-memory collection in the protected-token record format,
   materializes it once, validates it with the protected-token evaluator, and
   projects those same records into the exact-output scorer. Both results are
   returned separately, and a case-count mismatch fails evaluation. Invalid
   records or reference annotations reject the whole evaluation; the
   coordinator does not return a partial result or an overall acceptance flag.
4. **Corpus metadata manifest, reference bundle, and binding validator:**
   [`docs/AC07_CORPUS_MANIFEST.md`](AC07_CORPUS_MANIFEST.md) defines versioned
   corpus identity, case metadata, provenance declarations, review tracking,
   coverage, and SHA-256 hash syntax. The separate bundle supplies decoded
   input and expected strings; the binding validator checks exact IDs and
   content hashes against the manifest without printing the strings. Neither
   artifact nor validator approves references.
5. **Protected-reference annotation bundle and validator:**
   [`scripts/validate_cleanup_ac07_protected_references.py`](../scripts/validate_cleanup_ac07_protected_references.py)
   binds an independent token-annotation file to the same corpus identity and
   case IDs, checks that each case's annotation-kind set matches the manifest
   declaration, and reuses the protected-token scorer to validate the UTF-8
   ranges against the hash-bound input and expected strings. Its temporary
   `actual=expected` value is used only for the scorer's reference validation;
   all alteration and acceptance-gate results are discarded. It reports no
   production score and does not establish acceptance.

Run its 200-case synthetic demonstration from the repository root:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 scripts/evaluate_cleanup_ac07.py --demo
```

The demonstration creates cases and explicit token offsets in memory. It has
198 exact matches, two mismatches after the protected ranges, one annotated
numeric token, one accented proper noun, and zero altered protected tokens. It
reads no cleanup fixtures, runs no Rust code, and writes no case contents to
disk. Passing its two numerical gates verifies only the scorer wiring and
synthetic arithmetic; it does not establish acceptance.

## D. Corpus metadata manifest

The version 1 manifest records a corpus ID and revision, purpose, declared
review status, case IDs, cleanup categories and settings, provenance
references, reference review declarations, planned protected-token kinds, and
input/expected SHA-256 digests. [`scripts/validate_cleanup_ac07_manifest.py`](../scripts/validate_cleanup_ac07_manifest.py)
validates this structure and the digest format. The three-case synthetic demo
is pending and is not acceptance data.

The manifest establishes identity, provenance declarations, review tracking,
coverage, and planned hash binding. It contains no input transcript, expected
output, actual output, or full protected-token text. The separate reference
bundle contains only the input and expected strings needed to verify that the
manifest's pre-existing hashes match actual UTF-8 content. A successful
binding proves correspondence to the declared manifest; it does **not** prove
expected outputs are correct or independently approved. The validator cannot
authenticate a `review_evidence_ref`; a `reviewed` declaration is not
approval. The checked-in three-case bundle is synthetic only.

Before a future evaluation passes shared records to both scorers, it must
complete this verification sequence:

1. Validate manifest metadata and digest syntax with
   [`scripts/validate_cleanup_ac07_manifest.py`](../scripts/validate_cleanup_ac07_manifest.py).
2. Validate the separately supplied reference-bundle structure, including
   strict fields, version, strings, unique IDs, and duplicate JSON key checks.
3. Verify exact corpus ID, revision, and case-ID correspondence, then compare
   SHA-256 of each exact UTF-8 input and expected string with the corresponding
   manifest digests. A reordered bundle is valid when the exact IDs and content
   match. Do not normalize or repair mismatches.
4. Validate the separate protected-token annotation bundle against the same
   corpus identity and case IDs. Compare its kinds to manifest declarations,
   then validate every token's input and expected UTF-8 byte ranges against the
   bound reference strings. This validates reference metadata only; declared
   kinds do not prove annotation correctness, and successful ranges do not
   authenticate independent review.
5. Only after independent reference approval and production execution, assemble
   shared evaluation records with corresponding actual outputs and the
   independently reviewed annotations, then invoke the existing shared
   coordinator. This is the point at which actual-output preservation is
   evaluated.

The metadata validator checks hash syntax; the separate binding validator
checks manifest-to-content correspondence; the protected-reference validator
checks annotations against that bound reference content. These validation
steps do not calculate exact-output accuracy or prove production preservation.
None authenticates review evidence or establishes approval. The protected
reference validator's `actual=expected` scorer call is only a way to reuse its
input/expected range checks; its alteration and gate results are ignored.

## E. Actual acceptance evidence

Scoring a set of strings is only one part of AC-07. Before an acceptance result
can be reported, the project still needs all of the following:

1. **Independently reviewed expected outputs.** Reviewers must define the
   expected output from approved requirements and policy, without deriving it
   by copying output from the implementation under evaluation.
2. **An explicitly identified acceptance corpus.** Record its version,
   provenance, coverage, and review status in the metadata manifest, and
   separately supply and verify the corresponding case data. It must include
   at least 200 eligible transformations and be the corpus designated for
   acceptance.
3. **Actual production results.** Run the production Rust cleanup
   implementation on the identified corpus under the approved settings and
   modes. Preserve byte-for-byte output for scoring and retain only
   appropriately reviewed, privacy-safe evaluation artifacts.
4. **Protected-token gate.** Demonstrate zero altered numeric and proper-noun
   tokens in the protected acceptance cases with the separate protected-token
   evaluator using actual production outputs. Prior annotation binding checks
   only that the annotations select the declared tokens in the reference
   strings; they do not prove preservation by production cleanup. The
   exact-output scorer does not inspect token annotations or evaluate this
   gate.
5. **Approved mode semantics.** Resolve the proposed Clean/Verbatim policy,
   document the final semantics, and verify the required distinction with
   paired cases. [`CLEANUP_MODE_POLICY.md`](CLEANUP_MODE_POLICY.md) remains
   **PROPOSED — NOT APPROVED** until that decision is made.
6. **Production integration evidence.** Verify the cleanup output is produced
   by the relevant production path, then record focused and integration test
   results, environment, corpus identity, and unresolved platform limits.

The existing 200 cases in
[`cleanup_current_behavior.json`](../crates/settings/tests/fixtures/cleanup_current_behavior.json)
are a current-behavior characterization. Their tests check deterministic
agreement with explicit expectations, but those fixtures are not automatically
an independent acceptance oracle merely because the implementation passes
them. They have not been designated as the final approved acceptance corpus,
and the selected protected-token annotations do not establish corpus-wide
protection.

## Proposed evaluation sequence

1. Obtain a maintainer decision on final Clean/Verbatim semantics and update
   authoritative requirements and tests together where needed.
2. Have the corpus owner freeze and identify the independently reviewed
   expected-output set and protected-token annotations.
3. Validate manifest metadata, then validate the separate bundle structure and
   bind its exact IDs and SHA-256 digests to the manifest as described above.
   This proves declared hash correspondence only; independently review and
   approve the expected references separately.
4. Independently approve the expected references and their review evidence.
5. Run production cleanup on the approved corpus without changing expected
   strings after seeing the output, and preserve corresponding actual outputs.
6. Only after both reference approval and production execution, assemble the
   shared records and run `evaluate_cleanup_ac07_cases()` over the same corpus
   and outputs. It sends the shared in-memory records through both unchanged
   scorers and reports their results separately.
7. Record both numerical gates, the mode distinction, production integration
   evidence, and limitations. Report AC-07 only after every formal acceptance
   condition is satisfied.

This procedure is a proposal for future evaluation. The two scorers remain
independent and neither alone establishes final acceptance. Passing both
synthetic gates or validating manifest metadata does not establish acceptance:
verified case data and hashes, approved references, approved Clean/Verbatim
semantics, production cleanup results, and integration evidence are still
required. The coordinator does not approve Verbatim behavior, modify PRD
thresholds, or make a current-behavior test pass into AC-07 acceptance.
