#!/usr/bin/env python3
"""Validate metadata-only manifests for independently reviewed AC-07 corpora.

This utility checks schema and hash syntax only. It does not load case data,
run cleanup, authenticate review evidence, or establish AC-07 acceptance.
"""

from __future__ import annotations

import argparse
import json
import re
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence


SCHEMA_VERSION = 1
CATEGORIES = frozenset(
    {
        "whitespace",
        "commands",
        "vocabulary",
        "punctuation",
        "fillers",
        "modes",
        "unicode",
        "safety",
    }
)
MODES = frozenset({"clean", "verbatim"})
PURPOSES = frozenset({"synthetic_demo", "acceptance_candidate"})
REVIEW_STATUSES = frozenset({"pending", "reviewed"})
PROTECTED_KINDS = frozenset({"number", "proper_noun"})
HASH_PATTERN = re.compile(r"[0-9a-f]{64}\Z")

MANIFEST_FIELDS = frozenset(
    {
        "schema_version",
        "corpus_id",
        "revision",
        "purpose",
        "review_status",
        "review_evidence_ref",
        "cases",
    }
)
CASE_FIELDS = frozenset(
    {
        "id",
        "category",
        "mode",
        "remove_fillers",
        "commands_enabled",
        "input_sha256",
        "expected_sha256",
        "provenance_ref",
        "reference_review_status",
        "protected_kinds",
    }
)


class CleanupManifestValidationError(ValueError):
    """A manifest failed structural, semantic, or JSON parsing validation."""


@dataclass(frozen=True)
class CleanupManifestSummary:
    """Immutable, content-free counts for one syntactically valid manifest."""

    schema_version: int
    corpus_id: str
    revision: str
    purpose: str
    review_status: str
    indexed_case_count: int
    unique_case_count: int
    represented_category_count: int
    clean_case_count: int
    verbatim_case_count: int
    numeric_protection_case_count: int
    proper_noun_protection_case_count: int
    pending_reference_review_case_count: int
    syntactically_valid: bool


class _JSONObject(dict[str, object]):
    """Parsed JSON object retaining duplicate keys for contextual validation."""

    def __init__(self, pairs: list[tuple[str, object]]) -> None:
        super().__init__()
        duplicates: list[str] = []
        for key, value in pairs:
            if key in self:
                duplicates.append(key)
            self[key] = value
        self.duplicate_keys = tuple(duplicates)


def _object_context(value: Mapping[object, object], context: str) -> None:
    duplicate_keys = getattr(value, "duplicate_keys", ())
    if duplicate_keys:
        key = duplicate_keys[0]
        if isinstance(key, str) and key in MANIFEST_FIELDS | CASE_FIELDS:
            raise CleanupManifestValidationError(
                f"{context}: duplicate JSON field '{key}'"
            )
        raise CleanupManifestValidationError(
            f"{context}: duplicate JSON object field"
        )

    if any(not isinstance(key, str) for key in value):
        raise CleanupManifestValidationError(
            f"{context}: object contains an unsupported field"
        )


def _strict_fields(
    value: Mapping[object, object],
    expected: frozenset[str],
    context: str,
) -> None:
    _object_context(value, context)
    present = set(value)
    missing = expected - present
    if missing:
        field = sorted(missing)[0]
        raise CleanupManifestValidationError(
            f"{context}: missing required field '{field}'"
        )
    if present - expected:
        raise CleanupManifestValidationError(
            f"{context}: object contains an unsupported field"
        )


def _nonempty_string(
    value: object, field: str, context: str
) -> str:
    if not isinstance(value, str) or not value.strip():
        raise CleanupManifestValidationError(
            f"{context}: field '{field}' must be a nonempty string"
        )
    return value


def validate_cleanup_manifest(data: object) -> CleanupManifestSummary:
    """Validate manifest metadata and return immutable counts.

    Hash values are checked for lowercase SHA-256 syntax only. Verifying that
    they bind actual input and expected-output bytes requires a separate case
    data bundle, which this metadata-only utility intentionally does not read.

    Raises:
        CleanupManifestValidationError: If fields, values, review declarations,
            or case metadata are malformed. Messages identify fields or case
            positions and never include case content.
    """
    if not isinstance(data, Mapping):
        raise CleanupManifestValidationError("manifest must be a JSON object")
    _strict_fields(data, MANIFEST_FIELDS, "manifest")

    version = data["schema_version"]
    if type(version) is not int or version != SCHEMA_VERSION:
        raise CleanupManifestValidationError(
            "manifest: field 'schema_version' must equal integer 1"
        )

    corpus_id = _nonempty_string(data["corpus_id"], "corpus_id", "manifest")
    revision = _nonempty_string(data["revision"], "revision", "manifest")

    purpose = data["purpose"]
    if not isinstance(purpose, str) or purpose not in PURPOSES:
        raise CleanupManifestValidationError(
            "manifest: field 'purpose' must be 'synthetic_demo' or "
            "'acceptance_candidate'"
        )

    review_status = data["review_status"]
    if not isinstance(review_status, str) or review_status not in REVIEW_STATUSES:
        raise CleanupManifestValidationError(
            "manifest: field 'review_status' must be 'pending' or 'reviewed'"
        )

    review_evidence_ref = data["review_evidence_ref"]
    if review_status == "pending":
        if review_evidence_ref is not None:
            raise CleanupManifestValidationError(
                "manifest: field 'review_evidence_ref' must be null when "
                "review_status is pending"
            )
    else:
        _nonempty_string(
            review_evidence_ref, "review_evidence_ref", "manifest"
        )

    if purpose == "synthetic_demo" and review_status != "pending":
        raise CleanupManifestValidationError(
            "manifest: synthetic_demo purpose requires pending review_status"
        )

    cases = data["cases"]
    if not isinstance(cases, list) or not cases:
        raise CleanupManifestValidationError(
            "manifest: field 'cases' must be a nonempty array"
        )

    seen_ids: set[str] = set()
    categories: set[str] = set()
    clean_cases = 0
    verbatim_cases = 0
    numeric_protection_cases = 0
    proper_noun_protection_cases = 0
    pending_reference_reviews = 0

    for case_number, case in enumerate(cases, start=1):
        context = f"case {case_number}"
        if not isinstance(case, Mapping):
            raise CleanupManifestValidationError(
                f"{context}: record must be an object"
            )
        _strict_fields(case, CASE_FIELDS, context)

        case_id = _nonempty_string(case["id"], "id", context)
        if case_id in seen_ids:
            raise CleanupManifestValidationError(
                f"{context}: field 'id' duplicates an earlier case"
            )

        category = case["category"]
        if not isinstance(category, str) or category not in CATEGORIES:
            raise CleanupManifestValidationError(
                f"{context}: field 'category' is unsupported"
            )

        mode = case["mode"]
        if not isinstance(mode, str) or mode not in MODES:
            raise CleanupManifestValidationError(
                f"{context}: field 'mode' must be 'clean' or 'verbatim'"
            )

        for field in ("remove_fillers", "commands_enabled"):
            if type(case[field]) is not bool:
                raise CleanupManifestValidationError(
                    f"{context}: field '{field}' must be a boolean"
                )

        for field in ("input_sha256", "expected_sha256"):
            digest = case[field]
            if not isinstance(digest, str) or HASH_PATTERN.fullmatch(digest) is None:
                raise CleanupManifestValidationError(
                    f"{context}: field '{field}' must be 64 lowercase hex characters"
                )

        _nonempty_string(case["provenance_ref"], "provenance_ref", context)

        reference_status = case["reference_review_status"]
        if (
            not isinstance(reference_status, str)
            or reference_status not in REVIEW_STATUSES
        ):
            raise CleanupManifestValidationError(
                f"{context}: field 'reference_review_status' must be "
                "'pending' or 'reviewed'"
            )
        if purpose == "synthetic_demo" and reference_status != "pending":
            raise CleanupManifestValidationError(
                f"{context}: synthetic_demo cases cannot claim reviewed references"
            )

        protected_kinds = case["protected_kinds"]
        if not isinstance(protected_kinds, list):
            raise CleanupManifestValidationError(
                f"{context}: field 'protected_kinds' must be an array"
            )
        seen_kinds: set[str] = set()
        for protected_kind in protected_kinds:
            if (
                not isinstance(protected_kind, str)
                or protected_kind not in PROTECTED_KINDS
            ):
                raise CleanupManifestValidationError(
                    f"{context}: field 'protected_kinds' contains an unsupported value"
                )
            if protected_kind in seen_kinds:
                raise CleanupManifestValidationError(
                    f"{context}: field 'protected_kinds' contains a duplicate value"
                )
            seen_kinds.add(protected_kind)

        seen_ids.add(case_id)
        categories.add(category)
        if mode == "clean":
            clean_cases += 1
        else:
            verbatim_cases += 1
        if "number" in seen_kinds:
            numeric_protection_cases += 1
        if "proper_noun" in seen_kinds:
            proper_noun_protection_cases += 1
        if reference_status == "pending":
            pending_reference_reviews += 1

    case_count = len(cases)
    return CleanupManifestSummary(
        schema_version=SCHEMA_VERSION,
        corpus_id=corpus_id,
        revision=revision,
        purpose=purpose,
        review_status=review_status,
        indexed_case_count=case_count,
        unique_case_count=len(seen_ids),
        represented_category_count=len(categories),
        clean_case_count=clean_cases,
        verbatim_case_count=verbatim_cases,
        numeric_protection_case_count=numeric_protection_cases,
        proper_noun_protection_case_count=proper_noun_protection_cases,
        pending_reference_review_case_count=pending_reference_reviews,
        syntactically_valid=True,
    )


def load_cleanup_manifest(path: str | Path) -> CleanupManifestSummary:
    """Read JSON with duplicate-key detection, then validate its metadata."""
    try:
        raw_text = Path(path).read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise CleanupManifestValidationError(
            "manifest file could not be read as UTF-8 JSON"
        ) from error

    try:
        data = json.loads(raw_text, object_pairs_hook=_JSONObject)
    except json.JSONDecodeError as error:
        raise CleanupManifestValidationError(
            f"manifest JSON is invalid at line {error.lineno}, "
            f"column {error.colno}"
        ) from error

    return validate_cleanup_manifest(data)


def _print_demo(summary: CleanupManifestSummary) -> None:
    """Print the fixed, content-free summary after successful validation."""
    print("AC-07 corpus manifest: SYNTHETIC DRAFT")
    print(f"Schema version: {summary.schema_version}")
    print(f"Cases indexed: {summary.indexed_case_count}")
    print(
        "Unique case IDs: "
        f"{'PASS' if summary.unique_case_count == summary.indexed_case_count else 'FAIL'}"
    )
    print(f"Categories represented: {summary.represented_category_count}/8")
    print(
        "Clean and Verbatim represented: "
        f"{'PASS' if summary.clean_case_count and summary.verbatim_case_count else 'FAIL'}"
    )
    print("Content hashes: FORMAT VALIDATED ONLY")
    print("Reference review: PENDING")
    print("Corpus approval: NOT ESTABLISHED")
    print("AC-07 acceptance: NOT EVALUATED")


def _print_manifest_summary(summary: CleanupManifestSummary) -> None:
    """Print safe counts for an explicitly supplied metadata file."""
    print("AC-07 corpus manifest: METADATA VALIDATED")
    print(f"Schema version: {summary.schema_version}")
    print(f"Corpus ID: {summary.corpus_id}")
    print(f"Revision: {summary.revision}")
    print(f"Purpose: {summary.purpose}")
    print(f"Declared review status: {summary.review_status}")
    print(f"Cases indexed: {summary.indexed_case_count}")
    print(f"Unique case IDs: {summary.unique_case_count}")
    print(f"Categories represented: {summary.represented_category_count}/8")
    print("Content hashes: FORMAT VALIDATED ONLY")
    print("Corpus approval: NOT ESTABLISHED")
    print("AC-07 acceptance: NOT EVALUATED")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    options = parser.add_mutually_exclusive_group(required=True)
    options.add_argument(
        "--demo",
        action="store_true",
        help="validate only the checked-in synthetic metadata manifest",
    )
    options.add_argument(
        "--manifest",
        type=Path,
        help="validate an explicitly supplied metadata-only manifest",
    )
    arguments = parser.parse_args(argv)

    manifest_path = arguments.manifest
    if arguments.demo:
        manifest_path = (
            Path(__file__).resolve().parent
            / "testdata"
            / "synthetic_ac07_manifest.json"
        )

    try:
        summary = load_cleanup_manifest(manifest_path)
    except CleanupManifestValidationError as error:
        parser.error(str(error))

    if arguments.demo:
        _print_demo(summary)
    else:
        _print_manifest_summary(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
