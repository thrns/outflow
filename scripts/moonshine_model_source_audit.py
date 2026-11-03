#!/usr/bin/env python3
"""Audit the pinned Moonshine model-selection contract using local source only.

This source audit does not invoke Moonshine, fetch a manifest, download model
files, or write to the inspected checkout. A verified source contract does not
verify that the referenced model artifacts were acquired or are redistributable.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
from typing import Sequence


PINNED_REVISION = "234f60faa0eb388b01cdf7e60aca232af37aefda"
PINNED_ORIGIN = "https://github.com/moonshine-ai/moonshine.git"
HEADER = Path("core/moonshine-c-api.h")
C_API = Path("core/moonshine-c-api.cpp")
CATALOG = Path("core/moonshine-model-catalog.cpp")
CATALOG_HEADER = Path("core/moonshine-model-catalog.h")
METADATA_HEADER = Path("core/moonshine-model-file-metadata.h")
METADATA_CPP = Path("core/moonshine-model-file-metadata.generated.cpp")
LICENSE_DOC = Path("docs/license.md")
MODEL_DOC = Path("docs/models/available-models.md")
COMMAND_TIMEOUT_SECONDS = 10


@dataclass(frozen=True)
class ModelFileMetadata:
    """Vendor registry values for one URL; they do not verify a local file."""

    name: str
    size: int | None
    checksum: str | None
    checksum_type: str | None


def _run_git(source: Path, *arguments: str) -> tuple[int, str]:
    """Run a read-only Git metadata query; no fetch or checkout is performed."""
    try:
        result = subprocess.run(
            ["git", "-C", str(source), *arguments],
            capture_output=True,
            text=True,
            check=False,
            timeout=COMMAND_TIMEOUT_SECONDS,
        )
    except (OSError, subprocess.TimeoutExpired):
        return 1, ""
    return result.returncode, result.stdout.strip()


def _read(source: Path, relative: Path) -> str | None:
    try:
        return (source / relative).read_text(encoding="utf-8")
    except (OSError, UnicodeError):
        return None


def _define_value(text: str, name: str) -> int | None:
    match = re.search(
        rf"(?m)^\s*#\s*define\s+{re.escape(name)}\s+\(?\s*(\d+)\s*\)?",
        text,
    )
    return int(match.group(1)) if match else None


def _metadata_entries(text: str, url_prefix: str) -> dict[str, ModelFileMetadata]:
    """Parse the generated registry's bounded C++ aggregate-entry format."""
    entries: dict[str, ModelFileMetadata] = {}
    for match in re.finditer(r"\{([^{}]+)\}", text):
        body = match.group(1)
        quoted = list(re.finditer(r'"((?:[^"\\]|\\.)*)"', body))
        if len(quoted) < 3:
            continue
        url_parts = [item.group(1) for item in quoted[:-2]]
        url = "".join(url_parts)
        if not url.startswith(url_prefix):
            continue
        checksum = quoted[-2].group(1)
        checksum_type = quoted[-1].group(1)
        between_url_and_checksum = body[quoted[-3].end() : quoted[-2].start()]
        size_match = re.search(r",\s*(\d+)\s*,\s*$", between_url_and_checksum)
        name = url.rsplit("/", 1)[-1]
        entries[name] = ModelFileMetadata(
            name=name,
            size=int(size_match.group(1)) if size_match else None,
            checksum=checksum or None,
            checksum_type=checksum_type or None,
        )
    return entries


def _streaming_file_names(catalog: str) -> tuple[list[str], str | None]:
    match = re.search(
        r"if\s*\(is_streaming_arch\(model_arch\)\)\s*\{\s*"
        r"std::vector<std::string>\s+files\s*=\s*\{(?P<files>.*?)\};"
        r"(?P<after>.*?)return files;",
        catalog,
        re.DOTALL,
    )
    if match is None:
        return [], None
    required = re.findall(r'"([^"\n]+)"', match.group("files"))
    optional_match = re.search(
        r"include_word_timestamps\)\s*\{\s*files\.push_back\(\"([^\"]+)\"\)",
        match.group("after"),
    )
    return required, optional_match.group(1) if optional_match else None


def _english_catalog_block(catalog: str) -> str | None:
    match = re.search(
        r'\{"en"\s*,\s*"English"\s*,\s*\{(?P<body>.*?)\n\s*\}\s*\},',
        catalog,
        re.DOTALL,
    )
    return match.group("body") if match else None


def _check_template(source: Path, reason: str) -> dict[str, object]:
    return {
        "source_revision": "Mismatch",
        "header": "Missing",
        "architecture": "Unresolved",
        "loader": "Unresolved",
        "dependencies": "Unresolved",
        "artifact_list": "Not verified",
        "artifact_revision": "Not verified",
        "artifact_sizes": "Not verified",
        "artifact_sha256": "Not verified",
        "model_license": "Incomplete",
        "runtime_notices": "Incomplete",
        "acquisition": "Blocked",
        "source_root": str(source),
        "git_revision": "unavailable",
        "git_origin": "unavailable",
        "header_version": None,
        "architecture_name": None,
        "architecture_value": None,
        "english_default": "unresolved",
        "related_architectures": [],
        "quantized_path": "unresolved",
        "model_base_url": "unresolved",
        "required_files": [],
        "optional_files": [],
        "file_metadata": {},
        "evidence": [],
        "reason_codes": [reason],
        "source_contract_verified": False,
        "model_artifacts_verified": False,
        "exit_code": 2,
    }


def audit_source(source_dir: str | Path) -> dict[str, object]:
    """Inspect a clean pinned checkout and return a stable, read-only report."""
    source = Path(source_dir).expanduser()
    if not source.is_dir():
        return _check_template(source, "SOURCE_DIRECTORY_MISSING")

    revision_code, revision = _run_git(source, "rev-parse", "HEAD")
    status_code, status = _run_git(source, "status", "--short", "--untracked-files=all")
    origin_code, origin = _run_git(source, "remote", "get-url", "origin")
    source_ok = (
        revision_code == 0
        and revision == PINNED_REVISION
        and status_code == 0
        and not status
        and origin_code == 0
        and origin == PINNED_ORIGIN
    )
    if not source_ok:
        reason = "SOURCE_REVISION_OR_IDENTITY_MISMATCH"
        report = _check_template(source, reason)
        report.update(
            {
                "git_revision": revision or "unavailable",
                "git_origin": origin or "unavailable",
                "source_revision": "Verified" if revision == PINNED_REVISION else "Mismatch",
            }
        )
        return report

    header = _read(source, HEADER)
    api = _read(source, C_API)
    catalog = _read(source, CATALOG)
    catalog_header = _read(source, CATALOG_HEADER)
    metadata_header = _read(source, METADATA_HEADER)
    metadata_cpp = _read(source, METADATA_CPP)
    license_doc = _read(source, LICENSE_DOC)
    model_doc = _read(source, MODEL_DOC)
    report = _check_template(source, "SOURCE_CONTRACT_INCOMPLETE")
    report.update(
        {
            "source_revision": "Verified",
            "git_revision": revision,
            "git_origin": origin,
            "reason_codes": [],
            "exit_code": 0,
        }
    )

    if header is None:
        report["header"] = "Missing"
        report["reason_codes"].append("C_API_HEADER_MISSING")
        report["exit_code"] = 1
        return _finalize(report)

    header_version = _define_value(header, "MOONSHINE_HEADER_VERSION")
    report["header_version"] = header_version
    report["header"] = "Verified" if header_version == 30000 else "Missing"
    if header_version != 30000:
        report["reason_codes"].append("C_API_HEADER_VERSION_UNRESOLVED")

    arch_value = _define_value(header, "MOONSHINE_MODEL_ARCH_SMALL_STREAMING")
    base_value = _define_value(header, "MOONSHINE_MODEL_ARCH_BASE")
    base_streaming_value = _define_value(header, "MOONSHINE_MODEL_ARCH_BASE_STREAMING")
    tiny_value = _define_value(header, "MOONSHINE_MODEL_ARCH_TINY")
    tiny_streaming_value = _define_value(header, "MOONSHINE_MODEL_ARCH_TINY_STREAMING")
    report["related_architectures"] = [
        f"TINY={tiny_value}",
        f"BASE={base_value}",
        f"TINY_STREAMING={tiny_streaming_value}",
        f"BASE_STREAMING={base_streaming_value}",
        f"SMALL_STREAMING={arch_value}",
        "SMALL (non-streaming)=not defined",
    ]
    report["architecture_name"] = "MOONSHINE_MODEL_ARCH_SMALL_STREAMING"
    report["architecture_value"] = arch_value
    english_block = _english_catalog_block(catalog or "")
    arch_supported = (
        arch_value == 4
        and base_value == 1
        and base_streaming_value == 3
        and english_block is not None
        and "MOONSHINE_MODEL_ARCH_SMALL_STREAMING" in english_block
        and "/small-streaming-en" in english_block
        and bool(
            re.search(
                r"model_arch\s*==\s*MOONSHINE_MODEL_ARCH_SMALL_STREAMING",
                catalog or "",
            )
        )
    )
    report["architecture"] = "Verified" if arch_supported else "Unresolved"
    if arch_supported:
        report["evidence"].append(
            "core/moonshine-c-api.h:113; core/moonshine-model-catalog.cpp:149-165"
        )
    else:
        report["reason_codes"].append("SMALL_STREAMING_ARCHITECTURE_UNRESOLVED")

    loader_ok = bool(
        re.search(
            r"moonshine_load_transcriber_from_files\s*\(\s*const char\s*\*path\s*,\s*"
            r"uint32_t\s+model_arch",
            api or "",
        )
        and re.search(
            r"moonshine_load_transcriber_from_files\s*\(\s*const char\s*\*path\s*,\s*"
            r"uint32_t\s+model_arch",
            header,
        )
        and re.search(r"transcriber_options\.model_arch\s*=\s*model_arch\s*;", api or "")
    )
    report["loader"] = "Verified" if loader_ok else "Unresolved"
    if loader_ok:
        report["evidence"].append("core/moonshine-c-api.cpp:261-282")
    else:
        report["reason_codes"].append("MODEL_LOADER_CONTRACT_UNRESOLVED")

    required_files, optional_attention = _streaming_file_names(catalog or "")
    model_entry = None
    if english_block:
        model_entry = re.search(
            r"MOONSHINE_MODEL_ARCH_SMALL_STREAMING(?P<entry>.*?)(?:\},|$)",
            english_block,
            re.DOTALL,
        )
    quantized_path_match = re.search(
        r'kStreamingQuantizedDir\s*=\s*"([^\"]+)"', catalog or ""
    )
    quantized_path = quantized_path_match.group(1) if quantized_path_match else "unresolved"
    report["quantized_path"] = quantized_path
    small_url_ok = bool(model_entry and "/small-streaming-en" in model_entry.group("entry"))
    report["model_base_url"] = (
        "https://download.moonshine.ai/model/small-streaming-en/"
        f"{quantized_path.lstrip('/')}/"
        if small_url_ok and quantized_path != "unresolved"
        else "unresolved"
    )
    if small_url_ok and quantized_path != "unresolved":
        report["evidence"].append("core/moonshine-model-catalog.cpp:38-45,155-157")

    # Missing model_arch selects the first language catalog entry. Verify the
    # English entry order and the fallback behavior independently of its name.
    first_arch = None
    if english_block:
        first_match = re.search(r"MOONSHINE_MODEL_ARCH_([A-Z_]+)", english_block)
        if first_match:
            first_arch = first_match.group(1)
    catalog_first_fallback = bool(
        re.search(r"else\s*\{\s*model\s*=\s*&lang->models\.front\(\);", catalog or "")
    )
    if first_arch and catalog_first_fallback:
        default_value = _define_value(header, f"MOONSHINE_MODEL_ARCH_{first_arch}")
        report["english_default"] = f"{first_arch} ({default_value})"

    model_file_serializer = re.search(
        r"std::string\s+json_model_file\s*\([^)]*\)\s*\{(?P<body>.*?)return o;",
        api or "",
        re.DOTALL,
    )
    model_group_serializer = re.search(
        r"std::string\s+json_model_dependencies\s*\([^)]*\)\s*\{(?P<body>.*?)return o;",
        api or "",
        re.DOTALL,
    )
    manifest_serializer_ok = bool(
        model_file_serializer
        and all(
            re.search(r'\\?"' + re.escape(field) + r'\\?"', model_file_serializer.group("body"))
            for field in ("name", "url", "size", "checksum", "checksum_type")
        )
        and model_group_serializer
        and re.search(r'\\?"base_url\\?"', model_group_serializer.group("body"))
        and re.search(r'\\?"groups\\?"', model_group_serializer.group("body"))
    )

    deps_code_ok = bool(
        re.search(r"moonshine_get_stt_dependencies\s*\(", api or "")
        and re.search(r"stt_model_dependencies\s*\(", api or "")
        and catalog_header
        and re.search(r"stt_model_dependencies\s*\(", catalog_header)
        and re.search(r"stt_model_dependencies\s*\(", catalog or "")
        and metadata_header
        and re.search(r"find_model_file_metadata", catalog or "")
        and metadata_cpp
        and re.search(r"find_model_file_metadata\s*\(", metadata_cpp)
        and manifest_serializer_ok
    )
    report["dependencies"] = "Traced" if deps_code_ok else "Unresolved"
    if deps_code_ok:
        report["evidence"].append(
            "core/moonshine-c-api.cpp:2441-2503; core/moonshine-model-catalog.cpp:15-29,358-394; "
            "core/moonshine-model-file-metadata.generated.cpp:1-9"
        )
    else:
        report["reason_codes"].append("DEPENDENCY_MANIFEST_DISCOVERY_UNRESOLVED")

    required_files = required_files if small_url_ok and arch_supported else []
    optional_files: list[str] = []
    if optional_attention and small_url_ok and arch_supported:
        optional_files.append(f"word_timestamps: {optional_attention}")
    spelling_ok = bool(
        catalog
        and '"spelling_cnn.ort"' in catalog
        and '"spelling_cnn_meta.json"' in catalog
        and '"/spelling-en"' in catalog
    )
    if spelling_ok:
        optional_files.append("include_spelling: spelling_cnn.ort, spelling_cnn_meta.json")

    metadata: dict[str, ModelFileMetadata] = {}
    if metadata_cpp and quantized_path != "unresolved":
        url_prefix = (
            "https://download.moonshine.ai/model/small-streaming-en/"
            f"{quantized_path.lstrip('/')}/"
        )
        metadata = _metadata_entries(metadata_cpp, url_prefix)
    metadata_names = list(required_files)
    if optional_attention and small_url_ok and arch_supported:
        metadata_names.append(optional_attention)
    metadata_complete = bool(required_files) and all(
        name in metadata
        and metadata[name].size is not None
        and metadata[name].checksum is not None
        and metadata[name].checksum_type is not None
        for name in metadata_names
    )
    report["required_files"] = required_files
    report["optional_files"] = optional_files
    report["file_metadata"] = {
        name: {
            "expected_bytes": metadata[name].size,
            "vendor_checksum_type": metadata[name].checksum_type,
            "vendor_checksum": metadata[name].checksum,
            "vendor_checksum_present": metadata[name].checksum is not None,
        }
        for name in metadata_names
        if name in metadata
    }
    report["artifact_list"] = (
        "Verified" if required_files and small_url_ok and deps_code_ok else "Not verified"
    )
    report["artifact_revision"] = "Not verified"
    report["artifact_sizes"] = (
        "Not verified (source expected sizes present; model files absent)"
        if metadata_complete
        else "Not verified (source registry metadata incomplete; model files absent)"
    )
    report["artifact_sha256"] = "Not verified (source registry supplies CRC32C, not SHA-256)"
    if report["artifact_list"] == "Not verified":
        report["reason_codes"].append("ARTIFACT_MANIFEST_NOT_VERIFIED")
    if not metadata_complete:
        report["reason_codes"].append("SOURCE_ARTIFACT_METADATA_INCOMPLETE")
    report["reason_codes"].extend(
        ["ARTIFACT_REVISION_UNKNOWN", "LOCAL_MODEL_FILES_ABSENT", "PACKAGED_SHA256_REQUIRED"]
    )

    manifest_shape_ok = bool(
        manifest_serializer_ok
        and header
        and all(
            field in header
            for field in ('"base_url"', '"name"', '"url"', '"size"', '"checksum"', '"checksum_type"')
        )
        and "moonshine_free_buffer" in header
        and "moonshine_free_buffer" in api
    )
    if manifest_shape_ok:
        report["evidence"].append(
            "core/moonshine-c-api.h:1074-1119,319-333; core/moonshine-c-api.cpp:1815-1838,2441-2503"
        )

    license_ok = bool(
        license_doc
        and "all streaming speech-to-text models" in license_doc
        and "all English-language models" in license_doc
        and model_doc
        and re.search(r"\|\s*English\s*\|\s*Small Streaming\s*\|[^\n]*\|\s*MIT\s*\|", model_doc)
        and (source / "LICENSE").is_file()
    )
    report["model_license"] = "Present" if license_ok else "Incomplete"
    if license_ok:
        report["evidence"].append("docs/license.md:3-7; docs/models/available-models.md:14-18")
    else:
        report["reason_codes"].append("MODEL_SPECIFIC_LICENSE_EVIDENCE_INCOMPLETE")

    kiss_copying = _read(source, Path("core/third-party/kissfft/COPYING"))
    kiss_notice_missing = bool(
        not kiss_copying
        or "LICENSES/BSD-3-Clause" in kiss_copying
        and not (source / "core/third-party/kissfft/LICENSES/BSD-3-Clause").is_file()
    )
    ort_dir = source / "core/third-party/onnxruntime"
    ort_notice_missing = not ort_dir.is_dir() or not any(
        (ort_dir / name).is_file()
        for name in ("ThirdPartyNotices.txt", "THIRD_PARTY_NOTICES.txt", "NOTICE")
    )
    runtime_complete = not kiss_notice_missing and not ort_notice_missing
    report["runtime_notices"] = "Complete" if runtime_complete else "Incomplete"
    if not runtime_complete:
        report["reason_codes"].append("RUNTIME_THIRD_PARTY_NOTICES_INCOMPLETE")

    contract_verified = all(
        report[key] == expected
        for key, expected in (
            ("source_revision", "Verified"),
            ("header", "Verified"),
            ("architecture", "Verified"),
            ("loader", "Verified"),
            ("dependencies", "Traced"),
            ("artifact_list", "Verified"),
        )
    )
    report["source_contract_verified"] = contract_verified
    # A catalog and expected checksums do not prove a downloaded/packaged file.
    report["model_artifacts_verified"] = False
    report["acquisition"] = (
        "Ready for separate review"
        if contract_verified and metadata_complete and license_ok and runtime_complete
        else "Blocked"
    )
    report["exit_code"] = 0 if report["source_revision"] == "Verified" else 2
    if not contract_verified:
        report["reason_codes"].append("SOURCE_CONTRACT_INCOMPLETE")
        report["exit_code"] = 1
    return _finalize(report)


def _finalize(report: dict[str, object]) -> dict[str, object]:
    report["reason_codes"] = list(dict.fromkeys(report["reason_codes"]))
    return report


CHECKS = (
    ("Pinned source revision", "source_revision"),
    ("C API header", "header"),
    ("Small Streaming architecture", "architecture"),
    ("Model loader contract", "loader"),
    ("Dependency manifest discovery", "dependencies"),
    ("Exact model artifact list", "artifact_list"),
    ("Artifact revision", "artifact_revision"),
    ("Artifact byte sizes", "artifact_sizes"),
    ("Artifact SHA-256 values", "artifact_sha256"),
    ("Model license evidence", "model_license"),
    ("Runtime third-party notices", "runtime_notices"),
    ("Model acquisition readiness", "acquisition"),
)


def render_report(report: dict[str, object]) -> str:
    """Render deterministic human-readable findings without local paths."""
    lines = [
        "Moonshine model-source audit (local source only)",
        f"Pinned source revision: {report['git_revision']}",
        f"Origin: {report['git_origin']}",
        f"SOURCE_CONTRACT_VERIFIED: {'YES' if report['source_contract_verified'] else 'NO'}",
        f"MODEL_ARTIFACTS_VERIFIED: {'YES' if report['model_artifacts_verified'] else 'NO'}",
        "",
        "Check | Result",
        "--- | ---",
    ]
    for label, key in CHECKS:
        lines.append(f"{label} | {report[key]}")
    lines.extend(
        [
            "",
            f"C API header version: {report['header_version'] if report['header_version'] is not None else 'unavailable'}",
            f"Selected architecture: {report['architecture_name']} = {report['architecture_value'] if report['architecture_value'] is not None else 'unresolved'}",
            "Related architecture identifiers: " + "; ".join(report["related_architectures"]),
            f"English default when model_arch is omitted: {report['english_default']}",
            "Loader contract: explicit uint32_t model_arch argument is copied to TranscriberOptions.model_arch.",
            f"Quantized source path: {report['quantized_path']}",
            "Quantization selection: catalog URL path, not a separate STT variant option.",
            f"Model base URL in the local catalog: {report['model_base_url']}",
            "Required source-catalog files:",
        ]
    )
    for name in report["required_files"]:
        values = report["file_metadata"].get(name, {})
        size = values.get("expected_bytes", "unavailable")
        checksum_type = values.get("vendor_checksum_type", "unavailable")
        checksum = values.get("vendor_checksum") or "unavailable"
        lines.append(
            f"- {name} | source expected bytes: {size} | vendor checksum: {checksum_type}:{checksum}"
        )
    lines.append("Optional source-catalog files:")
    if report["optional_files"]:
        for optional in report["optional_files"]:
            if optional.startswith("word_timestamps: "):
                name = optional.split(": ", 1)[1]
                values = report["file_metadata"].get(name, {})
                lines.append(
                    f"- {optional} | source expected bytes: {values.get('expected_bytes', 'unavailable')}"
                    f" | vendor checksum: {values.get('vendor_checksum_type', 'unavailable')}:{values.get('vendor_checksum', 'unavailable')}"
                )
            else:
                lines.append(f"- {optional}")
    else:
        lines.append("- none verified")
    lines.extend(
        [
            "",
            "Manifest discovery: source-bundled catalog implementation and generated metadata registry; no remote manifest read.",
            "Manifest metadata fields: groups; group base_url and optional role; file name, URL, expected byte size (null if unknown), checksum, checksum_type.",
            "Selection options: model_arch; include_spelling/spelling/spelling_model_path; word_timestamps.",
            "Model architecture and artifact revision are not emitted as returned manifest fields.",
            "Model revision field: absent from the dependency manifest; the dated catalog directory is not artifact verification.",
            "Buffer release: moonshine_free_buffer (library-owned malloc/free boundary).",
            "",
            "Reason codes:",
        ]
    )
    codes = report["reason_codes"]
    lines.extend(f"- {code}" for code in codes) if codes else lines.append("- none")
    lines.append(f"Exit code: {report['exit_code']}")
    return "\n".join(lines) + "\n"


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", required=True, help="local pinned Moonshine source checkout")
    arguments = parser.parse_args(argv)
    report = audit_source(arguments.source_dir)
    print(render_report(report), end="")
    return int(report["exit_code"])


if __name__ == "__main__":
    raise SystemExit(main())
