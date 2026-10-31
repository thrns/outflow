#!/usr/bin/env python3
"""Validate synthetic ASR result counts without asserting model quality.

Schema v1 intentionally accepts only synthetic numerical results. Passing
validation checks arithmetic consistency; it does not prove that a model ran
or that the error counts came from genuine word alignments.
"""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Sequence


SCHEMA_VERSION = 1
RESULT_TYPE = "synthetic"
SAMPLE_ID_PATTERN = re.compile(r"synthetic-[0-9]+\Z")
# Declared WER is a JSON float, so compare it with a small absolute tolerance.
WER_ABSOLUTE_TOLERANCE = 1e-9
ROOT_FIELDS = {"schema_version", "result_type", "samples", "totals"}
SAMPLE_FIELDS = {
    "id",
    "reference_words",
    "substitutions",
    "deletions",
    "insertions",
}
SAMPLE_COUNT_FIELDS = (
    "reference_words",
    "substitutions",
    "deletions",
    "insertions",
)
TOTAL_FIELDS = {
    "utterances",
    "reference_words",
    "substitutions",
    "deletions",
    "insertions",
    "wer",
}
TOTAL_COUNT_FIELDS = (
    "utterances",
    "reference_words",
    "substitutions",
    "deletions",
    "insertions",
)


class ResultsValidationError(ValueError):
    """A results document failed safe, content-free validation."""

    def __init__(self, errors: Sequence[str]) -> None:
        self.errors = tuple(errors)
        super().__init__("; ".join(self.errors))


@dataclass(frozen=True)
class ResultsSummary:
    """Recalculated synthetic result totals suitable for a concise summary."""

    samples: int
    reference_words: int
    substitutions: int
    deletions: int
    insertions: int
    wer: float


class _DuplicateJsonKeyError(ValueError):
    """Signals duplicate JSON object keys without exposing their contents."""


def _unique_object(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    """Build a JSON object while rejecting repeated keys at every level."""
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise _DuplicateJsonKeyError
        result[key] = value
    return result


def _reject_nonstandard_number(_: str) -> None:
    """Reject JSON extensions such as NaN and Infinity."""
    raise ValueError


def load_results(path: Path) -> Any:
    """Read one UTF-8 JSON file and return its decoded value.

    Errors are intentionally generic: malformed files may contain private
    material, so parser messages and source values are never printed.
    """
    try:
        contents = path.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        raise ResultsValidationError(("file could not be read as UTF-8",)) from error

    try:
        return json.loads(
            contents,
            object_pairs_hook=_unique_object,
            parse_constant=_reject_nonstandard_number,
        )
    except _DuplicateJsonKeyError as error:
        raise ResultsValidationError(("JSON contains duplicate object keys",)) from error
    except (json.JSONDecodeError, ValueError, RecursionError) as error:
        raise ResultsValidationError(("file does not contain valid JSON",)) from error


def _check_fields(
    value: dict[str, Any], required_fields: set[str], label: str, errors: list[str]
) -> None:
    missing = required_fields - value.keys()
    if missing:
        errors.append(f"{label} is missing required field(s)")
    if value.keys() - required_fields:
        # Do not echo unknown keys: a rejected key could itself contain private data.
        errors.append(f"{label} contains unsupported field(s)")


def _nonnegative_integer(value: Any) -> bool:
    """Return true only for non-boolean, nonnegative JSON integer values."""
    return type(value) is int and value >= 0


def _declared_wer(value: Any) -> float | None:
    """Return a finite, nonnegative numeric WER or ``None`` when invalid."""
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return None
    try:
        numeric_value = float(value)
    except OverflowError:
        return None
    if not math.isfinite(numeric_value) or numeric_value < 0:
        return None
    return numeric_value


def validate_results(data: Any) -> ResultsSummary:
    """Validate schema-v1 synthetic counts and return recalculated totals.

    WER is the sum of substitutions, deletions, and insertions divided by the
    sum of reference words. Its value is not capped at 100%, since insertions
    can make mathematically valid WER exceed 1.0.

    Raises:
        ResultsValidationError: If the shape, counts, aggregates, or WER is invalid.
    """
    errors: list[str] = []
    if not isinstance(data, dict):
        raise ResultsValidationError(("root must be a JSON object",))

    _check_fields(data, ROOT_FIELDS, "root", errors)
    schema_version = data.get("schema_version")
    if type(schema_version) is not int or schema_version != SCHEMA_VERSION:
        errors.append("schema_version must be 1")
    if data.get("result_type") != RESULT_TYPE:
        errors.append("result_type must be 'synthetic'; measured results are not supported")

    samples = data.get("samples")
    sample_count_valid = isinstance(samples, list) and bool(samples)
    if not isinstance(samples, list):
        errors.append("samples must be a list")
    elif not samples:
        errors.append("samples must be a non-empty list")

    totals = data.get("totals")
    if not isinstance(totals, dict):
        errors.append("totals must be an object")
        totals = {}
    _check_fields(totals, TOTAL_FIELDS, "totals", errors)

    calculated = {field: 0 for field in SAMPLE_COUNT_FIELDS}
    seen_ids: set[str] = set()
    all_sample_counts_valid = sample_count_valid
    if isinstance(samples, list):
        for index, sample in enumerate(samples):
            label = f"samples[{index}]"
            if not isinstance(sample, dict):
                errors.append(f"{label} must be an object")
                all_sample_counts_valid = False
                continue

            _check_fields(sample, SAMPLE_FIELDS, label, errors)
            sample_id = sample.get("id")
            if not isinstance(sample_id, str) or not SAMPLE_ID_PATTERN.fullmatch(sample_id):
                errors.append(f"{label}.id must match the synthetic-NNN identifier format")
            elif sample_id in seen_ids:
                errors.append(f"{label}.id duplicates another sample identifier")
            else:
                seen_ids.add(sample_id)

            sample_counts: dict[str, int] = {}
            for field in SAMPLE_COUNT_FIELDS:
                value = sample.get(field)
                if not _nonnegative_integer(value):
                    errors.append(f"{label}.{field} must be a non-negative integer")
                    all_sample_counts_valid = False
                else:
                    sample_counts[field] = value

            reference_words = sample_counts.get("reference_words")
            if reference_words == 0:
                errors.append(f"{label}.reference_words must be at least 1")
                all_sample_counts_valid = False

            substitutions = sample_counts.get("substitutions")
            deletions = sample_counts.get("deletions")
            if (
                reference_words is not None
                and substitutions is not None
                and deletions is not None
                and substitutions + deletions > reference_words
            ):
                errors.append(
                    f"{label} substitutions plus deletions cannot exceed reference_words"
                )
                all_sample_counts_valid = False

            if len(sample_counts) == len(SAMPLE_COUNT_FIELDS):
                for field, value in sample_counts.items():
                    calculated[field] += value

    declared_counts: dict[str, int] = {}
    for field in TOTAL_COUNT_FIELDS:
        value = totals.get(field)
        if not _nonnegative_integer(value):
            errors.append(f"totals.{field} must be a non-negative integer")
        else:
            declared_counts[field] = value

    declared_wer = _declared_wer(totals.get("wer"))
    if declared_wer is None:
        errors.append("totals.wer must be a finite non-negative number")

    if sample_count_valid and "utterances" in declared_counts:
        if declared_counts["utterances"] != len(samples):
            errors.append("totals.utterances does not match the number of samples")

    if all_sample_counts_valid:
        expected_by_total = {
            "reference_words": calculated["reference_words"],
            "substitutions": calculated["substitutions"],
            "deletions": calculated["deletions"],
            "insertions": calculated["insertions"],
        }
        for field, expected in expected_by_total.items():
            if field in declared_counts and declared_counts[field] != expected:
                errors.append(f"totals.{field} does not match the sample sum")

        total_errors = (
            calculated["substitutions"]
            + calculated["deletions"]
            + calculated["insertions"]
        )
        calculated_wer = total_errors / calculated["reference_words"]
        if declared_wer is not None and not math.isclose(
            declared_wer,
            calculated_wer,
            rel_tol=0.0,
            abs_tol=WER_ABSOLUTE_TOLERANCE,
        ):
            errors.append("totals.wer does not match the recalculated corpus WER")
    else:
        calculated_wer = 0.0

    if errors:
        raise ResultsValidationError(errors)

    return ResultsSummary(
        samples=len(samples),
        reference_words=calculated["reference_words"],
        substitutions=calculated["substitutions"],
        deletions=calculated["deletions"],
        insertions=calculated["insertions"],
        wer=calculated_wer,
    )


def format_summary(summary: ResultsSummary) -> str:
    """Format a deterministic, explicitly synthetic validation summary."""
    return "\n".join(
        (
            "Outflow ASR results validation passed",
            "Result type: SYNTHETIC",
            f"Samples validated: {summary.samples}",
            f"Reference words: {summary.reference_words}",
            f"Substitutions: {summary.substitutions}",
            f"Deletions: {summary.deletions}",
            f"Insertions: {summary.insertions}",
            f"Corpus WER: {summary.wer:.2%}",
            "Real ASR benchmark status: NOT ASSESSED",
        )
    )


def main(argv: Sequence[str] | None = None) -> int:
    """Read and validate one synthetic ASR results JSON file."""
    parser = argparse.ArgumentParser(
        description="Validate arithmetic consistency in synthetic ASR results"
    )
    parser.add_argument("--file", required=True, type=Path, help="results JSON file to validate")
    arguments = parser.parse_args(argv)

    try:
        summary = validate_results(load_results(arguments.file))
    except ResultsValidationError as error:
        print("Outflow ASR results validation failed", file=sys.stderr)
        for message in error.errors:
            print(f"Error: {message}", file=sys.stderr)
        return 1

    print(format_summary(summary))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
