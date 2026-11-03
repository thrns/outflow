#!/usr/bin/env python3
"""Calculate lexical WER for two in-memory transcriptions.

The frozen ``lexical-wer-unicode-boundary-v1`` policy uses NFC, lowercase,
collapsed whitespace, and contiguous Unicode letters, numbers, and combining
marks as tokens; all other characters, including punctuation, are boundaries.
It does not expand contractions, convert number spellings, or correct vocabulary.

This development utility only scores strings supplied by its caller. It does
not load audio or datasets, persist results, or calculate punctuation accuracy.
"""

from __future__ import annotations

import argparse
import unicodedata
from dataclasses import dataclass
from typing import Sequence


NORMALIZATION_POLICY_VERSION = "lexical-wer-unicode-boundary-v1"


@dataclass(frozen=True)
class WerResult:
    """Word-level edit counts and WER for a normalized reference.

    ``wer`` is a fraction (for example, ``0.25`` means 25%). It is ``None``
    when the normalized reference has no words, because WER has no denominator.
    """

    substitutions: int
    deletions: int
    insertions: int
    reference_word_count: int
    edit_distance: int
    wer: float | None


def _tokenize(text: str, *, label: str) -> list[str]:
    """Normalize text and split it into provisional Unicode lexical tokens."""
    if not isinstance(text, str):
        raise TypeError(f"{label} must be a string")

    normalized = unicodedata.normalize("NFC", text).lower()
    normalized = " ".join(normalized.split())

    # The fixture policy says punctuation is excluded but does not define
    # whether it separates adjacent text. Treating it as a boundary is an
    # explicit provisional choice; literal token contents (including digits)
    # are otherwise kept unchanged.
    tokens: list[str] = []
    current: list[str] = []
    for character in normalized:
        category = unicodedata.category(character)
        if category[0] in {"L", "N", "M"}:
            current.append(character)
        elif current:
            tokens.append("".join(current))
            current.clear()
    if current:
        tokens.append("".join(current))
    return tokens


def calculate_wer(reference: str, prediction: str) -> WerResult:
    """Return word-level substitutions, deletions, insertions, and WER.

    Both strings use the fixture policy's NFC/lowercase/whitespace steps and
    the frozen Unicode token boundary rule documented above. The
    dynamic-programming alignment resolves equal-cost paths in this order:
    diagonal (match/substitution), deletion, then insertion.
    """
    reference_words = _tokenize(reference, label="reference")
    prediction_words = _tokenize(prediction, label="prediction")
    reference_count = len(reference_words)
    prediction_count = len(prediction_words)

    # Rows consume reference words; columns consume prediction words.
    distances = [
        [0] * (prediction_count + 1) for _ in range(reference_count + 1)
    ]
    for row in range(reference_count + 1):
        distances[row][0] = row
    for column in range(prediction_count + 1):
        distances[0][column] = column

    for row in range(1, reference_count + 1):
        for column in range(1, prediction_count + 1):
            substitution_cost = int(reference_words[row - 1] != prediction_words[column - 1])
            distances[row][column] = min(
                distances[row - 1][column - 1] + substitution_cost,
                distances[row - 1][column] + 1,
                distances[row][column - 1] + 1,
            )

    substitutions = deletions = insertions = 0
    row = reference_count
    column = prediction_count
    while row or column:
        # Prefer a diagonal step when tied. This keeps substitution counts
        # deterministic rather than reporting an equivalent delete+insert pair.
        if row and column:
            substitution_cost = int(reference_words[row - 1] != prediction_words[column - 1])
            if distances[row][column] == distances[row - 1][column - 1] + substitution_cost:
                substitutions += substitution_cost
                row -= 1
                column -= 1
                continue

        if row and distances[row][column] == distances[row - 1][column] + 1:
            deletions += 1
            row -= 1
            continue

        if column and distances[row][column] == distances[row][column - 1] + 1:
            insertions += 1
            column -= 1
            continue

        # Every DP cell has a predecessor. Reaching this branch would mean
        # the alignment and the cost table disagree due to an implementation bug.
        raise RuntimeError("could not reconstruct the minimum edit alignment")

    edit_distance = distances[reference_count][prediction_count]
    wer = edit_distance / reference_count if reference_count else None
    return WerResult(
        substitutions=substitutions,
        deletions=deletions,
        insertions=insertions,
        reference_word_count=reference_count,
        edit_distance=edit_distance,
        wer=wer,
    )


def _run_demo() -> None:
    """Print the self-contained synthetic deletion example."""
    result = calculate_wer("the quick brown fox", "the quick fox")
    print("Outflow WER calculator ready")
    print(f"Reference words: {result.reference_word_count}")
    print(f"Substitutions: {result.substitutions}")
    print(f"Deletions: {result.deletions}")
    print(f"Insertions: {result.insertions}")
    if result.wer is None:
        print("Word Error Rate: undefined (empty normalized reference)")
    else:
        print(f"Word Error Rate: {result.wer:.2%}")


def main(argv: Sequence[str] | None = None) -> int:
    """Run the requested synthetic demonstration mode."""
    parser = argparse.ArgumentParser(description="Outflow's development-only WER calculator")
    parser.add_argument("--demo", action="store_true", help="score a built-in synthetic example")
    arguments = parser.parse_args(argv)
    if not arguments.demo:
        parser.error("choose --demo; transcription scoring is also available through calculate_wer()")
    _run_demo()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
