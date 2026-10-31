#!/usr/bin/env python3
"""Validate the development-only English benchmark audio fixture manifest."""

from __future__ import annotations

import hashlib
import json
import re
import sys
from pathlib import Path, PurePosixPath, PureWindowsPath
from typing import Any


REPOSITORY_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_MANIFEST_PATH = REPOSITORY_ROOT / "docs" / "benchmark-fixtures.json"
SUPPORTED_SCHEMA_VERSION = 1
ALLOWED_COLLECTION_STATUSES = {"planned", "in_progress", "complete"}
ALLOWED_FIXTURE_STATUSES = {"planned", "ready"}
ALLOWED_CATEGORIES = {
    "short_speech",
    "long_speech",
    "quiet_environment",
    "noisy_environment",
    "accented_english",
    "proper_nouns_technical_vocabulary",
    "numbers",
    "silence",
}
ALLOWED_VERIFICATION_STATUSES = {"not_verified", "verified"}
SHA256_PATTERN = re.compile(r"^[0-9a-fA-F]{64}$")
REQUIRED_FIXTURE_FIELDS = {
    "id",
    "status",
    "category",
    "recording_condition",
    "audio_file",
    "audio_sha256",
    "reference_file",
    "provenance",
    "license_or_permission_status",
    "license_or_permission_evidence",
    "human_transcription_status",
}


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _safe_repository_file(
    value: Any,
    *,
    root: Path,
    label: str,
    errors: list[str],
    required: bool,
    require_nonempty: bool = False,
) -> Path | None:
    """Validate a repository-relative file reference without revealing paths."""
    if value is None:
        if required:
            errors.append(f"{label} is required")
        return None
    if not _nonempty_string(value):
        errors.append(f"{label} must be a non-empty repository-relative path or null")
        return None
    if "\\" in value or any(ord(character) < 32 for character in value):
        errors.append(f"{label} contains an unsafe path character")
        return None

    posix_path = PurePosixPath(value)
    windows_path = PureWindowsPath(value)
    if (
        posix_path.is_absolute()
        or windows_path.is_absolute()
        or windows_path.drive
        or ".." in posix_path.parts
    ):
        errors.append(f"{label} must not be absolute or contain directory traversal")
        return None

    candidate = root / Path(*posix_path.parts)
    try:
        resolved_root = root.resolve()
        resolved_candidate = candidate.resolve()
        resolved_candidate.relative_to(resolved_root)
    except (OSError, RuntimeError, ValueError):
        errors.append(f"{label} resolves outside the repository or cannot be resolved safely")
        return None

    if required:
        try:
            if not resolved_candidate.is_file():
                errors.append(f"{label} does not identify an accessible regular file")
                return None
            if require_nonempty and resolved_candidate.stat().st_size == 0:
                errors.append(f"{label} must identify a non-empty file")
                return None
        except OSError:
            errors.append(f"{label} does not identify an accessible regular file")
            return None
    return resolved_candidate


def _file_sha256(path: Path) -> str | None:
    """Hash a file incrementally so large audio does not load into memory."""
    digest = hashlib.sha256()
    try:
        with path.open("rb") as audio_file:
            for chunk in iter(lambda: audio_file.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError:
        return None
    return digest.hexdigest()


def validate_manifest(data: Any, base_dir: Path | str = REPOSITORY_ROOT) -> tuple[list[str], dict[str, int]]:
    """Return validation errors and readiness counts for parsed manifest data."""
    errors: list[str] = []
    counts = {"planned_entries": 0, "ready_audio_fixtures": 0}

    if not isinstance(data, dict):
        return ["manifest root must be a JSON object"], counts

    schema_version = data.get("schema_version")
    if (
        not isinstance(schema_version, int)
        or isinstance(schema_version, bool)
        or schema_version != SUPPORTED_SCHEMA_VERSION
    ):
        errors.append(f"schema_version must be {SUPPORTED_SCHEMA_VERSION}")

    if data.get("language") != "English":
        errors.append("language must be English")
    collection_status = data.get("collection_status")
    if not isinstance(collection_status, str) or collection_status not in ALLOWED_COLLECTION_STATUSES:
        errors.append(f"collection_status must be one of {sorted(ALLOWED_COLLECTION_STATUSES)}")
    if not _nonempty_string(data.get("reference_transcription_policy")):
        errors.append("reference_transcription_policy must be a non-empty string")

    integrity = data.get("audio_integrity_requirements")
    if not isinstance(integrity, dict):
        errors.append("audio_integrity_requirements must be an object")
    else:
        if integrity.get("checksum_algorithm") != "SHA-256":
            errors.append("audio_integrity_requirements.checksum_algorithm must be SHA-256")
        for field in ("checksum_scope", "missing_audio_policy"):
            if not _nonempty_string(integrity.get(field)):
                errors.append(f"audio_integrity_requirements.{field} must be a non-empty string")

    fixtures = data.get("fixtures")
    if not isinstance(fixtures, list):
        errors.append("fixtures must be a list")
        return errors, counts

    seen_ids: set[str] = set()
    root = Path(base_dir)
    for index, fixture in enumerate(fixtures):
        label = f"fixtures[{index}]"
        if not isinstance(fixture, dict):
            errors.append(f"{label} must be an object")
            continue

        entry_error_start = len(errors)
        for field in sorted(REQUIRED_FIXTURE_FIELDS - fixture.keys()):
            errors.append(f"{label}.{field} is required")

        fixture_id = fixture.get("id")
        if not _nonempty_string(fixture_id):
            errors.append(f"{label}.id must be a non-empty string")
        elif fixture_id in seen_ids:
            errors.append(f"{label}.id duplicates another fixture identifier")
        else:
            seen_ids.add(fixture_id)

        status = fixture.get("status")
        if not isinstance(status, str) or status not in ALLOWED_FIXTURE_STATUSES:
            errors.append(f"{label}.status must be one of {sorted(ALLOWED_FIXTURE_STATUSES)}")
        elif status == "planned":
            counts["planned_entries"] += 1

        category = fixture.get("category")
        if not isinstance(category, str) or category not in ALLOWED_CATEGORIES:
            errors.append(f"{label}.category must be one of {sorted(ALLOWED_CATEGORIES)}")
        if not _nonempty_string(fixture.get("recording_condition")):
            errors.append(f"{label}.recording_condition must be a non-empty string")

        audio_file_value = fixture.get("audio_file")
        audio_file = _safe_repository_file(
            audio_file_value,
            root=root,
            label=f"{label}.audio_file",
            errors=errors,
            required=status == "ready",
        )

        audio_sha256 = fixture.get("audio_sha256")
        if audio_sha256 is not None and (
            not isinstance(audio_sha256, str) or not SHA256_PATTERN.fullmatch(audio_sha256)
        ):
            errors.append(f"{label}.audio_sha256 must be 64 hexadecimal characters or null")
        if audio_sha256 is not None and audio_file_value is None:
            errors.append(f"{label}.audio_sha256 requires audio_file")

        reference_file_value = fixture.get("reference_file")
        reference_file = _safe_repository_file(
            reference_file_value,
            root=root,
            label=f"{label}.reference_file",
            errors=errors,
            required=status == "ready",
            require_nonempty=status == "ready",
        )

        provenance = fixture.get("provenance")
        if not isinstance(provenance, dict):
            errors.append(f"{label}.provenance must be an object")
            provenance = {}
        for field in ("source", "citation"):
            if provenance.get(field) is not None and not _nonempty_string(provenance.get(field)):
                errors.append(f"{label}.provenance.{field} must be a non-empty string or null")

        permission_status = fixture.get("license_or_permission_status")
        if not isinstance(permission_status, str) or permission_status not in ALLOWED_VERIFICATION_STATUSES:
            errors.append(
                f"{label}.license_or_permission_status must be one of "
                f"{sorted(ALLOWED_VERIFICATION_STATUSES)}"
            )
        permission_evidence = fixture.get("license_or_permission_evidence")
        if permission_evidence is not None and not _nonempty_string(permission_evidence):
            errors.append(f"{label}.license_or_permission_evidence must be a non-empty string or null")

        reference_status = fixture.get("human_transcription_status")
        if not isinstance(reference_status, str) or reference_status not in ALLOWED_VERIFICATION_STATUSES:
            errors.append(
                f"{label}.human_transcription_status must be one of "
                f"{sorted(ALLOWED_VERIFICATION_STATUSES)}"
            )

        if status == "ready":
            if not _nonempty_string(provenance.get("source")):
                errors.append(f"{label}.provenance.source is required for a ready fixture")
            if not _nonempty_string(provenance.get("citation")):
                errors.append(f"{label}.provenance.citation is required for a ready fixture")
            if permission_status != "verified":
                errors.append(f"{label}.license_or_permission_status must be verified for a ready fixture")
            if not _nonempty_string(permission_evidence):
                errors.append(f"{label}.license_or_permission_evidence is required for a ready fixture")
            if reference_status != "verified":
                errors.append(f"{label}.human_transcription_status must be verified for a ready fixture")
            if audio_sha256 is None:
                errors.append(f"{label}.audio_sha256 is required for a ready fixture")
            elif isinstance(audio_sha256, str) and SHA256_PATTERN.fullmatch(audio_sha256) and audio_file is not None:
                actual_sha256 = _file_sha256(audio_file)
                if actual_sha256 is None:
                    errors.append(f"{label}.audio_file could not be read for checksum verification")
                elif actual_sha256.lower() != audio_sha256.lower():
                    errors.append(f"{label}.audio_sha256 does not match the referenced audio file")

        if status == "ready" and len(errors) == entry_error_start:
            counts["ready_audio_fixtures"] += 1

    return errors, counts


def main() -> int:
    """Validate the repository manifest and print a concise readiness summary."""
    if len(sys.argv) > 2:
        print("Usage: python3 scripts/validate_fixture_manifest.py [manifest.json]", file=sys.stderr)
        return 2
    manifest_path = Path(sys.argv[1]) if len(sys.argv) == 2 else DEFAULT_MANIFEST_PATH
    try:
        data = json.loads(manifest_path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        print(f"Outflow benchmark fixture manifest invalid: cannot read JSON ({error.__class__.__name__}).")
        return 1

    # Fixture paths are repository-relative even though the default manifest lives in docs/.
    errors, counts = validate_manifest(data, REPOSITORY_ROOT)
    if errors:
        print(f"Outflow benchmark fixture manifest invalid: {len(errors)} issue(s).")
        for error in errors:
            print(f"- {error}")
        return 1

    print("Outflow benchmark fixture manifest valid")
    print(f"Manifest version: {data['schema_version']}")
    print(f"Planned entries: {counts['planned_entries']}")
    print(f"Ready audio fixtures: {counts['ready_audio_fixtures']}")
    readiness = "READY" if counts["ready_audio_fixtures"] > 0 else "NOT READY"
    print(f"ASR benchmark readiness: {readiness}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
