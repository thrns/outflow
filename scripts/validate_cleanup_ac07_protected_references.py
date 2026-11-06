#!/usr/bin/env python3
"""Bind protected-token annotations to verified AC-07 reference strings.

This development-only validator checks annotation structure and reference byte
ranges after the manifest/reference hashes have been verified. It never runs
cleanup or establishes protected-token scoring or AC-07 acceptance.
"""

from __future__ import annotations

import argparse
import json
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

try:  # Support direct script execution and package-based imports.
    from .score_cleanup_protected_ac07 import (
        ProtectedCleanupValidationError,
        score_protected_cleanup_cases,
    )
    from .validate_cleanup_ac07_reference_bundle import (
        CleanupReferenceBindingError,
        CleanupReferenceBindingSummary,
        validate_cleanup_reference_bundle,
    )
except ImportError:  # pragma: no cover - exercised by direct CLI invocation.
    from score_cleanup_protected_ac07 import (
        ProtectedCleanupValidationError,
        score_protected_cleanup_cases,
    )
    from validate_cleanup_ac07_reference_bundle import (
        CleanupReferenceBindingError,
        CleanupReferenceBindingSummary,
        validate_cleanup_reference_bundle,
    )


SCHEMA_VERSION = 1
TOKEN_KINDS = frozenset({"number", "proper_noun"})
BUNDLE_FIELDS = frozenset({"schema_version", "corpus_id", "revision", "cases"})
CASE_FIELDS = frozenset({"id", "protected_tokens"})
ANNOTATION_FIELDS = frozenset(
    {"kind", "text", "input_bytes", "output_bytes"}
)


class CleanupProtectedReferenceBindingError(ValueError):
    """A protected-token annotation bundle failed reference binding checks."""


@dataclass(frozen=True)
class CleanupProtectedReferenceBindingSummary:
    """Immutable, content-free summary of validated reference annotations.

    This summary reports reference integrity only. It intentionally contains
    no transcript/token text, actual-output measurements, or acceptance flag.
    """

    corpus_id: str
    revision: str
    reference_case_count: int
    annotation_case_count: int
    matched_case_id_count: int
    annotated_case_count: int
    numeric_reference_annotation_count: int
    proper_noun_reference_annotation_count: int
    protected_kind_declarations_agree: bool
    all_reference_byte_ranges_valid: bool


class _JSONObject(dict[str, object]):
    """Parsed JSON object that retains duplicate-key evidence."""

    def __init__(self, pairs: list[tuple[str, object]]) -> None:
        super().__init__()
        duplicates: list[str] = []
        for key, value in pairs:
            if key in self:
                duplicates.append(key)
            self[key] = value
        self.duplicate_keys = tuple(duplicates)


def _strict_fields(
    value: Mapping[object, object],
    expected: frozenset[str],
    context: str,
) -> None:
    duplicate_keys = getattr(value, "duplicate_keys", ())
    if duplicate_keys:
        key = duplicate_keys[0]
        if isinstance(key, str) and key in expected:
            raise CleanupProtectedReferenceBindingError(
                f"{context}: duplicate JSON field '{key}'"
            )
        raise CleanupProtectedReferenceBindingError(
            f"{context}: duplicate JSON object field"
        )

    if any(not isinstance(key, str) for key in value):
        raise CleanupProtectedReferenceBindingError(
            f"{context}: object contains an unsupported field"
        )

    present = set(value)
    missing = expected - present
    if missing:
        field = sorted(missing)[0]
        raise CleanupProtectedReferenceBindingError(
            f"{context}: missing required field '{field}'"
        )
    if present - expected:
        raise CleanupProtectedReferenceBindingError(
            f"{context}: object contains an unsupported field"
        )


def _nonempty_utf8_string(value: object, field: str, context: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise CleanupProtectedReferenceBindingError(
            f"{context}: field '{field}' must be a nonempty string"
        )
    try:
        value.encode("utf-8")
    except UnicodeEncodeError as error:
        raise CleanupProtectedReferenceBindingError(
            f"{context}: field '{field}' must be valid UTF-8"
        ) from error
    return value


def _invalid_json_constant(_value: str) -> None:
    """Reject Python's non-standard JSON NaN and Infinity extensions."""
    raise ValueError


def _load_json(path: str | Path, document: str) -> object:
    """Load one UTF-8 JSON document while retaining duplicate object keys."""
    try:
        raw_text = Path(path).read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise CleanupProtectedReferenceBindingError(
            f"{document} file could not be read as UTF-8 JSON"
        ) from error

    try:
        return json.loads(
            raw_text,
            object_pairs_hook=_JSONObject,
            parse_constant=_invalid_json_constant,
        )
    except json.JSONDecodeError as error:
        raise CleanupProtectedReferenceBindingError(
            f"{document} JSON is invalid at line {error.lineno}, "
            f"column {error.colno}"
        ) from error
    except ValueError as error:
        raise CleanupProtectedReferenceBindingError(
            f"{document} JSON contains an invalid value"
        ) from error


def _bind_reference_or_raise(
    manifest: object,
    reference_bundle: object,
) -> CleanupReferenceBindingSummary:
    """Run the established hash/ID binder and redact under this API's error."""
    try:
        return validate_cleanup_reference_bundle(manifest, reference_bundle)
    except CleanupReferenceBindingError as error:
        raise CleanupProtectedReferenceBindingError(
            f"reference binding failed: {error}"
        ) from error


def _validate_annotation_bundle(
    manifest: object,
    reference_bundle: object,
    annotation_bundle: object,
    reference_summary: CleanupReferenceBindingSummary,
) -> CleanupProtectedReferenceBindingSummary:
    if not isinstance(annotation_bundle, Mapping):
        raise CleanupProtectedReferenceBindingError(
            "annotation bundle must be a JSON object"
        )
    _strict_fields(annotation_bundle, BUNDLE_FIELDS, "annotation bundle")

    version = annotation_bundle["schema_version"]
    if type(version) is not int or version != SCHEMA_VERSION:
        raise CleanupProtectedReferenceBindingError(
            "annotation bundle: field 'schema_version' must equal integer 1"
        )

    corpus_id = _nonempty_utf8_string(
        annotation_bundle["corpus_id"], "corpus_id", "annotation bundle"
    )
    revision = _nonempty_utf8_string(
        annotation_bundle["revision"], "revision", "annotation bundle"
    )
    if corpus_id != reference_summary.corpus_id:
        raise CleanupProtectedReferenceBindingError(
            "annotation bundle: field 'corpus_id' does not match verified references"
        )
    if revision != reference_summary.revision:
        raise CleanupProtectedReferenceBindingError(
            "annotation bundle: field 'revision' does not match verified references"
        )

    annotation_cases = annotation_bundle["cases"]
    if not isinstance(annotation_cases, list) or not annotation_cases:
        raise CleanupProtectedReferenceBindingError(
            "annotation bundle: field 'cases' must be a nonempty array"
        )

    # The reference binder has already validated both collections and their
    # exact identity correspondence, so these maps contain only unique IDs.
    manifest_cases = manifest["cases"]  # type: ignore[index]
    reference_cases = reference_bundle["cases"]  # type: ignore[index]
    manifest_by_id = {case["id"]: case for case in manifest_cases}
    reference_by_id = {case["id"]: case for case in reference_cases}

    seen_ids: set[str] = set()
    scoring_records: list[dict[str, object]] = []
    annotated_case_count = 0
    numeric_annotation_count = 0
    proper_noun_annotation_count = 0

    for case_number, case in enumerate(annotation_cases, start=1):
        context = f"annotation bundle case {case_number}"
        if not isinstance(case, Mapping):
            raise CleanupProtectedReferenceBindingError(
                f"{context}: record must be an object"
            )
        _strict_fields(case, CASE_FIELDS, context)

        case_id = _nonempty_utf8_string(case["id"], "id", context)
        if case_id in seen_ids:
            raise CleanupProtectedReferenceBindingError(
                f"{context}: field 'id' duplicates an earlier case"
            )
        seen_ids.add(case_id)
        if case_id not in reference_by_id:
            raise CleanupProtectedReferenceBindingError(
                f"{context}: field 'id' is not declared in verified references"
            )

        annotations = case["protected_tokens"]
        if not isinstance(annotations, list):
            raise CleanupProtectedReferenceBindingError(
                f"{context}: field 'protected_tokens' must be an array"
            )

        annotation_kinds: set[str] = set()
        scorer_annotations: list[dict[str, object]] = []
        for token_number, annotation in enumerate(annotations, start=1):
            token_context = f"{context}, protected token {token_number}"
            if not isinstance(annotation, Mapping):
                raise CleanupProtectedReferenceBindingError(
                    f"{token_context}: record must be an object"
                )
            _strict_fields(annotation, ANNOTATION_FIELDS, token_context)

            kind = annotation["kind"]
            if not isinstance(kind, str) or kind not in TOKEN_KINDS:
                raise CleanupProtectedReferenceBindingError(
                    f"{token_context}: field 'kind' is unsupported"
                )
            text = annotation["text"]
            if not isinstance(text, str) or not text:
                raise CleanupProtectedReferenceBindingError(
                    f"{token_context}: field 'text' must be a nonempty string"
                )
            try:
                text.encode("utf-8")
            except UnicodeEncodeError as error:
                raise CleanupProtectedReferenceBindingError(
                    f"{token_context}: field 'text' must be valid UTF-8"
                ) from error

            annotation_kinds.add(kind)
            if kind == "number":
                numeric_annotation_count += 1
            else:
                proper_noun_annotation_count += 1
            scorer_annotations.append(
                {field: annotation[field] for field in ANNOTATION_FIELDS}
            )

        manifest_case = manifest_by_id[case_id]
        if annotation_kinds != set(manifest_case["protected_kinds"]):
            raise CleanupProtectedReferenceBindingError(
                f"{context}: field 'protected_tokens' kinds disagree with "
                "manifest field 'protected_kinds'"
            )

        if annotations:
            annotated_case_count += 1
        reference_case = reference_by_id[case_id]
        scoring_records.append(
            {
                "id": case_id,
                "input": reference_case["input"],
                "expected": reference_case["expected"],
                # The scorer is reused only to validate annotation metadata.
                # This equality is not a production cleanup measurement.
                "actual": reference_case["expected"],
                "protected_tokens": scorer_annotations,
            }
        )

    for reference_case_number, reference_case in enumerate(reference_cases, start=1):
        if reference_case["id"] not in seen_ids:
            raise CleanupProtectedReferenceBindingError(
                f"reference case {reference_case_number}: field 'id' has no "
                "matching annotation bundle case"
            )

    try:
        # Only the scorer's validation errors matter here. Its returned counts,
        # alterations, and gate result are deliberately discarded.
        score_protected_cleanup_cases(scoring_records)
    except ProtectedCleanupValidationError as error:
        raise CleanupProtectedReferenceBindingError(
            f"reference annotation validation failed: {error}"
        ) from error

    return CleanupProtectedReferenceBindingSummary(
        corpus_id=reference_summary.corpus_id,
        revision=reference_summary.revision,
        reference_case_count=reference_summary.bundle_case_count,
        annotation_case_count=len(annotation_cases),
        matched_case_id_count=len(annotation_cases),
        annotated_case_count=annotated_case_count,
        numeric_reference_annotation_count=numeric_annotation_count,
        proper_noun_reference_annotation_count=proper_noun_annotation_count,
        protected_kind_declarations_agree=True,
        all_reference_byte_ranges_valid=True,
    )


def validate_cleanup_protected_references(
    manifest: object,
    reference_bundle: object,
    annotation_bundle: object,
) -> CleanupProtectedReferenceBindingSummary:
    """Validate independent annotations against hash-bound reference strings.

    The existing manifest/reference validator runs first. Annotation case IDs
    then match those references exactly, independent of array order, and their
    kinds must equal each manifest case's declared kind set. The existing
    protected-token scorer validates UTF-8 byte ranges and overlap using
    ``actual=expected`` solely as a reference-validation input; no score or
    acceptance result is returned.

    Raises:
        CleanupProtectedReferenceBindingError: If any identity, structure,
            declaration, reference string, or byte-range check fails. Messages
            identify case positions and fields without printing IDs or text.
    """
    reference_summary = _bind_reference_or_raise(manifest, reference_bundle)
    return _validate_annotation_bundle(
        manifest, reference_bundle, annotation_bundle, reference_summary
    )


def load_cleanup_protected_reference_files(
    manifest_path: str | Path,
    reference_bundle_path: str | Path,
    annotation_bundle_path: str | Path,
) -> CleanupProtectedReferenceBindingSummary:
    """Load UTF-8 JSON artifacts and bind annotations to verified references.

    The manifest/reference pair is bound before annotation-file parsing, so
    annotation processing cannot mask a failed hash or case-ID binding.
    """
    manifest = _load_json(manifest_path, "manifest")
    reference_bundle = _load_json(reference_bundle_path, "reference bundle")
    reference_summary = _bind_reference_or_raise(manifest, reference_bundle)
    annotation_bundle = _load_json(annotation_bundle_path, "annotation bundle")
    return _validate_annotation_bundle(
        manifest, reference_bundle, annotation_bundle, reference_summary
    )


def _print_demo(summary: CleanupProtectedReferenceBindingSummary) -> None:
    """Print only verified identity and counts after all checks pass."""
    print("AC-07 protected-reference binding: SYNTHETIC DEMO")
    print(f"Corpus ID: {summary.corpus_id}")
    print(f"Revision: {summary.revision}")
    print(f"Reference cases: {summary.reference_case_count}")
    print(f"Annotation cases: {summary.annotation_case_count}")
    print(
        "Case-ID correspondence: "
        f"{'PASS' if summary.matched_case_id_count == summary.reference_case_count else 'FAIL'}"
    )
    print(
        "Protected kind declarations: "
        f"{'PASS' if summary.protected_kind_declarations_agree else 'FAIL'}"
    )
    print(f"Protected numeric references: {summary.numeric_reference_annotation_count}")
    print(
        "Protected proper-noun references: "
        f"{summary.proper_noun_reference_annotation_count}"
    )
    print(
        "UTF-8 reference ranges: "
        f"{'PASS' if summary.all_reference_byte_ranges_valid else 'FAIL'}"
    )
    print("Reference review: PENDING")
    print("Corpus approval: NOT ESTABLISHED")
    print("AC-07 acceptance: NOT EVALUATED")


def main(argv: Sequence[str] | None = None) -> int:
    """Run the checked-in synthetic demonstration."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--demo",
        action="store_true",
        help="validate only the checked-in synthetic manifest, references, and annotations",
    )
    args = parser.parse_args(argv)
    if not args.demo:
        parser.error("--demo is required")

    script_dir = Path(__file__).resolve().parent
    testdata_dir = script_dir / "testdata"
    try:
        summary = load_cleanup_protected_reference_files(
            testdata_dir / "synthetic_ac07_manifest.json",
            testdata_dir / "synthetic_ac07_reference_bundle.json",
            testdata_dir / "synthetic_ac07_protected_annotations.json",
        )
    except CleanupProtectedReferenceBindingError as error:
        parser.exit(1, f"error: {error}\n")

    _print_demo(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
