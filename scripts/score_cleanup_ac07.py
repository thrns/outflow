#!/usr/bin/env python3
"""Score exact cleanup outputs from caller-supplied strings only.

This standalone development utility does not invoke Outflow's Rust cleanup
pipeline, read fixture files, or persist case content. A numeric score is not
AC-07 acceptance evidence; the approved corpus, protected-token gate, final
Clean/Verbatim policy, and production integration evidence remain separate.
"""

from __future__ import annotations

import argparse
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from typing import Sequence


MINIMUM_CASES = 200
EXACT_OUTPUT_THRESHOLD_PERCENT = 99


class CleanupScoreValidationError(ValueError):
    """Caller-supplied scoring cases failed structural validation."""


@dataclass(frozen=True)
class CleanupScore:
    """Exact-output counts and independent corpus/percentage gate results."""

    total_cases: int
    exact_matches: int
    mismatches: int
    exact_output_percentage: float
    minimum_corpus_size_met: bool
    exact_output_threshold_met: bool


def score_cleanup_cases(
    cases: Iterable[Mapping[str, object]],
) -> CleanupScore:
    """Score records containing string ``id``, ``expected``, and ``actual`` fields.

    String comparison uses Python's exact code-point sequence equality. No
    normalization, case folding, whitespace collapsing, or punctuation
    handling is applied; empty expected and actual strings may match.

    Raises:
        CleanupScoreValidationError: If cases are empty, IDs are invalid or
            duplicated, a record is not a mapping, or a required value is not
            a string. Error messages identify case positions and fields only;
            they do not include expected or actual text.
    """
    if not isinstance(cases, Iterable):
        raise CleanupScoreValidationError("cases must be an iterable of records")

    seen_ids: set[str] = set()
    total_cases = 0
    exact_matches = 0

    for case_number, record in enumerate(cases, start=1):
        if not isinstance(record, Mapping):
            raise CleanupScoreValidationError(
                f"case {case_number}: record must be a mapping"
            )

        for field in ("id", "expected", "actual"):
            if field not in record:
                raise CleanupScoreValidationError(
                    f"case {case_number}: missing required field '{field}'"
                )

        case_id = record["id"]
        if not isinstance(case_id, str):
            raise CleanupScoreValidationError(
                f"case {case_number}: field 'id' must be a string"
            )
        if not case_id.strip():
            raise CleanupScoreValidationError(
                f"case {case_number}: field 'id' must be nonempty"
            )
        if case_id in seen_ids:
            raise CleanupScoreValidationError(
                f"case {case_number}: field 'id' duplicates an earlier case"
            )

        expected = record["expected"]
        if not isinstance(expected, str):
            raise CleanupScoreValidationError(
                f"case {case_number}: field 'expected' must be a string"
            )

        actual = record["actual"]
        if not isinstance(actual, str):
            raise CleanupScoreValidationError(
                f"case {case_number}: field 'actual' must be a string"
            )

        seen_ids.add(case_id)
        total_cases += 1
        if expected == actual:
            exact_matches += 1

    if total_cases == 0:
        raise CleanupScoreValidationError("at least one case is required")

    mismatches = total_cases - exact_matches
    exact_output_percentage = exact_matches / total_cases * 100
    # Keep the decision on integer arithmetic; formatting the percentage must
    # never turn a failing score into a passing threshold result.
    exact_output_threshold_met = (
        exact_matches * 100 >= total_cases * EXACT_OUTPUT_THRESHOLD_PERCENT
    )

    return CleanupScore(
        total_cases=total_cases,
        exact_matches=exact_matches,
        mismatches=mismatches,
        exact_output_percentage=exact_output_percentage,
        minimum_corpus_size_met=total_cases >= MINIMUM_CASES,
        exact_output_threshold_met=exact_output_threshold_met,
    )


def _demo_cases() -> list[dict[str, str]]:
    """Build 200 deterministic synthetic pairs without filesystem access."""
    cases = []
    for case_number in range(1, MINIMUM_CASES + 1):
        expected = f"Synthetic sentence {case_number}."
        actual = expected if case_number <= 198 else f"synthetic sentence {case_number}."
        cases.append(
            {
                "id": f"demo-{case_number:03d}",
                "expected": expected,
                "actual": actual,
            }
        )
    return cases


def _run_demo() -> None:
    """Print an arithmetic-only demonstration with no per-case content."""
    result = score_cleanup_cases(_demo_cases())
    print("AC-07 exact-output scorer: SYNTHETIC DEMO")
    print(f"Cases: {result.total_cases}")
    print(f"Exact matches: {result.exact_matches}")
    print(f"Mismatches: {result.mismatches}")
    print(f"Exact-output rate: {result.exact_output_percentage:.2f}%")
    print(
        "Minimum corpus size: "
        f"{'PASS' if result.minimum_corpus_size_met else 'FAIL'}"
    )
    print(
        "Exact-output threshold: "
        f"{'PASS' if result.exact_output_threshold_met else 'FAIL'}"
    )
    print("Protected-token acceptance: NOT EVALUATED")
    print("Final Clean/Verbatim policy: NOT APPROVED")
    print("AC-07 acceptance: NOT EVALUATED")


def main(argv: Sequence[str] | None = None) -> int:
    """Run the synthetic demonstration; real data is supplied through the API."""
    parser = argparse.ArgumentParser(
        description="Score exact cleanup outputs supplied directly in memory"
    )
    parser.add_argument(
        "--demo",
        action="store_true",
        help="run a synthetic 200-case arithmetic demonstration",
    )
    arguments = parser.parse_args(argv)
    if not arguments.demo:
        parser.error("choose --demo; use score_cleanup_cases() for in-memory records")
    _run_demo()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
