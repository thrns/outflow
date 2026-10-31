"""Tests for the local Moonshine licensing review validator."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from typing import Any

from scripts.validate_moonshine_license_review import (
    load_review,
    render_summary,
    validate_review,
)


REVIEW_PATH = Path(__file__).resolve().parents[2] / "docs" / "moonshine-license-review.json"


def load_repository_review() -> dict[str, Any]:
    return json.loads(REVIEW_PATH.read_text(encoding="utf-8"))


class MoonshineLicenseReviewValidatorTests(unittest.TestCase):
    def test_valid_preliminary_review_passes(self) -> None:
        review = load_repository_review()

        errors, counts = validate_review(review)

        self.assertEqual(errors, [])
        self.assertEqual(counts, {
            "upstream-documented": 6,
            "unverified": 3,
            "conflicting": 0,
            "claims": 9,
        })
        self.assertEqual(review["redistribution_approval"], "undetermined")

    def test_unsupported_schema_version_is_rejected(self) -> None:
        review = load_repository_review()
        review["schema_version"] = 2

        errors, _ = validate_review(review)

        self.assertIn("schema_version must be 1", errors)

    def test_duplicate_claim_ids_are_rejected(self) -> None:
        review = load_repository_review()
        review["claims"].append(copy.deepcopy(review["claims"][0]))

        errors, _ = validate_review(review)

        self.assertTrue(any("duplicates another claim identifier" in error for error in errors))

    def test_missing_source_reference_is_rejected(self) -> None:
        review = load_repository_review()
        review["claims"][0]["sources"] = []

        errors, _ = validate_review(review)

        self.assertTrue(any("sources must contain a source reference" in error for error in errors))

    def test_invalid_url_is_rejected(self) -> None:
        review = load_repository_review()
        review["claims"][0]["sources"][0]["evidence_url"] = "https://github.com:bad/moonshine"

        errors, _ = validate_review(review)

        self.assertTrue(any("must be a complete HTTPS URL" in error for error in errors))

    def test_invalid_evidence_status_is_rejected(self) -> None:
        review = load_repository_review()
        review["claims"][0]["evidence_status"] = "approved"

        errors, _ = validate_review(review)

        self.assertTrue(any("evidence_status is unsupported" in error for error in errors))

    def test_explicitly_unknown_source_revision_is_valid(self) -> None:
        review = load_repository_review()
        source = review["claims"][4]["sources"][1]

        self.assertEqual(source["source_revision"]["status"], "unverified")
        self.assertIsNone(source["source_revision"]["commit"])
        errors, _ = validate_review(review)

        self.assertEqual(errors, [])

    def test_claim_review_date_must_match_document_date(self) -> None:
        review = load_repository_review()
        review["claims"][0]["review_date"] = "2026-10-30"

        errors, _ = validate_review(review)

        self.assertTrue(any("review_date must match the review date" in error for error in errors))

    def test_invalid_source_revision_format_is_rejected(self) -> None:
        review = load_repository_review()
        revision = review["claims"][0]["sources"][0]["source_revision"]
        revision["commit"] = "234f60f"

        errors, _ = validate_review(review)

        self.assertTrue(any("full 40-character lowercase Git SHA" in error for error in errors))

    def test_placeholder_source_revision_is_rejected(self) -> None:
        review = load_repository_review()
        revision = review["claims"][0]["sources"][0]["source_revision"]
        revision["commit"] = "0" * 40
        review["claims"][0]["sources"][0]["evidence_url"] = (
            f"https://github.com/moonshine-ai/moonshine/blob/{revision['commit']}/docs/license.md"
        )

        errors, _ = validate_review(review)

        self.assertTrue(any("must not be a placeholder hash" in error for error in errors))

    def test_conflicting_evidence_status_is_supported_and_counted(self) -> None:
        review = load_repository_review()
        claim = review["claims"][0]
        claim["evidence_status"] = "conflicting"
        claim["remaining_limitations"].append("Conflicting upstream statements need manual resolution.")

        errors, counts = validate_review(review)

        self.assertEqual(errors, [])
        self.assertEqual(counts["conflicting"], 1)
        self.assertIn("1 conflicting", render_summary(review, counts))

    def test_redistribution_approval_cannot_be_claimed(self) -> None:
        review = load_repository_review()
        review["redistribution_approval"] = "approved"

        errors, _ = validate_review(review)

        self.assertIn(
            "redistribution_approval must remain undetermined in this preliminary review",
            errors,
        )

    def test_documentation_evidence_cannot_verify_artifact_integrity(self) -> None:
        review = load_repository_review()
        review["claims"][0]["evidence_scope"] = "model-artifact"

        errors, _ = validate_review(review)

        self.assertTrue(any("cannot use documentation evidence as verified artifact evidence" in error for error in errors))

    def test_deterministic_summary_output(self) -> None:
        review = load_repository_review()
        errors, counts = validate_review(review)
        self.assertEqual(errors, [])

        first = render_summary(review, counts)
        second = render_summary(review, counts)

        self.assertEqual(first, second)
        self.assertEqual(
            first,
            "\n".join(
                (
                    "Outflow Moonshine licensing review valid",
                    "Review date: 2026-10-31",
                    "Claims: 9",
                    "Evidence status: 6 upstream-documented, 3 unverified, 0 conflicting",
                    "Redistribution approval: UNDETERMINED",
                    "Model file integrity: NOT VERIFIED",
                    "Runtime binary integrity: NOT VERIFIED",
                )
            ),
        )

    def test_loader_rejects_duplicate_json_object_keys(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            temporary_path = Path(temporary_directory) / "duplicate-key-test.json"
            temporary_path.write_text('{"schema_version":1,"schema_version":1}', encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "not valid UTF-8 JSON"):
                load_review(temporary_path)


if __name__ == "__main__":
    unittest.main()
