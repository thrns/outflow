#!/usr/bin/env python3
"""Validate Outflow's development-only hardware inventory."""

from __future__ import annotations

import json
import math
import sys
from pathlib import Path
from typing import Any


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MATRIX_PATH = REPOSITORY_ROOT / "docs" / "benchmark-matrix.json"
REQUIRED_TARGET_IDS = {
    "macos_13_plus_arm64",
    "macos_13_plus_intel_x64",
    "windows_10_11_x64",
    "ubuntu_family_x11_x64",
    "fedora_family_x11_x64",
    "gnome_kde_wayland_x64",
}
ALLOWED_AVAILABILITY = {"AVAILABLE", "PENDING", "UNAVAILABLE"}
EVIDENCE_FIELDS = (
    "os_name",
    "os_version",
    "os_build",
    "cpu_model",
    "architecture",
    "physical_ram_gb",
)
REQUIRED_HARDWARE_FIELDS = ("machine_label", *EVIDENCE_FIELDS)
SENSITIVE_KEY_FRAGMENTS = {
    "serial",
    "uuid",
    "macaddress",
    "hostname",
    "username",
    "homedirectory",
    "appleid",
}


def _contains_sensitive_identification(value: Any) -> bool:
    """Return whether a JSON object uses an obviously identifying field name."""
    if isinstance(value, dict):
        for key, child in value.items():
            normalized_key = "".join(character for character in str(key).lower() if character.isalnum())
            if any(fragment in normalized_key for fragment in SENSITIVE_KEY_FRAGMENTS):
                return True
            if _contains_sensitive_identification(child):
                return True
    elif isinstance(value, list):
        return any(_contains_sensitive_identification(item) for item in value)
    return False


def validate_matrix(data: Any) -> tuple[list[str], dict[str, int]]:
    """Return validation errors and counts for a parsed matrix document."""
    errors: list[str] = []
    counts = {status: 0 for status in ALLOWED_AVAILABILITY}
    counts["complete_hardware_records"] = 0
    counts["physical_4gb_machines"] = 0

    if not isinstance(data, dict):
        return ["root must be a JSON object"], counts

    schema_version = data.get("schema_version")
    if not isinstance(schema_version, int) or isinstance(schema_version, bool) or schema_version != 1:
        errors.append("schema_version must be 1")
    if data.get("required_physical_ram_gb") != 4:
        errors.append("required_physical_ram_gb must be 4")
    if data.get("cpu_only_required") is not True:
        errors.append("cpu_only_required must be true")
    if data.get("performance_certification") != "NOT_ASSESSED":
        errors.append("performance_certification must be NOT_ASSESSED")
    if _contains_sensitive_identification(data):
        errors.append("matrix contains a prohibited sensitive identification field")

    targets = data.get("targets")
    if not isinstance(targets, list):
        errors.append("targets must be a list")
        return errors, counts

    seen_ids: set[str] = set()
    for index, target in enumerate(targets):
        label = f"targets[{index}]"
        if not isinstance(target, dict):
            errors.append(f"{label} must be an object")
            continue

        target_id = target.get("id")
        if not isinstance(target_id, str) or not target_id:
            errors.append(f"{label}.id must be a non-empty string")
        elif target_id in seen_ids:
            errors.append(f"duplicate target identifier: {target_id}")
        else:
            seen_ids.add(target_id)

        availability = target.get("availability")
        if not isinstance(availability, str) or availability not in ALLOWED_AVAILABILITY:
            errors.append(f"{label}.availability must be one of {sorted(ALLOWED_AVAILABILITY)}")
            continue
        counts[availability] += 1

        if target.get("required_physical_ram_gb") != 4:
            errors.append(f"{label}.required_physical_ram_gb must be 4")
        if target.get("cpu_only_required") is not True:
            errors.append(f"{label}.cpu_only_required must be true")
        if not isinstance(target.get("physical_4gb_test_machine_verified"), bool):
            errors.append(f"{label}.physical_4gb_test_machine_verified must be boolean")

        hardware = target.get("hardware")
        evidence = target.get("evidence")
        if not isinstance(hardware, dict):
            errors.append(f"{label}.hardware must be an object")
            hardware = {}
        if not isinstance(evidence, dict):
            errors.append(f"{label}.evidence must be an object")
            evidence = {}

        if availability == "AVAILABLE":
            for field in REQUIRED_HARDWARE_FIELDS:
                value = hardware.get(field)
                if field == "physical_ram_gb":
                    valid_value = (
                        (isinstance(value, int) and not isinstance(value, bool) and value > 0)
                        or (isinstance(value, float) and math.isfinite(value) and value > 0)
                    )
                else:
                    valid_value = isinstance(value, str) and bool(value.strip())
                if not valid_value:
                    errors.append(f"{label}.hardware.{field} must be verified for AVAILABLE hardware")

            for field in EVIDENCE_FIELDS:
                source = evidence.get(field)
                if not isinstance(source, str) or not source.strip():
                    errors.append(f"{label}.evidence.{field} must identify its verification source")

            if all(
                field in hardware and field in evidence and isinstance(evidence[field], str) and evidence[field].strip()
                for field in EVIDENCE_FIELDS
            ) and all(hardware.get(field) not in (None, "") for field in REQUIRED_HARDWARE_FIELDS):
                counts["complete_hardware_records"] += 1

        claims_physical_4gb = target.get("physical_4gb_test_machine_verified") is True
        if claims_physical_4gb:
            physical_ram = hardware.get("physical_ram_gb")
            matching_ram_evidence = isinstance(evidence.get("physical_ram_gb"), str) and bool(
                evidence["physical_ram_gb"].strip()
            )
            if availability != "AVAILABLE" or physical_ram != 4 or not matching_ram_evidence:
                errors.append(
                    f"{label} claims physical 4 GB verification without AVAILABLE hardware evidence of exactly 4 GB"
                )
            else:
                counts["physical_4gb_machines"] += 1

    missing_ids = sorted(REQUIRED_TARGET_IDS - seen_ids)
    if missing_ids:
        errors.append(f"missing required target identifiers: {', '.join(missing_ids)}")

    return errors, counts


def main() -> int:
    """Validate the repository matrix and print a concise inventory summary."""
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_MATRIX_PATH
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"Validation: FAIL — cannot read matrix ({error.__class__.__name__}).", file=sys.stderr)
        return 1

    errors, counts = validate_matrix(data)
    if errors:
        print(f"Validation: FAIL — {len(errors)} issue(s).")
        for error in errors:
            print(f"- {error}")
        return 1

    target_count = len(data["targets"])
    print(
        "Validation: PASS — "
        f"{target_count} targets; {counts['complete_hardware_records']} complete hardware record(s); "
        f"{counts['AVAILABLE']} AVAILABLE, {counts['PENDING']} PENDING, "
        f"{counts['UNAVAILABLE']} UNAVAILABLE."
    )
    print(
        "Physical 4 GB CPU-only machines verified: "
        f"{counts['physical_4gb_machines']}; performance certification: NOT ASSESSED."
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
