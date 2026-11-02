"""Tests for the isolated Moonshine configure runner.

The test checkout and CMake replies are synthetic fixtures. No test result is
evidence that the pinned upstream project configures successfully.
"""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

from scripts import probe_moonshine_cmake as probe
from scripts import try_moonshine_cmake_configure as configure


class MoonshineConfigureRunnerTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_root = tempfile.TemporaryDirectory(prefix="outflow-configure-test-")
        self.root = Path(self.temp_root.name)
        self.source = self.root / "moonshine"
        self.source.mkdir()
        self.expected_contents: dict[str, bytes] = {}
        for pinned in probe.PINNED_SOURCES:
            destination = self.source / pinned.path
            destination.parent.mkdir(parents=True, exist_ok=True)
            if pinned.path == "core/CMakeLists.txt":
                data = b"cmake_minimum_required(VERSION 3.18)\nproject(Moonshine)\n"
            else:
                data = f"# test fixture for {pinned.path}\n".encode()
            destination.write_bytes(data)
            self.expected_contents[pinned.path] = data

    def tearDown(self) -> None:
        self.temp_root.cleanup()

    def _verify_fixture(self, contents: dict[str, bytes]):
        differences = [
            path
            for path, expected in self.expected_contents.items()
            if contents.get(path) != expected
        ]
        if differences:
            return None, [f"Git blob SHA mismatch for {differences[0]}"]
        return object(), []

    def _make_cmake_reply(self, build_dir: Path, *, ort_path: Path | None = None) -> None:
        if ort_path is None:
            ort_path = self.source / "core" / "third-party" / "onnxruntime" / "missing.dylib"
        cache = "\n".join(
            (
                "CMAKE_BUILD_TYPE:STRING=Release",
                "CMAKE_C_COMPILER:FILEPATH=/usr/bin/clang",
                "CMAKE_CXX_COMPILER:FILEPATH=/usr/bin/clang++",
                "CMAKE_GENERATOR:INTERNAL=Unix Makefiles",
                "MOONSHINE_TTS_BUILD_ONNX:BOOL=OFF",
                "MOONSHINE_RELIABILITY:BOOL=OFF",
                "FETCHCONTENT_FULLY_DISCONNECTED:BOOL=ON",
                f"ONNXRUNTIME_LIB_PATH:INTERNAL={ort_path}",
                "",
            )
        )
        (build_dir / "CMakeCache.txt").write_text(cache, encoding="utf-8")

        reply_dir = build_dir / ".cmake" / "api" / "v1" / "reply"
        reply_dir.mkdir(parents=True, exist_ok=True)
        (reply_dir / "index-1.json").write_text(
            json.dumps({"reply": {"codemodel-v2": {"jsonFile": "codemodel-v2.json"}}}),
            encoding="utf-8",
        )
        (reply_dir / "codemodel-v2.json").write_text(
            json.dumps(
                {
                    "configurations": [
                        {
                            "name": "Release",
                            "targets": [
                                {"name": "moonshine", "type": "SHARED_LIBRARY"},
                                {"name": "moonshine_tts_core", "type": "STATIC_LIBRARY"},
                                {"name": "moonshine-test", "type": "EXECUTABLE"},
                                {"name": "benchmark", "type": "EXECUTABLE"},
                            ],
                        }
                    ]
                }
            ),
            encoding="utf-8",
        )

    def _run(
        self,
        *,
        configure_returncode: int = 0,
        configure_output: str = "",
        version: str = "3.30.1",
        timeout: bool = False,
        ort_path: Path | None = None,
        mutate_source: bool = False,
    ):
        calls: list[tuple[list[str], dict[str, object]]] = []
        cmake_commands: list[tuple[list[str], dict[str, object]]] = []

        def fake_run(command, **kwargs):
            command = list(command)
            calls.append((command, kwargs))
            if command[0] == "git":
                if "rev-parse" in command:
                    return subprocess.CompletedProcess(command, 0, probe.SOURCE_REVISION + "\n", "")
                if "status" in command:
                    return subprocess.CompletedProcess(command, 0, "", "")
            if command[0] == "/mock/bin/cmake" and command[1:] == ["--version"]:
                return subprocess.CompletedProcess(command, 0, f"cmake version {version}\n", "")
            if command[0] == "/mock/bin/cmake":
                cmake_commands.append((command, kwargs))
                if timeout:
                    raise subprocess.TimeoutExpired(
                        command,
                        configure.CONFIGURE_TIMEOUT_SECONDS,
                        output="configuring",
                        stderr="timed out",
                    )
                if mutate_source:
                    (self.source / "core" / "CMakeLists.txt").write_bytes(b"changed during configure\n")
                build_dir = Path(command[command.index("-B") + 1])
                if configure_returncode == 0:
                    self._make_cmake_reply(build_dir, ort_path=ort_path)
                stderr = configure_output if configure_returncode else ""
                stdout = configure_output if configure_returncode == 0 else ""
                return subprocess.CompletedProcess(command, configure_returncode, stdout, stderr)
            raise AssertionError(f"Unexpected subprocess command: {command}")

        with (
            mock.patch.object(configure.probe, "verify_sources", side_effect=self._verify_fixture),
            mock.patch.object(configure.shutil, "which", return_value="/mock/bin/cmake"),
            mock.patch.object(configure.subprocess, "run", side_effect=fake_run),
        ):
            result = configure.run_experiment(self.source)
        return result, calls, cmake_commands

    def test_missing_source_directory_is_blocked(self) -> None:
        result = configure.run_experiment(self.root / "missing")
        self.assertEqual(result.status, "BLOCKED")
        self.assertIn("Source directory is unavailable", result.blocker or "")

    def test_invalid_source_revision_is_blocked_before_cmake(self) -> None:
        with (
            mock.patch.object(configure, "_read_git_revision", return_value=("0" * 40, None)),
            mock.patch.object(configure.shutil, "which", side_effect=AssertionError("CMake should not be checked")),
        ):
            result = configure.run_experiment(self.source)
        self.assertEqual(result.status, "BLOCKED")
        self.assertIn("Source revision must be", result.blocker or "")

    def test_modified_pinned_source_file_fails_integrity_probe(self) -> None:
        (self.source / "core" / "bin-tokenizer" / "CMakeLists.txt").write_bytes(b"modified\n")
        result, calls, cmake_commands = self._run()
        self.assertEqual(result.status, "FAILED")
        self.assertEqual(result.source_integrity, "FAILED")
        self.assertIn("blob SHA mismatch", result.diagnostics)
        self.assertEqual(cmake_commands, [])

    def test_missing_cmake_executable_is_blocked(self) -> None:
        with (
            mock.patch.object(configure, "_read_git_revision", return_value=(probe.SOURCE_REVISION, None)),
            mock.patch.object(configure, "_read_git_status", return_value=("", None)),
            mock.patch.object(configure.probe, "verify_sources", side_effect=self._verify_fixture),
            mock.patch.object(configure.shutil, "which", return_value=None),
        ):
            result = configure.run_experiment(self.source)
        self.assertEqual(result.status, "BLOCKED")
        self.assertIn("CMake executable is not available", result.blocker or "")

    def test_unsupported_cmake_version_is_blocked(self) -> None:
        def version_run(command, **kwargs):
            return subprocess.CompletedProcess(command, 0, "cmake version 3.17.9\n", "")

        with (
            mock.patch.object(configure, "_read_git_revision", return_value=(probe.SOURCE_REVISION, None)),
            mock.patch.object(configure, "_read_git_status", return_value=("", None)),
            mock.patch.object(configure.probe, "verify_sources", side_effect=self._verify_fixture),
            mock.patch.object(configure.shutil, "which", return_value="/mock/bin/cmake"),
            mock.patch.object(configure.subprocess, "run", side_effect=version_run),
        ):
            result = configure.run_experiment(self.source)
        self.assertEqual(result.status, "BLOCKED")
        self.assertIn("below upstream minimum 3.18", result.blocker or "")

    def test_configure_arguments_use_only_the_requested_definitions(self) -> None:
        result, _, cmake_commands = self._run()
        self.assertEqual(result.status, "CONFIGURED")
        command, kwargs = cmake_commands[0]
        self.assertEqual(command[1:3], ["-S", str(self.source.resolve() / "core")])
        self.assertEqual(command[3], "-B")
        self.assertEqual(
            tuple(command[-4:]),
            configure.CONFIGURE_DEFINITIONS,
        )
        self.assertFalse(any("MOONSHINE_BUILD_SHARED" in arg or "MOONSHINE_BUILD_SWIFT" in arg for arg in command))
        self.assertEqual(kwargs["timeout"], configure.CONFIGURE_TIMEOUT_SECONDS)

    def test_build_directory_is_temporary_and_out_of_source(self) -> None:
        result, _, cmake_commands = self._run()
        command, kwargs = cmake_commands[0]
        build_dir = Path(command[command.index("-B") + 1])
        self.assertEqual(result.status, "CONFIGURED")
        self.assertNotEqual(build_dir, self.source)
        self.assertNotIn(self.source, build_dir.parents)
        self.assertEqual(Path(kwargs["cwd"]), build_dir)
        self.assertIn("TEMPORARY", configure.render_result(result))

    def test_successful_configuration_inspects_targets_and_cache(self) -> None:
        result, _, _ = self._run()
        rendered = configure.render_result(result)
        self.assertEqual(result.status, "CONFIGURED")
        self.assertEqual(result.target_graph_status, "INSPECTED")
        self.assertIn("moonshine target defined: moonshine (SHARED_LIBRARY)", rendered)
        self.assertIn("moonshine_tts_core (STATIC_LIBRARY)", rendered)
        self.assertIn("moonshine-test (EXECUTABLE)", rendered)
        self.assertIn("benchmark (EXECUTABLE)", rendered)
        self.assertIn("CMAKE_C_COMPILER: /usr/bin/clang", rendered)
        self.assertIn("MOONSHINE_TTS_BUILD_ONNX: OFF", rendered)

    def test_failed_configure_result_is_not_reported_as_success(self) -> None:
        result, _, _ = self._run(configure_returncode=1, configure_output="CMake Error at core/CMakeLists.txt: invalid project setup")
        self.assertEqual(result.status, "FAILED")
        self.assertEqual(result.configure_return_code, 1)
        self.assertIn("CMake configuration: FAILED", configure.render_result(result))

    def test_missing_configure_dependency_is_reported_as_blocked(self) -> None:
        result, _, _ = self._run(configure_returncode=1, configure_output="Could NOT find ONNXRuntime (missing: ONNXRUNTIME_LIB_PATH)")
        self.assertEqual(result.status, "BLOCKED")
        self.assertIn("missing dependency", result.blocker or "")

    def test_missing_native_compiler_is_reported_as_blocked(self) -> None:
        result, _, _ = self._run(
            configure_returncode=1,
            configure_output="No CMAKE_C_COMPILER could be found.",
        )
        self.assertEqual(result.status, "BLOCKED")
        self.assertIn("missing dependency", result.blocker or "")

    def test_configure_timeout_is_blocked(self) -> None:
        result, _, cmake_commands = self._run(timeout=True)
        self.assertEqual(result.status, "BLOCKED")
        self.assertTrue(result.configure_attempted)
        self.assertIn("exceeded", result.blocker or "")
        self.assertEqual(len(cmake_commands), 1)

    def test_source_checkout_is_verified_unchanged_after_attempt(self) -> None:
        before = configure.source_tree_fingerprint(self.source)
        result, _, _ = self._run()
        after = configure.source_tree_fingerprint(self.source)
        self.assertEqual(result.source_unchanged, "VERIFIED")
        self.assertEqual(before, after)

    def test_source_modification_during_configure_fails_the_experiment(self) -> None:
        result, _, _ = self._run(mutate_source=True)
        rendered = configure.render_result(result)
        self.assertEqual(result.configure_status, "CONFIGURED")
        self.assertEqual(result.status, "FAILED")
        self.assertEqual(result.source_unchanged, "FAILED")
        self.assertIn("CMake configuration: CONFIGURED", rendered)
        self.assertIn("Experiment outcome: FAILED", rendered)

    def test_temporary_build_directory_is_cleaned_up(self) -> None:
        result, _, cmake_commands = self._run()
        build_dir = Path(cmake_commands[0][0][cmake_commands[0][0].index("-B") + 1])
        self.assertEqual(result.status, "CONFIGURED")
        self.assertFalse(build_dir.exists())

    def test_runner_never_invokes_a_production_build_command(self) -> None:
        result, calls, _ = self._run()
        flattened = [arg for command, _ in calls for arg in command]
        self.assertEqual(result.status, "CONFIGURED")
        self.assertNotIn("--build", flattened)
        self.assertNotIn("--install", flattened)
        self.assertNotIn("ctest", flattened)

    def test_lfs_pointer_is_not_reported_as_a_verified_native_binary(self) -> None:
        pointer = self.source / "core" / "third-party" / "onnxruntime" / "libonnxruntime.dylib"
        pointer.parent.mkdir(parents=True, exist_ok=True)
        pointer.write_text(
            "version https://git-lfs.github.com/spec/v1\noid sha256:" + "a" * 64 + "\nsize 123\n",
            encoding="ascii",
        )
        result, _, _ = self._run(ort_path=pointer)
        self.assertEqual(result.status, "CONFIGURED")
        self.assertIn("LFS POINTER", result.onnx_library_state)
        self.assertNotIn("VERIFIED BINARY", result.onnx_library_state)

    def test_missing_onnx_library_is_reported_after_successful_configuration(self) -> None:
        result, _, _ = self._run()
        self.assertEqual(result.status, "CONFIGURED")
        self.assertIn("MISSING", result.onnx_library_state)
        self.assertIn("ONNX Runtime library status: MISSING", configure.render_result(result))

    def test_success_does_not_certify_a_minimal_stt_build(self) -> None:
        result, _, _ = self._run()
        rendered = configure.render_result(result)
        self.assertEqual(result.status, "CONFIGURED")
        self.assertIn("Minimal STT build: NOT PROVEN", rendered)
        self.assertIn("Binary contents: NOT VERIFIED", rendered)


if __name__ == "__main__":
    unittest.main()
