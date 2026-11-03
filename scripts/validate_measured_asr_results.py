#!/usr/bin/env python3
"""Validate sanitized native-ASR measurements, distinct from synthetic WER data."""

from __future__ import annotations

import argparse
from datetime import datetime
import json
import math
from pathlib import Path
import re
import sys
from typing import Any, Sequence

if __package__:
    from .run_native_asr_benchmark import (
        EVALUATION_VERSION,
        EXPECTED_ARCHITECTURE,
        EXPECTED_HEADER_VERSION,
        MODEL_CATALOG_PATH,
        REQUIRED_MODEL_ASSETS,
        RESULT_SCHEMA_VERSION,
        RESULT_TYPE,
        SOURCE_REVISION,
        _unique_object,
        _reject_constant,
    )
    from .validate_fixture_manifest import ALLOWED_CATEGORIES
else:
    from run_native_asr_benchmark import (
        EVALUATION_VERSION,
        EXPECTED_ARCHITECTURE,
        EXPECTED_HEADER_VERSION,
        MODEL_CATALOG_PATH,
        REQUIRED_MODEL_ASSETS,
        RESULT_SCHEMA_VERSION,
        RESULT_TYPE,
        SOURCE_REVISION,
        _unique_object,
        _reject_constant,
    )
    from validate_fixture_manifest import ALLOWED_CATEGORIES


SHA256_PATTERN = re.compile(r"[0-9a-f]{64}\Z")
FORBIDDEN_CONTENT_KEYS = {
    "hypothesis",
    "transcript",
    "transcription",
    "reference_text",
    "audio_path",
    "reference_path",
    "command_line",
    "environment",
    "hostname",
    "username",
    "serial_number",
}
ROOT_FIELDS = {
    "schema_version",
    "result_type",
    "evaluation_version",
    "created_at_utc",
    "normalization_policy_version",
    "reference_transcription_policy_version",
    "fixture_manifest_sha256",
    "source",
    "model",
    "native_binaries",
    "machine",
    "corpus",
    "aggregate",
    "timing",
    "fixtures",
    "release_gates",
    "limitations",
}


class MeasuredResultsError(ValueError):
    """A measured result is malformed or contains content that must not persist."""


def _is_hash(value: Any) -> bool:
    return isinstance(value, str) and bool(SHA256_PATTERN.fullmatch(value))


def _finite_number(value: Any, *, minimum: float = 0.0) -> bool:
    return (
        not isinstance(value, bool)
        and isinstance(value, (int, float))
        and math.isfinite(float(value))
        and value >= minimum
    )


def _reject_content_keys(value: Any) -> None:
    if isinstance(value, dict):
        if any(isinstance(key, str) and key.lower() in FORBIDDEN_CONTENT_KEYS for key in value):
            raise MeasuredResultsError("result contains a forbidden content or identity field")
        for child in value.values():
            _reject_content_keys(child)
    elif isinstance(value, list):
        for child in value:
            _reject_content_keys(child)


def validate_measured_results(value: Any) -> dict[str, Any]:
    """Validate measured result identity, privacy boundary, counts, and arithmetic."""

    if not isinstance(value, dict) or set(value) != ROOT_FIELDS:
        raise MeasuredResultsError("root fields do not match the measured schema")
    _reject_content_keys(value)
    if value["schema_version"] != RESULT_SCHEMA_VERSION or value["result_type"] != RESULT_TYPE:
        raise MeasuredResultsError("result type is not measured native ASR")
    if value["evaluation_version"] != EVALUATION_VERSION:
        raise MeasuredResultsError("unsupported evaluation version")
    if not _is_hash(value["fixture_manifest_sha256"]):
        raise MeasuredResultsError("fixture manifest hash is invalid")
    if value["normalization_policy_version"] != "lexical-wer-unicode-boundary-v1":
        raise MeasuredResultsError("normalization policy version is invalid")
    if value["reference_transcription_policy_version"] != "human-verified-verbatim-v1":
        raise MeasuredResultsError("reference policy version is invalid")
    try:
        datetime.fromisoformat(value["created_at_utc"].replace("Z", "+00:00"))
    except (AttributeError, TypeError, ValueError) as error:
        raise MeasuredResultsError("measurement timestamp is invalid") from error

    source = value["source"]
    if (
        not isinstance(source, dict)
        or set(source) != {"source_revision", "c_api_header_version", "model_architecture"}
        or source["source_revision"] != SOURCE_REVISION
        or source["c_api_header_version"] != EXPECTED_HEADER_VERSION
        or source["model_architecture"] != EXPECTED_ARCHITECTURE
    ):
        raise MeasuredResultsError("pinned source identity is invalid")

    model = value["model"]
    model_fields = {
        "catalog_path",
        "immutable_artifact_revision",
        "architecture",
        "c_api_header_version",
        "asset_sha256",
        "asset_bytes",
        "total_asset_bytes",
        "cold_process_load_ms",
        "warm_reload_samples_ms",
        "warm_reload_median_ms",
        "inference_process_load_ms",
        "redistribution_status",
        "license_review_status",
    }
    if not isinstance(model, dict) or set(model) != model_fields:
        raise MeasuredResultsError("model record shape is invalid")
    if model["catalog_path"] != MODEL_CATALOG_PATH or model["immutable_artifact_revision"] is not None:
        raise MeasuredResultsError("model catalog and immutable revision fields are conflated")
    if model["architecture"] != EXPECTED_ARCHITECTURE or model["c_api_header_version"] != EXPECTED_HEADER_VERSION:
        raise MeasuredResultsError("model ABI identity is invalid")
    hashes, sizes = model["asset_sha256"], model["asset_bytes"]
    if not isinstance(hashes, dict) or set(hashes) != REQUIRED_MODEL_ASSETS:
        raise MeasuredResultsError("model artifact hashes are incomplete")
    if not all(_is_hash(digest) for digest in hashes.values()):
        raise MeasuredResultsError("model artifact hash is invalid")
    if not isinstance(sizes, dict) or set(sizes) != REQUIRED_MODEL_ASSETS:
        raise MeasuredResultsError("model artifact byte counts are incomplete")
    if any(type(size) is not int or size <= 0 for size in sizes.values()):
        raise MeasuredResultsError("model artifact byte count is invalid")
    if model["total_asset_bytes"] != sum(sizes.values()):
        raise MeasuredResultsError("model total does not equal artifact byte counts")
    for field in ("cold_process_load_ms", "warm_reload_median_ms", "inference_process_load_ms"):
        if not _finite_number(model[field]):
            raise MeasuredResultsError("model load timing is invalid")
    warm_samples = model["warm_reload_samples_ms"]
    if not isinstance(warm_samples, list) or not warm_samples or any(not _finite_number(x) for x in warm_samples):
        raise MeasuredResultsError("warm model load samples are invalid")

    binaries = value["native_binaries"]
    if not isinstance(binaries, dict) or set(binaries) != {"native_runner", "moonshine_library", "onnxruntime_library"}:
        raise MeasuredResultsError("native binary record is incomplete")
    for binary in binaries.values():
        if (
            not isinstance(binary, dict)
            or set(binary) != {"bytes", "sha256"}
            or type(binary["bytes"]) is not int
            or binary["bytes"] <= 0
            or not _is_hash(binary["sha256"])
        ):
            raise MeasuredResultsError("native binary identity is invalid")

    machine = value["machine"]
    if not isinstance(machine, dict) or set(machine) != {
        "machine_label", "os", "os_release", "architecture", "physical_memory_bytes", "logical_cpu_count", "cpu_label"
    }:
        raise MeasuredResultsError("machine record shape is invalid")
    for field in ("machine_label", "os", "os_release", "architecture"):
        if machine[field] is not None and (not isinstance(machine[field], str) or not machine[field]):
            raise MeasuredResultsError("machine identity field is invalid")
    for field in ("physical_memory_bytes", "logical_cpu_count"):
        metric = machine[field]
        if metric is not None and (type(metric) is not int or metric <= 0):
            raise MeasuredResultsError("machine hardware metric is invalid")
    if machine["cpu_label"] is not None and not isinstance(machine["cpu_label"], str):
        raise MeasuredResultsError("CPU label is invalid")

    fixtures = value["fixtures"]
    if not isinstance(fixtures, list) or not fixtures:
        raise MeasuredResultsError("measured result contains no fixture rows")
    seen_ids: set[str] = set()
    speech_rows: list[dict[str, Any]] = []
    silence_count = 0
    for row in fixtures:
        if not isinstance(row, dict) or set(row) != {
            "fixture_id", "category", "audio_sha256", "reference_sha256", "audio_metadata", "wer",
            "error_counts", "no_speech_detected", "partial_calls", "partial_nonempty", "final_line_count",
            "recognition_ms", "stop_to_final_ms", "real_time_factor", "native_process_cpu_ms"
        }:
            raise MeasuredResultsError("fixture result shape is invalid")
        fixture_id = row["fixture_id"]
        if not isinstance(fixture_id, str) or not fixture_id or fixture_id in seen_ids:
            raise MeasuredResultsError("fixture IDs are invalid or duplicated")
        seen_ids.add(fixture_id)
        if row["category"] not in ALLOWED_CATEGORIES or not _is_hash(row["audio_sha256"]) or not _is_hash(row["reference_sha256"]):
            raise MeasuredResultsError("fixture category or checksum is invalid")
        metadata = row["audio_metadata"]
        if not isinstance(metadata, dict) or set(metadata) != {
            "sample_format", "sample_rate_hz", "channels", "frame_count", "duration_seconds"
        }:
            raise MeasuredResultsError("fixture audio metadata is invalid")
        if (
            metadata["sample_format"] != "PCM_S16LE"
            or any(type(metadata[field]) is not int or metadata[field] <= 0 for field in ("sample_rate_hz", "channels", "frame_count"))
            or not _finite_number(metadata["duration_seconds"])
            or abs(metadata["duration_seconds"] - metadata["frame_count"] / metadata["sample_rate_hz"]) > 0.001
        ):
            raise MeasuredResultsError("fixture audio metadata is inconsistent")
        for field in ("partial_calls", "final_line_count"):
            if type(row[field]) is not int or row[field] < 0:
                raise MeasuredResultsError("fixture inference count is invalid")
        if type(row["partial_nonempty"]) is not bool:
            raise MeasuredResultsError("fixture partial status is invalid")
        for field in ("recognition_ms", "stop_to_final_ms", "real_time_factor"):
            if not _finite_number(row[field]):
                raise MeasuredResultsError("fixture timing is invalid")
        if row["native_process_cpu_ms"] is not None and not _finite_number(row["native_process_cpu_ms"]):
            raise MeasuredResultsError("fixture CPU time is invalid")
        if row["category"] == "silence":
            silence_count += 1
            if row["wer"] is not None or row["error_counts"] is not None or type(row["no_speech_detected"]) is not bool:
                raise MeasuredResultsError("silence control must be separate from speech WER")
        else:
            counts = row["error_counts"]
            if not isinstance(counts, dict) or set(counts) != {"reference_words", "substitutions", "deletions", "insertions"}:
                raise MeasuredResultsError("speech fixture error counts are missing")
            if any(type(count) is not int or count < 0 for count in counts.values()) or counts["reference_words"] < 1:
                raise MeasuredResultsError("speech fixture error counts are invalid")
            expected_wer = (counts["substitutions"] + counts["deletions"] + counts["insertions"]) / counts["reference_words"]
            if not _finite_number(row["wer"]) or abs(row["wer"] - expected_wer) > 1e-9:
                raise MeasuredResultsError("speech fixture WER does not match its error counts")
            if row["no_speech_detected"] is not None:
                raise MeasuredResultsError("speech rows cannot declare no-speech detection")
            speech_rows.append(row)

    aggregate = value["aggregate"]
    if not isinstance(aggregate, dict) or set(aggregate) != {
        "speech_fixtures_scored", "reference_words", "substitutions", "deletions", "insertions", "wer"
    }:
        raise MeasuredResultsError("aggregate WER record shape is invalid")
    expected_counts = {
        "speech_fixtures_scored": len(speech_rows),
        "reference_words": sum(row["error_counts"]["reference_words"] for row in speech_rows),
        "substitutions": sum(row["error_counts"]["substitutions"] for row in speech_rows),
        "deletions": sum(row["error_counts"]["deletions"] for row in speech_rows),
        "insertions": sum(row["error_counts"]["insertions"] for row in speech_rows),
    }
    if any(aggregate[key] != expected for key, expected in expected_counts.items()):
        raise MeasuredResultsError("aggregate counts do not match fixture records")
    expected_wer = None if not speech_rows else (
        expected_counts["substitutions"] + expected_counts["deletions"] + expected_counts["insertions"]
    ) / expected_counts["reference_words"]
    if aggregate["wer"] != expected_wer:
        raise MeasuredResultsError("aggregate WER does not match fixture records")

    corpus = value["corpus"]
    corpus_fields = {
        "verified_ready_fixture_count", "short_phrase_count", "long_phrase_count", "quiet_environment_count",
        "noisy_environment_count", "accented_english_count", "proper_nouns_technical_vocabulary_count",
        "numbers_count", "silence_control_count"
    }
    if not isinstance(corpus, dict) or set(corpus) != corpus_fields:
        raise MeasuredResultsError("corpus summary shape is invalid")
    if corpus["verified_ready_fixture_count"] != len(fixtures) or corpus["silence_control_count"] != silence_count:
        raise MeasuredResultsError("corpus summary does not match fixture records")
    if any(type(count) is not int or count < 0 for count in corpus.values()):
        raise MeasuredResultsError("corpus summary count is invalid")
    if sum(corpus[field] for field in (
        "short_phrase_count", "long_phrase_count", "quiet_environment_count", "noisy_environment_count",
        "accented_english_count", "proper_nouns_technical_vocabulary_count", "numbers_count", "silence_control_count"
    )) < len(fixtures):
        # Category labels overlap in the manifest, so this check is intentionally one-sided.
        raise MeasuredResultsError("corpus summary omits fixture coverage")

    if not isinstance(value["timing"], dict) or set(value["timing"]) != {
        "recognition_p50_ms", "recognition_p95_ms", "stop_to_final_p50_ms", "stop_to_final_p95_ms",
        "real_time_factor_p50", "real_time_factor_p95"
    }:
        raise MeasuredResultsError("timing summary shape is invalid")
    if not isinstance(value["release_gates"], dict) or not isinstance(value["limitations"], list):
        raise MeasuredResultsError("release gate or limitations record is invalid")
    return {
        "fixtures": len(fixtures),
        "speech_fixtures": len(speech_rows),
        "silence_controls": silence_count,
        "wer": aggregate["wer"],
    }


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Validate sanitized measured native-ASR results.")
    parser.add_argument("--file", type=Path, required=True, help="measured result JSON; synthetic schema is rejected")
    arguments = parser.parse_args(argv)
    try:
        value = json.loads(
            arguments.file.read_text(encoding="utf-8"),
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
        summary = validate_measured_results(value)
    except (OSError, UnicodeError, json.JSONDecodeError, ValueError, RecursionError) as error:
        print(f"Measured ASR results invalid: {error}", file=sys.stderr)
        return 1
    print("MEASURED_NATIVE_ASR_RESULT: VALIDATED")
    print(f"Fixture rows: {summary['fixtures']}")
    print(f"Speech fixtures: {summary['speech_fixtures']}")
    print(f"Silence controls: {summary['silence_controls']}")
    print(f"Corpus WER: {summary['wer']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
