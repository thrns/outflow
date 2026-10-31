"""Tests for standard-library corpus-level lexical WER aggregation."""

from __future__ import annotations

import contextlib
import io
import unittest

from scripts.score_corpus_wer import calculate_corpus_wer, main
from scripts.score_wer import calculate_wer


class CorpusWerTests(unittest.TestCase):
    def test_identical_samples_have_zero_corpus_wer(self) -> None:
        result = calculate_corpus_wer(
            [("one two", "one two"), ("three", "three")]
        )

        self.assertEqual(result.wer, 0.0)
        self.assertEqual(result.edit_distance, 0)

    def test_single_sample_matches_individual_wer(self) -> None:
        individual = calculate_wer("one two", "one three")
        corpus = calculate_corpus_wer([("one two", "one three")])

        self.assertEqual(corpus.wer, individual.wer)
        self.assertEqual(corpus.substitutions, individual.substitutions)
        self.assertEqual(corpus.deletions, individual.deletions)
        self.assertEqual(corpus.insertions, individual.insertions)

    def test_multiple_samples_aggregate_error_counts_and_wer(self) -> None:
        result = calculate_corpus_wer(
            [("hello", "goodbye"), ("one two three", "one two three")]
        )

        self.assertEqual(result.utterances_scored, 2)
        self.assertEqual(result.reference_word_count, 4)
        self.assertEqual(result.substitutions, 1)
        self.assertEqual(result.deletions, 0)
        self.assertEqual(result.insertions, 0)
        self.assertEqual(result.wer, 0.25)

    def test_substitutions_are_summed_across_samples(self) -> None:
        result = calculate_corpus_wer([("a", "x"), ("b", "y")])

        self.assertEqual(result.substitutions, 2)

    def test_deletions_are_summed_across_samples(self) -> None:
        result = calculate_corpus_wer([("a b", "a"), ("c d", "")])

        self.assertEqual(result.deletions, 3)

    def test_insertions_are_summed_across_samples(self) -> None:
        result = calculate_corpus_wer([("a", "a b"), ("c", "c d e")])

        self.assertEqual(result.insertions, 3)

    def test_total_reference_word_count_is_summed(self) -> None:
        result = calculate_corpus_wer(
            [("one two", "one two"), ("three four five", "three four five")]
        )

        self.assertEqual(result.reference_word_count, 5)

    def test_total_scored_utterances_is_returned(self) -> None:
        result = calculate_corpus_wer(
            [("one", "one"), ("two", "two"), ("three", "three")]
        )

        self.assertEqual(result.utterances_scored, 3)

    def test_corpus_wer_is_weighted_not_arithmetic_mean(self) -> None:
        result = calculate_corpus_wer(
            [
                ("hello", "goodbye"),
                (
                    "one two three four five six seven eight nine",
                    "one two three four five six seven eight nine",
                ),
            ]
        )

        self.assertEqual(result.wer, 0.1)
        self.assertNotEqual(result.wer, (1.0 + 0.0) / 2)

    def test_demo_prints_ten_percent_corpus_wer(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = main(["--demo"])

        self.assertEqual(exit_code, 0)
        self.assertIn("Utterances scored: 2", output.getvalue())
        self.assertIn("Reference words: 10", output.getvalue())
        self.assertIn("Substitutions: 1", output.getvalue())
        self.assertIn("Corpus Word Error Rate: 10.00%", output.getvalue())

    def test_empty_collection_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "at least one"):
            calculate_corpus_wer([])

    def test_empty_normalized_reference_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "empty normalized reference"):
            calculate_corpus_wer([(" \t!!!", "one")])

    def test_empty_predictions_count_as_deletions(self) -> None:
        result = calculate_corpus_wer([("one two three", "")])

        self.assertEqual(result.reference_word_count, 3)
        self.assertEqual(result.deletions, 3)
        self.assertEqual(result.wer, 1.0)

    def test_unicode_normalization_matches_individual_scorer(self) -> None:
        reference = "CAFÉ 東京 １２３"
        prediction = "cafe\u0301 東京 １２３"
        individual = calculate_wer(reference, prediction)
        corpus = calculate_corpus_wer([(reference, prediction)])

        self.assertEqual(corpus.wer, individual.wer)
        self.assertEqual(corpus.wer, 0.0)
        self.assertEqual(corpus.reference_word_count, 3)

    def test_corpus_wer_can_exceed_one_hundred_percent(self) -> None:
        result = calculate_corpus_wer([("one", "one two three")])

        self.assertEqual(result.insertions, 2)
        self.assertEqual(result.wer, 2.0)

    def test_repeated_calculations_are_deterministic(self) -> None:
        samples = [("a b c", "x c"), ("cat", "cat a b")]

        self.assertEqual(calculate_corpus_wer(samples), calculate_corpus_wer(samples))

    def test_aggregate_counts_match_existing_sample_alignments(self) -> None:
        samples = [("a b c", "x c"), ("cat", "cat a b"), ("x y", "")]
        individual_results = [calculate_wer(reference, prediction) for reference, prediction in samples]
        corpus = calculate_corpus_wer(samples)

        self.assertEqual(
            corpus.substitutions,
            sum(result.substitutions for result in individual_results),
        )
        self.assertEqual(
            corpus.deletions,
            sum(result.deletions for result in individual_results),
        )
        self.assertEqual(
            corpus.insertions,
            sum(result.insertions for result in individual_results),
        )
        self.assertEqual(
            corpus.reference_word_count,
            sum(result.reference_word_count for result in individual_results),
        )


if __name__ == "__main__":
    unittest.main()
