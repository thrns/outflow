"""Synthetic regression tests for the read-only Moonshine source audit.

The tiny C++ strings below test parser behavior only. They are not an upstream
manifest or model artifact and must never be treated as one.
"""

from __future__ import annotations

import hashlib
from pathlib import Path
import socket
import tempfile
import unittest
from unittest.mock import patch

from scripts import moonshine_model_source_audit as audit


REQUIRED_FILES = (
    "adapter.ort",
    "cross_kv.ort",
    "decoder_kv.ort",
    "encoder.ort",
    "frontend.model.ort",
    "frontend.weights.ort",
    "streaming_config.json",
    "tokenizer.bin",
)


def _metadata_source(names: tuple[str, ...] = REQUIRED_FILES, incomplete: bool = False) -> str:
    lines = ["// synthetic generated registry fixture"]
    for index, name in enumerate(names):
        url = (
            "https://download.moonshine.ai/model/small-streaming-en/"
            f"quantized_26_08_21/{name}"
        )
        checksum = "" if incomplete and index == 0 else f"synthetic-crc-{index}"
        lines.append(f'{{"{url}", {100 + index}, "{checksum}", "crc32c"}},')
    return "\n".join(lines)


def _write_fixture(source: Path) -> None:
    (source / "core/third-party/kissfft").mkdir(parents=True)
    (source / "core/third-party/onnxruntime").mkdir(parents=True)
    (source / "docs/models").mkdir(parents=True)
    (source / "core/moonshine-c-api.h").write_text(
        """#define MOONSHINE_HEADER_VERSION (30000)
#define MOONSHINE_MODEL_ARCH_MEDIUM_STREAMING (5)
#define MOONSHINE_MODEL_ARCH_SMALL_STREAMING (4)
#define MOONSHINE_MODEL_ARCH_BASE (1)
#define MOONSHINE_MODEL_ARCH_BASE_STREAMING (3)
MOONSHINE_EXPORT int32_t moonshine_load_transcriber_from_files(
    const char *path, uint32_t model_arch, const moonshine_option_t *options);
{"base_url":"", "name":"", "url":"", "size":1,"checksum":"","checksum_type":"crc32c"}
MOONSHINE_EXPORT void moonshine_free_buffer(void *ptr);
""",
        encoding="utf-8",
    )
    (source / "core/moonshine-c-api.cpp").write_text(
        """int32_t moonshine_load_transcriber_from_files(
    const char *path, uint32_t model_arch, const moonshine_option_t *options) {
  transcriber_options.model_arch = model_arch;
}
int32_t moonshine_get_stt_dependencies(...) {
  return stt_model_dependencies(language, model_arch, include_spelling, include_word_timestamps);
}
std::string json_model_file(...) {
  o += "\\\"name\\\":";
  o += "\\\"url\\\":";
  o += "\\\"size\\\":";
  o += "\\\"checksum\\\":";
  o += "\\\"checksum_type\\\":";
  return o;
}
std::string json_model_dependencies(...) {
  o += "{\\\"groups\\\":[";
  o += "\\\"base_url\\\":";
  return o;
}
void moonshine_free_buffer(void *ptr) { std::free(ptr); }
""",
        encoding="utf-8",
    )
    (source / "core/moonshine-model-catalog.h").write_text(
        "std::optional<ModelDependencies> stt_model_dependencies();\n", encoding="utf-8"
    )
    (source / "core/moonshine-model-catalog.cpp").write_text(
        """constexpr const char* kStreamingQuantizedDir = "/quantized_26_08_21";
bool is_streaming_arch(int32_t model_arch) {
  return model_arch == MOONSHINE_MODEL_ARCH_SMALL_STREAMING;
}
const std::vector<SttLanguageEntry>& stt_catalog() {
  return {
    {"en", "English", {
      {MOONSHINE_MODEL_ARCH_MEDIUM_STREAMING, "medium"},
      {MOONSHINE_MODEL_ARCH_SMALL_STREAMING, "/small-streaming-en" + kStreamingQuantizedDir},
      {MOONSHINE_MODEL_ARCH_BASE, "base"},
    }},
  };
}
std::vector<std::string> stt_component_files(const std::string& language_code,
                                             int32_t model_arch,
                                             bool include_word_timestamps) {
  const bool is_english = (language_code == "en");
  if (is_streaming_arch(model_arch)) {
    std::vector<std::string> files = {
      "adapter.ort", "cross_kv.ort", "decoder_kv.ort", "encoder.ort",
      "frontend.model.ort", "frontend.weights.ort", "streaming_config.json", "tokenizer.bin",
    };
    if (is_english && include_word_timestamps) {
      files.push_back("decoder_kv_with_attention.ort");
    }
    return files;
  }
  return {};
}
const SpellingModelEntry spelling = {
  "/spelling-en", {"spelling_cnn.ort", "spelling_cnn_meta.json"}
};
std::optional<ModelDependencies> stt_model_dependencies(
    const std::string& language, std::optional<int32_t> model_arch,
    bool include_spelling, bool include_word_timestamps) {
  if (model_arch.has_value()) { return std::nullopt; }
  else { model = &lang->models.front(); }
  return find_model_file_metadata("url");
}
""",
        encoding="utf-8",
    )
    (source / "core/moonshine-model-file-metadata.h").write_text(
        "ModelFileMetadata find_model_file_metadata(const std::string& url);\n",
        encoding="utf-8",
    )
    (source / "core/moonshine-model-file-metadata.generated.cpp").write_text(
        _metadata_source()
        + "\nModelFileMetadata find_model_file_metadata(const std::string& url) { return {}; }\n",
        encoding="utf-8",
    )
    (source / "docs/license.md").write_text(
        "Moonshine models are MIT. This includes all streaming speech-to-text models "
        "and all English-language models.\n",
        encoding="utf-8",
    )
    (source / "docs/models/available-models.md").write_text(
        "| English | Small Streaming | 123 million | 7.84% | MIT |\n",
        encoding="utf-8",
    )
    (source / "LICENSE").write_text("MIT\n", encoding="utf-8")
    (source / "core/third-party/kissfft/COPYING").write_text(
        "See LICENSES/BSD-3-Clause\n", encoding="utf-8"
    )
    (source / "core/third-party/onnxruntime/LICENSE.txt").write_text(
        "MIT\n", encoding="utf-8"
    )


class MoonshineModelSourceAuditTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.source = Path(self.temporary.name) / "moonshine"
        self.source.mkdir()
        _write_fixture(self.source)

    def _git_query(self, source: Path, *arguments: str) -> tuple[int, str]:
        if arguments == ("rev-parse", "HEAD"):
            return 0, audit.PINNED_REVISION
        if arguments == ("status", "--short", "--untracked-files=all"):
            return 0, ""
        if arguments == ("remote", "get-url", "origin"):
            return 0, audit.PINNED_ORIGIN
        self.fail(f"unexpected Git command: {arguments!r}")

    def _audit(self, revision: str | None = None) -> dict[str, object]:
        def query(source: Path, *arguments: str) -> tuple[int, str]:
            if arguments == ("rev-parse", "HEAD") and revision is not None:
                return 0, revision
            return self._git_query(source, *arguments)

        with patch.object(audit, "_run_git", side_effect=query):
            return audit.audit_source(self.source)

    def test_missing_source_directory_is_blocked(self) -> None:
        report = audit.audit_source(self.source / "missing")
        self.assertEqual(report["source_revision"], "Mismatch")
        self.assertIn("SOURCE_DIRECTORY_MISSING", report["reason_codes"])
        self.assertEqual(report["exit_code"], 2)

    def test_incorrect_source_revision_stops_source_specific_checks(self) -> None:
        report = self._audit("0" * 40)
        self.assertEqual(report["source_revision"], "Mismatch")
        self.assertEqual(report["architecture"], "Unresolved")
        self.assertIn("SOURCE_REVISION_OR_IDENTITY_MISMATCH", report["reason_codes"])

    def test_missing_c_api_header_is_reported(self) -> None:
        (self.source / audit.HEADER).unlink()
        report = self._audit()
        self.assertEqual(report["header"], "Missing")
        self.assertIn("C_API_HEADER_MISSING", report["reason_codes"])

    def test_missing_small_streaming_architecture_is_unresolved(self) -> None:
        path = self.source / audit.HEADER
        path.write_text(
            path.read_text(encoding="utf-8").replace(
                "#define MOONSHINE_MODEL_ARCH_SMALL_STREAMING (4)\n", ""
            ),
            encoding="utf-8",
        )
        report = self._audit()
        self.assertEqual(report["architecture"], "Unresolved")

    def test_recognized_architecture_and_explicit_loader_value(self) -> None:
        report = self._audit()
        self.assertEqual(report["architecture_name"], "MOONSHINE_MODEL_ARCH_SMALL_STREAMING")
        self.assertEqual(report["architecture_value"], 4)
        self.assertEqual(report["loader"], "Verified")
        self.assertEqual(report["english_default"], "MEDIUM_STREAMING (5)")

    def test_missing_dependency_manifest_implementation_is_unresolved(self) -> None:
        path = self.source / audit.C_API
        path.write_text(
            path.read_text(encoding="utf-8").replace("stt_model_dependencies", "missing_impl"),
            encoding="utf-8",
        )
        report = self._audit()
        self.assertEqual(report["dependencies"], "Unresolved")

    def test_local_catalog_and_generated_registry_are_traced(self) -> None:
        report = self._audit()
        self.assertEqual(report["dependencies"], "Traced")
        self.assertEqual(report["artifact_list"], "Verified")
        self.assertEqual(tuple(report["required_files"]), REQUIRED_FILES)
        self.assertEqual(report["quantized_path"], "/quantized_26_08_21")

    def test_incomplete_vendor_artifact_metadata_stays_unverified(self) -> None:
        path = self.source / audit.METADATA_CPP
        path.write_text(_metadata_source(incomplete=True), encoding="utf-8")
        report = self._audit()
        self.assertIn("registry metadata incomplete", report["artifact_sizes"])
        self.assertIn("SOURCE_ARTIFACT_METADATA_INCOMPLETE", report["reason_codes"])

    def test_crc32c_is_not_reported_as_packaged_sha256(self) -> None:
        report = self._audit()
        self.assertIn("CRC32C", str(report["artifact_sha256"]).upper())
        self.assertEqual(report["artifact_sha256"].split()[0], "Not")
        self.assertFalse(report["model_artifacts_verified"])

    def test_missing_small_model_specific_license_evidence_is_incomplete(self) -> None:
        (self.source / audit.MODEL_DOC).unlink()
        report = self._audit()
        self.assertEqual(report["model_license"], "Incomplete")
        self.assertIn("MODEL_SPECIFIC_LICENSE_EVIDENCE_INCOMPLETE", report["reason_codes"])

    def test_report_order_is_deterministic(self) -> None:
        report = self._audit()
        first = audit.render_report(report)
        second = audit.render_report(self._audit())
        self.assertEqual(first, second)
        self.assertLess(first.index("Pinned source revision |"), first.index("C API header |"))
        self.assertLess(first.index("C API header |"), first.index("Small Streaming architecture |"))

    def test_audit_uses_only_local_reads_and_does_not_modify_checkout(self) -> None:
        before_directories = {
            path.relative_to(self.source).as_posix()
            for path in self.source.rglob("*")
            if path.is_dir()
        }
        before = {
            path.relative_to(self.source).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in self.source.rglob("*")
            if path.is_file()
        }
        git_calls: list[tuple[str, ...]] = []

        def local_git(source: Path, *arguments: str) -> tuple[int, str]:
            git_calls.append(arguments)
            return self._git_query(source, *arguments)

        with (
            patch.object(audit, "_run_git", side_effect=local_git),
            patch.object(
                socket, "create_connection", side_effect=AssertionError("network access attempted")
            ),
            patch.object(
                socket.socket, "connect", side_effect=AssertionError("network access attempted")
            ),
        ):
            report = audit.audit_source(self.source)

        after_directories = {
            path.relative_to(self.source).as_posix()
            for path in self.source.rglob("*")
            if path.is_dir()
        }
        after = {
            path.relative_to(self.source).as_posix(): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in self.source.rglob("*")
            if path.is_file()
        }
        self.assertEqual(before_directories, after_directories)
        self.assertEqual(before, after)
        self.assertEqual(report["source_revision"], "Verified")
        self.assertEqual(
            git_calls,
            [
                ("rev-parse", "HEAD"),
                ("status", "--short", "--untracked-files=all"),
                ("remote", "get-url", "origin"),
            ],
        )


if __name__ == "__main__":
    unittest.main()
