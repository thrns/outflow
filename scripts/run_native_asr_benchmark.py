#!/usr/bin/env python3
"""Run verified English fixtures through the pinned native Moonshine C API.

The native helper returns final hypotheses over captured stdout. This runner
keeps that pipe and all reference/hypothesis strings in volatile memory, scores
raw ASR output with the frozen lexical WER policy, then writes only hashes,
counts, timings, and sanitized machine/model provenance. It refuses planned,
unverified, mismatched, or malformed fixtures and never prints native output.
"""

from __future__ import annotations

import argparse
from datetime import UTC, datetime
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import sys
import wave
from typing import Any, Mapping, Sequence

if __package__:
    from .score_corpus_wer import calculate_corpus_wer
    from .score_wer import NORMALIZATION_POLICY_VERSION, calculate_wer
    from .validate_fixture_manifest import REPOSITORY_ROOT, validate_manifest
    from .validate_model_asset_inventory import load_manifest as load_asset_manifest
    from .validate_model_asset_inventory import validate_manifest as validate_asset_manifest
else:
    from score_corpus_wer import calculate_corpus_wer
    from score_wer import NORMALIZATION_POLICY_VERSION, calculate_wer
    from validate_fixture_manifest import REPOSITORY_ROOT, validate_manifest
    from validate_model_asset_inventory import load_manifest as load_asset_manifest
    from validate_model_asset_inventory import validate_manifest as validate_asset_manifest


RESULT_SCHEMA_VERSION = 1
RESULT_TYPE = "measured_native_asr"
EVALUATION_VERSION = "outflow-native-asr-benchmark-v1"
SOURCE_REVISION = "234f60faa0eb388b01cdf7e60aca232af37aefda"
MODEL_CATALOG_PATH = "small-streaming-en/quantized_26_08_21"
EXPECTED_ARCHITECTURE = 4
EXPECTED_HEADER_VERSION = 30000
REQUIRED_MODEL_ASSETS = {
    "adapter.ort",
    "cross_kv.ort",
    "decoder_kv.ort",
    "encoder.ort",
    "frontend.model.ort",
    "frontend.weights.ort",
    "streaming_config.json",
    "tokenizer.bin",
}
MAX_MANIFEST_BYTES = 4 * 1024 * 1024
MAX_REFERENCE_BYTES = 1024 * 1024
MAX_AUDIO_BYTES = 64 * 1024 * 1024
MAX_AUDIO_SECONDS = 120.0
SHA256_PATTERN = re.compile(r"[0-9a-fA-F]{64}\Z")
MACHINE_LABEL_PATTERN = re.compile(r"[A-Za-z0-9._-]{1,64}\Z")


class BenchmarkError(RuntimeError):
    """A benchmark input, source, model, native run, or output failed safely."""


class _DuplicateJsonKey(ValueError):
    """Indicates a repeated key without exposing untrusted contents."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise _DuplicateJsonKey
        result[key] = value
    return result


def _reject_constant(_: str) -> None:
    raise ValueError("non-finite number")


def load_json(path: Path) -> Any:
    """Read a bounded JSON file and return content-free errors."""

    try:
        raw = path.read_bytes()
    except OSError as error:
        raise BenchmarkError("a benchmark input file could not be read") from error
    if len(raw) > MAX_MANIFEST_BYTES:
        raise BenchmarkError("a benchmark input file exceeds the supported size limit")
    try:
        return json.loads(
            raw.decode("utf-8"),
            object_pairs_hook=_unique_object,
            parse_constant=_reject_constant,
        )
    except (_DuplicateJsonKey, UnicodeError, json.JSONDecodeError, ValueError, RecursionError) as error:
        raise BenchmarkError("a benchmark input file is not valid JSON") from error


def file_sha256(path: Path) -> str:
    """Hash a file incrementally without logging its contents or path."""

    digest = hashlib.sha256()
    try:
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise BenchmarkError("a benchmark artifact could not be hashed") from error
    return digest.hexdigest()


def validate_wav(path: Path, declared: Mapping[str, Any]) -> dict[str, Any]:
    """Check PCM16 mono WAV metadata against the frozen manifest."""

    try:
        if path.stat().st_size > MAX_AUDIO_BYTES:
            raise BenchmarkError("a benchmark audio fixture exceeds the size limit")
        with wave.open(str(path), "rb") as audio:
            channels = audio.getnchannels()
            sample_width = audio.getsampwidth()
            sample_rate = audio.getframerate()
            frame_count = audio.getnframes()
            compression = audio.getcomptype()
    except (OSError, EOFError, wave.Error) as error:
        raise BenchmarkError("a benchmark fixture is not a readable PCM WAV") from error
    duration = frame_count / sample_rate if sample_rate > 0 else 0.0
    if (
        channels != 1
        or sample_width != 2
        or compression != "NONE"
        or sample_rate <= 0
        or frame_count <= 0
        or duration > MAX_AUDIO_SECONDS
    ):
        raise BenchmarkError("a benchmark fixture is not supported mono PCM16 audio")
    actual = {
        "sample_format": "PCM_S16LE",
        "sample_rate_hz": sample_rate,
        "channels": channels,
        "frame_count": frame_count,
        "duration_seconds": duration,
    }
    if any(
        declared.get(key) != value
        for key, value in actual.items()
        if key != "duration_seconds"
    ):
        raise BenchmarkError("measured fixture audio format does not match its manifest")
    try:
        declared_duration = float(declared["duration_seconds"])
    except (KeyError, TypeError, ValueError, OverflowError) as error:
        raise BenchmarkError("fixture duration metadata is invalid") from error
    if not math.isfinite(declared_duration) or abs(declared_duration - duration) > 0.001:
        raise BenchmarkError("measured fixture duration does not match its manifest")
    return actual


def _contained_path(repository_root: Path, relative_path: str) -> Path:
    """Resolve a manifest path only when it remains within the repository."""

    if not isinstance(relative_path, str) or not relative_path or "\\" in relative_path:
        raise BenchmarkError("a benchmark fixture path is invalid")
    candidate = repository_root / relative_path
    try:
        root = repository_root.resolve(strict=True)
        resolved = candidate.resolve(strict=True)
        resolved.relative_to(root)
    except (OSError, RuntimeError, ValueError) as error:
        raise BenchmarkError("a benchmark fixture path is missing or escapes the repository") from error
    if not resolved.is_file():
        raise BenchmarkError("a benchmark fixture path is not a regular file")
    return resolved


def load_verified_fixtures(manifest_path: Path, repository_root: Path) -> tuple[dict[str, Any], list[dict[str, Any]], str]:
    """Validate readiness, file hashes, audio metadata, and references."""

    manifest = load_json(manifest_path)
    errors, counts = validate_manifest(manifest, repository_root)
    if errors:
        raise BenchmarkError(f"fixture manifest validation failed ({len(errors)} issue(s))")
    if manifest.get("normalization_policy_version") != NORMALIZATION_POLICY_VERSION:
        raise BenchmarkError("fixture manifest normalization policy does not match the scorer")
    ready = [fixture for fixture in manifest["fixtures"] if fixture.get("status") == "ready"]
    if counts["ready_audio_fixtures"] != len(ready) or not ready:
        raise BenchmarkError("no verified ready fixtures are available for native evaluation")

    loaded: list[dict[str, Any]] = []
    for fixture in ready:
        audio_path = _contained_path(repository_root, fixture["audio_file"])
        reference_path = _contained_path(repository_root, fixture["reference_file"])
        actual_audio_hash = file_sha256(audio_path)
        if actual_audio_hash.lower() != fixture["audio_sha256"].lower():
            raise BenchmarkError("fixture audio checksum changed after manifest validation")
        actual_reference_hash = file_sha256(reference_path)
        if actual_reference_hash.lower() != fixture["reference_sha256"].lower():
            raise BenchmarkError("fixture reference checksum changed after manifest validation")
        audio_metadata = validate_wav(audio_path, fixture["audio_metadata"])
        try:
            if reference_path.stat().st_size > MAX_REFERENCE_BYTES:
                raise BenchmarkError("a benchmark reference exceeds the size limit")
            reference = reference_path.read_text(encoding="utf-8")
        except (OSError, UnicodeError) as error:
            raise BenchmarkError("a benchmark reference could not be read as UTF-8") from error
        if fixture["category"] != "silence" and calculate_wer(reference, "").wer is None:
            raise BenchmarkError("a speech fixture has an empty normalized reference")
        loaded.append(
            {
                "id": fixture["id"],
                "category": fixture["category"],
                "audio_path": audio_path,
                "reference": reference,
                "audio_sha256": actual_audio_hash,
                "reference_sha256": actual_reference_hash,
                "audio_metadata": audio_metadata,
            }
        )
    return manifest, loaded, file_sha256(manifest_path)


def _is_sha256(value: Any) -> bool:
    return isinstance(value, str) and bool(SHA256_PATTERN.fullmatch(value))


def verify_source(source_dir: Path) -> dict[str, Any]:
    """Verify the exact clean upstream source and pinned C API constants."""

    try:
        revision = subprocess.run(
            ["git", "-C", str(source_dir), "rev-parse", "HEAD"],
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        ).stdout.strip()
        status = subprocess.run(
            ["git", "-C", str(source_dir), "status", "--porcelain", "--untracked-files=all"],
            check=True,
            capture_output=True,
            text=True,
            timeout=10,
        ).stdout
        header = (source_dir / "core" / "moonshine-c-api.h").read_text(encoding="utf-8")
    except (OSError, subprocess.SubprocessError, UnicodeError) as error:
        raise BenchmarkError("pinned Moonshine source or C API header could not be verified") from error
    if revision != SOURCE_REVISION or status.strip():
        raise BenchmarkError("Moonshine source is not the required clean pinned revision")
    header_match = re.search(r"#define\s+MOONSHINE_HEADER_VERSION\s+(\d+)", header)
    architecture_match = re.search(r"MOONSHINE_MODEL_ARCH_SMALL_STREAMING\s*=\s*(\d+)", header)
    if (
        header_match is None
        or architecture_match is None
        or int(header_match.group(1)) != EXPECTED_HEADER_VERSION
        or int(architecture_match.group(1)) != EXPECTED_ARCHITECTURE
    ):
        raise BenchmarkError("Moonshine C API header does not match the expected contract")
    return {
        "source_revision": revision,
        "c_api_header_version": int(header_match.group(1)),
        "model_architecture": int(architecture_match.group(1)),
    }


def verify_model_assets(manifest_path: Path, asset_root: Path) -> dict[str, Any]:
    """Validate the eight local model files and keep observed SHA separate from revision."""

    asset_manifest = load_asset_manifest(manifest_path)
    errors, counts = validate_asset_manifest(asset_manifest, asset_root)
    if errors:
        raise BenchmarkError(f"model asset inventory validation failed ({len(errors)} issue(s))")
    selected = [
        asset
        for asset in asset_manifest["assets"]
        if asset["asset_id"].startswith("moonshine-small-streaming-")
    ]
    names = {asset["file_path"] for asset in selected if asset["file_path"] is not None}
    if names != REQUIRED_MODEL_ASSETS or len(selected) != len(REQUIRED_MODEL_ASSETS):
        raise BenchmarkError("Small Streaming inventory does not contain exactly eight expected files")
    if any(asset["status"] != "file-verified" for asset in selected):
        raise BenchmarkError("one or more Small Streaming files are not locally hash-verified")
    if any(asset["source_revision"] is not None for asset in selected):
        raise BenchmarkError("model source revision is conflated with the observed artifact SHA")
    if any(not _is_sha256(asset["sha256"]) for asset in selected):
        raise BenchmarkError("one or more Small Streaming artifact SHA-256 values are missing")
    model_bytes = sum((asset_root / asset["file_path"]).stat().st_size for asset in selected)
    return {
        "catalog_path": MODEL_CATALOG_PATH,
        "immutable_artifact_revision": None,
        "asset_sha256": {asset["file_path"]: asset["sha256"].lower() for asset in sorted(selected, key=lambda x: x["file_path"])},
        "asset_bytes": {asset["file_path"]: (asset_root / asset["file_path"]).stat().st_size for asset in sorted(selected, key=lambda x: x["file_path"])},
        "total_bytes": model_bytes,
        "redistribution_status": "undetermined",
        "license_review_status": "review-pending",
    }


def _run_json_command(command: Sequence[str], *, timeout_seconds: float) -> Any:
    """Run the local native helper and parse captured stdout without exposing it."""

    try:
        completed = subprocess.run(
            list(command),
            check=False,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="strict",
            timeout=timeout_seconds,
        )
    except (OSError, UnicodeError, subprocess.SubprocessError) as error:
        raise BenchmarkError("native Moonshine command failed or timed out") from error
    if completed.returncode != 0:
        # Do not include stdout/stderr: either stream could contain transcript text.
        raise BenchmarkError(f"native Moonshine command failed (exit {completed.returncode})")
    try:
        return json.loads(completed.stdout, parse_constant=_reject_constant)
    except (json.JSONDecodeError, ValueError, RecursionError) as error:
        raise BenchmarkError("native Moonshine command returned malformed output") from error


def parse_load_probe(value: Any, *, required_samples: int = 2) -> list[float]:
    """Validate cold-process and warm-reload timing samples from the helper."""

    if not isinstance(value, dict) or set(value) != {"schema_version", "model_load_ms"}:
        raise BenchmarkError("native model-load probe has an invalid shape")
    samples = value["model_load_ms"]
    if value["schema_version"] != 1 or not isinstance(samples, list) or len(samples) < required_samples:
        raise BenchmarkError("native model-load probe is incomplete")
    if any(
        isinstance(sample, bool)
        or not isinstance(sample, (int, float))
        or not math.isfinite(float(sample))
        or sample < 0
        for sample in samples
    ):
        raise BenchmarkError("native model-load probe contains invalid timing values")
    return [float(sample) for sample in samples]


def parse_native_output(value: Any, expected_count: int) -> tuple[float, list[dict[str, Any]]]:
    """Validate batch inference output while keeping hypotheses in memory only."""

    required_root = {"schema_version", "architecture", "header_version", "model_load_ms", "outputs"}
    if not isinstance(value, dict) or set(value) != required_root:
        raise BenchmarkError("native inference output has an invalid shape")
    if (
        value["schema_version"] != 1
        or value["architecture"] != EXPECTED_ARCHITECTURE
        or value["header_version"] != EXPECTED_HEADER_VERSION
    ):
        raise BenchmarkError("native inference output has an unexpected model contract")
    outputs = value["outputs"]
    if not isinstance(outputs, list) or len(outputs) != expected_count:
        raise BenchmarkError("native inference returned an unexpected fixture count")
    expected_fields = {
        "index",
        "hypothesis",
        "partial_calls",
        "partial_nonempty",
        "final_lines",
        "recognition_ms",
        "stop_to_final_ms",
        "real_time_factor",
        "cpu_ms",
    }
    for index, output in enumerate(outputs):
        if not isinstance(output, dict) or set(output) != expected_fields:
            raise BenchmarkError("native inference output has an invalid fixture record")
        if output["index"] != index or not isinstance(output["hypothesis"], str):
            raise BenchmarkError("native inference output fixture order is invalid")
        if type(output["partial_nonempty"]) is not bool:
            raise BenchmarkError("native inference partial status is invalid")
        for field in ("partial_calls", "final_lines"):
            if type(output[field]) is not int or output[field] < 0:
                raise BenchmarkError("native inference count field is invalid")
        for field in ("recognition_ms", "stop_to_final_ms", "real_time_factor"):
            number = output[field]
            if isinstance(number, bool) or not isinstance(number, (int, float)) or not math.isfinite(float(number)) or number < 0:
                raise BenchmarkError("native inference timing field is invalid")
        cpu_ms = output["cpu_ms"]
        if cpu_ms is not None and (
            isinstance(cpu_ms, bool)
            or not isinstance(cpu_ms, (int, float))
            or not math.isfinite(float(cpu_ms))
            or cpu_ms < 0
        ):
            raise BenchmarkError("native inference CPU timing field is invalid")
    model_load = value["model_load_ms"]
    if isinstance(model_load, bool) or not isinstance(model_load, (int, float)) or not math.isfinite(float(model_load)) or model_load < 0:
        raise BenchmarkError("native inference model-load timing is invalid")
    return float(model_load), outputs


def percentile(values: Sequence[float], fraction: float) -> float | None:
    """Return a linear-interpolated percentile without external dependencies."""

    if not values:
        return None
    if not 0 <= fraction <= 1:
        raise ValueError("fraction must be between zero and one")
    ordered = sorted(float(value) for value in values)
    position = (len(ordered) - 1) * fraction
    lower = math.floor(position)
    upper = math.ceil(position)
    if lower == upper:
        return ordered[lower]
    return ordered[lower] + (ordered[upper] - ordered[lower]) * (position - lower)


def _physical_memory_bytes() -> int | None:
    try:
        pages = os.sysconf("SC_PHYS_PAGES")
        page_size = os.sysconf("SC_PAGE_SIZE")
        if type(pages) is int and type(page_size) is int and pages > 0 and page_size > 0:
            return pages * page_size
    except (OSError, ValueError):
        return None
    return None


def machine_record(machine_label: str | None = None, cpu_label: str | None = None) -> dict[str, Any]:
    """Return host facts without hostname, serial number, username, or paths."""

    if machine_label is not None and not MACHINE_LABEL_PATTERN.fullmatch(machine_label):
        raise BenchmarkError("machine label must contain only simple non-identifying characters")
    safe_cpu = cpu_label or platform.processor() or None
    if safe_cpu is not None and (len(safe_cpu) > 100 or any(ord(char) < 32 for char in safe_cpu)):
        raise BenchmarkError("CPU label is invalid")
    return {
        "machine_label": machine_label,
        "os": platform.system(),
        "os_release": platform.release(),
        "architecture": platform.machine(),
        "physical_memory_bytes": _physical_memory_bytes(),
        "logical_cpu_count": os.cpu_count(),
        "cpu_label": safe_cpu,
    }


def _binary_record(path: Path) -> dict[str, Any]:
    """Return a binary hash and exact file length without its local path."""

    try:
        stat = path.stat()
        if not path.is_file():
            raise OSError
    except OSError as error:
        raise BenchmarkError("a required native binary is missing or unreadable") from error
    return {"bytes": stat.st_size, "sha256": file_sha256(path)}


def build_measured_result(
    *,
    manifest: dict[str, Any],
    fixtures: Sequence[dict[str, Any]],
    fixture_manifest_sha256: str,
    native_output: tuple[float, list[dict[str, Any]]],
    load_samples_ms: Sequence[float],
    native_files: Mapping[str, Path],
    source: Mapping[str, Any],
    model: Mapping[str, Any],
    machine: Mapping[str, Any],
    created_at_utc: str,
) -> dict[str, Any]:
    """Score in-memory hypotheses and construct a transcript-free result."""

    actual_model_load_ms, outputs = native_output
    if len(fixtures) != len(outputs):
        raise BenchmarkError("fixture and native result counts differ")
    sample_results: list[dict[str, Any]] = []
    speech_pairs: list[tuple[str, str]] = []
    for index, (fixture, output) in enumerate(zip(fixtures, outputs, strict=True)):
        hypothesis = output["hypothesis"]
        is_silence = fixture["category"] == "silence"
        wer = None
        counts = None
        if is_silence:
            no_speech_detected = not hypothesis.strip()
        else:
            alignment = calculate_wer(fixture["reference"], hypothesis)
            if alignment.wer is None:
                raise BenchmarkError("a speech fixture unexpectedly has no reference tokens")
            wer = alignment.wer
            counts = {
                "reference_words": alignment.reference_word_count,
                "substitutions": alignment.substitutions,
                "deletions": alignment.deletions,
                "insertions": alignment.insertions,
            }
            speech_pairs.append((fixture["reference"], hypothesis))
            no_speech_detected = None
        if not is_silence and not hypothesis.strip():
            # An empty final result is a valid recognition output and scores as deletions.
            no_speech_detected = False
        sample_results.append(
            {
                "fixture_id": fixture["id"],
                "category": fixture["category"],
                "audio_sha256": fixture["audio_sha256"],
                "reference_sha256": fixture["reference_sha256"],
                "audio_metadata": fixture["audio_metadata"],
                "wer": wer,
                "error_counts": counts,
                "no_speech_detected": no_speech_detected,
                "partial_calls": output["partial_calls"],
                "partial_nonempty": output["partial_nonempty"],
                "final_line_count": output["final_lines"],
                "recognition_ms": float(output["recognition_ms"]),
                "stop_to_final_ms": float(output["stop_to_final_ms"]),
                "real_time_factor": float(output["real_time_factor"]),
                "native_process_cpu_ms": None if output["cpu_ms"] is None else float(output["cpu_ms"]),
            }
        )
    if speech_pairs:
        aggregate = calculate_corpus_wer(speech_pairs)
        aggregate_record: dict[str, Any] = {
            "speech_fixtures_scored": aggregate.utterances_scored,
            "reference_words": aggregate.reference_word_count,
            "substitutions": aggregate.substitutions,
            "deletions": aggregate.deletions,
            "insertions": aggregate.insertions,
            "wer": aggregate.wer,
        }
    else:
        aggregate_record = {
            "speech_fixtures_scored": 0,
            "reference_words": 0,
            "substitutions": 0,
            "deletions": 0,
            "insertions": 0,
            "wer": None,
        }
    speech = [row for row in sample_results if row["category"] != "silence"]
    recognition = [float(row["recognition_ms"]) for row in speech]
    stop_to_final = [float(row["stop_to_final_ms"]) for row in speech]
    rtfs = [float(row["real_time_factor"]) for row in speech]
    ready_categories = [fixture["category"] for fixture in fixtures]
    short_count = ready_categories.count("short_speech")
    long_count = ready_categories.count("long_speech")
    required_coverage_met = short_count >= 100 and long_count >= 50
    wer_value = aggregate_record["wer"]
    physical_ram = machine.get("physical_memory_bytes")
    physical_4gb = physical_ram is not None and physical_ram <= 4_500_000_000
    results = {
        "schema_version": RESULT_SCHEMA_VERSION,
        "result_type": RESULT_TYPE,
        "evaluation_version": EVALUATION_VERSION,
        "created_at_utc": created_at_utc,
        "normalization_policy_version": manifest["normalization_policy_version"],
        "reference_transcription_policy_version": manifest["reference_transcription_policy_version"],
        "fixture_manifest_sha256": fixture_manifest_sha256,
        "source": dict(source),
        "model": {
            "catalog_path": model["catalog_path"],
            "immutable_artifact_revision": model["immutable_artifact_revision"],
            "architecture": EXPECTED_ARCHITECTURE,
            "c_api_header_version": EXPECTED_HEADER_VERSION,
            "asset_sha256": dict(model["asset_sha256"]),
            "asset_bytes": dict(model["asset_bytes"]),
            "total_asset_bytes": model["total_bytes"],
            "cold_process_load_ms": load_samples_ms[0],
            "warm_reload_samples_ms": list(load_samples_ms[1:]),
            "warm_reload_median_ms": statistics.median(load_samples_ms[1:]),
            "inference_process_load_ms": actual_model_load_ms,
            "redistribution_status": model["redistribution_status"],
            "license_review_status": model["license_review_status"],
        },
        "native_binaries": {name: _binary_record(path) for name, path in sorted(native_files.items())},
        "machine": dict(machine),
        "corpus": {
            "verified_ready_fixture_count": len(fixtures),
            "short_phrase_count": short_count,
            "long_phrase_count": long_count,
            "quiet_environment_count": ready_categories.count("quiet_environment"),
            "noisy_environment_count": ready_categories.count("noisy_environment"),
            "accented_english_count": ready_categories.count("accented_english"),
            "proper_nouns_technical_vocabulary_count": ready_categories.count("proper_nouns_technical_vocabulary"),
            "numbers_count": ready_categories.count("numbers"),
            "silence_control_count": ready_categories.count("silence"),
        },
        "aggregate": aggregate_record,
        "timing": {
            "recognition_p50_ms": percentile(recognition, 0.50),
            "recognition_p95_ms": percentile(recognition, 0.95),
            "stop_to_final_p50_ms": percentile(stop_to_final, 0.50),
            "stop_to_final_p95_ms": percentile(stop_to_final, 0.95),
            "real_time_factor_p50": percentile(rtfs, 0.50),
            "real_time_factor_p95": percentile(rtfs, 0.95),
        },
        "fixtures": sample_results,
        "release_gates": {
            "verified_corpus_coverage": "PASS" if required_coverage_met else "NOT_ASSESSED",
            "wer_at_or_below_12_percent": (
                "NOT_ASSESSED" if not required_coverage_met or wer_value is None
                else "PASS" if wer_value <= 0.12 else "FAIL"
            ),
            "rtf_p95_below_1": (
                "NOT_ASSESSED" if not required_coverage_met or not rtfs
                else "PASS" if percentile(rtfs, 0.95) < 1.0 else "FAIL"
            ),
            "stop_to_final_p95_below_1500_ms": (
                "NOT_ASSESSED" if not required_coverage_met or not stop_to_final
                else "PASS" if percentile(stop_to_final, 0.95) < 1500 else "FAIL"
            ),
            "physical_4gb_gate": "BLOCKED" if not physical_4gb else "NOT_ASSESSED",
            "process_tree_memory": "NOT_MEASURED",
            "installed_application_size": "NOT_MEASURED",
            "redistribution_approval": "NOT_APPROVED",
        },
        "limitations": [
            "Only verified ready fixtures in the frozen manifest were measured.",
            "Raw hypotheses and reference text are not written to the result file.",
            "Cold process load is the first model load in a fresh probe process; OS disk cache state is uncontrolled.",
            "Warm reloads are later model loads in the same probe process and do not establish a cold-start distribution.",
            "The native library and model measurements do not represent a complete Outflow app.",
            "A 4 GB gate requires physical hardware; a host with more RAM cannot certify it.",
        ],
    }
    # Drop all transcript-bearing local collections before returning the result.
    del speech_pairs, fixtures, outputs
    return results


def _write_result(result: dict[str, Any], output: Path) -> None:
    """Write a measured result once; never replace an existing evidence file."""

    try:
        with output.open("x", encoding="utf-8") as destination:
            json.dump(result, destination, indent=2, sort_keys=True, ensure_ascii=False)
            destination.write("\n")
    except FileExistsError as error:
        raise BenchmarkError("result file already exists; choose a new evidence path") from error
    except OSError as error:
        raise BenchmarkError("measured result could not be written") from error


def run_benchmark(arguments: argparse.Namespace) -> dict[str, Any]:
    """Validate provenance, run native inference, score and persist metrics."""

    repository_root = Path(arguments.repository_root).resolve(strict=True)
    manifest, fixtures, manifest_hash = load_verified_fixtures(arguments.fixture_manifest, repository_root)
    source = verify_source(arguments.source_dir)
    asset_root = arguments.asset_root or arguments.model_dir
    model = verify_model_assets(arguments.asset_manifest, asset_root)
    if Path(arguments.model_dir).resolve(strict=True) != Path(asset_root).resolve(strict=True):
        raise BenchmarkError("model directory must match the validated asset root")

    native_runner = Path(arguments.native_runner)
    native_files = {
        "native_runner": native_runner,
        "moonshine_library": Path(arguments.moonshine_library),
        "onnxruntime_library": Path(arguments.onnxruntime_library),
    }
    for native_file in native_files.values():
        _binary_record(native_file)
    load_probe = _run_json_command(
        [str(native_runner), "--load-probe", str(arguments.model_dir), "3"],
        timeout_seconds=arguments.timeout_seconds,
    )
    load_samples = parse_load_probe(load_probe, required_samples=3)
    audio_paths = [str(fixture["audio_path"]) for fixture in fixtures]
    native_json = _run_json_command(
        [str(native_runner), str(arguments.model_dir), *audio_paths],
        timeout_seconds=arguments.timeout_seconds,
    )
    native_output = parse_native_output(native_json, len(fixtures))
    prepared = [dict(fixture) for fixture in fixtures]
    for fixture in prepared:
        fixture["audio_metadata"] = dict(fixture["audio_metadata"])
    machine = machine_record(arguments.machine_label, arguments.cpu_label)
    result = build_measured_result(
        manifest=manifest,
        fixtures=prepared,
        fixture_manifest_sha256=manifest_hash,
        native_output=native_output,
        load_samples_ms=load_samples,
        native_files=native_files,
        source=source,
        model=model,
        machine=machine,
        created_at_utc=datetime.now(UTC).isoformat(timespec="seconds").replace("+00:00", "Z"),
    )
    # Do not retain transcript-bearing native protocol objects after scoring.
    del fixtures, prepared, native_json, native_output, load_probe, manifest
    if arguments.output is not None:
        _write_result(result, arguments.output)
    return result


def _default_binary(source_dir: Path, kind: str) -> Path:
    if kind == "moonshine":
        return source_dir / "core" / "build" / "moonshine-core" / "libmoonshine.dylib"
    if kind == "onnxruntime":
        return source_dir / "core" / "third-party" / "onnxruntime" / "lib" / "macos" / "arm64" / "libonnxruntime.1.23.2.dylib"
    raise ValueError("unknown binary kind")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Measure the pinned local Moonshine model on verified fixtures.")
    parser.add_argument("--fixture-manifest", type=Path, default=REPOSITORY_ROOT / "docs" / "benchmark-fixtures.json")
    parser.add_argument("--asset-manifest", type=Path, default=REPOSITORY_ROOT / "evidence" / "mvp" / "prompt-79-model-assets.json")
    parser.add_argument("--asset-root", type=Path, help="directory containing all eight validated model assets")
    parser.add_argument("--model-dir", type=Path, required=True, help="model directory passed to the native C API")
    parser.add_argument("--source-dir", type=Path, required=True, help="clean pinned Moonshine source checkout")
    parser.add_argument("--native-runner", type=Path, required=True, help="compiled tests/native/moonshine_asr_batch.cpp helper")
    parser.add_argument("--moonshine-library", type=Path, required=True, help="exact libmoonshine binary used by the helper")
    parser.add_argument("--onnxruntime-library", type=Path, required=True, help="exact ONNX Runtime binary used by the helper")
    parser.add_argument("--repository-root", type=Path, default=REPOSITORY_ROOT)
    parser.add_argument("--machine-label", help="non-identifying local machine label")
    parser.add_argument("--cpu-label", help="non-identifying CPU model label")
    parser.add_argument("--timeout-seconds", type=float, default=3600.0)
    parser.add_argument("--output", type=Path, help="new sanitized measured-results JSON path")
    arguments = parser.parse_args(argv)
    if arguments.timeout_seconds <= 0 or not math.isfinite(arguments.timeout_seconds):
        parser.error("--timeout-seconds must be positive and finite")
    try:
        result = run_benchmark(arguments)
    except (BenchmarkError, OSError, ValueError, KeyError, TypeError) as error:
        print(f"native ASR benchmark failed: {error}", file=sys.stderr)
        return 2
    print("MEASURED_NATIVE_ASR_RESULT: READY")
    print(f"Verified ready fixtures: {result['corpus']['verified_ready_fixture_count']}")
    print(f"Small Streaming model bytes: {result['model']['total_asset_bytes']}")
    print(f"Aggregate speech WER: {result['aggregate']['wer']}")
    print(f"Physical 4 GB gate: {result['release_gates']['physical_4gb_gate']}")
    print(f"Verified corpus coverage gate: {result['release_gates']['verified_corpus_coverage']}")
    if arguments.output is not None:
        print("Measured results file: WRITTEN WITHOUT TRANSCRIPT TEXT")
    else:
        print("Measured results file: NOT WRITTEN (supply --output)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
