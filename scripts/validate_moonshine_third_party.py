#!/usr/bin/env python3
"""Validate the local source-only Moonshine third-party inventory.

This standard-library-only validator checks inventory structure, immutable
source references, and conservative review states. It makes no network
requests and does not determine whether a license permits redistribution or
verify source files, runtime binaries, installers, or models.
"""

from __future__ import annotations

import argparse
from datetime import date
import json
import os
from pathlib import Path
import re
import sys
from typing import Any
from urllib.parse import urlsplit


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST_PATH = REPOSITORY_ROOT / "docs" / "moonshine-third-party.json"
MAX_MANIFEST_BYTES = 1_048_576
SUPPORTED_SCHEMA_VERSION = 1
PINNED_COMMIT = "234f60faa0eb388b01cdf7e60aca232af37aefda"
UPSTREAM_REPOSITORY = "moonshine-ai/moonshine"
BUILD_USE_CLASSIFICATIONS = {
    "confirmed-build-input",
    "conditional",
    "test-only",
    "unknown",
}
DEPENDENCY_CATEGORIES = {
    "linear-algebra",
    "test-framework",
    "audio-feature-extraction",
    "fft",
    "json-library",
    "inference-runtime",
    "utf8-library",
    "unicode-processing",
}
REQUIRED_DIRECTORIES = {
    "core/third-party/Eigen",
    "core/third-party/doctest",
    "core/third-party/kaldi-native-fbank",
    "core/third-party/kissfft",
    "core/third-party/nlohmann",
    "core/third-party/onnxruntime",
    "core/third-party/utf-8",
    "core/third-party/utf8proc",
}
COMMIT_PATTERN = re.compile(r"[0-9a-f]{40}\Z")
PLACEHOLDER_COMMITS = {"0" * 40, "f" * 40, "1" * 40}
ROOT_FIELDS = {
    "schema_version",
    "review_date",
    "upstream_repository",
    "reviewed_moonshine_commit",
    "evidence_scope",
    "binary_verification_status",
    "redistribution_approval",
    "dependencies",
}
DEPENDENCY_FIELDS = {
    "dependency_id",
    "upstream_directory",
    "reviewed_moonshine_commit",
    "component_name",
    "supplier",
    "dependency_category",
    "license_status",
    "license_identifiers",
    "license_evidence_urls",
    "attribution_or_notice_requirements",
    "source_build_evidence_urls",
    "build_use_classification",
    "redistribution_review_status",
    "runtime_use_assessment",
    "binary_verification_status",
    "outstanding_questions",
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


def load_inventory(path: str | os.PathLike[str]) -> Any:
    """Load a bounded UTF-8 JSON inventory without contacting upstream."""
    try:
        with Path(path).open("rb") as manifest_file:
            contents = manifest_file.read(MAX_MANIFEST_BYTES + 1)
    except (OSError, TypeError, ValueError) as error:
        raise ValueError("inventory could not be read") from error

    if len(contents) > MAX_MANIFEST_BYTES:
        raise ValueError("inventory exceeds the supported size limit")
    try:
        return json.loads(contents.decode("utf-8"), object_pairs_hook=_unique_object)
    except (UnicodeError, json.JSONDecodeError, ValueError, RecursionError) as error:
        raise ValueError("inventory is not valid UTF-8 JSON") from error


def _check_fields(
    value: dict[str, Any], required: set[str], label: str, errors: list[str]
) -> None:
    missing = required - value.keys()
    extra = value.keys() - required
    if missing:
        errors.append(f"{label} is missing required field(s)")
    if extra:
        errors.append(f"{label} contains unsupported field(s)")


def _valid_commit(value: Any) -> bool:
    return (
        isinstance(value, str)
        and COMMIT_PATTERN.fullmatch(value) is not None
        and value not in PLACEHOLDER_COMMITS
        and len(set(value)) > 1
    )


def _valid_https_url(value: Any) -> bool:
    """Accept complete HTTPS URLs without credentials or mutable query data."""
    if not _nonempty_string(value) or any(character.isspace() for character in value):
        return False
    try:
        parsed = urlsplit(value)
        _ = parsed.port
        return (
            parsed.scheme == "https"
            and bool(parsed.hostname)
            and bool(parsed.path)
            and parsed.username is None
            and parsed.password is None
            and not parsed.query
            and not parsed.fragment
        )
    except ValueError:
        return False


def _is_pinned_upstream_url(value: Any, commit: str) -> bool:
    if not _valid_https_url(value):
        return False
    try:
        parsed = urlsplit(value)
    except ValueError:
        return False
    expected_prefix = f"/{UPSTREAM_REPOSITORY}/"
    if parsed.hostname != "github.com" or not parsed.path.startswith(expected_prefix):
        return False
    return f"/blob/{commit}/" in parsed.path or f"/tree/{commit}/" in parsed.path


def _check_url_list(
    value: Any,
    label: str,
    commit: str,
    errors: list[str],
    *,
    require_nonempty: bool,
) -> None:
    if not isinstance(value, list):
        errors.append(f"{label} must be a list of immutable upstream HTTPS URLs")
        return
    if require_nonempty and not value:
        errors.append(f"{label} must contain at least one evidence URL")
    for index, url in enumerate(value):
        url_label = f"{label}[{index}]"
        if not _valid_https_url(url):
            errors.append(f"{url_label} must be a complete HTTPS URL")
        elif not _is_pinned_upstream_url(url, commit):
            errors.append(
                f"{url_label} must reference {UPSTREAM_REPOSITORY} at the recorded source revision"
            )


def validate_inventory(data: Any) -> tuple[list[str], dict[str, int]]:
    """Return local consistency errors and deterministic inventory counts."""
    counts = {
        "dependencies": 0,
        "verified_licenses": 0,
        "unknown_licenses": 0,
        **{classification: 0 for classification in sorted(BUILD_USE_CLASSIFICATIONS)},
    }
    errors: list[str] = []

    if not isinstance(data, dict):
        return ["inventory root must be a JSON object"], counts
    _check_fields(data, ROOT_FIELDS, "inventory", errors)

    if type(data.get("schema_version")) is not int or data.get("schema_version") != SUPPORTED_SCHEMA_VERSION:
        errors.append(f"schema_version must be {SUPPORTED_SCHEMA_VERSION}")

    reviewed_date = data.get("review_date")
    if not _nonempty_string(reviewed_date):
        errors.append("review_date must be an ISO calendar date")
    else:
        try:
            date.fromisoformat(reviewed_date)
        except ValueError:
            errors.append("review_date must be an ISO calendar date")

    if data.get("upstream_repository") != UPSTREAM_REPOSITORY:
        errors.append(f"upstream_repository must be {UPSTREAM_REPOSITORY}")

    root_commit = data.get("reviewed_moonshine_commit")
    if not _valid_commit(root_commit):
        errors.append("reviewed_moonshine_commit must be a full non-placeholder Git SHA")
    elif root_commit != PINNED_COMMIT:
        errors.append("reviewed_moonshine_commit must match the task's pinned source revision")

    if data.get("evidence_scope") != "pinned-source-tree-only":
        errors.append("evidence_scope must state pinned-source-tree-only")
    if data.get("binary_verification_status") != "not-verified":
        errors.append("source review cannot claim binary verification")
    if data.get("redistribution_approval") != "undetermined":
        errors.append("redistribution_approval must remain undetermined")

    dependencies = data.get("dependencies")
    if not isinstance(dependencies, list):
        errors.append("dependencies must be a list")
        return errors, counts
    counts["dependencies"] = len(dependencies)

    seen_ids: set[str] = set()
    seen_directories: set[str] = set()
    for index, dependency in enumerate(dependencies):
        label = f"dependencies[{index}]"
        if not isinstance(dependency, dict):
            errors.append(f"{label} must be an object")
            continue
        _check_fields(dependency, DEPENDENCY_FIELDS, label, errors)

        dependency_id = dependency.get("dependency_id")
        if not _nonempty_string(dependency_id):
            errors.append(f"{label}.dependency_id must be a non-empty string")
        elif dependency_id in seen_ids:
            errors.append(f"{label}.dependency_id duplicates another dependency identifier")
        else:
            seen_ids.add(dependency_id)

        directory = dependency.get("upstream_directory")
        if not _nonempty_string(directory):
            errors.append(f"{label}.upstream_directory must be a non-empty string")
        elif directory in seen_directories:
            errors.append(f"{label}.upstream_directory duplicates another directory")
        else:
            seen_directories.add(directory)

        for field in (
            "component_name",
            "supplier",
            "attribution_or_notice_requirements",
            "runtime_use_assessment",
        ):
            if not _nonempty_string(dependency.get(field)):
                errors.append(f"{label}.{field} must be a non-empty string")

        if dependency.get("dependency_category") not in DEPENDENCY_CATEGORIES:
            errors.append(f"{label}.dependency_category is unsupported")

        item_commit = dependency.get("reviewed_moonshine_commit")
        if not _valid_commit(item_commit):
            errors.append(f"{label}.reviewed_moonshine_commit must be a full non-placeholder Git SHA")
        elif item_commit != root_commit:
            errors.append(f"{label}.reviewed_moonshine_commit must match the inventory revision")

        license_status = dependency.get("license_status")
        identifiers = dependency.get("license_identifiers")
        if not isinstance(identifiers, list) or any(
            not _nonempty_string(identifier) for identifier in identifiers
        ):
            errors.append(f"{label}.license_identifiers must be a list of non-empty strings")
            identifiers = []
        if license_status == "verified":
            counts["verified_licenses"] += 1
            if not identifiers:
                errors.append(f"{label} verified license claim needs at least one identifier")
            if _valid_commit(root_commit):
                _check_url_list(
                    dependency.get("license_evidence_urls"),
                    f"{label}.license_evidence_urls",
                    root_commit,
                    errors,
                    require_nonempty=True,
                )
                license_urls = dependency.get("license_evidence_urls")
                if isinstance(license_urls, list) and _nonempty_string(directory):
                    for url_index, url in enumerate(license_urls):
                        if _valid_https_url(url) and f"/{directory}/" not in urlsplit(url).path:
                            errors.append(
                                f"{label}.license_evidence_urls[{url_index}] must point inside its dependency directory"
                            )
            else:
                errors.append(f"{label} verified license claim lacks a valid recorded revision")
        elif license_status == "unknown":
            counts["unknown_licenses"] += 1
            if identifiers:
                errors.append(f"{label} unknown license must not claim license identifiers")
            if _valid_commit(root_commit):
                _check_url_list(
                    dependency.get("license_evidence_urls"),
                    f"{label}.license_evidence_urls",
                    root_commit,
                    errors,
                    require_nonempty=False,
                )
                license_urls = dependency.get("license_evidence_urls")
                if isinstance(license_urls, list) and _nonempty_string(directory):
                    for url_index, url in enumerate(license_urls):
                        if _valid_https_url(url) and f"/{directory}/" not in urlsplit(url).path:
                            errors.append(
                                f"{label}.license_evidence_urls[{url_index}] must point inside its dependency directory"
                            )
            else:
                errors.append(f"{label} unknown license claim lacks a valid recorded revision")
        else:
            errors.append(f"{label}.license_status must be verified or unknown")

        if _valid_commit(root_commit):
            _check_url_list(
                dependency.get("source_build_evidence_urls"),
                f"{label}.source_build_evidence_urls",
                root_commit,
                errors,
                require_nonempty=True,
            )

        build_use = dependency.get("build_use_classification")
        if not isinstance(build_use, str) or build_use not in BUILD_USE_CLASSIFICATIONS:
            errors.append(f"{label}.build_use_classification is unsupported")
        else:
            counts[build_use] += 1

        if dependency.get("redistribution_review_status") != "undetermined":
            errors.append(f"{label}.redistribution_review_status must remain undetermined")
        if dependency.get("binary_verification_status") != "not-verified":
            errors.append(f"{label} source review cannot claim binary verification")

        questions = dependency.get("outstanding_questions")
        if not isinstance(questions, list) or any(
            not _nonempty_string(question) for question in questions
        ):
            errors.append(f"{label}.outstanding_questions must be a list of non-empty strings")

    missing_directories = REQUIRED_DIRECTORIES - seen_directories
    extra_directories = seen_directories - REQUIRED_DIRECTORIES
    if missing_directories:
        errors.append("dependencies are missing one or more required pinned directories")
    if extra_directories:
        errors.append("dependencies contain an unsupported upstream directory")

    return errors, counts


def render_summary(data: dict[str, Any], counts: dict[str, int]) -> str:
    """Render a stable summary suitable for evidence logs and tests."""
    return "\n".join(
        (
            "Outflow Moonshine third-party inventory valid",
            f"Reviewed commit: {data['reviewed_moonshine_commit']}",
            f"Dependencies: {counts['dependencies']}",
            (
                "Build use: "
                f"{counts['confirmed-build-input']} confirmed, "
                f"{counts['conditional']} conditional, "
                f"{counts['test-only']} test-only, "
                f"{counts['unknown']} unknown"
            ),
            (
                "License review: "
                f"{counts['verified_licenses']} verified, "
                f"{counts['unknown_licenses']} unknown"
            ),
            "Redistribution approval: UNDETERMINED",
            "Binary verification: NOT VERIFIED",
        )
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        default=DEFAULT_MANIFEST_PATH,
        help="path to the local third-party inventory JSON",
    )
    arguments = parser.parse_args(argv)

    try:
        inventory = load_inventory(arguments.manifest)
    except ValueError as error:
        print(f"Moonshine third-party inventory invalid: {error}", file=sys.stderr)
        return 1

    errors, counts = validate_inventory(inventory)
    if errors:
        print("Moonshine third-party inventory invalid", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(render_summary(inventory, counts))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
