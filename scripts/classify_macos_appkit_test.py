#!/usr/bin/env python3
"""Validate the development .app bundle and classify its isolated AppKit test."""

from __future__ import annotations

import argparse
from pathlib import Path
import plistlib
import re
import subprocess
import sys
from typing import Sequence


PASS_PREFIX = "PASS isolated AppKit pasteboard integration tests:"


class AppKitLaunchError(ValueError):
    """The app bundle is malformed or the test result is ambiguous."""


def validate_app_bundle(bundle_path: Path) -> Path:
    """Fail closed unless LaunchServices has a signed, executable app bundle."""

    try:
        bundle = bundle_path.resolve(strict=True)
        if not bundle.is_dir() or bundle.suffix != ".app":
            raise ValueError
        contents = bundle / "Contents"
        info_path = contents / "Info.plist"
        with info_path.open("rb") as source:
            info = plistlib.load(source)
    except (OSError, ValueError, plistlib.InvalidFileException) as error:
        raise AppKitLaunchError("app bundle or Info.plist is missing or invalid") from error
    if not isinstance(info, dict):
        raise AppKitLaunchError("app Info.plist is not a dictionary")
    executable_name = info.get("CFBundleExecutable")
    if (
        executable_name != "OutflowMicProbe"
        or info.get("CFBundlePackageType") != "APPL"
        or info.get("CFBundleIdentifier") != "ai.outflow.mic-probe-dev"
        or Path(executable_name).name != executable_name
    ):
        raise AppKitLaunchError("app bundle identity does not match the native test app")
    executable = contents / "MacOS" / executable_name
    try:
        resolved_executable = executable.resolve(strict=True)
        resolved_executable.relative_to((contents / "MacOS").resolve(strict=True))
        if not resolved_executable.is_file() or not (resolved_executable.stat().st_mode & 0o111):
            raise ValueError
    except (OSError, ValueError) as error:
        raise AppKitLaunchError("CFBundleExecutable is missing, outside Contents/MacOS, or not executable") from error

    try:
        signature = subprocess.run(
            ["/usr/bin/codesign", "--verify", "--deep", "--strict", str(bundle)],
            check=False,
            capture_output=True,
            text=True,
            timeout=15,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise AppKitLaunchError("app code signature could not be verified") from error
    if signature.returncode != 0:
        raise AppKitLaunchError("app bundle code signature verification failed")
    return bundle


def _is_launchservices_no_executable(log_text: str) -> bool:
    return (
        "kLSNoExecutableErr" in log_text
        or re.search(r"\berror\s+-10827\b", log_text) is not None
    )


def classify_appkit_result(
    launch_status: int, result_path: Path, launch_log_path: Path
) -> tuple[str, str]:
    """Never treat a missing result or a failed launch as a successful test."""

    try:
        log_text = launch_log_path.read_text(encoding="utf-8", errors="replace")
        result_text = result_path.read_text(encoding="utf-8", errors="strict") if result_path.exists() else None
    except (OSError, UnicodeError) as error:
        raise AppKitLaunchError("AppKit test evidence could not be read") from error

    if result_text is not None:
        result_lines = result_text.splitlines()
        if launch_status == 0 and len(result_lines) == 1 and result_lines[0].startswith(PASS_PREFIX):
            return "PASS", "AppKit desktop integration passed"
        return "FAIL", "AppKit test wrote a result that was failed or inconsistent with LaunchServices"
    if launch_status != 0 and _is_launchservices_no_executable(log_text):
        return "BLOCKED", "AppKit launch blocked by environment (LaunchServices kLSNoExecutableErr)"
    return "FAIL", "AppKit test produced no result and no recognized environment block"


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    validate = subparsers.add_parser("validate-bundle")
    validate.add_argument("--bundle", type=Path, required=True)
    classify = subparsers.add_parser("classify")
    classify.add_argument("--launch-status", type=int, required=True)
    classify.add_argument("--result", type=Path, required=True)
    classify.add_argument("--launch-log", type=Path, required=True)
    arguments = parser.parse_args(argv)

    try:
        if arguments.command == "validate-bundle":
            bundle = validate_app_bundle(arguments.bundle)
            print(f"PASS AppKit test bundle verified: {bundle.name}")
            return 0
        status, summary = classify_appkit_result(
            arguments.launch_status, arguments.result, arguments.launch_log
        )
    except AppKitLaunchError as error:
        print(f"FAIL AppKit test preparation: {error}", file=sys.stderr)
        return 1
    print(f"{status} {summary}")
    return {"PASS": 0, "BLOCKED": 2, "FAIL": 1}[status]


if __name__ == "__main__":
    raise SystemExit(main())
