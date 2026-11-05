use std::error::Error;

use outflow_core::text_cleanup::process_transcript_with_optional_fillers;
use outflow_core::vocabulary::VocabularyEntryRef;

const INPUT: &str = "um, rust , at ubc .";
const FILLERS_ENABLED: &str = "Rust , Edition, at UBC.";
const FILLERS_DISABLED: &str = "um, Rust , Edition, at UBC.";

fn main() -> Result<(), Box<dyn Error>> {
    let vocabulary = [
        VocabularyEntryRef {
            spoken: "rust",
            written: "Rust , Edition",
        },
        VocabularyEntryRef {
            spoken: "ubc",
            written: "UBC",
        },
    ];

    let enabled = process_transcript_with_optional_fillers(INPUT, &vocabulary, false, true)?;
    let disabled = process_transcript_with_optional_fillers(INPUT, &vocabulary, false, false)?;

    let protected_vocabulary = [VocabularyEntryRef {
        spoken: "rust",
        written: "um, keep this",
    }];
    let protected_output =
        process_transcript_with_optional_fillers("rust", &protected_vocabulary, false, true)?;
    let protected_replacement_preserved = protected_output == "um, keep this";

    println!("Input: {INPUT:?}");
    println!("Fillers enabled: {enabled:?}");
    println!("Fillers disabled: {disabled:?}");
    println!("Protected replacement preserved: {protected_replacement_preserved}");

    assert_eq!(
        enabled, FILLERS_ENABLED,
        "enabled output differed from expected"
    );
    assert_eq!(
        disabled, FILLERS_DISABLED,
        "disabled output differed from expected"
    );
    assert!(
        protected_replacement_preserved,
        "filler removal modified a protected vocabulary replacement"
    );

    Ok(())
}
