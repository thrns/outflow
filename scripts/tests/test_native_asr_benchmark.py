from __future__ import annotations

import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
import wave
from types import SimpleNamespace
from unittest.mock import patch

from scripts.run_native_asr_benchmark import (
    APPROVED_4GB_REFERENCE_CLASSES,
    DEFAULT_CANDIDATE,
    assess_physical_4gb_eligibility,
    BenchmarkError,
    EXPECTED_ARCHITECTURE,
    EXPECTED_HEADER_VERSION,
    MODEL_CATALOG_PATH,
    MODEL_CANDIDATES,
    REQUIRED_MODEL_ASSETS,
    SOURCE_REVISION,
    _crc32c_base64,
    derive_source_model_contract,
    prepare_candidate_asset_manifest,
    _run_json_command,
    _write_result,
    build_measured_result,
    load_verified_fixtures,
    parse_load_probe,
    parse_native_output,
    percentile,
    validate_wav,
    verify_source,
    verify_model_assets,
)
from scripts.score_wer import NORMALIZATION_POLICY_VERSION, calculate_wer
from scripts.validate_fixture_manifest import REPOSITORY_ROOT, validate_manifest
from scripts.validate_measured_asr_results import validate_measured_results


def native_output(hypothesis: str, *, index: int = 0) -> dict[str, object]:
    return {
        "index": index,
        "hypothesis": hypothesis,
        "partial_calls": 1,
        "partial_nonempty": True,
        "final_lines": 1 if hypothesis else 0,
        "recognition_ms": 40.0 + index,
        "stop_to_final_ms": 4.0 + index,
        "real_time_factor": 0.04,
        "cpu_ms": 55.0,
    }


def fixture_record(fixture_id: str, category: str, reference: str, *, index: int = 0) -> dict[str, object]:
    return {
        "id": fixture_id,
        "status": "ready",
        "category": category,
        "duration_class": "silence_control" if category == "silence" else "short",
        "coverage_attributes": [],
        "speaker_variety": None,
        "provenance_verified": True,
        "human_reference_verified": True,
        "reference": reference,
        "audio_sha256": hashlib.sha256(f"audio-{fixture_id}".encode()).hexdigest(),
        "reference_sha256": hashlib.sha256(f"reference-{fixture_id}".encode()).hexdigest(),
        "audio_metadata": {
            "sample_format": "PCM_S16LE",
            "sample_rate_hz": 16_000,
            "channels": 1,
            "frame_count": 16_000,
            "duration_seconds": 1.0,
        },
        "audio_path": Path(f"/tmp/{fixture_id}.wav"),
        "index": index,
    }


def output_fixture(hypothesis: str, *, index: int = 0) -> dict[str, object]:
    return native_output(hypothesis, index=index)


def write_candidate_source_fixture(
    source_root: Path, *, candidate: str = "tiny", architecture: int = 2,
    asset_root: Path | None = None,
) -> None:
    core = source_root / "core"
    core.mkdir(parents=True, exist_ok=True)
    (core / "moonshine-c-api.h").write_text(
        "#define MOONSHINE_HEADER_VERSION (30000)\n"
        "#define MOONSHINE_MODEL_ARCH_SMALL_STREAMING (4)\n"
        f"#define MOONSHINE_MODEL_ARCH_TINY_STREAMING ({architecture})\n",
        encoding="utf-8",
    )
    catalog_lines = [
        'constexpr const char* kStreamingQuantizedDir = "/quantized_26_08_21";',
        '{MOONSHINE_MODEL_ARCH_SMALL_STREAMING, std::string(kCdnModelBase) + "/small-streaming-en" + kStreamingQuantizedDir},',
        '{MOONSHINE_MODEL_ARCH_TINY_STREAMING, std::string(kCdnModelBase) + "/tiny-streaming-en" + kStreamingQuantizedDir},',
        'std::vector<std::string> stt_component_files(const std::string& language_code, int32_t model_arch, bool include_word_timestamps) {',
        '  std::vector<std::string> files = {',
    ]
    catalog_lines.extend(f'    "{name}",' for name in sorted(REQUIRED_MODEL_ASSETS))
    catalog_lines.extend(['  };', '}'])
    (core / "moonshine-model-catalog.cpp").write_text("\n".join(catalog_lines) + "\n", encoding="utf-8")

    identity = MODEL_CANDIDATES[candidate]["identity"]
    metadata_lines = []
    for name in sorted(REQUIRED_MODEL_ASSETS):
        contents = b"x"
        if asset_root is not None:
            asset_root.mkdir(parents=True, exist_ok=True)
            contents = f"verified fixture bytes for {name}\n".encode()
            (asset_root / name).write_bytes(contents)
        checksum_path = core / f"checksum-{name}"
        checksum_path.write_bytes(contents)
        checksum = _crc32c_base64(checksum_path)
        checksum_path.unlink()
        url = f"https://download.moonshine.ai/model/{identity}/quantized_26_08_21/{name}"
        metadata_lines.append(f'{{"{url}", {len(contents)}, "{checksum}", "crc32c"}},')
    metadata = "constexpr std::array<Entry, " + str(len(metadata_lines)) + "> kEntries = {{\n"
    metadata += "\n".join(metadata_lines) + "\n}};\n"
    (core / "moonshine-model-file-metadata.generated.cpp").write_text(metadata, encoding="utf-8")


class NativeAsrBenchmarkTests(unittest.TestCase):
    def test_verify_source_accepts_pinned_parenthesized_header_macros(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source_root = Path(temporary)
            write_candidate_source_fixture(source_root, candidate="small", architecture=4)
            git_outputs = [
                SimpleNamespace(stdout=SOURCE_REVISION + "\n"),
                SimpleNamespace(stdout=""),
            ]
            with patch(
                "scripts.run_native_asr_benchmark.subprocess.run",
                side_effect=git_outputs,
            ) as run:
                identity = verify_source(source_root)

        self.assertEqual(run.call_count, 2)
        self.assertEqual(identity["source_revision"], SOURCE_REVISION)
        self.assertEqual(identity["c_api_header_version"], EXPECTED_HEADER_VERSION)
        self.assertEqual(identity["candidate"], "small")
        self.assertEqual(identity["model_architecture"], EXPECTED_ARCHITECTURE)
        self.assertRegex(identity["source_contract_sha256"], r"^[0-9a-f]{64}$")

    def test_development_candidate_defaults_to_small_and_source_defines_tiny_contract(self) -> None:
        self.assertEqual(DEFAULT_CANDIDATE, "small")
        self.assertEqual(MODEL_CANDIDATES["tiny"]["architecture"], 2)
        with tempfile.TemporaryDirectory() as temporary:
            source_root = Path(temporary)
            write_candidate_source_fixture(source_root)
            contract = derive_source_model_contract(source_root, "tiny")

        self.assertEqual(contract["architecture"], 2)
        self.assertEqual(contract["model_identity"], "tiny-streaming-en")
        self.assertEqual(len(contract["required_files"]), 8)
        self.assertEqual(
            sum(item["expected_bytes"] for item in contract["required_files"].values()),
            8 * len(b"x"),
        )

    def test_source_contract_rejects_unsupported_architecture(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source_root = Path(temporary)
            write_candidate_source_fixture(source_root, architecture=9)
            with self.assertRaisesRegex(BenchmarkError, "candidate contract is unsupported"):
                derive_source_model_contract(source_root, "tiny")

    def test_tiny_manifest_preparation_blocks_missing_files_and_requires_approval(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_root = root / "source"
            asset_root = root / "tiny"
            write_candidate_source_fixture(source_root)
            asset_root.mkdir()
            with self.assertRaisesRegex(BenchmarkError, "incomplete"):
                prepare_candidate_asset_manifest(source_root, "tiny", asset_root, root / "missing.json")

            asset_root.rmdir()
            write_candidate_source_fixture(source_root, asset_root=asset_root)
            manifest_path = root / "candidate.json"
            manifest = prepare_candidate_asset_manifest(source_root, "tiny", asset_root, manifest_path)
            contract = derive_source_model_contract(source_root, "tiny")
            with self.assertRaisesRegex(BenchmarkError, "approval is not recorded"):
                verify_model_assets(
                    manifest_path, asset_root, candidate="tiny", contract=contract,
                    repository_root=root,
                )

        self.assertEqual(manifest["acquisition_approval_status"], "not-recorded")
        self.assertEqual(len(manifest["assets"]), 8)

    def test_tiny_manifest_rejects_identity_mismatch_and_incorrect_local_sha256(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_root = root / "source"
            asset_root = root / "tiny"
            write_candidate_source_fixture(source_root, asset_root=asset_root)
            contract = derive_source_model_contract(source_root, "tiny")
            manifest_path = root / "candidate.json"
            prepare_candidate_asset_manifest(source_root, "tiny", asset_root, manifest_path)
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            manifest["acquisition_approval_status"] = "approved"
            manifest["acquisition_approval_reference"] = "evidence/tiny-approval.md"
            approval = root / "evidence" / "tiny-approval.md"
            approval.parent.mkdir()
            approval.write_text(
                f"TINY EVALUATION ACQUISITION APPROVED\n{SOURCE_REVISION}\ntiny-streaming-en\n",
                encoding="utf-8",
            )

            manifest["model_identity"] = "small-streaming-en"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaisesRegex(BenchmarkError, "identity does not match"):
                verify_model_assets(
                    manifest_path, asset_root, candidate="tiny", contract=contract,
                    approval_reference="evidence/tiny-approval.md", repository_root=root,
                )

            manifest["model_identity"] = "tiny-streaming-en"
            manifest["assets"][0]["sha256"] = "0" * 64
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaises(BenchmarkError):
                verify_model_assets(
                    manifest_path, asset_root, candidate="tiny", contract=contract,
                    approval_reference="evidence/tiny-approval.md", repository_root=root,
                )

    def test_market_4gb_memory_boundary_includes_binary_reporting_only(self) -> None:
        machine = {
            "physical_memory_bytes": 4_000_000_000,
            "physical_hardware_status": "verified_physical",
            "reference_hardware_class": "approved-reference",
        }
        with patch(
            "scripts.run_native_asr_benchmark.APPROVED_4GB_REFERENCE_CLASSES",
            frozenset({"approved-reference"}),
        ):
            self.assertEqual(assess_physical_4gb_eligibility(machine), "ELIGIBLE")
            machine["physical_memory_bytes"] = 4_294_967_296
            self.assertEqual(assess_physical_4gb_eligibility(machine), "ELIGIBLE")
            machine["physical_memory_bytes"] = 4_294_967_297
            self.assertEqual(assess_physical_4gb_eligibility(machine), "INELIGIBLE")

    def test_wer_normalization_version_is_frozen_and_alignment_is_stable(self) -> None:
        self.assertEqual(NORMALIZATION_POLICY_VERSION, "lexical-wer-unicode-boundary-v1")
        result = calculate_wer("CAFÉ, １２３!", "cafe\u0301 １２３")
        self.assertEqual(result.substitutions, 0)
        self.assertEqual(result.deletions, 0)
        self.assertEqual(result.insertions, 0)
        self.assertEqual(result.wer, 0.0)

    def test_load_probe_parser_accepts_first_and_warm_load_samples(self) -> None:
        self.assertEqual(
            parse_load_probe({"schema_version": 1, "execution_provider": "CPUExecutionProvider", "architecture": EXPECTED_ARCHITECTURE, "header_version": EXPECTED_HEADER_VERSION, "model_load_ms": [245.8, 85.2, 76.8]}),
            [245.8, 85.2, 76.8],
        )

    def test_load_probe_parser_rejects_short_or_nonfinite_samples(self) -> None:
        for value in (
            {"schema_version": 1, "execution_provider": "CPUExecutionProvider", "architecture": EXPECTED_ARCHITECTURE, "header_version": EXPECTED_HEADER_VERSION, "model_load_ms": [10]},
            {"schema_version": 1, "execution_provider": "CPUExecutionProvider", "architecture": EXPECTED_ARCHITECTURE, "header_version": EXPECTED_HEADER_VERSION, "model_load_ms": [10, float("nan")]},
        ):
            with self.subTest(value=value), self.assertRaises(BenchmarkError):
                parse_load_probe(value)

    def test_native_output_parser_validates_architecture_order_and_metrics(self) -> None:
        value = {
            "schema_version": 1,
            "execution_provider": "CPUExecutionProvider",
            "architecture": EXPECTED_ARCHITECTURE,
            "header_version": EXPECTED_HEADER_VERSION,
            "model_load_ms": 100.5,
            "outputs": [native_output("hello", index=0), native_output("world", index=1)],
        }

        load_ms, outputs = parse_native_output(value, 2)

        self.assertEqual(load_ms, 100.5)
        self.assertEqual([row["hypothesis"] for row in outputs], ["hello", "world"])

    def test_native_output_parser_rejects_missing_or_nonfinite_fields(self) -> None:
        value = {
            "schema_version": 1,
            "execution_provider": "CPUExecutionProvider",
            "architecture": EXPECTED_ARCHITECTURE,
            "header_version": EXPECTED_HEADER_VERSION,
            "model_load_ms": 100,
            "outputs": [native_output("hello")],
        }
        malformed = dict(value)
        malformed["outputs"] = [dict(native_output("hello"), recognition_ms=float("inf"))]
        wrong_count = dict(value)
        wrong_count["outputs"] = []

        for candidate in (malformed, wrong_count):
            with self.subTest(candidate=candidate), self.assertRaises(BenchmarkError):
                parse_native_output(candidate, 1)
        with self.assertRaisesRegex(BenchmarkError, "unsupported development model architecture"):
            parse_native_output(value, 1, expected_architecture=9)

    def test_native_failure_does_not_echo_stdout_or_stderr_transcripts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            executable = Path(temporary) / "fails.py"
            executable.write_text(
                "#!/usr/bin/env python3\nimport sys\nprint('PRIVATE HYPOTHESIS')\nprint('PRIVATE ERROR', file=sys.stderr)\nsys.exit(7)\n",
                encoding="utf-8",
            )
            executable.chmod(0o755)

            with self.assertRaises(BenchmarkError) as raised:
                _run_json_command([str(executable)], timeout_seconds=5)

        message = str(raised.exception)
        self.assertIn("exit 7", message)
        self.assertNotIn("PRIVATE HYPOTHESIS", message)
        self.assertNotIn("PRIVATE ERROR", message)

    def test_native_model_load_failure_propagates_without_captured_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            executable = Path(temporary) / "model-load-fails.py"
            executable.write_text(
                "#!/usr/bin/env python3\nimport sys\nprint('Moonshine model load failed', file=sys.stderr)\nsys.exit(1)\n",
                encoding="utf-8",
            )
            executable.chmod(0o755)

            with self.assertRaises(BenchmarkError) as raised:
                _run_json_command([str(executable)], timeout_seconds=5)

        self.assertIn("exit 1", str(raised.exception))
        self.assertNotIn("Moonshine model load failed", str(raised.exception))

    def test_percentiles_use_deterministic_linear_interpolation(self) -> None:
        self.assertEqual(percentile([1, 2, 3, 4], 0.5), 2.5)
        self.assertEqual(percentile([1, 2, 3, 4], 0.95), 3.8499999999999996)
        self.assertIsNone(percentile([], 0.95))

    def test_wav_reader_and_manifest_metadata_match(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "one-second.wav"
            with wave.open(str(path), "wb") as audio:
                audio.setnchannels(1)
                audio.setsampwidth(2)
                audio.setframerate(16_000)
                audio.writeframes(b"\0\0" * 16_000)
            expected = {
                "sample_format": "PCM_S16LE",
                "sample_rate_hz": 16_000,
                "channels": 1,
                "frame_count": 16_000,
                "duration_seconds": 1.0,
            }

            actual = validate_wav(path, expected)

        self.assertEqual(actual, expected)

    def test_wav_metadata_mismatch_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "one-second.wav"
            with wave.open(str(path), "wb") as audio:
                audio.setnchannels(1)
                audio.setsampwidth(2)
                audio.setframerate(16_000)
                audio.writeframes(b"\0\0" * 16_000)
            expected = {
                "sample_format": "PCM_S16LE",
                "sample_rate_hz": 8_000,
                "channels": 1,
                "frame_count": 8_000,
                "duration_seconds": 1.0,
            }
            with self.assertRaisesRegex(BenchmarkError, "format does not match"):
                validate_wav(path, expected)

    def test_actual_manifest_is_blocked_when_it_has_no_ready_fixtures(self) -> None:
        manifest_path = REPOSITORY_ROOT / "docs" / "benchmark-fixtures.json"

        with self.assertRaisesRegex(BenchmarkError, "no verified ready fixtures"):
            load_verified_fixtures(manifest_path, REPOSITORY_ROOT)

    def test_measured_result_aggregates_wer_and_excludes_transcripts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            binary_paths: dict[str, Path] = {}
            for name in ("native_runner", "moonshine_library", "onnxruntime_library"):
                path = Path(temporary) / name
                path.write_bytes(name.encode("ascii"))
                binary_paths[name] = path

            fixtures = [
                fixture_record("short-001", "short_speech", "hello world", index=0),
                fixture_record("silence-001", "silence", "", index=1),
            ]
            outputs = [output_fixture("hello there", index=0), output_fixture("", index=1)]
            manifest = {
                "normalization_policy_version": NORMALIZATION_POLICY_VERSION,
                "reference_transcription_policy_version": "human-verified-verbatim-v1",
                "collection_status": "in_progress",
            }
            model = {
                "candidate": "small",
                "model_identity": "small-streaming-en",
                "catalog_path": MODEL_CATALOG_PATH,
                "immutable_artifact_revision": None,
                "architecture": EXPECTED_ARCHITECTURE,
                "source_contract_sha256": "d" * 64,
                "c_api_header_version": EXPECTED_HEADER_VERSION,
                "asset_sha256": {name: hashlib.sha256(name.encode()).hexdigest() for name in REQUIRED_MODEL_ASSETS},
                "asset_bytes": {name: 10 for name in REQUIRED_MODEL_ASSETS},
                "total_bytes": 10 * len(REQUIRED_MODEL_ASSETS),
                "redistribution_status": "undetermined",
                "license_review_status": "review-pending",
            }
            machine = {
                "machine_label": "MAC-TEST",
                "machine_identifier": "Mac-Test1,1",
                "os": "macOS",
                "os_release": "26.6.2",
                "architecture": "arm64",
                "physical_memory_bytes": 18_000_000_000,
                "physical_memory_source": "sysctl_hw.memsize",
                "physical_hardware_status": "unknown",
                "reference_hardware_class": None,
                "cpu_only_inference_exercised": True,
                "real_hardware_measurements_completed": False,
                "logical_cpu_count": 12,
                "cpu_label": "test CPU",
            }
            binary_hashes = {name: hashlib.sha256(name.encode("ascii")).hexdigest() for name in binary_paths}
            runtime_identity = {
                "architecture": "arm64",
                "c_api_version": EXPECTED_HEADER_VERSION,
                "loaded_image_count": 5,
                "unexpected_non_system_library_count": 0,
                "resolved_paths_match_inputs": True,
                "dependency_chain_verified": True,
                "native_runner_sha256": binary_hashes["native_runner"],
                "moonshine_sha256": binary_hashes["moonshine_library"],
                "onnxruntime_sha256": binary_hashes["onnxruntime_library"],
            }
            result = build_measured_result(
                manifest=manifest,
                fixtures=fixtures,
                fixture_manifest_sha256="c" * 64,
                native_output=(100.0, outputs),
                load_samples_ms=[120.0, 90.0, 80.0],
                native_files=binary_paths,
                source={
                    "source_revision": SOURCE_REVISION,
                    "c_api_header_version": EXPECTED_HEADER_VERSION,
                    "model_architecture": EXPECTED_ARCHITECTURE,
                    "candidate": "small",
                    "source_contract_sha256": "d" * 64,
                },
                model=model,
                machine=machine,
                native_runtime_identity=runtime_identity,
                created_at_utc="2026-11-03T20:00:00Z",
            )
            repeated = build_measured_result(
                manifest=manifest,
                fixtures=fixtures,
                fixture_manifest_sha256="c" * 64,
                native_output=(100.0, outputs),
                load_samples_ms=[120.0, 90.0, 80.0],
                native_files=binary_paths,
                source={
                    "source_revision": SOURCE_REVISION,
                    "c_api_header_version": EXPECTED_HEADER_VERSION,
                    "model_architecture": EXPECTED_ARCHITECTURE,
                    "candidate": "small",
                    "source_contract_sha256": "d" * 64,
                },
                model=model,
                machine=machine,
                native_runtime_identity=runtime_identity,
                created_at_utc="2026-11-03T20:00:00Z",
            )
            serialized = json.dumps(result, sort_keys=True)
            summary = validate_measured_results(result)

        self.assertEqual(result, repeated)
        self.assertEqual(result["aggregate"]["wer"], 0.5)
        self.assertEqual(result["fixtures"][1]["wer"], None)
        self.assertTrue(result["fixtures"][1]["no_speech_detected"])
        self.assertEqual(result["release_gates"]["verified_corpus_coverage"], "NOT_ASSESSED")
        self.assertEqual(result["release_gates"]["physical_4gb_eligibility"], "INELIGIBLE")
        self.assertEqual(result["release_gates"]["physical_4gb_gate"], "BLOCKED")
        self.assertEqual(summary["fixtures"], 2)
        self.assertNotIn("hello world", serialized)
        self.assertNotIn("hello there", serialized)

    def test_measured_result_is_validated_before_write(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "result.json"
            with self.assertRaisesRegex(ValueError, "root fields"):
                _write_result({"schema_version": 2}, output)
            self.assertFalse(output.exists())

    def test_measured_result_rejects_transcript_keys_and_synthetic_schema(self) -> None:
        with self.assertRaisesRegex(ValueError, "root fields"):
            validate_measured_results({"schema_version": 1, "result_type": "synthetic"})

        # The strict root schema also rejects accidental transcript-bearing extensions.
        with self.assertRaisesRegex(ValueError, "root fields"):
            validate_measured_results({"schema_version": 1, "result_type": "measured_native_asr", "hypothesis": "private"})


if __name__ == "__main__":
    unittest.main()
