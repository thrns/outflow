"""Tests for protected-token reference annotation binding."""

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
from unittest import mock

from scripts import validate_cleanup_ac07_protected_references as validator


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
TESTDATA_DIR = REPOSITORY_ROOT / "scripts" / "testdata"
MANIFEST_PATH = TESTDATA_DIR / "synthetic_ac07_manifest.json"
REFERENCE_PATH = TESTDATA_DIR / "synthetic_ac07_reference_bundle.json"
ANNOTATION_PATH = TESTDATA_DIR / "synthetic_ac07_protected_annotations.json"


def load_json(path: Path) -> dict[str, object]:
    """Read a checked-in synthetic fixture for in-memory mutation tests."""
    return json.loads(path.read_text(encoding="utf-8"))


def load_manifest() -> dict[str, object]:
    return load_json(MANIFEST_PATH)


def load_references() -> dict[str, object]:
    return load_json(REFERENCE_PATH)


def load_annotations() -> dict[str, object]:
    return load_json(ANNOTATION_PATH)


def validate(
    manifest: object | None = None,
    references: object | None = None,
    annotations: object | None = None,
) -> validator.CleanupProtectedReferenceBindingSummary:
    return validator.validate_cleanup_protected_references(
        load_manifest() if manifest is None else manifest,
        load_references() if references is None else references,
        load_annotations() if annotations is None else annotations,
    )


def make_single_case(
    input_text: str,
    expected_text: str,
    kinds: list[str],
    annotations: list[dict[str, object]],
) -> tuple[dict[str, object], dict[str, object], dict[str, object]]:
    """Build one hash-bound synthetic case; callers author offsets explicitly."""
    case_id = "in-memory-001"
    manifest_case = copy.deepcopy(load_manifest()["cases"][0])
    reference_case = copy.deepcopy(load_references()["cases"][0])
    manifest_case["id"] = case_id
    manifest_case["protected_kinds"] = kinds
    manifest_case["input_sha256"] = hashlib.sha256(
        input_text.encode("utf-8")
    ).hexdigest()
    manifest_case["expected_sha256"] = hashlib.sha256(
        expected_text.encode("utf-8")
    ).hexdigest()
    reference_case["id"] = case_id
    reference_case["input"] = input_text
    reference_case["expected"] = expected_text

    manifest = load_manifest()
    manifest["cases"] = [manifest_case]
    references = load_references()
    references["cases"] = [reference_case]
    annotation_bundle = load_annotations()
    annotation_bundle["cases"] = [
        {"id": case_id, "protected_tokens": annotations}
    ]
    return manifest, references, annotation_bundle


class CleanupProtectedReferenceBindingTests(unittest.TestCase):
    def test_valid_three_case_synthetic_binding(self) -> None:
        result = validate()

        self.assertEqual(result.corpus_id, "ac07-synthetic-manifest-demo")
        self.assertEqual(result.revision, "draft-001")
        self.assertEqual(result.reference_case_count, 3)
        self.assertEqual(result.annotation_case_count, 3)
        self.assertEqual(result.matched_case_id_count, 3)
        self.assertEqual(result.annotated_case_count, 2)
        self.assertEqual(result.numeric_reference_annotation_count, 1)
        self.assertEqual(result.proper_noun_reference_annotation_count, 1)
        self.assertTrue(result.protected_kind_declarations_agree)
        self.assertTrue(result.all_reference_byte_ranges_valid)
        self.assertNotIn("actual", result.__dataclass_fields__)
        self.assertNotIn("text", result.__dataclass_fields__)

    def test_reordered_annotation_cases_match_by_exact_id(self) -> None:
        annotations = load_annotations()
        annotations["cases"] = list(reversed(annotations["cases"]))

        result = validate(annotations=annotations)

        self.assertEqual(result.matched_case_id_count, 3)
        self.assertTrue(result.protected_kind_declarations_agree)

    def test_numeric_substring_uses_distinct_input_and_expected_offsets(self) -> None:
        case = load_annotations()["cases"][0]
        token = case["protected_tokens"][0]

        self.assertEqual(token["text"], "001")
        self.assertEqual(token["input_bytes"], [37, 40])
        self.assertEqual(token["output_bytes"], [40, 43])
        self.assertTrue(validate().all_reference_byte_ranges_valid)

    def test_accented_name_uses_utf8_byte_offsets(self) -> None:
        manifest, references, annotations = make_single_case(
            "Élodie has 42",
            "Élodie, has 42",
            ["proper_noun", "number"],
            [
                {
                    "kind": "proper_noun",
                    "text": "Élodie",
                    "input_bytes": [0, 7],
                    "output_bytes": [0, 7],
                },
                {
                    "kind": "number",
                    "text": "42",
                    "input_bytes": [12, 14],
                    "output_bytes": [13, 15],
                },
            ],
        )

        result = validate(manifest, references, annotations)

        self.assertEqual(result.annotated_case_count, 1)
        self.assertEqual(result.numeric_reference_annotation_count, 1)
        self.assertEqual(result.proper_noun_reference_annotation_count, 1)

    def test_multiple_annotations_in_one_case_are_valid(self) -> None:
        manifest, references, annotations = make_single_case(
            "Ref 42 AC-07",
            "Ref: 42 AC-07",
            ["number", "proper_noun"],
            [
                {
                    "kind": "number",
                    "text": "42",
                    "input_bytes": [4, 6],
                    "output_bytes": [5, 7],
                },
                {
                    "kind": "proper_noun",
                    "text": "AC-07",
                    "input_bytes": [7, 12],
                    "output_bytes": [8, 13],
                },
            ],
        )

        result = validate(manifest, references, annotations)

        self.assertEqual(result.annotated_case_count, 1)
        self.assertEqual(result.numeric_reference_annotation_count, 1)
        self.assertEqual(result.proper_noun_reference_annotation_count, 1)

    def test_repeated_nonoverlapping_annotations_of_one_kind_are_valid(self) -> None:
        manifest, references, annotations = make_single_case(
            "42 and 42",
            "42 and 42",
            ["number"],
            [
                {
                    "kind": "number",
                    "text": "42",
                    "input_bytes": [0, 2],
                    "output_bytes": [0, 2],
                },
                {
                    "kind": "number",
                    "text": "42",
                    "input_bytes": [7, 9],
                    "output_bytes": [7, 9],
                },
            ],
        )

        result = validate(manifest, references, annotations)

        self.assertEqual(result.numeric_reference_annotation_count, 2)
        self.assertEqual(result.annotated_case_count, 1)

    def test_missing_and_extra_case_ids_are_rejected(self) -> None:
        annotations = load_annotations()
        annotations["cases"] = annotations["cases"][:-1]
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "reference case 3: field 'id'.*no matching",
        ):
            validate(annotations=annotations)

        annotations = load_annotations()
        annotations["cases"][2]["id"] = "extra-case"
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "annotation bundle case 3: field 'id' is not declared",
        ):
            validate(annotations=annotations)

    def test_duplicate_annotation_case_id_is_rejected(self) -> None:
        annotations = load_annotations()
        annotations["cases"][1]["id"] = annotations["cases"][0]["id"]

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "annotation bundle case 2: field 'id' duplicates",
        ):
            validate(annotations=annotations)

    def test_corpus_id_and_revision_mismatches_are_rejected(self) -> None:
        annotations = load_annotations()
        annotations["corpus_id"] = "other-corpus"
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "annotation bundle: field 'corpus_id' does not match",
        ):
            validate(annotations=annotations)

        annotations = load_annotations()
        annotations["revision"] = "draft-002"
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "annotation bundle: field 'revision' does not match",
        ):
            validate(annotations=annotations)

    def test_unsupported_schema_version_is_rejected(self) -> None:
        for version in (2, True, 1.0):
            with self.subTest(version=version):
                annotations = load_annotations()
                annotations["schema_version"] = version
                with self.assertRaisesRegex(
                    validator.CleanupProtectedReferenceBindingError,
                    "field 'schema_version' must equal integer 1",
                ):
                    validate(annotations=annotations)

    def test_empty_annotation_case_id_is_rejected(self) -> None:
        annotations = load_annotations()
        annotations["cases"][0]["id"] = " \t"

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "annotation bundle case 1: field 'id' must be a nonempty string",
        ):
            validate(annotations=annotations)

    def test_unsupported_token_kind_is_rejected(self) -> None:
        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"][0]["kind"] = "date"

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "annotation bundle case 1, protected token 1: field 'kind' is unsupported",
        ):
            validate(annotations=annotations)

    def test_annotation_kind_set_must_match_manifest_declaration(self) -> None:
        manifest = load_manifest()
        manifest["cases"][0]["protected_kinds"] = []

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "protected_tokens' kinds disagree with manifest field 'protected_kinds'",
        ):
            validate(manifest=manifest)

    def test_missing_required_fields_are_rejected(self) -> None:
        annotations = load_annotations()
        del annotations["revision"]
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "annotation bundle: missing required field 'revision'",
        ):
            validate(annotations=annotations)

        annotations = load_annotations()
        del annotations["cases"][0]["protected_tokens"]
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "annotation bundle case 1: missing required field 'protected_tokens'",
        ):
            validate(annotations=annotations)

        annotations = load_annotations()
        del annotations["cases"][0]["protected_tokens"][0]["text"]
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "protected token 1: missing required field 'text'",
        ):
            validate(annotations=annotations)

    def test_unknown_fields_are_rejected(self) -> None:
        annotations = load_annotations()
        annotations["unexpected"] = "field"
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "annotation bundle: object contains an unsupported field",
        ):
            validate(annotations=annotations)

        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"][0]["unexpected"] = 1
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "protected token 1: object contains an unsupported field",
        ):
            validate(annotations=annotations)

    def test_incorrect_declared_token_text_is_rejected(self) -> None:
        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"][0]["text"] = "synthetic sentinel"

        with self.assertRaises(
            validator.CleanupProtectedReferenceBindingError
        ) as context:
            validate(annotations=annotations)

        self.assertIn(
            "field 'input_bytes' does not select the declared token",
            str(context.exception),
        )
        self.assertNotIn("synthetic sentinel", str(context.exception))

    def test_incorrect_expected_output_range_is_rejected(self) -> None:
        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"][0]["output_bytes"] = [39, 42]

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "field 'output_bytes' does not select the declared token",
        ):
            validate(annotations=annotations)

    def test_out_of_bounds_range_is_rejected(self) -> None:
        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"][0]["output_bytes"] = [40, 44]

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "field 'output_bytes' range is out of bounds",
        ):
            validate(annotations=annotations)

    def test_utf8_boundary_violation_is_rejected(self) -> None:
        manifest, references, annotations = make_single_case(
            "Élodie",
            "Élodie",
            ["proper_noun"],
            [
                {
                    "kind": "proper_noun",
                    "text": "lodie",
                    "input_bytes": [2, 7],
                    "output_bytes": [1, 6],
                }
            ],
        )

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "field 'output_bytes' range must start and end at UTF-8 character boundaries",
        ):
            validate(manifest, references, annotations)

    def test_overlapping_annotations_are_rejected(self) -> None:
        manifest, references, annotations = make_single_case(
            "1234",
            "1234",
            ["number"],
            [
                {
                    "kind": "number",
                    "text": "123",
                    "input_bytes": [0, 3],
                    "output_bytes": [0, 3],
                },
                {
                    "kind": "number",
                    "text": "34",
                    "input_bytes": [2, 4],
                    "output_bytes": [2, 4],
                },
            ],
        )

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "input_bytes' overlaps another protected input range",
        ):
            validate(manifest, references, annotations)

    def test_overlapping_expected_ranges_are_rejected(self) -> None:
        manifest, references, annotations = make_single_case(
            "1111",
            "11",
            ["number"],
            [
                {
                    "kind": "number",
                    "text": "11",
                    "input_bytes": [0, 2],
                    "output_bytes": [0, 2],
                },
                {
                    "kind": "number",
                    "text": "11",
                    "input_bytes": [2, 4],
                    "output_bytes": [0, 2],
                },
            ],
        )

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "output_bytes' overlaps another protected expected-output range",
        ):
            validate(manifest, references, annotations)

    def test_duplicate_json_object_keys_are_rejected_from_file_input(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            manifest_path = directory / "manifest.json"
            reference_path = directory / "references.json"
            annotation_path = directory / "annotations.json"
            manifest_path.write_text(json.dumps(load_manifest()), encoding="utf-8")
            reference_path.write_text(
                json.dumps(load_references()), encoding="utf-8"
            )
            annotation_path.write_text(
                '{"schema_version":1,"corpus_id":"ac07-synthetic-manifest-demo",'
                '"revision":"draft-001","revision":"draft-001","cases":[]}',
                encoding="utf-8",
            )

            with self.assertRaisesRegex(
                validator.CleanupProtectedReferenceBindingError,
                "annotation bundle: duplicate JSON field 'revision'",
            ):
                validator.load_cleanup_protected_reference_files(
                    manifest_path, reference_path, annotation_path
                )

    def test_invalid_and_non_string_values_are_rejected(self) -> None:
        annotations = load_annotations()
        annotations["cases"][0]["id"] = 7
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "field 'id' must be a nonempty string",
        ):
            validate(annotations=annotations)

        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"][0]["text"] = None
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "field 'text' must be a nonempty string",
        ):
            validate(annotations=annotations)

        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"][0]["text"] = "\ud800"
        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "field 'text' must be valid UTF-8",
        ):
            validate(annotations=annotations)

    def test_empty_annotations_fail_when_manifest_declares_protection(self) -> None:
        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"] = []

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "kinds disagree with manifest field 'protected_kinds'",
        ):
            validate(annotations=annotations)

    def test_nonempty_annotations_fail_when_manifest_declares_no_protection(self) -> None:
        annotations = load_annotations()
        annotations["cases"][1]["protected_tokens"] = [
            {
                "kind": "proper_noun",
                "text": "AC-07",
                "input_bytes": [0, 5],
                "output_bytes": [0, 5],
            }
        ]

        with self.assertRaisesRegex(
            validator.CleanupProtectedReferenceBindingError,
            "kinds disagree with manifest field 'protected_kinds'",
        ):
            validate(annotations=annotations)

    def test_reference_hash_failure_precedes_annotation_validation(self) -> None:
        for field in ("input", "expected"):
            with self.subTest(field=field):
                references = load_references()
                references["cases"][0][field] += "!"
                annotations = load_annotations()
                annotations["unexpected"] = "would fail if reached"

                with self.assertRaisesRegex(
                    validator.CleanupProtectedReferenceBindingError,
                    f"reference binding failed: reference bundle case 1: field '{field}' SHA-256",
                ):
                    validate(references=references, annotations=annotations)

    def test_repeated_validation_is_deterministic(self) -> None:
        first = validate()
        second = validate()

        self.assertEqual(first, second)

    def test_summary_is_immutable(self) -> None:
        result = validate()

        with self.assertRaises(FrozenInstanceError):
            result.reference_case_count = 4  # type: ignore[misc]

    def test_failure_returns_no_partial_summary(self) -> None:
        annotations = load_annotations()
        annotations["cases"][0]["protected_tokens"][0]["output_bytes"] = [40, 44]
        summary_constructor = validator.CleanupProtectedReferenceBindingSummary

        with mock.patch.object(
            validator,
            "CleanupProtectedReferenceBindingSummary",
            wraps=summary_constructor,
        ) as constructor:
            with self.assertRaises(
                validator.CleanupProtectedReferenceBindingError
            ):
                validate(annotations=annotations)

        constructor.assert_not_called()

    def test_demo_reports_synthetic_validation_without_approval_or_acceptance(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = validator.main(["--demo"])

        self.assertEqual(exit_code, 0)
        self.assertEqual(
            output.getvalue(),
            "AC-07 protected-reference binding: SYNTHETIC DEMO\n"
            "Corpus ID: ac07-synthetic-manifest-demo\n"
            "Revision: draft-001\n"
            "Reference cases: 3\n"
            "Annotation cases: 3\n"
            "Case-ID correspondence: PASS\n"
            "Protected kind declarations: PASS\n"
            "Protected numeric references: 1\n"
            "Protected proper-noun references: 1\n"
            "UTF-8 reference ranges: PASS\n"
            "Reference review: PENDING\n"
            "Corpus approval: NOT ESTABLISHED\n"
            "AC-07 acceptance: NOT EVALUATED\n",
        )
        self.assertNotIn("synthetic expected bytes", output.getvalue())
        self.assertNotIn("Protected-token gate: PASS", output.getvalue())

    def test_malformed_and_invalid_utf8_json_files_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temporary_directory:
            directory = Path(temporary_directory)
            manifest_path = directory / "manifest.json"
            reference_path = directory / "references.json"
            annotation_path = directory / "annotations.json"
            manifest_path.write_text(json.dumps(load_manifest()), encoding="utf-8")
            reference_path.write_text(
                json.dumps(load_references()), encoding="utf-8"
            )

            annotation_path.write_text("{", encoding="utf-8")
            with self.assertRaisesRegex(
                validator.CleanupProtectedReferenceBindingError,
                "annotation bundle JSON is invalid",
            ):
                validator.load_cleanup_protected_reference_files(
                    manifest_path, reference_path, annotation_path
                )

            annotation_path.write_bytes(b"\xff")
            with self.assertRaisesRegex(
                validator.CleanupProtectedReferenceBindingError,
                "annotation bundle file could not be read as UTF-8 JSON",
            ):
                validator.load_cleanup_protected_reference_files(
                    manifest_path, reference_path, annotation_path
                )


if __name__ == "__main__":
    unittest.main()
