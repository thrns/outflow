#!/usr/bin/env python3
"""Verify a small, pinned Moonshine CMake source contract without building it.

This probe only reads six CMake text files. Local mode is strictly offline;
upstream mode performs HTTPS GETs for those six immutable revision URLs and
keeps response bytes in memory. A successful source review does not configure,
compile, link, package, or inspect a native binary.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
import hashlib
from pathlib import Path, PurePosixPath
import re
import sys
from typing import Mapping
from urllib.error import HTTPError, URLError
from urllib.parse import urlsplit
from urllib.request import HTTPRedirectHandler, Request, build_opener


SOURCE_REVISION = "234f60faa0eb388b01cdf7e60aca232af37aefda"
UPSTREAM_BASE = "https://raw.githubusercontent.com/moonshine-ai/moonshine"
MAX_SOURCE_BYTES = 4_000_000
REQUEST_TIMEOUT_SECONDS = 20


@dataclass(frozen=True)
class PinnedSource:
    path: str
    blob_sha: str

    @property
    def url(self) -> str:
        return f"{UPSTREAM_BASE}/{SOURCE_REVISION}/{self.path}"


PINNED_SOURCES = (
    PinnedSource("core/CMakeLists.txt", "30c472029b6f219afff9df11faa03d413064834b"),
    PinnedSource(
        "core/moonshine-tts/CMakeLists.txt",
        "af834942deba237ec3e6d46cca1be4983a839fe1",
    ),
    PinnedSource(
        "core/ort-utils/CMakeLists.txt",
        "dddcabd53385365dfdef9592395498616f95dc7f",
    ),
    PinnedSource(
        "core/bin-tokenizer/CMakeLists.txt",
        "55c1dc34f03a136a8243b589b9d03b1c8c6efb77",
    ),
    PinnedSource(
        "core/third-party/onnxruntime/CMakeLists.txt",
        "a07e58a6b793771046da1fd9a93fde4dc8bc77e2",
    ),
    PinnedSource(
        "core/third-party/onnxruntime/find-ort-library-path.cmake",
        "9699d99ca60471963e1d0e12017048b62087c24d",
    ),
)
EXPECTED_BLOBS = {source.path: source.blob_sha for source in PINNED_SOURCES}


class ProbeError(Exception):
    """A local input or CMake contract check failed."""


class UpstreamRetrievalError(ProbeError):
    """Pinned upstream source could not be retrieved over verified HTTPS."""


@dataclass(frozen=True)
class CMakeCommand:
    name: str
    arguments: str
    line: int
    conditions: tuple[str, ...]


@dataclass(frozen=True)
class ProbeReport:
    verified_blobs: Mapping[str, str]
    tts_onnx_default: str
    tts_subtree: str
    diarization_in_main_target: str
    main_target_dependencies: tuple[str, ...]
    ort_library_path_linked: bool
    reliability_default: str
    sanitizer_default: str
    build_shared_default: str
    build_swift_default: str
    test_target_count: int
    utility_target_count: int


def git_blob_sha(data: bytes) -> str:
    """Return Git's SHA-1 object ID for a blob, including its header."""
    header = b"blob " + str(len(data)).encode("ascii") + b"\0"
    return hashlib.sha1(header + data).hexdigest()


def read_local_sources(source_dir: str | Path) -> dict[str, bytes]:
    """Read the six required files below a local pinned-source checkout."""
    root = Path(source_dir).expanduser()
    try:
        root_resolved = root.resolve(strict=True)
    except OSError as exc:
        raise ProbeError(f"Source directory is unavailable: {exc}") from exc
    if not root_resolved.is_dir():
        raise ProbeError("Source directory must name a directory")

    contents: dict[str, bytes] = {}
    errors: list[str] = []
    for source in PINNED_SOURCES:
        path = root_resolved.joinpath(*PurePosixPath(source.path).parts)
        try:
            resolved = path.resolve(strict=True)
            if root_resolved != resolved and root_resolved not in resolved.parents:
                errors.append(f"Pinned source path escapes source directory: {source.path}")
                continue
            if not resolved.is_file():
                errors.append(f"Pinned source path is not a regular file: {source.path}")
                continue
            contents[source.path] = resolved.read_bytes()
        except FileNotFoundError:
            errors.append(f"Missing required pinned source file: {source.path}")
        except OSError as exc:
            errors.append(f"Cannot read pinned source file {source.path}: {exc}")
    if errors:
        raise ProbeError("\n".join(errors))
    return contents


class _RejectRedirects(HTTPRedirectHandler):
    """Fail closed instead of fetching an unlisted redirect destination."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # type: ignore[no-untyped-def]
        raise URLError(f"unexpected redirect refused: {newurl}")


def fetch_upstream_sources() -> dict[str, bytes]:
    """Fetch only the six pinned CMake text files, without writing them to disk."""
    opener = build_opener(_RejectRedirects())
    contents: dict[str, bytes] = {}
    for source in PINNED_SOURCES:
        parsed = urlsplit(source.url)
        if (
            parsed.scheme != "https"
            or parsed.netloc != "raw.githubusercontent.com"
            or parsed.path != f"/moonshine-ai/moonshine/{SOURCE_REVISION}/{source.path}"
            or parsed.query
            or parsed.fragment
        ):
            raise UpstreamRetrievalError(f"Refusing non-pinned upstream URL for {source.path}")
        request = Request(
            source.url,
            headers={"User-Agent": "outflow-moonshine-cmake-probe/1"},
        )
        try:
            with opener.open(request, timeout=REQUEST_TIMEOUT_SECONDS) as response:
                if response.status != 200 or response.geturl() != source.url:
                    raise UpstreamRetrievalError(
                        f"Unexpected upstream response for {source.path}: "
                        f"HTTP {response.status}, final URL {response.geturl()}"
                    )
                data = response.read(MAX_SOURCE_BYTES + 1)
        except (HTTPError, URLError, TimeoutError, OSError) as exc:
            raise UpstreamRetrievalError(
                f"Could not retrieve pinned upstream file {source.path}: {exc}"
            ) from exc
        if len(data) > MAX_SOURCE_BYTES:
            raise UpstreamRetrievalError(
                f"Pinned upstream file exceeds the text size limit: {source.path}"
            )
        if b"\0" in data:
            raise UpstreamRetrievalError(f"Pinned upstream response is not text: {source.path}")
        contents[source.path] = data
    return contents


def verify_blob_hashes(
    contents: Mapping[str, bytes],
    expected_blobs: Mapping[str, str] = EXPECTED_BLOBS,
) -> tuple[dict[str, str], list[str]]:
    """Verify every expected source using Git's blob object format."""
    errors: list[str] = []
    verified: dict[str, str] = {}
    expected_paths = tuple(source.path for source in PINNED_SOURCES)
    for path in expected_paths:
        if path not in contents:
            errors.append(f"Missing required pinned source file: {path}")
            continue
        if path not in expected_blobs:
            errors.append(f"No expected Git blob SHA is configured for required source: {path}")
            continue
        actual = git_blob_sha(contents[path])
        expected = expected_blobs[path]
        verified[path] = actual
        if actual != expected:
            errors.append(
                f"Git blob SHA mismatch for {path}: expected {expected}, got {actual}"
            )
    unexpected = sorted(set(contents) - set(expected_paths))
    for path in unexpected:
        errors.append(f"Unexpected source supplied to pinned probe: {path}")
    return verified, errors


def _strip_line_comments(text: str) -> str:
    """Remove CMake # comments while preserving quoted # characters."""
    output: list[str] = []
    for line in text.splitlines(keepends=True):
        in_quote = False
        escaped = False
        kept: list[str] = []
        for character in line:
            if escaped:
                kept.append(character)
                escaped = False
                continue
            if character == "\\" and in_quote:
                kept.append(character)
                escaped = True
                continue
            if character == '"':
                in_quote = not in_quote
                kept.append(character)
                continue
            if character == "#" and not in_quote:
                newline = "\n" if line.endswith("\n") else ""
                kept.append(newline)
                break
            kept.append(character)
        output.extend(kept)
    return "".join(output)


_COMMAND_START = re.compile(r"(?im)^[ \t]*([A-Za-z_][A-Za-z0-9_]*)[ \t]*\(")


def parse_cmake_commands(text: str) -> list[CMakeCommand]:
    """Extract ordinary CMake command calls and their simple if nesting.

    This intentionally handles the observed CMake command style, comments,
    quoted strings, and multiline argument lists. It is not a general CMake
    interpreter or parser.
    """
    active_text = _strip_line_comments(text)
    commands: list[CMakeCommand] = []
    condition_stack: list[str] = []
    search_from = 0
    while match := _COMMAND_START.search(active_text, search_from):
        command_name = match.group(1).lower()
        opening = match.end() - 1
        in_quote = False
        escaped = False
        depth = 1
        cursor = opening + 1
        while cursor < len(active_text) and depth:
            character = active_text[cursor]
            if escaped:
                escaped = False
            elif character == "\\" and in_quote:
                escaped = True
            elif character == '"':
                in_quote = not in_quote
            elif not in_quote and character == "(":
                depth += 1
            elif not in_quote and character == ")":
                depth -= 1
            cursor += 1
        if depth:
            line = active_text.count("\n", 0, match.start()) + 1
            raise ProbeError(f"Unclosed CMake command at line {line}")

        arguments = active_text[opening + 1 : cursor - 1].strip()
        if command_name == "endif":
            if condition_stack:
                condition_stack.pop()
            conditions = tuple(condition_stack)
        else:
            conditions = tuple(condition_stack)
        line = active_text.count("\n", 0, match.start()) + 1
        commands.append(CMakeCommand(command_name, arguments, line, conditions))

        if command_name == "if":
            condition_stack.append(_normalize_cmake_expression(arguments))
        elif command_name == "else" and condition_stack:
            condition_stack[-1] = "<else>"
        elif command_name == "elseif" and condition_stack:
            condition_stack[-1] = _normalize_cmake_expression(arguments)
        search_from = cursor
    return commands


_TOKEN = re.compile(r'"(?:\\.|[^"\\])*"|[^\s]+')


def _tokens(arguments: str) -> list[str]:
    tokens = _TOKEN.findall(arguments)
    return [token[1:-1] if len(token) >= 2 and token[0] == token[-1] == '"' else token for token in tokens]


def _normalize_cmake_expression(expression: str) -> str:
    return " ".join(_tokens(expression)).upper()


def _command_tokens(command: CMakeCommand) -> list[str]:
    return _tokens(command.arguments)


def _cmake_identifier(token: str) -> str:
    """Normalize a plain or ${...} CMake variable reference."""
    if token.startswith("${") and token.endswith("}"):
        return token[2:-1]
    return token


def _matching_commands(
    command_sets: Mapping[str, list[CMakeCommand]], path: str, name: str
) -> list[CMakeCommand]:
    return [command for command in command_sets[path] if command.name == name]


def _first_arg_matches(command: CMakeCommand, value: str) -> bool:
    tokens = _command_tokens(command)
    return bool(tokens) and tokens[0].casefold() == value.casefold()


def _default_from_option(
    commands: list[CMakeCommand], option_name: str, errors: list[str]
) -> str | None:
    matches = [
        command
        for command in commands
        if command.name == "option" and _first_arg_matches(command, option_name)
    ]
    if len(matches) != 1:
        errors.append(
            f"{option_name} must have exactly one active option() declaration; found {len(matches)}"
        )
        return None
    tokens = _command_tokens(matches[0])
    if len(tokens) < 3:
        errors.append(f"{option_name} option() declaration has no default value")
        return None
    return tokens[-1].upper()


def _has_direct_target(
    commands: list[CMakeCommand], command_name: str, target: str
) -> bool:
    return any(
        command.name == command_name and _first_arg_matches(command, target)
        for command in commands
    )


def _source_list_commands(commands: list[CMakeCommand], variable: str) -> list[CMakeCommand]:
    found: list[CMakeCommand] = []
    for command in commands:
        tokens = _command_tokens(command)
        if command.name == "set" and tokens and tokens[0].casefold() == variable.casefold():
            found.append(command)
        elif (
            command.name == "list"
            and len(tokens) >= 2
            and tokens[0].casefold() in {"append", "prepend", "insert"}
            and tokens[1].casefold() == variable.casefold()
        ):
            found.append(command)
    return found


def _declared_default(
    commands: list[CMakeCommand], variable: str, errors: list[str]
) -> str | None:
    declarations = [
        (command, _command_tokens(command))
        for command in commands
        if command.name in {"option", "set"} and _first_arg_matches(command, variable)
    ]
    if len(declarations) != 1:
        errors.append(
            f"{variable} must have exactly one active option()/set() declaration; found {len(declarations)}"
        )
        return None
    command, tokens = declarations[0]
    if command.name == "option":
        if len(tokens) < 3:
            errors.append(f"{variable} option() declaration has no default value")
            return None
        return tokens[-1]
    if len(tokens) < 4 or tokens[2].casefold() != "cache":
        errors.append(f"{variable} must use set(... CACHE ...) to define its reviewed default")
        return None
    return tokens[1]


def analyze_cmake_contract(contents: Mapping[str, bytes]) -> tuple[ProbeReport | None, list[str]]:
    """Check the documented build constraints in source after blob verification."""
    errors: list[str] = []
    command_sets: dict[str, list[CMakeCommand]] = {}
    try:
        for source in PINNED_SOURCES:
            command_sets[source.path] = parse_cmake_commands(
                contents[source.path].decode("utf-8", errors="strict")
            )
    except (KeyError, UnicodeDecodeError, ProbeError) as exc:
        return None, [f"Cannot analyze pinned CMake source: {exc}"]

    root_path = "core/CMakeLists.txt"
    tts_path = "core/moonshine-tts/CMakeLists.txt"
    ort_utils_path = "core/ort-utils/CMakeLists.txt"
    onnx_path = "core/third-party/onnxruntime/CMakeLists.txt"
    find_ort_path = "core/third-party/onnxruntime/find-ort-library-path.cmake"
    root_commands = command_sets[root_path]
    tts_commands = command_sets[tts_path]
    all_commands = [command for commands in command_sets.values() for command in commands]

    tts_subdirectory = [
        command
        for command in _matching_commands(command_sets, root_path, "add_subdirectory")
        if any("moonshine-tts" in token.casefold() for token in _command_tokens(command))
    ]
    if not tts_subdirectory:
        errors.append("Root CMake must add the moonshine-tts subdirectory")
    elif not any(not command.conditions for command in tts_subdirectory):
        errors.append("Root moonshine-tts add_subdirectory() must be unconditional")
    elif any(command.conditions for command in tts_subdirectory):
        errors.append("Root moonshine-tts must not also be conditionally added")

    tts_default = _default_from_option(tts_commands, "MOONSHINE_TTS_BUILD_ONNX", errors)
    if tts_default is not None and tts_default != "ON":
        errors.append(
            f"MOONSHINE_TTS_BUILD_ONNX default changed: expected ON, found {tts_default}"
        )

    tts_option_ifs = [
        command
        for command in tts_commands
        if command.name == "if"
        and _normalize_cmake_expression(command.arguments) == "MOONSHINE_TTS_BUILD_ONNX"
    ]
    if not tts_option_ifs:
        errors.append("TTS ONNX option must guard its ONNX-related section with if(MOONSHINE_TTS_BUILD_ONNX)")
    tts_option_condition = "MOONSHINE_TTS_BUILD_ONNX"
    for target in ("moonshine_tts_core", "moonshine_tts_utf8proc"):
        target_commands = [
            command
            for command in tts_commands
            if command.name == "add_library" and _first_arg_matches(command, target)
        ]
        if not target_commands:
            errors.append(f"TTS subtree target {target} is missing")
        elif not any(tts_option_condition not in command.conditions for command in target_commands):
            errors.append(f"TTS subtree target {target} is incorrectly treated as fully ONNX-gated")
    onnx_tts_target = [
        command
        for command in tts_commands
        if command.name == "add_library" and _first_arg_matches(command, "moonshine_tts_ort")
    ]
    if not any(tts_option_condition in command.conditions for command in onnx_tts_target):
        errors.append("moonshine_tts_ort must be declared inside the TTS ONNX option section")
    root_tts_link = [
        command
        for command in root_commands
        if command.name == "target_link_libraries"
        and _first_arg_matches(command, "moonshine")
        and any(token.casefold() == "moonshine_tts_ort" for token in _command_tokens(command))
    ]
    if not any(tts_option_condition in command.conditions for command in root_tts_link):
        errors.append("moonshine_tts_ort must only be linked to moonshine inside the TTS ONNX section")

    main_source_sets = _source_list_commands(root_commands, "MOONSHINE_SRC_FILES")
    inline_source_list = any(
        PurePosixPath(_cmake_identifier(token).replace("\\", "/")).name.casefold()
        == "speaker-diarizer.cpp"
        for command in main_source_sets
        for token in _command_tokens(command)
    )
    diarization_append = any(
        command.name == "list"
        and len(_command_tokens(command)) >= 3
        and _command_tokens(command)[0].casefold() == "append"
        and _command_tokens(command)[1].casefold() == "moonshine_src_files"
        and "cpp_annote_src_files"
        in {_cmake_identifier(token).casefold() for token in _command_tokens(command)[2:]}
        for command in root_commands
    )
    if not inline_source_list:
        errors.append("speaker-diarizer.cpp must be in the active MOONSHINE_SRC_FILES declaration")
    cpp_annote_sources = _source_list_commands(root_commands, "CPP_ANNOTE_SRC_FILES")
    if not any(
        "cpp-annote.cpp" in token.casefold()
        for command in cpp_annote_sources
        for token in _command_tokens(command)
    ):
        errors.append("CPP_ANNOTE_SRC_FILES must contain active cpp-annote source files")
    if not diarization_append:
        errors.append("Diarization dependency sources must be appended to MOONSHINE_SRC_FILES")
    library_target = [
        command
        for command in root_commands
        if command.name == "add_library" and _first_arg_matches(command, "moonshine")
    ]
    if not any("${MOONSHINE_SRC_FILES}".casefold() in command.arguments.casefold() for command in library_target):
        errors.append("The moonshine target must consume MOONSHINE_SRC_FILES")

    main_links = [
        command
        for command in root_commands
        if command.name == "target_link_libraries" and _first_arg_matches(command, "moonshine")
    ]
    main_dependencies: list[str] = []
    for dependency in ("bin-tokenizer", "ort-utils"):
        if any(
            any(token.casefold() == dependency.casefold() for token in _command_tokens(command)[1:])
            for command in main_links
        ):
            main_dependencies.append(dependency)
        else:
            errors.append(f"The moonshine target must reference {dependency} in target_link_libraries()")

    ort_link_path = any(
        command.name == "target_link_libraries"
        and _first_arg_matches(command, "ort-utils")
        and "${ONNXRUNTIME_LIB_PATH}".casefold() in command.arguments.casefold()
        for command in command_sets[ort_utils_path]
    )
    if not ort_link_path:
        errors.append("ort-utils must link the configured ONNXRUNTIME_LIB_PATH")
    imported_onnxruntime = any(
        command.name == "add_library"
        and _first_arg_matches(command, "onnxruntime")
        and "imported" in {token.casefold() for token in _command_tokens(command)[1:]}
        for command in command_sets[onnx_path]
    )
    if not imported_onnxruntime:
        errors.append("ONNX Runtime CMake must declare its imported onnxruntime target")
    if not any(
        command.name == "set" and _first_arg_matches(command, "ONNXRUNTIME_LIB_PATH")
        for command in command_sets[find_ort_path]
    ):
        errors.append("Pinned ONNX Runtime path resolver must set ONNXRUNTIME_LIB_PATH")

    reliability_default = _default_from_option(root_commands, "MOONSHINE_RELIABILITY", errors)
    if reliability_default is not None and reliability_default != "OFF":
        errors.append(f"MOONSHINE_RELIABILITY default changed: expected OFF, found {reliability_default}")
    reliability_condition = "MOONSHINE_RELIABILITY"
    if not any(
        command.name == "if"
        and _normalize_cmake_expression(command.arguments) == reliability_condition
        for command in root_commands
    ):
        errors.append("MOONSHINE_RELIABILITY must guard its reliability build section")
    reliability_section = [
        command
        for command in root_commands
        if reliability_condition in command.conditions and "<else>" not in command.conditions
    ]
    if not any(command.name == "add_compile_options" for command in reliability_section):
        errors.append("MOONSHINE_RELIABILITY section must add its compile options")
    sanitizer_flag = "-fsanitize=${MOONSHINE_SANITIZER}"
    if not any(
        command.name == "add_compile_options" and sanitizer_flag.casefold() in command.arguments.casefold()
        for command in reliability_section
    ):
        errors.append("MOONSHINE_SANITIZER must feed the reliability compile flags")
    if not any(
        command.name == "add_link_options" and sanitizer_flag.casefold() in command.arguments.casefold()
        for command in reliability_section
    ):
        errors.append("MOONSHINE_SANITIZER must feed the reliability link flags")
    sanitizer_default = _declared_default(root_commands, "MOONSHINE_SANITIZER", errors)
    if sanitizer_default is not None and sanitizer_default != "address,undefined":
        errors.append(
            "MOONSHINE_SANITIZER default changed: expected address,undefined, "
            f"found {sanitizer_default}"
        )

    shared_condition = next(
        (
            command
            for command in root_commands
            if command.name == "if"
            and "MOONSHINE_BUILD_SHARED" in _normalize_cmake_expression(command.arguments)
            and "MOONSHINE_BUILD_SWIFT" in _normalize_cmake_expression(command.arguments)
        ),
        None,
    )
    if shared_condition is None:
        errors.append(
            "MOONSHINE_BUILD_SHARED and MOONSHINE_BUILD_SWIFT must appear in the main library kind condition"
        )
    else:
        static_variant = any(
            command.name == "add_library"
            and _first_arg_matches(command, "moonshine")
            and any(token.casefold() == "static" for token in _command_tokens(command)[1:])
            and any(
                "MOONSHINE_BUILD_SHARED" in condition
                and "MOONSHINE_BUILD_SWIFT" in condition
                for condition in command.conditions
            )
            for command in root_commands
        )
        shared_variant = any(
            command.name == "add_library"
            and _first_arg_matches(command, "moonshine")
            and any(token.casefold() == "shared" for token in _command_tokens(command)[1:])
            and "<else>" in command.conditions
            for command in root_commands
        )
        if not static_variant or not shared_variant:
            errors.append(
                "Main library STATIC/SHARED targets no longer match the reviewed shared/Swift condition"
            )

    unknown_defaults: dict[str, str] = {}
    for variable in ("MOONSHINE_BUILD_SHARED", "MOONSHINE_BUILD_SWIFT"):
        referenced = any(
            re.search(rf"\b{variable}\b", command.arguments, re.IGNORECASE)
            for command in all_commands
        )
        assigned = any(
            command.name in {"set", "option"} and _first_arg_matches(command, variable)
            for command in all_commands
        )
        if not referenced:
            errors.append(f"{variable} reference is missing from the six pinned CMake files")
        if assigned:
            errors.append(
                f"{variable} now has an active option()/set() assignment in the inspected files; "
                "review its default instead of treating it as unknown"
            )
        unknown_defaults[variable] = "UNKNOWN (not assigned in inspected files)"

    executable_commands = [command for command in all_commands if command.name == "add_executable"]
    direct_test_targets = [
        _command_tokens(command)[0]
        for command in executable_commands
        if _command_tokens(command)
        and re.search(r"(?:^|[-_])test(?:$|[-_])|test$", _command_tokens(command)[0], re.IGNORECASE)
    ]
    dynamic_tts_test_builder = any(
        command.name == "function"
        and _first_arg_matches(command, "moonshine_tts_add_test")
        for command in tts_commands
    ) and any(command.name == "add_test" for command in tts_commands)
    if not direct_test_targets and not dynamic_tts_test_builder:
        errors.append("No source-defined test executable or test-builder behavior was found")
    utility_targets = [
        _command_tokens(command)[0]
        for command in executable_commands
        if _command_tokens(command)
        and re.search(r"benchmark|bench|smoke|_cli|g2p|batch", _command_tokens(command)[0], re.IGNORECASE)
    ]
    if not utility_targets:
        errors.append("No source-defined build utility executable was found")

    if errors:
        return None, errors
    return (
        ProbeReport(
            verified_blobs={},
            tts_onnx_default=tts_default or "",
            tts_subtree="included",
            diarization_in_main_target="included",
            main_target_dependencies=tuple(main_dependencies),
            ort_library_path_linked=ort_link_path and imported_onnxruntime,
            reliability_default=reliability_default or "",
            sanitizer_default=sanitizer_default or "",
            build_shared_default=unknown_defaults["MOONSHINE_BUILD_SHARED"],
            build_swift_default=unknown_defaults["MOONSHINE_BUILD_SWIFT"],
            test_target_count=len(direct_test_targets) + (1 if dynamic_tts_test_builder else 0),
            utility_target_count=len(utility_targets),
        ),
        [],
    )


def verify_sources(
    contents: Mapping[str, bytes],
    expected_blobs: Mapping[str, str] = EXPECTED_BLOBS,
) -> tuple[ProbeReport | None, list[str]]:
    """Verify exact pinned bytes, then inspect their narrow CMake contract."""
    hashes, errors = verify_blob_hashes(contents, expected_blobs)
    if errors:
        return None, errors
    report, errors = analyze_cmake_contract(contents)
    if report is None or errors:
        return None, errors
    report = ProbeReport(
        verified_blobs=hashes,
        tts_onnx_default=report.tts_onnx_default,
        tts_subtree=report.tts_subtree,
        diarization_in_main_target=report.diarization_in_main_target,
        main_target_dependencies=report.main_target_dependencies,
        ort_library_path_linked=report.ort_library_path_linked,
        reliability_default=report.reliability_default,
        sanitizer_default=report.sanitizer_default,
        build_shared_default=report.build_shared_default,
        build_swift_default=report.build_swift_default,
        test_target_count=report.test_target_count,
        utility_target_count=report.utility_target_count,
    )
    return report, []


def render_report(report: ProbeReport) -> str:
    """Render deterministic, source-only results and the required feasibility boundary."""
    lines = [
        "Outflow Moonshine CMake source probe passed",
        f"Source revision: {SOURCE_REVISION}",
        f"Pinned source files: {len(report.verified_blobs)} verified",
        f"TTS ONNX default: {report.tts_onnx_default}",
        f"TTS subtree: {report.tts_subtree}",
        f"Diarization sources: {report.diarization_in_main_target} in main target",
        "Minimal STT exclusions: NOT PROVEN",
        "CMake configure: NOT RUN",
        "Native compilation: NOT RUN",
        "Binary contents: NOT VERIFIED",
        "Source integrity: VERIFIED",
        "CMake configuration behavior: VERIFIED",
        "Intended minimal STT scope: DOCUMENTED",
        "Exact minimal build configuration: NOT PROVEN",
        "Production packaging: NOT VERIFIED",
        f"Main target dependencies: {', '.join(report.main_target_dependencies)}",
        f"ONNX Runtime path linked through ort-utils: {'YES' if report.ort_library_path_linked else 'NO'}",
        f"MOONSHINE_RELIABILITY default: {report.reliability_default}",
        f"MOONSHINE_SANITIZER default: {report.sanitizer_default}",
        f"MOONSHINE_BUILD_SHARED default: {report.build_shared_default}",
        f"MOONSHINE_BUILD_SWIFT default: {report.build_swift_default}",
        "Test/build utility declarations: PRESENT; production packaging NOT VERIFIED",
        "Verified Git blobs:",
    ]
    lines.extend(f"  {path}: {report.verified_blobs[path]}" for path in EXPECTED_BLOBS)
    return "\n".join(lines)


def _argument_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description=(
            "Verify the six pinned Moonshine CMake Git blobs and a narrow source contract; "
            "does not configure or compile."
        )
    )
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--source-dir", type=Path, help="offline root of a pinned Moonshine checkout")
    modes.add_argument(
        "--verify-upstream",
        action="store_true",
        help="retrieve only the six pinned CMake text files over HTTPS into memory",
    )
    return parser


def main(argv: list[str] | None = None) -> int:
    args = _argument_parser().parse_args(argv)
    if args.verify_upstream:
        try:
            contents = fetch_upstream_sources()
        except UpstreamRetrievalError as exc:
            print(f"BLOCKED: upstream source retrieval failed: {exc}", file=sys.stderr)
            return 2
    else:
        try:
            contents = read_local_sources(args.source_dir)
        except ProbeError as exc:
            print(f"ERROR: {exc}", file=sys.stderr)
            return 1

    report, errors = verify_sources(contents)
    if errors or report is None:
        print("Outflow Moonshine CMake source probe failed", file=sys.stderr)
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print(render_report(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
