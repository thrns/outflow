#!/usr/bin/env python3
"""Validate the planned Moonshine third-party notice manifest.

This standard-library-only validator compares notice metadata with the local
Moonshine source inventory. It makes no network requests, writes no files, and
does not verify packaged artifacts or approve redistribution.
"""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path, PurePosixPath, PureWindowsPath
import sys
from typing import Any
from urllib.parse import urlsplit

# Loading the existing validator must not create bytecode-cache files as a
# side effect of a read-only validation run.
_previous_dont_write_bytecode = sys.dont_write_bytecode
sys.dont_write_bytecode = True
try:  # Support both direct script execution and imports by unittest.
    from .validate_moonshine_third_party import load_inventory, validate_inventory
except ImportError:  # pragma: no cover - exercised by direct CLI invocation.
    from validate_moonshine_third_party import load_inventory, validate_inventory
finally:
    sys.dont_write_bytecode = _previous_dont_write_bytecode


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST_PATH = REPOSITORY_ROOT / "docs" / "third-party-notices.json"
DEFAULT_INVENTORY_PATH = REPOSITORY_ROOT / "docs" / "moonshine-third-party.json"
MAX_MANIFEST_BYTES = 1_048_576
SUPPORTED_SCHEMA_VERSION = 1
PINNED_COMMIT = "234f60faa0eb388b01cdf7e60aca232af37aefda"
UPSTREAM_REPOSITORY = "moonshine-ai/moonshine"
NOTICE_STATES = {"planned", "blocked"}
ROOT_FIELDS = {
    "schema_version",
    "upstream_repository",
    "upstream_commit",
    "source_inventory",
    "redistribution_approval",
    "dependencies",
}
DEPENDENCY_FIELDS = {
    "dependency_id",
    "component_name",
    "license_expression",
    "license_evidence_url",
    "build_use_classification",
    "notice_required_if_shipped",
    "planned_notice_destination",
    "notice_verification_status",
    "status_explanation",
    "outstanding_questions",
    "redistribution_approval",
}


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Reject duplicate JSON object keys at every nesting level."""
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON object key")
        result[key] = value
    return result


def load_manifest(path: str | os.PathLike[str]) -> Any:
    """Load a bounded UTF-8 manifest without making network requests."""
    try:
        with Path(path).open("rb") as manifest_file:
            contents = manifest_file.read(MAX_MANIFEST_BYTES + 1)
    except (OSError, TypeError, ValueError) as error:
        raise ValueError("manifest could not be read") from error
    if len(contents) > MAX_MANIFEST_BYTES:
        raise ValueError("manifest exceeds the supported size limit")
    try:
        return json.loads(contents.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeError, json.JSONDecodeError, ValueError, RecursionError) as error:
        raise ValueError("manifest is not valid UTF-8 JSON") from error


def _check_fields(
    value: dict[str, Any], required: set[str], label: str, errors: list[str]
) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        errors.append(f"{label} is missing required field(s): {', '.join(sorted(missing))}")
    if extra:
        errors.append(f"{label} contains unsupported field(s): {', '.join(sorted(extra))}")


def _valid_immutable_license_url(value: Any, commit: str, dependency: dict[str, Any]) -> bool:
    """Require an HTTPS GitHub blob URL cited by this dependency's audit record."""
    if not _nonempty_string(value) or any(character.isspace() for character in value):
        return False
    try:
        parsed = urlsplit(value)
        _ = parsed.port
        return (
            parsed.scheme == "https"
            and parsed.hostname == "github.com"
            and parsed.username is None
            and parsed.password is None
            and not parsed.query
            and not parsed.fragment
            and parsed.path.startswith(f"/{UPSTREAM_REPOSITORY}/blob/{commit}/")
            and value in dependency.get("license_evidence_urls", [])
        )
    except (TypeError, ValueError):
        return False


def _safe_destination(value: Any) -> bool:
    """Accept null or a clean repository-relative POSIX path."""
    if value is None:
        return True
    if not _nonempty_string(value) or "\\" in value or "\x00" in value:
        return False
    posix_path = PurePosixPath(value)
    windows_path = PureWindowsPath(value)
    return (
        not posix_path.is_absolute()
        and not windows_path.is_absolute()
        and windows_path.drive == ""
        and all(part not in {"", ".", ".."} for part in value.split("/"))
    )


def validate_manifest(
    data: Any, inventory: Any
) -> tuple[list[str], dict[str, int]]:
    """Return local consistency errors and deterministic notice-state counts."""
    counts = {"dependencies": 0, "planned": 0, "blocked": 0, "packaged": 0}
    errors: list[str] = []

    if not isinstance(inventory, dict):
        return ["source inventory root must be a JSON object"], counts
    inventory_errors, _ = validate_inventory(inventory)
    if inventory_errors:
        return ["source inventory is invalid; run validate_moonshine_third_party.py"], counts
    if inventory.get("reviewed_moonshine_commit") != PINNED_COMMIT:
        return [f"source inventory must use pinned commit {PINNED_COMMIT}"], counts

    if not isinstance(data, dict):
        return ["manifest root must be a JSON object"], counts
    _check_fields(data, ROOT_FIELDS, "manifest", errors)
    if type(data.get("schema_version")) is not int or data.get("schema_version") != SUPPORTED_SCHEMA_VERSION:
        errors.append(f"schema_version must be {SUPPORTED_SCHEMA_VERSION}")
    if data.get("upstream_repository") != UPSTREAM_REPOSITORY:
        errors.append(f"upstream_repository must be {UPSTREAM_REPOSITORY}")
    if data.get("upstream_commit") != inventory.get("reviewed_moonshine_commit"):
        errors.append("upstream_commit must match the source inventory's reviewed commit")
    if data.get("upstream_commit") != PINNED_COMMIT:
        errors.append(f"upstream_commit must be {PINNED_COMMIT}")
    if data.get("source_inventory") != "docs/moonshine-third-party.json":
        errors.append("source_inventory must identify docs/moonshine-third-party.json")
    if data.get("redistribution_approval") != "undetermined":
        errors.append("redistribution_approval must remain undetermined")

    audited_dependencies = inventory.get("dependencies")
    if not isinstance(audited_dependencies, list) or len(audited_dependencies) != 8:
        errors.append("source inventory must contain exactly eight audited dependencies")
        return errors, counts
    audited_by_id = {item["dependency_id"]: item for item in audited_dependencies}

    dependencies = data.get("dependencies")
    if not isinstance(dependencies, list):
        errors.append("dependencies must be a list")
        return errors, counts
    counts["dependencies"] = len(dependencies)

    seen_ids: set[str] = set()
    for index, dependency in enumerate(dependencies):
        label = f"dependencies[{index}]"
        if not isinstance(dependency, dict):
            errors.append(f"{label} must be an object")
            continue
        _check_fields(dependency, DEPENDENCY_FIELDS, label, errors)
        dependency_id = dependency.get("dependency_id")
        if not _nonempty_string(dependency_id):
            errors.append(f"{label}.dependency_id must be a non-empty string")
            continue
        if dependency_id in seen_ids:
            errors.append(f"{label}.dependency_id duplicates another dependency identifier")
            continue
        seen_ids.add(dependency_id)

        audited = audited_by_id.get(dependency_id)
        if audited is None:
            errors.append(f"{label}.dependency_id is not present in the source inventory")
            continue

        if dependency.get("component_name") != audited.get("component_name"):
            errors.append(f"{label}.component_name must match the source inventory")
        identifiers = audited.get("license_identifiers")
        if not isinstance(identifiers, list) or not identifiers:
            errors.append(f"{label} has no verified license identifiers in the source inventory")
        else:
            expected_expression = " AND ".join(identifiers)
            if dependency.get("license_expression") != expected_expression:
                errors.append(f"{label}.license_expression must match the source inventory")
        if dependency.get("build_use_classification") != audited.get("build_use_classification"):
            errors.append(f"{label}.build_use_classification must match the source inventory")

        if not _valid_immutable_license_url(
            dependency.get("license_evidence_url"), PINNED_COMMIT, audited
        ):
            errors.append(
                f"{label}.license_evidence_url must be an immutable upstream license URL from the source inventory"
            )

        required_if_shipped = bool(audited.get("attribution_or_notice_requirements"))
        if type(dependency.get("notice_required_if_shipped")) is not bool or dependency.get(
            "notice_required_if_shipped"
        ) is not required_if_shipped:
            errors.append(
                f"{label}.notice_required_if_shipped must reflect the source inventory's notice requirements"
            )

        if not _safe_destination(dependency.get("planned_notice_destination")):
            errors.append(f"{label}.planned_notice_destination must be null or a safe repository-relative path")

        state = dependency.get("notice_verification_status")
        if not isinstance(state, str) or state not in NOTICE_STATES:
            errors.append(f"{label}.notice_verification_status is unsupported; use planned or blocked")
        else:
            counts[state] += 1
            if state == "blocked" and not _nonempty_string(dependency.get("status_explanation")):
                errors.append(f"{label}.status_explanation is required for a blocked notice")
        if state == "planned" and not _nonempty_string(dependency.get("status_explanation")):
            errors.append(f"{label}.status_explanation must explain the planned notice state")

        questions = dependency.get("outstanding_questions")
        if not isinstance(questions, list) or any(not _nonempty_string(question) for question in questions):
            errors.append(f"{label}.outstanding_questions must be a list of non-empty strings")
        elif not set(audited.get("outstanding_questions", [])).issubset(set(questions)):
            errors.append(f"{label}.outstanding_questions must carry forward all source inventory questions")

        if dependency.get("redistribution_approval") != "undetermined":
            errors.append(f"{label}.redistribution_approval must remain undetermined")

    missing_ids = set(audited_by_id) - seen_ids
    if missing_ids:
        errors.append("dependencies are missing one or more IDs from the source inventory")
    unknown_ids = seen_ids - set(audited_by_id)
    if unknown_ids:
        errors.append("dependencies contain IDs not present in the source inventory")
    if len(dependencies) != 8:
        errors.append("dependencies must contain exactly eight audited dependency IDs")

    return errors, counts


def render_summary(data: dict[str, Any], counts: dict[str, int]) -> str:
    """Render a stable readiness summary; supported states cannot claim packaging."""
    readiness = "INCOMPLETE" if counts["packaged"] != counts["dependencies"] else "READY"
    return "\n".join(
        (
            "Outflow third-party notice manifest valid",
            f"Manifest version: {data['schema_version']}",
            f"Dependencies: {counts['dependencies']}",
            f"Packaged notices: {counts['packaged']}",
            f"Redistribution approval: {data['redistribution_approval'].upper()}",
            f"Notice readiness: {readiness}",
            f"Notice states: {counts['planned']} planned, {counts['blocked']} blocked",
        )
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", default=DEFAULT_MANIFEST_PATH, help="notice manifest JSON path")
    parser.add_argument("--inventory", default=DEFAULT_INVENTORY_PATH, help="audited dependency inventory JSON path")
    arguments = parser.parse_args(argv)

    try:
        manifest = load_manifest(arguments.manifest)
        inventory = load_inventory(arguments.inventory)
    except ValueError as error:
        print(f"Third-party notice manifest invalid: {error}", file=sys.stderr)
        return 1

    errors, counts = validate_manifest(manifest, inventory)
    if errors:
        print("Third-party notice manifest invalid", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(render_summary(manifest, counts))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
