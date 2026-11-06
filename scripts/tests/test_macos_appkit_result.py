from __future__ import annotations

from pathlib import Path
import plistlib
import tempfile
import unittest
from unittest.mock import patch

from scripts.classify_macos_appkit_test import (
    AppKitLaunchError,
    classify_appkit_result,
    validate_app_bundle,
)


class AppKitResultClassificationTests(unittest.TestCase):
    def test_only_zero_status_and_explicit_result_pass_marker_pass(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = root / "result.txt"
            launch_log = root / "launch.log"
            result.write_text("PASS isolated AppKit pasteboard integration tests: 8 values\n", encoding="utf-8")
            launch_log.write_text("", encoding="utf-8")
            self.assertEqual(classify_appkit_result(0, result, launch_log)[0], "PASS")
            self.assertEqual(classify_appkit_result(1, result, launch_log)[0], "FAIL")

    def test_missing_result_is_blocked_only_for_known_launchservices_error(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = root / "missing-result.txt"
            launch_log = root / "launch.log"
            launch_log.write_text("LSOpenURLsWithCompletionHandler failed: kLSNoExecutableErr\n", encoding="utf-8")
            self.assertEqual(classify_appkit_result(1, result, launch_log)[0], "BLOCKED")
            launch_log.write_text("permission denied\n", encoding="utf-8")
            self.assertEqual(classify_appkit_result(1, result, launch_log)[0], "FAIL")
            self.assertEqual(classify_appkit_result(0, result, launch_log)[0], "FAIL")

    def test_failure_result_never_becomes_pass_after_successful_build_or_launch(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = root / "result.txt"
            launch_log = root / "launch.log"
            result.write_text("FAIL isolated AppKit pasteboard integration tests: 1 mismatch\n", encoding="utf-8")
            launch_log.write_text("", encoding="utf-8")
            self.assertEqual(classify_appkit_result(0, result, launch_log)[0], "FAIL")

    def test_pass_marker_cannot_hide_an_additional_failure_result(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            result = root / "result.txt"
            launch_log = root / "launch.log"
            result.write_text(
                "PASS isolated AppKit pasteboard integration tests: 8 values\n"
                "FAIL isolated AppKit pasteboard integration tests: mismatch\n",
                encoding="utf-8",
            )
            launch_log.write_text("", encoding="utf-8")
            self.assertEqual(classify_appkit_result(0, result, launch_log)[0], "FAIL")

    def test_bundle_verifier_checks_plist_executable_and_signature(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            bundle = Path(temporary) / "OutflowMicProbe.app"
            executable = bundle / "Contents" / "MacOS" / "OutflowMicProbe"
            executable.parent.mkdir(parents=True)
            executable.write_bytes(b"Mach-O test fixture")
            executable.chmod(0o755)
            with (bundle / "Contents" / "Info.plist").open("wb") as output:
                plistlib.dump(
                    {
                        "CFBundleExecutable": "OutflowMicProbe",
                        "CFBundlePackageType": "APPL",
                        "CFBundleIdentifier": "ai.outflow.mic-probe-dev",
                    },
                    output,
                )
            with patch("scripts.classify_macos_appkit_test.subprocess.run") as run:
                run.return_value.returncode = 0
                self.assertEqual(validate_app_bundle(bundle), bundle.resolve())
                run.assert_called_once()

            with (bundle / "Contents" / "Info.plist").open("wb") as output:
                plistlib.dump(
                    {
                        "CFBundleExecutable": "Missing",
                        "CFBundlePackageType": "APPL",
                        "CFBundleIdentifier": "ai.outflow.mic-probe-dev",
                    },
                    output,
                )
            with self.assertRaisesRegex(AppKitLaunchError, "identity"):
                validate_app_bundle(bundle)


if __name__ == "__main__":
    unittest.main()
