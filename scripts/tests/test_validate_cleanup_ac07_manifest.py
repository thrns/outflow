"""Tests for the metadata-only AC-07 corpus manifest validator."""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import tempfile
import unittest
from dataclasses import FrozenInstanceError
from pathlib import Path

from scripts import validate_cleanup_ac07_manifest as manifest_validator


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
SAMPLE_PATH = (
    REPOSITORY_ROOT / "scripts" / "testdata" / "synthetic_ac07_manifest.json"
)
SYNTHETIC_HASH_SOURCES = {
    "demo-001": (
        "AC-07 synthetic input bytes for demo-001",
        "AC-07 synthetic expected bytes for demo-001",
    ),
    "demo-002": (
        "AC-07 synthetic input bytes for demo-002",
        "AC-07 synthetic expected bytes for demo-002",
    ),
    "demo-003": (
        "AC-07 synthetic input bytes for demo-003",
        "AC-07 synthetic expected bytes for demo-003",
    ),
}


def load_sample_data() -> dict[str, object]:
    """Load the small checked-in metadata sample for test mutations."""
    return json.loads(SAMPLE_PATH.read_text(encoding="utf-8"))


class CleanupManifestValidationTests(unittest.TestCase):
    def test_valid_synthetic_draft_and_genuine_hashes(self) -> None:
        data = load_sample_data()
        result = manifest_validator.validate_cleanup_manifest(data)

        self.assertTrue(result.syntactically_valid)
        self.assertEqual(result.corpus_id, "ac07-synthetic-manifest-demo")
        self.assertEqual(result.purpose, "synthetic_demo")
        self.assertEqual(result.review_status, "pending")
        self.assertEqual(result.indexed_case_count, 3)

        for case in data["cases"]:
            input_source, expected_source = SYNTHETIC_HASH_SOURCES[case["id"]]
            self.assertEqual(
                case["input_sha256"],
                hashlib.sha256(input_source.encode("utf-8")).hexdigest(),
            )
            self.assertEqual(
                case["expected_sha256"],
                hashlib.sha256(expected_source.encode("utf-8")).hexdigest(),
            )

    def test_repeated_validation_is_deterministic(self) -> None:
        data = load_sample_data()
        first = manifest_validator.validate_cleanup_manifest(data)
        second = manifest_validator.validate_cleanup_manifest(data)

        self.assertEqual(first, second)

    def test_result_is_immutable(self) -> None:
        result = manifest_validator.validate_cleanup_manifest(load_sample_data())

        with self.assertRaises(FrozenInstanceError):
            result.corpus_id = "changed"  # type: ignore[misc]

    def test_unsupported_schema_version_is_rejected(self) -> None:
        data = load_sample_data()
        data["schema_version"] = 2

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "schema_version"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_missing_required_field_is_rejected(self) -> None:
        data = load_sample_data()
        del data["revision"]

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "revision"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_unknown_field_is_rejected(self) -> None:
        data = load_sample_data()
        data["unexpected"] = "synthetic metadata"

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "unsupported field"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_errors_identify_case_and_field_without_echoing_case_values(self) -> None:
        data = load_sample_data()
        data["cases"][0]["category"] = "synthetic secret marker"

        with self.assertRaises(
            manifest_validator.CleanupManifestValidationError
        ) as context:
            manifest_validator.validate_cleanup_manifest(data)

        message = str(context.exception)
        self.assertIn("case 1", message)
        self.assertIn("category", message)
        self.assertNotIn("synthetic secret marker", message)

    def test_duplicate_case_id_is_rejected(self) -> None:
        data = load_sample_data()
        data["cases"][1]["id"] = data["cases"][0]["id"]

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "case 2.*id"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_invalid_category_is_rejected(self) -> None:
        data = load_sample_data()
        data["cases"][0]["category"] = "not-a-category"

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "category"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_invalid_mode_is_rejected(self) -> None:
        data = load_sample_data()
        data["cases"][0]["mode"] = "CLEAN"

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "mode"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_nonboolean_setting_is_rejected(self) -> None:
        data = load_sample_data()
        data["cases"][0]["remove_fillers"] = 1

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "remove_fillers"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_malformed_and_uppercase_hashes_are_rejected(self) -> None:
        for invalid_hash in (
            "not-a-digest",
            "A" * 64,
            "a" * 63,
            "g" * 64,
        ):
            with self.subTest(invalid_hash=invalid_hash[:12]):
                data = load_sample_data()
                data["cases"][0]["input_sha256"] = invalid_hash
                with self.assertRaisesRegex(
                    manifest_validator.CleanupManifestValidationError,
                    "input_sha256",
                ):
                    manifest_validator.validate_cleanup_manifest(data)

    def test_missing_provenance_reference_is_rejected(self) -> None:
        data = load_sample_data()
        data["cases"][0]["provenance_ref"] = "  "

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "provenance_ref"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_duplicate_protected_kind_is_rejected(self) -> None:
        data = load_sample_data()
        data["cases"][0]["protected_kinds"] = ["number", "number"]

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "duplicate"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_unsupported_protected_kind_is_rejected(self) -> None:
        data = load_sample_data()
        data["cases"][0]["protected_kinds"] = ["email"]

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "protected_kinds"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_synthetic_demo_cannot_claim_review_approval(self) -> None:
        reviewed_corpus = load_sample_data()
        reviewed_corpus["review_status"] = "reviewed"
        reviewed_corpus["review_evidence_ref"] = "review-record:synthetic"
        with self.assertRaises(
            manifest_validator.CleanupManifestValidationError
        ):
            manifest_validator.validate_cleanup_manifest(reviewed_corpus)

        reviewed_reference = load_sample_data()
        reviewed_reference["cases"][0]["reference_review_status"] = "reviewed"
        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "reviewed references"
        ):
            manifest_validator.validate_cleanup_manifest(reviewed_reference)

    def test_reviewed_candidate_requires_evidence_reference(self) -> None:
        data = load_sample_data()
        data["purpose"] = "acceptance_candidate"
        data["review_status"] = "reviewed"

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError,
            "review_evidence_ref",
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_reviewed_candidate_records_declarations_without_authenticating_them(
        self,
    ) -> None:
        data = load_sample_data()
        data["purpose"] = "acceptance_candidate"
        data["review_status"] = "reviewed"
        data["review_evidence_ref"] = "review-record:separate-evidence"
        data["cases"][0]["reference_review_status"] = "reviewed"

        result = manifest_validator.validate_cleanup_manifest(data)

        self.assertEqual(result.review_status, "reviewed")
        self.assertEqual(result.pending_reference_review_case_count, 2)
        self.assertTrue(result.syntactically_valid)

    def test_duplicate_json_object_keys_are_rejected(self) -> None:
        duplicate_json = '{"schema_version":1,"schema_version":1}'
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "duplicate.json"
            path.write_text(duplicate_json, encoding="utf-8")

            with self.assertRaisesRegex(
                manifest_validator.CleanupManifestValidationError,
                "duplicate JSON field 'schema_version'",
            ):
                manifest_validator.load_cleanup_manifest(path)

    def test_empty_case_collection_is_rejected(self) -> None:
        data = load_sample_data()
        data["cases"] = []

        with self.assertRaisesRegex(
            manifest_validator.CleanupManifestValidationError, "nonempty array"
        ):
            manifest_validator.validate_cleanup_manifest(data)

    def test_counts_modes_categories_and_declared_protection_kinds(self) -> None:
        result = manifest_validator.validate_cleanup_manifest(load_sample_data())

        self.assertEqual(result.indexed_case_count, 3)
        self.assertEqual(result.unique_case_count, 3)
        self.assertEqual(result.represented_category_count, 3)
        self.assertEqual(result.clean_case_count, 2)
        self.assertEqual(result.verbatim_case_count, 1)
        self.assertEqual(result.numeric_protection_case_count, 1)
        self.assertEqual(result.proper_noun_protection_case_count, 1)
        self.assertEqual(result.pending_reference_review_case_count, 3)

    def test_demo_output_states_no_approval_or_acceptance(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = manifest_validator.main(["--demo"])

        self.assertEqual(status, 0)
        self.assertEqual(
            output.getvalue(),
            "AC-07 corpus manifest: SYNTHETIC DRAFT\n"
            "Schema version: 1\n"
            "Cases indexed: 3\n"
            "Unique case IDs: PASS\n"
            "Categories represented: 3/8\n"
            "Clean and Verbatim represented: PASS\n"
            "Content hashes: FORMAT VALIDATED ONLY\n"
            "Reference review: PENDING\n"
            "Corpus approval: NOT ESTABLISHED\n"
            "AC-07 acceptance: NOT EVALUATED\n",
        )


if __name__ == "__main__":
    unittest.main()
