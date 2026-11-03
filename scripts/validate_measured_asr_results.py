#!/usr/bin/env python3
"""Validate sanitized native-ASR measurements, distinct from synthetic WER data."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import json
import math
from pathlib import Path
import re
import statistics
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
        assess_physical_4gb_eligibility,
        _unique_object,
        _reject_constant,
        percentile,
    )
    from .validate_fixture_manifest import (
        ALLOWED_CATEGORIES,
        ALLOWED_COVERAGE_ATTRIBUTES,
        ALLOWED_DURATION_CLASSES,
        assess_corpus_coverage,
    )
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
        assess_physical_4gb_eligibility,
        _unique_object,
        _reject_constant,
        percentile,
    )
    from validate_fixture_manifest import (
        ALLOWED_CATEGORIES,
        ALLOWED_COVERAGE_ATTRIBUTES,
        ALLOWED_DURATION_CLASSES,
        assess_corpus_coverage,
    )


SHA256_PATTERN = re.compile(r"[0-9a-f]{64}\Z")
FIXTURE_ID_PATTERN = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,63}\Z")
MACHINE_FIELD_PATTERN = re.compile(r"[A-Za-z0-9,._-]{1,100}\Z")
FORBIDDEN_CONTENT_KEYS = {
    "hypothesis", "transcript", "transcription", "reference_text", "audio_path",
    "reference_path", "command_line", "environment", "hostname", "username",
    "serial_number", "audio", "pcm", "waveform", "loaded_images", "loaded_library_paths",
}
ROOT_FIELDS = {
    "schema_version", "result_type", "evaluation_version", "created_at_utc",
    "normalization_policy_version", "reference_transcription_policy_version",
    "fixture_manifest_sha256", "source", "model", "native_binaries",
    "native_runtime_identity", "machine", "corpus", "aggregate", "timing",
    "fixtures", "release_gates", "limitations",
}
CORPUS_FIELDS = {
    "ready_fixture_count", "short_utterance_count", "long_utterance_count",
    "quiet_speech_count", "noisy_speech_count", "accented_speech_count",
    "accent_variety_count", "proper_nouns_count", "technical_vocabulary_count",
    "numbers_count", "abbreviations_count", "silence_control_count",
    "provenance_verified_fixture_count", "human_verified_reference_count",
    "requirements", "complete",
}
MACHINE_FIELDS = {
    "machine_label", "machine_identifier", "os", "os_release", "architecture",
    "physical_memory_bytes", "physical_memory_source", "physical_hardware_status",
    "reference_hardware_class", "cpu_only_inference_exercised",
    "real_hardware_measurements_completed", "logical_cpu_count", "cpu_label",
}
RELEASE_GATE_FIELDS = {
    "verified_corpus_coverage", "wer_at_or_below_12_percent", "rtf_p95_below_1",
    "stop_to_final_p95_below_1500_ms", "physical_4gb_eligibility", "physical_4gb_gate",
    "cpu_only_inference", "real_hardware_measurements", "release_performance_requirements",
    "process_tree_memory", "installed_application_size", "redistribution_approval",
}
TIMING_FIELDS = {
    "recognition_p50_ms", "recognition_p95_ms", "stop_to_final_p50_ms",
    "stop_to_final_p95_ms", "real_time_factor_p50", "real_time_factor_p95",
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


def _same_number(actual: Any, expected: float | None, *, tolerance: float = 1e-9) -> bool:
    if expected is None:
        return actual is None
    return _finite_number(actual) and abs(float(actual) - expected) <= tolerance


def _validate_machine(machine: Any) -> None:
    if not isinstance(machine, dict) or set(machine) != MACHINE_FIELDS:
        raise MeasuredResultsError("machine record shape is invalid")
    for field in ("machine_label", "machine_identifier", "os", "os_release", "architecture"):
        value = machine[field]
        if value is not None and (
            not isinstance(value, str) or not value or len(value) > 100
            or any(ord(char) < 32 for char in value)
        ):
            raise MeasuredResultsError("machine identity field is invalid")
    if machine["machine_identifier"] is not None and not MACHINE_FIELD_PATTERN.fullmatch(machine["machine_identifier"]):
        raise MeasuredResultsError("machine identifier is invalid")
    if machine["machine_label"] is not None and not re.fullmatch(r"[A-Za-z0-9._-]{1,64}", machine["machine_label"]):
        raise MeasuredResultsError("machine label is invalid")
    for field in ("physical_memory_bytes", "logical_cpu_count"):
        metric = machine[field]
        if metric is not None and (type(metric) is not int or metric <= 0):
            raise MeasuredResultsError("machine hardware metric is invalid")
    if machine["physical_memory_source"] is not None and (
        not isinstance(machine["physical_memory_source"], str)
        or machine["physical_memory_source"] not in {"sysctl_hw.memsize", "sysconf_pages"}
    ):
        raise MeasuredResultsError("physical memory source is unsupported")
    if (machine["physical_memory_bytes"] is None) != (machine["physical_memory_source"] is None):
        raise MeasuredResultsError("physical memory value and measurement source disagree")
    if not isinstance(machine["physical_hardware_status"], str) or machine["physical_hardware_status"] not in {
        "unknown", "verified_physical", "virtual_or_simulated"
    }:
        raise MeasuredResultsError("physical hardware status is invalid")
    if machine["reference_hardware_class"] is not None and (
        not isinstance(machine["reference_hardware_class"], str)
        or not machine["reference_hardware_class"]
        or len(machine["reference_hardware_class"]) > 100
    ):
        raise MeasuredResultsError("reference hardware class is invalid")
    for field in ("cpu_only_inference_exercised", "real_hardware_measurements_completed"):
        if type(machine[field]) is not bool:
            raise MeasuredResultsError("machine measurement status is invalid")
    if machine["physical_hardware_status"] != "verified_physical" and machine["real_hardware_measurements_completed"]:
        raise MeasuredResultsError("real hardware measurements require verified physical hardware")
    if machine["cpu_label"] is not None and (
        not isinstance(machine["cpu_label"], str) or len(machine["cpu_label"]) > 100
        or any(ord(char) < 32 for char in machine["cpu_label"])
    ):
        raise MeasuredResultsError("CPU label is invalid")


def validate_measured_results(value: Any) -> dict[str, Any]:
    """Validate native result identity, measurement arithmetic, coverage and privacy."""
    if not isinstance(value, dict) or set(value) != ROOT_FIELDS:
        raise MeasuredResultsError("root fields do not match the measured schema")
    _reject_content_keys(value)
    if type(value["schema_version"]) is not int or value["schema_version"] != RESULT_SCHEMA_VERSION or value["result_type"] != RESULT_TYPE:
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
        timestamp = datetime.fromisoformat(value["created_at_utc"].replace("Z", "+00:00"))
        if timestamp.tzinfo is None or timestamp.utcoffset() != timezone.utc.utcoffset(timestamp):
            raise ValueError
    except (AttributeError, TypeError, ValueError) as error:
        raise MeasuredResultsError("measurement timestamp must be UTC") from error

    source = value["source"]
    if (
        not isinstance(source, dict)
        or set(source) != {"source_revision", "c_api_header_version", "model_architecture"}
        or source["source_revision"] != SOURCE_REVISION
        or type(source["c_api_header_version"]) is not int
        or source["c_api_header_version"] != EXPECTED_HEADER_VERSION
        or type(source["model_architecture"]) is not int
        or source["model_architecture"] != EXPECTED_ARCHITECTURE
    ):
        raise MeasuredResultsError("pinned source identity is invalid")

    model = value["model"]
    model_fields = {
        "catalog_path", "immutable_artifact_revision", "architecture", "c_api_header_version",
        "asset_sha256", "asset_bytes", "total_asset_bytes", "cold_process_load_ms",
        "warm_reload_samples_ms", "warm_reload_median_ms", "inference_process_load_ms",
        "redistribution_status", "license_review_status",
    }
    if not isinstance(model, dict) or set(model) != model_fields:
        raise MeasuredResultsError("model record shape is invalid")
    if model["catalog_path"] != MODEL_CATALOG_PATH or model["immutable_artifact_revision"] is not None:
        raise MeasuredResultsError("model catalog and immutable revision fields are conflated")
    if type(model["architecture"]) is not int or model["architecture"] != EXPECTED_ARCHITECTURE or type(model["c_api_header_version"]) is not int or model["c_api_header_version"] != EXPECTED_HEADER_VERSION:
        raise MeasuredResultsError("model ABI identity is invalid")
    hashes, sizes = model["asset_sha256"], model["asset_bytes"]
    if not isinstance(hashes, dict) or set(hashes) != REQUIRED_MODEL_ASSETS or not all(_is_hash(digest) for digest in hashes.values()):
        raise MeasuredResultsError("model artifact hashes are incomplete or invalid")
    if not isinstance(sizes, dict) or set(sizes) != REQUIRED_MODEL_ASSETS or any(type(size) is not int or size <= 0 for size in sizes.values()):
        raise MeasuredResultsError("model artifact byte counts are incomplete or invalid")
    if type(model["total_asset_bytes"]) is not int or model["total_asset_bytes"] != sum(sizes.values()):
        raise MeasuredResultsError("model total does not equal artifact byte counts")
    for field in ("cold_process_load_ms", "warm_reload_median_ms", "inference_process_load_ms"):
        if not _finite_number(model[field]):
            raise MeasuredResultsError("model load timing is invalid")
    warm_samples = model["warm_reload_samples_ms"]
    if not isinstance(warm_samples, list) or not warm_samples or any(not _finite_number(sample) for sample in warm_samples):
        raise MeasuredResultsError("warm model load samples are invalid")
    if not _same_number(model["warm_reload_median_ms"], statistics.median(warm_samples)):
        raise MeasuredResultsError("warm model-load median does not match its samples")
    if model["redistribution_status"] != "undetermined" or model["license_review_status"] != "review-pending":
        raise MeasuredResultsError("model approval status is unsupported")

    binaries = value["native_binaries"]
    expected_binaries = {"native_runner", "moonshine_library", "onnxruntime_library"}
    if not isinstance(binaries, dict) or set(binaries) != expected_binaries:
        raise MeasuredResultsError("native binary record is incomplete")
    for binary in binaries.values():
        if not isinstance(binary, dict) or set(binary) != {"bytes", "sha256"} or type(binary["bytes"]) is not int or binary["bytes"] <= 0 or not _is_hash(binary["sha256"]):
            raise MeasuredResultsError("native binary identity is invalid")

    machine = value["machine"]
    _validate_machine(machine)
    runtime = value["native_runtime_identity"]
    if not isinstance(runtime, dict) or set(runtime) != {
        "architecture", "c_api_version", "loaded_image_count", "unexpected_non_system_library_count",
        "resolved_paths_match_inputs", "dependency_chain_verified", "native_runner_sha256",
        "moonshine_sha256", "onnxruntime_sha256",
    }:
        raise MeasuredResultsError("native runtime identity shape is invalid")
    if (
        runtime["architecture"] != machine["architecture"]
        or type(runtime["c_api_version"]) is not int or runtime["c_api_version"] != EXPECTED_HEADER_VERSION
        or type(runtime["loaded_image_count"]) is not int or runtime["loaded_image_count"] < 3
        or type(runtime["unexpected_non_system_library_count"]) is not int or runtime["unexpected_non_system_library_count"] != 0
        or runtime["resolved_paths_match_inputs"] is not True
        or runtime["dependency_chain_verified"] is not True
        or runtime["native_runner_sha256"] != binaries["native_runner"]["sha256"]
        or runtime["moonshine_sha256"] != binaries["moonshine_library"]["sha256"]
        or runtime["onnxruntime_sha256"] != binaries["onnxruntime_library"]["sha256"]
    ):
        raise MeasuredResultsError("loaded native runtime does not match the recorded binaries")

    fixtures = value["fixtures"]
    if not isinstance(fixtures, list) or not fixtures:
        raise MeasuredResultsError("measured result contains no fixture rows")
    seen_ids: set[str] = set()
    speech_rows: list[dict[str, Any]] = []
    coverage_fixtures: list[dict[str, Any]] = []
    silence_count = 0
    for index, row in enumerate(fixtures):
        row_fields = {
            "fixture_id", "category", "duration_class", "coverage_attributes", "speaker_variety",
            "provenance_verified", "human_reference_verified", "audio_sha256", "reference_sha256",
            "audio_metadata", "wer", "error_counts", "no_speech_detected", "partial_calls",
            "partial_nonempty", "final_line_count", "recognition_ms", "stop_to_final_ms",
            "real_time_factor", "native_process_cpu_ms",
        }
        if not isinstance(row, dict) or set(row) != row_fields:
            raise MeasuredResultsError(f"fixture result shape is invalid at row {index}")
        fixture_id = row["fixture_id"]
        if not isinstance(fixture_id, str) or not FIXTURE_ID_PATTERN.fullmatch(fixture_id) or fixture_id in seen_ids:
            raise MeasuredResultsError("fixture IDs are invalid or duplicated")
        seen_ids.add(fixture_id)
        category = row["category"]
        duration_class = row["duration_class"]
        attrs = row["coverage_attributes"]
        if not isinstance(category, str) or category not in ALLOWED_CATEGORIES or not isinstance(duration_class, str) or duration_class not in ALLOWED_DURATION_CLASSES:
            raise MeasuredResultsError("fixture category or duration class is invalid")
        if not isinstance(attrs, list) or any(not isinstance(attr, str) or attr not in ALLOWED_COVERAGE_ATTRIBUTES for attr in attrs) or len(attrs) != len(set(attrs)):
            raise MeasuredResultsError("fixture coverage attributes are invalid")
        if {"quiet_environment", "noisy_environment"}.issubset(set(attrs)):
            raise MeasuredResultsError("fixture cannot be both quiet and noisy")
        if duration_class == "silence_control":
            if category != "silence" or attrs:
                raise MeasuredResultsError("silence control has incompatible category or speech coverage")
        elif category == "silence":
            raise MeasuredResultsError("speech fixture cannot use the silence category")
        if category == "short_speech" and duration_class != "short" or category == "long_speech" and duration_class != "long":
            raise MeasuredResultsError("legacy speech category conflicts with duration class")
        category_attribute = {
            "quiet_environment": "quiet_environment", "noisy_environment": "noisy_environment",
            "accented_english": "accented_english", "numbers": "numbers",
        }.get(category)
        if category_attribute is not None and category_attribute not in attrs:
            raise MeasuredResultsError("legacy category is missing from coverage attributes")
        if category == "proper_nouns_technical_vocabulary" and not set(attrs).intersection({"proper_nouns", "technical_vocabulary"}):
            raise MeasuredResultsError("legacy vocabulary category has no matching coverage attribute")
        variety = row["speaker_variety"]
        if "accented_english" in attrs:
            if not isinstance(variety, str) or not variety.strip() or len(variety) > 100:
                raise MeasuredResultsError("accented fixture is missing its source-reported variety label")
        elif variety is not None:
            raise MeasuredResultsError("speaker variety label requires accented English coverage")
        if row["provenance_verified"] is not True or row["human_reference_verified"] is not True:
            raise MeasuredResultsError("measured fixture is missing verified provenance or a human reference")
        for hash_field in ("audio_sha256", "reference_sha256"):
            if not _is_hash(row[hash_field]):
                raise MeasuredResultsError("fixture checksum is invalid")

        metadata = row["audio_metadata"]
        if not isinstance(metadata, dict) or set(metadata) != {"sample_format", "sample_rate_hz", "channels", "frame_count", "duration_seconds"}:
            raise MeasuredResultsError("fixture audio metadata is invalid")
        if (
            metadata["sample_format"] != "PCM_S16LE"
            or any(type(metadata[field]) is not int or metadata[field] <= 0 for field in ("sample_rate_hz", "channels", "frame_count"))
            or not _finite_number(metadata["duration_seconds"], minimum=0.000001)
            or metadata["duration_seconds"] > 120
            or abs(metadata["duration_seconds"] - metadata["frame_count"] / metadata["sample_rate_hz"]) > 0.001
        ):
            raise MeasuredResultsError("fixture audio metadata is inconsistent")
        if duration_class == "short" and metadata["duration_seconds"] > 5:
            raise MeasuredResultsError("short fixture duration exceeds 5 seconds")
        if duration_class == "long" and metadata["duration_seconds"] <= 5:
            raise MeasuredResultsError("long fixture duration must exceed 5 seconds")
        for field in ("partial_calls", "final_line_count"):
            if type(row[field]) is not int or row[field] < 0:
                raise MeasuredResultsError("fixture inference count is invalid")
        if type(row["partial_nonempty"]) is not bool:
            raise MeasuredResultsError("fixture partial status is invalid")
        for field in ("recognition_ms", "stop_to_final_ms", "real_time_factor"):
            if not _finite_number(row[field]):
                raise MeasuredResultsError("fixture timing is invalid")
        if row["stop_to_final_ms"] > row["recognition_ms"] + 0.01:
            raise MeasuredResultsError("stop-to-final time cannot exceed total recognition time")
        if row["native_process_cpu_ms"] is not None and not _finite_number(row["native_process_cpu_ms"]):
            raise MeasuredResultsError("fixture CPU time is invalid")
        if duration_class == "silence_control":
            silence_count += 1
            if row["wer"] is not None or row["error_counts"] is not None or type(row["no_speech_detected"]) is not bool:
                raise MeasuredResultsError("silence control must be separate from speech WER")
        else:
            counts = row["error_counts"]
            if not isinstance(counts, dict) or set(counts) != {"reference_words", "substitutions", "deletions", "insertions"}:
                raise MeasuredResultsError("speech fixture error counts are missing")
            if any(type(count) is not int or count < 0 for count in counts.values()) or counts["reference_words"] < 1:
                raise MeasuredResultsError("speech fixture error counts are invalid")
            if counts["substitutions"] + counts["deletions"] > counts["reference_words"]:
                raise MeasuredResultsError("speech substitutions and deletions exceed reference words")
            expected_wer = (counts["substitutions"] + counts["deletions"] + counts["insertions"]) / counts["reference_words"]
            if not _same_number(row["wer"], expected_wer):
                raise MeasuredResultsError("speech fixture WER does not match its error counts")
            if row["no_speech_detected"] is not None:
                raise MeasuredResultsError("speech rows cannot declare no-speech detection")
            expected_rtf = row["recognition_ms"] / (metadata["duration_seconds"] * 1000)
            if abs(row["real_time_factor"] - expected_rtf) > 0.01:
                raise MeasuredResultsError("fixture real-time factor does not match recognition time and audio duration")
            speech_rows.append(row)
        coverage_fixtures.append({
            "status": "ready", "duration_class": duration_class, "coverage_attributes": attrs,
            "speaker_variety": variety, "provenance_verified": row["provenance_verified"],
            "human_reference_verified": row["human_reference_verified"],
        })

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
    if any(type(aggregate[key]) is not int or aggregate[key] != expected for key, expected in expected_counts.items()):
        raise MeasuredResultsError("aggregate counts do not match fixture records")
    expected_wer = None if not speech_rows else (expected_counts["substitutions"] + expected_counts["deletions"] + expected_counts["insertions"]) / expected_counts["reference_words"]
    if not _same_number(aggregate["wer"], expected_wer):
        raise MeasuredResultsError("aggregate WER does not match fixture records")

    corpus = value["corpus"]
    if not isinstance(corpus, dict) or set(corpus) != CORPUS_FIELDS:
        raise MeasuredResultsError("corpus summary shape is invalid")
    expected_corpus = assess_corpus_coverage(coverage_fixtures)
    if corpus != expected_corpus:
        raise MeasuredResultsError("corpus coverage summary does not match fixture records")

    if not isinstance(value["timing"], dict) or set(value["timing"]) != TIMING_FIELDS:
        raise MeasuredResultsError("timing summary shape is invalid")
    timing_values = {
        "recognition_p50_ms": percentile([row["recognition_ms"] for row in speech_rows], 0.50),
        "recognition_p95_ms": percentile([row["recognition_ms"] for row in speech_rows], 0.95),
        "stop_to_final_p50_ms": percentile([row["stop_to_final_ms"] for row in speech_rows], 0.50),
        "stop_to_final_p95_ms": percentile([row["stop_to_final_ms"] for row in speech_rows], 0.95),
        "real_time_factor_p50": percentile([row["real_time_factor"] for row in speech_rows], 0.50),
        "real_time_factor_p95": percentile([row["real_time_factor"] for row in speech_rows], 0.95),
    }
    if any(not _same_number(value["timing"][field], expected, tolerance=1e-8) for field, expected in timing_values.items()):
        raise MeasuredResultsError("timing summary does not match fixture measurements")

    gates = value["release_gates"]
    if not isinstance(gates, dict) or set(gates) != RELEASE_GATE_FIELDS:
        raise MeasuredResultsError("release gate record shape is invalid")
    coverage_status = "PASS" if expected_corpus["complete"] else "NOT_ASSESSED"
    wer_status = "NOT_ASSESSED" if not expected_corpus["complete"] or expected_wer is None else "PASS" if expected_wer <= 0.12 else "FAIL"
    rtf = timing_values["real_time_factor_p95"]
    rtf_status = "NOT_ASSESSED" if not expected_corpus["complete"] or rtf is None else "PASS" if rtf < 1.0 else "FAIL"
    stop_time = timing_values["stop_to_final_p95_ms"]
    stop_status = "NOT_ASSESSED" if not expected_corpus["complete"] or stop_time is None else "PASS" if stop_time < 1500 else "FAIL"
    eligibility = assess_physical_4gb_eligibility(machine)
    expected_gates = {
        "verified_corpus_coverage": coverage_status,
        "wer_at_or_below_12_percent": wer_status,
        "rtf_p95_below_1": rtf_status,
        "stop_to_final_p95_below_1500_ms": stop_status,
        "physical_4gb_eligibility": eligibility,
        "physical_4gb_gate": "BLOCKED" if eligibility == "INELIGIBLE" else "NOT_ASSESSED",
        "cpu_only_inference": "PASS" if machine["cpu_only_inference_exercised"] else "NOT_ASSESSED",
        "real_hardware_measurements": "PASS" if machine["real_hardware_measurements_completed"] else "NOT_ASSESSED",
        "release_performance_requirements": "NOT_ASSESSED",
        "process_tree_memory": "NOT_MEASURED",
        "installed_application_size": "NOT_MEASURED",
        "redistribution_approval": "NOT_APPROVED",
    }
    if gates != expected_gates:
        raise MeasuredResultsError("release gates are inconsistent with measured evidence")
    limitations = value["limitations"]
    if not isinstance(limitations, list) or not limitations or any(not isinstance(item, str) or not item.strip() for item in limitations):
        raise MeasuredResultsError("result limitations are invalid")
    return {
        "fixtures": len(fixtures),
        "speech_fixtures": len(speech_rows),
        "silence_controls": silence_count,
        "wer": expected_wer,
        "coverage_complete": expected_corpus["complete"],
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
    print(f"Required diverse corpus coverage: {'PASS' if summary['coverage_complete'] else 'NOT ASSESSED'}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
