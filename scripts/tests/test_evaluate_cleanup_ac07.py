"""Tests for the shared in-memory AC-07 scorer coordinator."""

from __future__ import annotations

import contextlib
import copy
import io
import unittest
from dataclasses import FrozenInstanceError
from unittest import mock

from scripts import evaluate_cleanup_ac07 as coordinator
from scripts.score_cleanup_protected_ac07 import ProtectedCleanupValidationError


def make_cases(
    total: int = 200,
    exact_matches: int = 198,
    *,
    annotate: bool = True,
) -> list[dict[str, object]]:
    """Create shared synthetic records with explicit UTF-8 byte ranges."""
    input_text = "Dr . Élodie has 2026"
    expected = "Dr. Élodie has 2026."
    cases: list[dict[str, object]] = []

    for case_number in range(1, total + 1):
        actual = (
            expected if case_number <= exact_matches else "Dr. Élodie has 2026!"
        )
        protected_tokens: list[dict[str, object]] = []
        if annotate and case_number == 1:
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
                "id": f"case-{case_number:03d}",
                "input": input_text,
                "expected": expected,
                "actual": actual,
                "protected_tokens": protected_tokens,
            }
        )
    return cases


class OneShotIterable:
    """An iterable that fails if the coordinator asks for a second pass."""

    def __init__(self, records: list[dict[str, object]]) -> None:
        self.records = records
        self.iterations = 0

    def __iter__(self):
        self.iterations += 1
        if self.iterations > 1:
            raise AssertionError("source iterable was consumed more than once")
        return iter(self.records)


class CleanupAc07CoordinatorTests(unittest.TestCase):
    def test_both_scorers_receive_the_same_200_shared_cases(self) -> None:
        cases = make_cases()
        original = coordinator.score_protected_cleanup_cases
        with (
            mock.patch.object(
                coordinator, "score_protected_cleanup_cases", wraps=original
            ) as protected_scorer,
            mock.patch.object(
                coordinator,
                "score_cleanup_cases",
                wraps=coordinator.score_cleanup_cases,
            ) as exact_scorer,
        ):
            result = coordinator.evaluate_cleanup_ac07_cases(cases)

        protected_records = protected_scorer.call_args.args[0]
        exact_records = exact_scorer.call_args.args[0]
        self.assertEqual(len(protected_records), 200)
        self.assertEqual(len(exact_records), 200)
        self.assertEqual(protected_records, cases)
        self.assertEqual(
            exact_records,
            [
                {field: case[field] for field in ("id", "expected", "actual")}
                for case in cases
            ],
        )
        self.assertEqual(
            result.exact_output_score.total_cases,
            result.protected_token_score.total_cases,
        )

    def test_198_of_200_and_unchanged_tokens_pass_synthetic_gates(self) -> None:
        result = coordinator.evaluate_cleanup_ac07_cases(make_cases())

        self.assertEqual(result.exact_output_score.exact_matches, 198)
        self.assertEqual(result.exact_output_score.mismatches, 2)
        self.assertTrue(result.exact_output_score.minimum_corpus_size_met)
        self.assertTrue(result.exact_output_score.exact_output_threshold_met)
        self.assertTrue(result.protected_token_score.protected_token_gate_passes)
        self.assertEqual(result.protected_token_score.altered_number_tokens, 0)
        self.assertEqual(result.protected_token_score.altered_proper_noun_tokens, 0)

    def test_mismatch_after_protected_ranges_does_not_fail_protection_gate(self) -> None:
        result = coordinator.evaluate_cleanup_ac07_cases(make_cases(200, 199))

        self.assertEqual(result.exact_output_score.mismatches, 1)
        self.assertTrue(result.protected_token_score.protected_token_gate_passes)

    def test_modified_protected_number_fails_gate_at_at_least_99_percent(self) -> None:
        cases = make_cases(200, 200)
        cases[0]["actual"] = "Dr. Élodie has 2027."

        result = coordinator.evaluate_cleanup_ac07_cases(cases)

        self.assertEqual(result.exact_output_score.exact_matches, 199)
        self.assertTrue(result.exact_output_score.exact_output_threshold_met)
        self.assertEqual(result.protected_token_score.altered_number_tokens, 1)
        self.assertFalse(result.protected_token_score.protected_token_gate_passes)

    def test_modified_proper_noun_is_detected(self) -> None:
        cases = make_cases(200, 200)
        cases[0]["actual"] = "Dr. Elodie has 2026."

        result = coordinator.evaluate_cleanup_ac07_cases(cases)

        self.assertEqual(result.protected_token_score.altered_proper_noun_tokens, 1)
        self.assertFalse(result.protected_token_score.zero_alterations_observed)

    def test_197_of_200_fails_exact_threshold_with_tokens_unchanged(self) -> None:
        result = coordinator.evaluate_cleanup_ac07_cases(make_cases(200, 197))

        self.assertEqual(result.exact_output_score.exact_matches, 197)
        self.assertFalse(result.exact_output_score.exact_output_threshold_met)
        self.assertEqual(result.protected_token_score.altered_number_tokens, 0)
        self.assertTrue(result.protected_token_score.protected_token_gate_passes)

    def test_199_of_199_fails_minimum_corpus_size(self) -> None:
        result = coordinator.evaluate_cleanup_ac07_cases(make_cases(199, 199))

        self.assertTrue(result.exact_output_score.exact_output_threshold_met)
        self.assertFalse(result.exact_output_score.minimum_corpus_size_met)

    def test_empty_annotation_lists_remain_in_exact_output_score(self) -> None:
        result = coordinator.evaluate_cleanup_ac07_cases(
            make_cases(200, 198, annotate=False)
        )

        self.assertEqual(result.exact_output_score.total_cases, 200)
        self.assertEqual(result.exact_output_score.exact_matches, 198)
        self.assertEqual(result.protected_token_score.total_cases, 200)
        self.assertFalse(result.protected_token_score.protected_token_gate_passes)

    def test_no_annotations_fails_protected_token_coverage(self) -> None:
        result = coordinator.evaluate_cleanup_ac07_cases(
            make_cases(200, 200, annotate=False)
        )

        self.assertEqual(result.protected_token_score.annotated_cases, 0)
        self.assertFalse(result.protected_token_score.both_token_kinds_represented)
        self.assertFalse(result.protected_token_score.protected_token_gate_passes)

    def test_corpus_with_only_one_token_kind_fails_coverage(self) -> None:
        cases = make_cases(200, 200)
        cases[0]["protected_tokens"] = cases[0]["protected_tokens"][1:]

        result = coordinator.evaluate_cleanup_ac07_cases(cases)

        self.assertEqual(result.protected_token_score.protected_number_tokens, 1)
        self.assertEqual(result.protected_token_score.protected_proper_noun_tokens, 0)
        self.assertFalse(result.protected_token_score.protected_token_gate_passes)

    def test_invalid_utf8_reference_offsets_raise_validation_error(self) -> None:
        cases = make_cases(1, 1)
        cases[0]["protected_tokens"][0]["input_bytes"] = [5, 6]

        with self.assertRaisesRegex(
            ProtectedCleanupValidationError, "UTF-8 character boundaries"
        ):
            coordinator.evaluate_cleanup_ac07_cases(cases)

    def test_duplicate_ids_raise_validation_error(self) -> None:
        cases = make_cases(2, 2)
        cases[1]["id"] = cases[0]["id"]

        with self.assertRaisesRegex(ProtectedCleanupValidationError, "duplicates"):
            coordinator.evaluate_cleanup_ac07_cases(cases)

    def test_missing_fields_raise_validation_error(self) -> None:
        cases = make_cases(1, 1)
        del cases[0]["protected_tokens"]

        with self.assertRaisesRegex(ProtectedCleanupValidationError, "missing"):
            coordinator.evaluate_cleanup_ac07_cases(cases)

    def test_one_shot_iterable_is_consumed_once_without_losing_records(self) -> None:
        cases = OneShotIterable(make_cases(200, 198))

        result = coordinator.evaluate_cleanup_ac07_cases(cases)

        self.assertEqual(cases.iterations, 1)
        self.assertEqual(result.exact_output_score.total_cases, 200)
        self.assertEqual(result.protected_token_score.total_cases, 200)

    def test_repeated_evaluations_are_deterministic(self) -> None:
        cases = make_cases()

        first = coordinator.evaluate_cleanup_ac07_cases(cases)
        second = coordinator.evaluate_cleanup_ac07_cases(copy.deepcopy(cases))

        self.assertEqual(first, second)

    def test_demo_reports_synthetic_results_without_formal_acceptance(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = coordinator.main(["--demo"])

        self.assertEqual(exit_code, 0)
        self.assertEqual(
            output.getvalue(),
            "AC-07 combined evaluator: SYNTHETIC DEMO\n"
            "Cases: 200\n"
            "Exact matches: 198\n"
            "Mismatches: 2\n"
            "Exact-output rate: 99.00%\n"
            "Minimum corpus size: PASS\n"
            "Exact-output threshold: PASS\n"
            "Protected numeric tokens: 1 (0 altered)\n"
            "Protected proper nouns: 1 (0 altered)\n"
            "Protected-token gate: PASS (synthetic only)\n"
            "Shared case count: PASS\n"
            "Final Clean/Verbatim policy: NOT APPROVED\n"
            "AC-07 acceptance: NOT EVALUATED\n",
        )
        self.assertNotIn("AC-07 acceptance: PASS", output.getvalue())

    def test_result_dataclass_is_immutable(self) -> None:
        result = coordinator.evaluate_cleanup_ac07_cases(make_cases(200, 200))

        with self.assertRaises(FrozenInstanceError):
            result.exact_output_score = result.exact_output_score  # type: ignore[misc]


if __name__ == "__main__":
    unittest.main()
