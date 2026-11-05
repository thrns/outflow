//! Conservative transcript whitespace normalization for C-06.
//!
//! This first standalone cleanup stage normalizes horizontal ASCII spacing
//! and line endings while preserving every other character. It performs no
//! punctuation, capitalization, filler, vocabulary, or dictation-command
//! changes and has no storage, network, speech, UI, or global-state effects.

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

#[cfg(test)]
mod tests {
    use super::normalize_transcript_whitespace;

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
}
