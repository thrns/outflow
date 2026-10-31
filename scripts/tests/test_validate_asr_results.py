"""Tests for the synthetic ASR results arithmetic validator."""

from __future__ import annotations

import contextlib
import copy
import io
import json
import tempfile
import unittest
from pathlib import Path
from typing import Any

from scripts.score_corpus_wer import calculate_corpus_wer
from scripts.validate_asr_results import (
    ResultsValidationError,
    format_summary,
    load_results,
    main,
    validate_results,
)


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
RESULTS_PATH = REPOSITORY_ROOT / "scripts" / "testdata" / "synthetic_asr_results.json"


def load_synthetic_results() -> dict[str, Any]:
    return json.loads(RESULTS_PATH.read_text(encoding="utf-8"))


class AsrResultsValidatorTests(unittest.TestCase):
    def test_valid_synthetic_results_are_accepted(self) -> None:
        summary = validate_results(load_synthetic_results())

        self.assertEqual(summary.samples, 2)
        self.assertEqual(summary.reference_words, 10)
        self.assertEqual(summary.substitutions, 1)
        self.assertEqual(summary.deletions, 0)
        self.assertEqual(summary.insertions, 0)
        self.assertEqual(summary.wer, 0.1)

    def test_unsupported_schema_versions_are_rejected(self) -> None:
        for version in (2, True, "1"):
            with self.subTest(version=version):
                data = load_synthetic_results()
                data["schema_version"] = version

                with self.assertRaisesRegex(ResultsValidationError, "schema_version must be 1"):
                    validate_results(data)

    def test_unsupported_result_types_are_rejected(self) -> None:
        for result_type in ("measured", "certified"):
            with self.subTest(result_type=result_type):
                data = load_synthetic_results()
                data["result_type"] = result_type

                with self.assertRaisesRegex(ResultsValidationError, "measured results are not supported"):
                    validate_results(data)

    def test_duplicate_sample_ids_are_rejected(self) -> None:
        data = load_synthetic_results()
        data["samples"][1]["id"] = data["samples"][0]["id"]

        with self.assertRaisesRegex(ResultsValidationError, "duplicates another sample identifier"):
            validate_results(data)

    def test_identifying_sample_ids_are_rejected_without_echoing_values(self) -> None:
        data = load_synthetic_results()
        data["samples"][0]["id"] = "jane.doe@example.com"

        with self.assertRaises(ResultsValidationError) as caught:
            validate_results(data)

        self.assertIn("synthetic-NNN identifier format", str(caught.exception))
        self.assertNotIn("jane.doe@example.com", str(caught.exception))

    def test_missing_required_fields_are_rejected(self) -> None:
        data = load_synthetic_results()
        del data["samples"][0]["insertions"]
        del data["totals"]["deletions"]

        with self.assertRaises(ResultsValidationError) as caught:
            validate_results(data)

        self.assertIn("samples[0] is missing required field(s)", caught.exception.errors)
        self.assertIn("samples[0].insertions must be a non-negative integer", caught.exception.errors)
        self.assertIn("totals is missing required field(s)", caught.exception.errors)

    def test_unknown_fields_are_rejected_at_root_and_sample_levels(self) -> None:
        data = load_synthetic_results()
        data["unexpected"] = "private-looking value"
        data["samples"][0]["notes"] = "private-looking value"

        with self.assertRaises(ResultsValidationError) as caught:
            validate_results(data)

        self.assertIn("root contains unsupported field(s)", caught.exception.errors)
        self.assertIn("samples[0] contains unsupported field(s)", caught.exception.errors)
        self.assertNotIn("private-looking value", str(caught.exception))

    def test_negative_sample_and_aggregate_counts_are_rejected(self) -> None:
        for field in ("reference_words", "substitutions", "deletions", "insertions"):
            with self.subTest(scope="sample", field=field):
                data = load_synthetic_results()
                data["samples"][0][field] = -1
                with self.assertRaises(ResultsValidationError):
                    validate_results(data)

        for field in ("utterances", "reference_words", "substitutions", "deletions", "insertions"):
            with self.subTest(scope="totals", field=field):
                data = load_synthetic_results()
                data["totals"][field] = -1
                with self.assertRaises(ResultsValidationError):
                    validate_results(data)

    def test_non_integer_and_boolean_counts_are_rejected(self) -> None:
        for invalid in (1.0, "1", True, False):
            with self.subTest(invalid=invalid):
                data = load_synthetic_results()
                data["samples"][0]["substitutions"] = invalid
                with self.assertRaisesRegex(ResultsValidationError, "must be a non-negative integer"):
                    validate_results(data)

    def test_empty_sample_collection_is_rejected(self) -> None:
        data = load_synthetic_results()
        data["samples"] = []
        data["totals"] = {
            "utterances": 0,
            "reference_words": 0,
            "substitutions": 0,
            "deletions": 0,
            "insertions": 0,
            "wer": 0,
        }

        with self.assertRaisesRegex(ResultsValidationError, "samples must be a non-empty list"):
            validate_results(data)

    def test_empty_reference_samples_are_rejected(self) -> None:
        data = load_synthetic_results()
        data["samples"][0].update(
            {"reference_words": 0, "substitutions": 0, "deletions": 0}
        )
        data["totals"].update(
            {"reference_words": 9, "substitutions": 0, "wer": 0.0}
        )

        with self.assertRaisesRegex(ResultsValidationError, "reference_words must be at least 1"):
            validate_results(data)

    def test_substitutions_and_deletions_cannot_exceed_reference_words(self) -> None:
        for counts in (
            {"substitutions": 2, "deletions": 0},
            {"substitutions": 0, "deletions": 2},
            {"substitutions": 1, "deletions": 1},
        ):
            with self.subTest(counts=counts):
                data = load_synthetic_results()
                data["samples"][0].update({"reference_words": 1, **counts})

                with self.assertRaisesRegex(ResultsValidationError, "cannot exceed reference_words"):
                    validate_results(data)

    def test_incorrect_aggregate_totals_are_rejected(self) -> None:
        data = load_synthetic_results()
        data["totals"]["substitutions"] = 0

        with self.assertRaisesRegex(ResultsValidationError, "totals.substitutions does not match"):
            validate_results(data)

    def test_incorrect_declared_wer_is_rejected(self) -> None:
        data = load_synthetic_results()
        data["totals"]["wer"] = 0.2

        with self.assertRaisesRegex(ResultsValidationError, "totals.wer does not match"):
            validate_results(data)

    def test_wer_over_one_hundred_percent_is_accepted(self) -> None:
        data = {
            "schema_version": 1,
            "result_type": "synthetic",
            "samples": [
                {
                    "id": "synthetic-003",
                    "reference_words": 1,
                    "substitutions": 0,
                    "deletions": 0,
                    "insertions": 2,
                }
            ],
            "totals": {
                "utterances": 1,
                "reference_words": 1,
                "substitutions": 0,
                "deletions": 0,
                "insertions": 2,
                "wer": 2.0,
            },
        }

        summary = validate_results(data)

        self.assertEqual(summary.wer, 2.0)

    def test_sensitive_content_fields_are_rejected_without_echoing_values(self) -> None:
        for field in ("transcript", "audio", "username", "user_id"):
            with self.subTest(field=field):
                data = load_synthetic_results()
                data["samples"][0][field] = "PRIVATE TEST CONTENT"

                with self.assertRaises(ResultsValidationError) as caught:
                    validate_results(data)

                self.assertIn("contains unsupported field(s)", str(caught.exception))
                self.assertNotIn("PRIVATE TEST CONTENT", str(caught.exception))

    def test_summary_is_deterministic_and_marks_benchmark_not_assessed(self) -> None:
        expected = "\n".join(
            (
                "Outflow ASR results validation passed",
                "Result type: SYNTHETIC",
                "Samples validated: 2",
                "Reference words: 10",
                "Substitutions: 1",
                "Deletions: 0",
                "Insertions: 0",
                "Corpus WER: 10.00%",
                "Real ASR benchmark status: NOT ASSESSED",
            )
        )
        first = format_summary(validate_results(load_synthetic_results()))
        second = format_summary(validate_results(load_synthetic_results()))

        self.assertEqual(first, expected)
        self.assertEqual(second, expected)

    def test_cli_accepts_fixture_without_creating_result_files(self) -> None:
        original_content = RESULTS_PATH.read_bytes()
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = main(["--file", str(RESULTS_PATH)])

        self.assertEqual(exit_code, 0)
        self.assertIn("Samples validated: 2", output.getvalue())
        self.assertIn("Corpus WER: 10.00%", output.getvalue())
        self.assertIn("Real ASR benchmark status: NOT ASSESSED", output.getvalue())
        self.assertEqual(RESULTS_PATH.read_bytes(), original_content)

        with tempfile.TemporaryDirectory() as temporary_dir:
            input_path = Path(temporary_dir) / "input.json"
            input_path.write_bytes(original_content)
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(main(["--file", str(input_path)]), 0)
            self.assertEqual(sorted(path.name for path in Path(temporary_dir).iterdir()), ["input.json"])

    def test_cli_does_not_echo_invalid_sensitive_content(self) -> None:
        data = load_synthetic_results()
        data["samples"][0]["transcript"] = "PRIVATE CLI TEST CONTENT"
        with tempfile.TemporaryDirectory() as temporary_dir:
            path = Path(temporary_dir) / "invalid.json"
            path.write_text(json.dumps(data), encoding="utf-8")
            output = io.StringIO()
            with contextlib.redirect_stderr(output):
                exit_code = main(["--file", str(path)])

        self.assertEqual(exit_code, 1)
        self.assertIn("Outflow ASR results validation failed", output.getvalue())
        self.assertNotIn("PRIVATE CLI TEST CONTENT", output.getvalue())

    def test_arithmetic_matches_existing_synthetic_corpus_scorer(self) -> None:
        summary = validate_results(load_synthetic_results())
        scorer_result = calculate_corpus_wer(
            [
                ("hello", "goodbye"),
                ("one two three four five six seven eight nine", "one two three four five six seven eight nine"),
            ]
        )

        self.assertEqual(summary.samples, scorer_result.utterances_scored)
        self.assertEqual(summary.reference_words, scorer_result.reference_word_count)
        self.assertEqual(summary.substitutions, scorer_result.substitutions)
        self.assertEqual(summary.deletions, scorer_result.deletions)
        self.assertEqual(summary.insertions, scorer_result.insertions)
        self.assertEqual(summary.wer, scorer_result.wer)

    def test_duplicate_json_keys_are_rejected_without_printing_content(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            path = Path(temporary_dir) / "duplicate.json"
            path.write_text('{"token": "PRIVATE", "token": "PRIVATE"}', encoding="utf-8")

            with self.assertRaisesRegex(ResultsValidationError, "duplicate object keys") as caught:
                load_results(path)

        self.assertNotIn("PRIVATE", str(caught.exception))

    def test_nonstandard_json_numbers_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            path = Path(temporary_dir) / "nonstandard.json"
            path.write_text('{"schema_version": NaN}', encoding="utf-8")

            with self.assertRaisesRegex(ResultsValidationError, "valid JSON"):
                load_results(path)

            path.write_text("[" * 10000 + "]" * 10000, encoding="utf-8")
            with self.assertRaisesRegex(ResultsValidationError, "valid JSON"):
                load_results(path)

    def test_wer_comparison_uses_documented_absolute_tolerance(self) -> None:
        data = copy.deepcopy(load_synthetic_results())
        data["totals"]["wer"] += 5e-10

        summary = validate_results(data)

        self.assertEqual(summary.wer, 0.1)

    def test_boolean_and_nonfinite_declared_wer_are_rejected(self) -> None:
        for invalid in (True, "0.1", -0.1, float("inf")):
            with self.subTest(invalid=invalid):
                data = load_synthetic_results()
                data["totals"]["wer"] = invalid
                with self.assertRaisesRegex(ResultsValidationError, "totals.wer must be"):
                    validate_results(data)


if __name__ == "__main__":
    unittest.main()
