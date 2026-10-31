#!/usr/bin/env python3
"""Aggregate lexical WER for in-memory reference/prediction pairs.

Each pair is scored by :mod:`score_wer`, so corpus scoring uses the same
provisional NFC/lowercase/whitespace/tokenization behavior as individual WER.
Silence and no-speech detection must be evaluated separately: samples with an
empty normalized reference are rejected rather than omitted or assigned 0%.

This development utility uses only caller-supplied strings. It does not read
audio, datasets, model files, or fixture references, and it persists no results.
"""

from __future__ import annotations

import argparse
from dataclasses import dataclass
from typing import Iterable, Sequence

if __package__:
    from .score_wer import calculate_wer
else:  # Running ``python3 scripts/score_corpus_wer.py`` from the repository root.
    from score_wer import calculate_wer


@dataclass(frozen=True)
class CorpusWerResult:
    """Summed edit counts and word-weighted corpus WER.

    ``wer`` is a fraction (for example, ``0.10`` means 10%). It is computed
    from total substitutions, deletions, and insertions divided by total
    reference words; individual sample WER fractions are never averaged.
    """

    utterances_scored: int
    reference_word_count: int
    substitutions: int
    deletions: int
    insertions: int
    edit_distance: int
    wer: float


def calculate_corpus_wer(
    samples: Iterable[tuple[str, str]],
) -> CorpusWerResult:
    """Score reference/prediction pairs and return weighted corpus WER.

    The iterable must contain at least one pair, and every normalized
    reference must contain at least one word. Empty-reference samples are
    errors because silence/no-speech detection is evaluated separately from
    speech-recognition accuracy.

    Raises:
        TypeError: If a reference or prediction is not a string, as reported by
            the existing individual scorer.
        ValueError: If there are no samples or a sample has no normalized
            reference words.
    """
    utterances_scored = 0
    reference_word_count = 0
    substitutions = 0
    deletions = 0
    insertions = 0

    for utterances_scored, (reference, prediction) in enumerate(samples, start=1):
        result = calculate_wer(reference, prediction)
        if result.wer is None:
            raise ValueError(
                f"sample {utterances_scored} has an empty normalized reference; "
                "silence/no-speech samples must be evaluated separately"
            )

        reference_word_count += result.reference_word_count
        substitutions += result.substitutions
        deletions += result.deletions
        insertions += result.insertions

    if utterances_scored == 0:
        raise ValueError("at least one reference/prediction sample is required")

    edit_distance = substitutions + deletions + insertions
    # The per-sample guard guarantees this denominator is positive.
    wer = edit_distance / reference_word_count
    return CorpusWerResult(
        utterances_scored=utterances_scored,
        reference_word_count=reference_word_count,
        substitutions=substitutions,
        deletions=deletions,
        insertions=insertions,
        edit_distance=edit_distance,
        wer=wer,
    )


def _run_demo() -> None:
    """Print a two-sample synthetic example of weighted aggregation."""
    result = calculate_corpus_wer(
        [
            ("hello", "goodbye"),
            (
                "one two three four five six seven eight nine",
                "one two three four five six seven eight nine",
            ),
        ]
    )
    print("Outflow aggregate WER ready")
    print(f"Utterances scored: {result.utterances_scored}")
    print(f"Reference words: {result.reference_word_count}")
    print(f"Substitutions: {result.substitutions}")
    print(f"Deletions: {result.deletions}")
    print(f"Insertions: {result.insertions}")
    print(f"Corpus Word Error Rate: {result.wer:.2%}")


def main(argv: Sequence[str] | None = None) -> int:
    """Run the built-in synthetic demonstration."""
    parser = argparse.ArgumentParser(
        description="Outflow's development-only aggregate WER evaluator"
    )
    parser.add_argument("--demo", action="store_true", help="score two synthetic samples")
    arguments = parser.parse_args(argv)
    if not arguments.demo:
        parser.error("choose --demo; corpus scoring is also available through calculate_corpus_wer()")
    _run_demo()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
