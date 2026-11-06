"""Tests for separately supplied AC-07 actual-output identity binding."""

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

from scripts import validate_cleanup_ac07_actual_outputs as validator


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
TESTDATA_DIR = REPOSITORY_ROOT / "scripts" / "testdata"
MANIFEST_PATH = TESTDATA_DIR / "synthetic_ac07_manifest.json"
REFERENCE_PATH = TESTDATA_DIR / "synthetic_ac07_reference_bundle.json"
ANNOTATION_PATH = TESTDATA_DIR / "synthetic_ac07_protected_annotations.json"
ACTUAL_PATH = TESTDATA_DIR / "synthetic_ac07_actual_outputs.json"


def load_json(path: Path) -> dict[str, object]:
    """Read a checked-in synthetic artifact for in-memory mutation tests."""
    return json.loads(path.read_text(encoding="utf-8"))


def load_manifest() -> dict[str, object]:
    return load_json(MANIFEST_PATH)


def load_references() -> dict[str, object]:
    return load_json(REFERENCE_PATH)


def load_annotations() -> dict[str, object]:
    return load_json(ANNOTATION_PATH)


def load_actuals() -> dict[str, object]:
    return load_json(ACTUAL_PATH)


def validate(
    manifest: object | None = None,
    references: object | None = None,
    annotations: object | None = None,
    actuals: object | None = None,
) -> validator.CleanupActualOutputBindingSummary:
    return validator.validate_cleanup_actual_outputs(
        load_manifest() if manifest is None else manifest,
        load_references() if references is None else references,
        load_annotations() if annotations is None else annotations,
        load_actuals() if actuals is None else actuals,
    )


class CleanupActualOutputBindingTests(unittest.TestCase):
    def test_valid_three_case_synthetic_bundle(self) -> None:
        result = validate()

        self.assertEqual(result.corpus_id, "ac07-synthetic-manifest-demo")
        self.assertEqual(result.revision, "draft-001")
        self.assertEqual(result.run_id, "synthetic-run-001")
        self.assertEqual(result.declared_source_kind, "synthetic_demo")
        self.assertEqual(result.reference_case_count, 3)
        self.assertEqual(result.actual_output_case_count, 3)
        self.assertEqual(result.matched_case_id_count, 3)
        self.assertEqual(result.valid_utf8_actual_string_count, 3)
        self.assertTrue(result.full_case_correspondence_verified)
        self.assertFalse(result.production_execution_independently_verified)

    def test_actuals_are_separate_from_expected_strings(self) -> None:
        references = load_references()
        actuals = load_actuals()

        result = validate(references=references, actuals=actuals)

        self.assertTrue(result.full_case_correspondence_verified)
        self.assertNotEqual(
            actuals["cases"][0]["actual"], references["cases"][0]["expected"]
        )

    def test_reordered_actual_cases_match_by_exact_id(self) -> None:
        actuals = load_actuals()
        actuals["cases"] = list(reversed(actuals["cases"]))

        result = validate(actuals=actuals)

        self.assertEqual(result.matched_case_id_count, 3)
        self.assertTrue(result.full_case_correspondence_verified)

    def test_complete_id_correspondence_is_reported(self) -> None:
        result = validate()

        self.assertEqual(
            result.matched_case_id_count, result.reference_case_count
        )
        self.assertEqual(
            result.actual_output_case_count, result.reference_case_count
        )

    def test_missing_case_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["cases"].pop()

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "reference case 3: field 'id'.*no matching actual-output",
        ):
            validate(actuals=actuals)

    def test_extra_case_is_rejected(self) -> None:
        actuals = load_actuals()
        extra = copy.deepcopy(actuals["cases"][0])
        extra["id"] = "extra-case"
        actuals["cases"].append(extra)

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "actual-output bundle case 4: field 'id' is not declared",
        ):
            validate(actuals=actuals)

    def test_duplicate_case_id_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["cases"][1]["id"] = actuals["cases"][0]["id"]

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "actual-output bundle case 2: field 'id' duplicates",
        ):
            validate(actuals=actuals)

    def test_empty_case_collection_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["cases"] = []

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "field 'cases' must be a nonempty array",
        ):
            validate(actuals=actuals)

    def test_empty_case_id_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["cases"][0]["id"] = " \t"

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "actual-output bundle case 1: field 'id' must be a nonempty string",
        ):
            validate(actuals=actuals)

    def test_case_id_matching_is_case_sensitive(self) -> None:
        actuals = load_actuals()
        actuals["cases"][0]["id"] = "DEMO-001"

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "actual-output bundle case 1: field 'id' is not declared",
        ):
            validate(actuals=actuals)

    def test_corpus_id_mismatch_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["corpus_id"] = "another-corpus"

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError, "field 'corpus_id'"
        ):
            validate(actuals=actuals)

    def test_revision_mismatch_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["revision"] = "draft-002"

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError, "field 'revision'"
        ):
            validate(actuals=actuals)

    def test_unsupported_schema_version_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["schema_version"] = 2

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "field 'schema_version' must equal integer 1",
        ):
            validate(actuals=actuals)

    def test_boolean_schema_version_is_not_accepted_as_integer_one(self) -> None:
        actuals = load_actuals()
        actuals["schema_version"] = True

        with self.assertRaises(validator.CleanupActualOutputBindingError):
            validate(actuals=actuals)

    def test_missing_required_field_is_rejected(self) -> None:
        actuals = load_actuals()
        del actuals["run_id"]

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "missing required field 'run_id'",
        ):
            validate(actuals=actuals)

    def test_unknown_field_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["expected"] = "not allowed"

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "actual-output bundle: object contains an unsupported field",
        ):
            validate(actuals=actuals)

    def test_empty_or_invalid_identity_strings_are_rejected(self) -> None:
        for field in ("corpus_id", "revision", "run_id"):
            for invalid in ("", " \t", "\ud800"):
                with self.subTest(field=field, invalid=repr(invalid)):
                    actuals = load_actuals()
                    actuals[field] = invalid

                    with self.assertRaises(
                        validator.CleanupActualOutputBindingError
                    ):
                        validate(actuals=actuals)

    def test_unsupported_source_kind_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["source_kind"] = "unknown"

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "field 'source_kind' is unsupported",
        ):
            validate(actuals=actuals)

    def test_synthetic_execution_evidence_must_be_null(self) -> None:
        actuals = load_actuals()
        actuals["execution_evidence_ref"] = "synthetic-proof"

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "field 'execution_evidence_ref' must be null",
        ):
            validate(actuals=actuals)

    def test_production_declaration_requires_nonempty_evidence_reference(self) -> None:
        for evidence_ref in (None, "", " \t"):
            with self.subTest(evidence_ref=repr(evidence_ref)):
                actuals = load_actuals()
                actuals["source_kind"] = "production_declared"
                actuals["execution_evidence_ref"] = evidence_ref

                with self.assertRaisesRegex(
                    validator.CleanupActualOutputBindingError,
                    "field 'execution_evidence_ref' must be a nonempty string",
                ):
                    validate(actuals=actuals)

    def test_production_declaration_evidence_must_be_utf8(self) -> None:
        actuals = load_actuals()
        actuals["source_kind"] = "production_declared"
        actuals["execution_evidence_ref"] = "\ud800"

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "field 'execution_evidence_ref' must be valid UTF-8",
        ):
            validate(actuals=actuals)

    def test_synthetic_manifest_rejects_production_declaration(self) -> None:
        actuals = load_actuals()
        actuals["source_kind"] = "production_declared"
        actuals["execution_evidence_ref"] = "declared-run-reference"

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "source_kind' contradicts synthetic_demo manifest purpose",
        ):
            validate(actuals=actuals)

    def test_acceptance_candidate_production_declaration_remains_unverified(
        self,
    ) -> None:
        manifest = load_manifest()
        manifest["purpose"] = "acceptance_candidate"
        actuals = load_actuals()
        actuals["source_kind"] = "production_declared"
        actuals["execution_evidence_ref"] = "declared-run-reference"

        result = validate(manifest=manifest, actuals=actuals)

        self.assertEqual(result.declared_source_kind, "production_declared")
        self.assertFalse(result.production_execution_independently_verified)

    def test_empty_actual_output_is_valid(self) -> None:
        actuals = load_actuals()
        actuals["cases"][0]["actual"] = ""

        result = validate(actuals=actuals)

        self.assertEqual(result.valid_utf8_actual_string_count, 3)

    def test_non_string_actual_output_is_rejected(self) -> None:
        actuals = load_actuals()
        actuals["cases"][0]["actual"] = 7

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "actual-output bundle case 1: field 'actual' must be a string",
        ):
            validate(actuals=actuals)

    def test_invalid_utf8_actual_output_is_rejected_without_echoing_content(
        self,
    ) -> None:
        actuals = load_actuals()
        invalid_text = "private-synthetic-marker-\ud800"
        actuals["cases"][0]["actual"] = invalid_text

        with self.assertRaisesRegex(
            validator.CleanupActualOutputBindingError,
            "actual-output bundle case 1: field 'actual' must be valid UTF-8",
        ) as raised:
            validate(actuals=actuals)

        self.assertNotIn("private-synthetic-marker", str(raised.exception))

    def test_unicode_whitespace_and_punctuation_are_preserved_exactly(self) -> None:
        actuals = load_actuals()
        authored_value = "\tÉlodie  — line two\n!  "
        actuals["cases"][0]["actual"] = authored_value

        result = validate(actuals=actuals)

        self.assertEqual(actuals["cases"][0]["actual"], authored_value)
        self.assertEqual(result.valid_utf8_actual_string_count, 3)
        self.assertNotIn("actual", result.__dataclass_fields__)

    def test_duplicate_json_object_keys_are_rejected_from_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            path = Path(temporary_directory) / "actuals.json"
            path.write_text(
                '{"schema_version":1,"corpus_id":"first",'
                '"corpus_id":"second"}',
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                validator.CleanupActualOutputBindingError,
                "duplicate JSON field 'corpus_id'",
            ):
                validator._load_json(path, "actual-output bundle")

    def test_duplicate_json_case_keys_are_rejected_from_files(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            path = Path(temporary_directory) / "actuals.json"
            path.write_text(
                '{"cases":[{"id":"demo-001","id":"demo-002"}]}',
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                validator.CleanupActualOutputBindingError,
                "actual-output bundle case 1: duplicate JSON field 'id'",
            ):
                validator._load_json(path, "actual-output bundle")

    def test_malformed_json_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            path = Path(temporary_directory) / "actuals.json"
            path.write_text("{", encoding="utf-8")

            with self.assertRaisesRegex(
                validator.CleanupActualOutputBindingError,
                "actual-output bundle JSON is invalid at line 1",
            ):
                validator._load_json(path, "actual-output bundle")

    def test_nonstandard_json_constant_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            path = Path(temporary_directory) / "actuals.json"
            path.write_text('{"actual":NaN}', encoding="utf-8")

            with self.assertRaisesRegex(
                validator.CleanupActualOutputBindingError,
                "actual-output bundle JSON contains an unsupported value",
            ):
                validator._load_json(path, "actual-output bundle")

    def test_reference_hash_mismatch_fails_before_actual_validation(self) -> None:
        references = load_references()
        references["cases"][0]["expected"] += " changed"

        with mock.patch.object(
            validator, "_validate_actual_outputs_after_reference_binding"
        ) as actual_validation:
            with self.assertRaisesRegex(
                validator.CleanupActualOutputBindingError,
                "reference bundle case 1: field 'expected' SHA-256",
            ):
                validate(references=references, actuals=object())

        actual_validation.assert_not_called()

    def test_invalid_annotations_fail_before_actual_validation(self) -> None:
        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"][0]["output_bytes"] = [
            900,
            903,
        ]

        with mock.patch.object(
            validator, "_validate_actual_outputs_after_reference_binding"
        ) as actual_validation:
            with self.assertRaises(
                validator.CleanupActualOutputBindingError
            ):
                validate(annotations=annotations, actuals=object())

        actual_validation.assert_not_called()

    def test_repeated_validation_is_deterministic(self) -> None:
        self.assertEqual(validate(), validate())

    def test_result_is_immutable(self) -> None:
        result = validate()

        with self.assertRaises(FrozenInstanceError):
            result.run_id = "changed"  # type: ignore[misc]

    def test_production_verification_field_cannot_be_set_by_constructor(self) -> None:
        result = validator.CleanupActualOutputBindingSummary(
            corpus_id="synthetic",
            revision="draft",
            run_id="run",
            declared_source_kind="production_declared",
            reference_case_count=1,
            actual_output_case_count=1,
            matched_case_id_count=1,
            valid_utf8_actual_string_count=1,
            full_case_correspondence_verified=True,
        )

        self.assertFalse(result.production_execution_independently_verified)

    def test_failure_returns_no_partial_summary(self) -> None:
        actuals = load_actuals()
        actuals["cases"][0]["actual"] = None
        summary_constructor = validator.CleanupActualOutputBindingSummary

        with mock.patch.object(
            validator,
            "CleanupActualOutputBindingSummary",
            wraps=summary_constructor,
        ) as constructor:
            with self.assertRaises(
                validator.CleanupActualOutputBindingError
            ):
                validate(actuals=actuals)

        constructor.assert_not_called()

    def test_demo_reads_only_four_checked_in_synthetic_artifacts(self) -> None:
        output = io.StringIO()
        with (
            mock.patch.object(
                validator, "_load_json", wraps=validator._load_json
            ) as load_json_spy,
            contextlib.redirect_stdout(output),
        ):
            exit_code = validator.main(["--demo"])

        self.assertEqual(exit_code, 0)
        self.assertEqual(
            [call.args[0].name for call in load_json_spy.call_args_list],
            [
                "synthetic_ac07_manifest.json",
                "synthetic_ac07_reference_bundle.json",
                "synthetic_ac07_protected_annotations.json",
                "synthetic_ac07_actual_outputs.json",
            ],
        )

    def test_demo_does_not_expose_case_content(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            validator.main(["--demo"])

        rendered = output.getvalue()
        for case in load_actuals()["cases"]:
            self.assertNotIn(case["actual"], rendered)
        for case in load_references()["cases"]:
            self.assertNotIn(case["input"], rendered)
            self.assertNotIn(case["expected"], rendered)
        self.assertNotIn("Protected numeric references:", rendered)
        self.assertNotIn("Protected proper-noun references:", rendered)

    def test_demo_states_production_unverified_and_acceptance_unevaluated(
        self,
    ) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = validator.main(["--demo"])

        self.assertEqual(exit_code, 0)
        self.assertIn("Production execution: NOT VERIFIED\n", output.getvalue())
        self.assertIn("AC-07 acceptance: NOT EVALUATED\n", output.getvalue())
        self.assertNotIn("AC-07 acceptance: PASS", output.getvalue())


if __name__ == "__main__":
    unittest.main()
