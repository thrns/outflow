//! Conservative transcript whitespace normalization and composition for C-06.
//!
//! The first standalone cleanup stage normalizes horizontal ASCII spacing and
//! line endings while preserving every other character. A partial composition
//! then applies the existing validated vocabulary replacement stage. Neither
//! operation changes punctuation, capitalization, fillers, or dictation
//! commands, and this module has no storage, network, speech, UI, or global-
//! state effects.

use crate::vocabulary::{VocabularyCollectionError, VocabularyEntryRef};
use crate::vocabulary_post_correction::apply_vocabulary_post_corrections;

/// Normalizes ASCII horizontal whitespace and line endings in a transcript.
///
/// Windows CRLF and standalone carriage returns become `\n`. Runs of ASCII
/// spaces and tabs inside a line become one ASCII space, while leading and
/// trailing spaces and tabs on each line are removed. Newline count and
/// placement are preserved, including leading, trailing, and repeated
/// newlines. Other characters—including non-ASCII whitespace—are copied
/// unchanged.
///
/// The function scans the input once and returns a deterministic, idempotent
/// result. It does not alter words, punctuation, capitalization, numbers, or
/// Unicode text.
///
/// # Examples
///
/// ```
/// use outflow_core::text_cleanup::normalize_transcript_whitespace;
///
/// let normalized = normalize_transcript_whitespace("  hello\t  world.\r\n");
/// assert_eq!(normalized, "hello world.\n");
/// ```
pub fn normalize_transcript_whitespace(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut characters = input.chars().peekable();
    let mut line_has_content = false;

    while let Some(character) = characters.next() {
        match character {
            '\r' => {
                if characters.peek() == Some(&'\n') {
                    characters.next();
                }
                output.push('\n');
                line_has_content = false;
            }
            '\n' => {
                output.push('\n');
                line_has_content = false;
            }
            ' ' | '\t' => {
                while characters
                    .peek()
                    .is_some_and(|next| *next == ' ' || *next == '\t')
                {
                    characters.next();
                }

                // Delay the one possible space until its next character is
                // known, so line edges are trimmed without buffering a line.
                if line_has_content
                    && characters
                        .peek()
                        .is_some_and(|next| *next != '\r' && *next != '\n')
                {
                    output.push(' ');
                }
            }
            other => {
                output.push(other);
                line_has_content = true;
            }
        }
    }

    output
}

/// Normalizes transcript whitespace, then applies a validated vocabulary map.
///
/// This composes two existing stages in order: first
/// [`normalize_transcript_whitespace`] canonicalizes ASCII spaces, tabs, and
/// line endings; then [`apply_vocabulary_post_corrections`] validates and
/// applies the complete vocabulary collection using its Unicode 18 matching
/// and boundary rules. Future dictation-command recognition belongs between
/// these stages, as specified by FR-07.3.
///
/// Vocabulary matching sees the normalized original transcript. The existing
/// collection scanner preserves longest-match precedence and does not scan
/// inserted replacement text again during this call. Written values are
/// copied exactly, and the output is not whitespace-normalized a second time.
/// Thus, single-call non-cascading behavior does not make repeated application
/// of this whole function universally idempotent: with `rust` → `ubc` and
/// `ubc` → `UBC`, input `rust` yields `ubc` on the first call and `UBC` on a
/// later call. The vocabulary is never rewritten to force idempotence.
///
/// The input and borrowed entries are read-only and remain unchanged.
///
/// # Errors
///
/// Returns [`VocabularyCollectionError`] when the collection exceeds its
/// existing size limit, contains an invalid entry, or has duplicate spoken
/// phrases under the existing Unicode 18 canonical-caseless matching policy.
///
/// # Examples
///
/// ```
/// use outflow_core::text_cleanup::normalize_whitespace_and_apply_vocabulary;
/// use outflow_core::vocabulary::VocabularyEntryRef;
///
/// let vocabulary = [VocabularyEntryRef {
///     spoken: "rust",
///     written: "Rust",
/// }];
/// let output = normalize_whitespace_and_apply_vocabulary("  rust\t", &vocabulary)?;
/// assert_eq!(output, "Rust");
/// # Ok::<(), outflow_core::vocabulary::VocabularyCollectionError>(())
/// ```
pub fn normalize_whitespace_and_apply_vocabulary(
    input: &str,
    entries: &[VocabularyEntryRef<'_>],
) -> Result<String, VocabularyCollectionError> {
    // Keep these calls in FR-07.3 order. Command recognition can be inserted
    // between them later without duplicating either stage's implementation.
    let normalized = normalize_transcript_whitespace(input);
    apply_vocabulary_post_corrections(&normalized, entries)
}

#[cfg(test)]
mod tests {
    use super::{normalize_transcript_whitespace, normalize_whitespace_and_apply_vocabulary};
    use crate::vocabulary::{
        MAX_VOCABULARY_ENTRIES, VocabularyCollectionError, VocabularyEntryRef, VocabularyField,
        VocabularyValidationError,
    };

    fn vocabulary_refs<'a>(pairs: &[(&'a str, &'a str)]) -> Vec<VocabularyEntryRef<'a>> {
        pairs
            .iter()
            .map(|&(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect()
    }

    #[test]
    fn normalizes_table_of_whitespace_fixtures_idempotently() {
        let fixtures = [
            ("", ""),
            ("Already normalized.", "Already normalized."),
            ("I  visited   north   parkade.", "I visited north parkade."),
            ("I\tuse\trust.", "I use rust."),
            ("I \t \tuse\t  rust.", "I use rust."),
            ("   leading", "leading"),
            ("trailing \t  ", "trailing"),
            (" \t hello \t ", "hello"),
            ("one\n two \nthree", "one\ntwo\nthree"),
            ("one\r\ntwo\r\nthree", "one\ntwo\nthree"),
            ("one\rtwo\rthree", "one\ntwo\nthree"),
            ("one\r\ntwo\rthree\nfour", "one\ntwo\nthree\nfour"),
            ("one\n\n\n four", "one\n\n\nfour"),
            ("\n\nfirst", "\n\nfirst"),
            ("last\n\n", "last\n\n"),
            ("one\n \t \nthree", "one\n\nthree"),
            ("\t \t", ""),
            ("  Hello,   world!?! \t", "Hello, world!?!"),
            ("  North Parkade 42nd St.  ", "North Parkade 42nd St."),
            ("  café  déjà vu  ", "café déjà vu"),
            ("  cafe\u{301}   A\u{308}  ", "cafe\u{301} A\u{308}"),
            ("  😀 東京\tです  ", "😀 東京 です"),
            ("\u{00a0}keep\u{00a0}", "\u{00a0}keep\u{00a0}"),
            ("a\u{200b}b", "a\u{200b}b"),
            (" \u{2003}word\u{2003} ", "\u{2003}word\u{2003}"),
            ("\t \r\r\t ", "\n\n"),
            ("a\r\n \r\n\r\nb", "a\n\n\nb"),
            ("line \t\n", "line\n"),
            ("\n \tline", "\nline"),
            ("  １２３ ٤٢  007  ", "１２３ ٤٢ 007"),
            ("x\u{000b}y\u{000c}z", "x\u{000b}y\u{000c}z"),
            ("a  \r\n\r\n  b  ", "a\n\nb"),
        ];

        for (index, (input, expected)) in fixtures.into_iter().enumerate() {
            let original = input.to_owned();
            let actual = normalize_transcript_whitespace(input);

            assert_eq!(actual, expected, "fixture {index}, input {input:?}");
            assert_eq!(
                normalize_transcript_whitespace(&actual),
                actual,
                "fixture {index} was not idempotent"
            );
            assert_eq!(input, original, "fixture {index} changed its input");
        }
    }

    #[test]
    fn preserves_exact_newline_count_and_positions() {
        let input = "\r\none \t\r\n\r\n two\r";
        let normalized = normalize_transcript_whitespace(input);

        assert_eq!(normalized, "\none\n\ntwo\n");
        assert_eq!(
            normalized
                .chars()
                .filter(|character| *character == '\n')
                .count(),
            4
        );
    }

    #[test]
    fn trims_only_ascii_spaces_and_tabs() {
        let input = " \u{00a0}\u{2003}\u{2009} \t ";

        assert_eq!(
            normalize_transcript_whitespace(input),
            "\u{00a0}\u{2003}\u{2009}"
        );
    }

    #[test]
    fn preserves_words_punctuation_and_unicode_bytes() {
        let input = "  naïve cafe\u{301}, 東京 😀 007!?  ";
        let normalized = normalize_transcript_whitespace(input);

        assert_eq!(normalized, "naïve cafe\u{301}, 東京 😀 007!?");
        assert!(normalized.as_bytes().ends_with(b"!?"));
        assert!(normalized.contains("cafe\u{301}"));
    }

    #[test]
    fn normalization_of_empty_text_is_empty() {
        assert_eq!(normalize_transcript_whitespace(""), "");
    }

    #[test]
    fn composed_pipeline_preserves_empty_input() {
        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("", &[]),
            Ok(String::new())
        );
    }

    #[test]
    fn composed_pipeline_normalizes_with_an_empty_vocabulary() {
        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("  hello\t  world.  ", &[]),
            Ok("hello world.".to_owned())
        );
    }

    #[test]
    fn whitespace_is_normalized_before_vocabulary_correction() {
        let entries = vocabulary_refs(&[("north parkade", "North Parkade")]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("  north \t  parkade  ", &entries),
            Ok("North Parkade".to_owned())
        );
    }

    #[test]
    fn applies_multiple_entries_and_normalizes_multiword_matches_first() {
        let entries = vocabulary_refs(&[
            ("north parkade", "North Parkade"),
            ("rust", "Rust"),
            ("ubc", "UBC"),
        ]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary(
                "  I visited   north\t PARKADE, then rust  at  ubc.  ",
                &entries
            ),
            Ok("I visited North Parkade, then Rust at UBC.".to_owned())
        );
    }

    #[test]
    fn normalizes_crlf_and_standalone_carriage_returns() {
        let entries = vocabulary_refs(&[("rust", "Rust"), ("ubc", "UBC")]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary(" \trust \r\n  ubc\t \r rust ", &entries),
            Ok("Rust\nUBC\nRust".to_owned())
        );
    }

    #[test]
    fn preserves_paragraph_breaks_punctuation_numbers_and_unrelated_unicode() {
        let entries = vocabulary_refs(&[("rust", "Rust")]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary(
                "  (RUST), 007!? \r\n\r\n  東京 😀  ",
                &entries
            ),
            Ok("(Rust), 007!?\n\n東京 😀".to_owned())
        );
    }

    #[test]
    fn matches_accents_across_canonical_forms_and_preserves_combining_text() {
        let entries = vocabulary_refs(&[("café", "CAFÉ")]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("  cafe\u{301} and naïve  ", &entries),
            Ok("CAFÉ and naïve".to_owned())
        );
    }

    #[test]
    fn longest_vocabulary_match_wins_at_the_same_start() {
        let entries = vocabulary_refs(&[("new", "Short"), ("new york", "New York")]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("new \t york!", &entries),
            Ok("New York!".to_owned())
        );
    }

    #[test]
    fn replacements_do_not_cascade_within_one_call() {
        let entries = vocabulary_refs(&[("rust", "ubc"), ("ubc", "UBC")]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("rust", &entries),
            Ok("ubc".to_owned())
        );
    }

    #[test]
    fn written_replacements_keep_repeated_spaces_and_line_breaks_exactly() {
        let entries = vocabulary_refs(&[("label", "Written  text\ncontinued")]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("label!", &entries),
            Ok("Written  text\ncontinued!".to_owned())
        );
    }

    #[test]
    fn rejects_invalid_entries_with_the_existing_typed_error() {
        let entries = vocabulary_refs(&[("rust", " \t")]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("rust", &entries),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            })
        );
    }

    #[test]
    fn rejects_duplicate_spoken_phrases_with_the_existing_typed_error() {
        let entries = vocabulary_refs(&[("rust", "Rust"), ("RUST", "Written Rust")]);

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("rust", &entries),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn accepts_500_entries_and_rejects_501() {
        let owned_entries = (0..=MAX_VOCABULARY_ENTRIES)
            .map(|index| (format!("spoken-{index}"), format!("written-{index}")))
            .collect::<Vec<_>>();
        let entries = owned_entries
            .iter()
            .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("", &entries[..MAX_VOCABULARY_ENTRIES]),
            Ok(String::new())
        );
        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("", &entries),
            Err(VocabularyCollectionError::TooManyEntries {
                count: MAX_VOCABULARY_ENTRIES + 1,
                maximum: MAX_VOCABULARY_ENTRIES,
            })
        );
    }

    #[test]
    fn leaves_input_and_vocabulary_entries_unchanged() {
        let input = String::from("  rust\t and  ubc  ");
        let owned_entries = vec![
            (String::from("rust"), String::from("Rust")),
            (String::from("ubc"), String::from("UBC")),
        ];
        let original_input = input.clone();
        let original_entries = owned_entries.clone();
        let entries = owned_entries
            .iter()
            .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();

        assert_eq!(
            normalize_whitespace_and_apply_vocabulary(&input, &entries),
            Ok("Rust and UBC".to_owned())
        );
        assert_eq!(input, original_input);
        assert_eq!(owned_entries, original_entries);
    }

    #[test]
    fn repeated_calls_with_the_same_input_and_entries_are_deterministic() {
        let input = "  north \t parkade and rust  ";
        let entries = vocabulary_refs(&[("north parkade", "North Parkade"), ("rust", "Rust")]);

        let first = normalize_whitespace_and_apply_vocabulary(input, &entries);
        let second = normalize_whitespace_and_apply_vocabulary(input, &entries);

        assert_eq!(first, Ok("North Parkade and Rust".to_owned()));
        assert_eq!(second, Ok("North Parkade and Rust".to_owned()));
        assert_eq!(first, second);
    }

    #[test]
    fn demonstrates_that_reapplying_the_pipeline_can_change_output() {
        let entries = vocabulary_refs(&[("rust", "ubc"), ("ubc", "UBC")]);

        let first = normalize_whitespace_and_apply_vocabulary("rust", &entries)
            .expect("the test vocabulary is valid");
        let second = normalize_whitespace_and_apply_vocabulary(&first, &entries)
            .expect("the test vocabulary is valid");

        assert_eq!(first, "ubc");
        assert_eq!(second, "UBC");
        assert_ne!(first, second);
    }
}
