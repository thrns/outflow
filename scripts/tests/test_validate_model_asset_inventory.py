"""Tests for the development-only ASR model asset inventory validator."""

from __future__ import annotations

import copy
import hashlib
import json
import os
import tempfile
import unittest
from pathlib import Path
from typing import Any

from scripts.validate_model_asset_inventory import (
    InventoryValidationError,
    load_manifest,
    validate_manifest,
)


MANIFEST_PATH = Path(__file__).resolve().parents[2] / "docs" / "model-asset-inventory.json"


def load_inventory() -> dict[str, Any]:
    return json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))


def file_verified_asset(root: Path, *, create_file: bool = True) -> dict[str, Any]:
    """Return one file-verified row backed only by a small temporary fixture."""
    asset = copy.deepcopy(load_inventory()["assets"][0])
    asset.update(
        {
            "status": "file-verified",
            "source_revision": "synthetic-test-revision-1",
            "file_path": "model/synthetic.bin",
            "sha256": hashlib.sha256(b"synthetic model asset bytes\n").hexdigest(),
            "provenance_reference": "temporary synthetic validator fixture",
        }
    )
    if create_file:
        asset_path = root / "model" / "synthetic.bin"
        asset_path.parent.mkdir(parents=True, exist_ok=True)
        asset_path.write_bytes(b"synthetic model asset bytes\n")
    return asset


class ModelAssetInventoryValidatorTests(unittest.TestCase):
    def test_repository_planning_manifest_is_valid(self) -> None:
        errors, counts = validate_manifest(load_inventory())

        self.assertEqual(errors, [])
        self.assertEqual(counts, {
            "planned_assets": 4,
            "file_verified_assets": 0,
            "redistribution_approvals": 0,
        })

    def test_unsupported_schema_version_is_rejected(self) -> None:
        manifest = load_inventory()
        manifest["schema_version"] = 2

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("schema_version must be 1" in error for error in errors))

    def test_duplicate_asset_ids_are_rejected(self) -> None:
        manifest = load_inventory()
        manifest["assets"].append(copy.deepcopy(manifest["assets"][0]))

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("duplicates another asset identifier" in error for error in errors))

    def test_missing_required_field_is_rejected(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0].pop("source_revision")

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("is missing required field(s)" in error for error in errors))

    def test_unknown_asset_category_is_rejected(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["category"] = "model-weights"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("category must be one of" in error for error in errors))

    def test_unknown_asset_status_is_rejected(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["status"] = "bundled"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("status must be one of" in error for error in errors))

    def test_invalid_checksum_is_rejected(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["file_path"] = "model/not-present.bin"
        manifest["assets"][0]["sha256"] = "not-a-checksum"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("sha256 must be 64 hexadecimal characters" in error for error in errors))

    def test_absolute_asset_path_is_rejected(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["file_path"] = "/private/model.bin"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("file_path must be a safe relative path or null" in error for error in errors))

    def test_windows_absolute_asset_path_is_rejected(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["file_path"] = "C:/models/model.bin"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("file_path must be a safe relative path or null" in error for error in errors))

    def test_directory_traversal_path_is_rejected(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["file_path"] = "models/../../outside.bin"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("file_path must be a safe relative path or null" in error for error in errors))

    def test_planned_asset_accepts_explicitly_unknown_revision_and_checksum(self) -> None:
        manifest = load_inventory()
        asset = manifest["assets"][0]
        asset["source_revision"] = None
        asset["sha256"] = None

        errors, counts = validate_manifest(manifest)

        self.assertEqual(errors, [])
        self.assertEqual(counts["file_verified_assets"], 0)

    def test_file_verified_asset_requires_revision_file_hash_and_provenance(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["status"] = "file-verified"

        errors, _ = validate_manifest(manifest)

        for field in ("source_revision", "file_path", "sha256", "provenance_reference"):
            self.assertTrue(any(f"{field} is required for a file-verified asset" in e for e in errors))

    def test_file_verified_asset_rejects_placeholder_revision(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            asset = file_verified_asset(root)
            asset["source_revision"] = "main"
            errors, _ = validate_manifest({"schema_version": 1, "assets": [asset]}, root)

        self.assertTrue(any("source_revision must identify an exact revision" in e for e in errors))

    def test_missing_local_file_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            asset = file_verified_asset(root, create_file=False)
            errors, _ = validate_manifest({"schema_version": 1, "assets": [asset]}, root)

        self.assertTrue(any("does not identify a safe local regular file" in e for e in errors))

    def test_matching_synthetic_checksum_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            asset = file_verified_asset(root)
            errors, counts = validate_manifest({"schema_version": 1, "assets": [asset]}, root)

        self.assertEqual(errors, [])
        self.assertEqual(counts["file_verified_assets"], 1)

    def test_mismatched_checksum_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            asset = file_verified_asset(root)
            asset["sha256"] = "0" * 64
            errors, _ = validate_manifest({"schema_version": 1, "assets": [asset]}, root)

        self.assertTrue(any("sha256 does not match the local file" in error for error in errors))

    def test_symlink_escaping_asset_root_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as asset_dir, tempfile.TemporaryDirectory() as outside_dir:
            root = Path(asset_dir)
            outside_file = Path(outside_dir) / "outside.bin"
            outside_file.write_bytes(b"synthetic outside asset\n")
            link = root / "model" / "synthetic.bin"
            link.parent.mkdir()
            try:
                os.symlink(outside_file, link)
            except (OSError, NotImplementedError) as error:
                self.skipTest(f"symlinks are unavailable in this environment: {type(error).__name__}")

            asset = file_verified_asset(root, create_file=False)
            errors, _ = validate_manifest({"schema_version": 1, "assets": [asset]}, root)

        self.assertTrue(any("does not identify a safe local regular file" in e for e in errors))

    def test_unrecognized_license_review_state_is_rejected(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["license_review_status"] = "approved"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("license_review_status must be one of" in error for error in errors))

    def test_reviewed_license_requires_evidence_reference(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["license_review_status"] = "reviewed"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("license_evidence_reference is required" in error for error in errors))

    def test_redistribution_approval_requires_evidence_reference(self) -> None:
        manifest = load_inventory()
        asset = manifest["assets"][0]
        asset["license_review_status"] = "reviewed"
        asset["license_evidence_reference"] = "evidence/human-license-review.md"
        asset["redistribution_status"] = "approved"
        asset["redistribution_evidence_reference"] = None

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("redistribution_evidence_reference is required" in error for error in errors))

    def test_license_name_alone_does_not_create_redistribution_approval(self) -> None:
        manifest = load_inventory()
        asset = manifest["assets"][0]
        asset["license_identifier"] = "MIT"
        asset["license_review_status"] = "reviewed"
        asset["license_evidence_reference"] = "synthetic review fixture"

        errors, counts = validate_manifest(manifest)

        self.assertEqual(errors, [])
        self.assertEqual(counts["redistribution_approvals"], 0)

    def test_inventory_totals_are_calculated_from_rows(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            first = file_verified_asset(root)
            first["asset_id"] = "synthetic-first"
            second = file_verified_asset(root)
            second["asset_id"] = "synthetic-second"
            second["file_path"] = "model/second.bin"
            second_bytes = b"second synthetic model asset\n"
            (root / "model" / "second.bin").write_bytes(second_bytes)
            second["sha256"] = hashlib.sha256(second_bytes).hexdigest()
            second["license_review_status"] = "reviewed"
            second["license_evidence_reference"] = "temporary human review evidence"
            second["redistribution_status"] = "approved"
            second["redistribution_evidence_reference"] = "temporary approval evidence"
            errors, counts = validate_manifest(
                {"schema_version": 1, "assets": [load_inventory()["assets"][0], first, second]}, root
            )

        self.assertEqual(errors, [])
        self.assertEqual(counts, {
            "planned_assets": 1,
            "file_verified_assets": 2,
            "redistribution_approvals": 1,
        })

    def test_planned_entries_are_never_counted_as_file_verified(self) -> None:
        errors, counts = validate_manifest(load_inventory())

        self.assertEqual(errors, [])
        self.assertEqual(counts["planned_assets"], 4)
        self.assertEqual(counts["file_verified_assets"], 0)

    def test_repeated_validation_returns_deterministic_errors_and_totals(self) -> None:
        manifest = load_inventory()
        manifest["assets"][0]["category"] = "invalid-category"

        first_result = validate_manifest(manifest)
        second_result = validate_manifest(manifest)

        self.assertEqual(first_result, second_result)

    def test_json_loader_rejects_duplicate_object_keys_without_echoing_contents(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            path = Path(temporary_dir) / "duplicate.json"
            path.write_text('{"schema_version":1,"schema_version":1}', encoding="utf-8")
            with self.assertRaises(InventoryValidationError) as caught:
                load_manifest(path)

        self.assertEqual(caught.exception.errors, ("manifest contains duplicate JSON object keys",))


if __name__ == "__main__":
    unittest.main()
