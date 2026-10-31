#!/usr/bin/env python3
"""Check the local Moonshine licensing review record for schema consistency.

This standard-library-only tool does not contact upstream sources, validate
legal conclusions, inspect model files, or approve redistribution. It checks
that source-document evidence is not represented as artifact integrity proof.
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
DEFAULT_REVIEW_PATH = REPOSITORY_ROOT / "docs" / "moonshine-license-review.json"
MAX_REVIEW_BYTES = 1_048_576
SUPPORTED_SCHEMA_VERSION = 1
EVIDENCE_STATUSES = {"upstream-documented", "unverified", "conflicting"}
EVIDENCE_SCOPES = {
    "model-family-documentation",
    "repository-license-policy",
    "third-party-source-directory",
    "license-text",
    "model-artifact",
    "runtime-artifact",
}
REVISION_STATUSES = {"verified", "unverified"}
COMMIT_PATTERN = re.compile(r"[0-9a-f]{40}\Z")
INVALID_PERCENT_ESCAPE = re.compile(r"%(?![0-9a-fA-F]{2})")
PLACEHOLDER_COMMITS = {
    "0" * 40,
    "f" * 40,
    "1" * 40,
    "deadbeef" * 5,
    "0123456789abcdef" * 2 + "01234567",
    "abcdef0123456789" * 2 + "abcdef01",
}
ROOT_FIELDS = {
    "schema_version",
    "review_date",
    "redistribution_approval",
    "model_file_integrity_status",
    "runtime_binary_integrity_status",
    "claims",
}
CLAIM_FIELDS = {
    "claim_id",
    "review_date",
    "component_or_model_id",
    "claimed_license_or_boundary",
    "evidence_status",
    "evidence_scope",
    "sources",
    "remaining_limitations",
}
SOURCE_FIELDS = {
    "source_organization",
    "source_repository",
    "source_document_path",
    "evidence_url",
    "relevant_section",
    "source_revision",
}
REVISION_FIELDS = {"status", "commit", "note"}


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


def load_review(path: str | os.PathLike[str]) -> Any:
    """Load a bounded UTF-8 review record without making network requests."""
    try:
        with Path(path).open("rb") as review_file:
            contents = review_file.read(MAX_REVIEW_BYTES + 1)
    except (OSError, TypeError, ValueError) as error:
        raise ValueError("review record could not be read") from error

    if len(contents) > MAX_REVIEW_BYTES:
        raise ValueError("review record exceeds the supported size limit")
    try:
        decoded = contents.decode("utf-8")
        return json.loads(decoded, object_pairs_hook=_unique_object)
    except (UnicodeError, json.JSONDecodeError, ValueError, RecursionError) as error:
        raise ValueError("review record is not valid UTF-8 JSON") from error


def _check_fields(
    value: dict[str, Any], required: set[str], label: str, errors: list[str]
) -> None:
    if required - value.keys():
        errors.append(f"{label} is missing required field(s)")
    if value.keys() - required:
        errors.append(f"{label} contains unsupported field(s)")


def _valid_source_url(value: Any) -> bool:
    """Accept complete HTTPS source URLs while rejecting ambiguous forms."""
    if not _nonempty_string(value) or any(character.isspace() for character in value):
        return False
    if INVALID_PERCENT_ESCAPE.search(value):
        return False
    try:
        parsed = urlsplit(value)
        # Accessing .port also rejects malformed ports; source evidence is HTTPS only.
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


def _is_placeholder_commit(value: str) -> bool:
    return value in PLACEHOLDER_COMMITS or len(set(value)) == 1


def _validate_source(source: Any, label: str, errors: list[str]) -> None:
    if not isinstance(source, dict):
        errors.append(f"{label} must be an object")
        return
    _check_fields(source, SOURCE_FIELDS, label, errors)
    for field in (
        "source_organization",
        "source_repository",
        "source_document_path",
        "relevant_section",
    ):
        if not _nonempty_string(source.get(field)):
            errors.append(f"{label}.{field} must be a non-empty string")
    evidence_url = source.get("evidence_url")
    if not _valid_source_url(evidence_url):
        errors.append(f"{label}.evidence_url must be a complete HTTPS URL")

    revision = source.get("source_revision")
    revision_label = f"{label}.source_revision"
    if not isinstance(revision, dict):
        errors.append(f"{revision_label} must explicitly state revision status, commit, and note")
        return
    _check_fields(revision, REVISION_FIELDS, revision_label, errors)
    status = revision.get("status")
    commit = revision.get("commit")
    note = revision.get("note")
    if not isinstance(status, str) or status not in REVISION_STATUSES:
        errors.append(f"{revision_label}.status must be verified or unverified")
    if not _nonempty_string(note):
        errors.append(f"{revision_label}.note must explain the revision state")

    if status == "verified":
        if not isinstance(commit, str) or not COMMIT_PATTERN.fullmatch(commit):
            errors.append(f"{revision_label}.commit must be a full 40-character lowercase Git SHA")
        elif _is_placeholder_commit(commit):
            errors.append(f"{revision_label}.commit must not be a placeholder hash")
        elif _valid_source_url(evidence_url):
            try:
                path = urlsplit(evidence_url).path
            except ValueError:
                path = ""
            if f"/blob/{commit}/" not in path and f"/tree/{commit}/" not in path:
                errors.append(f"{revision_label}.commit must appear in an immutable GitHub source URL")
    elif status == "unverified":
        if commit is not None:
            errors.append(f"{revision_label}.commit must be null when the revision is unverified")


def validate_review(data: Any) -> tuple[list[str], dict[str, int]]:
    """Return local consistency errors and evidence-status counts."""
    counts = {status: 0 for status in EVIDENCE_STATUSES}
    counts["claims"] = 0
    errors: list[str] = []

    if not isinstance(data, dict):
        return ["review root must be a JSON object"], counts
    _check_fields(data, ROOT_FIELDS, "review", errors)

    version = data.get("schema_version")
    if type(version) is not int or version != SUPPORTED_SCHEMA_VERSION:
        errors.append(f"schema_version must be {SUPPORTED_SCHEMA_VERSION}")

    reviewed = data.get("review_date")
    if not _nonempty_string(reviewed):
        errors.append("review_date must be an ISO calendar date")
    else:
        try:
            date.fromisoformat(reviewed)
        except ValueError:
            errors.append("review_date must be an ISO calendar date")

    if data.get("redistribution_approval") != "undetermined":
        errors.append("redistribution_approval must remain undetermined in this preliminary review")
    if data.get("model_file_integrity_status") != "not-verified":
        errors.append("model_file_integrity_status must remain not-verified")
    if data.get("runtime_binary_integrity_status") != "not-verified":
        errors.append("runtime_binary_integrity_status must remain not-verified")

    claims = data.get("claims")
    if not isinstance(claims, list):
        errors.append("claims must be a list")
        return errors, counts
    if not claims:
        errors.append("claims must contain at least one licensing claim")
    counts["claims"] = len(claims)

    seen_ids: set[str] = set()
    for index, claim in enumerate(claims):
        label = f"claims[{index}]"
        if not isinstance(claim, dict):
            errors.append(f"{label} must be an object")
            continue
        _check_fields(claim, CLAIM_FIELDS, label, errors)

        claim_id = claim.get("claim_id")
        if not _nonempty_string(claim_id):
            errors.append(f"{label}.claim_id must be a non-empty string")
        elif claim_id in seen_ids:
            errors.append(f"{label}.claim_id duplicates another claim identifier")
        else:
            seen_ids.add(claim_id)

        for field in ("component_or_model_id", "claimed_license_or_boundary"):
            if not _nonempty_string(claim.get(field)):
                errors.append(f"{label}.{field} must be a non-empty string")
        claim_review_date = claim.get("review_date")
        if not _nonempty_string(claim_review_date):
            errors.append(f"{label}.review_date must be an ISO calendar date")
        else:
            try:
                date.fromisoformat(claim_review_date)
            except ValueError:
                errors.append(f"{label}.review_date must be an ISO calendar date")
        if _nonempty_string(reviewed) and claim_review_date != reviewed:
            errors.append(f"{label}.review_date must match the review date")

        evidence_status = claim.get("evidence_status")
        if not isinstance(evidence_status, str) or evidence_status not in EVIDENCE_STATUSES:
            errors.append(f"{label}.evidence_status is unsupported")
        else:
            counts[evidence_status] += 1

        evidence_scope = claim.get("evidence_scope")
        if not isinstance(evidence_scope, str) or evidence_scope not in EVIDENCE_SCOPES:
            errors.append(f"{label}.evidence_scope is unsupported")
        elif (
            evidence_status == "upstream-documented"
            and evidence_scope in {"model-artifact", "runtime-artifact"}
        ):
            errors.append(
                f"{label} cannot use documentation evidence as verified artifact evidence"
            )

        limitations = claim.get("remaining_limitations")
        if not isinstance(limitations, list) or not limitations or any(
            not _nonempty_string(item) for item in limitations
        ):
            errors.append(f"{label}.remaining_limitations must be a non-empty list of explanations")

        sources = claim.get("sources")
        if not isinstance(sources, list) or not sources:
            errors.append(f"{label}.sources must contain a source reference")
        else:
            for source_index, source in enumerate(sources):
                _validate_source(source, f"{label}.sources[{source_index}]", errors)
            if evidence_status == "upstream-documented" and not any(
                isinstance(source, dict)
                and _nonempty_string(source.get("source_document_path"))
                and _valid_source_url(source.get("evidence_url"))
                for source in sources
            ):
                errors.append(f"{label} documented claim needs a meaningful source reference")

    return errors, counts


def render_summary(review: dict[str, Any], counts: dict[str, int]) -> str:
    """Render a stable summary that contains no source paths or artifact data."""
    return "\n".join(
        (
            "Outflow Moonshine licensing review valid",
            f"Review date: {review['review_date']}",
            f"Claims: {counts['claims']}",
            "Evidence status: "
            f"{counts['upstream-documented']} upstream-documented, "
            f"{counts['unverified']} unverified, {counts['conflicting']} conflicting",
            "Redistribution approval: UNDETERMINED",
            "Model file integrity: NOT VERIFIED",
            "Runtime binary integrity: NOT VERIFIED",
        )
    )


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Validate a local Moonshine licensing review record (no network access)."
    )
    parser.add_argument(
        "--file",
        type=Path,
        default=DEFAULT_REVIEW_PATH,
        help="path to the review JSON (default: docs/moonshine-license-review.json)",
    )
    args = parser.parse_args(argv)
    try:
        review = load_review(args.file)
    except ValueError as error:
        print(f"Moonshine license review validation failed: {error}", file=sys.stderr)
        return 1

    errors, counts = validate_review(review)
    if errors:
        for error in errors:
            print(f"Moonshine license review validation failed: {error}", file=sys.stderr)
        return 1

    print(render_summary(review, counts))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
