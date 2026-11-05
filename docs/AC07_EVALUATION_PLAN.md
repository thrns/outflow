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

## B. Actual acceptance evidence

Scoring a set of strings is only one part of AC-07. Before an acceptance result
can be reported, the project still needs all of the following:

1. **Independently reviewed expected outputs.** Reviewers must define the
   expected output from approved requirements and policy, without deriving it
   by copying output from the implementation under evaluation.
2. **An explicitly identified acceptance corpus.** Record its version,
   provenance, coverage, and review status. It must include at least 200
   eligible transformations and be the corpus designated for acceptance.
3. **Actual production results.** Run the production Rust cleanup
   implementation on the identified corpus under the approved settings and
   modes. Preserve byte-for-byte output for scoring and retain only
   appropriately reviewed, privacy-safe evaluation artifacts.
4. **Protected-token gate.** Demonstrate zero altered numeric and proper-noun
   tokens in the protected acceptance cases. The exact-output scorer does not
   inspect token annotations or evaluate this gate.
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
3. Run the production cleanup path against that exact corpus without changing
   expected strings after seeing the output.
4. Use `score_cleanup_cases()` on the in-memory ID/expected/actual records and
   evaluate protected-token annotations with a separate reviewed check.
5. Record both gates, mode distinction, production integration evidence, and
   limitations. Report AC-07 only after every formal acceptance condition is
   satisfied.

This procedure is a proposal for future evaluation. The scorer does not
implement protected-token evaluation, approve Verbatim behavior, modify PRD
thresholds, or make a current-behavior test pass into AC-07 acceptance.
