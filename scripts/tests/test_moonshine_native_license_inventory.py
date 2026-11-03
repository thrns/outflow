from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from types import SimpleNamespace
from unittest import mock

from scripts import moonshine_native_license_inventory as inventory


REVISION = inventory.PINNED_REVISION


class MoonshineNativeLicenseInventoryTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        root = Path(self.temporary.name)
        self.source = root / "moonshine"
        self.build = root / "build"
        self._create_source()
        self._create_build()
        self.ort_bytes = b"synthetic local runtime artifact"
        self._write(self.source / inventory.ORT_RELATIVE_PATH, self.ort_bytes)

    def _write(self, path: Path, contents: str | bytes) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(contents.encode("utf-8") if isinstance(contents, str) else contents)

    def _create_source(self) -> None:
        self._write(self.source / "core/CMakeLists.txt", "project(moonshine)\n")
        licenses = {
            "LICENSE": "Moonshine MIT evidence fixture\n",
            "docs/license.md": "Moonshine source scope evidence fixture\n",
            "core/cpp-annote/LICENSE": "cpp-annote MIT evidence fixture\n",
            "core/third-party/Eigen/COPYING.MPL2": "Eigen MPL evidence fixture\n",
            "core/third-party/kaldi-native-fbank/LICENSE": "Kaldi Apache evidence fixture\n",
            "core/third-party/kissfft/COPYING": "KISS FFT refers to LICENSES/BSD-3-Clause\n",
            "core/third-party/utf-8/LICENSE.txt": "UTF8-CPP Boost evidence fixture\n",
            "core/third-party/onnxruntime/LICENSE.txt": "ONNX Runtime MIT evidence fixture\n",
            "core/third-party/doctest/LICENSE.txt": "doctest MIT evidence fixture\n",
            "core/third-party/nlohmann/LICENSE.MIT": "nlohmann MIT evidence fixture\n",
            "core/third-party/utf8proc/LICENSE.md": "utf8proc evidence fixture\n",
        }
        for path, contents in licenses.items():
            self._write(self.source / path, contents)

    def _create_build(self) -> None:
        self.build.mkdir(parents=True, exist_ok=True)
        ort_path = (self.source / inventory.ORT_RELATIVE_PATH).as_posix()
        cache = "\n".join(
            (
                f"CMAKE_HOME_DIRECTORY:INTERNAL={(self.source / 'core').as_posix()}",
                "CMAKE_OSX_ARCHITECTURES:STRING=arm64",
                "CMAKE_BUILD_TYPE:STRING=Release",
                f"ONNXRUNTIME_LIB_PATH:INTERNAL={ort_path}",
                "MOONSHINE_TTS_BUILD_ONNX:BOOL=OFF",
            )
        )
        self._write(self.build / "CMakeCache.txt", cache + "\n")
        self._write(
            self.build / "CMakeFiles/4.4.5/CMakeSystem.cmake",
            'set(CMAKE_HOST_SYSTEM_NAME "Darwin")\n'
            'set(CMAKE_SYSTEM_NAME "Darwin")\n'
            'set(CMAKE_HOST_SYSTEM_PROCESSOR "arm64")\n'
            'set(CMAKE_SYSTEM_PROCESSOR "arm64")\n',
        )
        reply = self.build / ".cmake/api/v1/reply"
        reply.mkdir(parents=True, exist_ok=True)
        self._write(
            reply / "index-2026-11-03T00-00-00.json",
            json.dumps({"reply": {"codemodel-v2": {"jsonFile": "codemodel-v2.json"}}}),
        )
        target_names = ("moonshine", "ort-utils", "moonshine-utils", "bin-tokenizer", "benchmark", "moonshine_tts_core", "moonshine-c-api-test")
        target_summaries = [
            {"name": name, "id": f"{name}::@fixture", "jsonFile": f"target-{name}.json"}
            for name in target_names
        ]
        self._write(
            reply / "codemodel-v2.json",
            json.dumps(
                {
                    "paths": {"source": (self.source / "core").as_posix(), "build": self.build.as_posix()},
                    "configurations": [{"targets": target_summaries}],
                }
            ),
        )
        target_data = {
            "moonshine": {
                "type": "SHARED_LIBRARY",
                "dependencies": [
                    {"id": "ort-utils::@fixture"},
                    {"id": "moonshine-utils::@fixture"},
                    {"id": "bin-tokenizer::@fixture"},
                ],
                "sources": [
                    {"path": "moonshine-c-api.cpp"},
                    {"path": "cpp-annote/src/cpp-annote.cpp"},
                    {"path": "third-party/kaldi-native-fbank/csrc/feature-fbank.cc"},
                    {"path": "third-party/kissfft/kiss_fft.c"},
                ],
                "compileGroups": [
                    {
                        "includes": [
                            {"path": (self.source / "core/third-party").as_posix()},
                            {"path": (self.source / "core/third-party/utf-8").as_posix()},
                        ],
                        "defines": [
                            {"define": "EIGEN_MPL2_ONLY=1"},
                            {"define": "MOONSHINE_C_API_MOONSHINE_TTS=0"},
                        ],
                    }
                ],
                "link": {
                    "commandFragments": [
                        {"fragment": ort_path, "role": "libraries"},
                        {"fragment": "-Wl,-rpath,@loader_path", "role": "libraries"},
                    ]
                },
            },
            "ort-utils": {"type": "STATIC_LIBRARY", "dependencies": [], "sources": []},
            "moonshine-utils": {"type": "STATIC_LIBRARY", "dependencies": [], "sources": []},
            "bin-tokenizer": {"type": "STATIC_LIBRARY", "dependencies": [], "sources": []},
            "benchmark": {"type": "EXECUTABLE", "dependencies": [], "sources": [{"path": "benchmark.cpp"}]},
            "moonshine_tts_core": {"type": "STATIC_LIBRARY", "dependencies": [], "sources": []},
            "moonshine-c-api-test": {"type": "EXECUTABLE", "dependencies": [], "sources": []},
        }
        for name, data in target_data.items():
            self._write(reply / f"target-{name}.json", json.dumps(data))

    def _git_mock(self, revision: str = REVISION, dirty: str = ""):
        def run(arguments, **_kwargs):
            if arguments[-2:] == ["rev-parse", "HEAD"]:
                return SimpleNamespace(returncode=0, stdout=revision + "\n", stderr="")
            if arguments[-3:] == ["status", "--porcelain", "--untracked-files=all"]:
                return SimpleNamespace(returncode=0, stdout=dirty, stderr="")
            raise AssertionError(f"unexpected external command: {arguments}")

        return run

    def _collect(self):
        expected = hashlib.sha256(self.ort_bytes).hexdigest()
        with mock.patch.object(inventory, "ORT_SHA256", expected), mock.patch.object(
            inventory.subprocess, "run", side_effect=self._git_mock()
        ):
            return inventory.collect_inventory(self.source, self.build)

    def _records(self):
        _, records, _ = self._collect()
        return {record.name: record for record in records}

    def test_missing_source_checkout_is_rejected(self) -> None:
        with self.assertRaisesRegex(inventory.InventoryError, "source checkout is missing"):
            inventory.collect_inventory(self.source / "missing", self.build)

    def test_incorrect_pinned_revision_is_rejected(self) -> None:
        with mock.patch.object(inventory.subprocess, "run", side_effect=self._git_mock("0" * 40)):
            with self.assertRaisesRegex(inventory.InventoryError, "not the pinned revision"):
                inventory.collect_inventory(self.source, self.build)

    def test_missing_cmake_cache_is_rejected(self) -> None:
        (self.build / "CMakeCache.txt").unlink()
        with mock.patch.object(inventory.subprocess, "run", side_effect=self._git_mock()):
            with self.assertRaisesRegex(inventory.InventoryError, "CMake cache is missing"):
                inventory.collect_inventory(self.source, self.build)

    def test_cache_pointing_at_different_source_is_rejected(self) -> None:
        cache_path = self.build / "CMakeCache.txt"
        cache_path.write_text(cache_path.read_text().replace(self.source.as_posix(), "/different/source"))
        with mock.patch.object(inventory.subprocess, "run", side_effect=self._git_mock()):
            with self.assertRaisesRegex(inventory.InventoryError, "cache source does not match"):
                inventory.collect_inventory(self.source, self.build)

    def test_valid_license_file_is_discovered(self) -> None:
        record = self._records()["Moonshine streaming C API target"]
        self.assertEqual(record.evidence, ("LICENSE", "docs/license.md"))
        self.assertEqual(record.status, "EVIDENCE_PRESENT")

    def test_missing_license_evidence_is_reported(self) -> None:
        (self.source / "core/third-party/doctest/LICENSE.txt").unlink()
        record = self._records()["doctest"]
        self.assertEqual(record.status, "MISSING")
        self.assertFalse(record.evidence)

    def test_license_file_sha256_is_calculated_from_bytes(self) -> None:
        path = self.source / "core/third-party/kaldi-native-fbank/LICENSE"
        record = self._records()["kaldi-native-fbank"]
        self.assertEqual(record.evidence_hashes, (hashlib.sha256(path.read_bytes()).hexdigest(),))

    def test_runtime_and_optional_components_are_distinguished(self) -> None:
        records = self._records()
        self.assertEqual(records["ort-utils"].role, "RUNTIME")
        self.assertEqual(records["KISS FFT"].role, "RUNTIME")
        self.assertEqual(records["doctest"].role, "BUILD-ONLY")
        self.assertEqual(records["nlohmann/json"].role, "OPTIONAL")
        self.assertEqual(records["Moonshine TTS/G2P target family"].role, "OPTIONAL")

    def test_unresolved_dependencies_remain_unresolved(self) -> None:
        target = self.build / ".cmake/api/v1/reply/target-moonshine.json"
        data = json.loads(target.read_text())
        data["link"]["commandFragments"] = [
            fragment for fragment in data["link"]["commandFragments"] if "onnxruntime" not in fragment["fragment"]
        ]
        target.write_text(json.dumps(data))
        records = self._records()
        self.assertEqual(records["ONNX Runtime"].role, "UNRESOLVED")
        self.assertEqual(records["KISS FFT"].status, "UNRESOLVED")
        self.assertEqual(records["macOS system library/framework closure"].status, "UNRESOLVED")

    def test_component_order_and_rendered_output_are_deterministic(self) -> None:
        details_a, records_a, _ = self._collect()
        details_b, records_b, _ = self._collect()
        self.assertEqual([r.name for r in records_a], sorted((r.name for r in records_a), key=str.casefold))
        report = inventory.render_report(details_a, records_a)
        self.assertEqual(report, inventory.render_report(details_b, records_b))
        self.assertNotIn(self.temporary.name, report)

    def test_inventory_uses_only_local_git_metadata_queries(self) -> None:
        calls = []

        def run(arguments, **kwargs):
            calls.append(arguments)
            return self._git_mock()(arguments, **kwargs)

        with mock.patch.object(inventory, "ORT_SHA256", hashlib.sha256(self.ort_bytes).hexdigest()), mock.patch.object(
            inventory.subprocess, "run", side_effect=run
        ):
            inventory.collect_inventory(self.source, self.build)
        self.assertEqual([call[-2:] for call in calls], [["rev-parse", "HEAD"], ["--porcelain", "--untracked-files=all"]])
        self.assertTrue(all(call[0] == "git" for call in calls))

    def test_inspected_source_and_build_files_remain_unchanged(self) -> None:
        def snapshot(root: Path) -> dict[str, bytes]:
            return {path.relative_to(root).as_posix(): path.read_bytes() for path in root.rglob("*") if path.is_file()}

        before_source = snapshot(self.source)
        before_build = snapshot(self.build)
        self._collect()
        self.assertEqual(snapshot(self.source), before_source)
        self.assertEqual(snapshot(self.build), before_build)


if __name__ == "__main__":
    unittest.main()
