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
import base64
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
    from .validate_fixture_manifest import REPOSITORY_ROOT, assess_corpus_coverage, validate_manifest
    from .validate_model_asset_inventory import load_manifest as load_asset_manifest
    from .validate_model_asset_inventory import validate_manifest as validate_asset_manifest
else:
    from score_corpus_wer import calculate_corpus_wer
    from score_wer import NORMALIZATION_POLICY_VERSION, calculate_wer
    from validate_fixture_manifest import REPOSITORY_ROOT, assess_corpus_coverage, validate_manifest
    from validate_model_asset_inventory import load_manifest as load_asset_manifest
    from validate_model_asset_inventory import validate_manifest as validate_asset_manifest


RESULT_SCHEMA_VERSION = 3
RESULT_TYPE = "measured_native_asr"
EVALUATION_VERSION = "outflow-native-asr-benchmark-v3"
SOURCE_REVISION = "234f60faa0eb388b01cdf7e60aca232af37aefda"
MODEL_CATALOG_PATH = "small-streaming-en/quantized_26_08_21"
MODEL_CANDIDATES = {
    "small": {
        "identity": "small-streaming-en",
        "macro": "MOONSHINE_MODEL_ARCH_SMALL_STREAMING",
        "architecture": 4,
        "asset_id_prefix": "moonshine-small-streaming-",
    },
    "tiny": {
        "identity": "tiny-streaming-en",
        "macro": "MOONSHINE_MODEL_ARCH_TINY_STREAMING",
        "architecture": 2,
        "asset_id_prefix": "moonshine-tiny-streaming-en-",
    },
}
DEFAULT_CANDIDATE = "small"
EXPECTED_ARCHITECTURE = MODEL_CANDIDATES[DEFAULT_CANDIDATE]["architecture"]
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
# A marketed 4 GB device may report 4 GiB (4,294,967,296) installed bytes.
# This inclusive ceiling is only a memory-size screen. Physical verification,
# an owner-approved reference-class allowlist, CPU-only inference, and measured
# release gates remain independent requirements.
MAX_4GB_REPORTED_MEMORY_BYTES = 4_294_967_296
APPROVED_4GB_REFERENCE_CLASSES: frozenset[str] = frozenset()
SHA256_PATTERN = re.compile(r"[0-9a-fA-F]{64}\Z")
MACHINE_LABEL_PATTERN = re.compile(r"[A-Za-z0-9._-]{1,64}\Z")
SYSTEM_LIBRARY_ROOTS = ("/System/Library/", "/usr/lib/", "/Library/Apple/System/Library/")


class BenchmarkError(RuntimeError):
    """A benchmark input, source, model, native run, or output failed safely."""


class _DuplicateJsonKey(ValueError):
    """Indicates a repeated key without exposing untrusted contents."""

    def __str__(self) -> str:
        return "duplicate JSON object key"


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
                "status": "ready",
                "category": fixture["category"],
                "duration_class": fixture["duration_class"],
                "coverage_attributes": list(fixture["coverage_attributes"]),
                "speaker_variety": fixture.get("speaker_variety"),
                "provenance_verified": True,
                "human_reference_verified": True,
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


def _canonical_file(path: Path) -> Path:
    try:
        resolved = path.resolve(strict=True)
    except (OSError, RuntimeError) as error:
        raise BenchmarkError("a required native binary could not be resolved") from error
    if not resolved.is_file():
        raise BenchmarkError("a required native binary is not a regular file")
    return resolved


def _system_tool_output(command: Sequence[str]) -> str:
    try:
        completed = subprocess.run(
            list(command), check=False, capture_output=True, text=True, encoding="utf-8",
            errors="strict", timeout=10,
        )
    except (OSError, UnicodeError, subprocess.SubprocessError) as error:
        raise BenchmarkError("macOS native-library inspection could not be completed") from error
    if completed.returncode != 0:
        raise BenchmarkError("macOS native-library inspection failed")
    return completed.stdout


def _otool_dependencies(path: Path) -> list[str]:
    output = _system_tool_output(("otool", "-L", str(path)))
    lines = output.splitlines()
    if not lines or not lines[0].rstrip().endswith(":"):
        raise BenchmarkError("otool returned an ambiguous dependency listing")
    dependencies = []
    for line in lines[1:]:
        stripped = line.strip()
        if not stripped:
            continue
        dependencies.append(stripped.split(" (", 1)[0])
    if len(dependencies) != len(set(dependencies)):
        raise BenchmarkError("native dependency listing contains duplicate entries")
    return dependencies


def _is_system_library(path: str) -> bool:
    return path.startswith(SYSTEM_LIBRARY_ROOTS)


def verify_native_runtime(
    native_runner: Path,
    moonshine_library: Path,
    onnxruntime_library: Path,
    *,
    host_architecture: str | None = None,
) -> dict[str, Any]:
    """Verify actual macOS dyld images against the exact benchmark inputs.

    The helper reports the loaded image paths and the resolved image containing
    `moonshine_get_version`; lipo/otool then verify the executable slice and
    declared dependency chain. Caller-supplied paths alone are insufficient.
    """
    if sys.platform != "darwin":
        raise BenchmarkError("actual native runtime identity is supported only on macOS")
    runner = _canonical_file(native_runner)
    moonshine = _canonical_file(moonshine_library)
    onnxruntime = _canonical_file(onnxruntime_library)
    initial_hashes = {
        runner: file_sha256(runner),
        moonshine: file_sha256(moonshine),
        onnxruntime: file_sha256(onnxruntime),
    }
    expected_arch = host_architecture or platform.machine()
    architecture_aliases = {"aarch64": "arm64", "x86_64": "x86_64", "arm64": "arm64"}
    expected_arch = architecture_aliases.get(expected_arch, "")
    if not expected_arch:
        raise BenchmarkError("host architecture is unsupported for native verification")

    probe = _run_json_command((str(runner), "--runtime-identity"), timeout_seconds=10)
    if not isinstance(probe, dict) or set(probe) != {
        "schema_version", "architecture", "pointer_bits", "c_api_version", "runner_path",
        "moonshine_api_path", "loaded_images",
    }:
        raise BenchmarkError("native runtime identity probe has an invalid shape")
    if (
        type(probe["schema_version"]) is not int or probe["schema_version"] != 1
        or probe["architecture"] != expected_arch
        or type(probe["pointer_bits"]) is not int or probe["pointer_bits"] != 64
        or type(probe["c_api_version"]) is not int or probe["c_api_version"] != EXPECTED_HEADER_VERSION
        or not isinstance(probe["loaded_images"], list) or not probe["loaded_images"]
    ):
        raise BenchmarkError("native runtime identity or architecture is unsupported")
    try:
        probe_runner = str(Path(probe["runner_path"]).resolve(strict=True))
        api_path = str(Path(probe["moonshine_api_path"]).resolve(strict=True))
        loaded: list[str] = []
        for value in probe["loaded_images"]:
            if not isinstance(value, str) or not Path(value).is_absolute() or ".." in Path(value).parts:
                raise ValueError
            try:
                loaded.append(str(Path(value).resolve(strict=True)))
            except (OSError, RuntimeError):
                # Apple dyld shared-cache images have logical paths but no
                # standalone file to resolve or hash on current macOS.
                if not _is_system_library(value):
                    raise
                loaded.append(value)
    except (OSError, RuntimeError, TypeError, ValueError) as error:
        raise BenchmarkError("dyld returned an unresolved or malformed image path") from error
    if probe_runner != str(runner) or api_path != str(moonshine):
        raise BenchmarkError("the helper or Moonshine C API resolved to an unexpected binary")
    if len(loaded) != len(set(loaded)):
        raise BenchmarkError("dyld returned ambiguous duplicate loaded-image paths")
    if loaded.count(str(runner)) != 1 or loaded.count(str(moonshine)) != 1 or loaded.count(str(onnxruntime)) != 1:
        raise BenchmarkError("dyld did not load exactly the helper, Moonshine, and ONNX Runtime binaries supplied")
    expected_images = {str(runner), str(moonshine), str(onnxruntime)}
    unexpected = [path for path in loaded if path not in expected_images and not _is_system_library(path)]
    if unexpected:
        raise BenchmarkError("dyld loaded an unapproved non-system library")

    for binary in (runner, moonshine, onnxruntime):
        architectures = set(_system_tool_output(("lipo", "-archs", str(binary))).split())
        if expected_arch not in architectures:
            raise BenchmarkError("a loaded native binary does not contain the active architecture")

    helper_deps = _otool_dependencies(runner)
    moonshine_deps = _otool_dependencies(moonshine)
    helper_moonshine = [name for name in helper_deps if Path(name).name == "libmoonshine.dylib"]
    moonshine_ort = [name for name in moonshine_deps if Path(name).name.startswith("libonnxruntime")]
    if len(helper_moonshine) != 1 or len(moonshine_ort) != 1:
        raise BenchmarkError("native dependency chain is missing or ambiguous")
    if Path(moonshine_ort[0]).name != onnxruntime.name:
        raise BenchmarkError("Moonshine declares an unexpected ONNX Runtime dependency")
    approved_rpath_dependencies = {helper_moonshine[0], moonshine_ort[0]}
    for dependency in helper_deps + moonshine_deps:
        name = Path(dependency).name
        if dependency in approved_rpath_dependencies:
            continue
        if dependency.startswith("/usr/lib/") or dependency.startswith("/System/Library/"):
            continue
        raise BenchmarkError("otool found an unapproved native dependency")

    final_hashes = {path: file_sha256(path) for path in initial_hashes}
    if final_hashes != initial_hashes:
        raise BenchmarkError("a native binary changed during runtime identity verification")
    return {
        "architecture": expected_arch,
        "c_api_version": EXPECTED_HEADER_VERSION,
        "loaded_image_count": len(loaded),
        "unexpected_non_system_library_count": 0,
        "resolved_paths_match_inputs": True,
        "dependency_chain_verified": True,
        "native_runner_sha256": final_hashes[runner],
        "moonshine_sha256": final_hashes[moonshine],
        "onnxruntime_sha256": final_hashes[onnxruntime],
    }


def derive_source_model_contract(source_dir: Path, candidate: str) -> dict[str, Any]:
    """Read the selected English streaming file contract from pinned source."""

    selected = MODEL_CANDIDATES.get(candidate)
    if selected is None:
        raise BenchmarkError("unsupported model candidate")
    try:
        header = (source_dir / "core" / "moonshine-c-api.h").read_text(encoding="utf-8")
        catalog = (source_dir / "core" / "moonshine-model-catalog.cpp").read_text(encoding="utf-8")
        metadata = (source_dir / "core" / "moonshine-model-file-metadata.generated.cpp").read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise BenchmarkError("pinned source model catalog could not be read") from error

    architecture_match = re.search(
        rf"#define\s+{re.escape(selected['macro'])}\s+\(\s*(\d+)\s*\)", header
    )
    header_match = re.search(
        r"#define\s+MOONSHINE_HEADER_VERSION\s+\(\s*(\d+)\s*\)", header
    )
    quantized_match = re.search(
        r'constexpr const char\*\s+kStreamingQuantizedDir\s*=\s*"([^"]+)"\s*;', catalog
    )
    if (
        architecture_match is None
        or int(architecture_match.group(1)) != selected["architecture"]
        or header_match is None
        or int(header_match.group(1)) != EXPECTED_HEADER_VERSION
        or quantized_match is None
    ):
        raise BenchmarkError("pinned C API candidate contract is unsupported")

    catalog_path = f"{selected['identity']}{quantized_match.group(1)}"
    base_url = f"https://download.moonshine.ai/model/{catalog_path}"
    catalog_binding = re.search(
        rf"\{{\s*{re.escape(selected['macro'])}\s*,\s*std::string\(kCdnModelBase\)\s*\+\s*"
        rf'"/{re.escape(selected["identity"])}"\s*\+\s*kStreamingQuantizedDir\s*\}}',
        catalog,
    )
    if catalog_binding is None:
        raise BenchmarkError("candidate is not present in the pinned English model catalog")

    component_files = re.search(
        r"std::vector<std::string>\s+stt_component_files\s*\([^)]*\)\s*\{.*?"
        r"std::vector<std::string>\s+files\s*=\s*\{(?P<files>.*?)\};",
        catalog,
        flags=re.DOTALL,
    )
    if component_files is None:
        raise BenchmarkError("pinned streaming model file contract could not be parsed")
    required_names = re.findall(r'"([^"\\]+)"', component_files.group("files"))
    if not required_names or len(required_names) != len(set(required_names)):
        raise BenchmarkError("pinned streaming model file contract is ambiguous")

    entry_pattern = re.compile(
        r"\{\s*(?P<url_expr>(?:\"(?:[^\"\\]|\\.)*\"\s*)+),\s*"
        r"(?P<size>-?\d+)\s*,\s*\"(?P<checksum>[^\"]*)\"\s*,\s*"
        r"\"(?P<checksum_type>[^\"]*)\"\s*\}",
        flags=re.DOTALL,
    )
    metadata_entries: dict[str, dict[str, Any]] = {}
    for entry in entry_pattern.finditer(metadata):
        url = "".join(re.findall(r'"((?:[^"\\]|\\.)*)"', entry.group("url_expr")))
        if not url.startswith(base_url + "/"):
            continue
        name = url.rsplit("/", 1)[-1]
        if name in metadata_entries:
            raise BenchmarkError("pinned model metadata contains duplicate file identities")
        metadata_entries[name] = {
            "url": url,
            "expected_bytes": int(entry.group("size")),
            "vendor_checksum": entry.group("checksum"),
            "vendor_checksum_type": entry.group("checksum_type"),
        }
    files: dict[str, dict[str, Any]] = {}
    for name in required_names:
        record = metadata_entries.get(name)
        if (
            record is None
            or record["expected_bytes"] <= 0
            or record["vendor_checksum_type"] != "crc32c"
            or not record["vendor_checksum"]
        ):
            raise BenchmarkError("pinned source metadata is incomplete for a required model file")
        files[name] = record

    contract = {
        "source_revision": SOURCE_REVISION,
        "candidate": candidate,
        "model_identity": selected["identity"],
        "architecture": selected["architecture"],
        "c_api_header_version": EXPECTED_HEADER_VERSION,
        "catalog_path": catalog_path,
        "required_files": files,
    }
    contract["source_contract_sha256"] = hashlib.sha256(
        json.dumps(contract, sort_keys=True, separators=(",", ":")).encode("utf-8")
    ).hexdigest()
    return contract


def verify_source(source_dir: Path, candidate: str = "small") -> dict[str, Any]:
    """Verify the exact clean upstream source and selected C API contract."""

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
    except (OSError, subprocess.SubprocessError, UnicodeError) as error:
        raise BenchmarkError("pinned Moonshine source or C API header could not be verified") from error
    if revision != SOURCE_REVISION or status.strip():
        raise BenchmarkError("Moonshine source is not the required clean pinned revision")
    contract = derive_source_model_contract(source_dir, candidate)
    return {
        "source_revision": revision,
        "c_api_header_version": EXPECTED_HEADER_VERSION,
        "model_architecture": contract["architecture"],
        "candidate": candidate,
        "source_contract_sha256": contract["source_contract_sha256"],
    }


def _crc32c_base64(path: Path) -> str:
    """Compute the pinned source registry's CRC32C check without networking."""

    table = []
    for value in range(256):
        crc_value = value
        for _ in range(8):
            crc_value = (crc_value >> 1) ^ (0x82F63B78 if crc_value & 1 else 0)
        table.append(crc_value)
    crc = 0xFFFFFFFF
    try:
        with path.open("rb") as source:
            for chunk in iter(lambda: source.read(1024 * 1024), b""):
                for byte in chunk:
                    crc = table[(crc ^ byte) & 0xFF] ^ (crc >> 8)
    except OSError as error:
        raise BenchmarkError("a candidate model artifact could not be read") from error
    checksum = (crc ^ 0xFFFFFFFF).to_bytes(4, "big")
    return base64.b64encode(checksum).decode("ascii")


def _candidate_manifest_assets(contract: Mapping[str, Any], asset_root: Path) -> list[dict[str, Any]]:
    """Create a local-only inventory from bytes that match pinned source metadata."""

    try:
        asset_root.lstat()
    except OSError as error:
        raise BenchmarkError("candidate artifact directory is missing") from error
    if not asset_root.is_dir() or asset_root.is_symlink():
        raise BenchmarkError("candidate artifact root must be a real directory")
    try:
        entries = list(asset_root.iterdir())
    except OSError as error:
        raise BenchmarkError("candidate artifact directory could not be inspected") from error
    if len(entries) != len(contract["required_files"]):
        raise BenchmarkError("candidate artifact directory is incomplete or contains unsupported files")

    assets = []
    for name, source_record in sorted(contract["required_files"].items()):
        path = asset_root / name
        try:
            info = path.lstat()
        except OSError as error:
            raise BenchmarkError("candidate model is blocked: a required artifact is missing") from error
        if not path.is_file() or path.is_symlink() or info.st_size != source_record["expected_bytes"]:
            raise BenchmarkError("candidate model is blocked: artifact type or source byte size does not match")
        if _crc32c_base64(path) != source_record["vendor_checksum"]:
            raise BenchmarkError("candidate model is blocked: local bytes do not match the pinned source checksum")
        identity = contract["model_identity"]
        provenance = (
            f"Pinned Moonshine source {contract['source_revision']}; "
            f"contract SHA-256 {contract['source_contract_sha256']}; "
            f"source expected bytes {source_record['expected_bytes']}; "
            f"vendor CRC32C {source_record['vendor_checksum']}; "
            f"locally observed SHA-256 {file_sha256(path)}."
        )
        assets.append(
            {
                "asset_id": f"moonshine-{identity}-{name.replace('.', '-')}",
                "category": "tokenizer" if name == "tokenizer.bin" else "model",
                "status": "file-verified",
                "source": "Moonshine AI",
                "source_url": source_record["url"],
                "source_revision": None,
                "file_path": name,
                "sha256": file_sha256(path),
                "provenance_reference": provenance,
                "license_identifier": None,
                "license_evidence_reference": None,
                "license_review_status": "review-pending",
                "redistribution_status": "undetermined",
                "redistribution_evidence_reference": None,
                "notice_attribution_status": "undetermined",
                "notice_attribution_evidence_reference": None,
            }
        )
    return assets


def prepare_candidate_asset_manifest(
    source_dir: Path, candidate: str, asset_root: Path, output_path: Path
) -> dict[str, Any]:
    """Write an exclusive local artifact inventory; never author an approval."""

    contract = derive_source_model_contract(source_dir, candidate)
    assets = _candidate_manifest_assets(contract, asset_root)
    document = {
        "schema_version": 1,
        "candidate": candidate,
        "model_identity": contract["model_identity"],
        "model_architecture": contract["architecture"],
        "source_revision": contract["source_revision"],
        "source_contract_sha256": contract["source_contract_sha256"],
        "acquisition_approval_status": "not-recorded",
        "acquisition_approval_reference": None,
        "assets": assets,
    }
    try:
        with output_path.open("x", encoding="utf-8") as output:
            json.dump(document, output, indent=2, sort_keys=True, ensure_ascii=False)
            output.write("\n")
    except FileExistsError as error:
        raise BenchmarkError("manifest output already exists; choose a new path") from error
    except OSError as error:
        raise BenchmarkError("candidate manifest could not be written") from error
    return document


def _validate_tiny_approval(
    document: Mapping[str, Any], approval_reference: str | None, repository_root: Path
) -> None:
    if (
        document.get("acquisition_approval_status") != "approved"
        or not isinstance(approval_reference, str)
        or not approval_reference
        or document.get("acquisition_approval_reference") != approval_reference
    ):
        raise BenchmarkError("BLOCKED: Tiny evaluation acquisition approval is not recorded")
    reference_path = Path(approval_reference)
    if reference_path.is_absolute() or ".." in reference_path.parts:
        raise BenchmarkError("Tiny approval reference must be a repository-relative evidence path")
    try:
        approval_file = (repository_root / reference_path).resolve(strict=True)
        approval_file.relative_to(repository_root.resolve(strict=True))
        approval_text = approval_file.read_text(encoding="utf-8")
    except (OSError, UnicodeError, ValueError) as error:
        raise BenchmarkError("BLOCKED: Tiny acquisition approval evidence could not be verified") from error
    if (
        "TINY EVALUATION ACQUISITION APPROVED" not in approval_text
        or SOURCE_REVISION not in approval_text
        or "tiny-streaming-en" not in approval_text
    ):
        raise BenchmarkError("BLOCKED: Tiny acquisition approval evidence does not authorize this source candidate")


def verify_model_assets(
    manifest_path: Path,
    asset_root: Path,
    *,
    candidate: str = "small",
    contract: Mapping[str, Any] | None = None,
    approval_reference: str | None = None,
    repository_root: Path = REPOSITORY_ROOT,
) -> dict[str, Any]:
    """Validate exact local files against source-derived candidate metadata."""

    selected_candidate = MODEL_CANDIDATES.get(candidate)
    if selected_candidate is None:
        raise BenchmarkError("unsupported model candidate")
    source_contract = contract
    strict_source_contract = source_contract is not None
    if source_contract is None and candidate == "small":
        source_contract = {
            "source_revision": SOURCE_REVISION,
            "candidate": "small",
            "model_identity": "small-streaming-en",
            "architecture": MODEL_CANDIDATES["small"]["architecture"],
            "c_api_header_version": EXPECTED_HEADER_VERSION,
            "catalog_path": MODEL_CATALOG_PATH,
            "source_contract_sha256": None,
            "required_files": {
                name: {
                    "url": f"https://download.moonshine.ai/model/{MODEL_CATALOG_PATH}/{name}",
                    "expected_bytes": None,
                    "vendor_checksum": None,
                    "vendor_checksum_type": None,
                }
                for name in REQUIRED_MODEL_ASSETS
            },
        }
    if source_contract is None:
        raise BenchmarkError("candidate source contract is required before model verification")
    if candidate == "small":
        asset_manifest = load_asset_manifest(manifest_path)
    else:
        asset_manifest = load_json(manifest_path)
        expected_root = {
            "schema_version", "candidate", "model_identity", "model_architecture",
            "source_revision", "source_contract_sha256", "acquisition_approval_status",
            "acquisition_approval_reference", "assets",
        }
        if not isinstance(asset_manifest, dict) or set(asset_manifest) != expected_root:
            raise BenchmarkError("Tiny candidate manifest metadata is incomplete")
        if (
            type(asset_manifest["schema_version"]) is not int
            or asset_manifest["schema_version"] != 1
            or asset_manifest["candidate"] != "tiny"
            or asset_manifest["model_identity"] != selected_candidate["identity"]
            or type(asset_manifest["model_architecture"]) is not int
            or asset_manifest["model_architecture"] != selected_candidate["architecture"]
            or asset_manifest["source_revision"] != SOURCE_REVISION
            or asset_manifest["source_contract_sha256"] != source_contract["source_contract_sha256"]
        ):
            raise BenchmarkError("Tiny candidate model identity does not match the pinned source contract")
        _validate_tiny_approval(asset_manifest, approval_reference, repository_root)
        asset_manifest = {"schema_version": 1, "assets": asset_manifest["assets"]}

    errors, _counts = validate_asset_manifest(asset_manifest, asset_root)
    if errors:
        raise BenchmarkError(f"model asset inventory validation failed ({len(errors)} issue(s))")
    selected = [
        asset for asset in asset_manifest["assets"]
        if isinstance(asset, dict) and asset.get("asset_id", "").startswith(
            selected_candidate["asset_id_prefix"]
        )
    ]
    required_files = source_contract["required_files"]
    names = {asset["file_path"] for asset in selected if asset["file_path"] is not None}
    if names != set(required_files) or len(selected) != len(required_files):
        raise BenchmarkError(f"{candidate.title()} Streaming inventory does not match its pinned source file contract")
    for asset in selected:
        name = asset["file_path"]
        expected_source = required_files[name]
        if (
            asset["status"] != "file-verified"
            or asset["source_revision"] is not None
            or not _is_sha256(asset["sha256"])
            or asset["source_url"] != expected_source["url"]
            or (
                strict_source_contract
                and candidate == "tiny"
                and SOURCE_REVISION not in asset["provenance_reference"]
            )
            or (
                strict_source_contract
                and candidate == "tiny"
                and source_contract["source_contract_sha256"] not in asset["provenance_reference"]
            )
            or (
                strict_source_contract
                and candidate == "tiny"
                and expected_source["vendor_checksum"] not in asset["provenance_reference"]
            )
        ):
            raise BenchmarkError(f"{candidate.title()} artifact identity or provenance is incomplete")
        path = asset_root / name
        if expected_source["expected_bytes"] is not None and path.stat().st_size != expected_source["expected_bytes"]:
            raise BenchmarkError(f"{candidate.title()} artifact byte size differs from the pinned source catalog")
        if strict_source_contract and _crc32c_base64(path) != expected_source["vendor_checksum"]:
            raise BenchmarkError(f"{candidate.title()} artifact does not match the pinned source CRC32C")

    ordered = sorted(selected, key=lambda item: item["file_path"])
    return {
        "candidate": candidate,
        "model_identity": selected_candidate["identity"],
        "catalog_path": source_contract["catalog_path"],
        "source_contract_sha256": source_contract["source_contract_sha256"],
        "architecture": selected_candidate["architecture"],
        "immutable_artifact_revision": None,
        "asset_sha256": {asset["file_path"]: asset["sha256"].lower() for asset in ordered},
        "asset_bytes": {asset["file_path"]: (asset_root / asset["file_path"]).stat().st_size for asset in ordered},
        "total_bytes": sum((asset_root / asset["file_path"]).stat().st_size for asset in ordered),
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


def _validate_candidate_architecture(architecture: int) -> None:
    if type(architecture) is not int or architecture not in {
        candidate["architecture"] for candidate in MODEL_CANDIDATES.values()
    }:
        raise BenchmarkError("unsupported development model architecture")


def _validate_candidate_architecture(architecture: int) -> None:
    if type(architecture) is not int or architecture not in {
        candidate["architecture"] for candidate in MODEL_CANDIDATES.values()
    }:
        raise BenchmarkError("unsupported development model architecture")


def parse_load_probe(
    value: Any,
    *,
    required_samples: int = 2,
    expected_architecture: int = EXPECTED_ARCHITECTURE,
) -> list[float]:
    """Validate cold-process and warm-reload timing samples from the helper."""

    _validate_candidate_architecture(expected_architecture)
    if not isinstance(value, dict) or set(value) != {
        "schema_version", "execution_provider", "architecture", "header_version", "model_load_ms"
    }:
        raise BenchmarkError("native model-load probe has an invalid shape")
    samples = value["model_load_ms"]
    if (
        type(value["schema_version"]) is not int or value["schema_version"] != 1
        or value["execution_provider"] != "CPUExecutionProvider"
        or type(value["architecture"]) is not int
        or value["architecture"] != expected_architecture
        or type(value["header_version"]) is not int
        or value["header_version"] != EXPECTED_HEADER_VERSION
        or not isinstance(samples, list) or len(samples) < required_samples
    ):
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


def parse_native_output(
    value: Any,
    expected_count: int,
    *,
    expected_architecture: int = EXPECTED_ARCHITECTURE,
) -> tuple[float, list[dict[str, Any]]]:
    """Validate batch inference output while keeping hypotheses in memory only."""

    _validate_candidate_architecture(expected_architecture)
    required_root = {"schema_version", "execution_provider", "architecture", "header_version", "model_load_ms", "outputs"}
    if not isinstance(value, dict) or set(value) != required_root:
        raise BenchmarkError("native inference output has an invalid shape")
    if (
        type(value["schema_version"]) is not int
        or value["schema_version"] != 1
        or value["execution_provider"] != "CPUExecutionProvider"
        or value["architecture"] != expected_architecture
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
    if sys.platform == "darwin":
        try:
            output = subprocess.run(
                ["sysctl", "-n", "hw.memsize"], check=True, capture_output=True,
                text=True, encoding="ascii", timeout=5,
            ).stdout.strip()
            value = int(output)
            return value if value > 0 else None
        except (OSError, UnicodeError, subprocess.SubprocessError, ValueError):
            return None
    try:
        pages = os.sysconf("SC_PHYS_PAGES")
        page_size = os.sysconf("SC_PAGE_SIZE")
        if type(pages) is int and type(page_size) is int and pages > 0 and page_size > 0:
            return pages * page_size
    except (OSError, ValueError):
        return None
    return None


def _machine_identifier() -> str | None:
    """Return a non-unique hardware model identifier, never a serial number."""
    if sys.platform != "darwin":
        return None
    try:
        value = subprocess.run(
            ["sysctl", "-n", "hw.model"], check=True, capture_output=True,
            text=True, encoding="ascii", timeout=5,
        ).stdout.strip()
    except (OSError, UnicodeError, subprocess.SubprocessError):
        return None
    if not value or len(value) > 100 or not re.fullmatch(r"[A-Za-z0-9,._-]+", value):
        return None
    return value


def machine_record(
    machine_label: str | None = None,
    cpu_label: str | None = None,
    *,
    cpu_only_inference_exercised: bool = False,
) -> dict[str, Any]:
    """Return host facts without hostname, serial number, username, or paths."""

    if machine_label is not None and not MACHINE_LABEL_PATTERN.fullmatch(machine_label):
        raise BenchmarkError("machine label must contain only simple non-identifying characters")
    safe_cpu = cpu_label or platform.processor() or None
    if safe_cpu is not None and (len(safe_cpu) > 100 or any(ord(char) < 32 for char in safe_cpu)):
        raise BenchmarkError("CPU label is invalid")
    physical_memory = _physical_memory_bytes()
    return {
        "machine_label": machine_label,
        "machine_identifier": _machine_identifier(),
        "os": platform.system(),
        "os_release": platform.release(),
        "architecture": platform.machine(),
        "physical_memory_bytes": physical_memory,
        "physical_memory_source": "sysctl_hw.memsize" if sys.platform == "darwin" and physical_memory is not None else "sysconf_pages" if physical_memory is not None else None,
        "physical_hardware_status": "unknown",
        "reference_hardware_class": None,
        "cpu_only_inference_exercised": cpu_only_inference_exercised,
        # A successful native run proves that a measurement happened, but this
        # field is reserved for runs whose physical-hardware status was
        # independently verified. The harness does not certify that status.
        "real_hardware_measurements_completed": False,
        "logical_cpu_count": os.cpu_count(),
        "cpu_label": safe_cpu,
    }


def assess_physical_4gb_eligibility(machine: Mapping[str, Any]) -> str:
    """Classify conservative physical 4 GB reference-machine eligibility.

    The PRD says 4 GB but does not define decimal versus binary reporting.
    Accepting up to exactly 4 GiB avoids rejecting a marketed 4 GB device solely
    because the OS reports binary bytes. This does not approve a device class:
    the caller still needs verified physical status and an approved class.
    """
    ram = machine.get("physical_memory_bytes")
    status = machine.get("physical_hardware_status")
    reference_class = machine.get("reference_hardware_class")
    if status == "virtual_or_simulated" or (type(ram) is int and ram > MAX_4GB_REPORTED_MEMORY_BYTES):
        return "INELIGIBLE"
    if (
        type(ram) is int
        and ram <= MAX_4GB_REPORTED_MEMORY_BYTES
        and status == "verified_physical"
        and isinstance(reference_class, str)
        and reference_class in APPROVED_4GB_REFERENCE_CLASSES
    ):
        return "ELIGIBLE"
    return "UNKNOWN"


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
    native_runtime_identity: Mapping[str, Any],
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
                "duration_class": fixture["duration_class"],
                "coverage_attributes": list(fixture["coverage_attributes"]),
                "speaker_variety": fixture.get("speaker_variety"),
                "provenance_verified": fixture.get("provenance_verified") is True,
                "human_reference_verified": fixture.get("human_reference_verified") is True,
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
    coverage = assess_corpus_coverage(list(fixtures))
    coverage["collection_status"] = manifest["collection_status"]
    required_coverage_met = coverage["complete"]
    wer_value = aggregate_record["wer"]
    physical_4gb_eligibility = assess_physical_4gb_eligibility(machine)
    physical_4gb_ineligible = physical_4gb_eligibility == "INELIGIBLE"
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
            "candidate": model["candidate"],
            "model_identity": model["model_identity"],
            "catalog_path": model["catalog_path"],
            "immutable_artifact_revision": model["immutable_artifact_revision"],
            "architecture": model["architecture"],
            "source_contract_sha256": model["source_contract_sha256"],
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
        "native_runtime_identity": dict(native_runtime_identity),
        "machine": dict(machine),
        "corpus": coverage,
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
            "physical_4gb_eligibility": physical_4gb_eligibility,
            "physical_4gb_gate": "BLOCKED" if physical_4gb_ineligible else "NOT_ASSESSED",
            "cpu_only_inference": "PASS" if machine.get("cpu_only_inference_exercised") is True else "NOT_ASSESSED",
            "real_hardware_measurements": "PASS" if machine.get("real_hardware_measurements_completed") is True else "NOT_ASSESSED",
            "release_performance_requirements": "NOT_ASSESSED",
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
            "Physical hardware and approved reference-class status are not inferred from a system RAM query or caller-supplied label.",
            "CPU-only status is based on a successful native run explicitly configured with CPUExecutionProvider.",
        ],
    }
    # Drop all transcript-bearing local collections before returning the result.
    del speech_pairs, fixtures, outputs
    return results


def _write_result(result: dict[str, Any], output: Path) -> None:
    """Write a measured result once; never replace an existing evidence file."""

    if __package__:
        from .validate_measured_asr_results import validate_measured_results
    else:
        from validate_measured_asr_results import validate_measured_results
    validate_measured_results(result)
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
    candidate = getattr(arguments, "candidate", DEFAULT_CANDIDATE)
    if candidate not in MODEL_CANDIDATES:
        raise BenchmarkError("unsupported model candidate")
    if candidate == "tiny" and getattr(arguments, "asset_manifest", None) is None:
        raise BenchmarkError("BLOCKED: Tiny requires an explicit verified candidate manifest")
    source = verify_source(arguments.source_dir, candidate)
    contract = derive_source_model_contract(arguments.source_dir, candidate)
    asset_root = arguments.asset_root or arguments.model_dir
    model = verify_model_assets(
        arguments.asset_manifest,
        asset_root,
        candidate=candidate,
        contract=contract,
        approval_reference=getattr(arguments, "acquisition_approval_reference", None),
        repository_root=repository_root,
    )
    manifest, fixtures, manifest_hash = load_verified_fixtures(arguments.fixture_manifest, repository_root)
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
    native_runtime_identity = verify_native_runtime(
        native_runner,
        native_files["moonshine_library"],
        native_files["onnxruntime_library"],
    )
    load_probe = _run_json_command(
        [str(native_runner), "--load-probe", str(arguments.model_dir), "3", str(model["architecture"])],
        timeout_seconds=arguments.timeout_seconds,
    )
    load_samples = parse_load_probe(
        load_probe, required_samples=3, expected_architecture=model["architecture"]
    )
    audio_paths = [str(fixture["audio_path"]) for fixture in fixtures]
    native_json = _run_json_command(
        [
            str(native_runner), "--model-arch", str(model["architecture"]),
            str(arguments.model_dir), *audio_paths,
        ],
        timeout_seconds=arguments.timeout_seconds,
    )
    native_output = parse_native_output(
        native_json, len(fixtures), expected_architecture=model["architecture"]
    )
    prepared = [dict(fixture) for fixture in fixtures]
    for fixture in prepared:
        fixture["audio_metadata"] = dict(fixture["audio_metadata"])
    machine = machine_record(
        arguments.machine_label,
        arguments.cpu_label,
        cpu_only_inference_exercised=True,
    )
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
        native_runtime_identity=native_runtime_identity,
        created_at_utc=datetime.now(UTC).isoformat(timespec="seconds").replace("+00:00", "Z"),
    )
    # Do not retain transcript-bearing native protocol objects after scoring.
    del fixtures, prepared, native_json, native_output, load_probe, manifest
    if __package__:
        from .validate_measured_asr_results import validate_measured_results
    else:
        from validate_measured_asr_results import validate_measured_results
    validate_measured_results(result)
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
    parser.add_argument("--candidate", choices=tuple(MODEL_CANDIDATES), default=DEFAULT_CANDIDATE, help="development-only candidate; Tiny must be selected explicitly")
    parser.add_argument("--asset-manifest", type=Path, help="Small inventory by default; required when candidate is Tiny")
    parser.add_argument("--acquisition-approval-reference", help="repository evidence path for maintainer-approved Tiny evaluation")
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
    if arguments.candidate == "small" and arguments.asset_manifest is None:
        arguments.asset_manifest = REPOSITORY_ROOT / "evidence" / "mvp" / "prompt-79-model-assets.json"
    if arguments.timeout_seconds <= 0 or not math.isfinite(arguments.timeout_seconds):
        parser.error("--timeout-seconds must be positive and finite")
    try:
        result = run_benchmark(arguments)
    except (BenchmarkError, OSError, ValueError, KeyError, TypeError) as error:
        print(f"native ASR benchmark failed: {error}", file=sys.stderr)
        return 2
    print("MEASURED_NATIVE_ASR_RESULT: READY")
    print(f"Verified ready fixtures: {result['corpus']['verified_ready_fixture_count']}")
    print(f"Candidate: {result['model']['candidate']} ({result['model']['model_identity']}, architecture {result['model']['architecture']})")
    print(f"Model bytes: {result['model']['total_asset_bytes']}")
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
