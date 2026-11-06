"""Tests for exact AC-07 manifest-to-reference-bundle binding."""

from __future__ import annotations

import contextlib
import copy
import hashlib
import io
import json
import tempfile
import unittest
from dataclasses import FrozenInstanceError
from pathlib import Path

from scripts import validate_cleanup_ac07_reference_bundle as binding_validator
from scripts.tests.test_validate_cleanup_ac07_manifest import SYNTHETIC_HASH_SOURCES


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
MANIFEST_PATH = (
    REPOSITORY_ROOT / "scripts" / "testdata" / "synthetic_ac07_manifest.json"
)
BUNDLE_PATH = (
    REPOSITORY_ROOT
    / "scripts"
    / "testdata"
    / "synthetic_ac07_reference_bundle.json"
)


def load_json(path: Path) -> dict[str, object]:
    """Load a checked-in synthetic input for in-memory mutation tests."""
    return json.loads(path.read_text(encoding="utf-8"))


def load_manifest() -> dict[str, object]:
    return load_json(MANIFEST_PATH)


def load_bundle() -> dict[str, object]:
    return load_json(BUNDLE_PATH)


class CleanupReferenceBindingTests(unittest.TestCase):
    def test_valid_three_case_synthetic_binding(self) -> None:
        result = binding_validator.validate_cleanup_reference_bundle(
            load_manifest(), load_bundle()
        )

        self.assertEqual(result.corpus_id, "ac07-synthetic-manifest-demo")
        self.assertEqual(result.revision, "draft-001")
        self.assertEqual(result.manifest_case_count, 3)
        self.assertEqual(result.bundle_case_count, 3)
        self.assertEqual(result.matched_case_id_count, 3)
        self.assertEqual(result.verified_input_hash_count, 3)
        self.assertEqual(result.verified_expected_hash_count, 3)
        self.assertTrue(result.all_declared_case_ids_and_hashes_match)
        self.assertNotIn("input", result.__dataclass_fields__)
        self.assertNotIn("expected", result.__dataclass_fields__)

    def test_actual_content_hashes_match_preexisting_manifest(self) -> None:
        manifest_cases = {case["id"]: case for case in load_manifest()["cases"]}
        for bundle_case in load_bundle()["cases"]:
            input_source, expected_source = SYNTHETIC_HASH_SOURCES[bundle_case["id"]]
            self.assertEqual(bundle_case["input"], input_source)
            self.assertEqual(bundle_case["expected"], expected_source)
            manifest_case = manifest_cases[bundle_case["id"]]
            self.assertEqual(
                hashlib.sha256(bundle_case["input"].encode("utf-8")).hexdigest(),
                manifest_case["input_sha256"],
            )
            self.assertEqual(
                hashlib.sha256(bundle_case["expected"].encode("utf-8")).hexdigest(),
                manifest_case["expected_sha256"],
            )

    def test_reordered_bundle_validates_by_case_id(self) -> None:
        bundle = load_bundle()
        bundle["cases"] = list(reversed(bundle["cases"]))

        result = binding_validator.validate_cleanup_reference_bundle(
            load_manifest(), bundle
        )

        self.assertEqual(result.matched_case_id_count, 3)
        self.assertTrue(result.all_declared_case_ids_and_hashes_match)

    def test_missing_case_id_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["cases"].pop()

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "manifest case 3: field 'id'.*no matching",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_unexpected_extra_case_id_is_rejected(self) -> None:
        bundle = load_bundle()
        extra = copy.deepcopy(bundle["cases"][0])
        extra["id"] = "extra-case"
        bundle["cases"].append(extra)

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "reference bundle case 4: field 'id' is not declared",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_duplicate_bundle_id_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["cases"][1]["id"] = bundle["cases"][0]["id"]

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "reference bundle case 2: field 'id' duplicates",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_corpus_id_mismatch_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["corpus_id"] = "other-corpus"

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError, "corpus_id"
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_revision_mismatch_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["revision"] = "draft-002"

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError, "revision"
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_empty_or_invalid_corpus_id_is_rejected(self) -> None:
        for corpus_id in ("", " \t", None):
            with self.subTest(corpus_id_type=type(corpus_id).__name__):
                bundle = load_bundle()
                bundle["corpus_id"] = corpus_id
                with self.assertRaisesRegex(
                    binding_validator.CleanupReferenceBindingError,
                    "corpus_id.*nonempty string",
                ):
                    binding_validator.validate_cleanup_reference_bundle(
                        load_manifest(), bundle
                    )

    def test_empty_or_invalid_revision_is_rejected(self) -> None:
        for revision in ("", " \t", None):
            with self.subTest(revision_type=type(revision).__name__):
                bundle = load_bundle()
                bundle["revision"] = revision
                with self.assertRaisesRegex(
                    binding_validator.CleanupReferenceBindingError,
                    "revision.*nonempty string",
                ):
                    binding_validator.validate_cleanup_reference_bundle(
                        load_manifest(), bundle
                    )

    def test_empty_bundle_case_id_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["id"] = " \t"

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "reference bundle case 1: field 'id' must be a nonempty string",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_empty_case_collection_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["cases"] = []

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError, "nonempty array"
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_missing_case_field_is_rejected(self) -> None:
        bundle = load_bundle()
        del bundle["cases"][0]["expected"]

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "case 1: missing required field 'expected'",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_unknown_case_field_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["actual"] = "not part of the reference bundle"

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "case 1: object contains an unsupported field",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_invalid_bundle_schema_version_is_rejected(self) -> None:
        for version in (2, True, 1.0):
            with self.subTest(version=version):
                bundle = load_bundle()
                bundle["schema_version"] = version
                with self.assertRaisesRegex(
                    binding_validator.CleanupReferenceBindingError,
                    "schema_version",
                ):
                    binding_validator.validate_cleanup_reference_bundle(
                        load_manifest(), bundle
                    )

    def test_missing_required_field_is_rejected(self) -> None:
        bundle = load_bundle()
        del bundle["revision"]

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError, "revision"
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_unknown_field_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["unexpected"] = "metadata"

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError, "unsupported field"
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_non_string_input_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["input"] = 42

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "reference bundle case 1: field 'input' must be a string",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_non_string_expected_output_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["expected"] = None

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "reference bundle case 1: field 'expected' must be a string",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_one_byte_input_modification_fails_hash_binding(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["input"] += "!"

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "field 'input' SHA-256",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_one_byte_expected_modification_fails_hash_binding(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["expected"] += "!"

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "field 'expected' SHA-256",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_unicode_normalization_differences_are_not_accepted(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["input"] = "caf\u00e9"
        manifest = load_manifest()
        manifest["cases"][0]["input_sha256"] = hashlib.sha256(
            "caf\u00e9".encode("utf-8")
        ).hexdigest()

        # The bundle is decomposed while the unchanged manifest declares the
        # composed bytes, so no normalization may make the digests agree.
        bundle["cases"][0]["input"] = "cafe\u0301"
        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "field 'input' SHA-256",
        ):
            binding_validator.validate_cleanup_reference_bundle(manifest, bundle)

    def test_whitespace_differences_are_not_accepted(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["input"] += " "

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "field 'input' SHA-256",
        ):
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

    def test_empty_strings_are_valid_when_manifest_hashes_match(self) -> None:
        manifest = load_manifest()
        bundle = load_bundle()
        manifest["cases"][0]["input_sha256"] = hashlib.sha256(b"").hexdigest()
        manifest["cases"][0]["expected_sha256"] = hashlib.sha256(b"").hexdigest()
        bundle["cases"][0]["input"] = ""
        bundle["cases"][0]["expected"] = ""

        result = binding_validator.validate_cleanup_reference_bundle(manifest, bundle)

        self.assertEqual(result.verified_input_hash_count, 3)
        self.assertEqual(result.verified_expected_hash_count, 3)

    def test_invalid_utf8_string_is_rejected_without_echoing_content(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["input"] = "\ud800PRIVATE_SENTINEL"

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "field 'input' cannot be encoded as UTF-8",
        ) as context:
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

        self.assertNotIn("PRIVATE_SENTINEL", str(context.exception))

    def test_invalid_utf8_expected_string_is_rejected(self) -> None:
        bundle = load_bundle()
        bundle["cases"][0]["expected"] = "\ud800PRIVATE_SENTINEL"

        with self.assertRaisesRegex(
            binding_validator.CleanupReferenceBindingError,
            "field 'expected' cannot be encoded as UTF-8",
        ) as context:
            binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

        self.assertNotIn("PRIVATE_SENTINEL", str(context.exception))

    def test_duplicate_json_object_keys_are_rejected_when_loading_files(self) -> None:
        bundle_text = BUNDLE_PATH.read_text(encoding="utf-8")
        duplicate_text = bundle_text.replace(
            '"schema_version": 1,',
            '"schema_version": 1, "schema_version": 1,',
            1,
        )
        with tempfile.TemporaryDirectory() as temp_dir:
            path = Path(temp_dir) / "duplicate-bundle.json"
            path.write_text(duplicate_text, encoding="utf-8")

            with self.assertRaisesRegex(
                binding_validator.CleanupReferenceBindingError,
                "duplicate JSON field 'schema_version'",
            ):
                binding_validator.load_cleanup_reference_bundle(MANIFEST_PATH, path)

    def test_repeated_validation_is_deterministic(self) -> None:
        manifest = load_manifest()
        bundle = load_bundle()

        first = binding_validator.validate_cleanup_reference_bundle(
            manifest, bundle
        )
        second = binding_validator.validate_cleanup_reference_bundle(
            manifest, bundle
        )

        self.assertEqual(first, second)

    def test_result_dataclass_is_immutable(self) -> None:
        result = binding_validator.validate_cleanup_reference_bundle(
            load_manifest(), load_bundle()
        )

        with self.assertRaises(FrozenInstanceError):
            result.corpus_id = "changed"  # type: ignore[misc]

    def test_failure_returns_no_successful_partial_result(self) -> None:
        result = None
        bundle = load_bundle()
        bundle["cases"][0]["input"] += "!"

        with self.assertRaises(binding_validator.CleanupReferenceBindingError):
            result = binding_validator.validate_cleanup_reference_bundle(
                load_manifest(), bundle
            )

        self.assertIsNone(result)

    def test_demo_reports_pending_states_and_never_prints_case_content(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            status = binding_validator.main(["--demo"])

        self.assertEqual(status, 0)
        self.assertEqual(
            output.getvalue(),
            "AC-07 reference-bundle binding: SYNTHETIC DEMO\n"
            "Corpus ID: ac07-synthetic-manifest-demo\n"
            "Revision: draft-001\n"
            "Manifest cases: 3\n"
            "Bundle cases: 3\n"
            "Case-ID correspondence: PASS\n"
            "Input SHA-256 hashes verified: 3/3\n"
            "Expected SHA-256 hashes verified: 3/3\n"
            "Reference review: PENDING\n"
            "Corpus approval: NOT ESTABLISHED\n"
            "AC-07 acceptance: NOT EVALUATED\n",
        )
        for input_source, expected_source in SYNTHETIC_HASH_SOURCES.values():
            self.assertNotIn(input_source, output.getvalue())
            self.assertNotIn(expected_source, output.getvalue())


if __name__ == "__main__":
    unittest.main()
