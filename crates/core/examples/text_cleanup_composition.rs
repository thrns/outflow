use std::error::Error;

use outflow_core::text_cleanup::normalize_whitespace_and_apply_vocabulary;
use outflow_core::vocabulary::VocabularyEntryRef;

const INPUT: &str = "  I visited   north PARKADE.\r\n\r\n\tI use rust\t at  ubc.  \r\nThe cafe\u{301} serves 東京 tea 😀.";
const EXPECTED: &str =
    "I visited North Parkade.\n\nI use Rust at UBC.\nThe cafe\u{301} serves 東京 tea 😀.";
const UNRELATED_UNICODE: &str = "The cafe\u{301} serves 東京 tea 😀.";

fn main() -> Result<(), Box<dyn Error>> {
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

    let output = normalize_whitespace_and_apply_vocabulary(INPUT, &entries)?;
    let matches_expected = output == EXPECTED;
    let unrelated_unicode_preserved =
        INPUT.contains(UNRELATED_UNICODE) && output.ends_with(UNRELATED_UNICODE);

    println!("Input: {INPUT:?}");
    println!("Output: {output:?}");
    println!("Output matches expected: {matches_expected}");
    println!("Unrelated Unicode preserved: {unrelated_unicode_preserved}");

    assert_eq!(
        output, EXPECTED,
        "composition output differed from expected"
    );
    assert!(
        unrelated_unicode_preserved,
        "unrelated Unicode text was not preserved exactly"
    );

    Ok(())
}
