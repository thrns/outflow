#!/usr/bin/env python3
"""Coordinate the independent AC-07 scorers over one in-memory case set.

This utility validates the shared records with the protected-token evaluator,
then projects those same records into the exact-output scorer. Its synthetic
demonstration verifies scorer wiring and arithmetic only; it does not run Rust
cleanup or establish AC-07 acceptance.
"""

from __future__ import annotations

import argparse
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from typing import Sequence

try:  # Support direct script execution and package-based imports.
    from .score_cleanup_ac07 import CleanupScore, score_cleanup_cases
    from .score_cleanup_protected_ac07 import (
        ProtectedCleanupScore,
        ProtectedCleanupValidationError,
        score_protected_cleanup_cases,
    )
except ImportError:  # pragma: no cover - exercised by direct CLI invocation.
    from score_cleanup_ac07 import CleanupScore, score_cleanup_cases
    from score_cleanup_protected_ac07 import (
        ProtectedCleanupScore,
        ProtectedCleanupValidationError,
        score_protected_cleanup_cases,
    )


@dataclass(frozen=True)
class CleanupAc07Evaluation:
    """Independent exact-output and protected-token results for one corpus."""

    exact_output_score: CleanupScore
    protected_token_score: ProtectedCleanupScore


def evaluate_cleanup_ac07_cases(
    cases: Iterable[Mapping[str, object]],
) -> CleanupAc07Evaluation:
    """Evaluate shared records with both existing AC-07 scorers.

    The iterable is materialized once. The protected-token evaluator is the
    canonical validator for every shared record and its reference annotations;
    only after it succeeds are the same records projected for exact scoring.

    Raises:
        ProtectedCleanupValidationError: If the input is not iterable, the
            corpus is empty, or any record or reference annotation is invalid.
        CleanupScoreValidationError: If projected exact-output records fail
            the exact scorer's validation.
        RuntimeError: If the existing scorers report different case counts.
    """
    if not isinstance(cases, Iterable):
        raise ProtectedCleanupValidationError(
            "cases must be an iterable of records"
        )

    shared_cases = list(cases)
    protected_token_score = score_protected_cleanup_cases(shared_cases)
    exact_cases = [
        {field: case[field] for field in ("id", "expected", "actual")}
        for case in shared_cases
    ]
    exact_output_score = score_cleanup_cases(exact_cases)

    if exact_output_score.total_cases != protected_token_score.total_cases:
        raise RuntimeError("AC-07 scorers returned different case counts")

    return CleanupAc07Evaluation(
        exact_output_score=exact_output_score,
        protected_token_score=protected_token_score,
    )


def _demo_cases() -> list[dict[str, object]]:
    """Build the 200 explicitly annotated synthetic cases in memory."""
    input_text = "Dr . Élodie has 2026"
    expected_text = "Dr. Élodie has 2026."
    cases: list[dict[str, object]] = []

    for case_number in range(1, 201):
        actual_text = (
            expected_text
            if case_number <= 198
            else "Dr. Élodie has 2026!"
        )
        protected_tokens: list[dict[str, object]] = []
        if case_number == 1:
            # These UTF-8 byte offsets are authored explicitly. The accented
            # É occupies two bytes, so its six-character name spans 7 bytes.
            protected_tokens = [
                {
                    "kind": "proper_noun",
                    "text": "Élodie",
                    "input_bytes": [5, 12],
                    "output_bytes": [4, 11],
                },
                {
                    "kind": "number",
                    "text": "2026",
                    "input_bytes": [17, 21],
                    "output_bytes": [16, 20],
                },
            ]

        cases.append(
            {
                "id": f"demo-{case_number:03d}",
                "input": input_text,
                "expected": expected_text,
                "actual": actual_text,
                "protected_tokens": protected_tokens,
            }
        )

    return cases


def _run_demo() -> None:
    """Print counts only after both scorers successfully evaluate the demo."""
    result = evaluate_cleanup_ac07_cases(_demo_cases())
    exact = result.exact_output_score
    protected = result.protected_token_score

    print("AC-07 combined evaluator: SYNTHETIC DEMO")
    print(f"Cases: {exact.total_cases}")
    print(f"Exact matches: {exact.exact_matches}")
    print(f"Mismatches: {exact.mismatches}")
    print(f"Exact-output rate: {exact.exact_output_percentage:.2f}%")
    print(
        "Minimum corpus size: "
        f"{'PASS' if exact.minimum_corpus_size_met else 'FAIL'}"
    )
    print(
        "Exact-output threshold: "
        f"{'PASS' if exact.exact_output_threshold_met else 'FAIL'}"
    )
    print(
        "Protected numeric tokens: "
        f"{protected.protected_number_tokens} "
        f"({protected.altered_number_tokens} altered)"
    )
    print(
        "Protected proper nouns: "
        f"{protected.protected_proper_noun_tokens} "
        f"({protected.altered_proper_noun_tokens} altered)"
    )
    gate_status = "PASS" if protected.protected_token_gate_passes else "FAIL"
    print(f"Protected-token gate: {gate_status} (synthetic only)")
    print("Shared case count: PASS")
    print("Final Clean/Verbatim policy: NOT APPROVED")
    print("AC-07 acceptance: NOT EVALUATED")


def main(argv: Sequence[str] | None = None) -> int:
    """Run the synthetic demonstration; real cases are supplied in memory."""
    parser = argparse.ArgumentParser(
        description="Evaluate shared in-memory records with both AC-07 scorers"
    )
    parser.add_argument(
        "--demo",
        action="store_true",
        help="run a synthetic 200-case coordinator demonstration",
    )
    arguments = parser.parse_args(argv)
    if not arguments.demo:
        parser.error(
            "choose --demo; use evaluate_cleanup_ac07_cases() "
            "for in-memory records"
        )
    _run_demo()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
