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

from scripts.run_native_asr_benchmark import (
    BenchmarkError,
    EXPECTED_ARCHITECTURE,
    EXPECTED_HEADER_VERSION,
    MODEL_CATALOG_PATH,
    REQUIRED_MODEL_ASSETS,
    SOURCE_REVISION,
    _run_json_command,
    build_measured_result,
    load_verified_fixtures,
    parse_load_probe,
    parse_native_output,
    percentile,
    validate_wav,
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
        "real_time_factor": 0.4,
        "cpu_ms": 55.0,
    }


def fixture_record(fixture_id: str, category: str, reference: str, *, index: int = 0) -> dict[str, object]:
    return {
        "id": fixture_id,
        "category": category,
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


class NativeAsrBenchmarkTests(unittest.TestCase):
    def test_wer_normalization_version_is_frozen_and_alignment_is_stable(self) -> None:
        self.assertEqual(NORMALIZATION_POLICY_VERSION, "lexical-wer-unicode-boundary-v1")
        result = calculate_wer("CAFÉ, １２３!", "cafe\u0301 １２３")
        self.assertEqual(result.substitutions, 0)
        self.assertEqual(result.deletions, 0)
        self.assertEqual(result.insertions, 0)
        self.assertEqual(result.wer, 0.0)

    def test_load_probe_parser_accepts_first_and_warm_load_samples(self) -> None:
        self.assertEqual(
            parse_load_probe({"schema_version": 1, "model_load_ms": [245.8, 85.2, 76.8]}),
            [245.8, 85.2, 76.8],
        )

    def test_load_probe_parser_rejects_short_or_nonfinite_samples(self) -> None:
        for value in (
            {"schema_version": 1, "model_load_ms": [10]},
            {"schema_version": 1, "model_load_ms": [10, float("nan")]},
        ):
            with self.subTest(value=value), self.assertRaises(BenchmarkError):
                parse_load_probe(value)

    def test_native_output_parser_validates_architecture_order_and_metrics(self) -> None:
        value = {
            "schema_version": 1,
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
            }
            model = {
                "catalog_path": MODEL_CATALOG_PATH,
                "immutable_artifact_revision": None,
                "asset_sha256": {name: hashlib.sha256(name.encode()).hexdigest() for name in REQUIRED_MODEL_ASSETS},
                "asset_bytes": {name: 10 for name in REQUIRED_MODEL_ASSETS},
                "total_bytes": 10 * len(REQUIRED_MODEL_ASSETS),
                "redistribution_status": "undetermined",
                "license_review_status": "review-pending",
            }
            machine = {
                "machine_label": "MAC-TEST",
                "os": "macOS",
                "os_release": "26.6.2",
                "architecture": "arm64",
                "physical_memory_bytes": 18_000_000_000,
                "logical_cpu_count": 12,
                "cpu_label": "test CPU",
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
                },
                model=model,
                machine=machine,
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
                },
                model=model,
                machine=machine,
                created_at_utc="2026-11-03T20:00:00Z",
            )
            serialized = json.dumps(result, sort_keys=True)
            summary = validate_measured_results(result)

        self.assertEqual(result, repeated)
        self.assertEqual(result["aggregate"]["wer"], 0.5)
        self.assertEqual(result["fixtures"][1]["wer"], None)
        self.assertTrue(result["fixtures"][1]["no_speech_detected"])
        self.assertEqual(result["release_gates"]["verified_corpus_coverage"], "NOT_ASSESSED")
        self.assertEqual(summary["fixtures"], 2)
        self.assertNotIn("hello world", serialized)
        self.assertNotIn("hello there", serialized)

    def test_measured_result_rejects_transcript_keys_and_synthetic_schema(self) -> None:
        with self.assertRaisesRegex(ValueError, "root fields"):
            validate_measured_results({"schema_version": 1, "result_type": "synthetic"})

        # The strict root schema also rejects accidental transcript-bearing extensions.
        with self.assertRaisesRegex(ValueError, "root fields"):
            validate_measured_results({"schema_version": 1, "result_type": "measured_native_asr", "hypothesis": "private"})


if __name__ == "__main__":
    unittest.main()
