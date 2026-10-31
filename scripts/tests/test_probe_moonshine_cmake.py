"""Synthetic-source tests for the read-only Moonshine CMake probe."""

from __future__ import annotations

import tempfile
from pathlib import Path
import urllib.request
from unittest import mock
import unittest

from scripts import probe_moonshine_cmake as probe


def synthetic_sources() -> dict[str, bytes]:
    """Return small CMake fixtures with the expected source relationships."""
    return {
        "core/CMakeLists.txt": b'''\
option(MOONSHINE_RELIABILITY "Reliability instrumentation" OFF)
set(MOONSHINE_SANITIZER "address,undefined" CACHE STRING "Sanitizers")
if(MOONSHINE_RELIABILITY)
  add_compile_options(-fsanitize=${MOONSHINE_SANITIZER})
  add_link_options(-fsanitize=${MOONSHINE_SANITIZER})
endif()
if (NOT MOONSHINE_BUILD_SHARED AND (IOS OR MOONSHINE_BUILD_SWIFT))
  add_library(moonshine STATIC ${MOONSHINE_SRC_FILES})
else()
  add_library(moonshine SHARED ${MOONSHINE_SRC_FILES})
endif()
add_subdirectory(${CMAKE_CURRENT_LIST_DIR}/ort-utils ${CMAKE_BINARY_DIR}/ort-utils)
add_subdirectory(${CMAKE_CURRENT_LIST_DIR}/bin-tokenizer ${CMAKE_BINARY_DIR}/bin-tokenizer)
add_subdirectory(${CMAKE_CURRENT_LIST_DIR}/moonshine-tts ${CMAKE_BINARY_DIR}/moonshine-tts)
set(MOONSHINE_SRC_FILES
  moonshine-streaming-model.cpp
  speaker-diarizer.cpp
)
set(CPP_ANNOTE_SRC_FILES cpp-annote/src/cpp-annote.cpp)
list(APPEND MOONSHINE_SRC_FILES ${CPP_ANNOTE_SRC_FILES})
target_link_libraries(moonshine PUBLIC bin-tokenizer ort-utils)
if(MOONSHINE_TTS_BUILD_ONNX)
  target_link_libraries(moonshine PRIVATE moonshine_tts_ort)
endif()
add_executable(moonshine-test tests/moonshine-test.cpp)
add_executable(benchmark benchmark.cpp)
add_executable(moonshine-download-smoke download-smoke.cpp)
''',
        "core/moonshine-tts/CMakeLists.txt": b'''\
add_library(moonshine_tts_utf8proc STATIC utf8proc.c)
add_library(moonshine_tts_core STATIC tts-core.cpp)
function(moonshine_tts_add_test name)
  add_executable(${name} tests/${name}.cpp)
  add_test(NAME ${name} COMMAND ${name})
endfunction()
moonshine_tts_add_test(tts_core_test)
add_executable(german_rule_g2p g2p.cpp)
option(MOONSHINE_TTS_BUILD_ONNX "Build TTS ONNX Runtime targets" ON)
if(MOONSHINE_TTS_BUILD_ONNX)
  add_library(moonshine_tts_ort STATIC tts-ort.cpp)
  target_link_libraries(moonshine_tts_ort PUBLIC ort-utils)
endif()
''',
        "core/ort-utils/CMakeLists.txt": b'''\
add_library(ort-utils STATIC ort-utils.cpp)
target_link_libraries(ort-utils PUBLIC ${ONNXRUNTIME_LIB_PATH})
add_executable(ort-utils-test test.cpp)
''',
        "core/bin-tokenizer/CMakeLists.txt": b'''\
add_library(bin-tokenizer STATIC bin-tokenizer.cpp)
add_executable(bin-tokenizer-test test.cpp)
''',
        "core/third-party/onnxruntime/CMakeLists.txt": b'''\
add_library(onnxruntime SHARED IMPORTED)
add_executable(onnxruntime-test test.cpp)
''',
        "core/third-party/onnxruntime/find-ort-library-path.cmake": b'''\
set(ONNXRUNTIME_LIB_PATH "${CMAKE_CURRENT_LIST_DIR}/lib/libonnxruntime.so" CACHE INTERNAL "")
''',
    }


def fixture_hashes(contents: dict[str, bytes]) -> dict[str, str]:
    return {path: probe.git_blob_sha(data) for path, data in contents.items()}


class MoonshineCmakeProbeTests(unittest.TestCase):
    def verify_fixture(self, contents: dict[str, bytes] | None = None):
        source_bytes = contents if contents is not None else synthetic_sources()
        return probe.verify_sources(source_bytes, fixture_hashes(source_bytes))

    def test_git_blob_hash_includes_git_object_header(self) -> None:
        self.assertEqual(
            probe.git_blob_sha(b"hello\n"),
            "ce013625030ba8dba906f756967f9e9ca394464a",
        )

    def test_modified_file_is_detected_by_git_blob_mismatch(self) -> None:
        sources = synthetic_sources()
        expected = fixture_hashes(sources)
        sources["core/CMakeLists.txt"] += b"# edited\n"

        _, errors = probe.verify_sources(sources, expected)

        self.assertTrue(any("Git blob SHA mismatch for core/CMakeLists.txt" in error for error in errors))

    def test_missing_source_file_is_rejected_by_local_reader(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            for path, data in synthetic_sources().items():
                destination = root / path
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)
            (root / "core/bin-tokenizer/CMakeLists.txt").unlink()

            with self.assertRaisesRegex(probe.ProbeError, "Missing required pinned source file: core/bin-tokenizer/CMakeLists.txt"):
                probe.read_local_sources(root)

    def test_all_six_expected_source_files_are_required(self) -> None:
        sources = synthetic_sources()
        del sources["core/third-party/onnxruntime/find-ort-library-path.cmake"]

        _, errors = probe.verify_sources(sources, fixture_hashes(synthetic_sources()))

        self.assertIn(
            "Missing required pinned source file: core/third-party/onnxruntime/find-ort-library-path.cmake",
            errors,
        )

    def test_tts_onnx_option_default_is_detected(self) -> None:
        report, errors = self.verify_fixture()

        self.assertEqual(errors, [])
        self.assertIsNotNone(report)
        self.assertEqual(report.tts_onnx_default, "ON")
        self.assertEqual(report.reliability_default, "OFF")
        self.assertEqual(report.sanitizer_default, "address,undefined")
        self.assertEqual(
            report.build_shared_default,
            "UNKNOWN (not assigned in inspected files)",
        )
        self.assertEqual(
            report.build_swift_default,
            "UNKNOWN (not assigned in inspected files)",
        )

    def test_changed_tts_option_default_fails_contract(self) -> None:
        sources = synthetic_sources()
        sources["core/moonshine-tts/CMakeLists.txt"] = sources[
            "core/moonshine-tts/CMakeLists.txt"
        ].replace(b"targets\" ON)", b"targets\" OFF)")

        _, errors = self.verify_fixture(sources)

        self.assertIn("MOONSHINE_TTS_BUILD_ONNX default changed: expected ON, found OFF", errors)

    def test_unconditional_tts_subdirectory_is_detected(self) -> None:
        report, errors = self.verify_fixture()

        self.assertEqual(errors, [])
        self.assertEqual(report.tts_subtree, "included")

    def test_conditionally_gated_tts_subdirectory_is_rejected(self) -> None:
        sources = synthetic_sources()
        root = sources["core/CMakeLists.txt"].decode()
        root = root.replace(
            "add_subdirectory(${CMAKE_CURRENT_LIST_DIR}/moonshine-tts ${CMAKE_BINARY_DIR}/moonshine-tts)",
            "if(MOONSHINE_ENABLE_TTS)\n"
            "  add_subdirectory(${CMAKE_CURRENT_LIST_DIR}/moonshine-tts ${CMAKE_BINARY_DIR}/moonshine-tts)\n"
            "endif()",
        )
        sources["core/CMakeLists.txt"] = root.encode()

        _, errors = self.verify_fixture(sources)

        self.assertIn("Root moonshine-tts add_subdirectory() must be unconditional", errors)

    def test_diarization_source_and_append_are_detected(self) -> None:
        report, errors = self.verify_fixture()

        self.assertEqual(errors, [])
        self.assertEqual(report.diarization_in_main_target, "included")

    def test_tokenizer_and_onnx_runtime_references_are_detected(self) -> None:
        report, errors = self.verify_fixture()

        self.assertEqual(errors, [])
        self.assertEqual(report.main_target_dependencies, ("bin-tokenizer", "ort-utils"))
        self.assertTrue(report.ort_library_path_linked)

    def test_commented_out_cmake_statements_are_not_active(self) -> None:
        sources = synthetic_sources()
        root = sources["core/CMakeLists.txt"].decode()
        root = root.replace(
            "add_subdirectory(${CMAKE_CURRENT_LIST_DIR}/moonshine-tts ${CMAKE_BINARY_DIR}/moonshine-tts)",
            "# add_subdirectory(${CMAKE_CURRENT_LIST_DIR}/moonshine-tts ${CMAKE_BINARY_DIR}/moonshine-tts)",
        )
        sources["core/CMakeLists.txt"] = root.encode()

        _, errors = self.verify_fixture(sources)

        self.assertIn("Root CMake must add the moonshine-tts subdirectory", errors)

    def test_missing_contract_has_clear_diagnostic(self) -> None:
        sources = synthetic_sources()
        sources["core/moonshine-tts/CMakeLists.txt"] = sources[
            "core/moonshine-tts/CMakeLists.txt"
        ].replace(
            b'option(MOONSHINE_TTS_BUILD_ONNX "Build TTS ONNX Runtime targets" ON)',
            b'# option(MOONSHINE_TTS_BUILD_ONNX "Build TTS ONNX Runtime targets" ON)',
        )

        _, errors = self.verify_fixture(sources)

        self.assertIn(
            "MOONSHINE_TTS_BUILD_ONNX must have exactly one active option() declaration; found 0",
            errors,
        )

    def test_report_does_not_claim_compilation_or_binary_verification(self) -> None:
        report, errors = self.verify_fixture()
        self.assertEqual(errors, [])

        rendered = probe.render_report(report)

        self.assertIn("Minimal STT exclusions: NOT PROVEN", rendered)
        self.assertIn("CMake configure: NOT RUN", rendered)
        self.assertIn("Native compilation: NOT RUN", rendered)
        self.assertIn("Binary contents: NOT VERIFIED", rendered)
        self.assertIn("Production packaging: NOT VERIFIED", rendered)

    def test_offline_local_mode_never_calls_urlopen(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            sources = synthetic_sources()
            for path, data in sources.items():
                destination = root / path
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(data)

            with (
                mock.patch.object(probe, "build_opener", side_effect=AssertionError("network used")),
                mock.patch.object(urllib.request, "urlopen", side_effect=AssertionError("network used")),
            ):
                local = probe.read_local_sources(root)
                report, errors = probe.verify_sources(local, fixture_hashes(sources))

        self.assertEqual(errors, [])
        self.assertIsNotNone(report)

    def test_repeated_verification_is_deterministic(self) -> None:
        sources = synthetic_sources()
        expected = fixture_hashes(sources)

        first, first_errors = probe.verify_sources(sources, expected)
        second, second_errors = probe.verify_sources(sources, expected)

        self.assertEqual(first_errors, second_errors)
        self.assertEqual(probe.render_report(first), probe.render_report(second))


if __name__ == "__main__":
    unittest.main()
