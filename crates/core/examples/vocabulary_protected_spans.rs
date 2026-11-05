use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_core::vocabulary_post_correction::apply_vocabulary_post_corrections_with_spans;

fn all_ranges_match(
    text: &str,
    ranges: &[std::ops::Range<usize>],
    expected_replacements: &[&str],
) -> bool {
    if ranges.len() != expected_replacements.len() {
        return false;
    }

    let mut previous_end = 0;
    ranges
        .iter()
        .zip(expected_replacements)
        .all(|(range, expected)| {
            let is_valid = range.start <= range.end
                && range.end <= text.len()
                && range.start >= previous_end
                && text.is_char_boundary(range.start)
                && text.is_char_boundary(range.end)
                && text.get(range.clone()) == Some(*expected);
            previous_end = range.end;
            is_valid
        })
}

fn main() {
    let input = "rust , at ubc .";
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
    let corrected = apply_vocabulary_post_corrections_with_spans(input, &entries)
        .expect("the synthetic vocabulary collection is valid");
    let expected_output = "Rust , Edition , at UBC .";
    let expected_ranges = [0..14, 20..23];
    let expected_replacements = ["Rust , Edition", "UBC"];
    let all_assertions_passed = corrected.text == expected_output
        && corrected.protected_ranges == expected_ranges
        && all_ranges_match(
            &corrected.text,
            &corrected.protected_ranges,
            &expected_replacements,
        );

    println!("Original input: {input:?}");
    println!("Corrected output: {:?}", corrected.text);
    println!("Protected ranges: {}", corrected.protected_ranges.len());
    for (index, range) in corrected.protected_ranges.iter().enumerate() {
        match corrected.text.get(range.clone()) {
            Some(replacement) => {
                println!("Protected replacement {}: {replacement:?}", index + 1);
            }
            None => println!("Protected replacement {}: <invalid range>", index + 1),
        }
    }
    println!("All range assertions passed: {all_assertions_passed}");

    assert_eq!(corrected.text, expected_output);
    assert_eq!(corrected.protected_ranges, expected_ranges);
    assert!(all_assertions_passed, "protected range assertions failed");
}
