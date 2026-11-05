"""Tests for the independent exact-output AC-07 scoring utility."""

from __future__ import annotations

import contextlib
import io
import unittest

from scripts.score_cleanup_ac07 import (
    CleanupScoreValidationError,
    main,
    score_cleanup_cases,
)


def make_cases(total: int, exact_matches: int) -> list[dict[str, str]]:
    """Create unique synthetic records in memory for threshold tests."""
    return [
        {
            "id": f"case-{case_number:03d}",
            "expected": f"Expected {case_number}",
            "actual": (
                f"Expected {case_number}"
                if case_number <= exact_matches
                else f"Actual {case_number}"
            ),
        }
        for case_number in range(1, total + 1)
    ]


class CleanupAc07ScorerTests(unittest.TestCase):
    def test_200_of_200_exact_matches_pass_both_checks(self) -> None:
        result = score_cleanup_cases(make_cases(200, 200))

        self.assertEqual(result.total_cases, 200)
        self.assertEqual(result.exact_matches, 200)
        self.assertEqual(result.mismatches, 0)
        self.assertEqual(result.exact_output_percentage, 100.0)
        self.assertTrue(result.minimum_corpus_size_met)
        self.assertTrue(result.exact_output_threshold_met)

    def test_198_of_200_passes_at_exactly_99_percent(self) -> None:
        result = score_cleanup_cases(make_cases(200, 198))

        self.assertEqual(result.exact_output_percentage, 99.0)
        self.assertTrue(result.minimum_corpus_size_met)
        self.assertTrue(result.exact_output_threshold_met)

    def test_197_of_200_fails_at_98_5_percent(self) -> None:
        result = score_cleanup_cases(make_cases(200, 197))

        self.assertEqual(result.exact_output_percentage, 98.5)
        self.assertFalse(result.exact_output_threshold_met)

    def test_199_of_199_passes_percentage_but_fails_minimum_count(self) -> None:
        result = score_cleanup_cases(make_cases(199, 199))

        self.assertTrue(result.exact_output_threshold_met)
        self.assertFalse(result.minimum_corpus_size_met)

    def test_199_of_201_passes_threshold_without_rounding(self) -> None:
        result = score_cleanup_cases(make_cases(201, 199))

        self.assertTrue(result.exact_output_threshold_met)
        self.assertTrue(result.minimum_corpus_size_met)
        self.assertAlmostEqual(result.exact_output_percentage, 99.0049751243781)

    def test_198_of_201_fails_even_when_whole_percent_would_round_up(self) -> None:
        result = score_cleanup_cases(make_cases(201, 198))

        self.assertFalse(result.exact_output_threshold_met)
        self.assertAlmostEqual(result.exact_output_percentage, 98.50746268656717)

    def test_capitalization_difference_is_a_mismatch(self) -> None:
        result = score_cleanup_cases(
            [{"id": "case-1", "expected": "Hello", "actual": "hello"}]
        )

        self.assertEqual(result.mismatches, 1)

    def test_whitespace_difference_is_a_mismatch(self) -> None:
        result = score_cleanup_cases(
            [{"id": "case-1", "expected": "Hello world", "actual": "Hello  world"}]
        )

        self.assertEqual(result.mismatches, 1)

    def test_composed_and_decomposed_unicode_are_not_identical(self) -> None:
        result = score_cleanup_cases(
            [{"id": "case-1", "expected": "é", "actual": "e\u0301"}]
        )

        self.assertEqual(result.mismatches, 1)

    def test_empty_expected_and_actual_strings_can_match(self) -> None:
        result = score_cleanup_cases(
            [{"id": "case-1", "expected": "", "actual": ""}]
        )

        self.assertEqual(result.exact_matches, 1)

    def test_duplicate_ids_are_rejected(self) -> None:
        cases = [
            {"id": "same", "expected": "a", "actual": "a"},
            {"id": "same", "expected": "b", "actual": "b"},
        ]

        with self.assertRaisesRegex(CleanupScoreValidationError, "duplicates"):
            score_cleanup_cases(cases)

    def test_empty_and_whitespace_only_ids_are_rejected(self) -> None:
        for case_id in ("", " \t\n"):
            with self.subTest(case_id=repr(case_id)):
                cases = [{"id": case_id, "expected": "", "actual": ""}]
                with self.assertRaisesRegex(CleanupScoreValidationError, "nonempty"):
                    score_cleanup_cases(cases)

    def test_non_string_expected_and_actual_values_are_rejected(self) -> None:
        for field in ("expected", "actual"):
            with self.subTest(field=field):
                case = {"id": "case-1", "expected": "text", "actual": "text"}
                case[field] = 7
                with self.assertRaisesRegex(CleanupScoreValidationError, field):
                    score_cleanup_cases([case])

    def test_empty_collection_is_rejected(self) -> None:
        with self.assertRaisesRegex(CleanupScoreValidationError, "at least one"):
            score_cleanup_cases([])

    def test_invalid_records_have_explicit_errors(self) -> None:
        with self.assertRaisesRegex(CleanupScoreValidationError, "mapping"):
            score_cleanup_cases(["not a record"])
        with self.assertRaisesRegex(CleanupScoreValidationError, "missing required"):
            score_cleanup_cases([{"id": "case-1", "expected": "text"}])
        with self.assertRaisesRegex(CleanupScoreValidationError, "id.*string"):
            score_cleanup_cases([{"id": 1, "expected": "text", "actual": "text"}])

    def test_repeated_scoring_is_deterministic(self) -> None:
        cases = make_cases(201, 199)

        self.assertEqual(score_cleanup_cases(cases), score_cleanup_cases(cases))

    def test_demo_reports_acceptance_as_not_evaluated(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = main(["--demo"])

        self.assertEqual(exit_code, 0)
        self.assertIn("AC-07 acceptance: NOT EVALUATED", output.getvalue())
        self.assertIn("Protected-token acceptance: NOT EVALUATED", output.getvalue())
        self.assertIn("Final Clean/Verbatim policy: NOT APPROVED", output.getvalue())


if __name__ == "__main__":
    unittest.main()
