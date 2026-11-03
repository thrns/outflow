"""Unit tests for the development-only benchmark fixture manifest validator."""

from __future__ import annotations

import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from typing import Any

from scripts.validate_fixture_manifest import assess_corpus_coverage, validate_manifest


MANIFEST_PATH = Path(__file__).resolve().parents[2] / "docs" / "benchmark-fixtures.json"


def load_manifest() -> dict[str, Any]:
    return json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))


def ready_fixture(root: Path) -> dict[str, Any]:
    audio_path = root / "audio" / "synthetic.wav"
    reference_path = root / "references" / "synthetic.txt"
    audio_path.parent.mkdir(parents=True, exist_ok=True)
    reference_path.parent.mkdir(parents=True, exist_ok=True)
    audio_bytes = b"synthetic validator audio bytes; not a speech recording\n"
    audio_path.write_bytes(audio_bytes)
    reference_path.write_text("synthetic validator reference\n", encoding="utf-8")

    fixture = copy.deepcopy(load_manifest()["fixtures"][0])
    fixture.update(
        {
            "id": "synthetic_valid_fixture",
            "status": "ready",
            "duration_class": "short",
            "coverage_attributes": [],
            "speaker_variety": None,
            "audio_file": "audio/synthetic.wav",
            "audio_sha256": hashlib.sha256(audio_bytes).hexdigest(),
            "reference_file": "references/synthetic.txt",
            "reference_sha256": hashlib.sha256(b"synthetic validator reference\n").hexdigest(),
            "audio_metadata": {
                "sample_format": "PCM_S16LE",
                "sample_rate_hz": 16_000,
                "channels": 1,
                "frame_count": 16_000,
                "duration_seconds": 1.0,
            },
            "human_transcription_review_method": "synthetic fixture test; no human-audio claim",
            "provenance": {
                "source": "synthetic validator test data",
                "source_release": "temporary test fixture",
                "citation": "created only in a temporary test directory",
            },
            "license_or_permission_status": "verified",
            "license_or_permission_evidence": "synthetic test data; no external rights claim",
            "human_transcription_status": "verified",
        }
    )
    return fixture


class FixtureManifestValidatorTests(unittest.TestCase):
    def test_valid_manifest_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            manifest = load_manifest()
            manifest["collection_status"] = "in_progress"
            manifest["fixtures"] = [ready_fixture(root)]

            errors, counts = validate_manifest(manifest, root)

        self.assertEqual(errors, [])
        self.assertEqual(counts["planned_entries"], 0)
        self.assertEqual(counts["ready_audio_fixtures"], 1)

    def test_duplicate_fixture_ids_are_rejected(self) -> None:
        manifest = load_manifest()
        manifest["fixtures"].append(copy.deepcopy(manifest["fixtures"][0]))

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("duplicates another fixture identifier" in error for error in errors))

    def test_corpus_coverage_rejects_short_and_long_only_collection(self) -> None:
        entries = [
            {"status": "ready", "duration_class": "short", "coverage_attributes": [], "speaker_variety": None,
             "provenance_verified": True, "human_reference_verified": True}
            for _ in range(100)
        ] + [
            {"status": "ready", "duration_class": "long", "coverage_attributes": [], "speaker_variety": None,
             "provenance_verified": True, "human_reference_verified": True}
            for _ in range(50)
        ]

        coverage = assess_corpus_coverage(entries)

        self.assertEqual(coverage["short_utterance_count"], 100)
        self.assertEqual(coverage["long_utterance_count"], 50)
        self.assertFalse(coverage["complete"])
        self.assertEqual(coverage["quiet_speech_count"], 0)
        self.assertEqual(coverage["noisy_speech_count"], 0)
        self.assertEqual(coverage["accent_variety_count"], 0)

    def test_complete_coverage_requires_every_dimension_and_verified_input(self) -> None:
        entries = [
            {"status": "ready", "duration_class": "short", "coverage_attributes": [], "speaker_variety": None,
             "provenance_verified": True, "human_reference_verified": True}
            for _ in range(100)
        ] + [
            {"status": "ready", "duration_class": "long", "coverage_attributes": [], "speaker_variety": None,
             "provenance_verified": True, "human_reference_verified": True}
            for _ in range(50)
        ]
        entries[0]["coverage_attributes"] = ["quiet_environment", "proper_nouns", "technical_vocabulary", "numbers", "abbreviations"]
        entries[1]["coverage_attributes"] = ["noisy_environment", "accented_english"]
        entries[1]["speaker_variety"] = "source variety A"
        entries[2]["coverage_attributes"] = ["accented_english"]
        entries[2]["speaker_variety"] = "source variety B"
        entries.append({
            "status": "ready", "duration_class": "silence_control", "coverage_attributes": [], "speaker_variety": None,
            "provenance_verified": True, "human_reference_verified": True,
        })

        coverage = assess_corpus_coverage(entries)

        self.assertEqual(coverage["short_utterance_count"], 100)
        self.assertEqual(coverage["long_utterance_count"], 50)
        self.assertEqual(coverage["accent_variety_count"], 2)
        self.assertTrue(coverage["complete"])
        entries[-1]["human_reference_verified"] = False
        self.assertFalse(assess_corpus_coverage(entries)["complete"])

    def test_duplicate_and_malformed_coverage_metadata_are_rejected(self) -> None:
        manifest = load_manifest()
        fixture = copy.deepcopy(manifest["fixtures"][0])
        fixture["coverage_attributes"] = ["numbers", "numbers"]
        fixture["duration_class"] = "unexpected"
        fixture["speaker_variety"] = 42
        manifest["fixtures"] = [fixture]

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("unsupported or duplicate values" in error for error in errors))
        self.assertTrue(any("duration_class is unsupported" in error for error in errors))
        self.assertTrue(any("speaker_variety must be" in error for error in errors))

    def test_unsupported_quiet_noisy_and_silence_speech_combinations_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            manifest = load_manifest()
            fixture = ready_fixture(root)
            fixture["coverage_attributes"] = ["quiet_environment", "noisy_environment"]
            manifest["collection_status"] = "in_progress"
            manifest["fixtures"] = [fixture]

            errors, _ = validate_manifest(manifest, root)

        self.assertTrue(any("cannot be classified as both quiet and noisy" in error for error in errors))

        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            manifest = load_manifest()
            fixture = ready_fixture(root)
            fixture.update({"category": "silence", "duration_class": "silence_control", "coverage_attributes": ["accented_english"], "speaker_variety": "source variety"})
            manifest["collection_status"] = "in_progress"
            manifest["fixtures"] = [fixture]

            errors, _ = validate_manifest(manifest, root)

        self.assertTrue(any("silence controls cannot carry speech coverage attributes" in error for error in errors))

    def test_unsupported_manifest_versions_are_rejected(self) -> None:
        manifest = load_manifest()
        manifest["schema_version"] = 2

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("schema_version must be 1" in error for error in errors))

    def test_invalid_fixture_statuses_are_rejected(self) -> None:
        manifest = load_manifest()
        manifest["fixtures"][0]["status"] = "collected"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("status must be one of" in error for error in errors))

    def test_missing_required_metadata_is_rejected(self) -> None:
        manifest = load_manifest()
        manifest.pop("reference_transcription_policy")

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("reference_transcription_policy must be a non-empty string" in error for error in errors))

    def test_missing_policy_versions_are_rejected(self) -> None:
        manifest = load_manifest()
        manifest.pop("normalization_policy_version")

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("normalization_policy_version must be a non-empty string" in error for error in errors))

    def test_malformed_checksums_are_rejected(self) -> None:
        manifest = load_manifest()
        manifest["fixtures"][0]["audio_sha256"] = "not-a-sha256"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("audio_sha256 must be 64 hexadecimal characters" in error for error in errors))

    def test_ready_entries_require_verified_provenance_and_permission(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            manifest = load_manifest()
            fixture = ready_fixture(root)
            fixture["provenance"] = {"source": None, "citation": None}
            fixture["license_or_permission_status"] = "not_verified"
            fixture["license_or_permission_evidence"] = None
            manifest["collection_status"] = "in_progress"
            manifest["fixtures"] = [fixture]

            errors, counts = validate_manifest(manifest, root)

        self.assertTrue(any("provenance.source is required" in error for error in errors))
        self.assertTrue(any("license_or_permission_status must be verified" in error for error in errors))
        self.assertEqual(counts["ready_audio_fixtures"], 0)

    def test_ready_entries_require_verified_human_references(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            manifest = load_manifest()
            fixture = ready_fixture(root)
            fixture["human_transcription_status"] = "not_verified"
            manifest["collection_status"] = "in_progress"
            manifest["fixtures"] = [fixture]

            errors, counts = validate_manifest(manifest, root)

        self.assertTrue(any("human_transcription_status must be verified" in error for error in errors))
        self.assertEqual(counts["ready_audio_fixtures"], 0)

    def test_ready_entries_require_actual_format_duration_and_reference_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            manifest = load_manifest()
            fixture = ready_fixture(root)
            fixture["audio_metadata"]["duration_seconds"] = 2.0
            fixture["reference_sha256"] = "0" * 64
            manifest["collection_status"] = "in_progress"
            manifest["fixtures"] = [fixture]

            errors, counts = validate_manifest(manifest, root)

        self.assertTrue(any("duration does not match" in error for error in errors))
        self.assertTrue(any("reference_sha256 does not match" in error for error in errors))
        self.assertEqual(counts["ready_audio_fixtures"], 0)

    def test_unsafe_audio_paths_are_rejected(self) -> None:
        manifest = load_manifest()
        manifest["fixtures"][0]["audio_file"] = "../outside.wav"

        errors, _ = validate_manifest(manifest)

        self.assertTrue(any("must not be absolute or contain directory traversal" in error for error in errors))

    def test_planned_entries_are_not_counted_as_benchmark_ready(self) -> None:
        errors, counts = validate_manifest(load_manifest())

        self.assertEqual(errors, [])
        self.assertEqual(counts["planned_entries"], len(load_manifest()["fixtures"]))
        self.assertEqual(counts["ready_audio_fixtures"], 0)

    def test_missing_or_mismatched_audio_files_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_dir:
            root = Path(temporary_dir)
            manifest = load_manifest()
            fixture = ready_fixture(root)
            manifest["collection_status"] = "in_progress"
            manifest["fixtures"] = [fixture]

            fixture["audio_file"] = "audio/missing.wav"
            missing_errors, missing_counts = validate_manifest(manifest, root)
            self.assertTrue(any("audio_file does not identify an accessible regular file" in e for e in missing_errors))
            self.assertEqual(missing_counts["ready_audio_fixtures"], 0)

            fixture["audio_file"] = "audio/synthetic.wav"
            fixture["audio_sha256"] = "0" * 64
            mismatch_errors, mismatch_counts = validate_manifest(manifest, root)

        self.assertTrue(any("does not match the referenced audio file" in error for error in mismatch_errors))
        self.assertEqual(mismatch_counts["ready_audio_fixtures"], 0)


if __name__ == "__main__":
    unittest.main()
