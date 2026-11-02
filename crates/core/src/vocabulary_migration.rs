//! Read-only compatibility inspection for moving vocabulary matching policies.
//!
//! This module owns no vocabulary data and performs no settings or filesystem
//! access. It validates each borrowed entry using C-06/C-08's existing rules,
//! then compares every spoken-phrase pair under the legacy Rust lowercase
//! behavior and the current Unicode 18.0.0 canonical-caseless policy. Its
//! temporary comparison keys are dropped before returning; original strings
//! remain untouched.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use crate::vocabulary::{
    MAX_VOCABULARY_ENTRIES, VocabularyEntryRef, VocabularyValidationError,
    validate_vocabulary_entry,
};
use crate::vocabulary_matching::{VocabularyMatchingVersion, comparison_key};

/// Stable identifiers for the policies compared by this compatibility check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VocabularyMatchingPolicyId {
    /// Legacy matching by Rust's `str::to_lowercase()`.
    LegacyRustLowercaseV1,
    /// Unicode 18.0.0 canonical-caseless matching using ICU4X compiled data.
    Unicode18CanonicalCaselessV1,
}

/// Matching policy supported by trusted evidence for the source collection.
///
/// The caller must supply verified provenance before selecting either
/// `Verified` variant. A settings schema version, including schema v1, does
/// not establish which matching policy was used to validate or write a file.
/// Use [`VocabularySourcePolicy::Unknown`] when that evidence is absent or
/// untrusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VocabularySourcePolicy {
    /// The source matching policy cannot be established from trusted evidence.
    Unknown,
    /// Trusted evidence establishes Rust `str::to_lowercase()` validation.
    VerifiedLegacyLowercase,
    /// Trusted evidence establishes Unicode 18.0.0 canonical-caseless V1 validation.
    VerifiedUnicode18CanonicalCaselessV1,
}

/// Readiness of a validated vocabulary collection for a possible future migration.
///
/// These outcomes describe assessment only. None of them writes a settings
/// file, changes a schema, resolves conflicts, or authorizes persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VocabularyMigrationReadiness {
    /// There are no vocabulary entries to migrate, regardless of provenance.
    ///
    /// This says nothing about the provenance or rewrite safety of the complete
    /// settings file.
    EmptyCollection,
    /// Entries exist, but trusted evidence does not establish their source policy.
    ///
    /// Conflicts are reported against both policies where found. An empty
    /// conflict list does not make an unknown-provenance collection eligible
    /// for automatic migration.
    SourcePolicyUnknown {
        /// Ordered, index-only conflicts found by the compatibility checker.
        conflicts: Vec<VocabularyMigrationConflict>,
    },
    /// One or more conflicting pairs prevent safe adoption of the target policy.
    ///
    /// The report preserves indices and classifications; it does not select a
    /// preferred entry or rewrite either mapping.
    ManualResolutionRequired {
        /// Ordered conflicts requiring explicit review.
        conflicts: Vec<VocabularyMigrationConflict>,
    },
    /// Verified legacy data is valid under both source and target policies.
    ///
    /// This is compatibility with a future explicit migration, not a migration
    /// operation or permission to rewrite settings.
    CompatibleForExplicitMigration,
    /// Verified data already uses the current policy and has no conflicts.
    AlreadyCurrent,
}

impl VocabularyMatchingPolicyId {
    /// Returns the stable machine-readable identifier for this policy.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::LegacyRustLowercaseV1 => "rust-str-to-lowercase-v1",
            Self::Unicode18CanonicalCaselessV1 => "unicode-18.0.0-canonical-caseless-v1",
        }
    }

    /// Returns a concise human-readable policy name.
    pub const fn label(self) -> &'static str {
        match self {
            Self::LegacyRustLowercaseV1 => "Legacy Rust lowercase",
            Self::Unicode18CanonicalCaselessV1 => "Unicode 18.0.0 canonical caseless V1",
        }
    }
}

/// Why two spoken phrases are reported as a compatibility conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VocabularyMigrationConflictKind {
    /// The pair already compares equal under the legacy lowercase policy.
    ExistingLegacyConflict,
    /// The pair differs under legacy lowercase but compares equal under Unicode 18.
    NewUnicode18Conflict,
}

/// One conflicting pair, identified only by its stable input positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VocabularyMigrationConflict {
    /// Index of the earlier entry in the caller's input slice.
    pub original_index: usize,
    /// Index of the later entry that conflicts with `original_index`.
    pub conflicting_index: usize,
    /// Whether the conflict existed under lowercase matching or is new under Unicode 18.
    pub classification: VocabularyMigrationConflictKind,
    /// Policy used as the source behavior for the comparison.
    pub source_policy: VocabularyMatchingPolicyId,
    /// Policy considered as the target behavior for the comparison.
    pub target_policy: VocabularyMatchingPolicyId,
}

/// A redacted error encountered before compatibility comparisons begin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VocabularyMigrationError {
    /// The input exceeds the existing maximum vocabulary size.
    TooManyEntries {
        /// Number of supplied entries.
        count: usize,
        /// Existing maximum number of entries.
        maximum: usize,
    },
    /// An entry failed the existing per-entry validator.
    InvalidEntry {
        /// Zero-based index of the invalid entry.
        index: usize,
        /// Typed validation reason, with no user text.
        source: VocabularyValidationError,
    },
}

impl Display for VocabularyMigrationError {
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
        }
    }
}

impl Error for VocabularyMigrationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidEntry { source, .. } => Some(source),
            Self::TooManyEntries { .. } => None,
        }
    }
}

/// Inspects a borrowed vocabulary for conflicts between legacy and Unicode 18 matching.
///
/// The collection limit is checked first. Each entry is then passed to
/// [`validate_vocabulary_entry`] in input order, preserving the existing
/// nonempty and 128 Unicode-scalar rules and their typed errors. Only after all
/// entries are valid are all spoken-phrase pairs compared. Results are ordered
/// by the earlier index and then the later index. Written phrases are validated
/// but do not affect conflict matching.
///
/// `ExistingLegacyConflict` means the two original spoken phrases compare equal
/// after Rust `str::to_lowercase()`. `NewUnicode18Conflict` means those
/// lowercase results differ but the keys from
/// [`VocabularyMatchingVersion::Unicode18CanonicalCaselessV1`] compare equal.
/// The latter policy is Unicode 18.0.0 D145 as implemented by
/// [`comparison_key`]. Neither policy rewrites the original strings. The
/// temporary keys are not returned or persisted, and this function cannot
/// access settings files or resolve conflicts.
///
/// # Errors
///
/// Returns [`VocabularyMigrationError::TooManyEntries`] above the existing
/// 500-entry cap, or [`VocabularyMigrationError::InvalidEntry`] for the first
/// invalid entry in input order. Existing and newly introduced conflicts are
/// returned as index-only diagnostics rather than errors, so callers can
/// request manual resolution without selecting a winner.
pub fn inspect_vocabulary_migration(
    entries: &[VocabularyEntryRef<'_>],
) -> Result<Vec<VocabularyMigrationConflict>, VocabularyMigrationError> {
    if entries.len() > MAX_VOCABULARY_ENTRIES {
        return Err(VocabularyMigrationError::TooManyEntries {
            count: entries.len(),
            maximum: MAX_VOCABULARY_ENTRIES,
        });
    }

    for (index, entry) in entries.iter().enumerate() {
        validate_vocabulary_entry(entry.spoken, entry.written)
            .map_err(|source| VocabularyMigrationError::InvalidEntry { index, source })?;
    }

    // These keys live only for this call. Keeping each policy result once makes
    // the stable pairwise report efficient at the 500-entry collection limit.
    let legacy_keys = entries
        .iter()
        .map(|entry| entry.spoken.to_lowercase())
        .collect::<Vec<_>>();
    let unicode18_keys = entries
        .iter()
        .map(|entry| {
            comparison_key(
                entry.spoken,
                VocabularyMatchingVersion::Unicode18CanonicalCaselessV1,
            )
        })
        .collect::<Vec<_>>();

    let mut conflicts = Vec::new();
    for original_index in 0..entries.len() {
        for conflicting_index in (original_index + 1)..entries.len() {
            let legacy_equal = legacy_keys[original_index] == legacy_keys[conflicting_index];
            let unicode18_equal =
                unicode18_keys[original_index] == unicode18_keys[conflicting_index];

            let classification = if legacy_equal {
                VocabularyMigrationConflictKind::ExistingLegacyConflict
            } else if unicode18_equal {
                VocabularyMigrationConflictKind::NewUnicode18Conflict
            } else {
                continue;
            };

            conflicts.push(VocabularyMigrationConflict {
                original_index,
                conflicting_index,
                classification,
                source_policy: VocabularyMatchingPolicyId::LegacyRustLowercaseV1,
                target_policy: VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1,
            });
        }
    }

    Ok(conflicts)
}

/// Assesses whether a borrowed vocabulary is ready for a possible future
/// matching-policy migration.
///
/// This function reuses [`inspect_vocabulary_migration`] for collection-size,
/// entry validation, and the complete deterministic conflict report. An empty
/// collection returns [`VocabularyMigrationReadiness::EmptyCollection`]
/// regardless of source-policy provenance. For nonempty collections,
/// [`VocabularySourcePolicy::Unknown`] always remains review-required, even
/// when no conflicts are detected. Verified legacy data with no conflicts is
/// [`VocabularyMigrationReadiness::CompatibleForExplicitMigration`]; verified
/// current data with no conflicts is
/// [`VocabularyMigrationReadiness::AlreadyCurrent`]. Any reported conflict
/// under either verified source policy requires manual resolution.
///
/// The caller is responsible for selecting a verified source policy only when
/// trusted evidence supports it. In particular, settings schema v1 alone is
/// not evidence for either verified policy. This pure assessment neither reads
/// nor writes settings, selects a conflict winner, mutates entries, nor
/// authorizes a later settings-file rewrite. Original strings are borrowed and
/// unchanged; reports contain indices and typed classifications only.
///
/// # Errors
///
/// Returns the same typed size or first-invalid-entry error as
/// [`inspect_vocabulary_migration`], preserving its validation order.
pub fn assess_vocabulary_migration_readiness(
    entries: &[VocabularyEntryRef<'_>],
    source_policy: VocabularySourcePolicy,
) -> Result<VocabularyMigrationReadiness, VocabularyMigrationError> {
    let conflicts = inspect_vocabulary_migration(entries)?;

    if entries.is_empty() {
        return Ok(VocabularyMigrationReadiness::EmptyCollection);
    }

    match source_policy {
        VocabularySourcePolicy::Unknown => {
            Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts })
        }
        VocabularySourcePolicy::VerifiedLegacyLowercase if conflicts.is_empty() => {
            Ok(VocabularyMigrationReadiness::CompatibleForExplicitMigration)
        }
        VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1 if conflicts.is_empty() => {
            Ok(VocabularyMigrationReadiness::AlreadyCurrent)
        }
        VocabularySourcePolicy::VerifiedLegacyLowercase
        | VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1 => {
            Ok(VocabularyMigrationReadiness::ManualResolutionRequired { conflicts })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        VocabularyMatchingPolicyId, VocabularyMigrationConflict, VocabularyMigrationConflictKind,
        VocabularyMigrationError, VocabularyMigrationReadiness, VocabularySourcePolicy,
        assess_vocabulary_migration_readiness, inspect_vocabulary_migration,
    };
    use crate::vocabulary::{
        MAX_VOCABULARY_ENTRIES, VocabularyEntryRef, VocabularyField, VocabularyValidationError,
    };

    fn entry<'a>(spoken: &'a str, written: &'a str) -> VocabularyEntryRef<'a> {
        VocabularyEntryRef { spoken, written }
    }

    fn expected_conflict(
        original_index: usize,
        conflicting_index: usize,
        classification: VocabularyMigrationConflictKind,
    ) -> VocabularyMigrationConflict {
        VocabularyMigrationConflict {
            original_index,
            conflicting_index,
            classification,
            source_policy: VocabularyMatchingPolicyId::LegacyRustLowercaseV1,
            target_policy: VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1,
        }
    }

    fn owned_entries(count: usize) -> Vec<(String, String)> {
        (0..count)
            .map(|index| (format!("entry-{index}"), format!("replacement-{index}")))
            .collect()
    }

    fn borrowed_entries(entries: &[(String, String)]) -> Vec<VocabularyEntryRef<'_>> {
        entries
            .iter()
            .map(|(spoken, written)| entry(spoken, written))
            .collect()
    }

    #[test]
    fn accepts_an_empty_collection() {
        assert_eq!(inspect_vocabulary_migration(&[]), Ok(Vec::new()));
    }

    #[test]
    fn accepts_valid_distinct_entries_without_conflicts() {
        let entries = [
            entry("north parkade", "North Parkade"),
            entry("Rust", "Rust"),
        ];

        assert_eq!(inspect_vocabulary_migration(&entries), Ok(Vec::new()));
    }

    #[test]
    fn accepts_exactly_500_valid_entries() {
        let owned = owned_entries(MAX_VOCABULARY_ENTRIES);
        let entries = borrowed_entries(&owned);

        assert_eq!(entries.len(), MAX_VOCABULARY_ENTRIES);
        assert_eq!(inspect_vocabulary_migration(&entries), Ok(Vec::new()));
    }

    #[test]
    fn rejects_501_entries_before_per_entry_validation() {
        let mut owned = owned_entries(MAX_VOCABULARY_ENTRIES + 1);
        owned[0].0.clear();
        let entries = borrowed_entries(&owned);

        assert_eq!(
            inspect_vocabulary_migration(&entries),
            Err(VocabularyMigrationError::TooManyEntries {
                count: MAX_VOCABULARY_ENTRIES + 1,
                maximum: MAX_VOCABULARY_ENTRIES,
            })
        );
    }

    #[test]
    fn reports_the_first_invalid_spoken_field_with_typed_reason() {
        let entries = [entry("valid", "written"), entry("  \t", "replacement")];

        assert_eq!(
            inspect_vocabulary_migration(&entries),
            Err(VocabularyMigrationError::InvalidEntry {
                index: 1,
                source: VocabularyValidationError::Empty(VocabularyField::Spoken),
            })
        );
    }

    #[test]
    fn reports_the_first_invalid_written_field_with_typed_reason() {
        let entries = [entry("spoken", "\n  ")];

        assert_eq!(
            inspect_vocabulary_migration(&entries),
            Err(VocabularyMigrationError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            })
        );
    }

    #[test]
    fn preserves_the_128_scalar_limit_for_both_fields() {
        let overlong = "x".repeat(129);
        let spoken_error = inspect_vocabulary_migration(&[entry(&overlong, "replacement")]);
        assert_eq!(
            spoken_error,
            Err(VocabularyMigrationError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::TooLong(VocabularyField::Spoken),
            })
        );

        let written_error = inspect_vocabulary_migration(&[entry("spoken", &overlong)]);
        assert_eq!(
            written_error,
            Err(VocabularyMigrationError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::TooLong(VocabularyField::Written),
            })
        );
    }

    #[test]
    fn classifies_ascii_case_insensitive_conflicts_as_existing_legacy_conflicts() {
        let entries = [entry("ALPHA", "first"), entry("alpha", "second")];

        assert_eq!(
            inspect_vocabulary_migration(&entries),
            Ok(vec![expected_conflict(
                0,
                1,
                VocabularyMigrationConflictKind::ExistingLegacyConflict,
            )])
        );
    }

    #[test]
    fn classifies_non_ascii_lowercase_conflicts_as_existing_legacy_conflicts() {
        let entries = [entry("ÉCOLE", "first"), entry("école", "second")];

        assert_eq!(
            inspect_vocabulary_migration(&entries),
            Ok(vec![expected_conflict(
                0,
                1,
                VocabularyMigrationConflictKind::ExistingLegacyConflict,
            )])
        );
    }

    #[test]
    fn reports_german_sharp_s_as_a_new_unicode18_conflict() {
        assert_eq!(
            inspect_vocabulary_migration(&[entry("straße", "first"), entry("STRASSE", "second")]),
            Ok(vec![expected_conflict(
                0,
                1,
                VocabularyMigrationConflictKind::NewUnicode18Conflict,
            )])
        );
    }

    #[test]
    fn reports_composed_and_decomposed_accents_as_a_new_unicode18_conflict() {
        assert_eq!(
            inspect_vocabulary_migration(&[entry("é", "first"), entry("e\u{301}", "second")]),
            Ok(vec![expected_conflict(
                0,
                1,
                VocabularyMigrationConflictKind::NewUnicode18Conflict,
            )])
        );
    }

    #[test]
    fn reports_greek_sigma_variants_as_a_new_unicode18_conflict() {
        assert_eq!(
            inspect_vocabulary_migration(&[entry("σ", "first"), entry("ς", "second")]),
            Ok(vec![expected_conflict(
                0,
                1,
                VocabularyMigrationConflictKind::NewUnicode18Conflict,
            )])
        );
    }

    #[test]
    fn keeps_default_turkish_dotted_and_dotless_i_pairs_distinct() {
        for (first, second) in [("I", "ı"), ("İ", "i")] {
            assert_eq!(
                inspect_vocabulary_migration(&[entry(first, "first"), entry(second, "second")]),
                Ok(Vec::new()),
                "these values must remain distinct under both policies"
            );
        }
    }

    #[test]
    fn keeps_cross_script_confusables_distinct() {
        assert_eq!(
            inspect_vocabulary_migration(&[entry("a", "first"), entry("а", "second")]),
            Ok(Vec::new())
        );
    }

    #[test]
    fn keeps_values_with_trailing_whitespace_distinct() {
        assert_eq!(
            inspect_vocabulary_migration(&[
                entry("north parkade", "first"),
                entry("north parkade ", "second"),
            ]),
            Ok(Vec::new())
        );
    }

    #[test]
    fn reports_multiple_conflicts_in_stable_pairwise_input_order() {
        let entries = [
            entry("straße", "a"),
            entry("STRASSE", "b"),
            entry("Straße", "c"),
            entry("é", "d"),
            entry("e\u{301}", "e"),
        ];

        assert_eq!(
            inspect_vocabulary_migration(&entries),
            Ok(vec![
                expected_conflict(0, 1, VocabularyMigrationConflictKind::NewUnicode18Conflict,),
                expected_conflict(
                    0,
                    2,
                    VocabularyMigrationConflictKind::ExistingLegacyConflict,
                ),
                expected_conflict(1, 2, VocabularyMigrationConflictKind::NewUnicode18Conflict,),
                expected_conflict(3, 4, VocabularyMigrationConflictKind::NewUnicode18Conflict,),
            ])
        );
    }

    #[test]
    fn spoken_conflicts_are_reported_even_when_written_replacements_differ() {
        let entries = [
            entry("café", "Cafe"),
            entry("CAFÉ", "different replacement"),
        ];

        assert_eq!(
            inspect_vocabulary_migration(&entries),
            Ok(vec![expected_conflict(
                0,
                1,
                VocabularyMigrationConflictKind::ExistingLegacyConflict,
            )])
        );
    }

    #[test]
    fn reports_the_correct_original_and_conflicting_indices() {
        let entries = [
            entry("alpha", "one"),
            entry("beta", "two"),
            entry("ALPHA", "three"),
        ];
        let conflicts = inspect_vocabulary_migration(&entries).expect("entries are valid");

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].original_index, 0);
        assert_eq!(conflicts[0].conflicting_index, 2);
    }

    #[test]
    fn reports_classification_and_policy_identifiers() {
        let conflicts = inspect_vocabulary_migration(&[
            entry("already", "one"),
            entry("ALREADY", "two"),
            entry("straße", "three"),
            entry("STRASSE", "four"),
        ])
        .expect("entries are valid");

        assert_eq!(
            conflicts[0].classification,
            VocabularyMigrationConflictKind::ExistingLegacyConflict
        );
        assert_eq!(
            conflicts[0].source_policy,
            VocabularyMatchingPolicyId::LegacyRustLowercaseV1
        );
        assert_eq!(
            conflicts[0].target_policy,
            VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1
        );
        assert_eq!(
            conflicts[1].classification,
            VocabularyMigrationConflictKind::NewUnicode18Conflict
        );
        assert_eq!(
            conflicts[0].source_policy.identifier(),
            "rust-str-to-lowercase-v1"
        );
        assert_eq!(
            conflicts[0].target_policy.identifier(),
            "unicode-18.0.0-canonical-caseless-v1"
        );
    }

    #[test]
    fn preserves_all_original_text_after_inspection() {
        let owned = vec![
            (String::from("straße"), String::from("First Replacement")),
            (String::from("STRASSE"), String::from("Second Replacement")),
        ];
        let before = owned.clone();
        let entries = borrowed_entries(&owned);

        let conflicts = inspect_vocabulary_migration(&entries).expect("entries are valid");

        assert_eq!(conflicts.len(), 1);
        assert_eq!(owned, before);
    }

    #[test]
    fn repeated_calls_return_identical_conflicts() {
        let entries = [
            entry("straße", "first"),
            entry("STRASSE", "second"),
            entry("Σ", "third"),
            entry("ς", "fourth"),
        ];

        let first = inspect_vocabulary_migration(&entries);
        for _ in 0..5 {
            assert_eq!(inspect_vocabulary_migration(&entries), first);
        }
    }

    #[test]
    fn reports_both_entries_without_automatically_resolving_conflicts() {
        let owned = vec![
            (
                String::from("straße"),
                String::from("keep this replacement"),
            ),
            (
                String::from("STRASSE"),
                String::from("keep that replacement"),
            ),
        ];
        let before = owned.clone();
        let entries = borrowed_entries(&owned);
        let conflicts = inspect_vocabulary_migration(&entries).expect("entries are valid");

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].original_index, 0);
        assert_eq!(conflicts[0].conflicting_index, 1);
        assert_eq!(owned, before);
    }

    #[test]
    fn has_no_path_or_persistence_surface_and_does_not_mutate_inputs() {
        let owned = vec![
            (String::from("é"), String::from("composed")),
            (String::from("e\u{301}"), String::from("decomposed")),
        ];
        let before = owned.clone();
        let entries = borrowed_entries(&owned);
        let _conflicts = inspect_vocabulary_migration(&entries).expect("entries are valid");

        assert_eq!(owned, before);
        assert_eq!(entries.len(), before.len());
    }

    #[test]
    fn formatted_errors_and_conflicts_never_include_vocabulary_text() {
        let private_spoken = "private-spoken-value";
        let private_written = "private-written-value";
        let invalid = [entry("", private_written)];
        let error = inspect_vocabulary_migration(&invalid).expect_err("empty spoken is invalid");
        assert!(!error.to_string().contains(private_spoken));
        assert!(!error.to_string().contains(private_written));

        let conflicts = inspect_vocabulary_migration(&[
            entry("straße", "private-first-replacement"),
            entry("STRASSE", "private-second-replacement"),
        ])
        .expect("entries are valid");
        let report = format!("{conflicts:?}");
        assert!(!report.contains("straße"));
        assert!(!report.contains("STRASSE"));
        assert!(!report.contains("private-first-replacement"));
        assert!(!report.contains("private-second-replacement"));
    }

    #[test]
    fn empty_collection_needs_no_vocabulary_migration_for_any_provenance() {
        for source_policy in [
            VocabularySourcePolicy::Unknown,
            VocabularySourcePolicy::VerifiedLegacyLowercase,
            VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1,
        ] {
            assert_eq!(
                assess_vocabulary_migration_readiness(&[], source_policy),
                Ok(VocabularyMigrationReadiness::EmptyCollection)
            );
        }
    }

    #[test]
    fn schema_v1_does_not_supply_matching_policy_provenance() {
        // Schema v1 has no matching-policy marker. The caller must pass
        // Unknown unless a separate trusted source establishes the policy.
        let entries = [entry("north parkade", "North Parkade")];

        assert_eq!(
            assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown),
            Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
                conflicts: Vec::new(),
            })
        );
    }

    #[test]
    fn unknown_policy_with_no_conflicts_still_requires_review() {
        let entries = [entry("distinct source", "replacement")];

        let readiness =
            assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown)
                .expect("entry is valid");

        assert_eq!(
            readiness,
            VocabularyMigrationReadiness::SourcePolicyUnknown {
                conflicts: Vec::new(),
            }
        );
        assert!(!matches!(
            readiness,
            VocabularyMigrationReadiness::CompatibleForExplicitMigration
                | VocabularyMigrationReadiness::AlreadyCurrent
        ));
    }

    #[test]
    fn unknown_policy_retains_unicode18_conflicts_without_approving_migration() {
        let entries = [
            entry("straße", "one"),
            entry("STRASSE", "two"),
            entry("é", "three"),
            entry("e\u{301}", "four"),
            entry("σ", "five"),
            entry("ς", "six"),
        ];

        assert_eq!(
            assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown),
            Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
                conflicts: vec![
                    expected_conflict(0, 1, VocabularyMigrationConflictKind::NewUnicode18Conflict,),
                    expected_conflict(2, 3, VocabularyMigrationConflictKind::NewUnicode18Conflict,),
                    expected_conflict(4, 5, VocabularyMigrationConflictKind::NewUnicode18Conflict,),
                ],
            })
        );
    }

    #[test]
    fn verified_legacy_collection_without_conflicts_is_compatible_only() {
        let entries = [entry("north parkade", "North Parkade")];

        assert_eq!(
            assess_vocabulary_migration_readiness(
                &entries,
                VocabularySourcePolicy::VerifiedLegacyLowercase,
            ),
            Ok(VocabularyMigrationReadiness::CompatibleForExplicitMigration)
        );
    }

    #[test]
    fn verified_legacy_collection_with_lowercase_conflict_requires_manual_resolution() {
        let entries = [entry("Alpha", "first"), entry("ALPHA", "second")];

        assert_eq!(
            assess_vocabulary_migration_readiness(
                &entries,
                VocabularySourcePolicy::VerifiedLegacyLowercase,
            ),
            Ok(VocabularyMigrationReadiness::ManualResolutionRequired {
                conflicts: vec![expected_conflict(
                    0,
                    1,
                    VocabularyMigrationConflictKind::ExistingLegacyConflict,
                )],
            })
        );
    }

    #[test]
    fn verified_legacy_collection_with_new_unicode18_conflict_requires_manual_resolution() {
        let entries = [entry("straße", "first"), entry("STRASSE", "second")];

        assert_eq!(
            assess_vocabulary_migration_readiness(
                &entries,
                VocabularySourcePolicy::VerifiedLegacyLowercase,
            ),
            Ok(VocabularyMigrationReadiness::ManualResolutionRequired {
                conflicts: vec![expected_conflict(
                    0,
                    1,
                    VocabularyMigrationConflictKind::NewUnicode18Conflict,
                )],
            })
        );
    }

    #[test]
    fn verified_current_collection_without_conflicts_is_already_current() {
        let entries = [entry("north parkade", "North Parkade")];

        assert_eq!(
            assess_vocabulary_migration_readiness(
                &entries,
                VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1,
            ),
            Ok(VocabularyMigrationReadiness::AlreadyCurrent)
        );
    }

    #[test]
    fn verified_current_collection_with_a_duplicate_requires_manual_resolution() {
        let entries = [entry("Alpha", "first"), entry("ALPHA", "second")];

        assert_eq!(
            assess_vocabulary_migration_readiness(
                &entries,
                VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1,
            ),
            Ok(VocabularyMigrationReadiness::ManualResolutionRequired {
                conflicts: vec![expected_conflict(
                    0,
                    1,
                    VocabularyMigrationConflictKind::ExistingLegacyConflict,
                )],
            })
        );
    }

    #[test]
    fn valid_turkish_i_pairs_and_cross_script_confusables_remain_compatible() {
        for (first, second) in [
            ("I", "ı"),
            ("İ", "i"),
            ("a", "а"), // Latin a and Cyrillic a are visually confusable, not equal.
        ] {
            let entries = [entry(first, "first"), entry(second, "second")];
            assert_eq!(
                assess_vocabulary_migration_readiness(
                    &entries,
                    VocabularySourcePolicy::VerifiedLegacyLowercase,
                ),
                Ok(VocabularyMigrationReadiness::CompatibleForExplicitMigration)
            );
        }
    }

    #[test]
    fn assessment_forwards_invalid_entry_and_capacity_errors() {
        assert_eq!(
            assess_vocabulary_migration_readiness(
                &[entry(" \t", "replacement")],
                VocabularySourcePolicy::VerifiedLegacyLowercase,
            ),
            Err(VocabularyMigrationError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Spoken),
            })
        );

        let overlong_written = "x".repeat(129);
        assert_eq!(
            assess_vocabulary_migration_readiness(
                &[entry("spoken", &overlong_written)],
                VocabularySourcePolicy::VerifiedLegacyLowercase,
            ),
            Err(VocabularyMigrationError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::TooLong(VocabularyField::Written),
            })
        );

        let owned = owned_entries(MAX_VOCABULARY_ENTRIES + 1);
        let entries = borrowed_entries(&owned);
        assert_eq!(
            assess_vocabulary_migration_readiness(
                &entries,
                VocabularySourcePolicy::VerifiedLegacyLowercase,
            ),
            Err(VocabularyMigrationError::TooManyEntries {
                count: MAX_VOCABULARY_ENTRIES + 1,
                maximum: MAX_VOCABULARY_ENTRIES,
            })
        );
    }

    #[test]
    fn assessment_accepts_exactly_500_valid_entries() {
        let owned = owned_entries(MAX_VOCABULARY_ENTRIES);
        let entries = borrowed_entries(&owned);

        assert_eq!(
            assess_vocabulary_migration_readiness(
                &entries,
                VocabularySourcePolicy::VerifiedLegacyLowercase,
            ),
            Ok(VocabularyMigrationReadiness::CompatibleForExplicitMigration)
        );
    }

    #[test]
    fn readiness_conflicts_keep_all_indices_classifications_and_input_order() {
        let entries = [
            entry("straße", "a"),
            entry("STRASSE", "b"),
            entry("Straße", "c"),
            entry("é", "d"),
            entry("e\u{301}", "e"),
        ];

        assert_eq!(
            assess_vocabulary_migration_readiness(
                &entries,
                VocabularySourcePolicy::VerifiedLegacyLowercase,
            ),
            Ok(VocabularyMigrationReadiness::ManualResolutionRequired {
                conflicts: vec![
                    expected_conflict(0, 1, VocabularyMigrationConflictKind::NewUnicode18Conflict,),
                    expected_conflict(
                        0,
                        2,
                        VocabularyMigrationConflictKind::ExistingLegacyConflict,
                    ),
                    expected_conflict(1, 2, VocabularyMigrationConflictKind::NewUnicode18Conflict,),
                    expected_conflict(3, 4, VocabularyMigrationConflictKind::NewUnicode18Conflict,),
                ],
            })
        );
    }

    #[test]
    fn unknown_provenance_never_returns_migration_eligibility() {
        for entries in [
            vec![entry("distinct", "replacement")],
            vec![entry("straße", "first"), entry("STRASSE", "second")],
        ] {
            let result =
                assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown)
                    .expect("entries are valid");

            assert!(matches!(
                result,
                VocabularyMigrationReadiness::SourcePolicyUnknown { .. }
            ));
            assert!(!matches!(
                result,
                VocabularyMigrationReadiness::CompatibleForExplicitMigration
                    | VocabularyMigrationReadiness::AlreadyCurrent
            ));
        }
    }

    #[test]
    fn assessment_is_deterministic_read_only_and_has_no_persistence_input() {
        let owned = vec![
            (String::from("straße"), String::from("First Replacement")),
            (String::from("STRASSE"), String::from("Second Replacement")),
        ];
        let before = owned.clone();
        let entries = borrowed_entries(&owned);
        let first = assess_vocabulary_migration_readiness(
            &entries,
            VocabularySourcePolicy::VerifiedLegacyLowercase,
        );

        for _ in 0..5 {
            assert_eq!(
                assess_vocabulary_migration_readiness(
                    &entries,
                    VocabularySourcePolicy::VerifiedLegacyLowercase,
                ),
                first
            );
        }

        assert_eq!(owned, before);
        assert_eq!(entries.len(), before.len());
        assert!(matches!(
            first,
            Ok(VocabularyMigrationReadiness::ManualResolutionRequired { .. })
        ));
        // The public function accepts only borrowed entries and policy evidence;
        // it has no settings path or persistence operation to invoke.
    }
}
