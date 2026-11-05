//! Conservative transcript whitespace normalization and composition for C-06.
//!
//! The standalone cleanup stages normalize horizontal ASCII spacing and line
//! endings, optionally recognize two explicit dictation commands, and apply
//! validated vocabulary corrections in FR-07.3 order. A separate composed
//! API also applies conservative punctuation spacing after vocabulary
//! correction while protecting replacement bytes. A further composition may
//! remove supported line-leading fillers outside protected replacements.
//! These partial pipelines do not change capitalization and have no storage,
//! network, speech, UI, or global-state effects.

use std::ops::Range;

use crate::vocabulary::{VocabularyCollectionError, VocabularyEntryRef};
use crate::vocabulary_post_correction::{
    apply_vocabulary_post_corrections, apply_vocabulary_post_corrections_with_spans,
};

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

/// Removes obvious ASCII spaces and tabs before supported punctuation marks.
///
/// The supported marks are `, . ! ? ; :`. A horizontal ASCII whitespace run
/// is removed only when the preceding non-whitespace character is Unicode
/// alphabetic, the mark is followed by whitespace or the end of input, and
/// the adjacent alphanumeric token contains no numeric character. Spaced
/// punctuation or quote sequences are left unchanged as ambiguous. The
/// function preserves all words, marks, capitalization, numbers, Unicode
/// characters, emoji, and existing line and paragraph breaks. It never adds
/// or changes punctuation or inserts spacing after punctuation.
///
/// This intentionally narrow rule is a standalone partial FR-07.1 / FR-07.3
/// stage; it is not complete punctuation restoration. The composed
/// [`normalize_whitespace_commands_vocabulary_and_punctuation`] API applies
/// this same rule after vocabulary correction while preserving replacement
/// spans.
///
/// The scan is deterministic, idempotent, and linear-time. It reads but does
/// not modify the input and uses no global state.
///
/// # Examples
///
/// ```
/// use outflow_core::text_cleanup::normalize_punctuation_spacing;
///
/// assert_eq!(normalize_punctuation_spacing("Hello , world !"), "Hello, world!");
/// assert_eq!(normalize_punctuation_spacing("Version 3 . 14"), "Version 3 . 14");
/// ```
pub fn normalize_punctuation_spacing(input: &str) -> String {
    normalize_punctuation_spacing_with_protection(input, &[]).text
}

/// Removes a narrowly recognized filler at the beginning of each line.
///
/// When `enabled` is `true`, this removes only an ASCII case-insensitive
/// `um, ` or `uh, ` prefix when the next character is alphabetic. The next
/// word and all other text are copied exactly. If another supported filler
/// token follows immediately, the complete line is preserved rather than
/// guessing which hesitation was intentional. A line with leading spaces or
/// tabs is not considered to start with a filler.
///
/// The caller should pass normalized transcript input: recognized phrases
/// contain exactly one ASCII space after the comma. This function does not
/// normalize whitespace. It recognizes line starts after LF and CR characters
/// while preserving those bytes and all unrelated UTF-8 text. It is a
/// standalone partial FR-07.2 stage. The composed
/// [`process_transcript_with_optional_fillers`] API uses the same recognition
/// policy after punctuation spacing and protects vocabulary replacement
/// ranges.
///
/// With `enabled` set to `false`, the input is returned unchanged, which
/// represents Verbatim mode or disabled filler removal. The function is
/// deterministic and idempotent and has no storage, network, or global-state
/// effects.
///
/// # Examples
///
/// ```
/// use outflow_core::text_cleanup::remove_leading_isolated_fillers;
///
/// assert_eq!(remove_leading_isolated_fillers("um, Rust.", true), "Rust.");
/// assert_eq!(remove_leading_isolated_fillers("um, Rust.", false), "um, Rust.");
/// assert_eq!(remove_leading_isolated_fillers("um, uh, yes", true), "um, uh, yes");
/// ```
pub fn remove_leading_isolated_fillers(input: &str, enabled: bool) -> String {
    remove_leading_isolated_fillers_with_protection(input, &[], enabled).text
}

/// Recognizes the existing conservative filler prefix and returns its local
/// byte range. Both the standalone and protected-aware paths use this policy
/// so integration cannot broaden recognition by accident.
fn line_leading_isolated_filler_range(line: &str) -> Option<Range<usize>> {
    let prefix_length = if line
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("um, "))
        || line
            .get(..4)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("uh, "))
    {
        4
    } else {
        return None;
    };

    let remaining = &line[prefix_length..];
    if !remaining.chars().next().is_some_and(char::is_alphabetic)
        || starts_with_supported_filler_comma(remaining)
    {
        return None;
    }

    Some(0..prefix_length)
}

fn starts_with_supported_filler_comma(text: &str) -> bool {
    text.as_bytes().get(..3).is_some_and(|prefix| {
        prefix[..2].eq_ignore_ascii_case(b"um") && prefix[2] == b','
            || prefix[..2].eq_ignore_ascii_case(b"uh") && prefix[2] == b','
    })
}

/// Text and adjusted replacement provenance after optional filler deletion.
///
/// Ranges are private and always index `text`; deletions that precede a
/// replacement shift its offsets, while intersecting candidates are retained.
#[derive(Debug)]
struct ProtectedFillerOutput {
    text: String,
    protected_ranges: Vec<Range<usize>>,
}

fn remove_leading_isolated_fillers_with_protection(
    input: &str,
    protected_ranges: &[Range<usize>],
    enabled: bool,
) -> ProtectedFillerOutput {
    if !enabled {
        return ProtectedFillerOutput {
            text: input.to_owned(),
            protected_ranges: protected_ranges.to_vec(),
        };
    }

    let mut output = String::with_capacity(input.len());
    let mut removed_ranges = Vec::new();
    let mut protected_range_index = 0;
    let mut line_start = 0;

    for line in input.split_inclusive(['\n', '\r']) {
        if let Some(local_range) = line_leading_isolated_filler_range(line) {
            let candidate = (line_start + local_range.start)..(line_start + local_range.end);
            if protected_range_intersects(
                protected_ranges,
                &mut protected_range_index,
                candidate.clone(),
            ) {
                // Do not partially trim a candidate if any of its bytes came
                // from a written replacement.
                output.push_str(line);
            } else {
                output.push_str(&line[..local_range.start]);
                output.push_str(&line[local_range.end..]);
                removed_ranges.push(candidate);
            }
        } else {
            output.push_str(line);
        }

        line_start += line.len();
    }

    let adjusted_protected_ranges =
        adjust_protected_ranges_after_deletions(protected_ranges, &removed_ranges);

    ProtectedFillerOutput {
        text: output,
        protected_ranges: adjusted_protected_ranges,
    }
}

/// Punctuation output and its adjusted internal provenance ranges.
///
/// The ranges are kept private: callers of the composed pipeline receive only
/// final text, never offsets that could be mistaken for offsets in an earlier
/// stage.
#[derive(Debug)]
struct ProtectedPunctuationOutput {
    text: String,
    protected_ranges: Vec<Range<usize>>,
}

fn normalize_punctuation_spacing_with_protection(
    input: &str,
    protected_ranges: &[Range<usize>],
) -> ProtectedPunctuationOutput {
    let characters: Vec<char> = input.chars().collect();
    let mut byte_offsets = input
        .char_indices()
        .map(|(byte_offset, _)| byte_offset)
        .collect::<Vec<_>>();
    byte_offsets.push(input.len());
    let token_contains_number = alphanumeric_token_number_flags(&characters);

    let mut output = String::with_capacity(input.len());
    let mut removed_ranges = Vec::new();
    let mut protected_range_index = 0;
    let mut index = 0;

    while index < characters.len() {
        if is_horizontal_ascii_whitespace(characters[index]) {
            let whitespace_start = index;
            while characters
                .get(index)
                .is_some_and(|character| is_horizontal_ascii_whitespace(*character))
            {
                index += 1;
            }

            if should_remove_punctuation_spacing(
                &characters,
                &byte_offsets,
                &token_contains_number,
                whitespace_start,
                index,
                protected_ranges,
                &mut protected_range_index,
            ) {
                removed_ranges.push(byte_offsets[whitespace_start]..byte_offsets[index]);
            } else {
                output.extend(characters[whitespace_start..index].iter().copied());
            }
        } else {
            output.push(characters[index]);
            index += 1;
        }
    }

    let adjusted_protected_ranges =
        adjust_protected_ranges_after_deletions(protected_ranges, &removed_ranges);

    ProtectedPunctuationOutput {
        text: output,
        protected_ranges: adjusted_protected_ranges,
    }
}

fn should_remove_punctuation_spacing(
    characters: &[char],
    byte_offsets: &[usize],
    token_contains_number: &[bool],
    whitespace_start: usize,
    mark_index: usize,
    protected_ranges: &[Range<usize>],
    protected_range_index: &mut usize,
) -> bool {
    let Some(&mark) = characters.get(mark_index) else {
        return false;
    };
    if !is_supported_punctuation(mark)
        || !characters
            .get(mark_index + 1)
            .is_none_or(|character| character.is_whitespace())
    {
        return false;
    }

    let Some(preceding_index) = whitespace_start.checked_sub(1) else {
        return false;
    };
    if !characters[preceding_index].is_alphabetic()
        || adjacent_alphanumeric_token_contains_number(token_contains_number, preceding_index)
        || is_ambiguous_punctuation_sequence(characters, mark_index)
        || protected_range_intersects(
            protected_ranges,
            protected_range_index,
            byte_offsets[whitespace_start]..byte_offsets[mark_index],
        )
    {
        return false;
    }

    true
}

fn adjacent_alphanumeric_token_contains_number(token_contains_number: &[bool], end: usize) -> bool {
    token_contains_number.get(end).copied().unwrap_or(false)
}

fn alphanumeric_token_number_flags(characters: &[char]) -> Vec<bool> {
    let mut token_contains_number = false;

    characters
        .iter()
        .map(|character| {
            if character.is_alphanumeric() {
                token_contains_number |= character.is_numeric();
            } else {
                token_contains_number = false;
            }

            token_contains_number
        })
        .collect()
}

fn protected_range_intersects(
    protected_ranges: &[Range<usize>],
    range_index: &mut usize,
    candidate: Range<usize>,
) -> bool {
    while protected_ranges
        .get(*range_index)
        .is_some_and(|protected| protected.end <= candidate.start)
    {
        *range_index += 1;
    }

    protected_ranges
        .get(*range_index)
        .is_some_and(|protected| protected.start < candidate.end)
}

fn adjust_protected_ranges_after_deletions(
    protected_ranges: &[Range<usize>],
    removed_ranges: &[Range<usize>],
) -> Vec<Range<usize>> {
    let mut removed_index = 0;
    let mut removed_bytes = 0;

    protected_ranges
        .iter()
        .map(|protected| {
            while removed_ranges
                .get(removed_index)
                .is_some_and(|removed| removed.end <= protected.start)
            {
                let removed = &removed_ranges[removed_index];
                removed_bytes += removed.end - removed.start;
                removed_index += 1;
            }

            (protected.start - removed_bytes)..(protected.end - removed_bytes)
        })
        .collect()
}

fn is_ambiguous_punctuation_sequence(characters: &[char], mark_index: usize) -> bool {
    let mut following_index = mark_index + 1;
    while characters
        .get(following_index)
        .is_some_and(|character| is_horizontal_ascii_whitespace(*character))
    {
        following_index += 1;
    }

    characters
        .get(following_index)
        .is_some_and(|character| is_ambiguous_punctuation_delimiter(*character))
}

fn is_horizontal_ascii_whitespace(character: char) -> bool {
    matches!(character, ' ' | '\t')
}

fn is_supported_punctuation(character: char) -> bool {
    matches!(character, ',' | '.' | '!' | '?' | ';' | ':')
}

fn is_ambiguous_punctuation_delimiter(character: char) -> bool {
    character.is_ascii_punctuation()
        || matches!(
            character,
            '…' | '“' | '”' | '‘' | '’' | '«' | '»' | '‹' | '›'
        )
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

/// Normalizes whitespace, optionally applies dictation commands and
/// vocabulary corrections, then removes only unambiguous punctuation spacing.
///
/// The stages run in FR-07.3 order:
/// [`normalize_transcript_whitespace`], [`apply_dictation_commands`],
/// [`apply_vocabulary_post_corrections_with_spans`], then the same punctuation
/// spacing rule used by [`normalize_punctuation_spacing`]. The vocabulary
/// stage's UTF-8 byte ranges are passed directly to punctuation formatting.
/// Candidate whitespace runs that overlap a written replacement are kept
/// whole, so every written replacement remains byte-for-byte unchanged.
/// Eligible spacing outside replacements can still be corrected, including
/// when the preceding alphabetic character is part of a replacement.
///
/// Vocabulary replacements are non-cascading within one call. Reapplying the
/// complete pipeline is not universally idempotent: for example, `rust` →
/// `ubc` and `ubc` → `UBC` produces `ubc` on the first call and `UBC` on a
/// later call. The configured vocabulary is never rewritten to force
/// idempotence. This API returns final text only; it does not expose byte
/// ranges from an intermediate or formatted string.
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
/// use outflow_core::text_cleanup::normalize_whitespace_commands_vocabulary_and_punctuation;
/// use outflow_core::vocabulary::{VocabularyCollectionError, VocabularyEntryRef};
///
/// let vocabulary = [
///     VocabularyEntryRef { spoken: "rust", written: "Rust , Edition" },
///     VocabularyEntryRef { spoken: "ubc", written: "UBC" },
/// ];
/// let output = normalize_whitespace_commands_vocabulary_and_punctuation(
///     "rust , at ubc .",
///     &vocabulary,
///     false,
/// )?;
/// assert_eq!(output, "Rust , Edition, at UBC.");
/// # Ok::<(), VocabularyCollectionError>(())
/// ```
pub fn normalize_whitespace_commands_vocabulary_and_punctuation(
    input: &str,
    entries: &[VocabularyEntryRef<'_>],
    commands_enabled: bool,
) -> Result<String, VocabularyCollectionError> {
    let formatted = normalize_whitespace_commands_vocabulary_and_punctuation_with_spans(
        input,
        entries,
        commands_enabled,
    )?;

    // Updated provenance is private to this stage; callers receive final text
    // only, so intermediate byte offsets cannot escape as stale metadata.
    Ok(formatted.text)
}

/// Runs the current cleanup order with conservative filler removal controlled
/// independently from opt-in dictation-command recognition.
///
/// The stages are whitespace normalization, optional dictation commands,
/// vocabulary post-correction, protected punctuation spacing, and optional
/// protected-aware filler removal. A filler prefix is removed only when
/// `fillers_enabled` is `true`; `false` preserves it exactly. Written
/// vocabulary replacements are never modified, including replacements that
/// themselves begin with a filler-like phrase. The flag controls this stage
/// only and does not select the full Clean/Verbatim settings policy.
///
/// This function returns final text only; intermediate byte provenance stays
/// private because later stages can shift offsets. A single call is
/// deterministic, but repeated calls are not universally idempotent when a
/// vocabulary replacement matches another entry on a later call.
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
/// use outflow_core::text_cleanup::process_transcript_with_optional_fillers;
/// use outflow_core::vocabulary::VocabularyEntryRef;
///
/// let vocabulary = [VocabularyEntryRef { spoken: "rust", written: "Rust" }];
/// let output = process_transcript_with_optional_fillers(
///     "um, rust .",
///     &vocabulary,
///     false,
///     true,
/// )?;
/// assert_eq!(output, "Rust.");
/// # Ok::<(), outflow_core::vocabulary::VocabularyCollectionError>(())
/// ```
pub fn process_transcript_with_optional_fillers(
    input: &str,
    entries: &[VocabularyEntryRef<'_>],
    commands_enabled: bool,
    fillers_enabled: bool,
) -> Result<String, VocabularyCollectionError> {
    let formatted = normalize_whitespace_commands_vocabulary_and_punctuation_with_spans(
        input,
        entries,
        commands_enabled,
    )?;
    let ProtectedFillerOutput {
        text,
        protected_ranges: _adjusted_internal_ranges,
    } = remove_leading_isolated_fillers_with_protection(
        &formatted.text,
        &formatted.protected_ranges,
        fillers_enabled,
    );

    Ok(text)
}

fn normalize_whitespace_commands_vocabulary_and_punctuation_with_spans(
    input: &str,
    entries: &[VocabularyEntryRef<'_>],
    commands_enabled: bool,
) -> Result<ProtectedPunctuationOutput, VocabularyCollectionError> {
    let normalized = normalize_transcript_whitespace(input);
    let commanded = apply_dictation_commands(&normalized, commands_enabled);
    let corrected = apply_vocabulary_post_corrections_with_spans(&commanded, entries)?;
    Ok(normalize_punctuation_spacing_with_protection(
        &corrected.text,
        &corrected.protected_ranges,
    ))
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
        apply_dictation_commands, normalize_punctuation_spacing,
        normalize_punctuation_spacing_with_protection, normalize_transcript_whitespace,
        normalize_whitespace_and_apply_vocabulary,
        normalize_whitespace_commands_and_apply_vocabulary,
        normalize_whitespace_commands_vocabulary_and_punctuation,
        normalize_whitespace_commands_vocabulary_and_punctuation_with_spans,
        process_transcript_with_optional_fillers, remove_leading_isolated_fillers,
        remove_leading_isolated_fillers_with_protection,
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

    fn assert_protected_pipeline_fixture(
        input: &str,
        entries: &[VocabularyEntryRef<'_>],
        commands_enabled: bool,
        expected: &str,
    ) -> String {
        let actual = normalize_whitespace_commands_vocabulary_and_punctuation(
            input,
            entries,
            commands_enabled,
        )
        .expect("the fixture vocabulary is valid");

        let normalized = normalize_transcript_whitespace(input);
        let commanded = apply_dictation_commands(&normalized, commands_enabled);
        let corrected =
            crate::vocabulary_post_correction::apply_vocabulary_post_corrections_with_spans(
                &commanded, entries,
            )
            .expect("the fixture vocabulary is valid");
        let formatted = normalize_punctuation_spacing_with_protection(
            &corrected.text,
            &corrected.protected_ranges,
        );

        assert_eq!(actual, expected);
        assert_eq!(formatted.text, actual);
        assert_eq!(
            formatted.protected_ranges.len(),
            corrected.protected_ranges.len(),
            "every inserted replacement must retain its ordered provenance"
        );

        for (index, (source_range, final_range)) in corrected
            .protected_ranges
            .iter()
            .zip(&formatted.protected_ranges)
            .enumerate()
        {
            assert_eq!(
                &formatted.text[final_range.clone()],
                &corrected.text[source_range.clone()],
                "protected replacement at provenance index {index} changed"
            );
        }

        actual
    }

    #[test]
    fn removes_only_supported_line_leading_fillers_in_clean_mode() {
        let fixtures = [
            ("empty enabled", "", true, ""),
            ("empty disabled", "", false, ""),
            (
                "disabled um",
                "um, so I went home.",
                false,
                "um, so I went home.",
            ),
            ("disabled uh", "Uh, I use Rust.", false, "Uh, I use Rust."),
            (
                "lowercase um",
                "um, so I went home.",
                true,
                "so I went home.",
            ),
            ("lowercase uh", "uh, I use Rust.", true, "I use Rust."),
            (
                "uppercase um",
                "UM, So I went home.",
                true,
                "So I went home.",
            ),
            (
                "mixed-case um",
                "uM, café is nearby.",
                true,
                "café is nearby.",
            ),
            ("title-case um", "Um, I am ready.", true, "I am ready."),
            ("uppercase uh", "UH, We can begin.", true, "We can begin."),
            (
                "mixed-case uh",
                "uH, Rust is local.",
                true,
                "Rust is local.",
            ),
            (
                "filler after first line",
                "first line\num, second line",
                true,
                "first line\nsecond line",
            ),
            (
                "both fillers at separate line starts",
                "um, first\nuh, second",
                true,
                "first\nsecond",
            ),
            (
                "filler after blank paragraph",
                "first\n\nUh, second",
                true,
                "first\n\nsecond",
            ),
            (
                "filler in sentence middle",
                "I, um, went home.",
                true,
                "I, um, went home.",
            ),
            (
                "fillers in sentence middle",
                "I said um, not uh.",
                true,
                "I said um, not uh.",
            ),
            ("filler at sentence end", "I went, um,", true, "I went, um,"),
            (
                "filler at end of final line",
                "I said uh.",
                true,
                "I said uh.",
            ),
            (
                "repeated different fillers",
                "um, uh, yes",
                true,
                "um, uh, yes",
            ),
            (
                "repeated different fillers reversed",
                "Uh, UM, yes",
                true,
                "Uh, UM, yes",
            ),
            ("repeated um", "um, um, okay", true, "um, um, okay"),
            ("repeated uh", "uh, uh, yes", true, "uh, uh, yes"),
            (
                "repeated incomplete second filler",
                "um, uh,",
                true,
                "um, uh,",
            ),
            ("incomplete um token", "um", true, "um"),
            ("incomplete uh token", "uh", true, "uh"),
            ("comma without space", "um,yes", true, "um,yes"),
            ("comma with no following text", "uh, ", true, "uh, "),
            ("two spaces after comma", "um,  yes", true, "um,  yes"),
            ("tab after comma", "uh,\tyes", true, "uh,\tyes"),
            (
                "number after comma",
                "um, 2026 was different.",
                true,
                "um, 2026 was different.",
            ),
            (
                "decimal after comma",
                "uh, 3.14 is known.",
                true,
                "uh, 3.14 is known.",
            ),
            (
                "abbreviation preserved",
                "um, e.g. this one.",
                true,
                "e.g. this one.",
            ),
            (
                "title preserved",
                "uh, Dr. Smith spoke.",
                true,
                "Dr. Smith spoke.",
            ),
            (
                "proper noun preserved",
                "um, North Parkade is nearby.",
                true,
                "North Parkade is nearby.",
            ),
            ("acronym preserved", "uh, UBC!", true, "UBC!"),
            ("quoted filler phrase", "\"um, yes\"", true, "\"um, yes\""),
            ("curly quoted next word", "um, “yes”", true, "um, “yes”"),
            ("single quoted next word", "uh, 'yes'", true, "uh, 'yes'"),
            ("parenthesized next word", "um, (yes)", true, "um, (yes)"),
            ("punctuation next", "uh, !go", true, "uh, !go"),
            ("period next", "um, .", true, "um, ."),
            ("leading space", " um, yes", true, " um, yes"),
            ("leading tab", "\tum, yes", true, "\tum, yes"),
            ("multiple leading spaces", "  uh, okay", true, "  uh, okay"),
            (
                "newlines and paragraph bytes preserved",
                "um, first\n\nuh, second\n",
                true,
                "first\n\nsecond\n",
            ),
            (
                "CRLF bytes preserved",
                "um, first\r\nuh, second\r\n",
                true,
                "first\r\nsecond\r\n",
            ),
            (
                "standalone CR line break preserved",
                "um, first\ruh, second\r",
                true,
                "first\rsecond\r",
            ),
            (
                "precomposed accent preserved",
                "um, café is nearby.",
                true,
                "café is nearby.",
            ),
            (
                "combining text preserved",
                "uh, cafe\u{301} is nearby.",
                true,
                "cafe\u{301} is nearby.",
            ),
            (
                "non-ASCII filler spelling unchanged",
                "úm, yes",
                true,
                "úm, yes",
            ),
            ("Japanese next word", "um, 東京です。", true, "東京です。"),
            (
                "emoji immediately after comma",
                "uh, 😀 yes",
                true,
                "uh, 😀 yes",
            ),
            ("emoji after next word", "um, café 😀", true, "café 😀"),
            (
                "non-ASCII space after comma",
                "um,\u{a0}yes",
                true,
                "um,\u{a0}yes",
            ),
            (
                "unrelated lines unchanged",
                "plain text\n\tindented text\n",
                true,
                "plain text\n\tindented text\n",
            ),
        ];

        assert_eq!(fixtures.len(), 55);
        for (name, input, enabled, expected) in fixtures {
            let actual = remove_leading_isolated_fillers(input, enabled);
            assert_eq!(actual, expected, "fixture: {name}");
            assert_eq!(
                remove_leading_isolated_fillers(input, enabled),
                actual,
                "determinism fixture: {name}"
            );
            assert_eq!(
                remove_leading_isolated_fillers(&actual, enabled),
                actual,
                "idempotence fixture: {name}"
            );
        }
    }

    #[test]
    fn leading_filler_removal_does_not_modify_original_input() {
        let input = String::from("um, café 😀\nUh, 東京");
        let snapshot = input.clone();

        assert_eq!(
            remove_leading_isolated_fillers(&input, true),
            "café 😀\n東京"
        );
        assert_eq!(input, snapshot);
    }

    #[test]
    fn normalizes_punctuation_spacing_exact_output_fixtures() {
        let fixtures = [
            ("empty input", "", ""),
            ("plain text", "hello world", "hello world"),
            ("already correct", "Hello, world!", "Hello, world!"),
            ("comma", "Hello ,", "Hello,"),
            ("period", "Hello .", "Hello."),
            ("exclamation", "Hello !", "Hello!"),
            ("question", "Ready ?", "Ready?"),
            ("semicolon", "Wait ;", "Wait;"),
            ("colon", "Note :", "Note:"),
            ("repeated spaces", "Hello   , world", "Hello, world"),
            ("repeated tabs", "Hello\t\t!", "Hello!"),
            ("mixed horizontal whitespace", "Hello \t ?", "Hello?"),
            (
                "multiple fixes",
                "Hello , world ! Are you ready ?",
                "Hello, world! Are you ready?",
            ),
            (
                "supported punctuation stays unchanged",
                "a,b.c!d?e;f:g",
                "a,b.c!d?e;f:g",
            ),
            ("period ellipsis", "Hello... world", "Hello... world"),
            (
                "spaced ellipsis is ambiguous",
                "Hello . . . world",
                "Hello . . . world",
            ),
            (
                "spaced punctuation pair",
                "Hello , ; world",
                "Hello , ; world",
            ),
            ("spaced question pair", "Hello ! ? world", "Hello ! ? world"),
            ("decimal", "3 . 14", "3 . 14"),
            ("year-like numbers", "2026 , 2027", "2026 , 2027"),
            ("tabbed year-like numbers", "2026\t,\t2027", "2026\t,\t2027"),
            ("date", "2026-11-04", "2026-11-04"),
            ("spaced date", "2026 - 11 - 04", "2026 - 11 - 04"),
            (
                "version",
                "Version 3 . 14 remains unchanged.",
                "Version 3 . 14 remains unchanged.",
            ),
            ("version token", "v1 . 2", "v1 . 2"),
            (
                "number inside alphanumeric token",
                "v1beta , release",
                "v1beta , release",
            ),
            ("number in word token", "alpha2 , beta", "alpha2 , beta"),
            ("numeric token punctuation", "1 . 2 . 3", "1 . 2 . 3"),
            ("abbreviation punctuation", "e.g. , value", "e.g. , value"),
            (
                "abbreviation in sentence",
                "Dr. Smith , said",
                "Dr. Smith, said",
            ),
            (
                "proper noun capitalization",
                "North Parkade , UBC",
                "North Parkade, UBC",
            ),
            (
                "Unicode precomposed letters",
                "café , 東京 !",
                "café, 東京!",
            ),
            ("Japanese letters", "東京 , 世界 !", "東京, 世界!"),
            ("Greek letters", "γειά , κόσμε !", "γειά, κόσμε!"),
            (
                "combining mark remains",
                "cafe\u{301} , word",
                "cafe\u{301} , word",
            ),
            (
                "combining mark with punctuation",
                "e\u{301} ,",
                "e\u{301} ,",
            ),
            ("emoji before punctuation", "Hello 😀 ,", "Hello 😀 ,"),
            ("emoji after punctuation", "Hello , 😀", "Hello, 😀"),
            ("quote closes punctuation", "“Hello ,”", "“Hello ,”"),
            ("spaced quote is ambiguous", "“Hello , ”", "“Hello , ”"),
            ("quoted phrase comma", "“Hello , world”", "“Hello, world”"),
            (
                "parenthesis after mark is ambiguous",
                "Hello , )",
                "Hello , )",
            ),
            (
                "space before punctuation and newline",
                "Hello ,\nworld !",
                "Hello,\nworld!",
            ),
            (
                "paragraph break retained",
                "Hello ,\n\nworld .",
                "Hello,\n\nworld.",
            ),
            (
                "multiple paragraph breaks retained",
                "first ,\n\n\nlast !",
                "first,\n\n\nlast!",
            ),
            ("CRLF retained", "Hello ,\r\nworld !", "Hello,\r\nworld!"),
            (
                "newline before mark blocks edit",
                "Hello \n, world",
                "Hello \n, world",
            ),
            (
                "spaces after mark are preserved",
                "Hello ,  world",
                "Hello,  world",
            ),
            (
                "unrelated leading and trailing spaces",
                "  Hello ,  ",
                "  Hello,  ",
            ),
            (
                "unrelated tabs are preserved",
                "\tHello ,\tworld",
                "\tHello,\tworld",
            ),
            (
                "nonbreaking space is preserved",
                "Hello\u{a0} , world",
                "Hello\u{a0} , world",
            ),
            (
                "em space is preserved",
                "Hello \u{2003}, world",
                "Hello \u{2003}, world",
            ),
            (
                "non-ASCII whitespace before mark",
                "Hello\u{a0},",
                "Hello\u{a0},",
            ),
            ("punctuation followed by mark", "Hello ,\t?", "Hello ,\t?"),
            ("punctuation followed by period", "Hello , .", "Hello , ."),
            ("punctuation followed by quote", "Hello , \"", "Hello , \""),
            ("newline separates punctuation", "Hello ,\n?", "Hello,\n?"),
            (
                "unsupported dash unchanged",
                "Hello - world",
                "Hello - world",
            ),
            ("punctuation only", ", . ! ? ; :", ", . ! ? ; :"),
            ("punctuation at start", " , hello", " , hello"),
            ("punctuation adjacent to number", "abc 2 ,", "abc 2 ,"),
            (
                "combining text elsewhere stays exact",
                "e\u{301} 😀  東京 ,",
                "e\u{301} 😀  東京,",
            ),
        ];

        assert_eq!(fixtures.len(), 62);
        for (name, input, expected) in fixtures {
            let actual = normalize_punctuation_spacing(input);
            assert_eq!(actual, expected, "fixture: {name}");
            assert_eq!(
                normalize_punctuation_spacing(&actual),
                actual,
                "idempotence fixture: {name}"
            );
            assert_eq!(
                normalize_punctuation_spacing(input),
                actual,
                "determinism fixture: {name}"
            );
        }
    }

    #[test]
    fn punctuation_spacing_does_not_modify_original_input() {
        let original = String::from("Hello , café 😀\nVersion 3 . 14");
        let snapshot = original.clone();

        let output = normalize_punctuation_spacing(&original);

        assert_eq!(output, "Hello, café 😀\nVersion 3 . 14");
        assert_eq!(original, snapshot);
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

    #[test]
    fn protected_composed_pipeline_handles_empty_and_ordinary_text() {
        assert_protected_pipeline_fixture("", &[], false, "");
        assert_protected_pipeline_fixture("  \t \r\n", &[], false, "\n");
        assert_protected_pipeline_fixture("  hello\t world.  ", &[], false, "hello world.");
        assert_protected_pipeline_fixture("Hello , world !", &[], false, "Hello, world!");
    }

    #[test]
    fn protected_composed_pipeline_preserves_the_requested_vocabulary_example() {
        let entries = vocabulary_refs(&[("rust", "Rust , Edition"), ("ubc", "UBC")]);

        assert_protected_pipeline_fixture(
            "rust , at ubc .",
            &entries,
            false,
            "Rust , Edition, at UBC.",
        );
    }

    #[test]
    fn protected_composed_pipeline_preserves_exact_leading_trailing_and_intersecting_spaces() {
        let trailing = vocabulary_refs(&[("rust", "Rust ")]);
        assert_protected_pipeline_fixture("rust,", &trailing, false, "Rust ,");

        let leading = vocabulary_refs(&[("rust", " Rust")]);
        assert_protected_pipeline_fixture("rust ,", &leading, false, " Rust,");

        // The source contributes one space after the replacement's protected
        // trailing space. Since the complete run intersects the range, neither
        // byte is deleted.
        assert_protected_pipeline_fixture("rust  ,", &trailing, false, "Rust  ,");

        let newlines = vocabulary_refs(&[("rust", "Rust ,\nEdition ")]);
        assert_protected_pipeline_fixture("rust !", &newlines, false, "Rust ,\nEdition  !");
    }

    #[test]
    fn protected_composed_pipeline_handles_multiple_repeated_and_unicode_replacements() {
        let entries = vocabulary_refs(&[("rust", "Rust , Edition"), ("ubc", "UBC")]);
        assert_protected_pipeline_fixture(
            "rust , and ubc !",
            &entries,
            false,
            "Rust , Edition, and UBC!",
        );

        let repeated = vocabulary_refs(&[("rust", "Same , Phrase")]);
        assert_protected_pipeline_fixture(
            "rust, then rust !",
            &repeated,
            false,
            "Same , Phrase, then Same , Phrase!",
        );

        assert_protected_pipeline_fixture(
            "😀 rust , café , 東京 !",
            &entries,
            false,
            "😀 Rust , Edition, café, 東京!",
        );

        // A combining mark is not an alphabetic character for the existing
        // spacing rule, while the later correction outside the replacement is.
        assert_protected_pipeline_fixture(
            "cafe\u{301} , rust ,",
            &entries,
            false,
            "cafe\u{301} , Rust , Edition,",
        );
    }

    #[test]
    fn protected_composed_pipeline_retains_paragraph_breaks_and_numeric_ambiguity() {
        assert_protected_pipeline_fixture("café ,\n\n東京 !", &[], false, "café,\n\n東京!");
        assert_protected_pipeline_fixture(
            "Version 3 . 14 and v1 . 2; e.g. , value. Dr. Smith , said! Wait , ; Hello , world",
            &[],
            false,
            "Version 3 . 14 and v1 . 2; e.g. , value. Dr. Smith, said! Wait , ; Hello, world",
        );
        assert_protected_pipeline_fixture("“Hello ,”", &[], false, "“Hello ,”");
        assert_protected_pipeline_fixture("“Hello , ”", &[], false, "“Hello , ”");
        assert_protected_pipeline_fixture("“Hello , world”", &[], false, "“Hello, world”");
    }

    #[test]
    fn protected_composed_pipeline_runs_commands_before_vocabulary_correction() {
        let entries = vocabulary_refs(&[("new line", "Literal"), ("rust", "Rust")]);

        assert_protected_pipeline_fixture("a new line b", &entries, true, "a\nb");
        assert_protected_pipeline_fixture("a new line b", &entries, false, "a Literal b");
        assert_protected_pipeline_fixture("rust new line rust .", &entries, true, "Rust\nRust.");
        assert_protected_pipeline_fixture(
            "rust new line rust .",
            &entries,
            false,
            "Rust Literal Rust.",
        );
    }

    #[test]
    fn protected_composed_pipeline_keeps_longest_match_and_non_cascading_rules() {
        let entries = vocabulary_refs(&[
            ("new", "Short"),
            ("new york", "New York"),
            ("rust", "ubc"),
            ("ubc", "UBC , Place"),
        ]);

        assert_protected_pipeline_fixture(
            "new york and rust .",
            &entries,
            false,
            "New York and ubc.",
        );
    }

    #[test]
    fn protected_composed_pipeline_reports_existing_validation_errors() {
        let invalid = vocabulary_refs(&[("rust", " \t")]);
        assert_eq!(
            normalize_whitespace_commands_vocabulary_and_punctuation("rust .", &invalid, false),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            })
        );

        let duplicate = vocabulary_refs(&[("café", "Cafe"), ("cafe\u{301}", "Other")]);
        assert_eq!(
            normalize_whitespace_commands_vocabulary_and_punctuation("café .", &duplicate, false),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn protected_composed_pipeline_is_deterministic_and_does_not_mutate_inputs() {
        let input = String::from("😀 rust , then rust !");
        let vocabulary = vec![(String::from("rust"), String::from("Same , Phrase"))];
        let input_before = input.clone();
        let vocabulary_before = vocabulary.clone();
        let entries = vocabulary
            .iter()
            .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();

        let first =
            normalize_whitespace_commands_vocabulary_and_punctuation(&input, &entries, false);
        let second =
            normalize_whitespace_commands_vocabulary_and_punctuation(&input, &entries, false);

        assert_eq!(
            first,
            Ok("😀 Same , Phrase, then Same , Phrase!".to_owned())
        );
        assert_eq!(second, first);
        assert_eq!(input, input_before);
        assert_eq!(vocabulary, vocabulary_before);
    }

    #[test]
    fn protected_composed_pipeline_retains_existing_api_behavior() {
        assert_eq!(normalize_punctuation_spacing("Hello ,"), "Hello,");
        assert_eq!(
            normalize_whitespace_commands_and_apply_vocabulary("Hello ,", &[], false),
            Ok("Hello ,".to_owned())
        );
        assert_eq!(
            normalize_whitespace_and_apply_vocabulary("Hello ,", &[]),
            Ok("Hello ,".to_owned())
        );
    }

    #[test]
    fn protected_composed_pipeline_documents_non_idempotent_vocabulary_cascades() {
        let entries = vocabulary_refs(&[("rust", "ubc"), ("ubc", "UBC")]);
        let first =
            normalize_whitespace_commands_vocabulary_and_punctuation("rust ,", &entries, false)
                .expect("the test vocabulary is valid");
        let second =
            normalize_whitespace_commands_vocabulary_and_punctuation(&first, &entries, false)
                .expect("the test vocabulary is valid");

        assert_eq!(first, "ubc,");
        assert_eq!(second, "UBC,");
        assert_ne!(first, second);
    }

    #[test]
    fn optional_filler_pipeline_handles_empty_and_enabled_disabled_modes() {
        let entries = vocabulary_refs(&[("rust", "Rust"), ("ubc", "UBC")]);

        assert_eq!(
            process_transcript_with_optional_fillers("", &[], false, true),
            Ok(String::new())
        );
        assert_eq!(
            process_transcript_with_optional_fillers("um, rust at ubc.", &entries, false, true),
            Ok("Rust at UBC.".to_owned())
        );
        assert_eq!(
            process_transcript_with_optional_fillers("um, rust at ubc.", &entries, false, false),
            Ok("um, Rust at UBC.".to_owned())
        );

        let single = vocabulary_refs(&[("rust", "Rust")]);
        assert_eq!(
            process_transcript_with_optional_fillers("um, rust.", &single, false, true),
            Ok("Rust.".to_owned())
        );
    }

    #[test]
    fn optional_filler_pipeline_preserves_protected_filler_like_replacements() {
        let one_line = vocabulary_refs(&[("rust", "um, keep this")]);
        assert_eq!(
            process_transcript_with_optional_fillers("rust", &one_line, false, true),
            Ok("um, keep this".to_owned())
        );

        let followed_by_ordinary_filler = "um, keep this\nordinary text";
        let entries = vocabulary_refs(&[("rust", "um, keep this")]);
        assert_eq!(
            process_transcript_with_optional_fillers(
                "rust\nuh, ordinary text",
                &entries,
                false,
                true,
            ),
            Ok(followed_by_ordinary_filler.to_owned())
        );

        let multi_line = vocabulary_refs(&[("rust", "first line\num, protected second line")]);
        assert_eq!(
            process_transcript_with_optional_fillers("rust", &multi_line, false, true),
            Ok("first line\num, protected second line".to_owned())
        );

        // The deletion candidate overlaps the protected `um` replacement, so
        // the complete `um, ` prefix is retained rather than partially cut.
        let overlapping = vocabulary_refs(&[("um", "um")]);
        assert_eq!(
            process_transcript_with_optional_fillers("um, yes", &overlapping, false, true),
            Ok("um, yes".to_owned())
        );
    }

    #[test]
    fn optional_filler_pipeline_formats_punctuation_before_removal() {
        let entries = vocabulary_refs(&[("rust", "Rust , Edition"), ("ubc", "UBC")]);

        assert_eq!(
            process_transcript_with_optional_fillers("um, rust , at ubc .", &entries, false, true,),
            Ok("Rust , Edition, at UBC.".to_owned())
        );
        assert_eq!(
            process_transcript_with_optional_fillers("um, rust , at ubc .", &entries, false, false,),
            Ok("um, Rust , Edition, at UBC.".to_owned())
        );
    }

    #[test]
    fn optional_filler_pipeline_preserves_unicode_line_breaks_and_repeated_fillers() {
        assert_eq!(
            process_transcript_with_optional_fillers(
                "um, café 😀\n\nuh, 東京\r\n",
                &[],
                false,
                true,
            ),
            Ok("café 😀\n\n東京\n".to_owned())
        );
        assert_eq!(
            process_transcript_with_optional_fillers("um, uh, yes\nuh, uh, keep", &[], false, true),
            Ok("um, uh, yes\nuh, uh, keep".to_owned())
        );
        assert_eq!(
            process_transcript_with_optional_fillers(
                "um, 2026\n\"uh, yes\"\nuh, cafe\u{301}",
                &[],
                false,
                true,
            ),
            Ok("um, 2026\n\"uh, yes\"\ncafe\u{301}".to_owned())
        );
    }

    #[test]
    fn optional_filler_pipeline_runs_commands_before_vocabulary() {
        let entries = vocabulary_refs(&[("new line", "Literal"), ("rust", "Rust")]);

        assert_eq!(
            process_transcript_with_optional_fillers("um, yes new line rust", &entries, true, true,),
            Ok("yes\nRust".to_owned())
        );
        assert_eq!(
            process_transcript_with_optional_fillers(
                "um, yes new line rust",
                &entries,
                false,
                true,
            ),
            Ok("yes Literal Rust".to_owned())
        );
        assert_eq!(
            process_transcript_with_optional_fillers(
                "um, yes new line rust",
                &entries,
                true,
                false,
            ),
            Ok("um, yes\nRust".to_owned())
        );

        let longest = vocabulary_refs(&[("new", "Short"), ("new york", "New York")]);
        assert_eq!(
            process_transcript_with_optional_fillers("um, new york", &longest, false, true,),
            Ok("New York".to_owned())
        );
        let non_cascading = vocabulary_refs(&[("rust", "ubc"), ("ubc", "UBC")]);
        assert_eq!(
            process_transcript_with_optional_fillers("rust", &non_cascading, false, true),
            Ok("ubc".to_owned())
        );
    }

    #[test]
    fn optional_filler_pipeline_reports_collection_errors_and_is_deterministic() {
        let invalid = vocabulary_refs(&[("rust", " \t")]);
        assert_eq!(
            process_transcript_with_optional_fillers("rust", &invalid, false, true),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            })
        );

        let duplicate = vocabulary_refs(&[("café", "Cafe"), ("cafe\u{301}", "Other")]);
        assert_eq!(
            process_transcript_with_optional_fillers("café", &duplicate, false, true),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        );

        let input = String::from("um, rust\nuh, ubc 😀");
        let input_before = input.clone();
        let vocabulary = vec![
            (String::from("rust"), String::from("Rust")),
            (String::from("ubc"), String::from("UBC")),
        ];
        let vocabulary_before = vocabulary.clone();
        let entries = vocabulary
            .iter()
            .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();

        let first = process_transcript_with_optional_fillers(&input, &entries, false, true);
        let second = process_transcript_with_optional_fillers(&input, &entries, false, true);
        assert_eq!(first, Ok("Rust\nUBC 😀".to_owned()));
        assert_eq!(second, first);
        assert_eq!(input, input_before);
        assert_eq!(vocabulary, vocabulary_before);
    }

    #[test]
    fn optional_filler_pipeline_keeps_existing_public_api_behavior() {
        let entries = vocabulary_refs(&[("rust", "Rust")]);
        assert_eq!(
            normalize_whitespace_commands_vocabulary_and_punctuation("um, rust ,", &entries, false,),
            Ok("um, Rust,".to_owned())
        );
        assert_eq!(remove_leading_isolated_fillers("um, Rust,", true), "Rust,");
        assert_eq!(
            process_transcript_with_optional_fillers("um, rust ,", &entries, false, true),
            Ok("Rust,".to_owned())
        );
    }

    #[test]
    fn protected_filler_provenance_is_valid_adjusted_and_byte_preserved() {
        let entries =
            vocabulary_refs(&[("rust", "Rüst 😀"), ("ubc", "UBC"), ("cafe", "Cafe\u{301}")]);
        let input = "um, rust\r\nuh, ubc\num, cafe";
        let formatted = normalize_whitespace_commands_vocabulary_and_punctuation_with_spans(
            input, &entries, false,
        )
        .expect("the test vocabulary is valid");
        let before = formatted
            .protected_ranges
            .iter()
            .map(|range| formatted.text[range.clone()].to_owned())
            .collect::<Vec<_>>();
        let after = remove_leading_isolated_fillers_with_protection(
            &formatted.text,
            &formatted.protected_ranges,
            true,
        );

        assert_eq!(formatted.text, "um, Rüst 😀\nuh, UBC\num, Cafe\u{301}");
        assert_eq!(after.text, "Rüst 😀\nUBC\nCafe\u{301}");
        assert_eq!(before, ["Rüst 😀", "UBC", "Cafe\u{301}"]);
        assert_eq!(after.protected_ranges.len(), before.len());
        assert_eq!(
            after
                .protected_ranges
                .iter()
                .map(|range| range.start)
                .collect::<Vec<_>>(),
            [0, 11, 15],
            "each preceding filler deletion must shift later UTF-8 byte ranges"
        );

        let assert_valid_ranges = |text: &str, ranges: &[std::ops::Range<usize>]| {
            let mut previous_end = 0;
            for range in ranges {
                assert!(range.start <= range.end);
                assert!(
                    text.get(range.clone()).is_some(),
                    "range must use UTF-8 boundaries"
                );
                assert!(
                    range.start >= previous_end,
                    "ranges must be ordered and non-overlapping"
                );
                previous_end = range.end;
            }
        };
        assert_valid_ranges(&formatted.text, &formatted.protected_ranges);
        assert_valid_ranges(&after.text, &after.protected_ranges);

        for (index, (before_range, after_range)) in formatted
            .protected_ranges
            .iter()
            .zip(&after.protected_ranges)
            .enumerate()
        {
            assert_eq!(
                &formatted.text[before_range.clone()],
                &after.text[after_range.clone()],
                "protected replacement {index} must remain byte-for-byte identical"
            );
        }
    }
}
