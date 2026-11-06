"""Tests for validated AC-07 bundle assembly and shared scoring."""

from __future__ import annotations

import contextlib
import copy
import io
import json
import tempfile
import unittest
from dataclasses import FrozenInstanceError
from pathlib import Path
from unittest import mock

from scripts import evaluate_cleanup_ac07 as coordinator
from scripts import score_cleanup_ac07_bundles as bundle_scorer
from scripts import validate_cleanup_ac07_actual_outputs as output_validator


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
TESTDATA_DIR = REPOSITORY_ROOT / "scripts" / "testdata"


def load_json(path: Path) -> dict[str, object]:
    return json.loads(path.read_text(encoding="utf-8"))


def load_bundles() -> tuple[dict[str, object], ...]:
    return (
        load_json(TESTDATA_DIR / "synthetic_ac07_manifest.json"),
        load_json(TESTDATA_DIR / "synthetic_ac07_reference_bundle.json"),
        load_json(TESTDATA_DIR / "synthetic_ac07_protected_annotations.json"),
        load_json(TESTDATA_DIR / "synthetic_ac07_actual_outputs.json"),
    )


def score(bundles: tuple[object, object, object, object] | None = None):
    return bundle_scorer.score_verified_cleanup_bundles(
        *(load_bundles() if bundles is None else bundles)
    )


class VerifiedCleanupBundleScoringTests(unittest.TestCase):
    def test_successfully_assembles_all_three_valid_cases(self) -> None:
        result = score()

        self.assertEqual(result.assembled_case_count, 3)
        self.assertTrue(
            result.actual_output_binding_summary.full_case_correspondence_verified
        )
        self.assertEqual(
            result.evaluation_result.exact_output_score.total_cases, 3
        )
        self.assertEqual(
            result.evaluation_result.protected_token_score.total_cases, 3
        )

    def test_exact_id_records_are_passed_to_existing_coordinator(self) -> None:
        manifest, references, annotations, actuals = load_bundles()
        expected_by_id = {
            case["id"]: case for case in references["cases"]
        }
        annotation_by_id = {
            case["id"]: case for case in annotations["cases"]
        }
        actual_by_id = {case["id"]: case for case in actuals["cases"]}
        expected_records = [
            {
                "id": case["id"],
                "input": expected_by_id[case["id"]]["input"],
                "expected": expected_by_id[case["id"]]["expected"],
                "actual": actual_by_id[case["id"]]["actual"],
                "protected_tokens": annotation_by_id[case["id"]][
                    "protected_tokens"
                ],
            }
            for case in manifest["cases"]
        ]

        with mock.patch.object(
            bundle_scorer,
            "evaluate_cleanup_ac07_cases",
            wraps=coordinator.evaluate_cleanup_ac07_cases,
        ) as evaluate:
            score((manifest, references, annotations, actuals))

        self.assertEqual(evaluate.call_count, 1)
        self.assertEqual(evaluate.call_args.args[0], expected_records)
        self.assertEqual(
            [record["id"] for record in evaluate.call_args.args[0]],
            [case["id"] for case in manifest["cases"]],
        )

    def test_reordering_reference_annotation_and_actual_arrays_is_stable(self) -> None:
        original = load_bundles()
        reordered = tuple(copy.deepcopy(bundle) for bundle in original)
        for bundle in reordered[1:]:
            bundle["cases"] = list(reversed(bundle["cases"]))

        self.assertEqual(score(original), score(reordered))

    def test_manifest_order_controls_assembled_record_order(self) -> None:
        manifest, references, annotations, actuals = load_bundles()
        manifest["cases"] = list(reversed(manifest["cases"]))
        with mock.patch.object(
            bundle_scorer,
            "evaluate_cleanup_ac07_cases",
            wraps=coordinator.evaluate_cleanup_ac07_cases,
        ) as evaluate:
            score((manifest, references, annotations, actuals))

        assembled_ids = [record["id"] for record in evaluate.call_args.args[0]]
        self.assertEqual(
            assembled_ids, [case["id"] for case in manifest["cases"]]
        )

    def test_checked_in_synthetic_artifacts_produce_zero_of_three_matches(self) -> None:
        result = score()
        exact = result.evaluation_result.exact_output_score

        self.assertEqual(exact.total_cases, 3)
        self.assertEqual(exact.exact_matches, 0)
        self.assertEqual(exact.mismatches, 3)
        self.assertEqual(exact.exact_output_percentage, 0.0)

    def test_all_three_synthetic_mismatches_are_included_in_score(self) -> None:
        result = score()

        self.assertEqual(result.assembled_case_count, 3)
        self.assertEqual(result.evaluation_result.exact_output_score.mismatches, 3)
        self.assertEqual(
            result.evaluation_result.protected_token_score.total_cases, 3
        )

    def test_synthetic_numeric_reference_mutation_is_counted(self) -> None:
        result = score()
        protected = result.evaluation_result.protected_token_score

        self.assertEqual(protected.protected_number_tokens, 1)
        self.assertEqual(protected.altered_number_tokens, 1)

    def test_synthetic_proper_noun_reference_mutation_is_counted(self) -> None:
        result = score()
        protected = result.evaluation_result.protected_token_score

        self.assertEqual(protected.protected_proper_noun_tokens, 1)
        self.assertEqual(protected.altered_proper_noun_tokens, 1)

    def test_three_cases_fail_existing_minimum_corpus_gate(self) -> None:
        self.assertFalse(
            score().evaluation_result.exact_output_score.minimum_corpus_size_met
        )

    def test_zero_exact_matches_fail_existing_threshold(self) -> None:
        self.assertFalse(
            score().evaluation_result.exact_output_score.exact_output_threshold_met
        )

    def test_protected_gate_fails_independently_of_exact_output_score(self) -> None:
        result = score()

        self.assertFalse(
            result.evaluation_result.protected_token_score.protected_token_gate_passes
        )
        self.assertEqual(result.evaluation_result.exact_output_score.mismatches, 3)

    def test_cases_without_annotations_are_still_scored(self) -> None:
        manifest, references, annotations, actuals = load_bundles()
        annotations["cases"][1]["protected_tokens"] = []

        with mock.patch.object(
            bundle_scorer,
            "evaluate_cleanup_ac07_cases",
            wraps=coordinator.evaluate_cleanup_ac07_cases,
        ) as evaluate:
            result = score((manifest, references, annotations, actuals))

        records = evaluate.call_args.args[0]
        self.assertEqual(len(records), 3)
        self.assertEqual(records[1]["protected_tokens"], [])
        self.assertEqual(result.evaluation_result.exact_output_score.total_cases, 3)
        self.assertEqual(result.evaluation_result.protected_token_score.total_cases, 3)

    def test_missing_actual_id_fails_before_scoring(self) -> None:
        bundles = list(load_bundles())
        bundles[3]["cases"].pop()
        with mock.patch.object(bundle_scorer, "evaluate_cleanup_ac07_cases") as evaluate:
            with self.assertRaisesRegex(
                output_validator.CleanupActualOutputBindingError,
                "no matching actual-output",
            ):
                score(tuple(bundles))
        evaluate.assert_not_called()

    def test_duplicate_actual_id_fails_before_scoring(self) -> None:
        bundles = list(load_bundles())
        bundles[3]["cases"][1]["id"] = bundles[3]["cases"][0]["id"]
        with mock.patch.object(bundle_scorer, "evaluate_cleanup_ac07_cases") as evaluate:
            with self.assertRaisesRegex(
                output_validator.CleanupActualOutputBindingError, "duplicates"
            ):
                score(tuple(bundles))
        evaluate.assert_not_called()

    def test_reference_hash_corruption_fails_before_scoring(self) -> None:
        bundles = list(load_bundles())
        bundles[1]["cases"][0]["input"] += " changed"
        with mock.patch.object(bundle_scorer, "evaluate_cleanup_ac07_cases") as evaluate:
            with self.assertRaisesRegex(
                output_validator.CleanupActualOutputBindingError, "SHA-256"
            ):
                score(tuple(bundles))
        evaluate.assert_not_called()

    def test_invalid_protected_annotation_fails_before_scoring(self) -> None:
        bundles = list(load_bundles())
        bundles[2]["cases"][0]["protected_tokens"][0]["kind"] = "date"
        with mock.patch.object(bundle_scorer, "evaluate_cleanup_ac07_cases") as evaluate:
            with self.assertRaises(output_validator.CleanupActualOutputBindingError):
                score(tuple(bundles))
        evaluate.assert_not_called()

    def test_invalid_utf8_actual_output_fails_before_scoring(self) -> None:
        bundles = list(load_bundles())
        bundles[3]["cases"][0]["actual"] = "invalid-\ud800"
        with mock.patch.object(bundle_scorer, "evaluate_cleanup_ac07_cases") as evaluate:
            with self.assertRaisesRegex(
                output_validator.CleanupActualOutputBindingError, "valid UTF-8"
            ):
                score(tuple(bundles))
        evaluate.assert_not_called()

    def test_production_source_declaration_remains_unverified(self) -> None:
        bundles = list(load_bundles())
        bundles[0]["purpose"] = "acceptance_candidate"
        bundles[0]["review_status"] = "reviewed"
        bundles[0]["review_evidence_ref"] = "declared-review-reference"
        bundles[3]["source_kind"] = "production_declared"
        bundles[3]["execution_evidence_ref"] = "declared-run-reference"

        result = score(tuple(bundles))

        self.assertEqual(
            result.actual_output_binding_summary.declared_source_kind,
            "production_declared",
        )
        self.assertFalse(
            result.actual_output_binding_summary.production_execution_independently_verified
        )

    def test_explicit_matching_in_memory_outputs_can_match(self) -> None:
        bundles = list(load_bundles())
        expected_by_id = {
            case["id"]: case["expected"] for case in bundles[1]["cases"]
        }
        for actual_case in bundles[3]["cases"]:
            actual_case["actual"] = expected_by_id[actual_case["id"]]

        result = score(tuple(bundles))
        exact = result.evaluation_result.exact_output_score
        protected = result.evaluation_result.protected_token_score

        self.assertEqual(exact.exact_matches, 3)
        self.assertEqual(exact.mismatches, 0)
        self.assertTrue(protected.protected_token_gate_passes)
        self.assertEqual(protected.altered_number_tokens, 0)
        self.assertEqual(protected.altered_proper_noun_tokens, 0)

    def test_mismatch_outside_protected_annotations_does_not_alter_tokens(self) -> None:
        bundles = list(load_bundles())
        references_by_id = {
            case["id"]: case for case in bundles[1]["cases"]
        }
        for actual_case in bundles[3]["cases"]:
            actual_case["actual"] = references_by_id[actual_case["id"]][
                "expected"
            ]
        bundles[3]["cases"][1]["actual"] += "!"

        result = score(tuple(bundles))
        exact = result.evaluation_result.exact_output_score
        protected = result.evaluation_result.protected_token_score

        self.assertEqual(exact.exact_matches, 2)
        self.assertEqual(exact.mismatches, 1)
        self.assertEqual(protected.altered_number_tokens, 0)
        self.assertEqual(protected.altered_proper_noun_tokens, 0)
        self.assertTrue(protected.protected_token_gate_passes)

    def test_repeated_evaluations_are_deterministic(self) -> None:
        self.assertEqual(score(), score())

    def test_returned_result_is_immutable(self) -> None:
        result = score()

        with self.assertRaises(FrozenInstanceError):
            result.assembled_case_count = 4  # type: ignore[misc]

    def test_failed_validation_returns_no_partial_result_and_never_scores(self) -> None:
        bundles = list(load_bundles())
        bundles[3]["cases"].pop()
        returned_results: list[object] = []
        with mock.patch.object(bundle_scorer, "evaluate_cleanup_ac07_cases") as evaluate:
            with self.assertRaises(
                output_validator.CleanupActualOutputBindingError
            ):
                returned_results.append(score(tuple(bundles)))
        self.assertEqual(returned_results, [])
        evaluate.assert_not_called()

    def test_demo_prints_counts_and_statuses_without_source_content(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            self.assertEqual(bundle_scorer.main(["--demo"]), 0)

        rendered = output.getvalue()
        self.assertEqual(
            rendered,
            "AC-07 verified-bundle scoring: SYNTHETIC DEMO\n"
            "Corpus ID: ac07-synthetic-manifest-demo\n"
            "Revision: draft-001\n"
            "Run ID: synthetic-run-001\n"
            "Verified case associations: 3/3\n"
            "Exact matches: 0\n"
            "Mismatches: 3\n"
            "Exact-output rate: 0.00%\n"
            "Minimum corpus size: FAIL\n"
            "Exact-output threshold: FAIL\n"
            "Protected numeric tokens: 1 (1 altered)\n"
            "Protected proper nouns: 1 (1 altered)\n"
            "Protected-token gate: FAIL\n"
            "Output source: SYNTHETIC DEMO\n"
            "Production execution: NOT VERIFIED\n"
            "Reference review: PENDING\n"
            "Corpus approval: NOT ESTABLISHED\n"
            "AC-07 acceptance: NOT EVALUATED\n",
        )
        for source_text in (
            "AC-07 synthetic input bytes for demo-001",
            "AC-07 synthetic expected bytes for demo-001",
            "Independent synthetic output for demo-001",
        ):
            self.assertNotIn(source_text, rendered)
        self.assertNotRegex(rendered, r"(?<![\w-])001(?![\w-])")

    def test_demo_strict_loader_rejects_duplicate_malformed_and_nonstandard_json(self) -> None:
        invalid_documents = (
            '{"schema_version": 1, "schema_version": 1}',
            '{"schema_version": 1,',
            '{"value": NaN}',
        )
        for document in invalid_documents:
            with self.subTest(document=document):
                with tempfile.NamedTemporaryFile(
                    mode="w", encoding="utf-8", suffix=".json"
                ) as temp_file:
                    temp_file.write(document)
                    temp_file.flush()
                    with self.assertRaises(
                        output_validator.CleanupActualOutputBindingError
                    ):
                        output_validator._load_json(temp_file.name, "test bundle")

    def test_demo_loader_reads_only_four_existing_synthetic_artifacts(self) -> None:
        original_loader = output_validator._load_json
        with mock.patch.object(
            output_validator, "_load_json", wraps=original_loader
        ) as strict_loader:
            bundle_scorer._load_synthetic_bundles()

        self.assertEqual(strict_loader.call_count, 4)
        loaded_paths = [Path(call.args[0]).name for call in strict_loader.call_args_list]
        self.assertEqual(
            loaded_paths,
            [
                "synthetic_ac07_manifest.json",
                "synthetic_ac07_reference_bundle.json",
                "synthetic_ac07_protected_annotations.json",
                "synthetic_ac07_actual_outputs.json",
            ],
        )


if __name__ == "__main__":
    unittest.main()
