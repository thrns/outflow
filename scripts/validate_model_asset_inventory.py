#!/usr/bin/env python3
"""Validate planned and locally file-verified ASR asset inventory metadata.

This development-only tool is the structured inventory companion to C-09 and
TS-09. It checks local file integrity for explicitly file-verified entries;
it neither decides legal rights nor proves that files are bundled in an
installer. It uses only Python's standard library and never accesses a network.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import stat
import sys
from typing import Any, Sequence
from urllib.parse import urlparse


SCHEMA_VERSION = 1
MAX_MANIFEST_BYTES = 1_048_576
HASH_CHUNK_BYTES = 1_048_576
SHA256_PATTERN = re.compile(r"[0-9a-fA-F]{64}\Z")
WINDOWS_DRIVE_PATTERN = re.compile(r"[a-zA-Z]:")
ASSET_CATEGORIES = {"model", "tokenizer", "native-runtime", "license-notice"}
ASSET_STATUSES = {"planned", "file-verified"}
LICENSE_REVIEW_STATUSES = {"not-reviewed", "review-pending", "reviewed"}
REDISTRIBUTION_STATUSES = {"undetermined", "approved", "blocked"}
NOTICE_STATUSES = {"undetermined", "required", "not-required", "prepared"}
ROOT_FIELDS = {"schema_version", "assets"}
ASSET_FIELDS = {
    "asset_id",
    "category",
    "status",
    "source",
    "source_url",
    "source_revision",
    "file_path",
    "sha256",
    "provenance_reference",
    "license_identifier",
    "license_evidence_reference",
    "license_review_status",
    "redistribution_status",
    "redistribution_evidence_reference",
    "notice_attribution_status",
    "notice_attribution_evidence_reference",
}
REPOSITORY_ROOT = Path(__file__).resolve().parents[1]


class InventoryValidationError(ValueError):
    """A safe, content-free message for a manifest loading failure."""

    def __init__(self, errors: Sequence[str]) -> None:
        self.errors = tuple(errors)
        super().__init__("; ".join(self.errors))


class _DuplicateJsonKeyError(ValueError):
    """Signals a repeated key in a JSON object."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Build JSON objects while rejecting duplicate keys at every level."""
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise _DuplicateJsonKeyError
        result[key] = value
    return result


def _reject_nonstandard_number(_: str) -> None:
    """Reject non-standard JSON values such as NaN and Infinity."""
    raise ValueError


def load_manifest(path: str | os.PathLike[str]) -> Any:
    """Read a bounded UTF-8 JSON manifest without exposing its contents or path."""
    try:
        with Path(path).open("rb") as manifest_file:
            raw_contents = manifest_file.read(MAX_MANIFEST_BYTES + 1)
    except (OSError, TypeError, ValueError) as error:
        raise InventoryValidationError(("manifest could not be read",)) from error

    if len(raw_contents) > MAX_MANIFEST_BYTES:
        raise InventoryValidationError(("manifest exceeds the supported size limit",))
    try:
        contents = raw_contents.decode("utf-8")
    except UnicodeError as error:
        raise InventoryValidationError(("manifest is not valid UTF-8",)) from error

    try:
        return json.loads(
            contents,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_nonstandard_number,
        )
    except _DuplicateJsonKeyError as error:
        raise InventoryValidationError(("manifest contains duplicate JSON object keys",)) from error
    except (json.JSONDecodeError, ValueError, RecursionError) as error:
        raise InventoryValidationError(("manifest does not contain valid JSON",)) from error


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _check_fields(
    value: dict[str, Any], required_fields: set[str], label: str, errors: list[str]
) -> None:
    if required_fields - value.keys():
        errors.append(f"{label} is missing required field(s)")
    if value.keys() - required_fields:
        # Do not echo keys or values: rejected metadata may contain local details.
        errors.append(f"{label} contains unsupported field(s)")


def _safe_relative_path(value: Any) -> bool:
    """Accept canonical slash-separated relative paths without traversal syntax."""
    if not _nonempty_string(value) or "\\" in value or "\x00" in value:
        return False
    if value.startswith("/") or WINDOWS_DRIVE_PATTERN.match(value):
        return False
    parts = value.split("/")
    if any(part in {"", ".", ".."} for part in parts):
        return False
    return not PurePosixPath(value).is_absolute()


def _asset_root_path(value: str | os.PathLike[str]) -> Path | None:
    """Return a real, non-symlink directory or ``None`` without leaking its path."""
    try:
        supplied = Path(value)
        info = supplied.lstat()
        if stat.S_ISLNK(info.st_mode) or not stat.S_ISDIR(info.st_mode):
            return None
        return supplied.resolve(strict=True)
    except (OSError, RuntimeError, TypeError, ValueError):
        return None


def _contained_regular_file(relative_path: str, root: Path) -> Path | None:
    """Resolve a file only after ensuring symlinks stay within the supplied root."""
    try:
        candidate = root.joinpath(*PurePosixPath(relative_path).parts)
        resolved = candidate.resolve(strict=True)
        if os.path.commonpath((str(root), str(resolved))) != str(root):
            return None
        if not stat.S_ISREG(resolved.stat().st_mode):
            return None
        return resolved
    except (OSError, RuntimeError, ValueError):
        return None


def _file_sha256(path: Path) -> str | None:
    """Hash an asset incrementally without loading a large model into memory."""
    digest = hashlib.sha256()
    try:
        with path.open("rb") as asset_file:
            for chunk in iter(lambda: asset_file.read(HASH_CHUNK_BYTES), b""):
                digest.update(chunk)
    except OSError:
        return None
    return digest.hexdigest()


def _valid_optional_string(value: Any) -> bool:
    return value is None or _nonempty_string(value)


def _valid_source_url(value: Any) -> bool:
    if value is None:
        return True
    if not _nonempty_string(value):
        return False
    try:
        parsed = urlparse(value)
        return parsed.scheme in {"http", "https"} and bool(parsed.netloc)
    except ValueError:
        return False


def validate_manifest(
    data: Any, asset_root: str | os.PathLike[str] = REPOSITORY_ROOT
) -> tuple[list[str], dict[str, int]]:
    """Validate inventory structure, licensing metadata, and verified file hashes.

    Unknown revision, path, checksum, license and provenance values are
    represented as explicit ``None`` values. A ``file-verified`` status only
    records local integrity and provenance checks; it does not grant rights or
    establish that an asset is included in a release package.
    """
    errors: list[str] = []
    counts = {
        "planned_assets": 0,
        "file_verified_assets": 0,
        "redistribution_approvals": 0,
    }

    if not isinstance(data, dict):
        return ["manifest root must be a JSON object"], counts
    _check_fields(data, ROOT_FIELDS, "manifest", errors)

    schema_version = data.get("schema_version")
    if type(schema_version) is not int or schema_version != SCHEMA_VERSION:
        errors.append(f"schema_version must be {SCHEMA_VERSION}")

    assets = data.get("assets")
    if not isinstance(assets, list):
        errors.append("assets must be a list")
        return errors, counts

    seen_ids: set[str] = set()
    asset_root_path: Path | None = None
    root_checked = False
    for index, asset in enumerate(assets):
        label = f"assets[{index}]"
        if not isinstance(asset, dict):
            errors.append(f"{label} must be an object")
            continue
        _check_fields(asset, ASSET_FIELDS, label, errors)

        asset_id = asset.get("asset_id")
        if not _nonempty_string(asset_id):
            errors.append(f"{label}.asset_id must be a non-empty string")
        elif asset_id in seen_ids:
            errors.append(f"{label}.asset_id duplicates another asset identifier")
        else:
            seen_ids.add(asset_id)

        category = asset.get("category")
        if not isinstance(category, str) or category not in ASSET_CATEGORIES:
            errors.append(f"{label}.category must be one of {sorted(ASSET_CATEGORIES)}")

        status = asset.get("status")
        if not isinstance(status, str) or status not in ASSET_STATUSES:
            errors.append(f"{label}.status must be one of {sorted(ASSET_STATUSES)}")
        elif status == "planned":
            counts["planned_assets"] += 1
        else:
            counts["file_verified_assets"] += 1

        if not _nonempty_string(asset.get("source")):
            errors.append(f"{label}.source must be a non-empty string")
        if not _valid_source_url(asset.get("source_url")):
            errors.append(f"{label}.source_url must be an HTTP(S) URL or null")
        for field in (
            "source_revision",
            "provenance_reference",
            "license_identifier",
            "license_evidence_reference",
            "redistribution_evidence_reference",
            "notice_attribution_evidence_reference",
        ):
            if not _valid_optional_string(asset.get(field)):
                errors.append(f"{label}.{field} must be a non-empty string or null")

        file_path = asset.get("file_path")
        if file_path is not None and not _safe_relative_path(file_path):
            errors.append(f"{label}.file_path must be a safe relative path or null")

        checksum = asset.get("sha256")
        if checksum is not None and (
            not isinstance(checksum, str) or not SHA256_PATTERN.fullmatch(checksum)
        ):
            errors.append(f"{label}.sha256 must be 64 hexadecimal characters or null")
        if checksum is not None and file_path is None:
            errors.append(f"{label}.sha256 requires file_path")

        license_review_status = asset.get("license_review_status")
        if (
            not isinstance(license_review_status, str)
            or license_review_status not in LICENSE_REVIEW_STATUSES
        ):
            errors.append(
                f"{label}.license_review_status must be one of "
                f"{sorted(LICENSE_REVIEW_STATUSES)}"
            )
        elif license_review_status == "reviewed" and not _nonempty_string(
            asset.get("license_evidence_reference")
        ):
            errors.append(f"{label}.license_evidence_reference is required for reviewed metadata")
        elif asset.get("license_identifier") is not None and license_review_status != "reviewed":
            errors.append(f"{label}.license_identifier requires reviewed license metadata")

        redistribution_status = asset.get("redistribution_status")
        if (
            not isinstance(redistribution_status, str)
            or redistribution_status not in REDISTRIBUTION_STATUSES
        ):
            errors.append(
                f"{label}.redistribution_status must be one of "
                f"{sorted(REDISTRIBUTION_STATUSES)}"
            )
        elif redistribution_status in {"approved", "blocked"} and not _nonempty_string(
            asset.get("redistribution_evidence_reference")
        ):
            errors.append(
                f"{label}.redistribution_evidence_reference is required for a redistribution decision"
            )
        elif redistribution_status == "approved":
            if license_review_status != "reviewed":
                errors.append(
                    f"{label}.license_review_status must be reviewed before redistribution approval"
                )
            else:
                counts["redistribution_approvals"] += 1

        notice_status = asset.get("notice_attribution_status")
        if not isinstance(notice_status, str) or notice_status not in NOTICE_STATUSES:
            errors.append(f"{label}.notice_attribution_status must be one of {sorted(NOTICE_STATUSES)}")
        elif notice_status != "undetermined" and not _nonempty_string(
            asset.get("notice_attribution_evidence_reference")
        ):
            errors.append(
                f"{label}.notice_attribution_evidence_reference is required for a notice decision"
            )

        if status == "file-verified":
            for field in ("source_revision", "file_path", "sha256", "provenance_reference"):
                if not _nonempty_string(asset.get(field)):
                    errors.append(f"{label}.{field} is required for a file-verified asset")
            revision = asset.get("source_revision")
            if _nonempty_string(revision) and revision.strip().lower() in {
                "unknown",
                "tbd",
                "todo",
                "latest",
                "main",
                "master",
            }:
                errors.append(f"{label}.source_revision must identify an exact revision")

            if _safe_relative_path(file_path):
                if not root_checked:
                    asset_root_path = _asset_root_path(asset_root)
                    root_checked = True
                    if asset_root_path is None:
                        errors.append("asset root must be an accessible non-symlink directory")
                if asset_root_path is not None:
                    local_file = _contained_regular_file(file_path, asset_root_path)
                    if local_file is None:
                        errors.append(f"{label}.file_path does not identify a safe local regular file")
                    elif isinstance(checksum, str) and SHA256_PATTERN.fullmatch(checksum):
                        actual_checksum = _file_sha256(local_file)
                        if actual_checksum is None:
                            errors.append(f"{label}.file_path could not be read for checksum verification")
                        elif actual_checksum.lower() != checksum.lower():
                            errors.append(f"{label}.sha256 does not match the local file")

    return errors, counts


def format_summary(counts: dict[str, int]) -> str:
    """Render the stable, content-free inventory summary."""
    return "\n".join(
        (
            "Outflow model asset inventory valid",
            f"Manifest version: {SCHEMA_VERSION}",
            f"Planned assets: {counts['planned_assets']}",
            f"File-verified assets: {counts['file_verified_assets']}",
            f"Redistribution approvals: {counts['redistribution_approvals']}",
            "Bundled model status: NOT VERIFIED",
            "T-009 licensing audit: INCOMPLETE",
        )
    )


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--manifest", required=True, help="JSON inventory manifest")
    parser.add_argument(
        "--asset-root",
        default=str(REPOSITORY_ROOT),
        help="root directory for local file-verified assets (default: repository root)",
    )
    args = parser.parse_args(argv)

    try:
        data = load_manifest(args.manifest)
    except InventoryValidationError as error:
        print("Outflow model asset inventory invalid", file=sys.stderr)
        for message in error.errors:
            print(f"Error: {message}", file=sys.stderr)
        return 1

    errors, counts = validate_manifest(data, args.asset_root)
    if errors:
        print("Outflow model asset inventory invalid", file=sys.stderr)
        for message in errors:
            print(f"Error: {message}", file=sys.stderr)
        return 1

    print(format_summary(counts))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
