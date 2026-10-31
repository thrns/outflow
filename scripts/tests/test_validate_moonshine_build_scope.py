"""Tests for the planning-only Moonshine build-scope validator."""

from __future__ import annotations

import copy
import json
from pathlib import Path
import unittest
from typing import Any

from scripts.validate_moonshine_build_scope import render_summary, validate_scope


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
SCOPE_PATH = REPOSITORY_ROOT / "docs" / "moonshine-build-scope.json"
INVENTORY_PATH = REPOSITORY_ROOT / "docs" / "moonshine-third-party.json"
NOTICES_PATH = REPOSITORY_ROOT / "docs" / "third-party-notices.json"


def load_documents() -> tuple[dict[str, Any], dict[str, Any], dict[str, Any]]:
    return tuple(
        json.loads(path.read_text(encoding="utf-8"))
        for path in (SCOPE_PATH, INVENTORY_PATH, NOTICES_PATH)
    )  # type: ignore[return-value]


class MoonshineBuildScopeValidatorTests(unittest.TestCase):
    def validate(self, manifest: dict[str, Any] | None = None) -> tuple[list[str], dict[str, Any]]:
        source, inventory, notices = load_documents()
        errors, summary = validate_scope(
            copy.deepcopy(manifest if manifest is not None else source),
            inventory,
            notices,
        )
        return errors, summary

    def test_valid_planning_manifest_passes_without_claiming_a_build(self) -> None:
        manifest, _, _ = load_documents()

        errors, summary = self.validate(manifest)

        self.assertEqual(errors, [])
        self.assertEqual(manifest["execution"]["compilation"], "NOT_RUN")
        self.assertEqual(manifest["execution"]["binary_contents"], "NOT_VERIFIED")
        self.assertEqual(summary["feature_count"], len(manifest["features"]))
        self.assertEqual(summary["question_count"], len(manifest["unresolved_build_questions"]))

    def test_unsupported_schema_version_is_rejected(self) -> None:
        manifest, _, _ = load_documents()
        manifest["schema_version"] = 2

        errors, _ = self.validate(manifest)

        self.assertIn("schema_version must be 1", errors)

    def test_incorrect_source_revision_is_rejected(self) -> None:
        manifest, _, _ = load_documents()
        manifest["source"]["revision"] = "main"

        errors, _ = self.validate(manifest)

        self.assertTrue(any("source.revision must be" in error for error in errors))

    def test_duplicate_feature_ids_are_rejected(self) -> None:
        manifest, _, _ = load_documents()
        manifest["features"].append(copy.deepcopy(manifest["features"][0]))

        errors, _ = self.validate(manifest)

        self.assertTrue(any("duplicates another feature identifier" in error for error in errors))

    def test_missing_required_fields_are_rejected(self) -> None:
        manifest, _, _ = load_documents()
        del manifest["intended_build_target"]

        errors, _ = self.validate(manifest)

        self.assertTrue(any("build-scope is missing required field" in error for error in errors))

    def test_invalid_scope_classification_is_rejected(self) -> None:
        manifest, _, _ = load_documents()
        manifest["features"][0]["intended_scope"] = "optional"

        errors, _ = self.validate(manifest)

        self.assertTrue(any("intended_scope must be required, excluded, or undetermined" in error for error in errors))

    def test_invalid_source_evidence_status_is_rejected(self) -> None:
        manifest, _, _ = load_documents()
        manifest["features"][0]["source_evidence_status"] = "compiled"

        errors, _ = self.validate(manifest)

        self.assertTrue(any("source_evidence_status must be confirmed, conditional, or unverified" in error for error in errors))

    def test_malformed_or_mutable_evidence_url_is_rejected(self) -> None:
        manifest, _, _ = load_documents()
        manifest["source_evidence"][0]["url"] = (
            "https://github.com/moonshine-ai/moonshine/blob/main/core/CMakeLists.txt#L110-L122"
        )

        errors, _ = self.validate(manifest)

        self.assertTrue(any("immutable pinned source URL" in error for error in errors))

    def test_unsupported_or_invented_build_option_claim_is_rejected(self) -> None:
        manifest, _, _ = load_documents()
        feature = next(item for item in manifest["features"] if item["feature_id"] == "tts-g2p")
        feature["build_control"]["option_name"] = "MOONSHINE_BUILD_TESTS"

        errors, _ = self.validate(manifest)

        self.assertTrue(any("unsupported or invented build option" in error for error in errors))

    def test_false_compilation_or_binary_verification_claim_is_rejected(self) -> None:
        manifest, _, _ = load_documents()
        manifest["execution"]["compilation"] = "PASS"
        manifest["execution"]["binary_contents"] = "VERIFIED"
        manifest["features"][0]["actual_compiled_status"] = "compiled"

        errors, _ = self.validate(manifest)

        self.assertTrue(any("execution.compilation must remain NOT_RUN" in error for error in errors))
        self.assertTrue(any("execution.binary_contents must remain NOT_VERIFIED" in error for error in errors))
        self.assertTrue(any("actual_compiled_status must remain not-tested" in error for error in errors))

    def test_excluded_feature_keeps_unresolved_build_behavior_open(self) -> None:
        manifest, _, _ = load_documents()
        diarization = next(
            item for item in manifest["features"] if item["feature_id"] == "speaker-diarization"
        )
        diarization["unresolved_question_ids"] = []

        errors, _ = self.validate(manifest)

        self.assertTrue(any("must retain unresolved build questions" in error for error in errors))

    def test_build_options_record_verified_defaults_and_unknown_shared_default(self) -> None:
        manifest, _, _ = load_documents()
        option = next(
            item for item in manifest["build_options"] if item["name"] == "MOONSHINE_TTS_BUILD_ONNX"
        )
        option["default"] = "OFF"

        errors, _ = self.validate(manifest)

        self.assertTrue(any("default does not match the reviewed CMake value" in error for error in errors))

    def test_dependency_inventory_and_notice_mismatches_are_rejected(self) -> None:
        manifest, inventory, notices = load_documents()
        inventory["dependencies"][0]["build_use_classification"] = "test-only"
        errors, _ = validate_scope(manifest, inventory, notices)

        self.assertTrue(any("notice manifest classification differs" in error for error in errors))
        self.assertTrue(any("build scope classification differs" in error for error in errors))

    def test_cpu_and_platform_certification_claims_are_rejected(self) -> None:
        manifest, _, _ = load_documents()
        manifest["application"]["cpu_only_verification_status"] = "certified"
        manifest["application"]["platform_verification_status"] = "PASS"

        errors, _ = self.validate(manifest)

        self.assertTrue(any("cpu_only_verification_status must remain 'not-tested'" in error for error in errors))
        self.assertTrue(any("platform_verification_status must remain 'not-tested'" in error for error in errors))

    def test_summary_output_is_deterministic_and_derived_from_manifest(self) -> None:
        manifest, _, _ = load_documents()
        errors, summary = self.validate(manifest)
        self.assertEqual(errors, [])

        first = render_summary(manifest, summary)
        second = render_summary(manifest, summary)

        self.assertEqual(first, second)
        self.assertEqual(
            first,
            "\n".join(
                (
                    "Outflow Moonshine build scope valid",
                    "Source revision: 234f60faa0eb388b01cdf7e60aca232af37aefda",
                    "Target: English streaming speech-to-text, CPU-only",
                    "Feature scope: 6 required, 6 excluded, 0 undetermined",
                    "Source evidence: 6 confirmed, 4 conditional, 2 unverified",
                    "Unresolved build questions: 11",
                    "Build plan: DOCUMENTED",
                    "Compilation: NOT RUN",
                    "Binary contents: NOT VERIFIED",
                    "Minimal runtime feasibility: NOT ASSESSED",
                )
            ),
        )


if __name__ == "__main__":
    unittest.main()
