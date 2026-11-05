use std::error::Error;

use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_settings::cleanup_policy::process_transcript_with_filler_preference;
use outflow_settings::{CleanupMode, CleanupSettings};

const INPUT: &str = "um, rust , at ubc .";
const CLEAN_ENABLED: &str = "Rust , Edition, at UBC.";
const CLEAN_DISABLED: &str = "um, Rust , Edition, at UBC.";

fn main() -> Result<(), Box<dyn Error>> {
    let entries = [
        VocabularyEntryRef {
            spoken: "rust",
            written: "Rust , Edition",
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
            CLEAN_ENABLED,
        ),
        (
            CleanupMode::Clean,
            false,
            "Clean, remove_fillers=false",
            CLEAN_DISABLED,
        ),
        (
            CleanupMode::Verbatim,
            true,
            "Verbatim, remove_fillers=true",
            CLEAN_DISABLED,
        ),
        (
            CleanupMode::Verbatim,
            false,
            "Verbatim, remove_fillers=false",
            CLEAN_DISABLED,
        ),
    ] {
        let settings = CleanupSettings {
            mode,
            remove_fillers,
        };
        let output = process_transcript_with_filler_preference(INPUT, &entries, false, &settings)?;
        println!("{label}: {output:?}");
        assert_eq!(output, expected, "unexpected complete output for {label}");
    }

    let protected_entry = [VocabularyEntryRef {
        spoken: "rust",
        written: "um, keep this",
    }];
    let protected = process_transcript_with_filler_preference(
        "rust",
        &protected_entry,
        false,
        &CleanupSettings::default(),
    )?;
    assert_eq!(
        protected.as_bytes(),
        b"um, keep this",
        "filler-like vocabulary replacement must remain byte-for-byte unchanged"
    );

    Ok(())
}
