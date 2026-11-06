#!/usr/bin/env python3
"""Bind a separate AC-07 reference bundle to its metadata manifest.

This development-only validator verifies exact case identity and SHA-256
correspondence. It does not run cleanup, score accuracy, authenticate review,
or establish corpus approval or AC-07 acceptance.
"""

from __future__ import annotations

import argparse
import hashlib
import json
from collections.abc import Mapping
from dataclasses import dataclass
from pathlib import Path
from typing import Sequence

try:  # Support direct script execution and package-based imports.
    from .validate_cleanup_ac07_manifest import (
        CleanupManifestSummary,
        CleanupManifestValidationError,
        validate_cleanup_manifest,
    )
except ImportError:  # pragma: no cover - exercised by direct CLI invocation.
    from validate_cleanup_ac07_manifest import (
        CleanupManifestSummary,
        CleanupManifestValidationError,
        validate_cleanup_manifest,
    )


SCHEMA_VERSION = 1
BUNDLE_FIELDS = frozenset({"schema_version", "corpus_id", "revision", "cases"})
CASE_FIELDS = frozenset({"id", "input", "expected"})


class CleanupReferenceBindingError(ValueError):
    """A manifest and reference bundle failed strict binding validation."""


@dataclass(frozen=True)
class CleanupReferenceBindingSummary:
    """Immutable, content-free counts for a fully verified reference bundle."""

    corpus_id: str
    revision: str
    manifest_case_count: int
    bundle_case_count: int
    matched_case_id_count: int
    verified_input_hash_count: int
    verified_expected_hash_count: int
    all_declared_case_ids_and_hashes_match: bool


class _JSONObject(dict[str, object]):
    """Parsed JSON object that records duplicate keys for strict validation."""

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
            raise CleanupReferenceBindingError(
                f"{context}: duplicate JSON field '{key}'"
            )
        raise CleanupReferenceBindingError(
            f"{context}: duplicate JSON object field"
        )

    if any(not isinstance(key, str) for key in value):
        raise CleanupReferenceBindingError(
            f"{context}: object contains an unsupported field"
        )

    present = set(value)
    missing = expected - present
    if missing:
        field = sorted(missing)[0]
        raise CleanupReferenceBindingError(
            f"{context}: missing required field '{field}'"
        )
    if present - expected:
        raise CleanupReferenceBindingError(
            f"{context}: object contains an unsupported field"
        )


def _nonempty_string(value: object, field: str, context: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise CleanupReferenceBindingError(
            f"{context}: field '{field}' must be a nonempty string"
        )
    try:
        value.encode("utf-8")
    except UnicodeEncodeError as error:
        raise CleanupReferenceBindingError(
            f"{context}: field '{field}' cannot be encoded as UTF-8"
        ) from error
    return value


def _load_json(path: str | Path, document: str) -> object:
    """Load one UTF-8 JSON file while preserving duplicate-key evidence."""
    try:
        raw_text = Path(path).read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise CleanupReferenceBindingError(
            f"{document} file could not be read as UTF-8 JSON"
        ) from error

    try:
        return json.loads(raw_text, object_pairs_hook=_JSONObject)
    except json.JSONDecodeError as error:
        raise CleanupReferenceBindingError(
            f"{document} JSON is invalid at line {error.lineno}, "
            f"column {error.colno}"
        ) from error


def load_cleanup_reference_bundle(
    manifest_path: str | Path,
    bundle_path: str | Path,
) -> CleanupReferenceBindingSummary:
    """Load and bind manifest and bundle files with duplicate-key checks."""
    manifest = _load_json(manifest_path, "manifest")
    _validate_manifest_or_raise(manifest)
    bundle = _load_json(bundle_path, "reference bundle")
    return validate_cleanup_reference_bundle(manifest, bundle)


def _validate_manifest_or_raise(manifest: object) -> CleanupManifestSummary:
    """Run the existing metadata validator and redact its errors at boundary."""
    try:
        return validate_cleanup_manifest(manifest)
    except CleanupManifestValidationError as error:
        raise CleanupReferenceBindingError(
            f"manifest validation failed: {error}"
        ) from error


def validate_cleanup_reference_bundle(
    manifest: object,
    bundle: object,
) -> CleanupReferenceBindingSummary:
    """Validate exact case-ID and UTF-8 SHA-256 correspondence.

    The existing metadata validator runs before the bundle is inspected. Hashes
    are computed from each decoded string's exact UTF-8 encoding; no Unicode,
    punctuation, capitalization, whitespace, or line-ending changes occur.

    Raises:
        CleanupReferenceBindingError: If either object is invalid or any
            identity or digest differs. Messages identify fields and case
            positions without printing case IDs or transcript content.
    """
    manifest_summary = _validate_manifest_or_raise(manifest)

    if not isinstance(bundle, Mapping):
        raise CleanupReferenceBindingError(
            "reference bundle must be a JSON object"
        )
    _strict_fields(bundle, BUNDLE_FIELDS, "reference bundle")

    version = bundle["schema_version"]
    if type(version) is not int or version != SCHEMA_VERSION:
        raise CleanupReferenceBindingError(
            "reference bundle: field 'schema_version' must equal integer 1"
        )

    corpus_id = _nonempty_string(
        bundle["corpus_id"], "corpus_id", "reference bundle"
    )
    revision = _nonempty_string(
        bundle["revision"], "revision", "reference bundle"
    )
    if corpus_id != manifest_summary.corpus_id:
        raise CleanupReferenceBindingError(
            "reference bundle: field 'corpus_id' does not match manifest"
        )
    if revision != manifest_summary.revision:
        raise CleanupReferenceBindingError(
            "reference bundle: field 'revision' does not match manifest"
        )

    cases = bundle["cases"]
    if not isinstance(cases, list) or not cases:
        raise CleanupReferenceBindingError(
            "reference bundle: field 'cases' must be a nonempty array"
        )

    manifest_cases = manifest["cases"]  # Safe after validate_cleanup_manifest.
    manifest_by_id: dict[str, Mapping[object, object]] = {
        case["id"]: case for case in manifest_cases
    }
    seen_ids: set[str] = set()
    verified_cases: list[tuple[int, str, str, str]] = []

    for case_number, case in enumerate(cases, start=1):
        context = f"reference bundle case {case_number}"
        if not isinstance(case, Mapping):
            raise CleanupReferenceBindingError(
                f"{context}: record must be an object"
            )
        _strict_fields(case, CASE_FIELDS, context)

        case_id = _nonempty_string(case["id"], "id", context)
        if case_id in seen_ids:
            raise CleanupReferenceBindingError(
                f"{context}: field 'id' duplicates an earlier case"
            )
        seen_ids.add(case_id)

        digests: dict[str, str] = {}
        for field in ("input", "expected"):
            value = case[field]
            if not isinstance(value, str):
                raise CleanupReferenceBindingError(
                    f"{context}: field '{field}' must be a string"
                )
            try:
                encoded = value.encode("utf-8")
            except UnicodeEncodeError as error:
                raise CleanupReferenceBindingError(
                    f"{context}: field '{field}' cannot be encoded as UTF-8"
                ) from error
            digests[field] = hashlib.sha256(encoded).hexdigest()

        verified_cases.append(
            (case_number, case_id, digests["input"], digests["expected"])
        )

    for bundle_case_number, case_id, _, _ in verified_cases:
        if case_id not in manifest_by_id:
            raise CleanupReferenceBindingError(
                f"reference bundle case {bundle_case_number}: field 'id' is not "
                "declared in manifest"
            )

    bundle_ids = {case_id for _, case_id, _, _ in verified_cases}
    for manifest_case_number, case in enumerate(manifest_cases, start=1):
        if case["id"] not in bundle_ids:
            raise CleanupReferenceBindingError(
                f"manifest case {manifest_case_number}: field 'id' has no "
                "matching reference bundle case"
            )

    for bundle_case_number, case_id, input_hash, expected_hash in verified_cases:
        manifest_case = manifest_by_id[case_id]
        if input_hash != manifest_case["input_sha256"]:
            raise CleanupReferenceBindingError(
                f"reference bundle case {bundle_case_number}: field 'input' "
                "SHA-256 does not match manifest field 'input_sha256'"
            )
        if expected_hash != manifest_case["expected_sha256"]:
            raise CleanupReferenceBindingError(
                f"reference bundle case {bundle_case_number}: field 'expected' "
                "SHA-256 does not match manifest field 'expected_sha256'"
            )

    bundle_count = len(verified_cases)
    return CleanupReferenceBindingSummary(
        corpus_id=manifest_summary.corpus_id,
        revision=manifest_summary.revision,
        manifest_case_count=manifest_summary.indexed_case_count,
        bundle_case_count=bundle_count,
        matched_case_id_count=bundle_count,
        verified_input_hash_count=bundle_count,
        verified_expected_hash_count=bundle_count,
        all_declared_case_ids_and_hashes_match=True,
    )


def _print_demo(summary: CleanupReferenceBindingSummary) -> None:
    """Print only fixed labels and verified counts after complete success."""
    print("AC-07 reference-bundle binding: SYNTHETIC DEMO")
    print(f"Corpus ID: {summary.corpus_id}")
    print(f"Revision: {summary.revision}")
    print(f"Manifest cases: {summary.manifest_case_count}")
    print(f"Bundle cases: {summary.bundle_case_count}")
    print(
        "Case-ID correspondence: "
        f"{'PASS' if summary.all_declared_case_ids_and_hashes_match else 'FAIL'}"
    )
    print(
        "Input SHA-256 hashes verified: "
        f"{summary.verified_input_hash_count}/{summary.manifest_case_count}"
    )
    print(
        "Expected SHA-256 hashes verified: "
        f"{summary.verified_expected_hash_count}/{summary.manifest_case_count}"
    )
    print("Reference review: PENDING")
    print("Corpus approval: NOT ESTABLISHED")
    print("AC-07 acceptance: NOT EVALUATED")


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--demo",
        action="store_true",
        required=True,
        help="bind only the checked-in synthetic manifest and reference bundle",
    )
    arguments = parser.parse_args(argv)

    testdata = Path(__file__).resolve().parent / "testdata"
    try:
        summary = load_cleanup_reference_bundle(
            testdata / "synthetic_ac07_manifest.json",
            testdata / "synthetic_ac07_reference_bundle.json",
        )
    except CleanupReferenceBindingError as error:
        parser.error(str(error))

    if arguments.demo:
        _print_demo(summary)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
