from __future__ import annotations

import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from typing import Any

from scripts.run_native_asr_benchmark import (
    EVALUATION_VERSION,
    MODEL_CANDIDATES,
    EXPECTED_ARCHITECTURE,
    EXPECTED_HEADER_VERSION,
    MODEL_CATALOG_PATH,
    REQUIRED_MODEL_ASSETS,
    RESULT_SCHEMA_VERSION,
    RESULT_TYPE,
    SOURCE_REVISION,
)
from scripts.score_wer import NORMALIZATION_POLICY_VERSION
from scripts.validate_fixture_manifest import assess_corpus_coverage
from scripts.compare_native_asr_candidates import ComparisonBlocked, compare_candidate_results
from scripts.validate_measured_asr_results import MeasuredResultsError, main, validate_measured_results


def valid_result() -> dict[str, Any]:
    binary_hashes = {
        name: hashlib.sha256(name.encode("ascii")).hexdigest()
        for name in ("native_runner", "moonshine_library", "onnxruntime_library")
    }
    fixture = {
        "fixture_id": "speech_001",
        "category": "short_speech",
        "duration_class": "short",
        "coverage_attributes": [],
        "speaker_variety": None,
        "provenance_verified": True,
        "human_reference_verified": True,
        "audio_sha256": hashlib.sha256(b"audio").hexdigest(),
        "reference_sha256": hashlib.sha256(b"reference").hexdigest(),
        "audio_metadata": {
            "sample_format": "PCM_S16LE",
            "sample_rate_hz": 16_000,
            "channels": 1,
            "frame_count": 16_000,
            "duration_seconds": 1.0,
        },
        "wer": 0.0,
        "error_counts": {"reference_words": 2, "substitutions": 0, "deletions": 0, "insertions": 0},
        "no_speech_detected": None,
        "partial_calls": 1,
        "partial_nonempty": True,
        "final_line_count": 1,
        "recognition_ms": 40.0,
        "stop_to_final_ms": 4.0,
        "real_time_factor": 0.04,
        "native_process_cpu_ms": 55.0,
    }
    coverage_input = [{
        "status": "ready",
        "duration_class": fixture["duration_class"],
        "coverage_attributes": fixture["coverage_attributes"],
        "speaker_variety": fixture["speaker_variety"],
        "provenance_verified": True,
        "human_reference_verified": True,
    }]
    coverage = assess_corpus_coverage(coverage_input)
    coverage["collection_status"] = "planned"
    memory = 18_000_000_000
    timing = {
        "recognition_p50_ms": 40.0,
        "recognition_p95_ms": 40.0,
        "stop_to_final_p50_ms": 4.0,
        "stop_to_final_p95_ms": 4.0,
        "real_time_factor_p50": 0.04,
        "real_time_factor_p95": 0.04,
    }
    return {
        "schema_version": RESULT_SCHEMA_VERSION,
        "result_type": RESULT_TYPE,
        "evaluation_version": EVALUATION_VERSION,
        "created_at_utc": "2026-11-03T20:00:00Z",
        "normalization_policy_version": NORMALIZATION_POLICY_VERSION,
        "reference_transcription_policy_version": "human-verified-verbatim-v1",
        "fixture_manifest_sha256": "a" * 64,
        "source": {
            "source_revision": SOURCE_REVISION,
            "c_api_header_version": EXPECTED_HEADER_VERSION,
            "model_architecture": EXPECTED_ARCHITECTURE,
            "candidate": "small",
            "source_contract_sha256": "b" * 64,
        },
        "model": {
            "candidate": "small",
            "model_identity": MODEL_CANDIDATES["small"]["identity"],
            "catalog_path": MODEL_CATALOG_PATH,
            "immutable_artifact_revision": None,
            "architecture": EXPECTED_ARCHITECTURE,
            "source_contract_sha256": "b" * 64,
            "c_api_header_version": EXPECTED_HEADER_VERSION,
            "asset_sha256": {name: hashlib.sha256(name.encode()).hexdigest() for name in REQUIRED_MODEL_ASSETS},
            "asset_bytes": {name: 1 for name in REQUIRED_MODEL_ASSETS},
            "total_asset_bytes": len(REQUIRED_MODEL_ASSETS),
            "cold_process_load_ms": 120.0,
            "warm_reload_samples_ms": [80.0],
            "warm_reload_median_ms": 80.0,
            "inference_process_load_ms": 90.0,
            "redistribution_status": "undetermined",
            "license_review_status": "review-pending",
        },
        "native_binaries": {
            name: {"bytes": 100, "sha256": digest}
            for name, digest in binary_hashes.items()
        },
        "native_runtime_identity": {
            "architecture": "arm64",
            "c_api_version": EXPECTED_HEADER_VERSION,
            "loaded_image_count": 5,
            "unexpected_non_system_library_count": 0,
            "resolved_paths_match_inputs": True,
            "dependency_chain_verified": True,
            "native_runner_sha256": binary_hashes["native_runner"],
            "moonshine_sha256": binary_hashes["moonshine_library"],
            "onnxruntime_sha256": binary_hashes["onnxruntime_library"],
        },
        "machine": {
            "machine_label": "MAC-TEST",
            "machine_identifier": "Mac-Test1,1",
            "os": "macOS",
            "os_release": "26.6.2",
            "architecture": "arm64",
            "physical_memory_bytes": memory,
            "physical_memory_source": "sysctl_hw.memsize",
            "physical_hardware_status": "unknown",
            "reference_hardware_class": None,
            "cpu_only_inference_exercised": True,
            "real_hardware_measurements_completed": False,
            "logical_cpu_count": 12,
            "cpu_label": "Test CPU",
        },
        "corpus": coverage,
        "aggregate": {
            "speech_fixtures_scored": 1,
            "reference_words": 2,
            "substitutions": 0,
            "deletions": 0,
            "insertions": 0,
            "wer": 0.0,
        },
        "timing": timing,
        "fixtures": [fixture],
        "release_gates": {
            "verified_corpus_coverage": "NOT_ASSESSED",
            "wer_at_or_below_12_percent": "NOT_ASSESSED",
            "rtf_p95_below_1": "NOT_ASSESSED",
            "stop_to_final_p95_below_1500_ms": "NOT_ASSESSED",
            "physical_4gb_eligibility": "INELIGIBLE",
            "physical_4gb_gate": "BLOCKED",
            "cpu_only_inference": "PASS",
            "real_hardware_measurements": "NOT_ASSESSED",
            "release_performance_requirements": "NOT_ASSESSED",
            "process_tree_memory": "NOT_MEASURED",
            "installed_application_size": "NOT_MEASURED",
            "redistribution_approval": "NOT_APPROVED",
        },
        "limitations": ["Partial fixture set; release requirements remain unassessed."],
    }


class MeasuredResultsValidatorTests(unittest.TestCase):
    def test_valid_measured_result_is_accepted(self) -> None:
        summary = validate_measured_results(valid_result())
        self.assertEqual(summary["fixtures"], 1)
        self.assertEqual(summary["speech_fixtures"], 1)
        self.assertFalse(summary["coverage_complete"])

    def test_result_architecture_must_match_selected_candidate(self) -> None:
        value = valid_result()
        value["source"]["model_architecture"] = MODEL_CANDIDATES["tiny"]["architecture"]
        with self.assertRaisesRegex(MeasuredResultsError, "pinned source identity"):
            validate_measured_results(value)

    def test_candidate_comparison_is_blocked_for_planned_corpus(self) -> None:
        small = valid_result()
        tiny = valid_result()
        tiny["source"].update(candidate="tiny", model_architecture=MODEL_CANDIDATES["tiny"]["architecture"])
        tiny["model"].update(
            candidate="tiny",
            model_identity=MODEL_CANDIDATES["tiny"]["identity"],
            catalog_path="tiny-streaming-en/quantized_26_08_21",
            architecture=MODEL_CANDIDATES["tiny"]["architecture"],
        )
        with self.assertRaisesRegex(ComparisonBlocked, "complete approved corpus"):
            compare_candidate_results(small, tiny)

    def test_malformed_root_and_unsupported_result_types_are_rejected(self) -> None:
        with self.assertRaisesRegex(MeasuredResultsError, "root fields"):
            validate_measured_results({"schema_version": RESULT_SCHEMA_VERSION})
        value = valid_result()
        value["result_type"] = "synthetic"
        with self.assertRaisesRegex(MeasuredResultsError, "result type"):
            validate_measured_results(value)
        value = valid_result()
        value["schema_version"] = True
        with self.assertRaisesRegex(MeasuredResultsError, "result type"):
            validate_measured_results(value)

    def test_invalid_fixture_and_aggregate_wer_arithmetic_are_rejected(self) -> None:
        value = valid_result()
        value["fixtures"][0]["wer"] = 0.5
        with self.assertRaisesRegex(MeasuredResultsError, "fixture WER"):
            validate_measured_results(value)
        value = valid_result()
        value["aggregate"]["insertions"] = 1
        with self.assertRaisesRegex(MeasuredResultsError, "aggregate counts"):
            validate_measured_results(value)
        value = valid_result()
        value["fixtures"][0]["error_counts"] = {"reference_words": 1, "substitutions": 1, "deletions": 1, "insertions": 0}
        with self.assertRaisesRegex(MeasuredResultsError, "exceed reference words"):
            validate_measured_results(value)

    def test_impossible_audio_and_timing_metadata_are_rejected(self) -> None:
        value = valid_result()
        value["fixtures"][0]["audio_metadata"]["frame_count"] = 8_000
        with self.assertRaisesRegex(MeasuredResultsError, "metadata"):
            validate_measured_results(value)
        value = valid_result()
        value["fixtures"][0]["real_time_factor"] = 10.0
        with self.assertRaisesRegex(MeasuredResultsError, "real-time factor"):
            validate_measured_results(value)

    def test_forbidden_content_and_invalid_runtime_identity_are_rejected(self) -> None:
        value = valid_result()
        value["fixtures"][0]["hypothesis"] = "private content"
        with self.assertRaisesRegex(MeasuredResultsError, "forbidden content"):
            validate_measured_results(value)
        value = valid_result()
        value["native_runtime_identity"]["onnxruntime_sha256"] = "0" * 64
        with self.assertRaisesRegex(MeasuredResultsError, "loaded native runtime"):
            validate_measured_results(value)
        value = valid_result()
        value["native_runtime_identity"]["resolved_paths_match_inputs"] = False
        with self.assertRaisesRegex(MeasuredResultsError, "loaded native runtime"):
            validate_measured_results(value)
        value = valid_result()
        value["native_runtime_identity"]["native_runner_sha256"] = "0" * 64
        with self.assertRaisesRegex(MeasuredResultsError, "loaded native runtime"):
            validate_measured_results(value)

    def test_incomplete_corpus_cannot_claim_complete_coverage(self) -> None:
        value = valid_result()
        value["corpus"]["short_utterance_count"] = 100
        value["corpus"]["long_utterance_count"] = 50
        value["corpus"]["complete"] = True
        value["release_gates"]["verified_corpus_coverage"] = "PASS"
        with self.assertRaisesRegex(MeasuredResultsError, "corpus coverage summary"):
            validate_measured_results(value)

    def test_hardware_identity_fields_reject_wrong_json_types(self) -> None:
        value = valid_result()
        value["machine"]["physical_memory_source"] = []
        with self.assertRaisesRegex(MeasuredResultsError, "physical memory source"):
            validate_measured_results(value)
        value = valid_result()
        value["machine"]["physical_hardware_status"] = {}
        with self.assertRaisesRegex(MeasuredResultsError, "physical hardware status"):
            validate_measured_results(value)

    def test_4gb_memory_requires_exact_approved_physical_hardware_and_stays_unassessed(self) -> None:
        value = valid_result()
        value["machine"]["physical_memory_bytes"] = 4_000_000_000
        value["machine"]["physical_hardware_status"] = "unknown"
        value["release_gates"]["physical_4gb_eligibility"] = "UNKNOWN"
        value["release_gates"]["physical_4gb_gate"] = "NOT_ASSESSED"
        validate_measured_results(value)

        value["machine"]["physical_hardware_status"] = "verified_physical"
        value["machine"]["reference_hardware_class"] = "reference-mac-4gb"
        # Caller-supplied class labels are not enough; this repository's
        # approved reference-class allowlist remains empty pending review.
        validate_measured_results(value)

        with patch("scripts.run_native_asr_benchmark.APPROVED_4GB_REFERENCE_CLASSES", frozenset({"reference-mac-4gb"})):
            value["release_gates"]["physical_4gb_eligibility"] = "ELIGIBLE"
            value["release_gates"]["physical_4gb_gate"] = "NOT_ASSESSED"
            validate_measured_results(value)
            value["machine"]["physical_memory_bytes"] = 4_294_967_296
            validate_measured_results(value)
            # Reference eligibility alone cannot pass the actual benchmark gate.
            value["release_gates"]["physical_4gb_gate"] = "PASS"
            with self.assertRaisesRegex(MeasuredResultsError, "release gates"):
                validate_measured_results(value)

    def test_more_than_4gb_is_never_eligible_even_with_reference_labels(self) -> None:
        value = valid_result()
        value["machine"]["physical_memory_bytes"] = 4_294_967_297
        value["machine"]["physical_hardware_status"] = "verified_physical"
        value["machine"]["reference_hardware_class"] = "reference-mac-4gb"
        value["release_gates"]["physical_4gb_eligibility"] = "ELIGIBLE"
        with self.assertRaisesRegex(MeasuredResultsError, "release gates"):
            validate_measured_results(value)

    def test_unknown_memory_and_simulated_hardware_cannot_certify_reference_status(self) -> None:
        value = valid_result()
        value["machine"].update({
            "physical_memory_bytes": None,
            "physical_memory_source": None,
            "physical_hardware_status": "verified_physical",
            "reference_hardware_class": "reference-mac-4gb",
        })
        value["release_gates"]["physical_4gb_eligibility"] = "UNKNOWN"
        value["release_gates"]["physical_4gb_gate"] = "NOT_ASSESSED"
        validate_measured_results(value)

        value["machine"].update({
            "physical_memory_bytes": 4_000_000_000,
            "physical_memory_source": "sysctl_hw.memsize",
            "physical_hardware_status": "virtual_or_simulated",
        })
        value["release_gates"]["physical_4gb_eligibility"] = "INELIGIBLE"
        value["release_gates"]["physical_4gb_gate"] = "BLOCKED"
        validate_measured_results(value)

    def test_unverified_hardware_cannot_claim_real_hardware_measurement_completion(self) -> None:
        value = valid_result()
        value["machine"]["real_hardware_measurements_completed"] = True
        value["release_gates"]["real_hardware_measurements"] = "PASS"
        with self.assertRaisesRegex(MeasuredResultsError, "verified physical hardware"):
            validate_measured_results(value)

    def test_duplicate_json_keys_are_rejected_by_file_validator(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "invalid.json"
            path.write_text('{"schema_version":2,"schema_version":2}', encoding="utf-8")
            self.assertEqual(main(["--file", str(path)]), 1)


if __name__ == "__main__":
    unittest.main()
