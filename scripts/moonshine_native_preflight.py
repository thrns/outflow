#!/usr/bin/env python3
"""Inspect local prerequisites for an offline Moonshine CMake configure.

This command only reads project/source metadata and runs bounded local version
or Git inspection commands. It never configures CMake, builds, downloads,
installs, or edits a source checkout. A ready result means only that a native
configure can be attempted; it is not build, ABI, inference, or release proof.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import subprocess
from typing import Callable, Mapping, Sequence

try:  # Direct script execution and package-based unittest imports.
    from . import validate_model_asset_inventory as inventory
except ImportError:  # pragma: no cover - exercised by direct CLI execution.
    import validate_model_asset_inventory as inventory


COMMAND_TIMEOUT_SECONDS = 8
VERSION_PATTERN = re.compile(r"(?im)^\s*cmake version\s+([0-9]+(?:\.[0-9]+){1,2})\b")
MINIMUM_CMAKE_PATTERN = re.compile(
    r"(?im)^\s*cmake_minimum_required\s*\(\s*VERSION\s+([0-9]+(?:\.[0-9]+){1,2})"
)
REVISION_ASSIGNMENT_PATTERN = re.compile(r'^SOURCE_REVISION\s*=\s*["\']([0-9a-fA-F]{40})["\']\s*$', re.M)
HEADER_VERSION_PATTERN = re.compile(
    # The pinned upstream header wraps its integer macro value in parentheses.
    r"(?im)^\s*#\s*define\s+MOONSHINE_HEADER_VERSION\s+\(?\s*([0-9]+)\s*\)?\b"
)
LFS_POINTER_VERSION = "version https://git-lfs.github.com/spec/v1"
LFS_OID_PATTERN = re.compile(r"^oid sha256:([0-9a-f]{64})$")
LFS_SIZE_PATTERN = re.compile(r"^size ([1-9][0-9]*)$")
# At the pinned Moonshine revision, the native APPLE/arm64 branch in
# core/third-party/onnxruntime/find-ort-library-path.cmake selects this dylib.
ONNX_RUNTIME_RELATIVE_PATH = Path(
    "core/third-party/onnxruntime/lib/macos/arm64/libonnxruntime.1.23.2.dylib"
)
ONNX_RUNTIME_EXPECTED_ARCHITECTURE = "arm64"
HEADER_EXTENSIONS = {".h", ".hh", ".hpp", ".hxx"}
HEADER_MAX_BYTES = 2_000_000

SUPPORTED_ARCHITECTURES = {
    "Darwin": {"arm64", "x64"},
    "Windows": {"x64"},
    "Linux": {"x64"},
}

CompilerLocator = Callable[[str], str | None]
CommandRunner = Callable[..., subprocess.CompletedProcess[str]]


@dataclass(frozen=True)
class Check:
    """One ordered readiness observation and its user-actionable details."""

    name: str
    status: str
    details: tuple[str, ...] = ()
    blocks_configure: bool = False


@dataclass(frozen=True)
class PreflightReport:
    """Read-only observations collected for one local readiness check."""

    checks: tuple[Check, ...]
    ready: bool


@dataclass(frozen=True)
class LfsObjectMetadata:
    """Integrity values parsed from a Git-tracked LFS pointer."""

    sha256: str
    size_bytes: int


def _parse_lfs_pointer(contents: str) -> LfsObjectMetadata | None:
    """Parse the simple SHA-256 LFS pointer form tracked by the pinned source."""
    lines = contents.splitlines()
    if len(lines) != 3 or lines[0] != LFS_POINTER_VERSION:
        return None
    oid_match = LFS_OID_PATTERN.fullmatch(lines[1])
    size_match = LFS_SIZE_PATTERN.fullmatch(lines[2])
    if oid_match is None or size_match is None:
        return None
    return LfsObjectMetadata(oid_match.group(1), int(size_match.group(1)))


def _macho_library_architecture(path: Path) -> str | None:
    """Inspect the thin 64-bit Mach-O dylib header without loading the binary.

    Fat or unfamiliar formats stay unverified. The pinned macOS arm64 artifact
    is a thin dylib, as independently confirmed with Apple's `lipo` tool.
    """
    try:
        with path.open("rb") as library:
            header = library.read(32)
    except OSError:
        return None
    if len(header) < 16:
        return None

    magic = header[:4]
    if magic == b"\xcf\xfa\xed\xfe":
        byte_order = "little"
    elif magic == b"\xfe\xed\xfa\xcf":
        byte_order = "big"
    else:
        return None

    cpu_type = int.from_bytes(header[4:8], byte_order, signed=True)
    file_type = int.from_bytes(header[12:16], byte_order)
    if file_type != 6:  # MH_DYLIB; this path is not the Swift static archive.
        return None
    return {
        0x0100000C: "arm64",
        0x01000007: "x86_64",
    }.get(cpu_type)


def verify_native_artifact(
    artifact_path: Path,
    pointer_contents: str | None,
    *,
    expected_architecture: str,
) -> Check:
    """Verify a materialized native library against tracked LFS metadata.

    This is read-only. A pointer, missing file, size/hash mismatch, or
    unrecognized/incompatible Mach-O header never counts as configure-ready.
    """
    expected = _parse_lfs_pointer(pointer_contents or "")
    expected_description = (
        f"Pinned LFS object SHA-256 {expected.sha256}, size {expected.size_bytes} bytes."
        if expected is not None
        else "The pinned Git object did not provide a valid SHA-256 LFS pointer."
    )

    if not artifact_path.exists():
        return Check(
            "ONNX Runtime native library",
            "MISSING",
            (
                f"Required path: {artifact_path}",
                expected_description,
                "The native Moonshine CMake configuration requires this platform library.",
                "Blocks native configuration.",
            ),
            True,
        )
    if artifact_path.is_symlink() or not artifact_path.is_file():
        return Check(
            "ONNX Runtime native library",
            "PRESENT_UNVERIFIED",
            (
                f"Required path is not a regular, non-symlink file: {artifact_path}",
                expected_description,
                "Blocks native configuration.",
            ),
            True,
        )

    try:
        with artifact_path.open("rb") as library:
            prefix = library.read(160)
        actual_size = artifact_path.stat().st_size
    except OSError as error:
        return Check(
            "ONNX Runtime native library",
            "PRESENT_UNVERIFIED",
            (
                f"Could not read the required library ({type(error).__name__}).",
                expected_description,
                "Blocks native configuration.",
            ),
            True,
        )

    if prefix.startswith(LFS_POINTER_VERSION.encode("ascii")):
        return Check(
            "ONNX Runtime native library",
            "LFS_POINTER_NOT_MATERIALIZED",
            (
                f"Required path contains Git LFS pointer text, not a native library: {artifact_path}",
                expected_description,
                "A pointer's presence does not make the native library available.",
                "Blocks native configuration.",
            ),
            True,
        )
    if expected is None:
        return Check(
            "ONNX Runtime native library",
            "PRESENT_UNVERIFIED",
            (
                f"A file exists at {artifact_path}, but its pinned LFS identity is unavailable.",
                expected_description,
                "Blocks native configuration.",
            ),
            True,
        )
    if actual_size != expected.size_bytes:
        return Check(
            "ONNX Runtime native library",
            "PRESENT_UNVERIFIED",
            (
                f"File size mismatch: expected {expected.size_bytes} bytes, found {actual_size} bytes.",
                f"Pinned LFS object SHA-256 {expected.sha256}.",
                "Blocks native configuration.",
            ),
            True,
        )

    digest = hashlib.sha256()
    try:
        with artifact_path.open("rb") as library:
            for chunk in iter(lambda: library.read(1024 * 1024), b""):
                digest.update(chunk)
    except OSError as error:
        return Check(
            "ONNX Runtime native library",
            "PRESENT_UNVERIFIED",
            (
                f"Could not hash the required library ({type(error).__name__}).",
                expected_description,
                "Blocks native configuration.",
            ),
            True,
        )
    actual_sha256 = digest.hexdigest()
    if actual_sha256 != expected.sha256:
        return Check(
            "ONNX Runtime native library",
            "PRESENT_UNVERIFIED",
            (
                f"SHA-256 mismatch: expected {expected.sha256}, found {actual_sha256}.",
                f"File size matches the pinned {expected.size_bytes}-byte LFS object.",
                "Blocks native configuration.",
            ),
            True,
        )

    architecture = _macho_library_architecture(artifact_path)
    if architecture is None:
        architecture_detail = "The file is not a recognized 64-bit Mach-O dynamic library."
    elif architecture != expected_architecture:
        architecture_detail = (
            f"Architecture mismatch: expected {expected_architecture}; found {architecture}."
        )
    else:
        architecture_detail = ""
    if architecture_detail:
        return Check(
            "ONNX Runtime native library",
            "PRESENT_UNVERIFIED",
            (
                f"Pinned size and SHA-256 match ({expected.size_bytes} bytes, {actual_sha256}).",
                architecture_detail,
                "Blocks native configuration.",
            ),
            True,
        )

    return Check(
        "ONNX Runtime native library",
        "VERIFIED",
        (
            f"Verified {artifact_path} against pinned LFS SHA-256 {actual_sha256} and size {actual_size} bytes.",
            f"Mach-O dynamic library contains the required {expected_architecture} architecture.",
        ),
    )


def _normalized_host(system: str, machine: str) -> tuple[str, str, str, bool]:
    """Normalize Python platform names against the documented MVP target set."""
    normalized_system = {"Darwin": "Darwin", "Windows": "Windows", "Linux": "Linux"}.get(system)
    raw_arch = machine.casefold()
    if raw_arch in {"arm64", "aarch64"}:
        architecture = "arm64"
    elif raw_arch in {"x86_64", "amd64", "x64"}:
        architecture = "x64"
    else:
        architecture = machine or "unknown"
    if normalized_system is None:
        label = f"{system or 'unknown'} {architecture}"
        return label, system, architecture, False
    os_label = {"Darwin": "macOS", "Windows": "Windows", "Linux": "Linux"}[normalized_system]
    label = f"{os_label} {architecture}"
    supported = architecture in SUPPORTED_ARCHITECTURES[normalized_system]
    return label, normalized_system, architecture, supported


def _version_tuple(value: str) -> tuple[int, ...]:
    return tuple(int(part) for part in value.split("."))


def _version_at_least(actual: str, minimum: str) -> bool:
    actual_parts = _version_tuple(actual)
    minimum_parts = _version_tuple(minimum)
    width = max(len(actual_parts), len(minimum_parts))
    return actual_parts + (0,) * (width - len(actual_parts)) >= minimum_parts + (0,) * (
        width - len(minimum_parts)
    )


def _expected_source_revision(repository_root: Path) -> str | None:
    """Read the existing pin from the source probe; do not maintain a second pin."""
    probe_path = repository_root / "scripts" / "probe_moonshine_cmake.py"
    try:
        text = probe_path.read_text(encoding="utf-8")
    except OSError:
        return None
    match = REVISION_ASSIGNMENT_PATTERN.search(text)
    return match.group(1).lower() if match is not None else None


def _minimum_cmake_version(source_dir: Path | None) -> str | None:
    if source_dir is None:
        return None
    cmake_file = source_dir / "core" / "CMakeLists.txt"
    try:
        text = cmake_file.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    match = MINIMUM_CMAKE_PATTERN.search(text)
    return match.group(1) if match is not None else None


def _cmake_version(
    cmake_path: str,
    run: CommandRunner,
) -> tuple[str | None, str | None]:
    try:
        result = run(
            [cmake_path, "--version"],
            capture_output=True,
            text=True,
            timeout=COMMAND_TIMEOUT_SECONDS,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        return None, f"could not run CMake version query ({type(error).__name__})"
    output = f"{result.stdout or ''}\n{result.stderr or ''}"
    match = VERSION_PATTERN.search(output)
    if result.returncode != 0 or match is None:
        return None, "CMake --version exited unsuccessfully or returned unrecognized output"
    return match.group(1), None


def _compiler_command(
    variable: str,
    candidates: Sequence[str],
    environment: Mapping[str, str],
    which: CompilerLocator,
) -> tuple[str | None, str, str]:
    configured = environment.get(variable, "").strip()
    command_text = configured or candidates[0]
    try:
        command_parts = shlex.split(command_text)
    except ValueError:
        command_parts = []
    if not command_parts:
        return None, command_text, f"{variable} is not a usable executable command"

    if configured:
        executable = which(command_parts[0])
        if executable is None:
            return None, command_parts[0], f"{variable} selects an executable not found on PATH"
        return executable, command_parts[0], ""

    for candidate in candidates:
        executable = which(candidate)
        if executable is not None:
            return executable, candidate, ""
    return None, " or ".join(candidates), f"no {variable} compiler executable was found on PATH"


def _source_directory(source_dir: str | os.PathLike[str] | None) -> tuple[Path | None, Check]:
    if source_dir is None:
        return None, Check(
            "Moonshine source directory",
            "MISSING",
            (
                "No local checkout path is configured or supplied; no directory was checked.",
                "Required because the native configure needs the Moonshine CMake source tree.",
                "Verify locally: python3 scripts/moonshine_native_preflight.py --source-dir <existing-checkout>",
                "Blocks native configuration.",
            ),
            True,
        )
    requested = Path(source_dir).expanduser()
    try:
        resolved = requested.resolve(strict=True)
    except (OSError, RuntimeError):
        return None, Check(
            "Moonshine source directory",
            "MISSING",
            (
                f"Checked path: {requested}",
                "Required because the native configure needs the Moonshine CMake source tree.",
                f"Verify locally: git -C {requested} rev-parse HEAD",
                "Blocks native configuration.",
            ),
            True,
        )
    if not resolved.is_dir():
        return None, Check(
            "Moonshine source directory",
            "MISSING",
            (
                f"Checked path: {resolved} (not a directory)",
                "Required because the native configure needs the Moonshine CMake source tree.",
                f"Verify locally: git -C {resolved} rev-parse HEAD",
                "Blocks native configuration.",
            ),
            True,
        )
    if not (resolved / "core" / "CMakeLists.txt").is_file():
        return resolved, Check(
            "Moonshine source directory",
            "MISSING",
            (
                f"Directory exists at {resolved}, but the required core/CMakeLists.txt entry point is missing.",
                "A directory with a Moonshine-like name is not enough to establish a usable source checkout.",
                f"Verify locally: test -f {resolved / 'core' / 'CMakeLists.txt'}",
                "Blocks native configuration.",
            ),
            True,
        )
    return resolved, Check(
        "Moonshine source directory",
        "PRESENT",
        (
            f"Checked path: {resolved}",
            "Directory presence alone does not establish a valid Moonshine checkout.",
        ),
    )


def _find_c_api_header(source_dir: Path) -> tuple[Path | None, str | None, str | None]:
    """Find a header that declares the documented version macro and loader API."""
    walk_errors: list[str] = []

    def record_walk_error(error: OSError) -> None:
        walk_errors.append(type(error).__name__)

    for current, directories, filenames in os.walk(source_dir, onerror=record_walk_error, followlinks=False):
        directories[:] = sorted(name for name in directories if name != ".git")
        for filename in sorted(filenames):
            candidate = Path(current) / filename
            if candidate.suffix.casefold() not in HEADER_EXTENSIONS:
                continue
            try:
                if candidate.is_symlink() or not candidate.is_file() or candidate.stat().st_size > HEADER_MAX_BYTES:
                    continue
                contents = candidate.read_text(encoding="utf-8", errors="replace")
            except OSError:
                continue
            version_match = HEADER_VERSION_PATTERN.search(contents)
            if version_match and "moonshine_load_transcriber_from_files" in contents:
                return candidate, version_match.group(1), None
    if walk_errors:
        return None, None, "one or more source directories could not be inspected"
    return None, None, None


def _source_revision_check(
    source_dir: Path | None,
    expected_revision: str | None,
    run: CommandRunner,
) -> Check:
    if source_dir is None:
        return Check(
            "Source revision",
            "UNVERIFIED",
            (
                "No source checkout is available to inspect.",
                "Required because the configure experiment accepts only the pinned, clean source revision.",
                "Verify locally: git -C <source-dir> rev-parse HEAD and git -C <source-dir> status --porcelain --untracked-files=all",
                "Blocks native configuration.",
            ),
            True,
        )
    if expected_revision is None:
        return Check(
            "Source revision",
            "UNVERIFIED",
            (
                "Could not read the existing source revision pin from scripts/probe_moonshine_cmake.py.",
                "Required to compare this checkout with the repository's approved source revision.",
                "Verify locally: inspect SOURCE_REVISION in scripts/probe_moonshine_cmake.py.",
                "Blocks native configuration.",
            ),
            True,
        )

    def git(arguments: Sequence[str]) -> subprocess.CompletedProcess[str] | None:
        try:
            return run(
                ["git", "-C", str(source_dir), *arguments],
                capture_output=True,
                text=True,
                timeout=COMMAND_TIMEOUT_SECONDS,
                check=False,
            )
        except (OSError, subprocess.TimeoutExpired):
            return None

    root_result = git(("rev-parse", "--show-toplevel"))
    if root_result is None or root_result.returncode != 0:
        detail = "The path is not an inspectable local Git checkout."
    elif not (root_result.stdout or "").strip():
        detail = "Git returned an empty checkout root."
    elif Path((root_result.stdout or "").strip()).resolve() != source_dir:
        detail = "The supplied directory is inside, rather than the root of, a Git checkout."
    else:
        revision_result = git(("rev-parse", "HEAD"))
        if revision_result is None or revision_result.returncode != 0:
            detail = "git rev-parse HEAD could not read a local revision."
        else:
            revision = (revision_result.stdout or "").strip().lower()
            if not re.fullmatch(r"[0-9a-f]{40}", revision):
                detail = "Git returned no valid 40-character source revision."
            elif revision != expected_revision:
                detail = f"Expected pinned revision {expected_revision}; found {revision}."
            else:
                status_result = git(("status", "--porcelain", "--untracked-files=all"))
                if status_result is None or status_result.returncode != 0:
                    detail = "Could not verify that the source checkout is clean."
                elif (status_result.stdout or "").strip():
                    detail = "The pinned source checkout has local modifications or untracked files."
                else:
                    return Check(
                        "Source revision",
                        "VERIFIED",
                        (
                            f"HEAD matches the repository pin {expected_revision} and the checkout is clean.",
                            "This verifies source provenance only; it does not prove C ABI linkage or build success.",
                        ),
                    )
    return Check(
        "Source revision",
        "UNVERIFIED",
        (
            detail,
            f"Checked path: {source_dir}",
            "Required because the configure experiment accepts only the pinned, clean source revision.",
            "Verify locally: git -C <source-dir> rev-parse HEAD and git -C <source-dir> status --porcelain --untracked-files=all",
            "Blocks native configuration.",
        ),
        True,
    )


def _onnx_runtime_check(
    source_dir: Path | None,
    *,
    normalized_system: str | None,
    architecture: str,
    revision_check: Check,
    run: CommandRunner,
) -> Check:
    """Verify the pinned macOS arm64 ORT artifact without fetching or writing."""
    if normalized_system != "Darwin" or architecture != "arm64":
        return Check(
            "ONNX Runtime native library",
            "NOT VERIFIED",
            (
                "This artifact check covers only the pinned macOS arm64 library selected for this task.",
                f"Detected host: {normalized_system or 'unknown'} {architecture}.",
                "A platform-specific native library verifier is required before configure readiness can be reported.",
                "Blocks native configuration.",
            ),
            True,
        )
    if source_dir is None:
        return Check(
            "ONNX Runtime native library",
            "NOT VERIFIED",
            (
                "Cannot inspect the platform library until a local Moonshine source directory is available.",
                f"Required path in the pinned source: {ONNX_RUNTIME_RELATIVE_PATH}",
                "Blocks native configuration.",
            ),
            True,
        )
    if revision_check.status != "VERIFIED":
        return Check(
            "ONNX Runtime native library",
            "NOT VERIFIED",
            (
                "The source revision is not verified, so its ONNX Runtime pointer metadata cannot be trusted.",
                f"Expected path in the pinned source: {ONNX_RUNTIME_RELATIVE_PATH}",
                "Blocks native configuration.",
            ),
            True,
        )

    try:
        result = run(
            [
                "git",
                "-C",
                str(source_dir),
                "show",
                f"HEAD:{ONNX_RUNTIME_RELATIVE_PATH.as_posix()}",
            ],
            capture_output=True,
            text=True,
            timeout=COMMAND_TIMEOUT_SECONDS,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        result = None
    if result is None or result.returncode != 0:
        return Check(
            "ONNX Runtime native library",
            "NOT VERIFIED",
            (
                f"Could not read the tracked LFS metadata at {ONNX_RUNTIME_RELATIVE_PATH}.",
                "The preflight reads Git metadata only; it does not fetch or materialize LFS content.",
                "Blocks native configuration.",
            ),
            True,
        )

    return verify_native_artifact(
        source_dir / ONNX_RUNTIME_RELATIVE_PATH,
        result.stdout or "",
        expected_architecture=ONNX_RUNTIME_EXPECTED_ARCHITECTURE,
    )


def _model_artifact_check(
    manifest_path: Path,
    asset_root: Path,
) -> tuple[Check, Check]:
    """Inspect declared model/tokenizer paths without hashing or promoting them."""
    manifest_status = Check(
        "Local model manifest",
        "NOT CONFIGURED",
        (
            "The repository has a development asset inventory, but no T-013 runtime model manifest.",
            "Required for later model loading and checksum verification; not required for CMake configuration.",
            f"Inventory checked: {manifest_path}",
            f"Verify inventory locally: python3 scripts/validate_model_asset_inventory.py --manifest {manifest_path} --asset-root {asset_root}",
            "A production manifest and model artifacts are required for model inference, not for native configuration.",
        ),
    )

    try:
        data = inventory.load_manifest(manifest_path)
    except inventory.InventoryValidationError as error:
        return (
            manifest_status,
            Check(
                "Model artifacts",
                "NOT ASSESSED",
                (
                    f"The development inventory could not be inspected: {error.errors[0]}.",
                    f"Checked inventory: {manifest_path}",
                    "Required for model inference only; this does not block native CMake configuration.",
                ),
            ),
        )

    if not isinstance(data, dict) or not isinstance(data.get("assets"), list):
        return (
            manifest_status,
            Check(
                "Model artifacts",
                "NOT ASSESSED",
                (
                    "The development inventory does not contain a readable assets list.",
                    f"Checked inventory: {manifest_path}",
                    "Required for model inference only; this does not block native CMake configuration.",
                ),
            ),
        )

    model_assets = [
        asset
        for asset in data["assets"]
        if isinstance(asset, dict) and asset.get("category") in {"model", "tokenizer"}
    ]
    declared: list[tuple[str, Path | None]] = []
    undeclared = 0
    for asset in model_assets:
        relative_path = asset.get("file_path")
        if relative_path is None:
            undeclared += 1
            continue
        if not inventory._safe_relative_path(relative_path):
            declared.append((str(relative_path), None))
            continue
        resolved_root = inventory._asset_root_path(asset_root)
        if resolved_root is None:
            declared.append((str(relative_path), None))
            continue
        declared.append(
            (
                str(relative_path),
                inventory._contained_regular_file(relative_path, resolved_root),
            )
        )

    missing = [name for name, path in declared if path is None]
    if missing:
        details = [
            f"Declared model/tokenizer file is missing, unsafe, or not a regular file: {name}"
            for name in missing
        ]
        details.extend(
            (
                f"Checked asset root: {asset_root}",
                f"Verify locally: python3 scripts/validate_model_asset_inventory.py --manifest {manifest_path} --asset-root {asset_root}",
                "Blocks model inference/packaging only, not native CMake configuration.",
                "Presence checks do not verify checksums, source provenance, licensing, or redistribution rights.",
            )
        )
        artifact_check = Check("Model artifacts", "MISSING", tuple(details))
    elif not declared or undeclared:
        artifact_check = Check(
            "Model artifacts",
            "NOT ASSESSED",
            (
                f"{undeclared or len(model_assets)} model/tokenizer inventory entries have no local file path.",
                f"Checked inventory: {manifest_path}",
                "Required for model inference only; this does not block native CMake configuration.",
            ),
        )
    else:
        artifact_check = Check(
            "Model artifacts",
            "PRESENT",
            (
                f"All {len(declared)} declared model/tokenizer paths exist as local regular files under {asset_root}.",
                "Presence checks do not verify checksums, source provenance, licensing, or redistribution rights.",
                "Required for later model inference only; not required for native CMake configuration.",
            ),
        )
    return manifest_status, artifact_check


def inspect_preflight(
    *,
    repository_root: Path,
    source_dir: str | os.PathLike[str] | None = None,
    asset_inventory_path: str | os.PathLike[str] | None = None,
    asset_root: str | os.PathLike[str] | None = None,
    system: str | None = None,
    machine: str | None = None,
    environment: Mapping[str, str] | None = None,
    which: CompilerLocator = shutil.which,
    run: CommandRunner = subprocess.run,
) -> PreflightReport:
    """Collect deterministic local readiness checks through injectable adapters."""
    repo_root = repository_root.resolve()
    system_value = platform.system() if system is None else system
    machine_value = platform.machine() if machine is None else machine
    host_label, normalized_system, host_architecture, host_supported = _normalized_host(system_value, machine_value)

    host_check = Check(
        "Host OS and architecture",
        "DETECTED" if host_supported else "UNSUPPORTED",
        (
            f"Detected: {host_label}",
            "Supported project build targets are macOS arm64/x64, Windows x64, and Linux x64." if host_supported else "This host/architecture is outside the project's documented native build targets.",
            "Verify locally: python3 -c \"import platform; print(platform.system(), platform.machine())\"",
            "The target matrix remains subject to its separate platform and release gates." if host_supported else "An unsupported host blocks native configuration in this task.",
        ),
        not host_supported,
    )

    resolved_source, source_check = _source_directory(source_dir)
    expected_revision = _expected_source_revision(repo_root)
    revision_check = _source_revision_check(resolved_source, expected_revision, run)
    onnx_runtime_check = _onnx_runtime_check(
        resolved_source,
        normalized_system=normalized_system,
        architecture=host_architecture,
        revision_check=revision_check,
        run=run,
    )

    if resolved_source is None:
        header_check = Check(
            "Moonshine C API header",
            "NOT VERIFIED",
            (
                "Cannot inspect the C API header until a local Moonshine source directory is available.",
                "Required to compile the native C ABI consumer.",
                "Verify locally: rg -l 'MOONSHINE_HEADER_VERSION' <source-dir> -g '*.h'",
                "Blocks native configuration/integration readiness.",
            ),
            True,
        )
    else:
        header_path, header_version, scan_error = _find_c_api_header(resolved_source)
        if header_path is not None:
            header_check = Check(
                "Moonshine C API header",
                "PRESENT",
                (
                    f"Found: {header_path} (MOONSHINE_HEADER_VERSION={header_version}).",
                    "Header presence does not prove compatibility with a compiled runtime library.",
                ),
            )
        elif scan_error:
            header_check = Check(
                "Moonshine C API header",
                "NOT VERIFIED",
                (
                    f"Could not inspect every source directory under {resolved_source}.",
                    "Required to compile the native C ABI consumer.",
                    "Verify locally: rg -l 'MOONSHINE_HEADER_VERSION' <source-dir> -g '*.h'",
                    "Blocks native configuration/integration readiness.",
                ),
                True,
            )
        else:
            header_check = Check(
                "Moonshine C API header",
                "MISSING",
                (
                    f"Searched {resolved_source} for a C/C++ header declaring MOONSHINE_HEADER_VERSION and moonshine_load_transcriber_from_files.",
                    "Required to compile the native C ABI consumer.",
                    "Verify locally: rg -l 'MOONSHINE_HEADER_VERSION' <source-dir> -g '*.h'",
                    "Blocks native configuration/integration readiness.",
                ),
                True,
            )

    cmake_executable = which("cmake")
    if cmake_executable is None:
        cmake_check = Check(
            "CMake executable",
            "MISSING",
            (
                "Checked PATH for the `cmake` executable.",
                "Required to configure the local native project.",
                "Verify locally: command -v cmake",
                "Blocks native configuration.",
            ),
            True,
        )
        version_check = Check(
            "CMake version",
            "CANNOT DETERMINE",
            (
                "No CMake executable is available to query.",
                "A detected version must satisfy the minimum in the pinned source before configuration.",
                "Verify locally: cmake --version",
                "Blocks native configuration.",
            ),
            True,
        )
    else:
        cmake_check = Check(
            "CMake executable",
            "AVAILABLE",
            (f"Found on PATH: {cmake_executable}", "No configure command was run."),
        )
        cmake_version, cmake_error = _cmake_version(cmake_executable, run)
        if cmake_version is None:
            version_check = Check(
                "CMake version",
                "CANNOT DETERMINE",
                (
                    f"Could not parse a successful version response from {cmake_executable}: {cmake_error}.",
                    "A detected version must satisfy the minimum in the pinned source before configuration.",
                    f"Verify locally: {cmake_executable} --version",
                    "Blocks native configuration.",
                ),
                True,
            )
        else:
            minimum_version = (
                _minimum_cmake_version(resolved_source)
                if revision_check.status == "VERIFIED"
                else None
            )
            compatibility_detail = (
                f"Pinned source minimum: CMake {minimum_version}."
                if minimum_version
                else (
                    "Pinned source minimum could not be parsed from core/CMakeLists.txt."
                    if revision_check.status == "VERIFIED"
                    else "Pinned source minimum cannot be assessed until the exact clean checkout is verified."
                )
            )
            incompatible = minimum_version is not None and not _version_at_least(
                cmake_version, minimum_version
            )
            minimum_unavailable = revision_check.status == "VERIFIED" and minimum_version is None
            version_check = Check(
                "CMake version",
                "DETECTED",
                (
                    f"Detected: CMake {cmake_version} at {cmake_executable}.",
                    compatibility_detail,
                    "Version detection does not prove that CMake can configure this source.",
                    *(('Detected version is below the pinned source minimum and blocks configuration.',) if incompatible else ()),
                    *(('Verify locally: inspect the pinned core/CMakeLists.txt minimum and run cmake --version.',) if minimum_unavailable or incompatible else ()),
                ),
                incompatible or minimum_unavailable,
            )

    env = os.environ if environment is None else environment
    windows_host = normalized_system == "Windows"
    c_candidates = ("cl", "clang-cl", "clang", "gcc", "cc") if windows_host else ("cc", "clang", "gcc")
    cxx_candidates = ("cl", "clang-cl", "clang++", "g++", "c++") if windows_host else ("c++", "clang++", "g++")
    c_path, c_name, c_error = _compiler_command("CC", c_candidates, env, which)
    cxx_path, cxx_name, cxx_error = _compiler_command("CXX", cxx_candidates, env, which)
    if c_path is not None and cxx_path is not None:
        compiler_check = Check(
            "C/C++ compiler",
            "AVAILABLE",
            (
                f"C compiler: {c_name} ({c_path}).",
                f"C++ compiler: {cxx_name} ({cxx_path}).",
                "Executable presence is detected; compiler compatibility and CMake selection are not verified.",
            ),
        )
    else:
        missing_details = []
        if c_path is None:
            missing_details.append(f"C compiler missing: {c_error}; checked {c_name}.")
        if cxx_path is None:
            missing_details.append(f"C++ compiler missing: {cxx_error}; checked {cxx_name}.")
        missing_details.extend(
            (
                "Required by Moonshine's C/C++ native targets.",
                "Verify locally: command -v cc && command -v c++ (or check the configured CC/CXX commands).",
                "Blocks native configuration.",
            )
        )
        compiler_check = Check("C/C++ compiler", "MISSING", tuple(missing_details), True)

    default_inventory = repo_root / "docs" / "model-asset-inventory.json"
    manifest_path = Path(asset_inventory_path).expanduser() if asset_inventory_path is not None else default_inventory
    root_path = Path(asset_root).expanduser() if asset_root is not None else repo_root
    manifest_check, artifacts_check = _model_artifact_check(manifest_path, root_path)

    ordered_checks = (
        host_check,
        cmake_check,
        version_check,
        compiler_check,
        source_check,
        header_check,
        revision_check,
        onnx_runtime_check,
        manifest_check,
        artifacts_check,
    )
    ready = not any(check.blocks_configure for check in ordered_checks)
    readiness_check = Check(
        "Offline configuration readiness",
        "READY_TO_ATTEMPT_CONFIGURE" if ready else "BLOCKED",
        (
            "Minimum local prerequisites appear available; the next step is a separate controlled CMake configure attempt." if ready else "One or more prerequisites needed for native configuration are missing or unverifiable.",
            "Readiness does not prove successful configuration, compilation, ABI linkage, inference, model integrity/license, CPU-only behavior, accuracy, size, or memory compliance.",
        ),
    )
    return PreflightReport(ordered_checks + (readiness_check,), ready)


def render_report(report: PreflightReport) -> str:
    """Render checks in a stable order with reasons and local verification steps."""
    lines = ["Moonshine native build readiness", ""]
    for check in report.checks:
        lines.append(f"{check.name}: {check.status}")
        for detail in check.details:
            lines.append(f"  {detail}")
    return "\n".join(lines)


def _argument_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Check local offline prerequisites for a Moonshine native CMake configure. "
            "This command never configures, builds, downloads, or installs."
        )
    )
    parser.add_argument(
        "--source-dir",
        type=Path,
        help="existing local Moonshine checkout; must match the repository pin and be clean",
    )
    parser.add_argument(
        "--asset-inventory",
        type=Path,
        help="existing development asset inventory to inspect (default: docs/model-asset-inventory.json)",
    )
    parser.add_argument(
        "--asset-root",
        type=Path,
        help="root for declared local model/tokenizer paths (default: repository root)",
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = _argument_parser().parse_args(argv)
    repository_root = Path(__file__).resolve().parents[1]
    report = inspect_preflight(
        repository_root=repository_root,
        source_dir=args.source_dir,
        asset_inventory_path=args.asset_inventory,
        asset_root=args.asset_root,
    )
    print(render_report(report))
    return 0 if report.ready else 2


if __name__ == "__main__":
    raise SystemExit(main())
