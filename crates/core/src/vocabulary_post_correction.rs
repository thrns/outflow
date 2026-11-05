//! Pure, single-entry vocabulary replacement for C-06.
//!
//! The input is scanned only at UTF-8 character boundaries. Candidate spans
//! use the same Unicode 18 canonical-caseless key as vocabulary validation,
//! while output outside accepted spans is copied byte-for-byte from the
//! original input. This module has no storage, network, speech, UI, or global
//! mutable state.
//!
//! Word boundaries are conservative: Unicode letters and numbers, canonical
//! combining marks, underscore, apostrophes, join controls, variation
//! selectors, enclosing marks, and common invisible format controls count as
//! word continuations. This prevents a match from ending before an extender or
//! starting after one. Other punctuation and symbols separate words.

use icu_normalizer::properties::CanonicalCombiningClassMapBorrowed;

use crate::vocabulary::{VocabularyValidationError, validate_vocabulary_entry};
use crate::vocabulary_matching::{CURRENT_VOCABULARY_MATCHING_VERSION, comparison_key};

const COMBINING_CLASS_MAP: CanonicalCombiningClassMapBorrowed<'static> =
    CanonicalCombiningClassMapBorrowed::new();

/// Replaces each complete occurrence of one spoken phrase with its written
/// form, comparing the spoken phrase case-insensitively under Unicode 18 D145.
///
/// The replacement is inserted exactly as configured. Every byte outside a
/// matched phrase, including whitespace, punctuation, line breaks, and the
/// original capitalization, is preserved. Replacements do not overlap and
/// inserted text is not scanned again during this call.
///
/// Word boundaries use Unicode alphanumeric characters plus combining marks
/// and common word joiners. The function validates both mapping fields with
/// [`validate_vocabulary_entry`] before matching. It does not trim either
/// field or normalize output.
///
/// # Errors
///
/// Returns [`VocabularyValidationError`] when `spoken` or `written` is empty,
/// whitespace-only, or longer than the configured 128 Unicode scalar values.
///
/// # Examples
///
/// ```
/// use outflow_core::vocabulary_post_correction::apply_vocabulary_post_correction;
///
/// let corrected = apply_vocabulary_post_correction(
///     "I visited north parkade today.",
///     "north parkade",
///     "North Parkade",
/// )?;
/// assert_eq!(corrected, "I visited North Parkade today.");
/// # Ok::<(), outflow_core::vocabulary::VocabularyValidationError>(())
/// ```
pub fn apply_vocabulary_post_correction(
    text: &str,
    spoken: &str,
    written: &str,
) -> Result<String, VocabularyValidationError> {
    validate_vocabulary_entry(spoken, written)?;

    let version = CURRENT_VOCABULARY_MATCHING_VERSION;
    let spoken_key = comparison_key(spoken, version);
    let spoken_key_scalars = spoken_key.chars().count();

    let mut boundaries = text
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    boundaries.push(text.len());

    let mut output = String::with_capacity(text.len());
    let mut copied_until = 0;
    let mut start_index = 0;

    while start_index + 1 < boundaries.len() {
        let start = boundaries[start_index];
        if start < copied_until || has_word_character_before(text, start) {
            start_index += 1;
            continue;
        }

        let mut end_index = start_index + 1;
        let mut matched_end = None;
        while end_index < boundaries.len() {
            let end = boundaries[end_index];
            let candidate_key = comparison_key(&text[start..end], version);

            if candidate_key == spoken_key && !has_word_character_after(text, end) {
                matched_end = Some((end_index, end));
                break;
            }

            // Canonical decomposition and full case folding never remove
            // scalars, so once this candidate exceeds the mapping key length,
            // extending the span cannot produce an equal key.
            if candidate_key.chars().count() > spoken_key_scalars {
                break;
            }
            end_index += 1;
        }

        if let Some((matched_end_index, end)) = matched_end {
            output.push_str(&text[copied_until..start]);
            output.push_str(written);
            copied_until = end;
            start_index = matched_end_index;
        } else {
            start_index += 1;
        }
    }

    output.push_str(&text[copied_until..]);
    Ok(output)
}

fn has_word_character_before(text: &str, index: usize) -> bool {
    let Some(previous) = text[..index].chars().next_back() else {
        return false;
    };

    if is_apostrophe(previous) {
        let apostrophe_start = index - previous.len_utf8();
        return text[..apostrophe_start]
            .chars()
            .next_back()
            .is_some_and(is_word_continuation)
            && text[index..]
                .chars()
                .next()
                .is_some_and(is_word_continuation);
    }

    is_word_continuation(previous)
}

fn has_word_character_after(text: &str, index: usize) -> bool {
    let Some(next) = text[index..].chars().next() else {
        return false;
    };

    if is_apostrophe(next) {
        let apostrophe_end = index + next.len_utf8();
        return text[..index]
            .chars()
            .next_back()
            .is_some_and(is_word_continuation)
            && text[apostrophe_end..]
                .chars()
                .next()
                .is_some_and(is_word_continuation);
    }

    is_word_continuation(next)
}

fn is_word_continuation(character: char) -> bool {
    character.is_alphanumeric()
        || matches!(character, '_' | '\u{034F}' | '\u{200C}' | '\u{200D}')
        || is_canonical_combining_mark(character)
        || is_variation_selector(character)
        || is_enclosing_mark(character)
        || is_invisible_format_control(character)
}

fn is_apostrophe(character: char) -> bool {
    matches!(character, '\'' | '’')
}

fn is_canonical_combining_mark(character: char) -> bool {
    COMBINING_CLASS_MAP.get_u8(character) != 0
}

fn is_variation_selector(character: char) -> bool {
    matches!(character as u32, 0xFE00..=0xFE0F | 0xE0100..=0xE01EF)
}

fn is_enclosing_mark(character: char) -> bool {
    // Enclosing marks in these ranges have canonical combining class zero, so
    // the combining-class map alone would otherwise permit partial matches.
    matches!(character as u32, 0x20DD..=0x20E0 | 0x20E2..=0x20E4)
}

fn is_invisible_format_control(character: char) -> bool {
    matches!(
        character as u32,
        0x00AD | 0x061C | 0x180E | 0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x206F | 0xFEFF
    )
}

#[cfg(test)]
mod tests {
    use super::apply_vocabulary_post_correction;
    use crate::vocabulary::{VocabularyField, VocabularyValidationError};

    fn correct(text: &str, spoken: &str, written: &str) -> String {
        apply_vocabulary_post_correction(text, spoken, written)
            .expect("test mapping should be valid")
    }

    #[test]
    fn replaces_an_exact_phrase() {
        assert_eq!(
            correct(
                "I visited north parkade today.",
                "north parkade",
                "North Parkade"
            ),
            "I visited North Parkade today."
        );
    }

    #[test]
    fn matches_different_capitalization() {
        assert_eq!(correct("RUST and rust", "rust", "Rust"), "Rust and Rust");
    }

    #[test]
    fn replaces_multiple_non_overlapping_occurrences() {
        assert_eq!(
            correct("rust, then RUST; rust!", "rust", "Rust"),
            "Rust, then Rust; Rust!"
        );
    }

    #[test]
    fn matches_multiword_phrases_as_one_span() {
        assert_eq!(
            correct(
                "north parkade and North Parkade",
                "north parkade",
                "North Parkade"
            ),
            "North Parkade and North Parkade"
        );
    }

    #[test]
    fn preserves_punctuation_next_to_the_phrase() {
        assert_eq!(
            correct("(rust), ‘rust’...", "rust", "Rust"),
            "(Rust), ‘Rust’..."
        );
    }

    #[test]
    fn treats_apostrophes_as_word_joiners_only_inside_words() {
        assert_eq!(correct("can't", "can", "CAN"), "can't");
        assert_eq!(correct("can't", "can't", "cannot"), "cannot");
    }

    #[test]
    fn does_not_replace_a_phrase_inside_larger_ascii_words() {
        assert_eq!(
            correct("crust rusty rust", "rust", "Rust"),
            "crust rusty Rust"
        );
    }

    #[test]
    fn matches_accented_text_across_canonical_equivalence() {
        assert_eq!(
            correct("We visited CAFE\u{301} today.", "café", "Café"),
            "We visited Café today."
        );
        assert_eq!(correct("café", "cafe\u{301}", "CAFÉ"), "CAFÉ");
    }

    #[test]
    fn combining_marks_and_non_ascii_letters_prevent_partial_boundaries() {
        assert_eq!(correct("rust東京", "rust", "Rust"), "rust東京");
        assert_eq!(correct("東京rust", "rust", "Rust"), "東京rust");
        assert_eq!(correct("x\u{301}rust", "rust", "Rust"), "x\u{301}rust");
        assert_eq!(correct("x\u{20DD}rust", "rust", "Rust"), "x\u{20DD}rust");
        assert_eq!(correct("cafe\u{FE0F}", "cafe", "Café"), "cafe\u{FE0F}");
        assert_eq!(correct("rust\u{2060}y", "rust", "Rust"), "rust\u{2060}y");
    }

    #[test]
    fn cafe_does_not_partially_match_before_a_combining_accent() {
        let input = "cafe\u{301}";
        assert_eq!(correct(input, "cafe", "Café"), input);
    }

    #[test]
    fn preserves_emoji_and_unrelated_unicode_around_matches() {
        assert_eq!(
            correct("😀 rust 東京 🎉", "rust", "Rust"),
            "😀 Rust 東京 🎉"
        );
    }

    #[test]
    fn preserves_multiple_spaces_and_newlines() {
        assert_eq!(
            correct("one  rust\n\n  rust\tend", "rust", "Rust"),
            "one  Rust\n\n  Rust\tend"
        );
    }

    #[test]
    fn returns_unchanged_text_when_there_is_no_match() {
        let input = "nothing to correct — 東京 😀";
        assert_eq!(correct(input, "rust", "Rust"), input);
    }

    #[test]
    fn rejects_empty_or_invalid_mapping_fields() {
        assert_eq!(
            apply_vocabulary_post_correction("some text", "  \t", "Written"),
            Err(VocabularyValidationError::Empty(VocabularyField::Spoken))
        );
        assert_eq!(
            apply_vocabulary_post_correction("some text", "spoken", "\n  "),
            Err(VocabularyValidationError::Empty(VocabularyField::Written))
        );
        assert_eq!(
            apply_vocabulary_post_correction("some text", &"s".repeat(129), "Written"),
            Err(VocabularyValidationError::TooLong(VocabularyField::Spoken))
        );
        assert_eq!(
            apply_vocabulary_post_correction("some text", "spoken", &"w".repeat(129)),
            Err(VocabularyValidationError::TooLong(VocabularyField::Written))
        );
    }

    #[test]
    fn leaves_the_original_input_unchanged() {
        let input = String::from("north PARKADE and café\u{301}");
        let snapshot = input.clone();

        let result = apply_vocabulary_post_correction(&input, "north parkade", "North Parkade")
            .expect("test mapping should be valid");

        assert_eq!(input, snapshot);
        assert_eq!(result, "North Parkade and café\u{301}");
    }

    #[test]
    fn repeated_calls_are_deterministic() {
        let input = "north PARKADE — north parkade — café\u{301}";
        let expected = correct(input, "north parkade", "North Parkade");

        for _ in 0..20 {
            assert_eq!(
                apply_vocabulary_post_correction(input, "north parkade", "North Parkade"),
                Ok(expected.clone())
            );
        }
    }
}
