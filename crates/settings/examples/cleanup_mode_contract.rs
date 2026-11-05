use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_settings::cleanup_policy::process_transcript_with_filler_preference;
use outflow_settings::{CleanupMode, CleanupSettings};
use std::error::Error;

const INPUT: &str = "  um, rust  new line  at ubc .";
const CLEAN_FILLERS: &str = "Rust\nat UBC.";
const KEEP_FILLERS: &str = "um, Rust\nat UBC.";

fn main() -> Result<(), Box<dyn Error>> {
    let vocabulary = [
        VocabularyEntryRef {
            spoken: "rust",
            written: "Rust",
        },
        VocabularyEntryRef {
            spoken: "ubc",
            written: "UBC",
        },
    ];

    println!("Input: {INPUT:?}");
    for (mode, remove_fillers, label, expected) in [
        (
            CleanupMode::Clean,
            true,
            "Clean, remove_fillers=true",
            CLEAN_FILLERS,
        ),
        (
            CleanupMode::Clean,
            false,
            "Clean, remove_fillers=false",
            KEEP_FILLERS,
        ),
        (
            CleanupMode::Verbatim,
            true,
            "Verbatim, remove_fillers=true",
            KEEP_FILLERS,
        ),
        (
            CleanupMode::Verbatim,
            false,
            "Verbatim, remove_fillers=false",
            KEEP_FILLERS,
        ),
    ] {
        let cleanup = CleanupSettings {
            mode,
            remove_fillers,
        };
        let output = process_transcript_with_filler_preference(INPUT, &vocabulary, true, &cleanup)?;
        println!("{label}: {output:?}");
        assert_eq!(output, expected, "unexpected output for {label}");
    }

    let commands_disabled = process_transcript_with_filler_preference(
        INPUT,
        &vocabulary,
        false,
        &CleanupSettings {
            mode: CleanupMode::Verbatim,
            remove_fillers: true,
        },
    )?;
    println!("Commands disabled (Verbatim): {commands_disabled:?}");
    assert_eq!(commands_disabled, "um, Rust new line at UBC.");

    Ok(())
}
