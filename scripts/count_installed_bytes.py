#!/usr/bin/env python3
"""Count logical bytes in an explicitly supplied installation directory.

This read-only utility counts every regular-file directory entry, including
multiple hard-link names, and adds each validated symbolic link's own logical
length separately. Symbolic-link targets are validated but never traversed.
The result is a logical-size estimate; it does not report allocated disk
blocks or certify an Outflow release.
"""

from __future__ import annotations

import argparse
from dataclasses import asdict, dataclass
import json
import os
import stat
import sys
import tempfile


INSTALLED_BYTES_LIMIT = 950_000_000


class MeasurementError(Exception):
    """A controlled measurement failure that does not disclose filesystem paths."""


@dataclass(frozen=True)
class MeasurementResult:
    """Logical-size totals for one directory tree.

    ``total_logical_bytes`` includes regular-file entry sizes and symbolic-link
    entry sizes. The components remain separate so callers can see what was
    counted; hard-linked regular-file names each contribute their own size.
    """

    regular_file_entries: int
    regular_file_bytes: int
    symbolic_links_encountered: int
    symbolic_link_logical_bytes: int
    total_logical_bytes: int
    limit_bytes: int
    below_limit: bool


def is_below_release_limit(total_logical_bytes: int) -> bool:
    """Return whether a logical byte count is strictly below the decimal limit."""

    return total_logical_bytes < INSTALLED_BYTES_LIMIT


def _resolved_link_is_within_root(link_path: str, root_real_path: str) -> bool:
    """Resolve a link only for validation; callers never scan its target."""

    try:
        resolved = os.path.realpath(link_path, strict=True)
    except (OSError, RuntimeError, ValueError) as error:
        raise MeasurementError("A symbolic link is broken or cannot be validated safely.") from error

    try:
        return os.path.commonpath((root_real_path, resolved)) == root_real_path
    except ValueError:
        # Different drives (on platforms that have them) cannot be contained.
        return False


def count_installed_bytes(root: str | os.PathLike[str]) -> MeasurementResult:
    """Recursively count logical bytes without reading or modifying file contents.

    The root must be an existing directory and must not itself be a symbolic
    link. Descendant links are validated to reject broken or escaping targets,
    then counted by their own link-entry length without following them during
    traversal. Concurrent filesystem mutation is outside this utility's
    guarantees.

    Raises:
        MeasurementError: If the root is invalid, metadata is inaccessible, a
            directory cannot be read, a symbolic link is unsafe, or a special
            filesystem entry is present.
    """

    try:
        supplied_root = os.fspath(root)
    except TypeError as error:
        raise MeasurementError("The supplied root is not a valid filesystem path.") from error
    if not isinstance(supplied_root, str) or not supplied_root:
        raise MeasurementError("The supplied root is not a valid filesystem path.")

    root_path = os.path.abspath(supplied_root)
    try:
        root_info = os.lstat(root_path)
    except (OSError, ValueError) as error:
        raise MeasurementError("The supplied root is missing or its metadata is inaccessible.") from error

    if stat.S_ISLNK(root_info.st_mode):
        raise MeasurementError("The supplied root must not be a symbolic link.")
    if not stat.S_ISDIR(root_info.st_mode):
        raise MeasurementError("The supplied root is not a directory.")

    try:
        root_real_path = os.path.realpath(root_path, strict=True)
    except (OSError, RuntimeError, ValueError) as error:
        raise MeasurementError("The supplied root cannot be validated safely.") from error

    regular_file_entries = 0
    regular_file_bytes = 0
    symbolic_links_encountered = 0
    symbolic_link_logical_bytes = 0

    pending_directories = [root_path]
    while pending_directories:
        directory_path = pending_directories.pop()
        try:
            entries = os.scandir(directory_path)
        except (OSError, ValueError) as error:
            raise MeasurementError("A directory cannot be read.") from error

        try:
            with entries:
                for entry in entries:
                    try:
                        entry_info = entry.stat(follow_symlinks=False)
                    except (OSError, ValueError) as error:
                        raise MeasurementError(
                            "Filesystem metadata is inaccessible or changed during measurement."
                        ) from error

                    mode = entry_info.st_mode
                    if stat.S_ISLNK(mode):
                        symbolic_links_encountered += 1
                        symbolic_link_logical_bytes += entry_info.st_size
                        if not _resolved_link_is_within_root(entry.path, root_real_path):
                            raise MeasurementError("A symbolic link resolves outside the supplied root.")
                    elif stat.S_ISDIR(mode):
                        pending_directories.append(entry.path)
                    elif stat.S_ISREG(mode):
                        regular_file_entries += 1
                        regular_file_bytes += entry_info.st_size
                    else:
                        raise MeasurementError("An unexpected special filesystem entry was found.")
        except OSError as error:
            raise MeasurementError("A directory cannot be read.") from error

    total_logical_bytes = regular_file_bytes + symbolic_link_logical_bytes
    return MeasurementResult(
        regular_file_entries=regular_file_entries,
        regular_file_bytes=regular_file_bytes,
        symbolic_links_encountered=symbolic_links_encountered,
        symbolic_link_logical_bytes=symbolic_link_logical_bytes,
        total_logical_bytes=total_logical_bytes,
        limit_bytes=INSTALLED_BYTES_LIMIT,
        below_limit=is_below_release_limit(total_logical_bytes),
    )


def _print_demo() -> None:
    """Count three disposable fixtures without touching an application install."""

    with tempfile.TemporaryDirectory(prefix="outflow-installed-size-") as temporary_root:
        root = os.path.join(temporary_root, "synthetic-install")
        os.makedirs(os.path.join(root, "models"))
        os.makedirs(os.path.join(root, "licenses"))
        executable_path = os.path.join(root, "outflow-executable")
        with open(executable_path, "wb") as executable:
            executable.write(bytes(range(32)))
        os.chmod(executable_path, stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
        with open(os.path.join(root, "models", "model-placeholder"), "wb") as model:
            model.write(bytes(range(64)))
        with open(os.path.join(root, "licenses", "NOTICE"), "wb") as notice:
            notice.write(bytes(range(16)))

        result = count_installed_bytes(root)
        if result.regular_file_entries != 3 or result.total_logical_bytes != 112:
            raise MeasurementError("The synthetic demonstration did not match its expected total.")

    print("Outflow installed-byte counter ready")
    print("Files counted: 3")
    print("Total logical bytes: 112")
    print(f"Limit bytes: {INSTALLED_BYTES_LIMIT}")
    print("Budget comparison: BELOW LIMIT")
    print("Measurement type: SYNTHETIC")
    print("Release certification: NOT ASSESSED")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--root", help="installation directory to measure")
    mode.add_argument("--demo", action="store_true", help="run a synthetic 112-byte demonstration")
    arguments = parser.parse_args(argv)

    try:
        if arguments.demo:
            _print_demo()
            return 0

        result = count_installed_bytes(arguments.root)
    except MeasurementError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2

    payload = asdict(result)
    payload["measurement_type"] = "SUPPLIED_DIRECTORY"
    payload["release_certification"] = "NOT ASSESSED"
    print(json.dumps(payload, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
