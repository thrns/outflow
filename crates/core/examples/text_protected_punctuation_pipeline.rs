use std::error::Error;

use outflow_core::text_cleanup::{
    apply_dictation_commands, normalize_transcript_whitespace,
    normalize_whitespace_commands_vocabulary_and_punctuation,
};
use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_core::vocabulary_post_correction::apply_vocabulary_post_corrections_with_spans;

const INPUT: &str = "rust , at ubc .";
const EXPECTED: &str = "Rust , Edition, at UBC.";

fn protected_replacements_are_preserved(
    final_text: &str,
    corrected_text: &str,
    protected_ranges: &[std::ops::Range<usize>],
) -> bool {
    let mut search_from = 0;

    for protected_range in protected_ranges {
        let replacement = &corrected_text[protected_range.clone()];
        let Some(relative_start) = final_text[search_from..].find(replacement) else {
            return false;
        };
        let final_start = search_from + relative_start;
        let final_end = final_start + replacement.len();
        if final_text.get(final_start..final_end) != Some(replacement) {
            return false;
        }
        search_from = final_end;
    }

    !protected_ranges.is_empty()
}

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

    let output = normalize_whitespace_commands_vocabulary_and_punctuation(INPUT, &entries, false)?;

    // Recreate the vocabulary stage's ordered provenance ranges for this
    // example's preservation assertion. The public composed API intentionally
    // returns final text only and does not expose intermediate offsets.
    let normalized = normalize_transcript_whitespace(INPUT);
    let commanded = apply_dictation_commands(&normalized, false);
    let corrected = apply_vocabulary_post_corrections_with_spans(&commanded, &entries)?;
    let matches_expected = output == EXPECTED;
    let protected_replacement_preserved =
        protected_replacements_are_preserved(&output, &corrected.text, &corrected.protected_ranges);

    println!("Input: {INPUT:?}");
    println!("Output: {output:?}");
    println!("Matches expected: {matches_expected}");
    println!("Protected replacement preserved: {protected_replacement_preserved}");

    assert_eq!(output, EXPECTED, "composed pipeline output differed");
    assert!(
        protected_replacement_preserved,
        "a protected vocabulary replacement was changed or reordered"
    );

    Ok(())
}
