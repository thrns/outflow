#!/usr/bin/env python3
"""Attempt an isolated configure of the pinned Moonshine CMake source.

This runner verifies the upstream revision and the existing six-file source
contract before configuring. CMake writes only to a new, retained, out-of-source
directory so its cache and diagnostics remain inspectable. It never invokes a
build, install, or test command and does not certify a minimal STT binary.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass, field
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import stat
import subprocess
from typing import Sequence

try:  # Direct script execution and package-based unittest imports.
    from . import probe_moonshine_cmake as probe
except ImportError:  # pragma: no cover - exercised by direct CLI execution.
    import probe_moonshine_cmake as probe


SOURCE_REVISION = probe.SOURCE_REVISION
CONFIGURE_TIMEOUT_SECONDS = 300
VERSION_TIMEOUT_SECONDS = 10
MAX_DIAGNOSTIC_CHARS = 12_000
CONFIGURE_DEFINITIONS = (
    "-DCMAKE_BUILD_TYPE=Release",
    "-DMOONSHINE_TTS_BUILD_ONNX=OFF",
    "-DMOONSHINE_RELIABILITY=OFF",
    "-DFETCHCONTENT_FULLY_DISCONNECTED=ON",
    "-DFETCHCONTENT_UPDATES_DISCONNECTED=ON",
)
BLOCKED_EXIT_CODE = 2
FAILED_EXIT_CODE = 1

_MINIMUM_CMAKE = re.compile(
    r"(?im)^\s*cmake_minimum_required\s*\(\s*VERSION\s+([0-9]+(?:\.[0-9]+){1,2})"
)
_VERSION_LINE = re.compile(r"(?im)^cmake version\s+([0-9]+(?:\.[0-9]+){1,2})")
_CACHE_LINE = re.compile(r"^([^#/][^=]*?):[^=]*=(.*)$")
_DEPENDENCY_DIAGNOSTICS = (
    "could not find",
    "couldn't find",
    "could not locate",
    "not found",
    "no cmake_c_compiler could be found",
    "no cmake_cxx_compiler could be found",
    "cmake_c_compiler not set",
    "cmake_cxx_compiler not set",
    "cmake_c_compiler is not set",
    "cmake_cxx_compiler is not set",
    "cmake_make_program is not set",
    "unable to find a build program",
    "compiler is not able to compile a simple test program",
    "permission denied",
    "no such file or directory",
    "does not exist",
    "does not appear to contain",
    "missing dependency",
    "no content source",
    "no url specified",
)
_TARGET_NAME_HINTS = {
    "tts": lambda name: "tts" in name.casefold() or "g2p" in name.casefold(),
    "test": lambda name: "test" in name.casefold(),
    "benchmark": lambda name: "bench" in name.casefold(),
}


@dataclass
class ExperimentResult:
    """Observed state from one configure attempt."""

    status: str
    source_dir: str
    source_revision: str | None = None
    source_integrity: str = "NOT VERIFIED"
    source_unchanged: str = "NOT CHECKED"
    host: str = field(default_factory=lambda: f"{platform.system()} {platform.machine()}")
    cmake_version: str | None = None
    minimum_cmake_version: str | None = None
    configure_arguments: tuple[str, ...] = ()
    build_directory: str | None = None
    configure_return_code: int | None = None
    configure_attempted: bool = False
    configure_status: str | None = None
    blocker: str | None = None
    diagnostics: str = ""
    cache_values: dict[str, str] = field(default_factory=dict)
    generator: str | None = None
    targets: list[dict[str, str]] = field(default_factory=list)
    target_graph_status: str = "NOT AVAILABLE"
    onnx_library_path: str = "NOT SELECTED"
    onnx_library_state: str = "NOT INSPECTED"
    network_isolation: str = (
        "UNVERIFIED (FETCHCONTENT_FULLY_DISCONNECTED does not disable all configure-time network access)"
    )


def _git_output(source_dir: Path, arguments: Sequence[str]) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "-C", str(source_dir), *arguments],
        capture_output=True,
        text=True,
        timeout=VERSION_TIMEOUT_SECONDS,
        check=False,
    )


def _read_git_revision(source_dir: Path) -> tuple[str | None, str | None]:
    try:
        result = _git_output(source_dir, ("rev-parse", "HEAD"))
    except (OSError, subprocess.TimeoutExpired) as exc:
        return None, f"Could not inspect the source Git revision: {exc}"
    if result.returncode != 0:
        return None, _diagnostic_text(result.stderr or result.stdout) or "Git revision check failed"
    revision = result.stdout.strip()
    if not re.fullmatch(r"[0-9a-fA-F]{40}", revision):
        return None, f"Git returned an invalid HEAD revision: {revision or '(empty)'}"
    return revision.lower(), None


def _read_git_status(source_dir: Path) -> tuple[str | None, str | None]:
    try:
        result = _git_output(source_dir, ("status", "--porcelain", "--untracked-files=all"))
    except (OSError, subprocess.TimeoutExpired) as exc:
        return None, f"Could not inspect source checkout status: {exc}"
    if result.returncode != 0:
        return None, _diagnostic_text(result.stderr or result.stdout) or "Git status check failed"
    return result.stdout, None


def _diagnostic_text(value: str | bytes | None) -> str:
    if value is None:
        return ""
    if isinstance(value, bytes):
        value = value.decode("utf-8", errors="replace")
    return value.strip()


def source_tree_fingerprint(source_dir: Path) -> str:
    """Hash every checkout entry except Git internals, including ignored files.

    Git status is checked separately for index/worktree state. This walk also
    detects ignored configure-time writes that Git would not report.
    """
    root = source_dir.resolve(strict=True)
    digest = hashlib.sha256()
    pending = [root]
    while pending:
        directory = pending.pop()
        with os.scandir(directory) as scanner:
            entries = sorted(scanner, key=lambda entry: entry.name)
        for entry in entries:
            if directory == root and entry.name == ".git":
                continue
            path = Path(entry.path)
            relative = path.relative_to(root).as_posix().encode("utf-8", errors="surrogateescape")
            info = entry.stat(follow_symlinks=False)
            mode = stat.S_IFMT(info.st_mode) | stat.S_IMODE(info.st_mode)
            digest.update(relative + b"\0" + str(mode).encode("ascii") + b"\0")
            if stat.S_ISLNK(info.st_mode):
                digest.update(b"symlink\0" + os.readlink(path).encode("utf-8", errors="surrogateescape"))
            elif stat.S_ISDIR(info.st_mode):
                digest.update(b"directory\0")
                pending.append(path)
            elif stat.S_ISREG(info.st_mode):
                digest.update(b"file\0")
                with path.open("rb") as source_file:
                    for chunk in iter(lambda: source_file.read(1024 * 1024), b""):
                        digest.update(chunk)
            else:
                digest.update(f"special:{info.st_size}\0".encode("ascii"))
    return digest.hexdigest()


def _minimum_cmake_version(core_cmake: bytes) -> str | None:
    text = core_cmake.decode("utf-8", errors="replace")
    match = _MINIMUM_CMAKE.search(text)
    return match.group(1) if match else None


def _version_tuple(version: str) -> tuple[int, ...]:
    return tuple(int(piece) for piece in version.split("."))


def _version_at_least(actual: str, minimum: str) -> bool:
    actual_parts = _version_tuple(actual)
    minimum_parts = _version_tuple(minimum)
    width = max(len(actual_parts), len(minimum_parts))
    return actual_parts + (0,) * (width - len(actual_parts)) >= minimum_parts + (0,) * (
        width - len(minimum_parts)
    )


def _check_cmake(minimum_version: str) -> tuple[str | None, str | None, str | None]:
    executable = shutil.which("cmake")
    if executable is None:
        return None, None, "CMake executable is not available on PATH"
    try:
        result = subprocess.run(
            [executable, "--version"],
            capture_output=True,
            text=True,
            timeout=VERSION_TIMEOUT_SECONDS,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        return executable, None, f"Could not query CMake version: {exc}"
    if result.returncode != 0:
        return executable, None, _diagnostic_text(result.stderr or result.stdout) or "CMake version query failed"
    output = result.stdout + "\n" + result.stderr
    match = _VERSION_LINE.search(output)
    if match is None:
        return executable, None, "Could not parse CMake --version output"
    version = match.group(1)
    if not _version_at_least(version, minimum_version):
        return executable, version, f"CMake {version} is below upstream minimum {minimum_version}"
    return executable, version, None


def _cache_values(cache_path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    try:
        lines = cache_path.read_text(encoding="utf-8", errors="replace").splitlines()
    except OSError:
        return values
    for line in lines:
        match = _CACHE_LINE.match(line)
        if match:
            values[match.group(1)] = match.group(2)
    return values


def _target_graph(
    build_dir: Path,
    preferred_configuration: str = "Release",
) -> tuple[list[dict[str, str]], str]:
    reply_dir = build_dir / ".cmake" / "api" / "v1" / "reply"
    indexes = sorted(reply_dir.glob("index-*.json"))
    if not indexes:
        return [], "UNAVAILABLE (CMake File API reply not generated)"
    try:
        index = json.loads(indexes[-1].read_text(encoding="utf-8"))
        codemodel_ref = index.get("reply", {}).get("codemodel-v2", {})
        codemodel_path = (reply_dir / codemodel_ref["jsonFile"]).resolve(strict=True)
        if reply_dir.resolve() not in codemodel_path.parents:
            return [], "UNAVAILABLE (File API path escaped the build directory)"
        codemodel = json.loads(codemodel_path.read_text(encoding="utf-8"))
        configurations = codemodel.get("configurations", [])
        if not configurations:
            return [], "UNAVAILABLE (File API reported no configurations)"
        configuration = next(
            (
                item
                for item in configurations
                if str(item.get("name", "")).casefold() == preferred_configuration.casefold()
            ),
            configurations[0],
        )
        targets: list[dict[str, str]] = []
        for target in configuration.get("targets", []):
            name = str(target.get("name", ""))
            target_type = target.get("type")
            target_file = target.get("jsonFile")
            if not target_type and target_file:
                target_path = (reply_dir / str(target_file)).resolve(strict=True)
                if reply_dir.resolve() not in target_path.parents:
                    return [], "UNAVAILABLE (File API target path escaped the build directory)"
                target_data = json.loads(target_path.read_text(encoding="utf-8"))
                target_type = target_data.get("type", "UNKNOWN")
            targets.append({"name": name, "type": str(target_type or "UNKNOWN")})
        return sorted(targets, key=lambda target: target["name"].casefold()), "INSPECTED"
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as exc:
        return [], f"UNAVAILABLE (could not read CMake File API graph: {exc})"


def _binary_format(path: Path) -> str:
    """Identify common native-library binary containers without trusting pointers."""
    try:
        with path.open("rb") as library:
            header = library.read(4096)
            if header.startswith(b"version https://git-lfs.github.com/spec/v1"):
                return "LFS POINTER (not a usable native library)"
            if header.startswith(b"!<arch>\n"):
                return "ARCHIVE BINARY"
            if header.startswith(b"\x7fELF"):
                return "ELF BINARY"
            if header[:4] in {
                b"\xfe\xed\xfa\xce",
                b"\xce\xfa\xed\xfe",
                b"\xfe\xed\xfa\xcf",
                b"\xcf\xfa\xed\xfe",
                b"\xca\xfe\xba\xbe",
                b"\xbe\xba\xfe\xca",
                b"\xca\xfe\xba\xbf",
                b"\xbf\xba\xfe\xca",
            }:
                return "MACH-O BINARY"
            if header.startswith(b"MZ") and len(header) >= 64:
                pe_offset = int.from_bytes(header[60:64], "little")
                if pe_offset + 4 <= len(header) and header[pe_offset : pe_offset + 4] == b"PE\0\0":
                    return "PE BINARY"
            return "PRESENT (binary format not recognized; not verified)"
    except OSError as exc:
        return f"UNREADABLE ({exc})"


def _inspect_onnx_library(raw_path: str, source_core: Path) -> tuple[str, str]:
    if not raw_path:
        return "NOT SELECTED IN CMAKE CACHE", "NOT INSPECTED"
    candidate = Path(raw_path).expanduser()
    if not candidate.is_absolute():
        candidate = source_core / candidate
    candidate = candidate.resolve(strict=False)
    if not candidate.is_file():
        return str(candidate), "MISSING (selected path is not a file)"
    return str(candidate), _binary_format(candidate)


def _configure_status(returncode: int, output: str) -> tuple[str, str | None]:
    if returncode == 0:
        return "CONFIGURED", None
    lowered = output.casefold()
    if any(marker in lowered for marker in _DEPENDENCY_DIAGNOSTICS):
        return "BLOCKED", "CMake configuration reported a missing dependency or required file"
    return "FAILED", "CMake configuration returned a nonzero status"


def _set_diagnostics(result: ExperimentResult, stdout: str | bytes | None, stderr: str | bytes | None) -> None:
    combined = "\n".join(part for part in (_diagnostic_text(stdout), _diagnostic_text(stderr)) if part)
    result.diagnostics = combined[-MAX_DIAGNOSTIC_CHARS:]


def _create_build_directory(source_root: Path, requested: str | Path | None) -> tuple[Path | None, str | None]:
    """Create one new, retained build directory outside both source trees."""
    if requested is None:
        return None, "A new dedicated --build-dir is required so generated files remain inspectable"

    provided = Path(requested).expanduser()
    if os.path.lexists(provided):
        return None, f"Build directory already exists; refusing to reuse or overwrite it: {provided}"
    try:
        resolved = provided.resolve(strict=False)
    except OSError as exc:
        return None, f"Could not resolve the requested build directory: {exc}"

    if not resolved.parent.is_dir():
        return None, f"Build directory parent must already exist: {resolved.parent}"

    outflow_root = Path(__file__).resolve().parents[1]
    for label, root in (("Moonshine source", source_root), ("Outflow repository", outflow_root)):
        if resolved == root or resolved in root.parents or root in resolved.parents:
            return None, f"Build directory must be outside the {label} tree: {resolved}"

    try:
        resolved.mkdir()
    except OSError as exc:
        return None, f"Could not create the requested build directory: {exc}"
    return resolved, None


def _selection_arguments(
    *,
    generator: str | None,
    c_compiler: str | None,
    cxx_compiler: str | None,
    osx_architectures: str | None,
) -> tuple[str, ...]:
    arguments: list[str] = []
    if generator is not None:
        arguments.extend(("-G", generator))
    if c_compiler is not None:
        arguments.append(f"-DCMAKE_C_COMPILER={c_compiler}")
    if cxx_compiler is not None:
        arguments.append(f"-DCMAKE_CXX_COMPILER={cxx_compiler}")
    if osx_architectures is not None:
        arguments.append(f"-DCMAKE_OSX_ARCHITECTURES={osx_architectures}")
    return tuple(arguments)


def run_experiment(
    source_dir: str | Path,
    build_dir: str | Path | None = None,
    *,
    generator: str | None = None,
    c_compiler: str | None = None,
    cxx_compiler: str | None = None,
    osx_architectures: str | None = None,
) -> ExperimentResult:
    """Verify and configure one pinned checkout into a new retained build tree."""
    provided = Path(source_dir).expanduser()
    try:
        source_root = provided.resolve(strict=True)
    except OSError as exc:
        return ExperimentResult("BLOCKED", str(provided), blocker=f"Source directory is unavailable: {exc}")
    if not source_root.is_dir():
        return ExperimentResult("BLOCKED", str(source_root), blocker="Source root must be a directory")

    result = ExperimentResult("BLOCKED", str(source_root))
    revision, git_error = _read_git_revision(source_root)
    if git_error:
        result.blocker = f"Source checkout is not available as a Git repository: {git_error}"
        return result
    result.source_revision = revision
    if revision != SOURCE_REVISION:
        result.blocker = f"Source revision must be {SOURCE_REVISION}; found {revision}"
        return result

    initial_status, git_error = _read_git_status(source_root)
    if git_error:
        result.blocker = f"Could not verify a clean source checkout: {git_error}"
        return result
    if initial_status:
        result.blocker = "Source checkout is dirty; configure requires an unchanged pinned worktree"
        result.diagnostics = initial_status.strip()[-MAX_DIAGNOSTIC_CHARS:]
        return result

    try:
        contents = probe.read_local_sources(source_root)
    except probe.ProbeError as exc:
        result.blocker = f"Pinned CMake source files are unavailable: {exc}"
        return result
    report, source_errors = probe.verify_sources(contents)
    if report is None or source_errors:
        result.status = "FAILED"
        result.source_integrity = "FAILED"
        result.blocker = "The six pinned CMake files failed the existing Task 24 integrity/contract probe"
        result.diagnostics = "\n".join(source_errors)[-MAX_DIAGNOSTIC_CHARS:]
        return result
    result.source_integrity = "VERIFIED"

    minimum_version = _minimum_cmake_version(contents["core/CMakeLists.txt"])
    if minimum_version is None:
        result.status = "FAILED"
        result.blocker = "Pinned core/CMakeLists.txt has no parseable cmake_minimum_required(VERSION ...)"
        return result
    result.minimum_cmake_version = minimum_version

    try:
        initial_fingerprint = source_tree_fingerprint(source_root)
    except OSError as exc:
        result.status = "BLOCKED"
        result.blocker = f"Could not snapshot the source checkout before configure: {exc}"
        return result

    cmake, cmake_version, cmake_error = _check_cmake(minimum_version)
    result.cmake_version = cmake_version
    if cmake_error:
        result.status = "BLOCKED"
        result.blocker = cmake_error
        return result
    if cmake is None or cmake_version is None:
        result.status = "BLOCKED"
        result.blocker = "CMake prerequisite check did not return an executable and version"
        return result

    created_build_dir, build_error = _create_build_directory(source_root, build_dir)
    if build_error:
        result.status = "BLOCKED"
        result.blocker = build_error
    elif created_build_dir is not None:
        result.build_directory = str(created_build_dir)
        try:
            query_dir = created_build_dir / ".cmake" / "api" / "v1" / "query"
            query_dir.mkdir(parents=True)
            (query_dir / "codemodel-v2").write_text("", encoding="utf-8")

            command = (
                cmake,
                "-S",
                str(source_root / "core"),
                "-B",
                str(created_build_dir),
                *_selection_arguments(
                    generator=generator,
                    c_compiler=c_compiler,
                    cxx_compiler=cxx_compiler,
                    osx_architectures=osx_architectures,
                ),
                *CONFIGURE_DEFINITIONS,
            )
            result.configure_arguments = tuple(command[1:])
            result.configure_attempted = True
            try:
                configured = subprocess.run(
                    list(command),
                    cwd=created_build_dir,
                    capture_output=True,
                    text=True,
                    timeout=CONFIGURE_TIMEOUT_SECONDS,
                    check=False,
                )
            except subprocess.TimeoutExpired as exc:
                result.status = "BLOCKED"
                result.configure_status = "BLOCKED"
                result.blocker = f"CMake configuration exceeded {CONFIGURE_TIMEOUT_SECONDS} seconds"
                _set_diagnostics(result, exc.stdout, exc.stderr)
            except OSError as exc:
                result.status = "BLOCKED"
                result.configure_status = "BLOCKED"
                result.blocker = f"Could not start CMake configuration: {exc}"
            else:
                result.configure_return_code = configured.returncode
                _set_diagnostics(result, configured.stdout, configured.stderr)
                output = "\n".join((configured.stdout or "", configured.stderr or ""))
                result.configure_status, result.blocker = _configure_status(configured.returncode, output)
                result.status = result.configure_status
                if result.configure_status == "CONFIGURED":
                    result.cache_values = _cache_values(created_build_dir / "CMakeCache.txt")
                    result.generator = result.cache_values.get("CMAKE_GENERATOR")
                    preferred_configuration = result.cache_values.get("CMAKE_BUILD_TYPE") or "Release"
                    result.targets, result.target_graph_status = _target_graph(
                        created_build_dir,
                        preferred_configuration,
                    )
                    raw_ort_path = result.cache_values.get("ONNXRUNTIME_LIB_PATH", "")
                    result.onnx_library_path, result.onnx_library_state = _inspect_onnx_library(
                        raw_ort_path,
                        source_root / "core",
                    )
        except OSError as exc:
            result.status = "BLOCKED"
            result.blocker = f"Could not create or inspect the retained build directory: {exc}"

    try:
        final_fingerprint = source_tree_fingerprint(source_root)
        final_revision, final_git_error = _read_git_revision(source_root)
        final_status, status_error = _read_git_status(source_root)
        unchanged = (
            final_fingerprint == initial_fingerprint
            and final_revision == SOURCE_REVISION
            and final_git_error is None
            and status_error is None
            and final_status == initial_status
        )
    except OSError as exc:
        unchanged = False
        final_git_error = str(exc)
        status_error = None
        final_status = None
    result.source_unchanged = "VERIFIED" if unchanged else "FAILED"
    if not unchanged:
        result.status = "FAILED"
        result.blocker = "Source checkout changed during the experiment or its final state could not be verified"
        extra = "\n".join(part for part in (final_git_error, status_error, final_status) if part)
        if extra:
            result.diagnostics = (result.diagnostics + "\n" + extra)[-MAX_DIAGNOSTIC_CHARS:]
    return result


def _format_targets(targets: Sequence[dict[str, str]]) -> str:
    return ", ".join(f"{item['name']} ({item['type']})" for item in targets) or "NONE"


def render_result(result: ExperimentResult) -> str:
    """Render observed facts while keeping later proof stages explicitly open."""
    lines = [
        "Outflow Moonshine CMake configure experiment",
        f"Host: {result.host}",
        f"Source integrity: {result.source_integrity}",
        f"Source revision: {result.source_revision or 'NOT VERIFIED'}",
        f"CMake minimum required by upstream: {result.minimum_cmake_version or 'NOT READ'}",
        f"CMake version: {result.cmake_version or 'NOT AVAILABLE'}",
        f"Build directory: {result.build_directory or 'NOT CREATED'}" + (" (retained)" if result.build_directory else ""),
        f"Source checkout unchanged: {result.source_unchanged}",
        f"CMake configuration: {result.configure_status or 'NOT RUN'}",
        f"Experiment outcome: {result.status}",
        f"CMake return code: {result.configure_return_code if result.configure_return_code is not None else 'NOT AVAILABLE'}",
        f"Network isolation: {result.network_isolation}",
        "Moonshine compilation: NOT RUN",
        "Binary contents: NOT VERIFIED",
        "Minimal STT build: NOT PROVEN",
        "Runtime compatibility: NOT ASSESSED",
    ]
    if result.configure_arguments:
        lines.append("Configuration arguments:")
        lines.extend(f"  {argument}" for argument in result.configure_arguments)
    if result.configure_status == "CONFIGURED":
        lines.extend(
            (
                f"CMake generator: {result.generator or 'NOT REPORTED'}",
                "Configured cache values:",
            )
        )
        for name in (
            "CMAKE_BUILD_TYPE",
            "CMAKE_C_COMPILER",
            "CMAKE_CXX_COMPILER",
            "MOONSHINE_TTS_BUILD_ONNX",
            "MOONSHINE_RELIABILITY",
            "FETCHCONTENT_FULLY_DISCONNECTED",
            "FETCHCONTENT_UPDATES_DISCONNECTED",
            "CMAKE_OSX_ARCHITECTURES",
        ):
            lines.append(f"  {name}: {result.cache_values.get(name, 'NOT SET IN CACHE')}")
        moonshine = [item for item in result.targets if item["name"].casefold() == "moonshine"]
        lines.append(f"moonshine target defined: {_format_targets(moonshine)}")
        for label, predicate in _TARGET_NAME_HINTS.items():
            matching = [item for item in result.targets if predicate(item["name"])]
            lines.append(f"{label.capitalize()}-related targets defined: {_format_targets(matching)}")
        lines.append(f"Target graph: {result.target_graph_status}")
        lines.append(f"Selected ONNX Runtime library path: {result.onnx_library_path}")
        lines.append(f"ONNX Runtime library status: {result.onnx_library_state}")
    if result.blocker:
        lines.append(f"Blocker: {result.blocker}")
    if result.diagnostics:
        lines.append("Relevant diagnostics:")
        lines.append(result.diagnostics)
    return "\n".join(lines)


def _argument_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Verify a pinned Moonshine checkout and configure it in a new retained out-of-source build tree; "
            "does not compile targets."
        )
    )
    parser.add_argument("--source-dir", required=True, type=Path, help="root of the pinned Moonshine checkout")
    parser.add_argument(
        "--build-dir",
        required=True,
        type=Path,
        help="new build directory to create and retain; existing paths are rejected",
    )
    parser.add_argument("--generator", help="optional CMake generator name passed with -G")
    parser.add_argument("--c-compiler", help="optional C compiler path passed to CMake")
    parser.add_argument("--cxx-compiler", help="optional C++ compiler path passed to CMake")
    parser.add_argument(
        "--osx-architectures",
        help="optional CMAKE_OSX_ARCHITECTURES value (for example, arm64 on Apple Silicon)",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _argument_parser().parse_args(argv)
    result = run_experiment(
        args.source_dir,
        args.build_dir,
        generator=args.generator,
        c_compiler=args.c_compiler,
        cxx_compiler=args.cxx_compiler,
        osx_architectures=args.osx_architectures,
    )
    print(render_result(result))
    if result.status == "CONFIGURED":
        return 0
    if result.status == "BLOCKED":
        return BLOCKED_EXIT_CODE
    return FAILED_EXIT_CODE


if __name__ == "__main__":
    raise SystemExit(main())
