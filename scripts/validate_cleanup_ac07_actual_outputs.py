#!/usr/bin/env python3
"""Bind separately supplied AC-07 outputs to verified reference identities.

This standard-library validator checks output identity, source declarations,
case correspondence, and UTF-8 encodability only. It does not run cleanup,
compare actual values with expected values, score protected tokens, or
authenticate production execution.
"""

from __future__ import annotations

import argparse
import json
from collections.abc import Mapping
from dataclasses import dataclass, field
from pathlib import Path
from typing import Sequence

try:  # Support direct script execution and package-based imports.
    from .validate_cleanup_ac07_protected_references import (
        CleanupProtectedReferenceBindingError,
        CleanupProtectedReferenceBindingSummary,
        validate_cleanup_protected_references,
    )
except ImportError:  # pragma: no cover - exercised by direct CLI invocation.
    from validate_cleanup_ac07_protected_references import (
        CleanupProtectedReferenceBindingError,
        CleanupProtectedReferenceBindingSummary,
        validate_cleanup_protected_references,
    )


SCHEMA_VERSION = 1
SOURCE_KINDS = frozenset({"synthetic_demo", "production_declared"})
BUNDLE_FIELDS = frozenset(
    {
        "schema_version",
        "corpus_id",
        "revision",
        "run_id",
        "source_kind",
        "execution_evidence_ref",
        "cases",
    }
)
CASE_FIELDS = frozenset({"id", "actual"})
_KNOWN_FIELDS = BUNDLE_FIELDS | CASE_FIELDS


class CleanupActualOutputBindingError(ValueError):
    """A separately supplied AC-07 output bundle failed identity validation."""


@dataclass(frozen=True)
class CleanupActualOutputBindingSummary:
    """Immutable, content-free result of complete actual-output binding.

    The source kind and evidence reference are declarations only. In
    particular, ``production_execution_independently_verified`` is always
    false: this validator cannot authenticate that production code ran.
    """

    corpus_id: str
    revision: str
    run_id: str
    declared_source_kind: str
    reference_case_count: int
    actual_output_case_count: int
    matched_case_id_count: int
    valid_utf8_actual_string_count: int
    full_case_correspondence_verified: bool
    production_execution_independently_verified: bool = field(
        default=False, init=False
    )


class _JSONObject(dict[str, object]):
    """Parsed JSON object retaining duplicate-key evidence for rejection."""

    def __init__(self, pairs: list[tuple[str, object]]) -> None:
        super().__init__()
        self.duplicate_keys: list[str] = []
        for key, value in pairs:
            if key in self:
                self.duplicate_keys.append(key)
            self[key] = value


def _strict_fields(
    value: Mapping[object, object],
    expected: frozenset[str],
    context: str,
) -> None:
    duplicate_keys = getattr(value, "duplicate_keys", ())
    if duplicate_keys:
        key = duplicate_keys[0]
        if isinstance(key, str) and key in _KNOWN_FIELDS:
            raise CleanupActualOutputBindingError(
                f"{context}: duplicate JSON field '{key}'"
            )
        raise CleanupActualOutputBindingError(
            f"{context}: duplicate JSON object field"
        )

    if any(not isinstance(key, str) for key in value):
        raise CleanupActualOutputBindingError(
            f"{context}: object contains an unsupported field"
        )

    present = set(value)
    missing = expected - present
    if missing:
        field_name = sorted(missing)[0]
        raise CleanupActualOutputBindingError(
            f"{context}: missing required field '{field_name}'"
        )
    if present - expected:
        raise CleanupActualOutputBindingError(
            f"{context}: object contains an unsupported field"
        )


def _nonempty_utf8_string(value: object, field_name: str, context: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise CleanupActualOutputBindingError(
            f"{context}: field '{field_name}' must be a nonempty string"
        )
    try:
        value.encode("utf-8")
    except UnicodeEncodeError as error:
        raise CleanupActualOutputBindingError(
            f"{context}: field '{field_name}' must be valid UTF-8"
        ) from error
    return value


def _invalid_json_constant(_value: str) -> None:
    """Reject Python's non-standard JSON NaN and Infinity extensions."""
    raise ValueError


def _reject_duplicate_json_keys(value: object, context: str) -> None:
    """Reject duplicate keys anywhere without echoing untrusted key content."""
    if isinstance(value, _JSONObject):
        if value.duplicate_keys:
            key = value.duplicate_keys[0]
            if isinstance(key, str) and key in _KNOWN_FIELDS:
                raise CleanupActualOutputBindingError(
                    f"{context}: duplicate JSON field '{key}'"
                )
            raise CleanupActualOutputBindingError(
                f"{context}: duplicate JSON object field"
            )
        for key, child in value.items():
            if key == "cases" and isinstance(child, list):
                for case_number, case in enumerate(child, start=1):
                    _reject_duplicate_json_keys(
                        case, f"{context} case {case_number}"
                    )
            else:
                _reject_duplicate_json_keys(child, context)
    elif isinstance(value, list):
        for child in value:
            _reject_duplicate_json_keys(child, context)


def _load_json(path: str | Path, document: str) -> object:
    """Load strict UTF-8 JSON and reject duplicate keys and extensions."""
    try:
        raw_text = Path(path).read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise CleanupActualOutputBindingError(
            f"{document} file could not be read as UTF-8 JSON"
        ) from error

    try:
        value = json.loads(
            raw_text,
            object_pairs_hook=_JSONObject,
            parse_constant=_invalid_json_constant,
        )
    except json.JSONDecodeError as error:
        raise CleanupActualOutputBindingError(
            f"{document} JSON is invalid at line {error.lineno}, "
            f"column {error.colno}"
        ) from error
    except ValueError as error:
        raise CleanupActualOutputBindingError(
            f"{document} JSON contains an unsupported value"
        ) from error

    _reject_duplicate_json_keys(value, document)
    return value


def _validate_references_first(
    manifest: object,
    reference_bundle: object,
    annotation_bundle: object,
) -> CleanupProtectedReferenceBindingSummary:
    """Establish the existing manifest/reference/annotation binding first."""
    try:
        return validate_cleanup_protected_references(
            manifest, reference_bundle, annotation_bundle
        )
    except CleanupProtectedReferenceBindingError as error:
        raise CleanupActualOutputBindingError(
            f"reference and protected-annotation validation failed: {error}"
        ) from error


def _validate_actual_outputs_after_reference_binding(
    manifest: object,
    reference_bundle: object,
    actual_bundle: object,
    reference_summary: CleanupProtectedReferenceBindingSummary,
) -> CleanupActualOutputBindingSummary:
    """Validate outputs after the reference and annotation binding succeeded."""
    if not isinstance(actual_bundle, Mapping):
        raise CleanupActualOutputBindingError(
            "actual-output bundle must be a JSON object"
        )
    _strict_fields(actual_bundle, BUNDLE_FIELDS, "actual-output bundle")

    schema_version = actual_bundle["schema_version"]
    if type(schema_version) is not int or schema_version != SCHEMA_VERSION:
        raise CleanupActualOutputBindingError(
            "actual-output bundle: field 'schema_version' must equal integer 1"
        )

    corpus_id = _nonempty_utf8_string(
        actual_bundle["corpus_id"], "corpus_id", "actual-output bundle"
    )
    revision = _nonempty_utf8_string(
        actual_bundle["revision"], "revision", "actual-output bundle"
    )
    run_id = _nonempty_utf8_string(
        actual_bundle["run_id"], "run_id", "actual-output bundle"
    )
    if corpus_id != reference_summary.corpus_id:
        raise CleanupActualOutputBindingError(
            "actual-output bundle: field 'corpus_id' does not match verified references"
        )
    if revision != reference_summary.revision:
        raise CleanupActualOutputBindingError(
            "actual-output bundle: field 'revision' does not match verified references"
        )

    source_kind = actual_bundle["source_kind"]
    if not isinstance(source_kind, str) or source_kind not in SOURCE_KINDS:
        raise CleanupActualOutputBindingError(
            "actual-output bundle: field 'source_kind' is unsupported"
        )

    evidence_ref = actual_bundle["execution_evidence_ref"]
    if source_kind == "synthetic_demo":
        if evidence_ref is not None:
            raise CleanupActualOutputBindingError(
                "actual-output bundle: field 'execution_evidence_ref' must be null "
                "for synthetic_demo"
            )
    else:
        _nonempty_utf8_string(
            evidence_ref, "execution_evidence_ref", "actual-output bundle"
        )
        # A reference string is a declaration; it does not authenticate a run.

    purpose = manifest["purpose"]  # type: ignore[index]
    if purpose == "synthetic_demo" and source_kind == "production_declared":
        raise CleanupActualOutputBindingError(
            "actual-output bundle: field 'source_kind' contradicts synthetic_demo manifest purpose"
        )

    actual_cases = actual_bundle["cases"]
    if not isinstance(actual_cases, list) or not actual_cases:
        raise CleanupActualOutputBindingError(
            "actual-output bundle: field 'cases' must be a nonempty array"
        )

    # This collection was fully checked by the existing protected-reference
    # validator before this function is called; preserve its exact case IDs.
    reference_cases = reference_bundle["cases"]  # type: ignore[index]
    reference_by_id = {case["id"]: case for case in reference_cases}
    seen_ids: set[str] = set()
    valid_utf8_count = 0

    for case_number, case in enumerate(actual_cases, start=1):
        context = f"actual-output bundle case {case_number}"
        if not isinstance(case, Mapping):
            raise CleanupActualOutputBindingError(
                f"{context}: record must be an object"
            )
        _strict_fields(case, CASE_FIELDS, context)

        case_id = _nonempty_utf8_string(case["id"], "id", context)
        if case_id in seen_ids:
            raise CleanupActualOutputBindingError(
                f"{context}: field 'id' duplicates an earlier case"
            )
        seen_ids.add(case_id)
        if case_id not in reference_by_id:
            raise CleanupActualOutputBindingError(
                f"{context}: field 'id' is not declared in verified references"
            )

        actual_text = case["actual"]
        if not isinstance(actual_text, str):
            raise CleanupActualOutputBindingError(
                f"{context}: field 'actual' must be a string"
            )
        try:
            actual_text.encode("utf-8")
        except UnicodeEncodeError as error:
            raise CleanupActualOutputBindingError(
                f"{context}: field 'actual' must be valid UTF-8"
            ) from error
        valid_utf8_count += 1

    for reference_case_number, reference_case in enumerate(
        reference_cases, start=1
    ):
        if reference_case["id"] not in seen_ids:
            raise CleanupActualOutputBindingError(
                f"reference case {reference_case_number}: field 'id' has no "
                "matching actual-output bundle case"
            )

    return CleanupActualOutputBindingSummary(
        corpus_id=reference_summary.corpus_id,
        revision=reference_summary.revision,
        run_id=run_id,
        declared_source_kind=source_kind,
        reference_case_count=reference_summary.reference_case_count,
        actual_output_case_count=len(actual_cases),
        matched_case_id_count=len(seen_ids),
        valid_utf8_actual_string_count=valid_utf8_count,
        full_case_correspondence_verified=True,
    )


def validate_cleanup_actual_outputs(
    manifest: object,
    reference_bundle: object,
    annotation_bundle: object,
    actual_bundle: object,
) -> CleanupActualOutputBindingSummary:
    """Bind separately supplied outputs to already verified AC-07 references.

    The established protected-reference validator runs before ``actual_bundle``
    is inspected. Cases are associated by exact, case-sensitive ID. Actual
    text is checked only for string type and UTF-8 encodability; it is neither
    normalized nor compared with expected text or checked for token changes.
    A ``production_declared`` source and its evidence-reference string remain
    unauthenticated declarations.

    Raises:
        CleanupActualOutputBindingError: If reference binding or any actual
            output identity, schema, declaration, case, or UTF-8 check fails.
    """
    reference_summary = _validate_references_first(
        manifest, reference_bundle, annotation_bundle
    )
    return _validate_actual_outputs_after_reference_binding(
        manifest, reference_bundle, actual_bundle, reference_summary
    )


def _print_demo(summary: CleanupActualOutputBindingSummary) -> None:
    """Print only identity and counts after every validation succeeds."""
    print("AC-07 actual-output binding: SYNTHETIC DEMO")
    print(f"Corpus ID: {summary.corpus_id}")
    print(f"Revision: {summary.revision}")
    print(f"Run ID: {summary.run_id}")
    print(f"Reference cases: {summary.reference_case_count}")
    print(f"Actual-output cases: {summary.actual_output_case_count}")
    print(
        "Case-ID correspondence: "
        f"{'PASS' if summary.full_case_correspondence_verified else 'FAIL'}"
    )
    print(
        "Actual UTF-8 strings validated: "
        f"{summary.valid_utf8_actual_string_count}/"
        f"{summary.actual_output_case_count}"
    )
    print(f"Output source: {summary.declared_source_kind.replace('_', ' ').upper()}")
    print("Production execution: NOT VERIFIED")
    print("Reference review: PENDING")
    print("Corpus approval: NOT ESTABLISHED")
    print("AC-07 acceptance: NOT EVALUATED")


def main(argv: Sequence[str] | None = None) -> int:
    """Validate the four checked-in synthetic artifacts and print a summary."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--demo",
        action="store_true",
        help="validate only the four checked-in synthetic AC-07 artifacts",
    )
    args = parser.parse_args(argv)
    if not args.demo:
        parser.error("--demo is required")

    testdata_dir = Path(__file__).resolve().parent / "testdata"
    try:
        manifest = _load_json(
            testdata_dir / "synthetic_ac07_manifest.json", "manifest"
        )
        reference_bundle = _load_json(
            testdata_dir / "synthetic_ac07_reference_bundle.json",
            "reference bundle",
        )
        annotation_bundle = _load_json(
            testdata_dir / "synthetic_ac07_protected_annotations.json",
            "annotation bundle",
        )
        # Reject invalid reference metadata before opening the separately
        # supplied actual-output artifact.
        reference_summary = _validate_references_first(
            manifest, reference_bundle, annotation_bundle
        )
        actual_bundle = _load_json(
            testdata_dir / "synthetic_ac07_actual_outputs.json",
            "actual-output bundle",
        )
        summary = _validate_actual_outputs_after_reference_binding(
            manifest, reference_bundle, actual_bundle, reference_summary
        )
    except CleanupActualOutputBindingError as error:
        parser.exit(1, f"error: {error}\n")

    _print_demo(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
