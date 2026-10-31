"""Tests for the third-party notice manifest validator."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import unittest
from typing import Any

from scripts.validate_moonshine_third_party import load_inventory
from scripts.validate_third_party_notices import render_summary, validate_manifest


ROOT = Path(__file__).resolve().parents[2]
MANIFEST_PATH = ROOT / "docs" / "third-party-notices.json"
INVENTORY_PATH = ROOT / "docs" / "moonshine-third-party.json"


def load_planning_manifest() -> dict[str, Any]:
    return json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))


def load_inventory_data() -> dict[str, Any]:
    return load_inventory(INVENTORY_PATH)


class ThirdPartyNoticesValidatorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.manifest = load_planning_manifest()
        self.inventory = load_inventory_data()

    def validate(self) -> tuple[list[str], dict[str, int]]:
        return validate_manifest(self.manifest, self.inventory)

    def test_valid_planning_manifest_passes(self) -> None:
        errors, counts = self.validate()

        self.assertEqual(errors, [])
        self.assertEqual(counts, {"dependencies": 8, "planned": 4, "blocked": 4, "packaged": 0})

    def test_duplicate_ids_are_rejected(self) -> None:
        self.manifest["dependencies"][1]["dependency_id"] = self.manifest["dependencies"][0]["dependency_id"]

        errors, _ = self.validate()

        self.assertTrue(any("duplicates another dependency identifier" in error for error in errors))

    def test_missing_dependency_is_rejected(self) -> None:
        self.manifest["dependencies"].pop()

        errors, _ = self.validate()

        self.assertTrue(any("missing one or more IDs" in error for error in errors))
        self.assertIn("dependencies must contain exactly eight audited dependency IDs", errors)

    def test_unknown_dependency_is_rejected(self) -> None:
        self.manifest["dependencies"][0]["dependency_id"] = "moonshine-not-audited"

        errors, _ = self.validate()

        self.assertTrue(any("is not present in the source inventory" in error for error in errors))

    def test_license_mismatch_against_audit_is_rejected(self) -> None:
        self.manifest["dependencies"][0]["license_expression"] = "MIT"

        errors, _ = self.validate()

        self.assertIn("dependencies[0].license_expression must match the source inventory", errors)

    def test_incorrect_build_use_classification_is_rejected(self) -> None:
        self.manifest["dependencies"][0]["build_use_classification"] = "conditional"

        errors, _ = self.validate()

        self.assertIn("dependencies[0].build_use_classification must match the source inventory", errors)

    def test_invalid_evidence_url_is_rejected(self) -> None:
        self.manifest["dependencies"][0]["license_evidence_url"] = (
            "https://github.com/moonshine-ai/moonshine/blob/main/LICENSE"
        )

        errors, _ = self.validate()

        self.assertTrue(any("must be an immutable upstream license URL" in error for error in errors))

    def test_unsupported_notice_state_is_rejected(self) -> None:
        for state in ("packaged", "verified"):
            with self.subTest(state=state):
                self.manifest["dependencies"][0]["notice_verification_status"] = state

                errors, _ = self.validate()

                self.assertIn(
                    "dependencies[0].notice_verification_status is unsupported; use planned or blocked",
                    errors,
                )

    def test_unsafe_destination_path_is_rejected(self) -> None:
        self.manifest["dependencies"][0]["planned_notice_destination"] = "../release/THIRD-PARTY-NOTICES.txt"

        errors, _ = self.validate()

        self.assertIn(
            "dependencies[0].planned_notice_destination must be null or a safe repository-relative path",
            errors,
        )

    def test_blocked_notice_without_explanation_is_rejected(self) -> None:
        self.manifest["dependencies"][0]["status_explanation"] = ""

        errors, _ = self.validate()

        self.assertIn("dependencies[0].status_explanation is required for a blocked notice", errors)

    def test_unsupported_approval_claim_is_rejected(self) -> None:
        self.manifest["dependencies"][0]["redistribution_approval"] = "approved"

        errors, _ = self.validate()

        self.assertIn("dependencies[0].redistribution_approval must remain undetermined", errors)

    def test_planned_notices_are_not_counted_as_packaged(self) -> None:
        errors, counts = self.validate()
        self.assertEqual(errors, [])

        summary = render_summary(self.manifest, counts)

        self.assertIn("Packaged notices: 0", summary)
        self.assertEqual(counts["packaged"], 0)

    def test_summary_output_is_deterministic(self) -> None:
        errors, counts = self.validate()
        self.assertEqual(errors, [])

        first = render_summary(self.manifest, counts)
        second = render_summary(copy.deepcopy(self.manifest), dict(counts))

        self.assertEqual(first, second)
        self.assertEqual(
            first,
            "\n".join(
                (
                    "Outflow third-party notice manifest valid",
                    "Manifest version: 1",
                    "Dependencies: 8",
                    "Packaged notices: 0",
                    "Redistribution approval: UNDETERMINED",
                    "Notice readiness: INCOMPLETE",
                    "Notice states: 4 planned, 4 blocked",
                )
            ),
        )


if __name__ == "__main__":
    unittest.main()
