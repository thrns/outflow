"""Unit tests for the development-only benchmark matrix validator."""

from __future__ import annotations

import copy
import json
import unittest
from pathlib import Path
from typing import Any

from scripts.validate_benchmark_matrix import validate_matrix


MATRIX_PATH = Path(__file__).resolve().parents[2] / "docs" / "benchmark-matrix.json"


def load_matrix() -> dict[str, Any]:
    return json.loads(MATRIX_PATH.read_text(encoding="utf-8"))


class BenchmarkMatrixValidatorTests(unittest.TestCase):
    def test_repository_matrix_is_valid_and_reports_unmet_4gb_hardware(self) -> None:
        errors, counts = validate_matrix(load_matrix())

        self.assertEqual(errors, [])
        self.assertEqual(counts["complete_hardware_records"], 1)
        self.assertEqual(counts["physical_4gb_machines"], 0)
        self.assertEqual(counts["AVAILABLE"], 1)
        self.assertEqual(counts["UNAVAILABLE"], 5)

    def test_rejects_missing_required_target_identifier(self) -> None:
        matrix = load_matrix()
        matrix["targets"] = [
            target for target in matrix["targets"] if target["id"] != "windows_10_11_x64"
        ]

        errors, _ = validate_matrix(matrix)

        self.assertTrue(any("missing required target identifiers" in error for error in errors))

    def test_rejects_duplicate_target_identifier(self) -> None:
        matrix = load_matrix()
        matrix["targets"].append(copy.deepcopy(matrix["targets"][0]))

        errors, _ = validate_matrix(matrix)

        self.assertTrue(any("duplicate target identifier" in error for error in errors))

    def test_rejects_unknown_availability_status(self) -> None:
        matrix = load_matrix()
        matrix["targets"][0]["availability"] = "CERTIFIED"

        errors, _ = validate_matrix(matrix)

        self.assertTrue(any("availability must be one of" in error for error in errors))

    def test_available_record_requires_os_cpu_architecture_ram_and_evidence(self) -> None:
        matrix = load_matrix()
        target = matrix["targets"][0]
        target["hardware"]["cpu_model"] = None
        target["evidence"].pop("os_build")

        errors, _ = validate_matrix(matrix)

        self.assertTrue(any("hardware.cpu_model must be verified" in error for error in errors))
        self.assertTrue(any("evidence.os_build must identify" in error for error in errors))

    def test_pending_and_unavailable_may_have_unknown_hardware_values(self) -> None:
        matrix = load_matrix()
        matrix["targets"][1]["availability"] = "PENDING"

        errors, _ = validate_matrix(matrix)

        self.assertEqual(errors, [])

    def test_rejects_physical_4gb_claim_for_an_18gb_machine(self) -> None:
        matrix = load_matrix()
        matrix["targets"][0]["physical_4gb_test_machine_verified"] = True

        errors, _ = validate_matrix(matrix)

        self.assertTrue(any("without AVAILABLE hardware evidence of exactly 4 GB" in error for error in errors))

    def test_rejects_physical_4gb_claim_without_matching_ram_evidence(self) -> None:
        matrix = load_matrix()
        target = matrix["targets"][0]
        target["hardware"]["physical_ram_gb"] = 4
        target["evidence"].pop("physical_ram_gb")
        target["physical_4gb_test_machine_verified"] = True

        errors, _ = validate_matrix(matrix)

        self.assertTrue(any("evidence.physical_ram_gb must identify" in error for error in errors))
        self.assertTrue(any("without AVAILABLE hardware evidence of exactly 4 GB" in error for error in errors))

    def test_accepts_physical_4gb_claim_with_matching_machine_evidence(self) -> None:
        matrix = load_matrix()
        target = matrix["targets"][0]
        target["hardware"]["physical_ram_gb"] = 4
        target["physical_4gb_test_machine_verified"] = True

        errors, counts = validate_matrix(matrix)

        self.assertEqual(errors, [])
        self.assertEqual(counts["physical_4gb_machines"], 1)

    def test_rejects_non_string_availability_without_crashing(self) -> None:
        matrix = load_matrix()
        matrix["targets"][0]["availability"] = ["AVAILABLE"]

        errors, _ = validate_matrix(matrix)

        self.assertTrue(any("availability must be one of" in error for error in errors))

    def test_rejects_sensitive_identification_fields(self) -> None:
        matrix = load_matrix()
        matrix["targets"][0]["hardware"]["hardware_uuid"] = "redacted-test-value"
        matrix["targets"][0]["hardware"]["serial_number"] = "redacted-test-value"

        errors, _ = validate_matrix(matrix)

        self.assertTrue(any("prohibited sensitive identification field" in error for error in errors))


if __name__ == "__main__":
    unittest.main()
