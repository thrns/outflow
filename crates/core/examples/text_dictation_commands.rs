use outflow_core::text_cleanup::normalize_whitespace_commands_and_apply_vocabulary;
use outflow_core::vocabulary::{VocabularyCollectionError, VocabularyEntryRef};

const INPUT: &str = "  I use  rust  new line   at ubc  new paragraph  north parkade";
const EXPECTED_COMMANDS_ENABLED: &str = "I use Rust\nat UBC\n\nNorth Parkade";
const EXPECTED_COMMANDS_DISABLED: &str = "I use Rust new line at UBC new paragraph North Parkade";

fn main() -> Result<(), VocabularyCollectionError> {
    let vocabulary = [
        VocabularyEntryRef {
            spoken: "rust",
            written: "Rust",
        },
        VocabularyEntryRef {
            spoken: "ubc",
            written: "UBC",
        },
        VocabularyEntryRef {
            spoken: "north parkade",
            written: "North Parkade",
        },
    ];

    let commands_enabled =
        normalize_whitespace_commands_and_apply_vocabulary(INPUT, &vocabulary, true)?;
    let commands_disabled =
        normalize_whitespace_commands_and_apply_vocabulary(INPUT, &vocabulary, false)?;

    println!("input: {INPUT:?}");
    println!("commands enabled: {commands_enabled:?}");
    println!("commands disabled: {commands_disabled:?}");

    assert_eq!(commands_enabled, EXPECTED_COMMANDS_ENABLED);
    assert_eq!(commands_disabled, EXPECTED_COMMANDS_DISABLED);

    Ok(())
}
