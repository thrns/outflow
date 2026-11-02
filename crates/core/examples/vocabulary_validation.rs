use outflow_core::vocabulary::{
    MAX_VOCABULARY_ENTRIES, MAX_VOCABULARY_FIELD_SCALARS, VocabularyCollectionError,
    VocabularyEntryRef, VocabularyField, VocabularyValidationError, validate_vocabulary_collection,
    validate_vocabulary_entry,
};
use outflow_core::vocabulary_matching::{CURRENT_VOCABULARY_MATCHING_VERSION, comparison_key};
use std::process::ExitCode;

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

    let owned_collection = (0..=MAX_VOCABULARY_ENTRIES)
        .map(|index| (format!("spoken-{index}"), format!("written-{index}")))
        .collect::<Vec<_>>();
    let entries_500 = entry_refs(&owned_collection[..MAX_VOCABULARY_ENTRIES]);
    let entries_501 = entry_refs(&owned_collection);
    let exactly_500_accepted = validate_vocabulary_collection(&entries_500).is_ok();
    let over_limit_rejected = matches!(
        validate_vocabulary_collection(&entries_501),
        Err(VocabularyCollectionError::TooManyEntries { count, maximum })
            if count == MAX_VOCABULARY_ENTRIES + 1 && maximum == MAX_VOCABULARY_ENTRIES
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
    let ascii_case_duplicate = is_duplicate(&ascii_case_variation);

    let sharp_s_pair = [
        VocabularyEntryRef {
            spoken: "straße",
            written: "street",
        },
        VocabularyEntryRef {
            spoken: "STRASSE",
            written: "Street",
        },
    ];
    let sharp_s_full_fold_duplicate = is_duplicate(&sharp_s_pair);

    let canonical_accent_pair = [
        VocabularyEntryRef {
            spoken: "é",
            written: "accented form",
        },
        VocabularyEntryRef {
            spoken: "e\u{301}",
            written: "decomposed form",
        },
    ];
    let canonical_accent_duplicate = is_duplicate(&canonical_accent_pair);

    let original_spoken = String::from("  STRAẞE e\u{301}  ");
    let original_written = String::from("Street and accent");
    let entry = [VocabularyEntryRef {
        spoken: &original_spoken,
        written: &original_written,
    }];
    let key = comparison_key(&original_spoken, CURRENT_VOCABULARY_MATCHING_VERSION);
    let original_text_preserved = validate_vocabulary_collection(&entry).is_ok()
        && !key.is_empty()
        && original_spoken == "  STRAẞE e\u{301}  "
        && original_written == "Street and accent"
        && entry[0].spoken == "  STRAẞE e\u{301}  "
        && entry[0].written == "Street and accent";

    let checks = [
        ("Valid vocabulary mapping", valid_mapping),
        ("Empty spoken phrase rejected", empty_spoken_phrase),
        (
            "Overlong written replacement rejected",
            overlong_written_replacement,
        ),
        ("500-entry limit accepts 500", exactly_500_accepted),
        ("500-entry limit rejects 501", over_limit_rejected),
        ("ASCII case-insensitive duplicate", ascii_case_duplicate),
        (
            "German sharp-s full-fold duplicate",
            sharp_s_full_fold_duplicate,
        ),
        ("Canonical accent duplicate", canonical_accent_duplicate),
        (
            "Original vocabulary text preserved",
            original_text_preserved,
        ),
    ];

    println!("Outflow vocabulary validation demo");
    println!(
        "Matching policy: {}",
        CURRENT_VOCABULARY_MATCHING_VERSION.label()
    );
    for (label, passed) in checks {
        print_check(label, passed);
    }
    println!("Vocabulary persistence: NOT IMPLEMENTED");
    println!("Text replacement: NOT IMPLEMENTED");

    if checks.iter().all(|(_, passed)| *passed) {
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

fn is_duplicate(entries: &[VocabularyEntryRef<'_>]) -> bool {
    matches!(
        validate_vocabulary_collection(entries),
        Err(VocabularyCollectionError::DuplicateSpoken { .. })
    )
}

fn print_check(label: &str, passed: bool) {
    println!("{label}: {}", if passed { "PASS" } else { "FAIL" });
}
