#!/usr/bin/env python3
"""Validate separate, human-reviewed hardware evidence for ASR comparisons.

The benchmark result remains immutable. This sidecar only grants permission to
compare matching measurements after a reviewer verifies the machine identity,
physical status, and CPU-only measurement basis. It never approves a model,
certifies a 4 GB release target, or edits result fields.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
from typing import Any, Mapping

if __package__:
    from .run_native_asr_benchmark import _reject_constant, _unique_object
    from .validate_measured_asr_results import MeasuredResultsError, validate_measured_results
else:
    from run_native_asr_benchmark import _reject_constant, _unique_object
    from validate_measured_asr_results import MeasuredResultsError, validate_measured_results


MAX_HARDWARE_EVIDENCE_BYTES = 256 * 1024
SHA256_PATTERN = re.compile(r"[0-9a-f]{64}\Z")
MACHINE_SNAPSHOT_FIELDS = (
    "machine_label",
    "machine_identifier",
    "os",
    "os_release",
    "architecture",
    "physical_memory_bytes",
    "physical_memory_source",
    "cpu_label",
)


class HardwareEvidenceError(ValueError):
    """A missing, malformed, changed, or mismatched hardware review record."""


def load_hardware_evidence(path: Path) -> dict[str, Any]:
    """Read one bounded, duplicate-key-free review sidecar."""

    try:
        with path.open("rb") as evidence_file:
            raw = evidence_file.read(MAX_HARDWARE_EVIDENCE_BYTES + 1)
    except OSError as error:
        raise HardwareEvidenceError("hardware review evidence could not be read") from error
    if len(raw) > MAX_HARDWARE_EVIDENCE_BYTES:
        raise HardwareEvidenceError("hardware review evidence exceeds the size limit")
    try:
        value = json.loads(
            raw.decode("utf-8"),
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (UnicodeError, json.JSONDecodeError, ValueError, RecursionError) as error:
        raise HardwareEvidenceError("hardware review evidence is not valid JSON") from error
    if not isinstance(value, dict):
        raise HardwareEvidenceError("hardware review evidence has an invalid shape")
    return value


def validate_hardware_evidence(
    result: Mapping[str, Any],
    evidence: Mapping[str, Any],
    *,
    measured_result_sha256: str,
) -> dict[str, str]:
    """Validate review scope, result binding, session identity, and machine facts.

    The returned summary contains only comparison permission and the approved
    reference class. User-supplied evidence references and reviewer identity
    remain in the sidecar and are not copied into comparison output.
    """

    try:
        validate_measured_results(dict(result))
    except (MeasuredResultsError, TypeError) as error:
        raise HardwareEvidenceError("measured result failed validation") from error

    if not isinstance(measured_result_sha256, str) or not SHA256_PATTERN.fullmatch(measured_result_sha256):
        raise HardwareEvidenceError("measured result identity is invalid")
    if not isinstance(evidence, Mapping) or set(evidence) != {
        "schema_version",
        "evidence_type",
        "comparison_permission",
        "measured_result_sha256",
        "measurement",
        "review",
    }:
        raise HardwareEvidenceError("hardware review evidence shape is invalid")
    if (
        type(evidence["schema_version"]) is not int
        or evidence["schema_version"] != 1
        or evidence["evidence_type"] != "outflow_hardware_review"
        or evidence["comparison_permission"] != "approved_for_candidate_comparison_only"
        or evidence["measured_result_sha256"] != measured_result_sha256
    ):
        raise HardwareEvidenceError("hardware review evidence does not bind to this measured result")

    measurement = evidence["measurement"]
    if not isinstance(measurement, Mapping) or set(measurement) != {
        "candidate",
        "created_at_utc",
        "fixture_manifest_sha256",
        "machine",
    }:
        raise HardwareEvidenceError("hardware review measurement identity is invalid")
    machine = result["machine"]
    if (
        measurement["candidate"] != result["model"]["candidate"]
        or measurement["created_at_utc"] != result["created_at_utc"]
        or measurement["fixture_manifest_sha256"] != result["fixture_manifest_sha256"]
    ):
        raise HardwareEvidenceError("hardware review session identity does not match the measured result")

    machine_snapshot = measurement["machine"]
    if (
        not isinstance(machine_snapshot, Mapping)
        or set(machine_snapshot) != set(MACHINE_SNAPSHOT_FIELDS)
        or any(machine_snapshot[field] != machine[field] for field in MACHINE_SNAPSHOT_FIELDS)
        or not _nonempty_string(machine["machine_label"])
        or type(machine["physical_memory_bytes"]) is not int
        or machine["physical_memory_bytes"] <= 0
        or machine["cpu_only_inference_exercised"] is not True
    ):
        raise HardwareEvidenceError("hardware review machine identity does not match the measured result")

    review = evidence["review"]
    expected_review_fields = {
        "reviewer_id",
        "approval_reference",
        "machine_identity_verified",
        "machine_identity_evidence_reference",
        "physical_hardware_status",
        "physical_hardware_evidence_reference",
        "physical_memory_evidence_reference",
        "reference_hardware_class",
        "reference_class_evidence_reference",
        "real_hardware_measurements_completed",
        "cpu_only_inference_exercised",
    }
    if not isinstance(review, Mapping) or set(review) != expected_review_fields:
        raise HardwareEvidenceError("hardware review approval record is incomplete")
    evidence_references = (
        "approval_reference",
        "machine_identity_evidence_reference",
        "physical_hardware_evidence_reference",
        "physical_memory_evidence_reference",
        "reference_class_evidence_reference",
    )
    if (
        not _nonempty_string(review["reviewer_id"])
        or any(not _nonempty_string(review[field]) for field in evidence_references)
        or review["machine_identity_verified"] is not True
        or review["physical_hardware_status"] != "verified_physical"
        or not _nonempty_string(review["reference_hardware_class"])
        or review["real_hardware_measurements_completed"] is not True
        or review["cpu_only_inference_exercised"] is not True
    ):
        raise HardwareEvidenceError("hardware review does not approve physical CPU measurements")

    return {
        "comparison_permission": "approved_for_candidate_comparison_only",
        "reference_hardware_class": review["reference_hardware_class"],
    }


def sha256_bytes(raw: bytes) -> str:
    """Return the lowercase SHA-256 digest of exact measured-result bytes."""

    return hashlib.sha256(raw).hexdigest()


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip()) and len(value) <= 512 and not any(
        ord(character) < 32 for character in value
    )
