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
else:
    from run_native_asr_benchmark import BenchmarkError, _unique_object, _reject_constant
    from validate_measured_asr_results import MeasuredResultsError, validate_measured_results


class ComparisonBlocked(ValueError):
    """The two measurements do not meet the shared-comparison prerequisites."""


def load_result(path: Path) -> dict[str, Any]:
    """Load a bounded sanitized result and validate its measurement contract."""

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
    return result


def compare_candidate_results(small: dict[str, Any], tiny: dict[str, Any]) -> dict[str, Any]:
    """Return aggregate deltas only when corpus and hardware evidence match."""

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
    hardware_fields = (
        "machine_identifier", "os", "os_release", "architecture", "physical_memory_bytes",
        "cpu_label", "reference_hardware_class",
    )
    small_machine, tiny_machine = small["machine"], tiny["machine"]
    if (
        small_machine["physical_hardware_status"] != "verified_physical"
        or tiny_machine["physical_hardware_status"] != "verified_physical"
        or small_machine["real_hardware_measurements_completed"] is not True
        or tiny_machine["real_hardware_measurements_completed"] is not True
        or small_machine["cpu_only_inference_exercised"] is not True
        or tiny_machine["cpu_only_inference_exercised"] is not True
        or any(small_machine[field] != tiny_machine[field] for field in hardware_fields)
    ):
        raise ComparisonBlocked("candidate measurements are not comparable on verified physical hardware")

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
        "fixture_manifest_sha256": small["fixture_manifest_sha256"],
        "hardware_comparable": True,
        "candidates": {
            "small": candidate_summary(small),
            "tiny": candidate_summary(tiny),
        },
        "release_decision": "NOT_APPROVED",
        "limitations": [
            "This comparison does not certify full-application RSS or installed footprint.",
            "A candidate model remains unapproved until all PRD quality, latency, memory, disk, privacy, hardware, and licensing gates pass.",
        ],
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Compare Small and Tiny measured results on one approved corpus and hardware.")
    parser.add_argument("--small", type=Path, required=True)
    parser.add_argument("--tiny", type=Path, required=True)
    arguments = parser.parse_args(argv)
    try:
        result = compare_candidate_results(load_result(arguments.small), load_result(arguments.tiny))
    except (ComparisonBlocked, BenchmarkError, MeasuredResultsError) as error:
        print(f"BLOCKED candidate comparison: {error}", file=sys.stderr)
        return 2
    json.dump(result, sys.stdout, indent=2, sort_keys=True)
    sys.stdout.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
