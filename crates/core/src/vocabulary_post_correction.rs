//! Pure vocabulary replacement for C-06.
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

use std::collections::HashMap;

use crate::vocabulary::{
    VocabularyCollectionError, VocabularyEntryRef, VocabularyValidationError,
    validate_vocabulary_collection, validate_vocabulary_entry,
};
use crate::vocabulary_matching::{CURRENT_VOCABULARY_MATCHING_VERSION, comparison_key};

const COMBINING_CLASS_MAP: CanonicalCombiningClassMapBorrowed<'static> =
    CanonicalCombiningClassMapBorrowed::new();

/// Corrected text and the byte ranges occupied by inserted vocabulary values.
///
/// Each range indexes [`Self::text`] using UTF-8 byte offsets. It covers only
/// the exact written replacement inserted for one match; unmatched transcript
/// characters, spoken phrases, and vocabulary identifiers are not included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VocabularyCorrectedText {
    /// Corrected output with unmatched input bytes preserved.
    pub text: String,
    /// Ordered, non-overlapping UTF-8 byte ranges of written replacements.
    pub protected_ranges: Vec<std::ops::Range<usize>>,
}

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

    let mappings = [PreparedMapping {
        spoken_key: comparison_key(spoken, CURRENT_VOCABULARY_MATCHING_VERSION),
        written,
    }];

    Ok(apply_prepared_mappings_with_spans(text, &mappings).text)
}

/// Applies a complete vocabulary collection to the original text in one pass.
///
/// The entire collection is validated before the text is scanned. Entries use
/// the same Unicode 18 D145 comparison keys and conservative word boundaries
/// as [`apply_vocabulary_post_correction`]. Scanning proceeds left to right;
/// at each original-text position the longest matching span wins, with entry
/// order as a deterministic tie-breaker. Replacements never overlap, and
/// inserted text is not scanned again. The written value is copied exactly;
/// every unmatched byte in `text` remains unchanged.
///
/// An empty collection returns an owned copy of `text`. Collections may
/// contain up to 500 entries, each using the existing 128-scalar field limit.
/// Spoken phrases that share a canonical-caseless key are rejected before
/// scanning.
///
/// # Errors
///
/// Returns [`VocabularyCollectionError`] when there are more than 500
/// entries, any spoken or written field fails the existing vocabulary field
/// validation, or spoken phrases conflict under Unicode 18 canonical-caseless
/// matching. Error values identify indices and validation reasons without
/// exposing vocabulary text.
///
/// # Examples
///
/// ```
/// use outflow_core::vocabulary::{VocabularyEntryRef, VocabularyCollectionError};
/// use outflow_core::vocabulary_post_correction::apply_vocabulary_post_corrections;
///
/// let entries = [
///     VocabularyEntryRef { spoken: "rust", written: "Rust" },
///     VocabularyEntryRef { spoken: "ubc", written: "UBC" },
/// ];
/// let corrected = apply_vocabulary_post_corrections("rust at ubc", &entries)?;
/// assert_eq!(corrected, "Rust at UBC");
/// # Ok::<(), VocabularyCollectionError>(())
/// ```
pub fn apply_vocabulary_post_corrections(
    text: &str,
    entries: &[VocabularyEntryRef<'_>],
) -> Result<String, VocabularyCollectionError> {
    Ok(apply_vocabulary_post_corrections_with_spans(text, entries)?.text)
}

/// Applies a validated vocabulary collection and records every inserted
/// written replacement as a UTF-8 byte range in the returned text.
///
/// Matching uses the Unicode 18 canonical-caseless keys and conservative word
/// boundaries used by [`apply_vocabulary_post_corrections`]. Longest-match
/// precedence, left-to-right non-overlap, and non-cascading behavior are
/// unchanged. Ranges are recorded as replacements are appended, so each range
/// contains exactly its configured written value and excludes adjacent input.
/// No output scan is performed to infer replacement positions.
///
/// An empty collection or a collection with no matches returns an empty range
/// list. The input and entries are not modified. Error formatting continues to
/// identify only collection indices and validation reasons, never vocabulary
/// content.
///
/// # Errors
///
/// Returns [`VocabularyCollectionError`] for an oversized collection, an
/// invalid spoken or written field, or duplicate spoken phrases under the
/// Unicode 18 canonical-caseless policy.
///
/// # Examples
///
/// ```
/// use outflow_core::vocabulary::{VocabularyCollectionError, VocabularyEntryRef};
/// use outflow_core::vocabulary_post_correction::apply_vocabulary_post_corrections_with_spans;
///
/// let entries = [VocabularyEntryRef { spoken: "rust", written: "Rust" }];
/// let corrected = apply_vocabulary_post_corrections_with_spans("😀 rust!", &entries)?;
/// assert_eq!(corrected.text, "😀 Rust!");
/// assert_eq!(&corrected.text[corrected.protected_ranges[0].clone()], "Rust");
/// # Ok::<(), VocabularyCollectionError>(())
/// ```
pub fn apply_vocabulary_post_corrections_with_spans(
    text: &str,
    entries: &[VocabularyEntryRef<'_>],
) -> Result<VocabularyCorrectedText, VocabularyCollectionError> {
    validate_vocabulary_collection(entries)?;

    if entries.is_empty() {
        return Ok(VocabularyCorrectedText {
            text: text.to_owned(),
            protected_ranges: Vec::new(),
        });
    }

    let mappings = entries
        .iter()
        .map(|entry| PreparedMapping {
            spoken_key: comparison_key(entry.spoken, CURRENT_VOCABULARY_MATCHING_VERSION),
            written: entry.written,
        })
        .collect::<Vec<_>>();

    Ok(apply_prepared_mappings_with_spans(text, &mappings))
}

struct PreparedMapping<'a> {
    spoken_key: String,
    written: &'a str,
}

fn apply_prepared_mappings_with_spans(
    text: &str,
    mappings: &[PreparedMapping<'_>],
) -> VocabularyCorrectedText {
    if mappings.is_empty() {
        return VocabularyCorrectedText {
            text: text.to_owned(),
            protected_ranges: Vec::new(),
        };
    }

    let mut mapping_index_by_key = HashMap::<String, usize>::with_capacity(mappings.len());
    let mut maximum_key_scalars = 0;
    for (index, mapping) in mappings.iter().enumerate() {
        maximum_key_scalars = maximum_key_scalars.max(mapping.spoken_key.chars().count());
        // Keep the earliest entry if keys tie. Validated collections reject
        // such duplicates, but retaining this rule makes the scanner's final
        // tie-break deterministic if its input contract is reused internally.
        mapping_index_by_key
            .entry(mapping.spoken_key.clone())
            .or_insert(index);
    }

    let mut boundaries = text
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    boundaries.push(text.len());

    let mut output = String::with_capacity(text.len());
    let mut protected_ranges = Vec::new();
    let mut copied_until = 0;
    let mut start_index = 0;

    while start_index + 1 < boundaries.len() {
        let start = boundaries[start_index];
        if start < copied_until || has_word_character_before(text, start) {
            start_index += 1;
            continue;
        }

        let mut end_index = start_index + 1;
        let mut longest_match: Option<(usize, usize, usize)> = None;
        while end_index < boundaries.len() {
            let end = boundaries[end_index];
            let candidate_key =
                comparison_key(&text[start..end], CURRENT_VOCABULARY_MATCHING_VERSION);

            if !has_word_character_after(text, end)
                && let Some(&mapping_index) = mapping_index_by_key.get(candidate_key.as_str())
            {
                let should_replace =
                    longest_match.is_none_or(|(_, matched_end, matched_mapping_index)| {
                        let candidate_span_length = end - start;
                        let matched_span_length = matched_end - start;
                        candidate_span_length > matched_span_length
                            || (candidate_span_length == matched_span_length
                                && mapping_index < matched_mapping_index)
                    });
                if should_replace {
                    longest_match = Some((end_index, end, mapping_index));
                }
            }

            // Canonical decomposition and full case folding never remove
            // scalars, so extending a candidate beyond every mapping key
            // cannot produce an equal key.
            if candidate_key.chars().count() > maximum_key_scalars {
                break;
            }
            end_index += 1;
        }

        if let Some((matched_end_index, end, mapping_index)) = longest_match {
            output.push_str(&text[copied_until..start]);
            let replacement_start = output.len();
            output.push_str(mappings[mapping_index].written);
            protected_ranges.push(replacement_start..output.len());
            copied_until = end;
            start_index = matched_end_index;
        } else {
            start_index += 1;
        }
    }

    output.push_str(&text[copied_until..]);
    VocabularyCorrectedText {
        text: output,
        protected_ranges,
    }
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
    use super::{
        VocabularyCorrectedText, apply_vocabulary_post_correction,
        apply_vocabulary_post_corrections, apply_vocabulary_post_corrections_with_spans,
    };
    use crate::vocabulary::{
        MAX_VOCABULARY_ENTRIES, VocabularyCollectionError, VocabularyEntryRef, VocabularyField,
        VocabularyValidationError,
    };

    fn correct_many(
        text: &str,
        entries: &[(&str, &str)],
    ) -> Result<String, VocabularyCollectionError> {
        let entries = entries
            .iter()
            .map(|&(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();

        apply_vocabulary_post_corrections(text, &entries)
    }

    fn correct(text: &str, spoken: &str, written: &str) -> String {
        apply_vocabulary_post_correction(text, spoken, written)
            .expect("test mapping should be valid")
    }

    fn correct_many_with_spans(
        text: &str,
        entries: &[(&str, &str)],
    ) -> Result<VocabularyCorrectedText, VocabularyCollectionError> {
        let entries = entries
            .iter()
            .map(|&(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();

        apply_vocabulary_post_corrections_with_spans(text, &entries)
    }

    fn assert_protected_ranges(
        corrected: &VocabularyCorrectedText,
        expected_replacements: &[&str],
    ) {
        assert_eq!(
            corrected.protected_ranges.len(),
            expected_replacements.len()
        );

        let mut previous_end = 0;
        let actual_replacements = corrected
            .protected_ranges
            .iter()
            .map(|range| {
                assert!(range.start <= range.end);
                assert!(range.end <= corrected.text.len());
                assert!(range.start >= previous_end);
                assert!(corrected.text.is_char_boundary(range.start));
                assert!(corrected.text.is_char_boundary(range.end));
                previous_end = range.end;
                &corrected.text[range.clone()]
            })
            .collect::<Vec<_>>();

        assert_eq!(actual_replacements, expected_replacements);
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

    #[test]
    fn empty_vocabulary_returns_the_original_text() {
        let input = "  cafe\u{301}\n東京 😀  ";

        assert_eq!(correct_many(input, &[]), Ok(input.to_owned()));
    }

    #[test]
    fn applies_one_mapping_from_a_collection() {
        assert_eq!(
            correct_many("rust works", &[("rust", "Rust")]),
            Ok("Rust works".into())
        );
    }

    #[test]
    fn applies_multiple_independent_mappings_in_one_call() {
        assert_eq!(
            correct_many(
                "rust at ubc beside north parkade",
                &[
                    ("north parkade", "North Parkade"),
                    ("rust", "Rust"),
                    ("ubc", "UBC")
                ],
            ),
            Ok("Rust at UBC beside North Parkade".into())
        );
    }

    #[test]
    fn replaces_repeated_occurrences_for_each_mapping() {
        assert_eq!(
            correct_many("rust, RUST, then rust!", &[("rust", "Rust")]),
            Ok("Rust, Rust, then Rust!".into())
        );
    }

    #[test]
    fn matches_multiword_expressions_and_preserves_their_separators() {
        assert_eq!(
            correct_many(
                "  north PARKADE\nthen north parkade  ",
                &[("north parkade", "North Parkade")],
            ),
            Ok("  North Parkade\nthen North Parkade  ".into())
        );
    }

    #[test]
    fn resolves_overlapping_phrases_from_the_leftmost_original_position() {
        assert_eq!(
            correct_many(
                "new york city / york city",
                &[("new york city", "NYC"), ("york city", "York City")],
            ),
            Ok("NYC / York City".into())
        );
    }

    #[test]
    fn longest_matching_phrase_wins_even_when_a_shorter_entry_comes_first() {
        assert_eq!(
            correct_many(
                "new york today",
                &[("new", "New"), ("new york", "New York")],
            ),
            Ok("New York today".into())
        );
    }

    #[test]
    fn inserted_replacement_text_is_not_processed_again() {
        assert_eq!(
            correct_many("rust", &[("rust", "ubc"), ("ubc", "UBC")]),
            Ok("ubc".into())
        );
    }

    #[test]
    fn uses_unicode_full_case_folding_and_canonical_equivalence() {
        assert_eq!(
            correct_many(
                "STRASSE and cafe\u{301}",
                &[("straße", "Straße"), ("café", "Café")],
            ),
            Ok("Straße and Café".into())
        );
    }

    #[test]
    fn combining_marks_protect_complete_word_boundaries() {
        let input = "cafe\u{301} x\u{301}rust";

        assert_eq!(
            correct_many(input, &[("cafe", "Café"), ("rust", "Rust")]),
            Ok(input.into())
        );
    }

    #[test]
    fn preserves_emoji_japanese_and_unrelated_unicode_byte_for_byte() {
        let input = "😀 rust 東京 — cafe\u{301} 🎉";
        let expected = "😀 Rust 東京 — cafe\u{301} 🎉";

        assert_eq!(
            correct_many(input, &[("rust", "Rust")]),
            Ok(expected.into())
        );
    }

    #[test]
    fn rejects_canonically_duplicate_spoken_phrases() {
        assert_eq!(
            correct_many("text", &[("Straße", "First"), ("STRASSE", "Second")]),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn validates_the_whole_collection_before_scanning_text() {
        assert_eq!(
            correct_many("rust", &[("rust", "Rust"), ("PRIVATE SPOKEN", " \t")]),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 1,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            })
        );
    }

    #[test]
    fn enforces_field_limits_and_the_500_entry_collection_limit() {
        assert_eq!(
            correct_many("text", &[(" \t", "Written")]),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Spoken),
            })
        );

        assert_eq!(
            correct_many("text", &[("spoken", &"w".repeat(129))]),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::TooLong(VocabularyField::Written),
            })
        );

        let spoken_phrases = (0..=MAX_VOCABULARY_ENTRIES)
            .map(|index| format!("term-{index}"))
            .collect::<Vec<_>>();
        let entries = spoken_phrases
            .iter()
            .map(|spoken| VocabularyEntryRef {
                spoken,
                written: "replacement",
            })
            .collect::<Vec<_>>();

        assert_eq!(
            apply_vocabulary_post_corrections("term-0", &entries),
            Err(VocabularyCollectionError::TooManyEntries {
                count: MAX_VOCABULARY_ENTRIES + 1,
                maximum: MAX_VOCABULARY_ENTRIES,
            })
        );
    }

    #[test]
    fn accepts_the_maximum_500_entries() {
        let spoken_phrases = (0..MAX_VOCABULARY_ENTRIES)
            .map(|index| format!("term-{index}"))
            .collect::<Vec<_>>();
        let entries = spoken_phrases
            .iter()
            .map(|spoken| VocabularyEntryRef {
                spoken,
                written: "replacement",
            })
            .collect::<Vec<_>>();

        assert_eq!(
            apply_vocabulary_post_corrections("TERM-499", &entries),
            Ok("replacement".into())
        );
    }

    #[test]
    fn redacts_vocabulary_text_from_collection_errors() {
        let secret_spoken = "PRIVATE-SPOKEN-SENTINEL";
        let secret_written = "PRIVATE-WRITTEN-SENTINEL".repeat(12);
        let result = correct_many("text", &[(secret_spoken, &secret_written)]);
        let error = result.expect_err("overlong replacement must be rejected");
        let error_text = error.to_string();
        let error_debug = format!("{error:?}");

        assert!(!error_text.contains(secret_spoken));
        assert!(!error_text.contains(&secret_written));
        assert!(!error_debug.contains(secret_spoken));
        assert!(!error_debug.contains(&secret_written));
    }

    #[test]
    fn multi_entry_calls_are_deterministic_and_do_not_mutate_input() {
        let input = String::from("STRASSE; north PARKADE; rust; cafe\u{301} — 東京 😀");
        let snapshot = input.clone();
        let entries = [
            ("straße", "Straße"),
            ("north parkade", "North Parkade"),
            ("rust", "Rust"),
            ("café", "Café"),
        ];
        let expected = correct_many(&input, &entries).expect("test mappings should be valid");

        for _ in 0..20 {
            assert_eq!(correct_many(&input, &entries), Ok(expected.clone()));
        }
        assert_eq!(input, snapshot);
    }

    #[test]
    fn spans_are_empty_for_empty_collections_and_unmatched_text() {
        let input = "unchanged 東京 😀";
        let empty = correct_many_with_spans(input, &[]).expect("empty collection is valid");
        assert_eq!(empty.text, input);
        assert_protected_ranges(&empty, &[]);

        let unmatched =
            correct_many_with_spans(input, &[("rust", "Rust")]).expect("mapping should be valid");
        assert_eq!(unmatched.text, input);
        assert_protected_ranges(&unmatched, &[]);
    }

    #[test]
    fn records_one_replacement_without_including_adjacent_input() {
        let corrected = correct_many_with_spans("before rust, after", &[("rust", "Rust")])
            .expect("mapping should be valid");

        assert_eq!(corrected.text, "before Rust, after");
        assert_protected_ranges(&corrected, &["Rust"]);
    }

    #[test]
    fn records_multiple_replacements_and_repeated_occurrences_in_output_order() {
        let corrected = correct_many_with_spans(
            "rust at ubc, then RUST at UBC",
            &[("rust", "Rust"), ("ubc", "UBC")],
        )
        .expect("mappings should be valid");

        assert_eq!(corrected.text, "Rust at UBC, then Rust at UBC");
        assert_protected_ranges(&corrected, &["Rust", "UBC", "Rust", "UBC"]);
    }

    #[test]
    fn one_range_covers_a_multiple_word_replacement_exactly() {
        let corrected =
            correct_many_with_spans("north parkade!", &[("north parkade", "North Parkade")])
                .expect("mapping should be valid");

        assert_eq!(corrected.text, "North Parkade!");
        assert_protected_ranges(&corrected, &["North Parkade"]);
    }

    #[test]
    fn longest_match_precedence_is_reflected_in_the_protected_span() {
        let corrected = correct_many_with_spans(
            "new york today",
            &[("new", "New"), ("new york", "New York")],
        )
        .expect("mappings should be valid");

        assert_eq!(corrected.text, "New York today");
        assert_protected_ranges(&corrected, &["New York"]);
    }

    #[test]
    fn spans_do_not_cascade_from_inserted_text() {
        let corrected = correct_many_with_spans("rust", &[("rust", "ubc"), ("ubc", "UBC")])
            .expect("mappings should be valid");

        assert_eq!(corrected.text, "ubc");
        assert_protected_ranges(&corrected, &["ubc"]);
    }

    #[test]
    fn offsets_are_utf8_bytes_after_multibyte_input_and_replacements() {
        let corrected = correct_many_with_spans("😀 rust 東京", &[("rust", "naïve 😀")])
            .expect("mapping should be valid");

        assert_eq!(corrected.text, "😀 naïve 😀 東京");
        assert_eq!(corrected.protected_ranges[0], 5..16);
        assert_protected_ranges(&corrected, &["naïve 😀"]);
    }

    #[test]
    fn accented_canonical_matches_report_exact_replacement_bytes() {
        let corrected = correct_many_with_spans("cafe\u{301} then café", &[("café", "Café")])
            .expect("mapping should be valid");

        assert_eq!(corrected.text, "Café then Café");
        assert_eq!(corrected.protected_ranges, [0..5, 11..16]);
        assert_protected_ranges(&corrected, &["Café", "Café"]);
    }

    #[test]
    fn combining_characters_inside_replacements_keep_valid_boundaries() {
        let decomposed_written = "Cafe\u{301}";
        let corrected = correct_many_with_spans("😀 cafe\u{301}!", &[("café", decomposed_written)])
            .expect("mapping should be valid");

        assert_eq!(corrected.text, "😀 Cafe\u{301}!");
        assert_eq!(corrected.protected_ranges[0], 5..11);
        assert_protected_ranges(&corrected, &[decomposed_written]);
    }

    #[test]
    fn replacement_spaces_punctuation_and_newlines_are_protected_exactly() {
        let written = "Rust  , Edition!\n\n v 2";
        let corrected = correct_many_with_spans("before rust after", &[("rust", written)])
            .expect("mapping should be valid");

        assert_eq!(corrected.text, format!("before {written} after"));
        assert_protected_ranges(&corrected, &[written]);
    }

    #[test]
    fn invalid_and_duplicate_collections_are_rejected_by_the_shared_validator() {
        assert_eq!(
            correct_many_with_spans("rust", &[("rust", "  ")]),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            })
        );
        assert_eq!(
            correct_many_with_spans("text", &[("Straße", "First"), ("STRASSE", "Second")]),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn span_calls_preserve_input_and_collection_and_are_deterministic() {
        let input = String::from("😀 north PARKADE; café");
        let input_snapshot = input.clone();
        let entries = vec![
            VocabularyEntryRef {
                spoken: "north parkade",
                written: "North Parkade",
            },
            VocabularyEntryRef {
                spoken: "café",
                written: "Café",
            },
        ];
        let entries_snapshot = entries.clone();
        let expected = apply_vocabulary_post_corrections_with_spans(&input, &entries)
            .expect("mappings should be valid");

        for _ in 0..20 {
            assert_eq!(
                apply_vocabulary_post_corrections_with_spans(&input, &entries),
                Ok(expected.clone())
            );
        }
        assert_eq!(input, input_snapshot);
        assert_eq!(entries, entries_snapshot);
        assert_eq!(expected.text, "😀 North Parkade; Café");
        assert_protected_ranges(&expected, &["North Parkade", "Café"]);
    }

    #[test]
    fn text_only_multi_entry_api_matches_the_new_api_text() {
        let input = "😀 STRASSE; north PARKADE; rust; cafe\u{301}";
        let entries = [
            VocabularyEntryRef {
                spoken: "straße",
                written: "Straße",
            },
            VocabularyEntryRef {
                spoken: "north parkade",
                written: "North Parkade",
            },
            VocabularyEntryRef {
                spoken: "rust",
                written: "Rust",
            },
            VocabularyEntryRef {
                spoken: "café",
                written: "Café",
            },
        ];

        let with_spans = apply_vocabulary_post_corrections_with_spans(input, &entries)
            .expect("mappings should be valid");
        let text_only =
            apply_vocabulary_post_corrections(input, &entries).expect("mappings should be valid");

        assert_eq!(text_only, with_spans.text);
        assert_eq!(with_spans.text, "😀 Straße; North Parkade; Rust; Café");
        assert_protected_ranges(&with_spans, &["Straße", "North Parkade", "Rust", "Café"]);
    }
}
