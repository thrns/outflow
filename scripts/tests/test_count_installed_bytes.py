from __future__ import annotations

import contextlib
import io
import json
import os
from pathlib import Path
import stat
import tempfile
import unittest

from scripts.count_installed_bytes import (
    INSTALLED_BYTES_LIMIT,
    MeasurementError,
    _print_demo,
    count_installed_bytes,
    is_below_release_limit,
    main,
)


class InstalledByteCounterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary_directory.cleanup)
        self.root = Path(self.temporary_directory.name) / "install"
        self.root.mkdir()

    def write_file(self, relative_path: str, content: bytes) -> Path:
        path = self.root / relative_path
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return path

    def assert_symlinks_supported(self) -> None:
        if not hasattr(os, "symlink"):
            self.skipTest("This operating system does not provide symbolic links.")

    def make_symlink(self, target: str | Path, link: Path) -> None:
        self.assert_symlinks_supported()
        try:
            link.symlink_to(target)
        except (NotImplementedError, OSError) as error:
            if isinstance(error, OSError) and error.errno not in {
                getattr(os, "EACCES", 13),
                getattr(os, "EPERM", 1),
                getattr(os, "ENOSYS", 38),
                getattr(os, "ENOTSUP", 95),
                getattr(os, "EOPNOTSUPP", 95),
            }:
                raise
            self.skipTest("The current filesystem does not permit symbolic links.")

    def test_empty_directory_returns_zero_bytes(self) -> None:
        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 0)
        self.assertEqual(result.regular_file_bytes, 0)
        self.assertEqual(result.total_logical_bytes, 0)
        self.assertTrue(result.below_limit)

    def test_single_file_size_is_correct(self) -> None:
        self.write_file("application", b"dictation")

        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 1)
        self.assertEqual(result.total_logical_bytes, 9)

    def test_nested_directory_sizes_are_summed(self) -> None:
        self.write_file("bin/app", b"abc")
        self.write_file("share/models/weights", b"1234567")

        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 2)
        self.assertEqual(result.total_logical_bytes, 10)

    def test_different_file_name_types_are_counted(self) -> None:
        self.write_file("app.exe", b"x" * 2)
        self.write_file("libnative.dylib", b"y" * 3)
        self.write_file("model.unknown-extension", b"z" * 5)

        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 3)
        self.assertEqual(result.total_logical_bytes, 10)

    def test_hidden_files_are_included(self) -> None:
        self.write_file(".hidden", b"hidden")

        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 1)
        self.assertEqual(result.total_logical_bytes, 6)

    def test_multiple_files_are_not_omitted(self) -> None:
        for index in range(12):
            self.write_file(f"support/{index}.dat", bytes([index]) * (index + 1))

        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 12)
        self.assertEqual(result.total_logical_bytes, sum(range(1, 13)))

    def test_hard_link_names_are_counted_as_distinct_entries(self) -> None:
        original = self.write_file("models/weights", b"12345")
        alias = self.root / "models" / "weights-copy"
        try:
            os.link(original, alias)
        except (NotImplementedError, OSError) as error:
            if isinstance(error, OSError) and error.errno not in {
                getattr(os, "EPERM", 1),
                getattr(os, "ENOSYS", 38),
                getattr(os, "ENOTSUP", 95),
                getattr(os, "EOPNOTSUPP", 95),
            }:
                raise
            self.skipTest("The current filesystem does not support hard links.")

        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 2)
        self.assertEqual(result.regular_file_bytes, 10)

    def test_internal_file_symbolic_link_is_reported_but_not_followed(self) -> None:
        self.write_file("model.bin", b"weights")
        self.make_symlink(Path("model.bin"), self.root / "model-alias")

        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 1)
        self.assertEqual(result.regular_file_bytes, 7)
        self.assertEqual(result.symbolic_links_encountered, 1)
        self.assertEqual(result.symbolic_link_logical_bytes, len(os.fsencode("model.bin")))
        self.assertEqual(result.total_logical_bytes, 7 + len(os.fsencode("model.bin")))

    def test_symbolic_link_to_root_does_not_create_a_traversal_cycle(self) -> None:
        self.write_file("nested/payload", b"abc")
        self.make_symlink(self.root, self.root / "nested" / "back-to-root")

        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 1)
        self.assertEqual(result.regular_file_bytes, 3)
        self.assertEqual(result.symbolic_links_encountered, 1)

    def test_mutually_cyclic_symbolic_links_fail_without_traversal_recursion(self) -> None:
        self.make_symlink("link-b", self.root / "link-a")
        self.make_symlink("link-a", self.root / "link-b")

        with self.assertRaisesRegex(MeasurementError, "broken or cannot be validated"):
            count_installed_bytes(self.root)

    def test_symbolic_link_escaping_root_is_rejected(self) -> None:
        outside = Path(self.temporary_directory.name) / "outside"
        outside.write_bytes(b"private")
        self.make_symlink(outside, self.root / "outside-link")

        with self.assertRaisesRegex(MeasurementError, "outside"):
            count_installed_bytes(self.root)

    def test_broken_symbolic_link_is_rejected(self) -> None:
        self.make_symlink(self.root / "missing-target", self.root / "broken")

        with self.assertRaisesRegex(MeasurementError, "broken"):
            count_installed_bytes(self.root)

    def test_missing_root_is_rejected(self) -> None:
        with self.assertRaisesRegex(MeasurementError, "missing"):
            count_installed_bytes(self.root / "absent")

    def test_invalid_root_metadata_returns_a_controlled_error(self) -> None:
        with self.assertRaisesRegex(MeasurementError, "metadata is inaccessible"):
            count_installed_bytes("\0")

    def test_file_root_is_rejected(self) -> None:
        file_path = self.write_file("application", b"x")

        with self.assertRaisesRegex(MeasurementError, "not a directory"):
            count_installed_bytes(file_path)

    def test_symbolic_link_root_is_rejected(self) -> None:
        self.assert_symlinks_supported()
        alias = Path(self.temporary_directory.name) / "install-alias"
        self.make_symlink(self.root, alias)

        with self.assertRaisesRegex(MeasurementError, "must not be a symbolic link"):
            count_installed_bytes(alias)

    def test_special_fifo_entry_is_rejected_where_supported(self) -> None:
        if not hasattr(os, "mkfifo"):
            self.skipTest("This operating system does not provide FIFO filesystem entries.")
        os.mkfifo(self.root / "unexpected-pipe")

        with self.assertRaisesRegex(MeasurementError, "special filesystem entry"):
            count_installed_bytes(self.root)

    def test_inaccessible_directory_returns_a_controlled_error_when_meaningful(self) -> None:
        if os.name != "posix" or not hasattr(os, "geteuid"):
            self.skipTest("Permission-bit directory denial is not supported on this platform.")
        if os.geteuid() == 0:
            self.skipTest("Root can bypass the directory permission bits on this host.")

        blocked = self.root / "blocked"
        blocked.mkdir()
        (blocked / "payload").write_bytes(b"x")
        blocked.chmod(0)
        try:
            with self.assertRaisesRegex(MeasurementError, "directory cannot be read"):
                count_installed_bytes(self.root)
        finally:
            blocked.chmod(stat.S_IRWXU)

    def test_zero_byte_files_are_counted(self) -> None:
        self.write_file("empty", b"")

        result = count_installed_bytes(self.root)

        self.assertEqual(result.regular_file_entries, 1)
        self.assertEqual(result.total_logical_bytes, 0)

    def test_threshold_comparison_is_strictly_less_than(self) -> None:
        self.assertEqual(INSTALLED_BYTES_LIMIT, 950_000_000)
        self.assertFalse(is_below_release_limit(950_000_000))

    def test_one_byte_below_threshold_passes(self) -> None:
        self.assertTrue(is_below_release_limit(949_999_999))

    def test_one_byte_above_threshold_fails(self) -> None:
        self.assertFalse(is_below_release_limit(950_000_001))

    def test_demo_reports_the_synthetic_112_byte_total(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            _print_demo()

        self.assertEqual(
            output.getvalue(),
            "Outflow installed-byte counter ready\n"
            "Files counted: 3\n"
            "Total logical bytes: 112\n"
            "Limit bytes: 950000000\n"
            "Budget comparison: BELOW LIMIT\n"
            "Measurement type: SYNTHETIC\n"
            "Release certification: NOT ASSESSED\n",
        )

    def test_root_cli_returns_structured_json_without_disclosing_the_path(self) -> None:
        self.write_file("nested/payload", b"abc")
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = main(["--root", str(self.root)])

        result = json.loads(output.getvalue())
        self.assertEqual(exit_code, 0)
        self.assertEqual(result["regular_file_entries"], 1)
        self.assertEqual(result["total_logical_bytes"], 3)
        self.assertTrue(result["below_limit"])
        self.assertEqual(result["release_certification"], "NOT ASSESSED")
        self.assertNotIn(str(self.root), output.getvalue())

    def test_cli_errors_do_not_print_the_supplied_absolute_path(self) -> None:
        missing_root = self.root / "private-missing-root"
        output = io.StringIO()
        errors = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
            exit_code = main(["--root", str(missing_root)])

        self.assertEqual(exit_code, 2)
        self.assertEqual(output.getvalue(), "")
        self.assertNotIn(str(missing_root), errors.getvalue())

    def test_repeated_measurements_of_an_unchanged_tree_are_deterministic(self) -> None:
        self.write_file("bin/app", b"abc")
        self.write_file("share/model", b"defgh")

        first = count_installed_bytes(self.root)
        second = count_installed_bytes(self.root)

        self.assertEqual(first, second)

    def test_measurement_does_not_change_tree_entries_or_file_metadata(self) -> None:
        self.write_file("model.bin", b"weights")
        before = (self.root / "model.bin").stat()

        count_installed_bytes(self.root)

        after = (self.root / "model.bin").stat()
        self.assertEqual(before.st_size, after.st_size)
        self.assertEqual(before.st_mtime_ns, after.st_mtime_ns)
        self.assertEqual(sorted(path.name for path in self.root.iterdir()), ["model.bin"])


if __name__ == "__main__":
    unittest.main()
