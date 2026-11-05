#!/usr/bin/env python3
"""Evaluate byte-exact preservation of explicitly annotated AC-07 tokens.

This independent evaluator accepts reviewed input/expected/actual records in
memory. It does not derive annotations, invoke Rust cleanup, or read fixtures.
Passing its synthetic checks is not AC-07 acceptance evidence.
"""

from __future__ import annotations

import argparse
from collections.abc import Iterable, Mapping
from dataclasses import dataclass
from typing import Sequence


CASE_FIELDS = {"id", "input", "expected", "actual", "protected_tokens"}
ANNOTATION_FIELDS = {"kind", "text", "input_bytes", "output_bytes"}
TOKEN_KINDS = {"number", "proper_noun"}


class ProtectedCleanupValidationError(ValueError):
    """Evaluation records or protected-token reference metadata are invalid."""


@dataclass(frozen=True)
class ProtectedCleanupScore:
    """Immutable protected-token counts and evaluation-gate results."""

    total_cases: int
    annotated_cases: int
    protected_number_tokens: int
    altered_number_tokens: int
    protected_proper_noun_tokens: int
    altered_proper_noun_tokens: int
    both_token_kinds_represented: bool
    zero_alterations_observed: bool
    protected_token_gate_passes: bool


def _context(case_number: int, token_number: int | None = None) -> str:
    if token_number is None:
        return f"case {case_number}"
    return f"case {case_number}, token {token_number}"


def _utf8_data(
    value: object, field: str, case_number: int
) -> tuple[bytes, frozenset[int]]:
    if not isinstance(value, str):
        raise ProtectedCleanupValidationError(
            f"{_context(case_number)}: field '{field}' must be a string"
        )
    try:
        encoded = value.encode("utf-8")
    except UnicodeEncodeError as error:
        raise ProtectedCleanupValidationError(
            f"{_context(case_number)}: field '{field}' must be valid UTF-8"
        ) from error

    boundaries = {0}
    offset = 0
    for character in value:
        offset += len(character.encode("utf-8"))
        boundaries.add(offset)
    return encoded, frozenset(boundaries)


def _checked_range(
    value: object,
    field: str,
    encoded_text: bytes,
    boundaries: frozenset[int],
    token_bytes: bytes,
    case_number: int,
    token_number: int,
    kind: str,
) -> tuple[int, int]:
    context = f"{_context(case_number, token_number)} ({kind})"
    if not isinstance(value, (list, tuple)) or len(value) != 2:
        raise ProtectedCleanupValidationError(
            f"{context}: field '{field}' must contain exactly two byte offsets"
        )
    start, end = value
    if type(start) is not int or type(end) is not int or start < 0 or end < 0:
        raise ProtectedCleanupValidationError(
            f"{context}: field '{field}' offsets must be nonnegative integers"
        )
    if start >= end:
        raise ProtectedCleanupValidationError(
            f"{context}: field '{field}' range must be ordered and nonempty"
        )
    if end > len(encoded_text):
        raise ProtectedCleanupValidationError(
            f"{context}: field '{field}' range is out of bounds"
        )
    if start not in boundaries or end not in boundaries:
        raise ProtectedCleanupValidationError(
            f"{context}: field '{field}' range must start and end at UTF-8 "
            "character boundaries"
        )
    if encoded_text[start:end] != token_bytes:
        reference = "input" if field == "input_bytes" else "expected output"
        raise ProtectedCleanupValidationError(
            f"{context}: field '{field}' does not select the declared token "
            f"in {reference}"
        )
    return start, end


def _overlaps(left: tuple[int, int], right: tuple[int, int]) -> bool:
    return left[0] < right[1] and right[0] < left[1]


def _record_fields(record: Mapping[object, object], case_number: int) -> None:
    missing = CASE_FIELDS - record.keys()
    if missing:
        fields = ", ".join(f"'{field}'" for field in sorted(missing))
        raise ProtectedCleanupValidationError(
            f"{_context(case_number)}: missing required record field(s): {fields}"
        )
    if record.keys() - CASE_FIELDS:
        raise ProtectedCleanupValidationError(
            f"{_context(case_number)}: record contains unsupported field(s)"
        )


def _annotation_fields(
    annotation: Mapping[object, object], case_number: int, token_number: int
) -> None:
    kind = annotation.get("kind")
    context = (
        f"{_context(case_number, token_number)} ({kind})"
        if isinstance(kind, str) and kind in TOKEN_KINDS
        else _context(case_number, token_number)
    )
    missing = ANNOTATION_FIELDS - annotation.keys()
    if missing:
        fields = ", ".join(f"'{field}'" for field in sorted(missing))
        raise ProtectedCleanupValidationError(
            f"{context}: missing required annotation field(s): {fields}"
        )
    if annotation.keys() - ANNOTATION_FIELDS:
        raise ProtectedCleanupValidationError(
            f"{context}: annotation contains unsupported field(s)"
        )


def score_protected_cleanup_cases(
    cases: Iterable[Mapping[str, object]],
) -> ProtectedCleanupScore:
    """Validate records and score annotated token preservation at reviewed offsets.

    ``input_bytes`` is validated against ``input`` and ``output_bytes`` against
    ``expected``. The latter same offsets are checked against ``actual`` without
    searching or shifting them. Thus an insertion or deletion before a token
    can conservatively count as an alteration.

    Raises:
        ProtectedCleanupValidationError: If any record or input/reference
            annotation is malformed. Actual-output mismatches are returned as
            alteration counts instead of validation errors.
    """
    if not isinstance(cases, Iterable):
        raise ProtectedCleanupValidationError("cases must be an iterable of records")

    seen_ids: set[str] = set()
    total_cases = 0
    annotated_cases = 0
    protected_numbers = 0
    altered_numbers = 0
    protected_names = 0
    altered_names = 0
    represented_kinds: set[str] = set()

    for case_number, record in enumerate(cases, start=1):
        if not isinstance(record, Mapping):
            raise ProtectedCleanupValidationError(
                f"{_context(case_number)}: record must be a mapping"
            )
        _record_fields(record, case_number)

        case_id = record["id"]
        if not isinstance(case_id, str):
            raise ProtectedCleanupValidationError(
                f"{_context(case_number)}: field 'id' must be a string"
            )
        if not case_id.strip():
            raise ProtectedCleanupValidationError(
                f"{_context(case_number)}: field 'id' must be nonempty"
            )
        if case_id in seen_ids:
            raise ProtectedCleanupValidationError(
                f"{_context(case_number)}: field 'id' duplicates an earlier case"
            )

        input_bytes, input_boundaries = _utf8_data(
            record["input"], "input", case_number
        )
        expected_bytes, expected_boundaries = _utf8_data(
            record["expected"], "expected", case_number
        )
        actual_bytes, actual_boundaries = _utf8_data(
            record["actual"], "actual", case_number
        )
        annotations = record["protected_tokens"]
        if not isinstance(annotations, list):
            raise ProtectedCleanupValidationError(
                f"{_context(case_number)}: field 'protected_tokens' must be a list"
            )

        seen_ids.add(case_id)
        total_cases += 1
        if annotations:
            annotated_cases += 1

        input_ranges: list[tuple[int, int]] = []
        expected_ranges: list[tuple[int, int]] = []
        for token_number, annotation in enumerate(annotations, start=1):
            if not isinstance(annotation, Mapping):
                raise ProtectedCleanupValidationError(
                    f"{_context(case_number, token_number)}: field "
                    "'protected_tokens' entry must be a mapping"
                )
            _annotation_fields(annotation, case_number, token_number)

            kind = annotation["kind"]
            if not isinstance(kind, str) or kind not in TOKEN_KINDS:
                raise ProtectedCleanupValidationError(
                    f"{_context(case_number, token_number)}: field 'kind' "
                    "is unsupported"
                )
            context = f"{_context(case_number, token_number)} ({kind})"
            text = annotation["text"]
            if not isinstance(text, str):
                raise ProtectedCleanupValidationError(
                    f"{context}: field 'text' must be a string"
                )
            if not text:
                raise ProtectedCleanupValidationError(
                    f"{context}: field 'text' must be nonempty"
                )
            try:
                token_bytes = text.encode("utf-8")
            except UnicodeEncodeError as error:
                raise ProtectedCleanupValidationError(
                    f"{context}: field 'text' must be valid UTF-8"
                ) from error

            input_range = _checked_range(
                annotation["input_bytes"],
                "input_bytes",
                input_bytes,
                input_boundaries,
                token_bytes,
                case_number,
                token_number,
                kind,
            )
            output_range = _checked_range(
                annotation["output_bytes"],
                "output_bytes",
                expected_bytes,
                expected_boundaries,
                token_bytes,
                case_number,
                token_number,
                kind,
            )

            if any(_overlaps(input_range, previous) for previous in input_ranges):
                raise ProtectedCleanupValidationError(
                    f"{context}: field 'input_bytes' overlaps another protected "
                    "input range"
                )
            if any(_overlaps(output_range, previous) for previous in expected_ranges):
                raise ProtectedCleanupValidationError(
                    f"{context}: field 'output_bytes' overlaps another protected "
                    "expected-output range"
                )
            input_ranges.append(input_range)
            expected_ranges.append(output_range)
            represented_kinds.add(kind)

            start, end = output_range
            actual_is_boundary_safe = (
                start in actual_boundaries and end in actual_boundaries
            )
            preserved = (
                end <= len(actual_bytes)
                and actual_is_boundary_safe
                and actual_bytes[start:end] == token_bytes
            )
            if kind == "number":
                protected_numbers += 1
                if not preserved:
                    altered_numbers += 1
            else:
                protected_names += 1
                if not preserved:
                    altered_names += 1

    if total_cases == 0:
        raise ProtectedCleanupValidationError("at least one case is required")

    both_kinds = represented_kinds == TOKEN_KINDS
    zero_alterations = altered_numbers == 0 and altered_names == 0
    return ProtectedCleanupScore(
        total_cases=total_cases,
        annotated_cases=annotated_cases,
        protected_number_tokens=protected_numbers,
        altered_number_tokens=altered_numbers,
        protected_proper_noun_tokens=protected_names,
        altered_proper_noun_tokens=altered_names,
        both_token_kinds_represented=both_kinds,
        zero_alterations_observed=zero_alterations,
        protected_token_gate_passes=both_kinds and zero_alterations,
    )


def _demo_cases() -> list[dict[str, object]]:
    """Return two reviewed synthetic examples with manually declared ranges."""
    return [
        {
            "id": "demo-number-punctuation-shift",
            "input": "Hello , 2026",
            "expected": "Hello, 2026",
            "actual": "Hello, 2026",
            "protected_tokens": [
                {
                    "kind": "number",
                    "text": "2026",
                    "input_bytes": [8, 12],
                    "output_bytes": [7, 11],
                }
            ],
        },
        {
            "id": "demo-accented-proper-noun",
            "input": "Meet Élodie",
            "expected": "Meet Élodie!",
            "actual": "Meet Élodie!",
            "protected_tokens": [
                {
                    "kind": "proper_noun",
                    "text": "Élodie",
                    "input_bytes": [5, 12],
                    "output_bytes": [5, 12],
                }
            ],
        },
    ]


def _run_demo() -> None:
    result = score_protected_cleanup_cases(_demo_cases())
    print("AC-07 protected-token scorer: SYNTHETIC DEMO")
    print(f"Cases: {result.total_cases}")
    print(f"Annotated cases: {result.annotated_cases}")
    print(
        "Protected numeric tokens: "
        f"{result.protected_number_tokens} ({result.altered_number_tokens} altered)"
    )
    print(
        "Protected proper nouns: "
        f"{result.protected_proper_noun_tokens} "
        f"({result.altered_proper_noun_tokens} altered)"
    )
    print(
        "Both token kinds represented: "
        f"{'PASS' if result.both_token_kinds_represented else 'FAIL'}"
    )
    gate_status = "PASS" if result.protected_token_gate_passes else "FAIL"
    print(f"Protected-token gate: {gate_status} (synthetic only)")
    print("AC-07 acceptance: NOT EVALUATED")


def main(argv: Sequence[str] | None = None) -> int:
    """Run the synthetic demonstration; real data is supplied through the API."""
    parser = argparse.ArgumentParser(
        description="Score declared protected-token UTF-8 ranges in memory"
    )
    parser.add_argument(
        "--demo",
        action="store_true",
        help="run a two-case synthetic preservation demonstration",
    )
    arguments = parser.parse_args(argv)
    if not arguments.demo:
        parser.error(
            "choose --demo; use score_protected_cleanup_cases() for "
            "in-memory records"
        )
    _run_demo()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
