//! Pure adapter between persisted cleanup preferences and C-06's five-stage
//! text pipeline. This module controls only optional filler deletion; it does
//! not define complete Clean or Verbatim behavior and performs no I/O.

use outflow_core::text_cleanup::process_transcript_with_optional_fillers;
use outflow_core::vocabulary::{VocabularyCollectionError, VocabularyEntryRef};

use crate::{CleanupMode, CleanupSettings};

/// Runs the core cleanup pipeline using the supported filler preference rule.
///
/// Filler removal is enabled exactly when `cleanup.mode` is [`CleanupMode::Clean`]
/// and `cleanup.remove_fillers` is `true`. All other pipeline stages, including
/// optional commands, vocabulary correction, whitespace normalization, and
/// punctuation spacing, retain their existing behavior regardless of mode.
/// The function borrows its inputs and performs no file, microphone, or
/// transcript-storage operations.
///
/// # Errors
///
/// Returns the core pipeline's existing typed error if the vocabulary
/// collection is too large, contains an invalid entry, or contains duplicate
/// spoken phrases under the existing matching policy.
pub fn process_transcript_with_filler_preference(
    input: &str,
    entries: &[VocabularyEntryRef<'_>],
    commands_enabled: bool,
    cleanup: &CleanupSettings,
) -> Result<String, VocabularyCollectionError> {
    let fillers_enabled = cleanup.mode == CleanupMode::Clean && cleanup.remove_fillers;

    process_transcript_with_optional_fillers(input, entries, commands_enabled, fillers_enabled)
}

#[cfg(test)]
mod tests {
    use super::process_transcript_with_filler_preference;
    use crate::{CURRENT_SCHEMA_VERSION, CleanupMode, CleanupSettings, Settings};
    use outflow_core::vocabulary::{
        VocabularyCollectionError, VocabularyEntryRef, VocabularyField, VocabularyValidationError,
    };

    fn cleanup(mode: CleanupMode, remove_fillers: bool) -> CleanupSettings {
        CleanupSettings {
            mode,
            remove_fillers,
        }
    }

    fn refs<'a>(pairs: &'a [(&'a str, &'a str)]) -> Vec<VocabularyEntryRef<'a>> {
        pairs
            .iter()
            .map(|&(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect()
    }

    #[test]
    fn cleanup_policy_obeys_all_four_mode_and_flag_combinations() {
        for (mode, remove_fillers, expected) in [
            (CleanupMode::Clean, true, "yes"),
            (CleanupMode::Clean, false, "um, yes"),
            (CleanupMode::Verbatim, true, "um, yes"),
            (CleanupMode::Verbatim, false, "um, yes"),
        ] {
            assert_eq!(
                process_transcript_with_filler_preference(
                    "um, yes",
                    &[],
                    false,
                    &cleanup(mode, remove_fillers),
                ),
                Ok(expected.to_owned()),
                "unexpected output for {mode:?}, remove_fillers={remove_fillers}"
            );
        }
    }

    #[test]
    fn cleanup_policy_default_enables_the_supported_filler_rule() {
        let settings = CleanupSettings::default();
        assert_eq!(settings.mode, CleanupMode::Clean);
        assert!(settings.remove_fillers);
        assert_eq!(
            process_transcript_with_filler_preference("uh, yes", &[], false, &settings),
            Ok("yes".to_owned())
        );
    }

    #[test]
    fn cleanup_policy_removes_only_supported_line_leading_fillers() {
        let input = "um, first\nuh, second\nmm, third\ner, fourth\num, 2026\n\"uh, quoted\"";
        assert_eq!(
            process_transcript_with_filler_preference(
                input,
                &[],
                false,
                &cleanup(CleanupMode::Clean, true),
            ),
            Ok("first\nsecond\nmm, third\ner, fourth\num, 2026\n\"uh, quoted\"".to_owned())
        );
    }

    #[test]
    fn cleanup_policy_verbatim_and_disabled_clean_retain_fillers() {
        for settings in [
            cleanup(CleanupMode::Clean, false),
            cleanup(CleanupMode::Verbatim, true),
            cleanup(CleanupMode::Verbatim, false),
        ] {
            assert_eq!(
                process_transcript_with_filler_preference(
                    "um, first\nuh, second",
                    &[],
                    false,
                    &settings,
                ),
                Ok("um, first\nuh, second".to_owned())
            );
        }
    }

    #[test]
    fn cleanup_policy_keeps_repeated_fillers_unchanged() {
        assert_eq!(
            process_transcript_with_filler_preference(
                "um, uh, yes\nuh, uh, keep",
                &[],
                false,
                &cleanup(CleanupMode::Clean, true),
            ),
            Ok("um, uh, yes\nuh, uh, keep".to_owned())
        );
    }

    #[test]
    fn cleanup_policy_keeps_sentence_internal_fillers_unchanged() {
        assert_eq!(
            process_transcript_with_filler_preference(
                "I said um, not uh.",
                &[],
                false,
                &cleanup(CleanupMode::Clean, true),
            ),
            Ok("I said um, not uh.".to_owned())
        );
    }

    #[test]
    fn cleanup_policy_commands_are_independent_of_filler_preference() {
        let entries = refs(&[("new line", "Literal"), ("rust", "Rust")]);
        for commands_enabled in [false, true] {
            for (mode, remove_fillers, remove_expected) in [
                (CleanupMode::Clean, true, true),
                (CleanupMode::Clean, false, false),
                (CleanupMode::Verbatim, true, false),
                (CleanupMode::Verbatim, false, false),
            ] {
                let expected = match (commands_enabled, remove_expected) {
                    (false, true) => "yes Literal Rust",
                    (false, false) => "um, yes Literal Rust",
                    (true, true) => "yes\nRust",
                    (true, false) => "um, yes\nRust",
                };
                assert_eq!(
                    process_transcript_with_filler_preference(
                        "um, yes new line rust",
                        &entries,
                        commands_enabled,
                        &cleanup(mode, remove_fillers),
                    ),
                    Ok(expected.to_owned())
                );
            }
        }
    }

    #[test]
    fn cleanup_policy_keeps_vocabulary_correction_active_in_all_combinations() {
        let entries = refs(&[("rust", "Rust")]);
        for (mode, remove_fillers, expected) in [
            (CleanupMode::Clean, true, "Rust"),
            (CleanupMode::Clean, false, "um, Rust"),
            (CleanupMode::Verbatim, true, "um, Rust"),
            (CleanupMode::Verbatim, false, "um, Rust"),
        ] {
            assert_eq!(
                process_transcript_with_filler_preference(
                    "um, rust",
                    &entries,
                    false,
                    &cleanup(mode, remove_fillers),
                ),
                Ok(expected.to_owned())
            );
        }
    }

    #[test]
    fn cleanup_policy_preserves_protected_filler_like_replacements_exactly() {
        let entries = refs(&[("rust", "um, keep this")]);
        for (mode, remove_fillers) in [
            (CleanupMode::Clean, true),
            (CleanupMode::Clean, false),
            (CleanupMode::Verbatim, true),
            (CleanupMode::Verbatim, false),
        ] {
            let output = process_transcript_with_filler_preference(
                "rust",
                &entries,
                false,
                &cleanup(mode, remove_fillers),
            )
            .expect("valid protected replacement should process");
            assert_eq!(output.as_bytes(), b"um, keep this");
        }
    }

    #[test]
    fn cleanup_policy_punctuation_spacing_is_independent_of_filler_preference() {
        let entries = refs(&[("rust", "Rust , Edition"), ("ubc", "UBC")]);
        for (mode, remove_fillers, expected) in [
            (CleanupMode::Clean, true, "Rust , Edition, at UBC."),
            (CleanupMode::Clean, false, "um, Rust , Edition, at UBC."),
            (CleanupMode::Verbatim, true, "um, Rust , Edition, at UBC."),
            (CleanupMode::Verbatim, false, "um, Rust , Edition, at UBC."),
        ] {
            assert_eq!(
                process_transcript_with_filler_preference(
                    "um, rust , at ubc .",
                    &entries,
                    false,
                    &cleanup(mode, remove_fillers),
                ),
                Ok(expected.to_owned())
            );
        }
    }

    #[test]
    fn cleanup_policy_preserves_unicode_and_paragraph_breaks() {
        let combining = "cafe\u{301}";
        let input = format!("um, café {combining} 😀\n\nuh, 東京\r\nuh, {combining}");
        let expected = format!("café {combining} 😀\n\n東京\n{combining}");
        assert_eq!(
            process_transcript_with_filler_preference(
                &input,
                &[],
                false,
                &cleanup(CleanupMode::Clean, true),
            ),
            Ok(expected)
        );
    }

    #[test]
    fn cleanup_policy_returns_existing_typed_vocabulary_errors() {
        let invalid = refs(&[("rust", " \t")]);
        assert_eq!(
            process_transcript_with_filler_preference(
                "rust",
                &invalid,
                false,
                &cleanup(CleanupMode::Clean, true),
            ),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            })
        );

        let duplicate = refs(&[("café", "Cafe"), ("cafe\u{301}", "Other")]);
        assert_eq!(
            process_transcript_with_filler_preference(
                "café",
                &duplicate,
                false,
                &cleanup(CleanupMode::Verbatim, true),
            ),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn cleanup_policy_does_not_mutate_input_entries_or_preferences() {
        let input = String::from("um, rust\nuh, ubc 😀");
        let input_before = input.clone();
        let pairs = vec![
            (String::from("rust"), String::from("Rust")),
            (String::from("ubc"), String::from("UBC")),
        ];
        let pairs_before = pairs.clone();
        let entries = pairs
            .iter()
            .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();
        let settings = cleanup(CleanupMode::Clean, true);
        let settings_before = settings.clone();

        assert_eq!(
            process_transcript_with_filler_preference(&input, &entries, false, &settings),
            Ok("Rust\nUBC 😀".to_owned())
        );
        assert_eq!(input, input_before);
        assert_eq!(pairs, pairs_before);
        assert_eq!(settings, settings_before);
    }

    #[test]
    fn cleanup_policy_is_deterministic_for_identical_arguments() {
        let entries = refs(&[("rust", "Rust"), ("ubc", "UBC")]);
        let settings = cleanup(CleanupMode::Clean, true);
        let first = process_transcript_with_filler_preference(
            "um, rust\nuh, ubc 😀",
            &entries,
            false,
            &settings,
        );
        let second = process_transcript_with_filler_preference(
            "um, rust\nuh, ubc 😀",
            &entries,
            false,
            &settings,
        );
        assert_eq!(first, Ok("Rust\nUBC 😀".to_owned()));
        assert_eq!(second, first);
    }

    #[test]
    fn cleanup_policy_preserves_schema_v1_json_and_round_trip() {
        for (mode, remove_fillers, expected_mode) in [
            (CleanupMode::Clean, true, "clean"),
            (CleanupMode::Clean, false, "clean"),
            (CleanupMode::Verbatim, true, "verbatim"),
            (CleanupMode::Verbatim, false, "verbatim"),
        ] {
            let settings = Settings {
                cleanup: cleanup(mode, remove_fillers),
                ..Settings::default()
            };
            let json = settings.to_json().expect("settings serialize");
            let value: serde_json::Value =
                serde_json::from_str(&json).expect("serialized settings are valid JSON");

            assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
            assert_eq!(value["schema_version"], 1);
            assert_eq!(
                value["cleanup"],
                serde_json::json!({"mode": expected_mode, "remove_fillers": remove_fillers})
            );
            assert_eq!(Settings::from_json(&json), Ok(settings));
        }
    }
}
