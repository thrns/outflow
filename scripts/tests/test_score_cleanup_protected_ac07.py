"""Tests for the independent protected-token AC-07 evaluator."""

from __future__ import annotations

import contextlib
import copy
import io
import unittest
from dataclasses import FrozenInstanceError

from scripts.score_cleanup_protected_ac07 import (
    ProtectedCleanupValidationError,
    main,
    score_protected_cleanup_cases,
)


def make_case(
    *,
    case_id: str = "case-1",
    kind: str = "number",
    input_text: str = "42",
    expected: str = "42",
    actual: str = "42",
    token: str = "42",
    input_bytes: list[int] | tuple[int, int] = (0, 2),
    output_bytes: list[int] | tuple[int, int] = (0, 2),
) -> dict[str, object]:
    return {
        "id": case_id,
        "input": input_text,
        "expected": expected,
        "actual": actual,
        "protected_tokens": [
            {
                "kind": kind,
                "text": token,
                "input_bytes": input_bytes,
                "output_bytes": output_bytes,
            }
        ],
    }


class ProtectedCleanupAc07ScorerTests(unittest.TestCase):
    def test_valid_numeric_token_is_preserved(self) -> None:
        result = score_protected_cleanup_cases([make_case()])

        self.assertEqual(result.protected_number_tokens, 1)
        self.assertEqual(result.altered_number_tokens, 0)
        self.assertFalse(result.protected_token_gate_passes)

    def test_valid_accented_proper_noun_is_preserved(self) -> None:
        case = make_case(
            kind="proper_noun",
            input_text="Dr . Élodie",
            expected="Dr. Élodie",
            actual="Dr. Élodie",
            token="Élodie",
            input_bytes=[5, 12],
            output_bytes=[4, 11],
        )

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.protected_proper_noun_tokens, 1)
        self.assertEqual(result.altered_proper_noun_tokens, 0)

    def test_multibyte_prefix_uses_utf8_byte_offsets(self) -> None:
        case = make_case(
            input_text="É 42",
            expected="É 42",
            actual="É 42",
            token="42",
            input_bytes=[3, 5],
            output_bytes=[3, 5],
        )

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.altered_number_tokens, 0)

    def test_input_and_expected_ranges_can_differ_after_punctuation_edit(self) -> None:
        case = make_case(
            input_text="Hello , 2026",
            expected="Hello, 2026",
            actual="Hello, 2026",
            token="2026",
            input_bytes=[8, 12],
            output_bytes=[7, 11],
        )

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.protected_number_tokens, 1)
        self.assertEqual(result.altered_number_tokens, 0)

    def test_multiple_protected_tokens_in_one_case(self) -> None:
        case = {
            "id": "mixed-one-case",
            "input": "Dr . Élodie has 42",
            "expected": "Dr. Élodie has 42",
            "actual": "Dr. Élodie has 42",
            "protected_tokens": [
                {
                    "kind": "proper_noun",
                    "text": "Élodie",
                    "input_bytes": [5, 12],
                    "output_bytes": [4, 11],
                },
                {
                    "kind": "number",
                    "text": "42",
                    "input_bytes": [17, 19],
                    "output_bytes": [16, 18],
                },
            ],
        }

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.annotated_cases, 1)
        self.assertEqual(result.protected_proper_noun_tokens, 1)
        self.assertEqual(result.protected_number_tokens, 1)
        self.assertTrue(result.protected_token_gate_passes)

    def test_changed_numeric_bytes_are_counted_as_altered(self) -> None:
        case = make_case(actual="43")

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.protected_number_tokens, 1)
        self.assertEqual(result.altered_number_tokens, 1)
        self.assertFalse(result.zero_alterations_observed)

    def test_changed_proper_noun_bytes_are_counted_as_altered(self) -> None:
        case = make_case(
            kind="proper_noun",
            input_text="Élodie",
            expected="Élodie",
            actual="Élod1e",
            token="Élodie",
            input_bytes=[0, 7],
            output_bytes=[0, 7],
        )

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.altered_proper_noun_tokens, 1)

    def test_insertion_before_a_token_is_conservatively_altered(self) -> None:
        case = make_case(
            input_text="abc 42",
            expected="abc 42",
            actual="Xabc 42",
            token="42",
            input_bytes=[4, 6],
            output_bytes=[4, 6],
        )

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.altered_number_tokens, 1)

    def test_actual_output_shorter_than_declared_range_is_altered(self) -> None:
        case = make_case(actual="4")

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.altered_number_tokens, 1)

    def test_actual_range_ending_inside_utf8_character_is_altered(self) -> None:
        case = make_case(
            input_text="123456",
            expected="123456",
            actual="12345é",
            token="123456",
            input_bytes=[0, 6],
            output_bytes=[0, 6],
        )

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.altered_number_tokens, 1)

    def test_declared_input_range_must_select_the_declared_token(self) -> None:
        case = make_case(token="43")

        with self.assertRaisesRegex(ProtectedCleanupValidationError, "input_bytes"):
            score_protected_cleanup_cases([case])

    def test_declared_expected_range_must_select_the_declared_token(self) -> None:
        case = make_case(
            input_text="42",
            expected="43",
            actual="43",
            token="42",
        )

        with self.assertRaisesRegex(ProtectedCleanupValidationError, "output_bytes"):
            score_protected_cleanup_cases([case])

    def test_reference_ranges_out_of_bounds_are_rejected(self) -> None:
        for field, offsets in (
            ("input_bytes", [0, 3]),
            ("output_bytes", [0, 3]),
        ):
            with self.subTest(field=field):
                case = make_case()
                case["protected_tokens"][0][field] = offsets
                with self.assertRaisesRegex(
                    ProtectedCleanupValidationError, "out of bounds"
                ):
                    score_protected_cleanup_cases([case])

    def test_input_and_expected_ranges_must_use_utf8_boundaries(self) -> None:
        input_case = make_case(
            input_text="É",
            expected="É",
            actual="É",
            token="É",
            input_bytes=[1, 2],
            output_bytes=[0, 2],
        )
        expected_case = make_case(
            input_text="É",
            expected="É",
            actual="É",
            token="É",
            input_bytes=[0, 2],
            output_bytes=[0, 1],
        )

        for case, field in (
            (input_case, "input_bytes"),
            (expected_case, "output_bytes"),
        ):
            with self.subTest(field=field):
                with self.assertRaisesRegex(
                    ProtectedCleanupValidationError, "UTF-8 character boundaries"
                ):
                    score_protected_cleanup_cases([case])

    def test_protected_ranges_may_not_overlap(self) -> None:
        case = {
            "id": "overlapping",
            "input": "1234",
            "expected": "1234",
            "actual": "1234",
            "protected_tokens": [
                {
                    "kind": "number",
                    "text": "123",
                    "input_bytes": [0, 3],
                    "output_bytes": [0, 3],
                },
                {
                    "kind": "number",
                    "text": "234",
                    "input_bytes": [1, 4],
                    "output_bytes": [1, 4],
                },
            ],
        }

        with self.assertRaisesRegex(ProtectedCleanupValidationError, "overlaps"):
            score_protected_cleanup_cases([case])

    def test_unsupported_kind_is_rejected(self) -> None:
        case = make_case(kind="date")

        with self.assertRaisesRegex(ProtectedCleanupValidationError, "kind"):
            score_protected_cleanup_cases([case])

    def test_empty_token_text_is_rejected(self) -> None:
        case = make_case(token="")

        with self.assertRaisesRegex(ProtectedCleanupValidationError, "text.*nonempty"):
            score_protected_cleanup_cases([case])

    def test_non_string_token_text_is_rejected(self) -> None:
        case = make_case()
        case["protected_tokens"][0]["text"] = 42

        with self.assertRaisesRegex(ProtectedCleanupValidationError, "text.*string"):
            score_protected_cleanup_cases([case])

    def test_token_text_that_cannot_be_encoded_as_utf8_is_rejected(self) -> None:
        case = make_case(token="\ud800")

        with self.assertRaisesRegex(ProtectedCleanupValidationError, "text.*UTF-8"):
            score_protected_cleanup_cases([case])

    def test_duplicate_and_empty_case_ids_are_rejected(self) -> None:
        for cases in (
            [make_case(case_id="same"), make_case(case_id="same")],
            [make_case(case_id=" ")],
        ):
            with self.subTest(ids=[case["id"] for case in cases]):
                with self.assertRaises(ProtectedCleanupValidationError):
                    score_protected_cleanup_cases(cases)

    def test_missing_and_non_string_fields_are_rejected(self) -> None:
        missing = make_case()
        del missing["actual"]
        with self.assertRaisesRegex(ProtectedCleanupValidationError, "missing"):
            score_protected_cleanup_cases([missing])

        for field in ("id", "input", "expected", "actual", "protected_tokens"):
            case = make_case()
            case[field] = 7
            with self.subTest(field=field):
                with self.assertRaises(ProtectedCleanupValidationError):
                    score_protected_cleanup_cases([case])

    def test_invalid_utf8_surrogate_strings_are_rejected(self) -> None:
        for field in ("input", "expected", "actual"):
            case = make_case()
            case[field] = "\ud800"
            with self.subTest(field=field):
                with self.assertRaisesRegex(ProtectedCleanupValidationError, "UTF-8"):
                    score_protected_cleanup_cases([case])

    def test_unknown_record_and_annotation_fields_are_rejected(self) -> None:
        record_case = make_case()
        record_case["transcript"] = "private"
        annotation_case = make_case()
        annotation_case["protected_tokens"][0]["confidence"] = 1

        for case in (record_case, annotation_case):
            with self.subTest(case=case["id"]):
                with self.assertRaisesRegex(
                    ProtectedCleanupValidationError, "unsupported field"
                ) as caught:
                    score_protected_cleanup_cases([case])
                self.assertNotIn("private", str(caught.exception))

    def test_malformed_annotation_shapes_are_rejected(self) -> None:
        non_mapping = make_case()
        non_mapping["protected_tokens"] = ["not an annotation"]
        missing_field = make_case()
        del missing_field["protected_tokens"][0]["input_bytes"]
        bad_range = make_case(input_bytes=[0, True])
        wrong_range_length = make_case(input_bytes=[0, 1, 2])
        negative_range = make_case(input_bytes=[-1, 2])
        empty_range = make_case(input_bytes=[1, 1])
        reversed_range = make_case(input_bytes=[2, 1])
        cases = (
            non_mapping,
            missing_field,
            bad_range,
            wrong_range_length,
            negative_range,
            empty_range,
            reversed_range,
        )

        for case in cases:
            with self.subTest(case=case["id"]):
                with self.assertRaises(ProtectedCleanupValidationError):
                    score_protected_cleanup_cases([case])

    def test_no_protected_tokens_is_valid_but_gate_fails(self) -> None:
        case = make_case()
        case["protected_tokens"] = []

        result = score_protected_cleanup_cases([case])

        self.assertEqual(result.total_cases, 1)
        self.assertEqual(result.annotated_cases, 0)
        self.assertFalse(result.both_token_kinds_represented)
        self.assertTrue(result.zero_alterations_observed)
        self.assertFalse(result.protected_token_gate_passes)

    def test_only_one_token_kind_does_not_pass_gate(self) -> None:
        numeric = score_protected_cleanup_cases([make_case(kind="number")])
        proper_noun = score_protected_cleanup_cases(
            [
                make_case(
                    kind="proper_noun",
                    token="Élodie",
                    input_text="Élodie",
                    expected="Élodie",
                    actual="Élodie",
                    input_bytes=[0, 7],
                    output_bytes=[0, 7],
                )
            ]
        )

        self.assertFalse(numeric.both_token_kinds_represented)
        self.assertFalse(numeric.protected_token_gate_passes)
        self.assertFalse(proper_noun.both_token_kinds_represented)
        self.assertFalse(proper_noun.protected_token_gate_passes)

    def test_mixed_annotations_with_zero_alterations_pass_gate(self) -> None:
        result = score_protected_cleanup_cases(
            [
                make_case(case_id="number"),
                make_case(
                    case_id="name",
                    kind="proper_noun",
                    token="Élodie",
                    input_text="Élodie",
                    expected="Élodie",
                    actual="Élodie",
                    input_bytes=[0, 7],
                    output_bytes=[0, 7],
                ),
            ]
        )

        self.assertEqual(result.total_cases, 2)
        self.assertEqual(result.annotated_cases, 2)
        self.assertTrue(result.both_token_kinds_represented)
        self.assertTrue(result.zero_alterations_observed)
        self.assertTrue(result.protected_token_gate_passes)

    def test_empty_collection_and_non_mapping_records_are_rejected(self) -> None:
        with self.assertRaisesRegex(ProtectedCleanupValidationError, "at least one"):
            score_protected_cleanup_cases([])
        with self.assertRaisesRegex(ProtectedCleanupValidationError, "mapping"):
            score_protected_cleanup_cases(["not a record"])

    def test_repeated_evaluation_is_deterministic(self) -> None:
        cases = [make_case(), make_case(case_id="case-2", actual="43")]

        self.assertEqual(
            score_protected_cleanup_cases(cases),
            score_protected_cleanup_cases(copy.deepcopy(cases)),
        )

    def test_result_is_immutable(self) -> None:
        result = score_protected_cleanup_cases([make_case()])

        with self.assertRaises(FrozenInstanceError):
            result.total_cases = 2  # type: ignore[misc]

    def test_demo_reports_summary_and_ac07_not_evaluated(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = main(["--demo"])

        self.assertEqual(exit_code, 0)
        self.assertEqual(
            output.getvalue(),
            "AC-07 protected-token scorer: SYNTHETIC DEMO\n"
            "Cases: 2\n"
            "Annotated cases: 2\n"
            "Protected numeric tokens: 1 (0 altered)\n"
            "Protected proper nouns: 1 (0 altered)\n"
            "Both token kinds represented: PASS\n"
            "Protected-token gate: PASS (synthetic only)\n"
            "AC-07 acceptance: NOT EVALUATED\n",
        )


if __name__ == "__main__":
    unittest.main()
