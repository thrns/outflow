use std::error::Error;
use std::io;

use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_core::vocabulary_post_correction::apply_vocabulary_post_corrections;

fn main() -> Result<(), Box<dyn Error>> {
    let input = "I visited north PARKADE.\nI use rust at ubc.\nThe cafe\u{301} serves 東京 tea 😀.";
    let expected =
        "I visited North Parkade.\nI use Rust at UBC.\nThe cafe\u{301} serves 東京 tea 😀.";
    let entries = [
        VocabularyEntryRef {
            spoken: "north parkade",
            written: "North Parkade",
        },
        VocabularyEntryRef {
            spoken: "rust",
            written: "Rust",
        },
        VocabularyEntryRef {
            spoken: "ubc",
            written: "UBC",
        },
    ];
    let corrected = apply_vocabulary_post_corrections(input, &entries)?;
    if corrected != expected {
        return Err(
            io::Error::other("multi-entry correction differed from expected output").into(),
        );
    }

    println!("Input:\n{input}");
    println!("Output:\n{corrected}");
    println!(
        "Unrelated Unicode preserved: {}",
        corrected.contains("cafe\u{301} serves 東京 tea 😀.")
    );

    // The replacement produced for `rust` is not rescanned against the `ubc`
    // entry during this same operation.
    let cascading_entries = [
        VocabularyEntryRef {
            spoken: "rust",
            written: "ubc",
        },
        VocabularyEntryRef {
            spoken: "ubc",
            written: "UBC",
        },
    ];
    let non_cascading = apply_vocabulary_post_corrections("rust", &cascading_entries)?;
    println!("Non-cascading input: rust");
    println!("Non-cascading output: {non_cascading}");

    Ok(())
}
