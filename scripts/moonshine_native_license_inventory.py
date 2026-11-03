#!/usr/bin/env python3
"""Read-only inventory of the pinned Moonshine CMake runtime and license evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
from dataclasses import dataclass
from pathlib import Path
import re
import subprocess
import sys
from typing import Any, Sequence


PINNED_REVISION = "234f60faa0eb388b01cdf7e60aca232af37aefda"
ORT_RELATIVE_PATH = Path(
    "core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
)
ORT_SHA256 = "480790a978a48ad3e06ce86b6025e037bd70221637f5c104a8dde19617364cf4"
SYSTEM_FILE_VALUE = re.compile(r'^set\((CMAKE_(?:HOST_)?SYSTEM_(?:NAME|PROCESSOR)) "([^"]+)"\)', re.MULTILINE)


class InventoryError(ValueError):
    """An input did not match the pinned local source/configuration."""


@dataclass(frozen=True)
class Component:
    name: str
    role: str
    source: str
    revision: str
    evidence: tuple[str, ...]
    evidence_hashes: tuple[str, ...]
    status: str
    note: str = ""


def _git(source: Path, *arguments: str) -> str:
    """Run only local Git metadata queries; never fetch or execute hooks."""
    try:
        result = subprocess.run(
            ["git", "-C", str(source), *arguments],
            check=False,
            capture_output=True,
            text=True,
            timeout=15,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise InventoryError("local Git metadata could not be read") from error
    if result.returncode:
        raise InventoryError("local Git metadata query failed")
    return result.stdout.strip()


def _verify_source(source: Path) -> str:
    if not source.is_dir():
        raise InventoryError("Moonshine source checkout is missing")
    revision = _git(source, "rev-parse", "HEAD")
    if revision != PINNED_REVISION:
        raise InventoryError("Moonshine checkout is not the pinned revision")
    if _git(source, "status", "--porcelain", "--untracked-files=all"):
        raise InventoryError("Moonshine checkout is not clean")
    if not (source / "core" / "CMakeLists.txt").is_file():
        raise InventoryError("pinned native core CMake source is missing")
    return revision


def _read_cache(build: Path) -> dict[str, str]:
    cache_path = build / "CMakeCache.txt"
    if not cache_path.is_file():
        raise InventoryError("CMake cache is missing")
    values: dict[str, str] = {}
    try:
        for line in cache_path.read_text(encoding="utf-8").splitlines():
            if not line or line.startswith(("#", "//")) or "=" not in line:
                continue
            left, value = line.split("=", 1)
            key = left.split(":", 1)[0]
            values[key] = value
    except (OSError, UnicodeError) as error:
        raise InventoryError("CMake cache could not be read") from error
    if not values.get("CMAKE_HOME_DIRECTORY"):
        raise InventoryError("CMake cache has no source directory")
    return values


def _system_configuration(build: Path) -> tuple[str, str]:
    candidates = sorted((build / "CMakeFiles").glob("*/CMakeSystem.cmake"))
    for path in candidates:
        try:
            contents = path.read_text(encoding="utf-8")
        except (OSError, UnicodeError):
            continue
        values = dict(SYSTEM_FILE_VALUE.findall(contents))
        system_name = values.get("CMAKE_SYSTEM_NAME", values.get("CMAKE_HOST_SYSTEM_NAME", ""))
        processor = values.get("CMAKE_SYSTEM_PROCESSOR", values.get("CMAKE_HOST_SYSTEM_PROCESSOR", ""))
        if system_name or processor:
            return system_name, processor
    return "", ""


def _json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as error:
        raise InventoryError("CMake File API data is missing or invalid") from error
    if not isinstance(value, dict):
        raise InventoryError("CMake File API record has an unexpected shape")
    return value


def _file_api(build: Path, source: Path) -> tuple[dict[str, Any], dict[str, Any], dict[str, dict[str, Any]]]:
    reply = build / ".cmake" / "api" / "v1" / "reply"
    indexes = sorted(reply.glob("index-*.json"))
    if not indexes:
        raise InventoryError("CMake File API reply is missing")
    index = _json(indexes[-1])
    codemodel_ref = index.get("reply", {}).get("codemodel-v2", {})
    codemodel_file = codemodel_ref.get("jsonFile") if isinstance(codemodel_ref, dict) else None
    if not isinstance(codemodel_file, str):
        raise InventoryError("CMake File API has no codemodel-v2 reply")
    codemodel = _json(reply / codemodel_file)
    paths = codemodel.get("paths", {})
    if Path(str(paths.get("source", ""))).resolve() != (source / "core").resolve():
        raise InventoryError("CMake File API source does not match source-dir/core")
    if Path(str(paths.get("build", ""))).resolve() != build.resolve():
        raise InventoryError("CMake File API build directory does not match build-dir")
    configurations = codemodel.get("configurations", [])
    if not configurations:
        raise InventoryError("CMake File API has no configured targets")
    summaries = configurations[0].get("targets", [])
    targets: dict[str, dict[str, Any]] = {}
    for summary in summaries:
        name, filename = summary.get("name"), summary.get("jsonFile")
        if not isinstance(name, str) or not isinstance(filename, str):
            continue
        target_path = (reply / filename).resolve()
        try:
            target_path.relative_to(reply.resolve())
        except ValueError as error:
            raise InventoryError("CMake File API target path escapes reply directory") from error
        targets[name] = _json(target_path)
    if "moonshine" not in targets:
        raise InventoryError("CMake File API does not contain the moonshine target")
    return codemodel, targets["moonshine"], targets


def _sha256(path: Path) -> str:
    digest = hashlib.sha256()
    try:
        with path.open("rb") as stream:
            for chunk in iter(lambda: stream.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        raise InventoryError("local evidence file could not be hashed") from error
    return digest.hexdigest()


def _evidence(source: Path, paths: Sequence[str]) -> tuple[tuple[str, ...], tuple[str, ...]]:
    found: list[str] = []
    hashes: list[str] = []
    for relative in paths:
        path = source / relative
        if path.is_file():
            found.append(relative)
            hashes.append(_sha256(path))
    return tuple(found), tuple(hashes)


def _component(
    source: Path,
    name: str,
    role: str,
    origin: str,
    revision: str,
    evidence_candidates: Sequence[str],
    *,
    required: bool = True,
    unresolved_note: str = "",
) -> Component:
    evidence, hashes = _evidence(source, evidence_candidates)
    status = "EVIDENCE_PRESENT" if evidence else "MISSING"
    note = unresolved_note
    if required and not evidence:
        note = (note + "; " if note else "") + "no local license or notice file found"
    if unresolved_note:
        status = "UNRESOLVED"
    revision_label = f"{revision} (Moonshine source tree; standalone component version not stated)"
    return Component(name, role, origin, revision_label, evidence, hashes, status, note)


def _relative_source_path(path: str, source: Path) -> str:
    candidate = Path(path)
    if not candidate.is_absolute():
        candidate = source / candidate
    try:
        return candidate.resolve().relative_to(source.resolve()).as_posix()
    except (OSError, ValueError):
        return f"external/{candidate.name}"


def collect_inventory(source_dir: str | Path, build_dir: str | Path) -> tuple[str, list[Component], list[str]]:
    """Validate local inputs and return deterministic report fields."""
    source = Path(source_dir).resolve()
    build = Path(build_dir).resolve()
    revision = _verify_source(source)
    cache = _read_cache(build)
    configured_core = cache.get("CMAKE_HOME_DIRECTORY", "")
    if Path(configured_core).resolve() != (source / "core").resolve():
        raise InventoryError("CMake cache source does not match source-dir/core")
    if cache.get("CMAKE_OSX_ARCHITECTURES") != "arm64":
        raise InventoryError("CMake cache is not configured for macOS ARM64")
    system_name, processor = _system_configuration(build)
    if system_name not in {"Darwin", "macOS"} or processor.lower() not in {"arm64", "aarch64"}:
        raise InventoryError("CMake generated platform is not macOS ARM64")

    codemodel, moonshine, targets = _file_api(build, source)
    if moonshine.get("type") != "SHARED_LIBRARY":
        raise InventoryError("configured moonshine target is not a shared library")
    source_paths = sorted(
        _relative_source_path(str(item["path"]), source / "core")
        for item in moonshine.get("sources", [])
        if isinstance(item, dict) and isinstance(item.get("path"), str)
    )
    if not source_paths:
        raise InventoryError("moonshine target source list is empty")
    dependency_ids = {str(item.get("id", "")).split("::", 1)[0] for item in moonshine.get("dependencies", [])}
    runtime_targets = sorted(name for name in dependency_ids if name in targets)
    if not runtime_targets:
        raise InventoryError("moonshine target dependencies are unresolved in the File API")
    link_fragments = [
        str(fragment.get("fragment", ""))
        for fragment in moonshine.get("link", {}).get("commandFragments", [])
        if fragment.get("role") == "libraries"
    ]
    ort_configured = cache.get("ONNXRUNTIME_LIB_PATH", "")
    expected_ort = (source / ORT_RELATIVE_PATH).resolve()
    ort_linked = any(Path(fragment).resolve() == expected_ort for fragment in link_fragments if fragment)
    if ort_configured and Path(ort_configured).resolve() != expected_ort:
        raise InventoryError("CMake cache ONNX Runtime path differs from the pinned artifact")

    includes = {
        _relative_source_path(str(item.get("path", "")), source / "core")
        for group in moonshine.get("compileGroups", [])
        for item in group.get("includes", [])
        if isinstance(item, dict) and isinstance(item.get("path"), str)
    }
    defines = {
        str(item.get("define", ""))
        for group in moonshine.get("compileGroups", [])
        for item in group.get("defines", [])
        if isinstance(item, dict)
    }
    tts_disabled = cache.get("MOONSHINE_TTS_BUILD_ONNX") in {"OFF", "0", "FALSE"} and "MOONSHINE_C_API_MOONSHINE_TTS=0" in defines

    records: list[Component] = [
        _component(
            source,
            "Moonshine streaming C API target",
            "RUNTIME",
            "core/ (moonshine target)",
            revision,
            ("LICENSE", "docs/license.md"),
        ),
    ]
    target_origins = {
        "bin-tokenizer": "core/bin-tokenizer/",
        "moonshine-utils": "core/moonshine-utils/",
        "ort-utils": "core/ort-utils/",
    }
    for name in runtime_targets:
        records.append(_component(source, name, "RUNTIME", target_origins.get(name, f"core/{name}/"), revision, ("LICENSE",)))

    source_set = set(source_paths)
    if any(path.startswith("cpp-annote/") for path in source_set):
        records.append(_component(source, "cpp-annote", "RUNTIME", "core/cpp-annote/ (compiled into moonshine)", revision, ("core/cpp-annote/LICENSE",)))
    runtime_specs = (
        ("Eigen", any(path in {"third-party", "third-party/Eigen"} or path.endswith("/third-party") for path in includes) and "EIGEN_MPL2_ONLY=1" in defines,
         "core/third-party/Eigen/", ("core/third-party/Eigen/COPYING.MPL2",)),
        ("kaldi-native-fbank", any(path.startswith("third-party/kaldi-native-fbank/") for path in source_set),
         "core/third-party/kaldi-native-fbank/ (compiled into moonshine)", ("core/third-party/kaldi-native-fbank/LICENSE",)),
        ("KISS FFT", any(path.startswith("third-party/kissfft/") for path in source_set),
         "core/third-party/kissfft/ (compiled into moonshine)", ("core/third-party/kissfft/COPYING",)),
        ("UTF8-CPP", "third-party/utf-8" in includes,
         "core/third-party/utf-8/ (header input to moonshine)", ("core/third-party/utf-8/LICENSE.txt",)),
    )
    for name, selected, origin, evidence in runtime_specs:
        if selected:
            incomplete_notice = (
                "COPYING refers to core/third-party/kissfft/LICENSES/BSD-3-Clause, which is not present"
                if name == "KISS FFT" and not (source / "core/third-party/kissfft/LICENSES/BSD-3-Clause").is_file()
                else ""
            )
            records.append(
                _component(source, name, "RUNTIME", origin, revision, evidence, unresolved_note=incomplete_notice)
            )

    ort_notices = (
        "core/third-party/onnxruntime/ThirdPartyNotices.txt",
        "core/third-party/onnxruntime/NOTICE",
        "core/third-party/onnxruntime/NOTICE.txt",
        "ThirdPartyNotices.txt",
        "NOTICE",
        "NOTICE.txt",
    )
    available_notices, _ = _evidence(source, ort_notices)
    ort_evidence = ("core/third-party/onnxruntime/LICENSE.txt", *ort_notices)
    artifact_path = source / ORT_RELATIVE_PATH
    artifact_hash = _sha256(artifact_path) if artifact_path.is_file() else ""
    artifact_verified = bool(ort_linked and artifact_hash == ORT_SHA256)
    ort_note_parts: list[str] = []
    if not ort_linked:
        ort_note_parts.append("File API does not establish a link to the pinned ORT dylib")
    if not artifact_verified:
        ort_note_parts.append("local artifact is missing or does not match Task 75 SHA-256")
    if not available_notices:
        ort_note_parts.append("no local ONNX Runtime third-party notice file found")
    records.append(
        _component(
            source,
            "ONNX Runtime",
            "RUNTIME" if ort_linked else "UNRESOLVED",
            ORT_RELATIVE_PATH.as_posix() + "; core/third-party/onnxruntime/README.md",
            "1.23.2 (artifact SHA-256 checked against Task 75)",
            ort_evidence,
            unresolved_note="; ".join(ort_note_parts) if ort_note_parts else "",
        )
    )
    records[-1] = Component(
        **{**records[-1].__dict__, "revision": "1.23.2 (artifact SHA-256 checked against Task 75)"}
    )
    if artifact_verified:
        records[-1] = Component(
            **{**records[-1].__dict__, "note": ((records[-1].note + "; ") if records[-1].note else "") + f"artifact SHA-256 verified: {artifact_hash}"}
        )

    # The pinned File API configuration has TTS disabled, so these are not part
    # of the selected streaming target even though their separate targets exist.
    optional_specs = (
        ("doctest", "core/third-party/doctest/", ("core/third-party/doctest/LICENSE.txt",), "test-only"),
        ("nlohmann/json", "core/third-party/nlohmann/", ("core/third-party/nlohmann/LICENSE.MIT",), "optional TTS/G2P path"),
        ("utf8proc", "core/third-party/utf8proc/", ("core/third-party/utf8proc/LICENSE.md",), "optional TTS/G2P path"),
    )
    for name, origin, evidence, note in optional_specs:
        role = "BUILD-ONLY" if name == "doctest" else "OPTIONAL"
        records.append(_component(source, name, role, origin, revision, evidence, required=False, unresolved_note=""))
        if name == "doctest":
            records[-1] = Component(**{**records[-1].__dict__, "note": note})
        elif not tts_disabled:
            records[-1] = Component(**{**records[-1].__dict__, "note": "TTS/G2P selection is not disabled in this cache"})
        else:
            records[-1] = Component(**{**records[-1].__dict__, "note": note + "; MOONSHINE_TTS_BUILD_ONNX=OFF"})

    records.append(
        _component(
            source,
            "Moonshine TTS/G2P target family",
            "OPTIONAL",
            "core/moonshine-tts/ (not a dependency of the moonshine target)",
            revision,
            ("LICENSE", "docs/license.md"),
            required=False,
            unresolved_note="" if tts_disabled else "TTS/G2P option is not disabled in this cache",
        )
    )
    records.append(
        Component(
            "macOS system library/framework closure",
            "UNRESOLVED",
            "CMake File API link fragments; no built libmoonshine.dylib exists",
            "macOS ARM64 configure",
            (),
            (),
            "UNRESOLVED",
            "no explicit -framework fragment is present; actual transitive system dylibs require built-binary inspection",
        )
    )

    records.sort(key=lambda item: item.name.casefold())
    target_names = sorted(targets)
    optional_targets = [
        name for name in target_names
        if name != "moonshine"
        and name not in runtime_targets
        and any(token in name.casefold() for token in ("test", "benchmark", "tts", "g2p", "smoke"))
    ]
    platform = f"{system_name} {processor}; CMAKE_OSX_ARCHITECTURES={cache['CMAKE_OSX_ARCHITECTURES']}"
    test_targets = [name for name in optional_targets if "test" in name.casefold()]
    benchmark_targets = [name for name in optional_targets if "benchmark" in name.casefold()]
    tts_g2p_targets = [name for name in optional_targets if "tts" in name.casefold() or "g2p" in name.casefold()]
    utility_targets = [name for name in optional_targets if "smoke" in name.casefold() or "batch" in name.casefold()]

    def target_group(label: str, values: list[str]) -> str:
        examples = ", ".join(values[:6])
        suffix = f"; examples: {examples}" if examples else ""
        if len(values) > 6:
            suffix += f", … (+{len(values) - 6})"
        return f"  - {label}: {len(values)} target(s){suffix}"

    details = [
        f"Pinned source revision: {revision} (clean)",
        f"Configured platform: {platform}",
        f"CMake build type: {cache.get('CMAKE_BUILD_TYPE', 'not recorded')}",
        f"Moonshine target: {moonshine['type']} (artifact not built)",
        f"TTS/G2P selected: {'no' if tts_disabled else 'unresolved'}",
        f"CMake target dependencies: {', '.join(runtime_targets)}",
        f"ONNX Runtime link: {'confirmed in File API' if ort_linked else 'UNRESOLVED'}",
        f"ONNX Runtime artifact integrity: {'VERIFIED' if artifact_verified else 'UNRESOLVED'}"
        + (f" (SHA-256 {artifact_hash})" if artifact_hash else ""),
        f"Moonshine target source files ({len(source_paths)}):",
        *[f"  - core/{path}" for path in source_paths],
        "Optional configured targets not linked by the selected moonshine target:",
        target_group("tests", test_targets),
        target_group("benchmarks", benchmark_targets),
        target_group("TTS/G2P", tts_g2p_targets),
        target_group("smoke and batch utilities", utility_targets),
        "Build-only tools: CMake and the configured AppleClang toolchain; neither is an app runtime component.",
    ]
    return "\n".join(details), records, optional_targets


def render_report(details: str, components: Sequence[Component]) -> str:
    lines = [
        "Moonshine native dependency and license inventory",
        "",
        details,
        "",
        "Components (status describes local evidence, not redistribution approval):",
    ]
    for item in components:
        evidence = ", ".join(item.evidence) if item.evidence else "none found"
        hashes = ", ".join(item.evidence_hashes) if item.evidence_hashes else "none"
        lines.extend(
            [
                f"- Component: {item.name}",
                f"  Role: {item.role}",
                f"  Source: {item.source}",
                f"  Version/revision: {item.revision}",
                f"  License evidence: {evidence}",
                f"  Evidence SHA-256: {hashes}",
                f"  Status: {item.status}",
            ]
        )
        if item.note:
            lines.append(f"  Note: {item.note}")
    unresolved = [item.name for item in components if item.status in {"MISSING", "UNRESOLVED"}]
    lines.extend(
        [
            "",
            "Native source inventory: VERIFIED at the configured CMake target source/link level.",
            "Redistribution approval: NOT YET ESTABLISHED.",
            "REDISTRIBUTION_EVIDENCE_INCOMPLETE",
            "Moonshine license boundary: root LICENSE/docs/license.md exclude core/third-party; vendor terms are separate.",
            "Unresolved or missing evidence: " + (", ".join(unresolved) if unresolved else "none detected"),
            "A present license file is evidence to review; it does not prove complete notices, binary provenance, package contents, or redistribution approval.",
        ]
    )
    return "\n".join(lines)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-dir", required=True, help="clean Moonshine checkout at the pinned commit")
    parser.add_argument("--build-dir", required=True, help="existing Task 76 CMake build directory")
    arguments = parser.parse_args(argv)
    try:
        details, components, _ = collect_inventory(arguments.source_dir, arguments.build_dir)
        print(render_report(details, components))
    except InventoryError as error:
        print(f"inventory error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
