//! Pure custom-vocabulary phrase validation for C-06 and C-08.
//!
//! This module checks only nonempty content and the documented 128 Unicode
//! scalar-value limit. It does not normalize or rewrite accepted phrases, and
//! it performs no storage, network, logging, or global-state operations.
//!
//! Rust `&str` values are already valid UTF-8 and therefore cannot contain
//! unpaired surrogate code points. A frontend validator receives JavaScript
//! UTF-16 strings; malformed surrogate sequences must be rejected when those
//! strings are converted to Rust text (for example, with
//! `String::from_utf16`), before they can reach this API. Embedded controls and
//! other pathological-input rules have no repository-defined policy yet and
//! remain unresolved here.

use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::vocabulary_matching::{CURRENT_VOCABULARY_MATCHING_VERSION, comparison_key};

/// Maximum number of Unicode scalar values in either vocabulary phrase.
pub const MAX_VOCABULARY_FIELD_SCALARS: usize = 128;

/// Maximum number of entries accepted by collection validation.
pub const MAX_VOCABULARY_ENTRIES: usize = 500;

/// A borrowed, read-only view of one custom vocabulary entry.
///
/// This type owns no phrase data. Collection validation uses the references
/// only during the call and never changes either value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VocabularyEntryRef<'a> {
    /// The phrase expected from speech recognition.
    pub spoken: &'a str,
    /// The phrase to use as its written replacement.
    pub written: &'a str,
}

/// Identifies which side of a vocabulary entry failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocabularyField {
    /// The phrase expected from speech recognition.
    Spoken,
    /// The replacement phrase to write.
    Written,
}

impl Display for VocabularyField {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spoken => formatter.write_str("spoken"),
            Self::Written => formatter.write_str("written"),
        }
    }
}

/// A redacted vocabulary phrase validation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocabularyValidationError {
    /// The phrase is empty or contains only Unicode whitespace.
    Empty(VocabularyField),
    /// The phrase contains more than [`MAX_VOCABULARY_FIELD_SCALARS`] scalars.
    TooLong(VocabularyField),
}

impl Display for VocabularyValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty(field) => write!(
                formatter,
                "{field} vocabulary phrase must not be empty or whitespace-only"
            ),
            Self::TooLong(field) => write!(
                formatter,
                "{field} vocabulary phrase exceeds {MAX_VOCABULARY_FIELD_SCALARS} Unicode scalar values"
            ),
        }
    }
}

impl Error for VocabularyValidationError {}

/// A redacted failure while validating a vocabulary collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VocabularyCollectionError {
    /// The collection exceeds [`MAX_VOCABULARY_ENTRIES`].
    TooManyEntries {
        /// Number of entries supplied by the caller.
        count: usize,
        /// Maximum supported collection size.
        maximum: usize,
    },
    /// One entry failed the existing phrase validator.
    InvalidEntry {
        /// Zero-based index of the invalid entry.
        index: usize,
        /// Redacted field and validation reason.
        source: VocabularyValidationError,
    },
    /// Two spoken phrases have the same versioned canonical-caseless key.
    DuplicateSpoken {
        /// Zero-based index of the first matching entry.
        first_index: usize,
        /// Zero-based index of the later conflicting entry.
        duplicate_index: usize,
    },
}

impl Display for VocabularyCollectionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyEntries { count, maximum } => write!(
                formatter,
                "vocabulary collection has {count} entries; maximum is {maximum}"
            ),
            Self::InvalidEntry { index, source } => {
                write!(
                    formatter,
                    "vocabulary entry at index {index} is invalid: {source}"
                )
            }
            Self::DuplicateSpoken {
                first_index,
                duplicate_index,
            } => write!(
                formatter,
                "spoken phrase at index {duplicate_index} conflicts with entry at index {first_index}"
            ),
        }
    }
}

impl Error for VocabularyCollectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidEntry { source, .. } => Some(source),
            Self::TooManyEntries { .. } | Self::DuplicateSpoken { .. } => None,
        }
    }
}

/// Validates a read-only collection of vocabulary entries.
///
/// Validation precedence is deterministic: collection size is checked first,
/// then each entry is checked by [`validate_vocabulary_entry`] in input order,
/// and only after all entries are valid are spoken phrases checked for
/// conflicts in input order. The first error in that order is returned.
///
/// Spoken phrase conflicts use the key identified by
/// [`CURRENT_VOCABULARY_MATCHING_VERSION`], currently Unicode 18.0.0 canonical
/// caseless matching (D145). This applies NFD, default full case folding, and
/// NFD again. It does not trim, use locale-specific folding, apply compatibility
/// normalization, strip accents, or change accepted input. Written phrases do
/// not determine uniqueness. This function does not perform text replacement
/// or word-boundary matching, persist values, or retain phrase references
/// beyond this call.
///
/// # Errors
///
/// Returns [`VocabularyCollectionError::TooManyEntries`] when the slice
/// exceeds the collection limit, [`VocabularyCollectionError::InvalidEntry`]
/// for the first invalid entry in input order, or
/// [`VocabularyCollectionError::DuplicateSpoken`] for the first conflicting
/// pair after every entry has passed field validation.
pub fn validate_vocabulary_collection(
    entries: &[VocabularyEntryRef<'_>],
) -> Result<(), VocabularyCollectionError> {
    if entries.len() > MAX_VOCABULARY_ENTRIES {
        return Err(VocabularyCollectionError::TooManyEntries {
            count: entries.len(),
            maximum: MAX_VOCABULARY_ENTRIES,
        });
    }

    for (index, entry) in entries.iter().enumerate() {
        validate_vocabulary_entry(entry.spoken, entry.written)
            .map_err(|source| VocabularyCollectionError::InvalidEntry { index, source })?;
    }

    let mut first_index_by_key = HashMap::<String, usize>::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let key = comparison_key(entry.spoken, CURRENT_VOCABULARY_MATCHING_VERSION);
        if let Some(&first_index) = first_index_by_key.get(&key) {
            return Err(VocabularyCollectionError::DuplicateSpoken {
                first_index,
                duplicate_index: index,
            });
        }
        first_index_by_key.insert(key, index);
    }

    Ok(())
}

/// Validates the spoken and written phrases without changing either value.
///
/// Each phrase must contain at least one non-whitespace Unicode scalar value
/// and at most 128 Unicode scalar values. Rust's `str::chars()` iterates those
/// scalar values, so a multibyte UTF-8 character such as an emoji counts once
/// and a base character plus combining mark counts twice. The spoken field is
/// checked first if both fields are invalid.
///
/// This function does not trim, normalize, case-fold, or otherwise transform
/// accepted text. It also does not check duplicates, controls, storage limits,
/// or replacement behavior.
pub fn validate_vocabulary_entry(
    spoken: &str,
    written: &str,
) -> Result<(), VocabularyValidationError> {
    validate_field(spoken, VocabularyField::Spoken)?;
    validate_field(written, VocabularyField::Written)
}

fn validate_field(value: &str, field: VocabularyField) -> Result<(), VocabularyValidationError> {
    if value.chars().all(char::is_whitespace) {
        return Err(VocabularyValidationError::Empty(field));
    }

    // Stop after one scalar beyond the limit; counting UTF-8 bytes would
    // incorrectly reject multibyte characters that are within the contract.
    if value.chars().take(MAX_VOCABULARY_FIELD_SCALARS + 1).count() > MAX_VOCABULARY_FIELD_SCALARS {
        return Err(VocabularyValidationError::TooLong(field));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_VOCABULARY_ENTRIES, MAX_VOCABULARY_FIELD_SCALARS, VocabularyCollectionError,
        VocabularyEntryRef, VocabularyField, VocabularyValidationError,
        validate_vocabulary_collection, validate_vocabulary_entry,
    };

    #[test]
    fn accepts_nonempty_spoken_and_written_values() {
        assert_eq!(
            validate_vocabulary_entry("north parkade", "North Parkade"),
            Ok(())
        );
    }

    #[test]
    fn rejects_empty_spoken_field_with_field_and_reason() {
        assert_eq!(
            validate_vocabulary_entry("", "replacement"),
            Err(VocabularyValidationError::Empty(VocabularyField::Spoken))
        );
    }

    #[test]
    fn rejects_empty_written_field_with_field_and_reason() {
        assert_eq!(
            validate_vocabulary_entry("source", ""),
            Err(VocabularyValidationError::Empty(VocabularyField::Written))
        );
    }

    #[test]
    fn rejects_whitespace_only_spoken_field() {
        assert_eq!(
            validate_vocabulary_entry(" \t\n", "replacement"),
            Err(VocabularyValidationError::Empty(VocabularyField::Spoken))
        );
    }

    #[test]
    fn rejects_whitespace_only_written_field() {
        assert_eq!(
            validate_vocabulary_entry("source", " \t\n"),
            Err(VocabularyValidationError::Empty(VocabularyField::Written))
        );
    }

    #[test]
    fn accepts_exactly_128_ascii_scalars_in_both_fields() {
        let at_limit = "a".repeat(MAX_VOCABULARY_FIELD_SCALARS);

        assert_eq!(validate_vocabulary_entry(&at_limit, "replacement"), Ok(()));
        assert_eq!(validate_vocabulary_entry("source", &at_limit), Ok(()));
    }

    #[test]
    fn rejects_129_ascii_scalars_in_each_field() {
        let over_limit = "a".repeat(MAX_VOCABULARY_FIELD_SCALARS + 1);

        assert_eq!(
            validate_vocabulary_entry(&over_limit, "replacement"),
            Err(VocabularyValidationError::TooLong(VocabularyField::Spoken))
        );
        assert_eq!(
            validate_vocabulary_entry("source", &over_limit),
            Err(VocabularyValidationError::TooLong(VocabularyField::Written))
        );
    }

    #[test]
    fn accepts_exactly_128_emoji_scalars_in_both_fields() {
        let at_limit = "😀".repeat(MAX_VOCABULARY_FIELD_SCALARS);

        assert_eq!(validate_vocabulary_entry(&at_limit, "replacement"), Ok(()));
        assert_eq!(validate_vocabulary_entry("source", &at_limit), Ok(()));
    }

    #[test]
    fn rejects_129_emoji_scalars_in_each_field() {
        let over_limit = "😀".repeat(MAX_VOCABULARY_FIELD_SCALARS + 1);

        assert_eq!(
            validate_vocabulary_entry(&over_limit, "replacement"),
            Err(VocabularyValidationError::TooLong(VocabularyField::Spoken))
        );
        assert_eq!(
            validate_vocabulary_entry("source", &over_limit),
            Err(VocabularyValidationError::TooLong(VocabularyField::Written))
        );
    }

    #[test]
    fn accepts_non_ascii_text() {
        assert_eq!(validate_vocabulary_entry("café", "München"), Ok(()));
    }

    #[test]
    fn counts_combining_mark_as_a_separate_scalar_without_normalizing() {
        let decomposed = "e\u{301}";

        assert_eq!(decomposed.chars().count(), 2);
        assert_eq!(validate_vocabulary_entry(decomposed, "café"), Ok(()));
        assert_eq!(decomposed, "e\u{301}");
    }

    #[test]
    fn rejects_unicode_whitespace_only_values_in_both_fields() {
        let unicode_whitespace = "\u{2003}\u{3000}";

        assert_eq!(
            validate_vocabulary_entry(unicode_whitespace, "replacement"),
            Err(VocabularyValidationError::Empty(VocabularyField::Spoken))
        );
        assert_eq!(
            validate_vocabulary_entry("source", unicode_whitespace),
            Err(VocabularyValidationError::Empty(VocabularyField::Written))
        );
    }

    #[test]
    fn accepted_values_remain_unchanged() {
        let spoken = " North Parkade ";
        let written = "NORTH PARKADE!";

        assert_eq!(validate_vocabulary_entry(spoken, written), Ok(()));
        assert!(spoken == " North Parkade ");
        assert!(written == "NORTH PARKADE!");
    }

    #[test]
    fn validation_errors_do_not_include_phrase_contents() {
        let private_value = "private phrase";
        let error = validate_vocabulary_entry(
            &format!(
                "{private_value}{}",
                "x".repeat(MAX_VOCABULARY_FIELD_SCALARS)
            ),
            "replacement",
        )
        .expect_err("the synthetic phrase exceeds the scalar limit");

        assert_eq!(
            error,
            VocabularyValidationError::TooLong(VocabularyField::Spoken)
        );
        assert!(!error.to_string().contains(private_value));
    }

    #[test]
    fn repeated_calls_return_the_same_result() {
        let too_long = "a".repeat(MAX_VOCABULARY_FIELD_SCALARS + 1);
        for (spoken, written) in [
            ("source", "replacement"),
            (" \t", "replacement"),
            ("source", too_long.as_str()),
        ] {
            let expected = validate_vocabulary_entry(spoken, written);
            for _ in 0..3 {
                assert_eq!(validate_vocabulary_entry(spoken, written), expected);
            }
        }
    }

    #[test]
    fn accepts_empty_collection() {
        assert_eq!(validate_vocabulary_collection(&[]), Ok(()));
    }

    #[test]
    fn accepts_one_valid_entry() {
        let entries = [VocabularyEntryRef {
            spoken: "north parkade",
            written: "North Parkade",
        }];

        assert_eq!(validate_vocabulary_collection(&entries), Ok(()));
    }

    #[test]
    fn accepts_multiple_distinct_valid_entries() {
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
                spoken: "café",
                written: "Cafe",
            },
        ];

        assert_eq!(validate_vocabulary_collection(&entries), Ok(()));
    }

    #[test]
    fn accepts_exactly_500_valid_entries() {
        let owned_entries = (0..MAX_VOCABULARY_ENTRIES)
            .map(|index| (format!("spoken-{index}"), format!("written-{index}")))
            .collect::<Vec<_>>();
        let entries = owned_entries
            .iter()
            .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();

        assert_eq!(validate_vocabulary_collection(&entries), Ok(()));
    }

    #[test]
    fn rejects_501_entries_and_reports_count_and_limit() {
        let owned_entries = (0..=MAX_VOCABULARY_ENTRIES)
            .map(|index| (format!("spoken-{index}"), format!("written-{index}")))
            .collect::<Vec<_>>();
        let entries = owned_entries
            .iter()
            .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::TooManyEntries {
                count: MAX_VOCABULARY_ENTRIES + 1,
                maximum: MAX_VOCABULARY_ENTRIES,
            })
        );
    }

    #[test]
    fn invalid_spoken_field_reuses_entry_error_and_reports_index() {
        let entries = [
            VocabularyEntryRef {
                spoken: "valid",
                written: "replacement",
            },
            VocabularyEntryRef {
                spoken: " \t",
                written: "replacement",
            },
        ];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 1,
                source: VocabularyValidationError::Empty(VocabularyField::Spoken),
            })
        );
    }

    #[test]
    fn invalid_written_field_reuses_entry_error_and_reports_index() {
        let entries = [VocabularyEntryRef {
            spoken: "valid",
            written: "\u{2003}",
        }];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            })
        );
    }

    #[test]
    fn overlong_spoken_field_reports_existing_validation_reason() {
        let overlong = "s".repeat(MAX_VOCABULARY_FIELD_SCALARS + 1);
        let entries = [VocabularyEntryRef {
            spoken: &overlong,
            written: "replacement",
        }];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::TooLong(VocabularyField::Spoken),
            })
        );
    }

    #[test]
    fn overlong_written_field_reports_existing_validation_reason() {
        let overlong = "w".repeat(MAX_VOCABULARY_FIELD_SCALARS + 1);
        let entries = [VocabularyEntryRef {
            spoken: "source",
            written: &overlong,
        }];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::TooLong(VocabularyField::Written),
            })
        );
    }

    #[test]
    fn first_invalid_entry_is_reported_in_input_order() {
        let entries = [
            VocabularyEntryRef {
                spoken: "valid",
                written: "replacement",
            },
            VocabularyEntryRef {
                spoken: "",
                written: "replacement",
            },
            VocabularyEntryRef {
                spoken: "valid",
                written: " ",
            },
        ];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 1,
                source: VocabularyValidationError::Empty(VocabularyField::Spoken),
            })
        );
    }

    #[test]
    fn rejects_exact_duplicate_and_reports_both_indices() {
        let entries = [
            VocabularyEntryRef {
                spoken: "north parkade",
                written: "North Parkade",
            },
            VocabularyEntryRef {
                spoken: "north parkade",
                written: "Northern Parking Area",
            },
        ];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn rejects_ascii_case_variations() {
        let entries = [
            VocabularyEntryRef {
                spoken: "rust",
                written: "Rust",
            },
            VocabularyEntryRef {
                spoken: "RUST",
                written: "Rust Language",
            },
        ];

        assert!(matches!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        ));
    }

    #[test]
    fn rejects_non_ascii_lowercase_variations() {
        let entries = [
            VocabularyEntryRef {
                spoken: "CAFÉ",
                written: "Cafe",
            },
            VocabularyEntryRef {
                spoken: "café",
                written: "Coffee Shop",
            },
        ];

        assert!(matches!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        ));
    }

    #[test]
    fn spoken_phrase_conflicts_even_when_written_replacements_differ() {
        let entries = [
            VocabularyEntryRef {
                spoken: "rust",
                written: "Rust",
            },
            VocabularyEntryRef {
                spoken: "RUST",
                written: "Rust Language",
            },
        ];

        assert!(matches!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::DuplicateSpoken { .. })
        ));
    }

    #[test]
    fn same_written_replacement_is_allowed_for_distinct_spoken_phrases() {
        let entries = [
            VocabularyEntryRef {
                spoken: "first phrase",
                written: "Shared replacement",
            },
            VocabularyEntryRef {
                spoken: "second phrase",
                written: "Shared replacement",
            },
        ];

        assert_eq!(validate_vocabulary_collection(&entries), Ok(()));
    }

    #[test]
    fn repeated_collection_calls_return_identical_results() {
        let entries = [
            VocabularyEntryRef {
                spoken: "first phrase",
                written: "one",
            },
            VocabularyEntryRef {
                spoken: "RUST",
                written: "two",
            },
            VocabularyEntryRef {
                spoken: "rust",
                written: "three",
            },
        ];
        let expected = validate_vocabulary_collection(&entries);

        for _ in 0..5 {
            assert_eq!(validate_vocabulary_collection(&entries), expected);
        }
    }

    #[test]
    fn collection_validation_does_not_modify_input_values() {
        let spoken = String::from(" Rust ");
        let written = String::from("Rust Language");
        let entries = [VocabularyEntryRef {
            spoken: &spoken,
            written: &written,
        }];

        assert_eq!(validate_vocabulary_collection(&entries), Ok(()));
        assert_eq!(spoken, " Rust ");
        assert_eq!(written, "Rust Language");
        assert_eq!(entries[0].spoken, " Rust ");
        assert_eq!(entries[0].written, "Rust Language");
    }

    #[test]
    fn full_unicode_case_folding_detects_sharp_s_duplicates() {
        let entries = [
            VocabularyEntryRef {
                spoken: "straße",
                written: "street",
            },
            VocabularyEntryRef {
                spoken: "STRASSE",
                written: "Street",
            },
        ];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn collection_validator_uses_the_task_43_comparison_matrix() {
        let cases = [
            ("rust", "Rust", true),
            ("CAFÉ", "café", true),
            ("straße", "STRASSE", true),
            ("é", "e\u{301}", true),
            ("Σ", "σ", true),
            ("σ", "ς", true),
            ("North Parkade", "north parkade", true),
            ("north parkade", "north parkade ", false),
            ("I", "ı", false),
            ("İ", "i", false),
            ("①", "1", false),
            ("a", "а", false),
        ];

        for (first, second, expected_duplicate) in cases {
            let entries = [
                VocabularyEntryRef {
                    spoken: first,
                    written: "first replacement",
                },
                VocabularyEntryRef {
                    spoken: second,
                    written: "second replacement",
                },
            ];
            let result = validate_vocabulary_collection(&entries);
            if expected_duplicate {
                assert_eq!(
                    result,
                    Err(VocabularyCollectionError::DuplicateSpoken {
                        first_index: 0,
                        duplicate_index: 1,
                    }),
                    "expected a duplicate for {first:?} and {second:?}"
                );
            } else {
                assert_eq!(
                    result,
                    Ok(()),
                    "expected distinct spoken phrases for {first:?} and {second:?}"
                );
            }
        }
    }

    #[test]
    fn unicode_duplicate_with_different_replacements_is_rejected() {
        let entries = [
            VocabularyEntryRef {
                spoken: "é",
                written: "first replacement",
            },
            VocabularyEntryRef {
                spoken: "e\u{301}",
                written: "different replacement",
            },
        ];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: 1,
            })
        );
    }

    #[test]
    fn unicode_duplicate_at_the_end_of_a_500_entry_collection_is_reported() {
        let mut owned_entries = Vec::with_capacity(MAX_VOCABULARY_ENTRIES);
        owned_entries.push((String::from("straße"), String::from("first replacement")));
        for index in 1..MAX_VOCABULARY_ENTRIES - 1 {
            owned_entries.push((format!("entry-{index}"), format!("replacement-{index}")));
        }
        owned_entries.push((String::from("STRASSE"), String::from("last replacement")));
        let entries = owned_entries
            .iter()
            .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
            .collect::<Vec<_>>();

        assert_eq!(entries.len(), MAX_VOCABULARY_ENTRIES);
        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 0,
                duplicate_index: MAX_VOCABULARY_ENTRIES - 1,
            })
        );
    }

    #[test]
    fn collection_validation_does_not_perform_word_boundary_replacement() {
        let entries = [
            VocabularyEntryRef {
                spoken: "rust",
                written: "Rust",
            },
            VocabularyEntryRef {
                spoken: "crust",
                written: "Crust",
            },
        ];

        // Both entries are validated independently; no text is scanned or
        // rewritten to check replacement boundaries.
        assert_eq!(validate_vocabulary_collection(&entries), Ok(()));
    }

    #[test]
    fn collection_size_error_precedes_entry_errors() {
        let mut entries = vec![VocabularyEntryRef {
            spoken: "",
            written: "replacement",
        }];
        entries.resize(
            MAX_VOCABULARY_ENTRIES + 1,
            VocabularyEntryRef {
                spoken: "",
                written: "replacement",
            },
        );

        assert!(matches!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::TooManyEntries { .. })
        ));
    }

    #[test]
    fn entry_validation_precedes_duplicate_detection() {
        let entries = [
            VocabularyEntryRef {
                spoken: "rust",
                written: "Rust",
            },
            VocabularyEntryRef {
                spoken: "RUST",
                written: "Rust Language",
            },
            VocabularyEntryRef {
                spoken: " ",
                written: "replacement",
            },
        ];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::InvalidEntry {
                index: 2,
                source: VocabularyValidationError::Empty(VocabularyField::Spoken),
            })
        );
    }

    #[test]
    fn duplicate_detection_reports_first_conflict_in_input_order() {
        let entries = [
            VocabularyEntryRef {
                spoken: "alpha",
                written: "one",
            },
            VocabularyEntryRef {
                spoken: "beta",
                written: "two",
            },
            VocabularyEntryRef {
                spoken: "BETA",
                written: "three",
            },
            VocabularyEntryRef {
                spoken: "ALPHA",
                written: "four",
            },
        ];

        assert_eq!(
            validate_vocabulary_collection(&entries),
            Err(VocabularyCollectionError::DuplicateSpoken {
                first_index: 1,
                duplicate_index: 2,
            })
        );
    }

    #[test]
    fn collection_errors_do_not_expose_vocabulary_values() {
        let entries = [
            VocabularyEntryRef {
                spoken: "sensitive spoken sample",
                written: "first replacement",
            },
            VocabularyEntryRef {
                spoken: "SENSITIVE SPOKEN SAMPLE",
                written: "second replacement",
            },
        ];
        let error = validate_vocabulary_collection(&entries)
            .expect_err("the synthetic spoken phrases have the same lowercase key");
        let message = error.to_string();

        assert!(message.contains("0"));
        assert!(message.contains("1"));
        assert!(!message.contains("sensitive spoken sample"));
        assert!(!message.contains("first replacement"));
        assert!(!message.contains("second replacement"));
    }
}
