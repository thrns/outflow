"""Tests for the local Moonshine third-party inventory validator."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import tempfile
import unittest
from typing import Any

from scripts.validate_moonshine_third_party import (
    load_inventory,
    render_summary,
    validate_inventory,
)


INVENTORY_PATH = (
    Path(__file__).resolve().parents[2] / "docs" / "moonshine-third-party.json"
)


def load_repository_inventory() -> dict[str, Any]:
    return json.loads(INVENTORY_PATH.read_text(encoding="utf-8"))


class MoonshineThirdPartyValidatorTests(unittest.TestCase):
    def test_valid_inventory_passes_and_covers_eight_dependencies(self) -> None:
        inventory = load_repository_inventory()

        errors, counts = validate_inventory(inventory)

        self.assertEqual(errors, [])
        self.assertEqual(counts["dependencies"], 8)
        self.assertEqual(counts["verified_licenses"], 8)
        self.assertEqual(counts["confirmed-build-input"], 5)
        self.assertEqual(counts["conditional"], 2)
        self.assertEqual(counts["test-only"], 1)
        self.assertEqual(inventory["redistribution_approval"], "undetermined")

    def test_unsupported_schema_version_is_rejected(self) -> None:
        inventory = load_repository_inventory()
        inventory["schema_version"] = 2

        errors, _ = validate_inventory(inventory)

        self.assertIn("schema_version must be 1", errors)

    def test_duplicate_dependency_ids_are_rejected(self) -> None:
        inventory = load_repository_inventory()
        duplicate = copy.deepcopy(inventory["dependencies"][0])
        duplicate["upstream_directory"] = "core/third-party/doctest"
        inventory["dependencies"].append(duplicate)

        errors, _ = validate_inventory(inventory)

        self.assertTrue(any("duplicates another dependency identifier" in error for error in errors))

    def test_missing_required_dependency_is_rejected(self) -> None:
        inventory = load_repository_inventory()
        inventory["dependencies"].pop()

        errors, _ = validate_inventory(inventory)

        self.assertIn(
            "dependencies are missing one or more required pinned directories", errors
        )

    def test_unsupported_category_is_rejected(self) -> None:
        inventory = load_repository_inventory()
        inventory["dependencies"][0]["dependency_category"] = "runtime-approval"

        errors, _ = validate_inventory(inventory)

        self.assertIn("dependencies[0].dependency_category is unsupported", errors)

    def test_unsupported_build_use_classification_is_rejected(self) -> None:
        inventory = load_repository_inventory()
        inventory["dependencies"][0]["build_use_classification"] = "shipped"

        errors, _ = validate_inventory(inventory)

        self.assertIn("dependencies[0].build_use_classification is unsupported", errors)

    def test_explicit_unknown_build_use_classification_is_allowed(self) -> None:
        inventory = load_repository_inventory()
        inventory["dependencies"][0]["build_use_classification"] = "unknown"

        errors, counts = validate_inventory(inventory)

        self.assertEqual(errors, [])
        self.assertEqual(counts["unknown"], 1)

    def test_missing_license_evidence_url_is_rejected(self) -> None:
        inventory = load_repository_inventory()
        inventory["dependencies"][0]["license_evidence_urls"] = []

        errors, _ = validate_inventory(inventory)

        self.assertIn(
            "dependencies[0].license_evidence_urls must contain at least one evidence URL",
            errors,
        )

    def test_malformed_evidence_url_is_rejected(self) -> None:
        inventory = load_repository_inventory()
        inventory["dependencies"][0]["license_evidence_urls"][0] = (
            "https://github.com:bad/moonshine-ai/moonshine/blob/"
            "234f60faa0eb388b01cdf7e60aca232af37aefda/LICENSE"
        )

        errors, _ = validate_inventory(inventory)

        self.assertTrue(any("must be a complete HTTPS URL" in error for error in errors))

    def test_evidence_url_with_revision_mismatch_is_rejected(self) -> None:
        inventory = load_repository_inventory()
        inventory["dependencies"][0]["source_build_evidence_urls"][0] = (
            "https://github.com/moonshine-ai/moonshine/blob/"
            "abcdef0123456789abcdef0123456789abcdef01/core/CMakeLists.txt"
        )

        errors, _ = validate_inventory(inventory)

        self.assertTrue(
            any("must reference moonshine-ai/moonshine at the recorded source revision" in error
                for error in errors)
        )

    def test_explicitly_unknown_license_is_allowed(self) -> None:
        inventory = load_repository_inventory()
        dependency = inventory["dependencies"][0]
        dependency["license_status"] = "unknown"
        dependency["license_identifiers"] = []
        dependency["license_evidence_urls"] = []

        errors, counts = validate_inventory(inventory)

        self.assertEqual(errors, [])
        self.assertEqual(counts["unknown_licenses"], 1)

    def test_unknown_license_evidence_urls_are_still_validated(self) -> None:
        inventory = load_repository_inventory()
        dependency = inventory["dependencies"][0]
        dependency["license_status"] = "unknown"
        dependency["license_identifiers"] = []
        dependency["license_evidence_urls"] = ["not a URL"]

        errors, _ = validate_inventory(inventory)

        self.assertTrue(
            any("must be a complete HTTPS URL" in error for error in errors)
        )

    def test_redistribution_approval_cannot_be_claimed(self) -> None:
        inventory = load_repository_inventory()
        inventory["dependencies"][0]["redistribution_review_status"] = "approved"

        errors, _ = validate_inventory(inventory)

        self.assertIn(
            "dependencies[0].redistribution_review_status must remain undetermined",
            errors,
        )

    def test_source_review_cannot_be_presented_as_binary_verification(self) -> None:
        inventory = load_repository_inventory()
        inventory["binary_verification_status"] = "verified"

        errors, _ = validate_inventory(inventory)

        self.assertIn("source review cannot claim binary verification", errors)

    def test_deterministic_summary_output(self) -> None:
        inventory = load_repository_inventory()
        errors, counts = validate_inventory(inventory)
        self.assertEqual(errors, [])

        first = render_summary(inventory, counts)
        second = render_summary(inventory, counts)

        self.assertEqual(first, second)
        self.assertEqual(
            first,
            "\n".join(
                (
                    "Outflow Moonshine third-party inventory valid",
                    "Reviewed commit: 234f60faa0eb388b01cdf7e60aca232af37aefda",
                    "Dependencies: 8",
                    "Build use: 5 confirmed, 2 conditional, 1 test-only, 0 unknown",
                    "License review: 8 verified, 0 unknown",
                    "Redistribution approval: UNDETERMINED",
                    "Binary verification: NOT VERIFIED",
                )
            ),
        )

    def test_loader_rejects_duplicate_json_object_keys(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            temporary_path = Path(temporary_directory) / "invalid-inventory.json"
            temporary_path.write_text(
                '{"schema_version":1,"schema_version":1}', encoding="utf-8"
            )
            with self.assertRaisesRegex(ValueError, "not valid UTF-8 JSON"):
                load_inventory(temporary_path)


if __name__ == "__main__":
    unittest.main()
