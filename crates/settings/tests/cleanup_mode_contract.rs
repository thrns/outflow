//! Public cross-crate characterization of the existing Clean/Verbatim adapter.
//! These tests lock current behavior without deciding complete Verbatim policy.

use outflow_core::vocabulary::{
    VocabularyCollectionError, VocabularyEntryRef, VocabularyField, VocabularyValidationError,
};
use outflow_settings::cleanup_policy::process_transcript_with_filler_preference;
use outflow_settings::{CURRENT_SCHEMA_VERSION, CleanupMode, CleanupSettings, Settings};

fn refs<'a>(pairs: &[(&'a str, &'a str)]) -> Vec<VocabularyEntryRef<'a>> {
    pairs
        .iter()
        .map(|&(spoken, written)| VocabularyEntryRef { spoken, written })
        .collect()
}

fn cleanup(mode: CleanupMode, remove_fillers: bool) -> CleanupSettings {
    CleanupSettings {
        mode,
        remove_fillers,
    }
}

fn output(
    input: &str,
    vocabulary: &[VocabularyEntryRef<'_>],
    commands_enabled: bool,
    settings: &CleanupSettings,
) -> Result<String, VocabularyCollectionError> {
    process_transcript_with_filler_preference(input, vocabulary, commands_enabled, settings)
}

#[test]
fn four_mode_and_filler_preference_combinations_match_current_contract() {
    let vocabulary = refs(&[("rust", "Rust"), ("ubc", "UBC")]);
    let input = "  um, rust  new line  at ubc .";

    for (mode, remove_fillers, expected) in [
        (CleanupMode::Clean, true, "Rust\nat UBC."),
        (CleanupMode::Clean, false, "um, Rust\nat UBC."),
        (CleanupMode::Verbatim, true, "um, Rust\nat UBC."),
        (CleanupMode::Verbatim, false, "um, Rust\nat UBC."),
    ] {
        assert_eq!(
            output(input, &vocabulary, true, &cleanup(mode, remove_fillers)),
            Ok(expected.to_owned()),
            "mode={mode:?}, remove_fillers={remove_fillers}"
        );
    }
}

#[test]
fn commands_are_independent_and_other_stages_run_in_both_modes() {
    let vocabulary = refs(&[("rust", "Rust"), ("ubc", "UBC")]);
    let input = "  um, rust  new line  at ubc .";

    for mode in [CleanupMode::Clean, CleanupMode::Verbatim] {
        let settings = cleanup(mode, false);
        assert_eq!(
            output(input, &vocabulary, true, &settings),
            Ok("um, Rust\nat UBC.".to_owned()),
            "enabled command in {mode:?}"
        );
        assert_eq!(
            output(input, &vocabulary, false, &settings),
            Ok("um, Rust new line at UBC.".to_owned()),
            "disabled command in {mode:?}"
        );
    }
}

#[test]
fn whitespace_and_paragraph_normalization_run_in_both_modes() {
    let vocabulary = refs(&[("rust", "Rust"), ("ubc", "UBC")]);
    let input = " \tum,\t  rust  \r\n  uh,\t ubc . \n";

    assert_eq!(
        output(
            input,
            &vocabulary,
            false,
            &cleanup(CleanupMode::Clean, true)
        ),
        Ok("Rust\nUBC.\n".to_owned())
    );
    for settings in [
        cleanup(CleanupMode::Clean, false),
        cleanup(CleanupMode::Verbatim, true),
        cleanup(CleanupMode::Verbatim, false),
    ] {
        assert_eq!(
            output(input, &vocabulary, false, &settings),
            Ok("um, Rust\nuh, UBC.\n".to_owned())
        );
    }

    let paragraphs = "um, first\r\nuh, second\n\num, third\r";
    assert_eq!(
        output(paragraphs, &[], false, &cleanup(CleanupMode::Clean, true)),
        Ok("first\nsecond\n\nthird\n".to_owned())
    );
    assert_eq!(
        output(
            paragraphs,
            &[],
            false,
            &cleanup(CleanupMode::Verbatim, true)
        ),
        Ok("um, first\nuh, second\n\num, third\n".to_owned())
    );
}

#[test]
fn vocabulary_correction_and_protected_punctuation_spacing_run_in_both_modes() {
    let vocabulary = refs(&[("rust", "Rust , Edition")]);

    for mode in [CleanupMode::Clean, CleanupMode::Verbatim] {
        assert_eq!(
            output("rust .", &vocabulary, false, &cleanup(mode, true)),
            Ok("Rust , Edition.".to_owned()),
            "replacement bytes stay protected while adjacent punctuation spacing is corrected in {mode:?}"
        );
    }
}

#[test]
fn filler_like_vocabulary_replacements_and_repeated_fillers_are_preserved() {
    let vocabulary = refs(&[("rust", "um, keep this")]);
    for (mode, remove_fillers) in [
        (CleanupMode::Clean, true),
        (CleanupMode::Clean, false),
        (CleanupMode::Verbatim, true),
        (CleanupMode::Verbatim, false),
    ] {
        let replaced = output("rust", &vocabulary, false, &cleanup(mode, remove_fillers))
            .expect("valid replacement should process");
        assert_eq!(replaced.as_bytes(), b"um, keep this");
    }

    assert_eq!(
        output(
            "um, uh, maybe",
            &[],
            false,
            &cleanup(CleanupMode::Clean, true)
        ),
        Ok("um, uh, maybe".to_owned())
    );
}

#[test]
fn numbers_proper_nouns_punctuation_and_unicode_are_preserved() {
    let decomposed_cafe = "cafe\u{301}";
    let input = format!("uh, Dr. Élodie kept version 3 . 14 in {decomposed_cafe}, 東京 😀!");

    assert_eq!(
        output(&input, &[], false, &cleanup(CleanupMode::Clean, true)),
        Ok(format!(
            "Dr. Élodie kept version 3 . 14 in {decomposed_cafe}, 東京 😀!"
        ))
    );
    assert_eq!(
        output(&input, &[], false, &cleanup(CleanupMode::Verbatim, true)),
        Ok(format!(
            "uh, Dr. Élodie kept version 3 . 14 in {decomposed_cafe}, 東京 😀!"
        ))
    );
}

#[test]
fn invalid_and_duplicate_vocabulary_return_existing_typed_errors() {
    let invalid = refs(&[("rust", " \t")]);
    assert_eq!(
        output("rust", &invalid, false, &cleanup(CleanupMode::Clean, true)),
        Err(VocabularyCollectionError::InvalidEntry {
            index: 0,
            source: VocabularyValidationError::Empty(VocabularyField::Written),
        })
    );

    let duplicate = refs(&[("café", "Cafe"), ("cafe\u{301}", "Other")]);
    assert_eq!(
        output(
            "café",
            &duplicate,
            false,
            &cleanup(CleanupMode::Verbatim, true)
        ),
        Err(VocabularyCollectionError::DuplicateSpoken {
            first_index: 0,
            duplicate_index: 1,
        })
    );
}

#[test]
fn calls_are_deterministic_and_do_not_mutate_inputs_or_preferences() {
    let input = String::from("um, rust\nuh, ubc 😀");
    let input_before = input.clone();
    let pairs = vec![
        (String::from("rust"), String::from("Rust")),
        (String::from("ubc"), String::from("UBC")),
    ];
    let pairs_before = pairs.clone();
    let vocabulary = pairs
        .iter()
        .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
        .collect::<Vec<_>>();
    let settings = cleanup(CleanupMode::Clean, true);
    let settings_before = settings.clone();

    let first = output(&input, &vocabulary, false, &settings);
    for _ in 0..5 {
        assert_eq!(output(&input, &vocabulary, false, &settings), first);
    }
    assert_eq!(first, Ok("Rust\nUBC 😀".to_owned()));
    assert_eq!(input, input_before);
    assert_eq!(pairs, pairs_before);
    assert_eq!(settings, settings_before);
}

#[test]
fn schema_v1_settings_json_shape_and_cleanup_fields_are_unchanged() {
    for (mode, remove_fillers, serialized_mode) in [
        (CleanupMode::Clean, true, "clean"),
        (CleanupMode::Clean, false, "clean"),
        (CleanupMode::Verbatim, true, "verbatim"),
        (CleanupMode::Verbatim, false, "verbatim"),
    ] {
        let settings = Settings {
            cleanup: cleanup(mode, remove_fillers),
            ..Settings::default()
        };
        let json = settings.to_json().expect("schema-v1 settings serialize");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("serialized settings are valid JSON");

        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(
            value,
            serde_json::json!({
                "schema_version": 1,
                "language": "en",
                "recording": {
                    "push_to_talk": null,
                    "toggle": null,
                    "input_device": "system-default"
                },
                "cleanup": {
                    "mode": serialized_mode,
                    "remove_fillers": remove_fillers
                },
                "startup": {"run_at_login": false},
                "updates": {"user_initiated_only": true},
                "vocabulary": []
            })
        );
        assert_eq!(Settings::from_json(&json), Ok(settings));
    }
}
