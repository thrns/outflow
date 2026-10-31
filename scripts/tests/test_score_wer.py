"""Tests for the standard-library lexical WER calculator."""

from __future__ import annotations

import unittest

from scripts.score_wer import calculate_wer


class WerCalculatorTests(unittest.TestCase):
    def test_identical_transcriptions_have_zero_wer(self) -> None:
        result = calculate_wer("the quick brown fox", "the quick brown fox")

        self.assertEqual(result.wer, 0.0)
        self.assertEqual(result.edit_distance, 0)

    def test_single_substitution_is_counted(self) -> None:
        result = calculate_wer("the cat", "the cap")

        self.assertEqual(result.substitutions, 1)
        self.assertEqual(result.deletions, 0)
        self.assertEqual(result.insertions, 0)
        self.assertEqual(result.wer, 0.5)

    def test_single_deletion_is_counted(self) -> None:
        result = calculate_wer("the quick brown fox", "the quick fox")

        self.assertEqual(result.substitutions, 0)
        self.assertEqual(result.deletions, 1)
        self.assertEqual(result.insertions, 0)
        self.assertEqual(result.wer, 0.25)

    def test_single_insertion_is_counted(self) -> None:
        result = calculate_wer("the fox", "the quick fox")

        self.assertEqual(result.substitutions, 0)
        self.assertEqual(result.deletions, 0)
        self.assertEqual(result.insertions, 1)
        self.assertEqual(result.wer, 0.5)

    def test_multiple_word_errors_are_counted(self) -> None:
        result = calculate_wer("a b c", "x y z")

        self.assertEqual(result.substitutions, 3)
        self.assertEqual(result.deletions, 0)
        self.assertEqual(result.insertions, 0)
        self.assertEqual(result.edit_distance, 3)

    def test_multiple_simultaneous_error_types_are_counted(self) -> None:
        result = calculate_wer("a b c d", "x c d")

        self.assertEqual(result.substitutions, 1)
        self.assertEqual(result.deletions, 1)
        self.assertEqual(result.insertions, 0)
        self.assertEqual(result.edit_distance, 2)

    def test_reference_word_count_uses_normalized_tokens(self) -> None:
        result = calculate_wer("  One   two\nthree ", "one two three")

        self.assertEqual(result.reference_word_count, 3)

    def test_case_normalization_is_unicode_lowercase(self) -> None:
        result = calculate_wer("Outflow CAFÉ", "outflow café")

        self.assertEqual(result.wer, 0.0)

    def test_whitespace_is_collapsed(self) -> None:
        result = calculate_wer("\talpha  beta\n", "alpha beta")

        self.assertEqual(result.wer, 0.0)
        self.assertEqual(result.reference_word_count, 2)

    def test_unicode_composition_and_words_are_preserved(self) -> None:
        result = calculate_wer("CAFÉ 東京 １２３", "cafe\u0301 東京 １２３")

        self.assertEqual(result.wer, 0.0)
        self.assertEqual(result.reference_word_count, 3)

    def test_punctuation_is_excluded_from_lexical_tokens(self) -> None:
        result = calculate_wer("Hello, world!", "hello world")

        self.assertEqual(result.wer, 0.0)

    def test_empty_prediction_counts_all_reference_words_as_deletions(self) -> None:
        result = calculate_wer("one two three", "")

        self.assertEqual(result.reference_word_count, 3)
        self.assertEqual(result.deletions, 3)
        self.assertEqual(result.wer, 1.0)

    def test_empty_reference_has_undefined_wer_and_counts_insertions(self) -> None:
        result = calculate_wer("", "one two")

        self.assertEqual(result.reference_word_count, 0)
        self.assertEqual(result.insertions, 2)
        self.assertEqual(result.edit_distance, 2)
        self.assertIsNone(result.wer)

    def test_insertions_can_make_wer_exceed_one_hundred_percent(self) -> None:
        result = calculate_wer("one", "one two three")

        self.assertEqual(result.insertions, 2)
        self.assertEqual(result.wer, 2.0)

    def test_number_spellings_are_not_rewritten(self) -> None:
        result = calculate_wer("007", "7")

        self.assertEqual(result.substitutions, 1)
        self.assertEqual(result.wer, 1.0)

    def test_repeated_calculations_are_deterministic(self) -> None:
        first = calculate_wer("a b c d", "x c e d")
        second = calculate_wer("a b c d", "x c e d")

        self.assertEqual(first, second)

    def test_edit_counts_sum_to_edit_distance(self) -> None:
        result = calculate_wer("a b c d", "x c e d")

        self.assertEqual(
            result.substitutions + result.deletions + result.insertions,
            result.edit_distance,
        )

    def test_non_string_inputs_raise_clear_type_error(self) -> None:
        with self.assertRaisesRegex(TypeError, "reference must be a string"):
            calculate_wer(None, "prediction")  # type: ignore[arg-type]


if __name__ == "__main__":
    unittest.main()
