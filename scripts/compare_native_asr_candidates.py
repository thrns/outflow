#!/usr/bin/env python3
"""Compare two validated candidate measurements only on an approved common basis."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
from typing import Any, Sequence

if __package__:
    from .run_native_asr_benchmark import BenchmarkError, _unique_object, _reject_constant
    from .validate_measured_asr_results import MeasuredResultsError, validate_measured_results
    from .validate_hardware_evidence import (
        HardwareEvidenceError,
        load_hardware_evidence,
        sha256_bytes,
        validate_hardware_evidence,
    )
else:
    from run_native_asr_benchmark import BenchmarkError, _unique_object, _reject_constant
    from validate_measured_asr_results import MeasuredResultsError, validate_measured_results
    from validate_hardware_evidence import (
        HardwareEvidenceError,
        load_hardware_evidence,
        sha256_bytes,
        validate_hardware_evidence,
    )


class ComparisonBlocked(ValueError):
    """The two measurements do not meet the shared-comparison prerequisites."""


def load_result(path: Path) -> dict[str, Any]:
    """Load a bounded sanitized result and validate its measurement contract."""

    return load_result_with_sha256(path)[0]


def load_result_with_sha256(path: Path) -> tuple[dict[str, Any], str]:
    """Load one result and hash the exact bytes validated from that read."""

    try:
        raw = path.read_bytes()
        if len(raw) > 16 * 1024 * 1024:
            raise ComparisonBlocked("measured result exceeds the supported size limit")
        result = json.loads(
            raw.decode("utf-8"),
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
        validate_measured_results(result)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, RecursionError, TypeError) as error:
        if isinstance(error, ComparisonBlocked):
            raise
        raise ComparisonBlocked("candidate result could not be validated") from error
    return result, sha256_bytes(raw)


def compare_candidate_results(
    small: dict[str, Any],
    tiny: dict[str, Any],
    *,
    small_result_sha256: str | None = None,
    tiny_result_sha256: str | None = None,
    small_hardware_evidence: dict[str, Any] | None = None,
    tiny_hardware_evidence: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Return deltas only with matching human-reviewed comparison evidence."""

    try:
        validate_measured_results(small)
        validate_measured_results(tiny)
    except (MeasuredResultsError, TypeError) as error:
        raise ComparisonBlocked("candidate result failed measured-result validation") from error
    if {small["model"]["candidate"], tiny["model"]["candidate"]} != {"small", "tiny"}:
        raise ComparisonBlocked("comparison requires one Small and one Tiny measurement")
    if small["fixture_manifest_sha256"] != tiny["fixture_manifest_sha256"]:
        raise ComparisonBlocked("candidate measurements do not use the same frozen corpus")
    for result in (small, tiny):
        if (
            result["corpus"]["collection_status"] != "complete"
            or result["corpus"]["complete"] is not True
            or result["aggregate"]["wer"] is None
            or result["timing"]["recognition_p95_ms"] is None
            or result["timing"]["stop_to_final_p95_ms"] is None
        ):
            raise ComparisonBlocked("comparison requires real measurements on a complete approved corpus")
    if (
        small_result_sha256 is None
        or tiny_result_sha256 is None
        or small_hardware_evidence is None
        or tiny_hardware_evidence is None
    ):
        raise ComparisonBlocked("separate human-reviewed hardware evidence is required")
    try:
        small_review = validate_hardware_evidence(
            small, small_hardware_evidence, measured_result_sha256=small_result_sha256
        )
        tiny_review = validate_hardware_evidence(
            tiny, tiny_hardware_evidence, measured_result_sha256=tiny_result_sha256
        )
    except HardwareEvidenceError as error:
        raise ComparisonBlocked("hardware review evidence is missing, altered, or mismatched") from error

    hardware_fields = (
        "machine_label", "machine_identifier", "os", "os_release", "architecture",
        "physical_memory_bytes", "physical_memory_source", "cpu_label",
    )
    small_machine, tiny_machine = small["machine"], tiny["machine"]
    if (
        small_machine["cpu_only_inference_exercised"] is not True
        or tiny_machine["cpu_only_inference_exercised"] is not True
        or any(small_machine[field] != tiny_machine[field] for field in hardware_fields)
        or small_review["reference_hardware_class"] != tiny_review["reference_hardware_class"]
    ):
        raise ComparisonBlocked("candidate measurements do not match the approved hardware review basis")

    def candidate_summary(result: dict[str, Any]) -> dict[str, Any]:
        model = result["model"]
        return {
            "candidate": model["candidate"],
            "model_identity": model["model_identity"],
            "architecture": model["architecture"],
            "model_bytes": model["total_asset_bytes"],
            "wer": result["aggregate"]["wer"],
            "recognition_p95_ms": result["timing"]["recognition_p95_ms"],
            "stop_to_final_p95_ms": result["timing"]["stop_to_final_p95_ms"],
            "real_time_factor_p95": result["timing"]["real_time_factor_p95"],
        }

    return {
        "comparison_status": "READY_FOR_REVIEW_ONLY",
        "comparison_permission": "approved_for_candidate_comparison_only",
        "fixture_manifest_sha256": small["fixture_manifest_sha256"],
        "hardware_comparable": True,
        "physical_4gb_certification": "NOT_ESTABLISHED_BY_COMPARISON",
        "candidates": {
            "small": candidate_summary(small),
            "tiny": candidate_summary(tiny),
        },
        "release_decision": "NOT_APPROVED",
        "limitations": [
            "This comparison does not certify full-application RSS or installed footprint.",
            "Human-reviewed hardware evidence grants comparison permission only; it does not certify a physical 4 GB release target.",
            "A candidate model remains unapproved until all PRD quality, latency, memory, disk, privacy, hardware, and licensing gates pass.",
        ],
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Compare Small and Tiny measured results on one approved corpus and hardware.")
    parser.add_argument("--small", type=Path, required=True)
    parser.add_argument("--tiny", type=Path, required=True)
    parser.add_argument("--small-hardware-evidence", type=Path, required=True)
    parser.add_argument("--tiny-hardware-evidence", type=Path, required=True)
    arguments = parser.parse_args(argv)
    try:
        small, small_hash = load_result_with_sha256(arguments.small)
        tiny, tiny_hash = load_result_with_sha256(arguments.tiny)
        result = compare_candidate_results(
            small,
            tiny,
            small_result_sha256=small_hash,
            tiny_result_sha256=tiny_hash,
            small_hardware_evidence=load_hardware_evidence(arguments.small_hardware_evidence),
            tiny_hardware_evidence=load_hardware_evidence(arguments.tiny_hardware_evidence),
        )
    except (ComparisonBlocked, BenchmarkError, MeasuredResultsError, HardwareEvidenceError) as error:
        print(f"BLOCKED candidate comparison: {error}", file=sys.stderr)
        return 2
    json.dump(result, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
