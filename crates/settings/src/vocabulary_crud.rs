//! In-memory add, edit, and delete operations for C-08 vocabulary entries.
//!
//! These operations work on the schema-v1 `VocabularyEntry` collection and
//! reuse C-06's Unicode 18 canonical-caseless collection validator. Each
//! change is built and validated on a temporary clone, so errors preserve the
//! caller's complete collection. This module does not save settings, change
//! schema-v1 decoding or matching, or connect to a frontend, Tauri, speech
//! recognition, clipboard, or operating-system integration.

use crate::VocabularyEntry;
use outflow_core::vocabulary::{
    MAX_VOCABULARY_ENTRIES, VocabularyCollectionError, VocabularyEntryRef,
    VocabularyValidationError, validate_vocabulary_collection,
};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// A redacted failure while adding, editing, or deleting an in-memory entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VocabularyManagementError {
    /// Two collection entries use the same stable identifier.
    DuplicateId {
        /// Zero-based index of the first entry with this identifier.
        first_index: usize,
        /// Zero-based index of the later entry with this identifier.
        duplicate_index: usize,
    },
    /// Two spoken phrases conflict under Unicode 18 canonical-caseless matching.
    DuplicateSpoken {
        /// Zero-based index of the first matching entry.
        first_index: usize,
        /// Zero-based index of the later conflicting entry.
        duplicate_index: usize,
    },
    /// One spoken or written phrase failed the existing core validator.
    InvalidEntry {
        /// Zero-based index of the invalid entry in the candidate collection.
        index: usize,
        /// Redacted field and validation reason.
        source: VocabularyValidationError,
    },
    /// The requested stable identifier was not present in the collection.
    MissingId,
    /// Adding or retaining entries would exceed the 500-entry limit.
    CollectionFull {
        /// Maximum supported collection size.
        maximum: usize,
    },
}

impl Display for VocabularyManagementError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateId {
                first_index,
                duplicate_index,
            } => write!(
                formatter,
                "vocabulary identifiers at indices {first_index} and {duplicate_index} must be unique"
            ),
            Self::DuplicateSpoken {
                first_index,
                duplicate_index,
            } => write!(
                formatter,
                "spoken phrases at indices {first_index} and {duplicate_index} conflict under Unicode 18 matching"
            ),
            Self::InvalidEntry { index, source } => {
                write!(
                    formatter,
                    "vocabulary entry at index {index} is invalid: {source}"
                )
            }
            Self::MissingId => formatter.write_str("vocabulary entry identifier was not found"),
            Self::CollectionFull { maximum } => {
                write!(
                    formatter,
                    "vocabulary collection limit is {maximum} entries"
                )
            }
        }
    }
}

impl Error for VocabularyManagementError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidEntry { source, .. } => Some(source),
            Self::DuplicateId { .. }
            | Self::DuplicateSpoken { .. }
            | Self::MissingId
            | Self::CollectionFull { .. } => None,
        }
    }
}

/// Adds a validated entry to an in-memory vocabulary collection.
///
/// The original vector changes only when the candidate collection passes the
/// existing core phrase, Unicode 18 duplicate, and 500-entry checks, as well as
/// stable-ID uniqueness. The accepted phrase text is stored as supplied; this
/// function does not normalize it or persist the result.
///
/// # Errors
///
/// Returns [`VocabularyManagementError::DuplicateId`],
/// [`VocabularyManagementError::DuplicateSpoken`],
/// [`VocabularyManagementError::InvalidEntry`], or
/// [`VocabularyManagementError::CollectionFull`] without changing `entries`.
pub fn add_vocabulary_entry(
    entries: &mut Vec<VocabularyEntry>,
    entry: VocabularyEntry,
) -> Result<(), VocabularyManagementError> {
    let mut candidate = entries.clone();
    candidate.push(entry);
    validate_candidate(&candidate)?;
    *entries = candidate;
    Ok(())
}

/// Edits the spoken and written phrases of an existing stable ID in place.
///
/// The ID and vector position are retained. The updated collection is
/// validated before it replaces the original, so a rejected edit leaves every
/// entry unchanged. No settings file is read or written.
///
/// # Errors
///
/// Returns [`VocabularyManagementError::MissingId`] when `id` is absent, or
/// any applicable duplicate, invalid-entry, or capacity error when the
/// candidate collection is invalid. The original vector is unchanged on error.
pub fn edit_vocabulary_entry(
    entries: &mut Vec<VocabularyEntry>,
    id: &str,
    spoken: String,
    written: String,
) -> Result<(), VocabularyManagementError> {
    let index = find_unique_id(entries, id)?;
    let mut candidate = entries.clone();
    candidate[index].spoken = spoken;
    candidate[index].written = written;
    validate_candidate(&candidate)?;
    *entries = candidate;
    Ok(())
}

/// Deletes the entry identified by its stable ID from an in-memory collection.
///
/// Remaining entries keep their relative order. The candidate collection is
/// validated before it replaces the original; deleting an ambiguous duplicate
/// ID is rejected instead of removing an arbitrary row. No settings file is
/// read or written.
///
/// # Errors
///
/// Returns [`VocabularyManagementError::MissingId`] when `id` is absent, or a
/// duplicate, invalid-entry, or capacity error if the resulting collection is
/// invalid. The original vector is unchanged on error.
pub fn delete_vocabulary_entry(
    entries: &mut Vec<VocabularyEntry>,
    id: &str,
) -> Result<(), VocabularyManagementError> {
    let index = find_unique_id(entries, id)?;
    let mut candidate = entries.clone();
    candidate.remove(index);
    validate_candidate(&candidate)?;
    *entries = candidate;
    Ok(())
}

fn find_unique_id(
    entries: &[VocabularyEntry],
    id: &str,
) -> Result<usize, VocabularyManagementError> {
    let mut found = None;
    for (index, entry) in entries.iter().enumerate() {
        if entry.id == id {
            if let Some(first_index) = found {
                return Err(VocabularyManagementError::DuplicateId {
                    first_index,
                    duplicate_index: index,
                });
            }
            found = Some(index);
        }
    }
    found.ok_or(VocabularyManagementError::MissingId)
}

fn validate_candidate(entries: &[VocabularyEntry]) -> Result<(), VocabularyManagementError> {
    if entries.len() > MAX_VOCABULARY_ENTRIES {
        return Err(VocabularyManagementError::CollectionFull {
            maximum: MAX_VOCABULARY_ENTRIES,
        });
    }

    let borrowed = entries
        .iter()
        .map(|entry| VocabularyEntryRef {
            spoken: &entry.spoken,
            written: &entry.written,
        })
        .collect::<Vec<_>>();
    validate_vocabulary_collection(&borrowed).map_err(map_collection_error)?;

    let mut first_index_by_id = HashMap::<&str, usize>::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if let Some(&first_index) = first_index_by_id.get(entry.id.as_str()) {
            return Err(VocabularyManagementError::DuplicateId {
                first_index,
                duplicate_index: index,
            });
        }
        first_index_by_id.insert(&entry.id, index);
    }

    Ok(())
}

fn map_collection_error(error: VocabularyCollectionError) -> VocabularyManagementError {
    match error {
        VocabularyCollectionError::TooManyEntries { maximum, .. } => {
            VocabularyManagementError::CollectionFull { maximum }
        }
        VocabularyCollectionError::InvalidEntry { index, source } => {
            VocabularyManagementError::InvalidEntry { index, source }
        }
        VocabularyCollectionError::DuplicateSpoken {
            first_index,
            duplicate_index,
        } => VocabularyManagementError::DuplicateSpoken {
            first_index,
            duplicate_index,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        VocabularyManagementError, add_vocabulary_entry, delete_vocabulary_entry,
        edit_vocabulary_entry,
    };
    use crate::{MAX_VOCABULARY_ENTRIES, VocabularyEntry};

    fn entry(id: &str, spoken: &str, written: &str) -> VocabularyEntry {
        VocabularyEntry {
            id: id.to_owned(),
            spoken: spoken.to_owned(),
            written: written.to_owned(),
        }
    }

    fn assert_redacted(error: &VocabularyManagementError, secrets: &[&str]) {
        let rendered = format!("{error} {error:?}");
        for secret in secrets {
            assert!(!rendered.contains(secret), "error exposed private input");
        }
    }

    #[test]
    fn adds_a_valid_entry() {
        let mut entries = vec![entry("one", "existing", "Existing")];

        assert_eq!(
            add_vocabulary_entry(&mut entries, entry("two", "new phrase", "New Phrase")),
            Ok(())
        );
        assert_eq!(
            entries,
            vec![
                entry("one", "existing", "Existing"),
                entry("two", "new phrase", "New Phrase"),
            ]
        );
    }

    #[test]
    fn edits_values_while_preserving_id_and_position() {
        let mut entries = vec![
            entry("first", "alpha", "Alpha"),
            entry("target", "old spoken", "Old written"),
            entry("last", "omega", "Omega"),
        ];

        assert_eq!(
            edit_vocabulary_entry(
                &mut entries,
                "target",
                "new spoken".to_owned(),
                "New written".to_owned(),
            ),
            Ok(())
        );
        assert_eq!(
            entries,
            vec![
                entry("first", "alpha", "Alpha"),
                entry("target", "new spoken", "New written"),
                entry("last", "omega", "Omega"),
            ]
        );
    }

    #[test]
    fn deletes_by_stable_id_and_preserves_remaining_order() {
        let mut entries = vec![
            entry("first", "alpha", "Alpha"),
            entry("target", "middle", "Middle"),
            entry("last", "omega", "Omega"),
        ];

        assert_eq!(delete_vocabulary_entry(&mut entries, "target"), Ok(()));
        assert_eq!(
            entries,
            vec![
                entry("first", "alpha", "Alpha"),
                entry("last", "omega", "Omega")
            ]
        );
    }

    #[test]
    fn rejects_missing_ids_for_edit_and_delete_without_mutation() {
        let mut entries = vec![entry("existing-id", "private phrase", "private value")];
        let original = entries.clone();

        let edit_error = edit_vocabulary_entry(
            &mut entries,
            "missing-private-id",
            "replacement".to_owned(),
            "Replacement".to_owned(),
        )
        .expect_err("missing edit target must be rejected");
        assert_eq!(edit_error, VocabularyManagementError::MissingId);
        assert_redacted(&edit_error, &["missing-private-id", "private phrase"]);
        assert_eq!(entries, original);

        let delete_error = delete_vocabulary_entry(&mut entries, "missing-private-id")
            .expect_err("missing delete target must be rejected");
        assert_eq!(delete_error, VocabularyManagementError::MissingId);
        assert_redacted(&delete_error, &["missing-private-id", "private phrase"]);
        assert_eq!(entries, original);
    }

    #[test]
    fn rejects_duplicate_ids_without_mutating_the_collection() {
        let mut entries = vec![entry("private-id", "first phrase", "First")];
        let original = entries.clone();
        let error =
            add_vocabulary_entry(&mut entries, entry("private-id", "second phrase", "Second"))
                .expect_err("duplicate IDs must be rejected");

        assert!(matches!(
            error,
            VocabularyManagementError::DuplicateId { .. }
        ));
        assert_redacted(&error, &["private-id", "first phrase", "second phrase"]);
        assert_eq!(entries, original);
    }

    #[test]
    fn rejects_an_ambiguous_duplicate_id_for_edit_and_delete() {
        let mut entries = vec![
            entry("duplicate-id", "first phrase", "First"),
            entry("duplicate-id", "second phrase", "Second"),
        ];
        let original = entries.clone();

        let edit_error = edit_vocabulary_entry(
            &mut entries,
            "duplicate-id",
            "edited phrase".to_owned(),
            "Edited".to_owned(),
        )
        .expect_err("editing an ambiguous ID must be rejected");
        assert!(matches!(
            edit_error,
            VocabularyManagementError::DuplicateId { .. }
        ));
        assert_eq!(entries, original);

        let delete_error = delete_vocabulary_entry(&mut entries, "duplicate-id")
            .expect_err("deleting an ambiguous ID must be rejected");
        assert!(matches!(
            delete_error,
            VocabularyManagementError::DuplicateId { .. }
        ));
        assert_eq!(entries, original);
    }

    #[test]
    fn rejects_sharp_s_and_full_case_fold_duplicates_atomically() {
        assert_duplicate_rejected("straße", "STRASSE");
    }

    #[test]
    fn rejects_composed_and_decomposed_duplicates_atomically() {
        assert_duplicate_rejected("café", "cafe\u{301}");
    }

    fn assert_duplicate_rejected(first: &str, second: &str) {
        let mut entries = vec![entry("one", first, "First replacement")];
        let original = entries.clone();
        let error = add_vocabulary_entry(&mut entries, entry("two", second, "Second replacement"))
            .expect_err("canonical-caseless duplicates must be rejected");

        assert!(matches!(
            error,
            VocabularyManagementError::DuplicateSpoken { .. }
        ));
        assert_redacted(
            &error,
            &[first, second, "First replacement", "Second replacement"],
        );
        assert_eq!(entries, original);
    }

    #[test]
    fn accepts_unicode_and_emoji_without_normalizing_stored_text() {
        let spoken = "東京 😀 e\u{301}";
        let written = "naïve 文 🧭";
        let mut entries = Vec::new();

        assert_eq!(
            add_vocabulary_entry(&mut entries, entry("unicode", spoken, written)),
            Ok(())
        );
        assert_eq!(entries, vec![entry("unicode", spoken, written)]);
    }

    #[test]
    fn counts_unicode_scalar_values_for_the_128_character_field_limit() {
        let exactly_128_emoji = "😀".repeat(128);
        let mut entries = Vec::new();
        assert_eq!(
            add_vocabulary_entry(
                &mut entries,
                entry("boundary", &exactly_128_emoji, "written")
            ),
            Ok(())
        );

        let original = entries.clone();
        let too_long = "😀".repeat(129);
        let error = add_vocabulary_entry(&mut entries, entry("too-long", &too_long, "written"))
            .expect_err("129 Unicode scalar values must be rejected");
        assert!(matches!(
            error,
            VocabularyManagementError::InvalidEntry { .. }
        ));
        assert_redacted(&error, &[&too_long]);
        assert_eq!(entries, original);
    }

    #[test]
    fn enforces_the_field_limit_for_spoken_and_written_values_during_edit() {
        let too_long = "x".repeat(129);
        for (spoken, written) in [
            (too_long.clone(), "valid".to_owned()),
            ("valid".to_owned(), too_long.clone()),
        ] {
            let mut entries = vec![entry("stable", "original", "Original")];
            let original = entries.clone();
            let error = edit_vocabulary_entry(&mut entries, "stable", spoken, written)
                .expect_err("overlong field must be rejected");
            assert!(matches!(
                error,
                VocabularyManagementError::InvalidEntry { .. }
            ));
            assert_redacted(&error, &[&too_long]);
            assert_eq!(entries, original);
        }
    }

    #[test]
    fn accepts_500_entries_and_rejects_the_501st_without_mutation() {
        let mut entries = (0..MAX_VOCABULARY_ENTRIES - 1)
            .map(|index| {
                entry(
                    &format!("id-{index}"),
                    &format!("phrase-{index}"),
                    "Replacement",
                )
            })
            .collect::<Vec<_>>();
        add_vocabulary_entry(&mut entries, entry("last", "phrase-last", "Replacement"))
            .expect("the 500th valid entry should be accepted");
        assert_eq!(entries.len(), MAX_VOCABULARY_ENTRIES);

        let original = entries.clone();
        let error = add_vocabulary_entry(
            &mut entries,
            entry("overflow", "overflow phrase", "Replacement"),
        )
        .expect_err("the 501st entry must be rejected");
        assert_eq!(
            error,
            VocabularyManagementError::CollectionFull {
                maximum: MAX_VOCABULARY_ENTRIES,
            }
        );
        assert_redacted(&error, &["overflow", "overflow phrase"]);
        assert_eq!(entries, original);
    }

    #[test]
    fn rejected_duplicate_and_invalid_edits_preserve_the_entire_collection() {
        let mut entries = vec![
            entry("first", "first phrase", "First"),
            entry("target", "second phrase", "Second"),
            entry("last", "last phrase", "Last"),
        ];
        let original = entries.clone();

        let duplicate_error = edit_vocabulary_entry(
            &mut entries,
            "target",
            "FIRST PHRASE".to_owned(),
            "Changed".to_owned(),
        )
        .expect_err("a duplicate spoken phrase must be rejected");
        assert!(matches!(
            duplicate_error,
            VocabularyManagementError::DuplicateSpoken { .. }
        ));
        assert_eq!(entries, original);

        let invalid_error =
            edit_vocabulary_entry(&mut entries, "target", "".to_owned(), "Changed".to_owned())
                .expect_err("an empty spoken phrase must be rejected");
        assert!(matches!(
            invalid_error,
            VocabularyManagementError::InvalidEntry { .. }
        ));
        assert_redacted(&invalid_error, &["first phrase", "second phrase"]);
        assert_eq!(entries, original);
    }

    #[test]
    fn deleting_from_an_invalid_candidate_is_rejected_atomically() {
        let mut entries = vec![
            entry("remove", "to remove", "Remove"),
            entry("duplicate", "same phrase", "First"),
            entry("other", "SAME PHRASE", "Second"),
        ];
        let original = entries.clone();
        let error = delete_vocabulary_entry(&mut entries, "remove")
            .expect_err("remaining invalid duplicate phrases must be rejected");

        assert!(matches!(
            error,
            VocabularyManagementError::DuplicateSpoken { .. }
        ));
        assert_redacted(&error, &["same phrase", "SAME PHRASE"]);
        assert_eq!(entries, original);
    }
}
