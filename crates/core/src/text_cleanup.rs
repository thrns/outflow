//! Conservative transcript whitespace normalization and composition for C-06.
//!
//! The standalone cleanup stages normalize horizontal ASCII spacing and line
//! endings, optionally recognize two explicit dictation commands, and apply
//! validated vocabulary corrections in FR-07.3 order. This partial pipeline
//! does not change punctuation, capitalization, or fillers, and has no
//! storage, network, speech, UI, or global-state effects.

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

/// Converts recognized `new line` and `new paragraph` phrases into line breaks.
///
/// Matching is ASCII case-insensitive. A phrase must use exactly one ASCII
/// space between its words and be bounded by ASCII spaces, input edges, or
/// line boundaries (`\r` or `\n`). Punctuation and quotation marks are not
/// command boundaries. Immediate ASCII spaces around a match are removed so
/// the line break does not retain surrounding spaces. Existing line breaks
/// and every unrelated input character are preserved. The scan reads only the
/// original input and never scans inserted line breaks again.
///
/// This function is intended for normalized transcript input: use
/// [`normalize_transcript_whitespace`] first when source whitespace may
/// contain tabs, repeated spaces, or noncanonical line endings. Even exact
/// phrases can be ordinary spoken text; this is an opt-in literal rule, not a
/// semantic intent classifier.
///
/// When `enabled` is `false`, the input is returned unchanged.
///
/// # Examples
///
/// ```
/// use outflow_core::text_cleanup::apply_dictation_commands;
///
/// assert_eq!(apply_dictation_commands("hello NEW LINE world", true), "hello\nworld");
/// assert_eq!(apply_dictation_commands("new paragraph", false), "new paragraph");
/// ```
pub fn apply_dictation_commands(input: &str, enabled: bool) -> String {
    if !enabled {
        return input.to_owned();
    }

    let bytes = input.as_bytes();
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;

    while cursor < bytes.len() {
        let has_left_boundary = cursor == 0 || is_command_boundary(bytes[cursor - 1]);
        if has_left_boundary && let Some((phrase_length, newline_count)) = command_at(bytes, cursor)
        {
            // The separator before a command has already been copied. Remove
            // its trailing ASCII spaces, then skip the separator after the
            // command directly in the original input.
            while output.ends_with(' ') {
                output.pop();
            }

            cursor += phrase_length;
            while bytes.get(cursor) == Some(&b' ') {
                cursor += 1;
            }
            output.extend(std::iter::repeat_n('\n', newline_count));
            continue;
        }

        let character = input[cursor..]
            .chars()
            .next()
            .expect("cursor remains at a UTF-8 character boundary");
        output.push(character);
        cursor += character.len_utf8();
    }

    output
}

fn command_at(bytes: &[u8], start: usize) -> Option<(usize, usize)> {
    const COMMANDS: [(&[u8], usize); 2] = [(b"new line", 1), (b"new paragraph", 2)];

    COMMANDS.iter().find_map(|(phrase, newline_count)| {
        let end = start.checked_add(phrase.len())?;
        let is_phrase = bytes
            .get(start..end)
            .is_some_and(|candidate| candidate.eq_ignore_ascii_case(phrase));
        let has_right_boundary = end == bytes.len()
            || bytes
                .get(end)
                .is_some_and(|byte| is_command_boundary(*byte));

        (is_phrase && has_right_boundary).then_some((phrase.len(), *newline_count))
    })
}

fn is_command_boundary(byte: u8) -> bool {
    matches!(byte, b' ' | b'\r' | b'\n')
}

/// Normalizes whitespace, optionally applies dictation commands, then applies
/// a validated vocabulary map.
///
/// This composes the FR-07.3 stages in exact order:
/// [`normalize_transcript_whitespace`] canonicalizes ASCII spaces, tabs, and
/// line endings; [`apply_dictation_commands`] optionally converts the two
/// supported literal commands; then [`apply_vocabulary_post_corrections`]
/// validates and applies the complete collection using its Unicode 18
/// matching and boundary rules. Recognized commands therefore take precedence
/// over vocabulary entries for those phrases.
///
/// Vocabulary matching scans the normalized transcript after optional command
/// replacement. The existing collection scanner preserves longest-match
/// precedence and does not scan inserted replacement text again during this
/// call. Written values are copied exactly, and the output is not
/// whitespace-normalized a second time.
/// Thus, single-call non-cascading behavior does not make repeated application
/// of this whole function universally idempotent: with `rust` → `ubc` and
/// `ubc` → `UBC`, input `rust` yields `ubc` on the first call and `UBC` on a
/// later call. The vocabulary is never rewritten to force idempotence.
///
/// Vocabulary replacement output is not normalized afterward. The input and
/// borrowed entries are read-only and remain unchanged. Exact spoken command
/// phrases can also occur as ordinary language, so command recognition should
/// remain opt-in rather than acting as a semantic intent classifier.
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
/// use outflow_core::text_cleanup::normalize_whitespace_commands_and_apply_vocabulary;
/// use outflow_core::vocabulary::VocabularyEntryRef;
///
/// let vocabulary = [VocabularyEntryRef {
///     spoken: "rust",
///     written: "Rust",
/// }];
/// let output = normalize_whitespace_commands_and_apply_vocabulary(
///     "  rust  new line  ubc  ",
///     &vocabulary,
///     true,
/// )?;
/// assert_eq!(output, "Rust\nubc");
/// # Ok::<(), outflow_core::vocabulary::VocabularyCollectionError>(())
/// ```
pub fn normalize_whitespace_commands_and_apply_vocabulary(
    input: &str,
    entries: &[VocabularyEntryRef<'_>],
    commands_enabled: bool,
) -> Result<String, VocabularyCollectionError> {
    let normalized = normalize_transcript_whitespace(input);
    let commanded = apply_dictation_commands(&normalized, commands_enabled);
    apply_vocabulary_post_corrections(&commanded, entries)
}

/// Normalizes transcript whitespace and applies vocabulary corrections with
/// dictation-command recognition disabled.
///
/// This preserves the original two-stage API behavior by delegating to
/// [`normalize_whitespace_commands_and_apply_vocabulary`] with `false`.
pub fn normalize_whitespace_and_apply_vocabulary(
    input: &str,
    entries: &[VocabularyEntryRef<'_>],
) -> Result<String, VocabularyCollectionError> {
    normalize_whitespace_commands_and_apply_vocabulary(input, entries, false)
}

#[cfg(test)]
mod tests {
    use super::{
        apply_dictation_commands, normalize_transcript_whitespace,
        normalize_whitespace_and_apply_vocabulary,
        normalize_whitespace_commands_and_apply_vocabulary,
    };
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
    fn applies_dictation_commands_to_a_table_of_exact_fixtures() {
        let fixtures = [
            (
                "disabled command remains literal",
                "new line",
                false,
                "new line",
            ),
            (
                "disabled mixed case remains literal",
                "NEW PARAGRAPH",
                false,
                "NEW PARAGRAPH",
            ),
            ("new line at beginning", "new line start", true, "\nstart"),
            (
                "new line in middle",
                "before new line after",
                true,
                "before\nafter",
            ),
            ("new line at end", "before new line", true, "before\n"),
            (
                "new paragraph at beginning",
                "new paragraph start",
                true,
                "\n\nstart",
            ),
            (
                "new paragraph in middle",
                "before new paragraph after",
                true,
                "before\n\nafter",
            ),
            (
                "new paragraph at end",
                "before new paragraph",
                true,
                "before\n\n",
            ),
            ("uppercase command", "NEW LINE", true, "\n"),
            ("mixed case command", "NeW PaRaGrApH", true, "\n\n"),
            (
                "multiple commands",
                "a new line b new paragraph c",
                true,
                "a\nb\n\nc",
            ),
            (
                "consecutive commands",
                "new line new paragraph",
                true,
                "\n\n\n",
            ),
            (
                "spaces around command are removed",
                "left   new line   right",
                true,
                "left\nright",
            ),
            (
                "leading and trailing separator spaces",
                "  new line  ",
                true,
                "\n",
            ),
            (
                "existing newline before and after is retained",
                "before\nnew line\nafter",
                true,
                "before\n\n\nafter",
            ),
            (
                "existing paragraph break is retained",
                "before\n\nnew line\n\nafter",
                true,
                "before\n\n\n\n\nafter",
            ),
            (
                "command after line boundary",
                "first\nnew line next",
                true,
                "first\n\nnext",
            ),
            (
                "command before line boundary",
                "first new line\nnext",
                true,
                "first\n\nnext",
            ),
            (
                "punctuation outside command is preserved",
                "Hi, new line world!?",
                true,
                "Hi,\nworld!?",
            ),
            (
                "punctuation adjacent to command blocks match",
                "(new line)",
                true,
                "(new line)",
            ),
            (
                "double quotes block match",
                "\"new line\"",
                true,
                "\"new line\"",
            ),
            (
                "single quotes block match",
                "'new paragraph'",
                true,
                "'new paragraph'",
            ),
            (
                "prefix inside a larger word does not match",
                "renew line",
                true,
                "renew line",
            ),
            (
                "suffix inside a larger word does not match",
                "new linebreak",
                true,
                "new linebreak",
            ),
            (
                "phrase inside a larger word does not match",
                "brandnew line",
                true,
                "brandnew line",
            ),
            ("incomplete first word is unchanged", "new", true, "new"),
            (
                "incomplete second word is unchanged",
                "new par",
                true,
                "new par",
            ),
            (
                "two internal spaces are not a command",
                "new  line",
                true,
                "new  line",
            ),
            (
                "tab inside command is not a command",
                "new\tline",
                true,
                "new\tline",
            ),
            (
                "Unicode text and emoji are preserved",
                "café new line 東京 😀",
                true,
                "café\n東京 😀",
            ),
            (
                "Unicode adjacency blocks left boundary",
                "énew line",
                true,
                "énew line",
            ),
            (
                "Unicode adjacency blocks right boundary",
                "new lineé",
                true,
                "new lineé",
            ),
            (
                "nonbreaking space is not a command boundary",
                "\u{00a0}new line",
                true,
                "\u{00a0}new line",
            ),
            (
                "carriage returns are line boundaries",
                "a\rnew line\rb",
                true,
                "a\r\n\rb",
            ),
            (
                "CRLF remains while command creates another break",
                "a\r\nnew line\r\nb",
                true,
                "a\r\n\n\r\nb",
            ),
            (
                "repeated same command",
                "x new line y new line z",
                true,
                "x\ny\nz",
            ),
            ("empty input", "", true, ""),
        ];

        assert!(fixtures.len() >= 25);
        for (name, input, enabled, expected) in fixtures {
            assert_eq!(
                apply_dictation_commands(input, enabled),
                expected,
                "fixture {name}: input {input:?}"
            );
            assert_eq!(input, input.to_owned(), "fixture {name} changed its input");
        }
    }

    #[test]
    fn command_stage_preserves_a_supplied_string_when_disabled() {
        let input = "  new line\tnew paragraph  ";

        assert_eq!(apply_dictation_commands(input, false), input);
    }

    #[test]
    fn composing_pipeline_normalizes_tabs_and_crlf_before_commands() {
        let input = "  before\tnew\tline\r\n  after  ";

        assert_eq!(
            normalize_whitespace_commands_and_apply_vocabulary(input, &[], true),
            Ok("before\n\nafter".to_owned())
        );
    }

    #[test]
    fn commands_take_precedence_over_vocabulary_when_enabled() {
        let entries = vocabulary_refs(&[
            ("new line", "literal line"),
            ("new paragraph", "literal paragraph"),
        ]);

        assert_eq!(
            normalize_whitespace_commands_and_apply_vocabulary(
                "new line and new paragraph",
                &entries,
                true
            ),
            Ok("\nand\n\n".to_owned())
        );
        assert_eq!(
            normalize_whitespace_commands_and_apply_vocabulary(
                "new line and new paragraph",
                &entries,
                false
            ),
            Ok("literal line and literal paragraph".to_owned())
        );
    }

    #[test]
    fn vocabulary_correction_runs_after_commands_and_is_not_normalized_again() {
        let entries = vocabulary_refs(&[("ubc", "  UBC\t")]);

        assert_eq!(
            normalize_whitespace_commands_and_apply_vocabulary(
                "go new line at ubc",
                &entries,
                true
            ),
            Ok("go\nat   UBC\t".to_owned())
        );
    }

    #[test]
    fn new_composition_keeps_longest_match_and_non_cascading_behavior() {
        let entries = vocabulary_refs(&[
            ("new", "Short"),
            ("new york", "New York"),
            ("rust", "ubc"),
            ("ubc", "UBC"),
        ]);

        assert_eq!(
            normalize_whitespace_commands_and_apply_vocabulary("new york and rust", &entries, true),
            Ok("New York and ubc".to_owned())
        );
    }

    #[test]
    fn new_composition_returns_existing_typed_validation_errors() {
        let entries = vocabulary_refs(&[("rust", " \t")]);

        assert_eq!(
            normalize_whitespace_commands_and_apply_vocabulary("new line rust", &entries, true),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            })
        );
    }

    #[test]
    fn new_composition_leaves_input_and_entries_unchanged_and_is_deterministic() {
        let input = String::from("  rust  new line  ubc  ");
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

        let first = normalize_whitespace_commands_and_apply_vocabulary(&input, &entries, true);
        let second = normalize_whitespace_commands_and_apply_vocabulary(&input, &entries, true);

        assert_eq!(first, Ok("Rust\nUBC".to_owned()));
        assert_eq!(second, first);
        assert_eq!(input, original_input);
        assert_eq!(owned_entries, original_entries);
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
