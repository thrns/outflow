use std::process::ExitCode;

use outflow_core::vocabulary::{
    MAX_VOCABULARY_ENTRIES, MAX_VOCABULARY_FIELD_SCALARS, VocabularyCollectionError,
    VocabularyEntryRef, VocabularyField, VocabularyValidationError, validate_vocabulary_collection,
    validate_vocabulary_entry,
};

fn main() -> ExitCode {
    let valid_mapping = validate_vocabulary_entry("north parkade", "North Parkade").is_ok();
    let empty_spoken_phrase = matches!(
        validate_vocabulary_entry("", "North Parkade"),
        Err(VocabularyValidationError::Empty(VocabularyField::Spoken))
    );
    let overlong_written_replacement = matches!(
        validate_vocabulary_entry("source", &"x".repeat(MAX_VOCABULARY_FIELD_SCALARS + 1)),
        Err(VocabularyValidationError::TooLong(VocabularyField::Written))
    );
    let unicode_scalar_boundary =
        validate_vocabulary_entry(&"😀".repeat(MAX_VOCABULARY_FIELD_SCALARS), "replacement")
            .is_ok();

    let owned_collection = (0..=MAX_VOCABULARY_ENTRIES)
        .map(|index| (format!("spoken-{index}"), format!("written-{index}")))
        .collect::<Vec<_>>();
    let entries_500 = entry_refs(&owned_collection[..MAX_VOCABULARY_ENTRIES]);
    let entries_501 = entry_refs(&owned_collection);
    let exactly_500_accepted = validate_vocabulary_collection(&entries_500).is_ok();
    let over_limit_rejected = matches!(
        validate_vocabulary_collection(&entries_501),
        Err(VocabularyCollectionError::TooManyEntries {
            count,
            maximum,
        }) if count == MAX_VOCABULARY_ENTRIES + 1 && maximum == MAX_VOCABULARY_ENTRIES
    );

    let exact_duplicate = [
        VocabularyEntryRef {
            spoken: "north parkade",
            written: "North Parkade",
        },
        VocabularyEntryRef {
            spoken: "north parkade",
            written: "Northern Parking Area",
        },
    ];
    let exact_duplicate_rejected = matches!(
        validate_vocabulary_collection(&exact_duplicate),
        Err(VocabularyCollectionError::DuplicateSpoken { .. })
    );

    let ascii_case_variation = [
        VocabularyEntryRef {
            spoken: "rust",
            written: "Rust",
        },
        VocabularyEntryRef {
            spoken: "RUST",
            written: "Rust Language",
        },
    ];
    let ascii_case_conflict_rejected = matches!(
        validate_vocabulary_collection(&ascii_case_variation),
        Err(VocabularyCollectionError::DuplicateSpoken { .. })
    );

    let lowercase_unicode_pair = [
        VocabularyEntryRef {
            spoken: "CAFÉ",
            written: "Cafe",
        },
        VocabularyEntryRef {
            spoken: "café",
            written: "Coffee Shop",
        },
    ];
    let unicode_casefold_limitation = [
        VocabularyEntryRef {
            spoken: "straße",
            written: "street",
        },
        VocabularyEntryRef {
            spoken: "STRASSE",
            written: "Street",
        },
    ];
    let unicode_policy_is_provisional =
        matches!(
            validate_vocabulary_collection(&lowercase_unicode_pair),
            Err(VocabularyCollectionError::DuplicateSpoken { .. })
        ) && validate_vocabulary_collection(&unicode_casefold_limitation).is_ok();

    println!("Outflow vocabulary validation demo");
    print_case("Valid vocabulary mapping", valid_mapping, "PASS");
    print_case("Empty spoken phrase", empty_spoken_phrase, "REJECTED");
    print_case(
        "Overlong written replacement",
        overlong_written_replacement,
        "REJECTED",
    );
    print_case("Unicode scalar boundary", unicode_scalar_boundary, "PASS");
    println!("Vocabulary collection validation");
    print_case("500 entries", exactly_500_accepted, "PASS");
    print_case("501 entries", over_limit_rejected, "REJECTED");
    print_case(
        "Duplicate spoken phrase",
        exact_duplicate_rejected,
        "REJECTED",
    );
    print_case(
        "Case-insensitive ASCII conflict",
        ascii_case_conflict_rejected,
        "REJECTED",
    );
    print_case(
        "Unicode matching policy",
        unicode_policy_is_provisional,
        "PROVISIONAL",
    );
    println!("Vocabulary persistence: NOT IMPLEMENTED");
    println!("Text replacement: NOT IMPLEMENTED");

    if valid_mapping
        && empty_spoken_phrase
        && overlong_written_replacement
        && unicode_scalar_boundary
        && exactly_500_accepted
        && over_limit_rejected
        && exact_duplicate_rejected
        && ascii_case_conflict_rejected
        && unicode_policy_is_provisional
    {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn entry_refs(entries: &[(String, String)]) -> Vec<VocabularyEntryRef<'_>> {
    entries
        .iter()
        .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
        .collect()
}

fn print_case(label: &str, passed: bool, expected_result: &str) {
    let result = if passed { expected_result } else { "FAIL" };
    println!("{label}: {result}");
}
