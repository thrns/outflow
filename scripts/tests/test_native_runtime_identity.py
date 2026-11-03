from __future__ import annotations

from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from scripts.run_native_asr_benchmark import BenchmarkError, EXPECTED_HEADER_VERSION, verify_native_runtime
from scripts.verify_native_runtime import main


class NativeRuntimeIdentityTests(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.addCleanup(self.tempdir.cleanup)
        root = Path(self.tempdir.name)
        self.runner = root / "native-runner"
        self.moonshine = root / "libmoonshine.dylib"
        self.onnxruntime = root / "libonnxruntime.1.23.2.dylib"
        self.runner.write_bytes(b"runner bytes")
        self.moonshine.write_bytes(b"moonshine bytes")
        self.onnxruntime.write_bytes(b"onnxruntime bytes")
        self.probe = {
            "schema_version": 1,
            "architecture": "arm64",
            "pointer_bits": 64,
            "c_api_version": EXPECTED_HEADER_VERSION,
            "runner_path": str(self.runner.resolve()),
            "moonshine_api_path": str(self.moonshine.resolve()),
            "loaded_images": [str(self.runner.resolve()), str(self.moonshine.resolve()), str(self.onnxruntime.resolve())],
        }

    def tool_output(self, command: tuple[str, ...] | list[str]) -> str:
        if command[0] == "lipo":
            return "arm64"
        path = Path(command[-1])
        if path.resolve() == self.runner.resolve():
            return (
                f"{path}:\n"
                "\t@rpath/libmoonshine.dylib (compatibility version 0.0.0, current version 0.0.0)\n"
                "\t/usr/lib/libc++.1.dylib (compatibility version 1.0.0, current version 2100.0.0)\n"
            )
        if path.resolve() == self.moonshine.resolve():
            return (
                f"{path}:\n"
                "\t@rpath/libmoonshine.dylib (compatibility version 0.0.0, current version 0.0.0)\n"
                "\t@rpath/libonnxruntime.1.23.2.dylib (compatibility version 0.0.0, current version 1.23.2)\n"
                "\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0, current version 1.0.0)\n"
            )
        raise AssertionError("unexpected native dependency inspection target")

    def verify(self, probe: dict[str, object] | None = None, *, tool_side_effect=None) -> dict[str, object]:
        with (
            patch("scripts.run_native_asr_benchmark.sys.platform", "darwin"),
            patch("scripts.run_native_asr_benchmark._run_json_command", return_value=probe or self.probe),
            patch("scripts.run_native_asr_benchmark._system_tool_output", side_effect=tool_side_effect or self.tool_output),
        ):
            return verify_native_runtime(
                self.runner,
                self.moonshine,
                self.onnxruntime,
                host_architecture="arm64",
            )

    def test_runtime_identity_accepts_exact_loaded_cpu_binaries(self) -> None:
        result = self.verify()
        self.assertEqual(result["architecture"], "arm64")
        self.assertEqual(result["c_api_version"], EXPECTED_HEADER_VERSION)
        self.assertEqual(result["loaded_image_count"], 3)
        self.assertEqual(result["unexpected_non_system_library_count"], 0)

    def test_moonshine_api_symbol_from_mismatched_binary_is_rejected(self) -> None:
        other = Path(self.tempdir.name) / "other-moonshine.dylib"
        other.write_bytes(b"other")
        probe = dict(self.probe, moonshine_api_path=str(other.resolve()))
        with self.assertRaisesRegex(BenchmarkError, "unexpected binary"):
            self.verify(probe)

    def test_missing_loaded_library_is_rejected(self) -> None:
        probe = dict(self.probe, loaded_images=[str(self.runner.resolve()), str(self.moonshine.resolve())])
        with self.assertRaisesRegex(BenchmarkError, "did not load exactly"):
            self.verify(probe)

    def test_unsupported_active_architecture_is_rejected(self) -> None:
        probe = dict(self.probe, architecture="x86_64")
        with self.assertRaisesRegex(BenchmarkError, "identity or architecture"):
            self.verify(probe)

    def test_binary_hash_change_during_probe_is_rejected(self) -> None:
        def probe_then_mutate(*args, **kwargs):
            self.onnxruntime.write_bytes(b"changed after runtime probe")
            return self.probe

        with (
            patch("scripts.run_native_asr_benchmark.sys.platform", "darwin"),
            patch("scripts.run_native_asr_benchmark._run_json_command", side_effect=probe_then_mutate),
            patch("scripts.run_native_asr_benchmark._system_tool_output", side_effect=self.tool_output),
        ):
            with self.assertRaisesRegex(BenchmarkError, "changed during"):
                verify_native_runtime(self.runner, self.moonshine, self.onnxruntime, host_architecture="arm64")

    def test_ambiguous_onnx_runtime_dependency_is_rejected(self) -> None:
        def ambiguous_tool(command):
            output = self.tool_output(command)
            if command[0] == "otool" and Path(command[-1]).resolve() == self.moonshine.resolve():
                output += "\t@rpath/libonnxruntime.1.23.1.dylib (compatibility version 0.0.0, current version 1.23.1)\n"
            return output

        with self.assertRaisesRegex(BenchmarkError, "dependency chain is missing or ambiguous"):
            self.verify(tool_side_effect=ambiguous_tool)

    def test_unexpected_loaded_library_is_rejected(self) -> None:
        extra = Path(self.tempdir.name) / "libunapproved.dylib"
        extra.write_bytes(b"unapproved")
        probe = dict(self.probe, loaded_images=[*self.probe["loaded_images"], str(extra.resolve())])
        with self.assertRaisesRegex(BenchmarkError, "unapproved non-system"):
            self.verify(probe)

    def test_cli_prints_only_sanitized_runtime_identity(self) -> None:
        expected = {
            "architecture": "arm64",
            "c_api_version": EXPECTED_HEADER_VERSION,
            "loaded_image_count": 3,
            "unexpected_non_system_library_count": 0,
            "resolved_paths_match_inputs": True,
            "dependency_chain_verified": True,
            "native_runner_sha256": "c" * 64,
            "moonshine_sha256": "a" * 64,
            "onnxruntime_sha256": "b" * 64,
        }
        with patch("scripts.verify_native_runtime.verify_native_runtime", return_value=expected):
            self.assertEqual(main([
                "--native-runner", str(self.runner),
                "--moonshine-library", str(self.moonshine),
                "--onnxruntime-library", str(self.onnxruntime),
            ]), 0)

    def test_cli_fails_conservatively_when_identity_cannot_be_verified(self) -> None:
        with patch("scripts.verify_native_runtime.verify_native_runtime", side_effect=BenchmarkError("ambiguous")):
            self.assertEqual(main([
                "--native-runner", str(self.runner),
                "--moonshine-library", str(self.moonshine),
                "--onnxruntime-library", str(self.onnxruntime),
            ]), 1)


if __name__ == "__main__":
    unittest.main()
