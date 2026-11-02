"""Tests for the development-only vocabulary Unicode policy reference."""

from __future__ import annotations

import contextlib
import inspect
import io
import unicodedata
import unittest

from scripts.check_vocabulary_unicode_policy import (
    canonical_caseless_key,
    comparison_is_equal,
    main,
    unicode_data_version,
)


class VocabularyUnicodePolicyReferenceTests(unittest.TestCase):
    def test_ascii_case_variations_compare_equal(self) -> None:
        self.assertTrue(comparison_is_equal("rust", "Rust"))

    def test_non_ascii_case_variations_compare_equal(self) -> None:
        self.assertTrue(comparison_is_equal("CAF\u00c9", "caf\u00e9"))

    def test_german_sharp_s_uses_full_case_folding(self) -> None:
        self.assertTrue(comparison_is_equal("stra\u00dfe", "STRASSE"))

    def test_composed_and_decomposed_accents_compare_equal(self) -> None:
        self.assertTrue(comparison_is_equal("\u00e9", "e\u0301"))

    def test_greek_sigma_variants_compare_equal(self) -> None:
        self.assertTrue(comparison_is_equal("\u03a3", "\u03c3"))
        self.assertTrue(comparison_is_equal("\u03c3", "\u03c2"))

    def test_multiword_phrases_compare_case_insensitively(self) -> None:
        self.assertTrue(comparison_is_equal("North Parkade", "north parkade"))

    def test_leading_or_trailing_whitespace_remains_significant(self) -> None:
        self.assertFalse(comparison_is_equal("north parkade", "north parkade "))
        self.assertFalse(comparison_is_equal(" north parkade", "north parkade"))

    def test_turkish_dotless_i_remains_distinct_from_ascii_i(self) -> None:
        self.assertFalse(comparison_is_equal("I", "\u0131"))

    def test_turkish_dotted_i_remains_distinct_from_ascii_i(self) -> None:
        self.assertFalse(comparison_is_equal("\u0130", "i"))

    def test_compatibility_numeral_remains_distinct(self) -> None:
        self.assertFalse(comparison_is_equal("\u2460", "1"))

    def test_cross_script_visual_confusable_remains_distinct(self) -> None:
        self.assertFalse(comparison_is_equal("a", "\u0430"))

    def test_key_derivation_is_deterministic(self) -> None:
        value = "Stra\u00dfe e\u0301 \u03c2"
        first_key = canonical_caseless_key(value)
        second_key = canonical_caseless_key(value)

        self.assertEqual(first_key, second_key)
        self.assertEqual(first_key, "strasse e\u0301 \u03c3")

    def test_key_derivation_does_not_modify_original_values(self) -> None:
        spoken = " North Parkade "
        written = "NORTH PARKADE!"
        original_values = (spoken, written)

        canonical_caseless_key(spoken)

        self.assertEqual((spoken, written), original_values)

    def test_reference_function_does_not_accept_replacement_values(self) -> None:
        parameter_names = tuple(inspect.signature(canonical_caseless_key).parameters)

        self.assertEqual(parameter_names, ("value",))
        self.assertEqual(canonical_caseless_key("rust"), "rust")

    def test_demo_reports_the_python_unicode_data_version(self) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            exit_code = main(["--demo"])

        self.assertEqual(exit_code, 0)
        self.assertEqual(unicode_data_version(), unicodedata.unidata_version)
        self.assertIn(
            f"Python Unicode data version: {unicodedata.unidata_version}",
            output.getvalue(),
        )


if __name__ == "__main__":
    unittest.main()
