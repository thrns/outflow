"""Deterministic synthetic coverage for the offline Moonshine readiness check."""

from __future__ import annotations

import json
import hashlib
from pathlib import Path
import struct
import subprocess
import tempfile
import unittest

from scripts import moonshine_native_preflight as preflight


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PINNED_REVISION = preflight._expected_source_revision(REPOSITORY_ROOT)
ORT_RELATIVE_PATH = preflight.ONNX_RUNTIME_RELATIVE_PATH


def _synthetic_macho(architecture: str = "arm64") -> bytes:
    cpu_types = {"arm64": 0x0100000C, "x86_64": 0x01000007}
    return struct.pack("<IiiIIIII", 0xFEEDFACF, cpu_types[architecture], 0, 6, 0, 0, 0, 0) + b"fixture"


SYNTHETIC_ORT = _synthetic_macho()
SYNTHETIC_LFS_POINTER = (
    f"{preflight.LFS_POINTER_VERSION}\n"
    f"oid sha256:{hashlib.sha256(SYNTHETIC_ORT).hexdigest()}\n"
    f"size {len(SYNTHETIC_ORT)}\n"
)


def _completed(command: list[str], stdout: str = "", stderr: str = "", returncode: int = 0):
    return subprocess.CompletedProcess(command, returncode, stdout=stdout, stderr=stderr)


class FakeEnvironment:
    """Inject local tool and Git answers without depending on installed tools."""

    def __init__(
        self,
        source_dir: Path | None,
        *,
        cmake_path: str | None = "/fixture/bin/cmake",
        cmake_output: str = "cmake version 3.30.1\n",
        cmake_returncode: int = 0,
        c_path: str | None = "/fixture/bin/cc",
        cxx_path: str | None = "/fixture/bin/c++",
        git_revision: str | None = PINNED_REVISION,
        git_status: str = "",
        lfs_pointer: str | None = SYNTHETIC_LFS_POINTER,
    ) -> None:
        self.source_dir = source_dir
        self.cmake_path = cmake_path
        self.cmake_output = cmake_output
        self.cmake_returncode = cmake_returncode
        self.c_path = c_path
        self.cxx_path = cxx_path
        self.git_revision = git_revision
        self.git_status = git_status
        self.lfs_pointer = lfs_pointer
        self.commands: list[list[str]] = []

    def which(self, command: str) -> str | None:
        paths = {
            "cmake": self.cmake_path,
            "cc": self.c_path,
            "c++": self.cxx_path,
            "clang": None,
            "gcc": None,
            "clang++": None,
            "g++": None,
        }
        return paths.get(command)

    def run(self, command: list[str], **kwargs):
        self.commands.append(list(command))
        if command[-1:] == ["--version"] and command[0] == self.cmake_path:
            return _completed(
                command,
                stdout=self.cmake_output,
                returncode=self.cmake_returncode,
            )
        if command[:2] == ["git", "-C"]:
            source_path = Path(command[2])
            operation = command[3:]
            if operation == ["rev-parse", "--show-toplevel"]:
                return _completed(command, stdout=f"{source_path}\n")
            if operation == ["rev-parse", "HEAD"] and self.git_revision is not None:
                return _completed(command, stdout=f"{self.git_revision}\n")
            if operation == ["status", "--porcelain", "--untracked-files=all"]:
                return _completed(command, stdout=self.git_status)
            if operation == ["show", f"HEAD:{ORT_RELATIVE_PATH.as_posix()}"]:
                if self.lfs_pointer is None:
                    return _completed(command, stderr="missing blob", returncode=1)
                return _completed(command, stdout=self.lfs_pointer)
            return _completed(command, stderr="unavailable", returncode=1)
        raise AssertionError(f"Unexpected subprocess command: {command}")


class MoonshineNativePreflightTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="moonshine-preflight-test-")
        self.root = Path(self.temporary.name)
        self.source = self.root / "moonshine-checkout"
        self.make_source()
        self.ort_path = self.source / ORT_RELATIVE_PATH
        self.ort_path.parent.mkdir(parents=True, exist_ok=True)
        self.ort_path.write_bytes(SYNTHETIC_ORT)

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def make_source(self, *, header: bool = True) -> Path:
        (self.source / "core").mkdir(parents=True, exist_ok=True)
        (self.source / "core" / "CMakeLists.txt").write_text(
            "cmake_minimum_required(VERSION 3.20)\n", encoding="utf-8"
        )
        if header:
            include = self.source / "include"
            include.mkdir(exist_ok=True)
            (include / "moonshine.h").write_text(
                "#define MOONSHINE_HEADER_VERSION 30000\n"
                "void *moonshine_load_transcriber_from_files(void);\n",
                encoding="utf-8",
            )
        return self.source

    def inspect(self, fake: FakeEnvironment, **overrides) -> preflight.PreflightReport:
        arguments = {
            "repository_root": REPOSITORY_ROOT,
            "source_dir": self.source,
            "asset_root": self.root,
            "system": "Darwin",
            "machine": "arm64",
            "environment": {},
            "which": fake.which,
            "run": fake.run,
        }
        arguments.update(overrides)
        return preflight.inspect_preflight(**arguments)

    def check(self, report: preflight.PreflightReport, name: str) -> preflight.Check:
        return next(item for item in report.checks if item.name == name)

    def write_inventory(self, model_path: str | None, tokenizer_path: str | None) -> Path:
        data = json.loads((REPOSITORY_ROOT / "docs/model-asset-inventory.json").read_text(encoding="utf-8"))
        for asset in data["assets"]:
            if asset["category"] == "model":
                asset["file_path"] = model_path
            elif asset["category"] == "tokenizer":
                asset["file_path"] = tokenizer_path
        path = self.root / "asset-inventory.json"
        path.write_text(json.dumps(data), encoding="utf-8")
        return path

    def test_missing_cmake_is_reported_without_calling_it(self) -> None:
        fake = FakeEnvironment(self.source, cmake_path=None)
        report = self.inspect(fake)
        self.assertEqual(self.check(report, "CMake executable").status, "MISSING")
        self.assertEqual(self.check(report, "CMake version").status, "CANNOT DETERMINE")
        self.assertTrue(any(check.name == "Offline configuration readiness" and check.status == "BLOCKED" for check in report.checks))
        self.assertFalse(any(command and command[0] == "/fixture/bin/cmake" for command in fake.commands))

    def test_available_cmake_with_valid_version_is_detected(self) -> None:
        report = self.inspect(FakeEnvironment(self.source))
        self.assertEqual(self.check(report, "CMake executable").status, "AVAILABLE")
        self.assertIn("3.30.1", " ".join(self.check(report, "CMake version").details))
        self.assertEqual(self.check(report, "CMake version").status, "DETECTED")

    def test_cmake_below_pinned_source_minimum_blocks_readiness(self) -> None:
        fake = FakeEnvironment(self.source, cmake_output="cmake version 3.19.9\n")
        report = self.inspect(fake)
        version = self.check(report, "CMake version")
        self.assertEqual(version.status, "DETECTED")
        self.assertTrue(version.blocks_configure)
        self.assertTrue(any("below the pinned source minimum" in detail for detail in version.details))
        self.assertFalse(report.ready)

    def test_unreadable_cmake_version_output_cannot_be_determined(self) -> None:
        fake = FakeEnvironment(self.source, cmake_output="CMake from a wrapper\n")
        report = self.inspect(fake)
        self.assertEqual(self.check(report, "CMake executable").status, "AVAILABLE")
        self.assertEqual(self.check(report, "CMake version").status, "CANNOT DETERMINE")
        self.assertIn(["/fixture/bin/cmake", "--version"], fake.commands)

    def test_missing_c_or_cxx_compiler_is_reported(self) -> None:
        fake = FakeEnvironment(self.source, c_path=None)
        report = self.inspect(fake)
        compiler = self.check(report, "C/C++ compiler")
        self.assertEqual(compiler.status, "MISSING")
        self.assertTrue(any("C compiler missing" in detail for detail in compiler.details))
        self.assertTrue(any("command -v cc" in detail for detail in compiler.details))

    def test_missing_source_checkout_is_actionable(self) -> None:
        fake = FakeEnvironment(self.source)
        report = self.inspect(fake, source_dir=self.root / "absent-moonshine")
        source_check = self.check(report, "Moonshine source directory")
        self.assertEqual(source_check.status, "MISSING")
        self.assertTrue(any(str(self.root / "absent-moonshine") in line for line in source_check.details))
        self.assertEqual(self.check(report, "Moonshine C API header").status, "NOT VERIFIED")

    def test_source_directory_without_c_api_header_is_blocked(self) -> None:
        self.make_source(header=False)
        (self.source / "include" / "moonshine.h").unlink()
        report = self.inspect(FakeEnvironment(self.source))
        self.assertEqual(self.check(report, "Moonshine source directory").status, "PRESENT")
        self.assertEqual(self.check(report, "Moonshine C API header").status, "MISSING")
        self.assertEqual(self.check(report, "Source revision").status, "VERIFIED")
        self.assertFalse(report.ready)

    def test_moonshine_named_directory_without_cmake_entry_point_is_missing(self) -> None:
        (self.source / "core" / "CMakeLists.txt").unlink()
        report = self.inspect(FakeEnvironment(self.source))
        source = self.check(report, "Moonshine source directory")
        self.assertEqual(source.status, "MISSING")
        self.assertTrue(any("core/CMakeLists.txt" in detail for detail in source.details))
        self.assertFalse(report.ready)

    def test_valid_local_source_and_header_fixture_is_detected(self) -> None:
        report = self.inspect(FakeEnvironment(self.source))
        self.assertEqual(self.check(report, "Moonshine source directory").status, "PRESENT")
        header = self.check(report, "Moonshine C API header")
        self.assertEqual(header.status, "PRESENT")
        self.assertTrue(any("MOONSHINE_HEADER_VERSION=30000" in detail for detail in header.details))

    def test_parenthesized_upstream_header_version_is_detected(self) -> None:
        header_path = self.source / "include" / "moonshine.h"
        header_path.write_text(
            "#define MOONSHINE_HEADER_VERSION (30000)\n"
            "void *moonshine_load_transcriber_from_files(void);\n",
            encoding="utf-8",
        )

        report = self.inspect(FakeEnvironment(self.source))
        header = self.check(report, "Moonshine C API header")
        self.assertEqual(header.status, "PRESENT")
        self.assertTrue(any("MOONSHINE_HEADER_VERSION=30000" in detail for detail in header.details))

    def test_source_revision_missing_or_unverified_is_reported(self) -> None:
        fake = FakeEnvironment(self.source, git_revision="0" * 40)
        report = self.inspect(fake)
        revision = self.check(report, "Source revision")
        self.assertEqual(revision.status, "UNVERIFIED")
        self.assertTrue(any("Expected pinned revision" in detail for detail in revision.details))
        self.assertFalse(report.ready)

    def test_required_onnx_runtime_file_missing_blocks_readiness(self) -> None:
        self.ort_path.unlink()
        report = self.inspect(FakeEnvironment(self.source))
        artifact = self.check(report, "ONNX Runtime native library")
        self.assertEqual(artifact.status, "MISSING")
        self.assertTrue(artifact.blocks_configure)
        self.assertFalse(report.ready)

    def test_valid_git_lfs_pointer_is_detected(self) -> None:
        self.ort_path.write_text(SYNTHETIC_LFS_POINTER, encoding="ascii")
        report = self.inspect(FakeEnvironment(self.source))
        artifact = self.check(report, "ONNX Runtime native library")
        self.assertEqual(artifact.status, "LFS_POINTER_NOT_MATERIALIZED")
        self.assertTrue(artifact.blocks_configure)
        self.assertIn("pointer text", " ".join(artifact.details))
        self.assertFalse(report.ready)

    def test_lfs_pointer_is_not_treated_as_a_native_library(self) -> None:
        self.ort_path.write_text(SYNTHETIC_LFS_POINTER, encoding="ascii")
        result = preflight.verify_native_artifact(
            self.ort_path,
            SYNTHETIC_LFS_POINTER,
            expected_architecture="arm64",
        )
        self.assertEqual(result.status, "LFS_POINTER_NOT_MATERIALIZED")
        self.assertTrue(result.blocks_configure)

    def test_incorrect_lfs_artifact_size_is_rejected(self) -> None:
        pointer = SYNTHETIC_LFS_POINTER.replace(
            f"size {len(SYNTHETIC_ORT)}", f"size {len(SYNTHETIC_ORT) + 1}"
        )
        result = preflight.verify_native_artifact(
            self.ort_path,
            pointer,
            expected_architecture="arm64",
        )
        self.assertEqual(result.status, "PRESENT_UNVERIFIED")
        self.assertTrue(any("size mismatch" in detail for detail in result.details))

    def test_incorrect_lfs_artifact_checksum_is_rejected(self) -> None:
        pointer = SYNTHETIC_LFS_POINTER.replace(hashlib.sha256(SYNTHETIC_ORT).hexdigest(), "0" * 64)
        result = preflight.verify_native_artifact(
            self.ort_path,
            pointer,
            expected_architecture="arm64",
        )
        self.assertEqual(result.status, "PRESENT_UNVERIFIED")
        self.assertTrue(any("SHA-256 mismatch" in detail for detail in result.details))

    def test_verified_synthetic_arm64_artifact_is_accepted(self) -> None:
        result = preflight.verify_native_artifact(
            self.ort_path,
            SYNTHETIC_LFS_POINTER,
            expected_architecture="arm64",
        )
        self.assertEqual(result.status, "VERIFIED")
        self.assertFalse(result.blocks_configure)

    def test_unsupported_or_unverified_architecture_blocks_readiness(self) -> None:
        x86_artifact = self.root / "x86_64.dylib"
        x86_bytes = _synthetic_macho("x86_64")
        x86_artifact.write_bytes(x86_bytes)
        x86_pointer = (
            f"{preflight.LFS_POINTER_VERSION}\n"
            f"oid sha256:{hashlib.sha256(x86_bytes).hexdigest()}\n"
            f"size {len(x86_bytes)}\n"
        )
        unsupported = preflight.verify_native_artifact(
            x86_artifact,
            x86_pointer,
            expected_architecture="arm64",
        )
        self.assertEqual(unsupported.status, "PRESENT_UNVERIFIED")
        self.assertTrue(any("Architecture mismatch" in detail for detail in unsupported.details))

        unknown_artifact = self.root / "unknown.dylib"
        unknown_bytes = b"not a Mach-O library"
        unknown_artifact.write_bytes(unknown_bytes)
        unknown_pointer = (
            f"{preflight.LFS_POINTER_VERSION}\n"
            f"oid sha256:{hashlib.sha256(unknown_bytes).hexdigest()}\n"
            f"size {len(unknown_bytes)}\n"
        )
        unverified = preflight.verify_native_artifact(
            unknown_artifact,
            unknown_pointer,
            expected_architecture="arm64",
        )
        self.assertEqual(unverified.status, "PRESENT_UNVERIFIED")
        self.assertTrue(any("not a recognized" in detail for detail in unverified.details))

    def test_preflight_never_downloads_or_replaces_lfs_pointer(self) -> None:
        self.ort_path.write_text(SYNTHETIC_LFS_POINTER, encoding="ascii")
        before = self.ort_path.read_bytes()
        fake = FakeEnvironment(self.source)
        report = self.inspect(fake)
        self.assertEqual(self.check(report, "ONNX Runtime native library").status, "LFS_POINTER_NOT_MATERIALIZED")
        self.assertEqual(self.ort_path.read_bytes(), before)
        self.assertFalse(
            any(token in {"lfs", "fetch", "pull", "checkout", "curl", "wget"} for command in fake.commands for token in command)
        )

    def test_missing_model_inventory_does_not_block_native_configuration(self) -> None:
        missing_inventory = self.root / "missing-inventory.json"
        report = self.inspect(
            FakeEnvironment(self.source),
            asset_inventory_path=missing_inventory,
        )
        self.assertEqual(self.check(report, "Local model manifest").status, "NOT CONFIGURED")
        self.assertEqual(self.check(report, "Model artifacts").status, "NOT ASSESSED")
        self.assertTrue(report.ready)

    def test_declared_model_files_missing_from_present_inventory(self) -> None:
        manifest = self.write_inventory("models/small.ort", "models/tokenizer.bin")
        report = self.inspect(
            FakeEnvironment(self.source),
            asset_inventory_path=manifest,
        )
        self.assertEqual(self.check(report, "Local model manifest").status, "NOT CONFIGURED")
        artifacts = self.check(report, "Model artifacts")
        self.assertEqual(artifacts.status, "MISSING")
        self.assertTrue(any("models/small.ort" in detail for detail in artifacts.details))
        self.assertTrue(report.ready, "model assets are not required for a native CMake configure")

    def test_complete_synthetic_prerequisite_fixture_is_ready_to_attempt(self) -> None:
        (self.root / "models").mkdir()
        (self.root / "models/small.ort").write_bytes(b"synthetic model placeholder")
        (self.root / "models/tokenizer.bin").write_bytes(b"synthetic tokenizer placeholder")
        manifest = self.write_inventory("models/small.ort", "models/tokenizer.bin")
        report = self.inspect(
            FakeEnvironment(self.source),
            asset_inventory_path=manifest,
        )
        self.assertTrue(report.ready)
        self.assertEqual(self.check(report, "Offline configuration readiness").status, "READY_TO_ATTEMPT_CONFIGURE")
        self.assertEqual(self.check(report, "Model artifacts").status, "PRESENT")
        self.assertIn("do not verify checksums", " ".join(self.check(report, "Model artifacts").details).lower())

    def test_status_order_and_rendering_are_deterministic(self) -> None:
        report = self.inspect(FakeEnvironment(self.source))
        first = preflight.render_report(report)
        second = preflight.render_report(report)
        self.assertEqual(first, second)
        labels = [line.split(":", 1)[0] for line in first.splitlines() if ":" in line and not line.startswith("  ")]
        self.assertEqual(
            labels,
            [
                "Host OS and architecture",
                "CMake executable",
                "CMake version",
                "C/C++ compiler",
                "Moonshine source directory",
                "Moonshine C API header",
                "Source revision",
                "ONNX Runtime native library",
                "Local model manifest",
                "Model artifacts",
                "Offline configuration readiness",
            ],
        )

    def test_only_local_read_only_version_and_git_commands_are_run(self) -> None:
        fake = FakeEnvironment(self.source)
        self.inspect(fake)
        self.assertIn(["/fixture/bin/cmake", "--version"], fake.commands)
        for command in fake.commands:
            self.assertIn(command[0], {"git", "/fixture/bin/cmake"})
            if command[0] == "/fixture/bin/cmake":
                self.assertEqual(command[1:], ["--version"])
            else:
                self.assertIn(command[3:], [
                    ["rev-parse", "--show-toplevel"],
                    ["rev-parse", "HEAD"],
                    ["status", "--porcelain", "--untracked-files=all"],
                    ["show", f"HEAD:{ORT_RELATIVE_PATH.as_posix()}"],
                ])
        self.assertFalse(any(token in {"clone", "fetch", "pull", "install", "configure", "build"} for command in fake.commands for token in command))

    def test_source_and_model_files_are_unchanged(self) -> None:
        (self.root / "models").mkdir()
        (self.root / "models/small.ort").write_bytes(b"synthetic model placeholder")
        (self.root / "models/tokenizer.bin").write_bytes(b"synthetic tokenizer placeholder")
        manifest = self.write_inventory("models/small.ort", "models/tokenizer.bin")

        def snapshot(root: Path) -> dict[str, bytes]:
            return {
                path.relative_to(root).as_posix(): path.read_bytes()
                for path in root.rglob("*")
                if path.is_file()
            }

        before_source = snapshot(self.source)
        before_assets = snapshot(self.root / "models")
        before_manifest = manifest.read_bytes()
        self.inspect(FakeEnvironment(self.source), asset_inventory_path=manifest)
        self.assertEqual(snapshot(self.source), before_source)
        self.assertEqual(snapshot(self.root / "models"), before_assets)
        self.assertEqual(manifest.read_bytes(), before_manifest)


if __name__ == "__main__":
    unittest.main()
