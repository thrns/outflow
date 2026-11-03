#!/usr/bin/env python3
"""Validate the development-only English benchmark audio fixture manifest."""

from __future__ import annotations

import hashlib
import json
import math
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
ALLOWED_DURATION_CLASSES = {"short", "long", "silence_control"}
ALLOWED_COVERAGE_ATTRIBUTES = {
    "quiet_environment",
    "noisy_environment",
    "accented_english",
    "proper_nouns",
    "technical_vocabulary",
    "numbers",
    "abbreviations",
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
    "reference_sha256",
    "audio_metadata",
    "human_transcription_review_method",
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


def assess_corpus_coverage(fixtures: list[dict[str, Any]]) -> dict[str, Any]:
    """Summarize ready-fixture coverage against explicit inclusion criteria.

    This presence/count check does not claim statistical representativeness.
    Coverage needs at least one example per condition and two source-reported
    accent varieties, in addition to the established short/long counts.
    """
    ready = [fixture for fixture in fixtures if fixture.get("status") == "ready"]
    attributes = [
        {item for item in fixture.get("coverage_attributes", []) if isinstance(item, str)}
        for fixture in ready
        if isinstance(fixture.get("coverage_attributes", []), list)
    ]
    count = lambda name: sum(name in row for row in attributes)
    accent_varieties = {
        fixture.get("speaker_variety")
        for fixture in ready
        if isinstance(fixture.get("coverage_attributes"), list)
        and "accented_english" in fixture["coverage_attributes"]
        and isinstance(fixture.get("speaker_variety"), str)
    }
    durations = [fixture.get("duration_class") for fixture in ready]
    coverage: dict[str, Any] = {
        "ready_fixture_count": len(ready),
        "short_utterance_count": durations.count("short"),
        "long_utterance_count": durations.count("long"),
        "quiet_speech_count": count("quiet_environment"),
        "noisy_speech_count": count("noisy_environment"),
        "accented_speech_count": count("accented_english"),
        "accent_variety_count": len(accent_varieties),
        "proper_nouns_count": count("proper_nouns"),
        "technical_vocabulary_count": count("technical_vocabulary"),
        "numbers_count": count("numbers"),
        "abbreviations_count": count("abbreviations"),
        "silence_control_count": durations.count("silence_control"),
        "provenance_verified_fixture_count": sum(
            fixture.get("provenance_verified") is True
            or (
                isinstance(fixture.get("provenance"), dict)
                and all(_nonempty_string(fixture["provenance"].get(field)) for field in ("source", "source_release", "citation"))
                and fixture.get("license_or_permission_status") == "verified"
                and _nonempty_string(fixture.get("license_or_permission_evidence"))
            )
            for fixture in ready
        ),
        "human_verified_reference_count": sum(
            fixture.get("human_reference_verified") is True
            or (
                fixture.get("human_transcription_status") == "verified"
                and _nonempty_string(fixture.get("human_transcription_review_method"))
                and _nonempty_string(fixture.get("reference_sha256"))
            )
            for fixture in ready
        ),
    }
    requirements = {
        "short_utterance_count": coverage["short_utterance_count"] >= 100,
        "long_utterance_count": coverage["long_utterance_count"] >= 50,
        "quiet_speech_coverage": coverage["quiet_speech_count"] >= 1,
        "noisy_speech_coverage": coverage["noisy_speech_count"] >= 1,
        "accent_and_speaker_variety": coverage["accent_variety_count"] >= 2,
        "proper_noun_coverage": coverage["proper_nouns_count"] >= 1,
        "technical_vocabulary_coverage": coverage["technical_vocabulary_count"] >= 1,
        "number_coverage": coverage["numbers_count"] >= 1,
        "abbreviation_coverage": coverage["abbreviations_count"] >= 1,
        "silence_control_coverage": coverage["silence_control_count"] >= 1,
        "verified_provenance_for_all_ready_fixtures": bool(ready) and coverage["provenance_verified_fixture_count"] == len(ready),
        "human_verified_references_for_all_ready_fixtures": bool(ready) and coverage["human_verified_reference_count"] == len(ready),
    }
    coverage["requirements"] = requirements
    coverage["complete"] = all(requirements.values())
    return coverage


def validate_manifest(data: Any, base_dir: Path | str = REPOSITORY_ROOT) -> tuple[list[str], dict[str, Any]]:
    """Return validation errors and readiness counts for parsed manifest data."""
    errors: list[str] = []
    counts: dict[str, Any] = {"planned_entries": 0, "ready_audio_fixtures": 0}

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
    for field in ("reference_transcription_policy_version", "normalization_policy_version"):
        if not _nonempty_string(data.get(field)):
            errors.append(f"{field} must be a non-empty string")

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

        duration_class = fixture.get("duration_class")
        coverage_attributes = fixture.get("coverage_attributes")
        speaker_variety = fixture.get("speaker_variety")
        if duration_class is not None and (
            not isinstance(duration_class, str) or duration_class not in ALLOWED_DURATION_CLASSES
        ):
            errors.append(f"{label}.duration_class is unsupported")
        if coverage_attributes is not None:
            if (
                not isinstance(coverage_attributes, list)
                or any(not isinstance(item, str) or item not in ALLOWED_COVERAGE_ATTRIBUTES for item in coverage_attributes)
                or len(coverage_attributes) != len(set(coverage_attributes))
            ):
                errors.append(f"{label}.coverage_attributes contains unsupported or duplicate values")
                coverage_attributes = None
        if speaker_variety is not None and (
            not _nonempty_string(speaker_variety) or len(speaker_variety) > 100
        ):
            errors.append(f"{label}.speaker_variety must be a non-empty non-identifying label of at most 100 characters or null")
        if status == "ready":
            if not isinstance(duration_class, str) or duration_class not in ALLOWED_DURATION_CLASSES:
                errors.append(f"{label}.duration_class is required for a ready fixture")
            if coverage_attributes is None:
                errors.append(f"{label}.coverage_attributes is required for a ready fixture")
            attrs = set(coverage_attributes or [])
            if duration_class == "silence_control" and attrs:
                errors.append(f"{label} silence controls cannot carry speech coverage attributes")
            if duration_class == "silence_control" and category != "silence":
                errors.append(f"{label} silence_control duration requires the silence category")
            if isinstance(duration_class, str) and duration_class in {"short", "long"} and category == "silence":
                errors.append(f"{label} speech duration cannot use the silence category")
            if isinstance(category, str):
                if category == "short_speech" and duration_class != "short":
                    errors.append(f"{label} short_speech category conflicts with duration_class")
                if category == "long_speech" and duration_class != "long":
                    errors.append(f"{label} long_speech category conflicts with duration_class")
                if category == "silence" and duration_class != "silence_control":
                    errors.append(f"{label} silence category conflicts with duration_class")
                legacy_attribute = {
                    "quiet_environment": "quiet_environment",
                    "noisy_environment": "noisy_environment",
                    "accented_english": "accented_english",
                    "numbers": "numbers",
                }.get(category)
                if legacy_attribute and legacy_attribute not in attrs:
                    errors.append(f"{label}.category must be included in coverage_attributes")
                if category == "proper_nouns_technical_vocabulary" and not attrs.intersection({"proper_nouns", "technical_vocabulary"}):
                    errors.append(f"{label}.category requires proper_nouns or technical_vocabulary coverage")
            if {"quiet_environment", "noisy_environment"}.issubset(attrs):
                errors.append(f"{label} cannot be classified as both quiet and noisy")
            if "accented_english" in attrs and not _nonempty_string(speaker_variety):
                errors.append(f"{label}.speaker_variety is required for accented English coverage")
            if "accented_english" not in attrs and speaker_variety is not None:
                errors.append(f"{label}.speaker_variety requires accented English coverage")

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

        reference_sha256 = fixture.get("reference_sha256")
        if reference_sha256 is not None and (
            not isinstance(reference_sha256, str) or not SHA256_PATTERN.fullmatch(reference_sha256)
        ):
            errors.append(f"{label}.reference_sha256 must be 64 hexadecimal characters or null")
        if reference_sha256 is not None and reference_file_value is None:
            errors.append(f"{label}.reference_sha256 requires reference_file")

        audio_metadata = fixture.get("audio_metadata")
        if audio_metadata is not None:
            if not isinstance(audio_metadata, dict):
                errors.append(f"{label}.audio_metadata must be an object or null")
                audio_metadata = None
            else:
                expected_audio_fields = {
                    "sample_format",
                    "sample_rate_hz",
                    "channels",
                    "frame_count",
                    "duration_seconds",
                }
                if audio_metadata.keys() != expected_audio_fields:
                    errors.append(f"{label}.audio_metadata has missing or unsupported fields")
                if audio_metadata.get("sample_format") != "PCM_S16LE":
                    errors.append(f"{label}.audio_metadata.sample_format must be PCM_S16LE")
                for field in ("sample_rate_hz", "channels", "frame_count"):
                    value = audio_metadata.get(field)
                    if type(value) is not int or value <= 0:
                        errors.append(f"{label}.audio_metadata.{field} must be a positive integer")
                duration = audio_metadata.get("duration_seconds")
                if (
                    isinstance(duration, bool)
                    or not isinstance(duration, (int, float))
                    or not math.isfinite(float(duration))
                    or duration <= 0
                ):
                    errors.append(f"{label}.audio_metadata.duration_seconds must be positive and finite")
                elif (
                    type(audio_metadata.get("sample_rate_hz")) is int
                    and audio_metadata.get("sample_rate_hz") > 0
                    and type(audio_metadata.get("frame_count")) is int
                    and audio_metadata.get("frame_count") > 0
                    and abs(duration - audio_metadata["frame_count"] / audio_metadata["sample_rate_hz"]) > 0.001
                ):
                    errors.append(f"{label}.audio_metadata duration does not match its frame count")
                elif status == "ready" and duration_class == "short" and duration > 5.0:
                    errors.append(f"{label}.duration_class short requires an audio duration of at most 5 seconds")
                elif status == "ready" and duration_class == "long" and duration <= 5.0:
                    errors.append(f"{label}.duration_class long requires an audio duration over 5 seconds")

        review_method = fixture.get("human_transcription_review_method")
        if review_method is not None and not _nonempty_string(review_method):
            errors.append(f"{label}.human_transcription_review_method must be a non-empty string or null")

        provenance = fixture.get("provenance")
        if not isinstance(provenance, dict):
            errors.append(f"{label}.provenance must be an object")
            provenance = {}
        for field in ("source", "source_release", "citation"):
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
            if not _nonempty_string(provenance.get("source_release")):
                errors.append(f"{label}.provenance.source_release is required for a ready fixture")
            if not _nonempty_string(provenance.get("citation")):
                errors.append(f"{label}.provenance.citation is required for a ready fixture")
            if permission_status != "verified":
                errors.append(f"{label}.license_or_permission_status must be verified for a ready fixture")
            if not _nonempty_string(permission_evidence):
                errors.append(f"{label}.license_or_permission_evidence is required for a ready fixture")
            if reference_status != "verified":
                errors.append(f"{label}.human_transcription_status must be verified for a ready fixture")
            if not _nonempty_string(review_method):
                errors.append(f"{label}.human_transcription_review_method is required for a ready fixture")
            if audio_sha256 is None:
                errors.append(f"{label}.audio_sha256 is required for a ready fixture")
            elif isinstance(audio_sha256, str) and SHA256_PATTERN.fullmatch(audio_sha256) and audio_file is not None:
                actual_sha256 = _file_sha256(audio_file)
                if actual_sha256 is None:
                    errors.append(f"{label}.audio_file could not be read for checksum verification")
                elif actual_sha256.lower() != audio_sha256.lower():
                    errors.append(f"{label}.audio_sha256 does not match the referenced audio file")
            if audio_metadata is None:
                errors.append(f"{label}.audio_metadata is required for a ready fixture")
            if reference_sha256 is None:
                errors.append(f"{label}.reference_sha256 is required for a ready fixture")
            elif (
                isinstance(reference_sha256, str)
                and SHA256_PATTERN.fullmatch(reference_sha256)
                and reference_file is not None
            ):
                actual_reference_sha256 = _file_sha256(reference_file)
                if actual_reference_sha256 is None:
                    errors.append(f"{label}.reference_file could not be read for checksum verification")
                elif actual_reference_sha256.lower() != reference_sha256.lower():
                    errors.append(f"{label}.reference_sha256 does not match the referenced reference file")

        if status == "ready" and len(errors) == entry_error_start:
            counts["ready_audio_fixtures"] += 1

    counts["coverage"] = assess_corpus_coverage(fixtures)
    if collection_status == "complete" and not counts["coverage"]["complete"]:
        errors.append("collection_status cannot be complete while required fixture coverage is missing")

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
    coverage = counts["coverage"]
    print(
        "Coverage: "
        f"short={coverage['short_utterance_count']}/100, "
        f"long={coverage['long_utterance_count']}/50, "
        f"quiet={coverage['quiet_speech_count']}, noisy={coverage['noisy_speech_count']}, "
        f"accent varieties={coverage['accent_variety_count']}, "
        f"proper nouns={coverage['proper_nouns_count']}, "
        f"technical vocabulary={coverage['technical_vocabulary_count']}, "
        f"numbers={coverage['numbers_count']}, abbreviations={coverage['abbreviations_count']}, "
        f"silence={coverage['silence_control_count']}"
    )
    readiness = "READY" if counts["ready_audio_fixtures"] > 0 else "NOT READY"
    print(f"ASR benchmark readiness: {readiness}")
    print(f"Required diverse corpus coverage: {'PASS' if coverage['complete'] else 'NOT ASSESSED'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
