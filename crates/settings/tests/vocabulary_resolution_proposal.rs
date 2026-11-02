use std::error::Error;
use std::fmt::{self, Display, Formatter};

use outflow_core::vocabulary::{
    MAX_VOCABULARY_ENTRIES, VocabularyCollectionError, VocabularyEntryRef, VocabularyField,
    VocabularyValidationError, validate_vocabulary_collection,
};
use outflow_core::vocabulary_migration::{VocabularyMigrationError, inspect_vocabulary_migration};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProposedDisposition {
    Retain,
    OmitFromProposedCollection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ResolutionDecision {
    original_index: usize,
    disposition: ProposedDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolutionProposalAssessment {
    original_entry_count: usize,
    retained_original_indices: Vec<usize>,
    omitted_original_indices: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ProposedCollectionValidationError {
    TooManyEntries {
        count: usize,
        maximum: usize,
    },
    InvalidEntry {
        original_index: usize,
        source: VocabularyValidationError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ResolutionProposalError {
    MissingDecision {
        original_index: usize,
    },
    DuplicateDecisionIndex {
        original_index: usize,
    },
    OutOfRangeDecisionIndex {
        original_index: usize,
        original_entry_count: usize,
    },
    InvalidOriginalCollection(VocabularyMigrationError),
    InvalidProposedCollection(ProposedCollectionValidationError),
    RemainingUnicode18Conflict {
        first_original_index: usize,
        conflicting_original_index: usize,
    },
    CandidateIndexMappingInvariant {
        candidate_index: usize,
    },
}

impl Display for ProposedCollectionValidationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyEntries { count, maximum } => write!(
                formatter,
                "proposed vocabulary has {count} entries; maximum is {maximum}"
            ),
            Self::InvalidEntry {
                original_index,
                source,
            } => write!(
                formatter,
                "proposed vocabulary entry from original index {original_index} is invalid: {source}"
            ),
        }
    }
}

impl Error for ProposedCollectionValidationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidEntry { source, .. } => Some(source),
            Self::TooManyEntries { .. } => None,
        }
    }
}

impl Display for ResolutionProposalError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingDecision { original_index } => write!(
                formatter,
                "resolution proposal is missing a decision for original index {original_index}"
            ),
            Self::DuplicateDecisionIndex { original_index } => write!(
                formatter,
                "resolution proposal has multiple decisions for original index {original_index}"
            ),
            Self::OutOfRangeDecisionIndex {
                original_index,
                original_entry_count,
            } => write!(
                formatter,
                "resolution decision index {original_index} is outside {original_entry_count} original entries"
            ),
            Self::InvalidOriginalCollection(source) => {
                write!(
                    formatter,
                    "original vocabulary collection is invalid: {source}"
                )
            }
            Self::InvalidProposedCollection(source) => {
                write!(
                    formatter,
                    "proposed vocabulary collection is invalid: {source}"
                )
            }
            Self::RemainingUnicode18Conflict {
                first_original_index,
                conflicting_original_index,
            } => write!(
                formatter,
                "proposed vocabulary entries from original indices {first_original_index} and {conflicting_original_index} conflict under Unicode 18 matching"
            ),
            Self::CandidateIndexMappingInvariant { candidate_index } => write!(
                formatter,
                "proposed vocabulary validation returned unmapped candidate index {candidate_index}"
            ),
        }
    }
}

impl Error for ResolutionProposalError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidOriginalCollection(source) => Some(source),
            Self::InvalidProposedCollection(source) => Some(source),
            Self::MissingDecision { .. }
            | Self::DuplicateDecisionIndex { .. }
            | Self::OutOfRangeDecisionIndex { .. }
            | Self::RemainingUnicode18Conflict { .. }
            | Self::CandidateIndexMappingInvariant { .. } => None,
        }
    }
}

/// Validates an explicit, in-memory proposal while leaving the original entries untouched.
///
/// The original validator is intentionally used only for size and field
/// validation: its conflict report is not an admission decision because the
/// original collection may contain entries that were accepted under an older
/// comparison policy. Decisions are checked for range, duplication, and
/// completeness before retained entries are projected in original order.
/// The candidate is then checked by the existing Unicode 18 collection
/// validator. Its temporary candidate positions are mapped back to original
/// indices in every reported error.
fn validate_resolution_proposal(
    original: &[VocabularyEntryRef<'_>],
    decisions: &[ResolutionDecision],
) -> Result<ResolutionProposalAssessment, ResolutionProposalError> {
    // This existing core inspection validates the original collection's cap
    // and fields. Its conflict list is intentionally not treated as a failure.
    let _original_conflicts_are_not_rejection = inspect_vocabulary_migration(original)
        .map_err(ResolutionProposalError::InvalidOriginalCollection)?;

    let mut disposition_by_original_index = vec![None; original.len()];
    for decision in decisions {
        if decision.original_index >= original.len() {
            return Err(ResolutionProposalError::OutOfRangeDecisionIndex {
                original_index: decision.original_index,
                original_entry_count: original.len(),
            });
        }

        let slot = &mut disposition_by_original_index[decision.original_index];
        if slot.is_some() {
            return Err(ResolutionProposalError::DuplicateDecisionIndex {
                original_index: decision.original_index,
            });
        }
        *slot = Some(decision.disposition);
    }

    if let Some(original_index) = disposition_by_original_index
        .iter()
        .position(Option::is_none)
    {
        return Err(ResolutionProposalError::MissingDecision { original_index });
    }

    let mut proposed_entries = Vec::new();
    let mut candidate_to_original = Vec::new();
    let mut retained_original_indices = Vec::new();
    let mut omitted_original_indices = Vec::new();

    // Decisions are indexed by original position, so their input order cannot
    // reorder the hypothetical collection or its stable validation errors.
    for (original_index, (entry, disposition)) in original
        .iter()
        .zip(disposition_by_original_index.iter())
        .enumerate()
    {
        match disposition {
            Some(ProposedDisposition::Retain) => {
                proposed_entries.push(*entry);
                candidate_to_original.push(original_index);
                retained_original_indices.push(original_index);
            }
            Some(ProposedDisposition::OmitFromProposedCollection) => {
                omitted_original_indices.push(original_index);
            }
            None => return Err(ResolutionProposalError::MissingDecision { original_index }),
        }
    }

    match validate_vocabulary_collection(&proposed_entries) {
        Ok(()) => Ok(ResolutionProposalAssessment {
            original_entry_count: original.len(),
            retained_original_indices,
            omitted_original_indices,
        }),
        Err(VocabularyCollectionError::TooManyEntries { count, maximum }) => {
            Err(ResolutionProposalError::InvalidProposedCollection(
                ProposedCollectionValidationError::TooManyEntries { count, maximum },
            ))
        }
        Err(VocabularyCollectionError::InvalidEntry { index, source }) => {
            let original_index = candidate_to_original.get(index).copied().ok_or(
                ResolutionProposalError::CandidateIndexMappingInvariant {
                    candidate_index: index,
                },
            )?;
            Err(ResolutionProposalError::InvalidProposedCollection(
                ProposedCollectionValidationError::InvalidEntry {
                    original_index,
                    source,
                },
            ))
        }
        Err(VocabularyCollectionError::DuplicateSpoken {
            first_index,
            duplicate_index,
        }) => {
            let first_original_index = candidate_to_original.get(first_index).copied().ok_or(
                ResolutionProposalError::CandidateIndexMappingInvariant {
                    candidate_index: first_index,
                },
            )?;
            let conflicting_original_index = candidate_to_original
                .get(duplicate_index)
                .copied()
                .ok_or(ResolutionProposalError::CandidateIndexMappingInvariant {
                    candidate_index: duplicate_index,
                })?;
            Err(ResolutionProposalError::RemainingUnicode18Conflict {
                first_original_index,
                conflicting_original_index,
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviewSessionState {
    Active,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CancellationOutcome {
    Cancelled,
    AlreadyCancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReviewSessionError {
    CannotChangeAfterCancellation,
    CannotValidateAfterCancellation,
    Proposal(ResolutionProposalError),
}

/// A test-only review draft that borrows an immutable original collection.
///
/// Decisions are kept separately and may be revised by original index. This
/// deliberately models only pre-commit behavior: it has no settings or
/// filesystem interface, and cancellation discards the entire draft.
struct ResolutionReviewSession<'a> {
    original: &'a [VocabularyEntryRef<'a>],
    decisions: Vec<ResolutionDecision>,
    state: ReviewSessionState,
}

impl<'a> ResolutionReviewSession<'a> {
    fn new(original: &'a [VocabularyEntryRef<'a>]) -> Self {
        Self {
            original,
            decisions: Vec::new(),
            state: ReviewSessionState::Active,
        }
    }

    fn state(&self) -> ReviewSessionState {
        self.state
    }

    fn draft_decisions(&self) -> &[ResolutionDecision] {
        &self.decisions
    }

    fn change_decision(
        &mut self,
        original_index: usize,
        disposition: ProposedDisposition,
    ) -> Result<(), ReviewSessionError> {
        if self.state == ReviewSessionState::Cancelled {
            return Err(ReviewSessionError::CannotChangeAfterCancellation);
        }
        if original_index >= self.original.len() {
            return Err(ReviewSessionError::Proposal(
                ResolutionProposalError::OutOfRangeDecisionIndex {
                    original_index,
                    original_entry_count: self.original.len(),
                },
            ));
        }

        if let Some(decision) = self
            .decisions
            .iter_mut()
            .find(|decision| decision.original_index == original_index)
        {
            decision.disposition = disposition;
        } else {
            self.decisions.push(ResolutionDecision {
                original_index,
                disposition,
            });
        }
        Ok(())
    }

    fn validate(&self) -> Result<ResolutionProposalAssessment, ReviewSessionError> {
        if self.state == ReviewSessionState::Cancelled {
            return Err(ReviewSessionError::CannotValidateAfterCancellation);
        }
        validate_resolution_proposal(self.original, &self.decisions)
            .map_err(ReviewSessionError::Proposal)
    }

    fn cancel(&mut self) -> CancellationOutcome {
        match self.state {
            ReviewSessionState::Active => {
                self.decisions.clear();
                self.state = ReviewSessionState::Cancelled;
                CancellationOutcome::Cancelled
            }
            ReviewSessionState::Cancelled => CancellationOutcome::AlreadyCancelled,
        }
    }
}

fn synthetic_entries<'a>(pairs: &[(&'a str, &'a str)]) -> Vec<VocabularyEntryRef<'a>> {
    pairs
        .iter()
        .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
        .collect()
}

fn decisions(dispositions: &[ProposedDisposition]) -> Vec<ResolutionDecision> {
    dispositions
        .iter()
        .copied()
        .enumerate()
        .map(|(original_index, disposition)| ResolutionDecision {
            original_index,
            disposition,
        })
        .collect()
}

fn owned_unique_entries(count: usize) -> Vec<(String, String)> {
    (0..count)
        .map(|index| {
            (
                format!("synthetic-spoken-{index}"),
                format!("replacement-{index}"),
            )
        })
        .collect()
}

fn borrow_owned_entries(entries: &[(String, String)]) -> Vec<VocabularyEntryRef<'_>> {
    entries
        .iter()
        .map(|(spoken, written)| VocabularyEntryRef { spoken, written })
        .collect()
}

fn unicode_review_values() -> Vec<(String, String)> {
    vec![
        ("north parkade".to_owned(), "North Parkade".to_owned()),
        ("straße".to_owned(), "Street".to_owned()),
        ("STRASSE".to_owned(), "Avenue".to_owned()),
        ("é".to_owned(), "Composed Accent".to_owned()),
        ("e\u{301}".to_owned(), "Decomposed Accent".to_owned()),
        ("σ".to_owned(), "Sigma".to_owned()),
        ("ς".to_owned(), "Final Sigma".to_owned()),
    ]
}

fn assert_original_matches_snapshot(
    original: &[VocabularyEntryRef<'_>],
    owned: &[(String, String)],
    snapshot: &[(String, String)],
) {
    assert!(
        owned
            .iter()
            .zip(snapshot)
            .all(|(actual, expected)| actual == expected)
    );
    assert_eq!(owned.len(), snapshot.len());
    assert_eq!(original.len(), snapshot.len());
    assert!(original.iter().enumerate().all(|(index, entry)| {
        snapshot
            .get(index)
            .is_some_and(|(spoken, written)| entry.spoken == spoken && entry.written == written)
    }));
}

#[test]
fn empty_original_collection_needs_no_decisions() {
    let assessment = validate_resolution_proposal(&[], &[]).expect("empty proposal is valid");
    assert_eq!(
        assessment,
        ResolutionProposalAssessment {
            original_entry_count: 0,
            retained_original_indices: vec![],
            omitted_original_indices: vec![],
        }
    );
}

#[test]
fn one_valid_entry_can_be_retained() {
    let original = synthetic_entries(&[("north parkade", "North Parkade")]);
    let assessment = validate_resolution_proposal(
        &original,
        &[ResolutionDecision {
            original_index: 0,
            disposition: ProposedDisposition::Retain,
        }],
    )
    .expect("one valid retained entry passes");
    assert_eq!(assessment.retained_original_indices, vec![0]);
    assert!(assessment.omitted_original_indices.is_empty());
}

#[test]
fn all_entries_can_be_retained_when_there_are_no_conflicts() {
    let original = synthetic_entries(&[("north", "North"), ("south", "South"), ("east", "East")]);
    let assessment = validate_resolution_proposal(
        &original,
        &decisions(&[
            ProposedDisposition::Retain,
            ProposedDisposition::Retain,
            ProposedDisposition::Retain,
        ]),
    )
    .expect("all distinct entries pass");
    assert_eq!(assessment.retained_original_indices, vec![0, 1, 2]);
    assert!(assessment.omitted_original_indices.is_empty());
}

#[test]
fn one_explicit_omission_resolves_a_two_entry_conflict() {
    let original = synthetic_entries(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let assessment = validate_resolution_proposal(
        &original,
        &decisions(&[
            ProposedDisposition::Retain,
            ProposedDisposition::OmitFromProposedCollection,
        ]),
    )
    .expect("one retained side resolves the collision");
    assert_eq!(assessment.retained_original_indices, vec![0]);
    assert_eq!(assessment.omitted_original_indices, vec![1]);
}

#[test]
fn three_connected_entries_can_be_reduced_to_one_retained_entry() {
    let original = synthetic_entries(&[
        ("straße", "Street"),
        ("STRASSE", "Avenue"),
        ("Strasse", "Road"),
    ]);
    let assessment = validate_resolution_proposal(
        &original,
        &decisions(&[
            ProposedDisposition::Retain,
            ProposedDisposition::OmitFromProposedCollection,
            ProposedDisposition::OmitFromProposedCollection,
        ]),
    )
    .expect("one of three equivalent entries is unique");
    assert_eq!(assessment.retained_original_indices, vec![0]);
    assert_eq!(assessment.omitted_original_indices, vec![1, 2]);
}

#[test]
fn three_connected_entries_with_two_retained_report_original_conflict_indices() {
    let original = synthetic_entries(&[
        ("straße", "Street"),
        ("STRASSE", "Avenue"),
        ("Strasse", "Road"),
    ]);
    assert_eq!(
        validate_resolution_proposal(
            &original,
            &decisions(&[
                ProposedDisposition::Retain,
                ProposedDisposition::OmitFromProposedCollection,
                ProposedDisposition::Retain,
            ]),
        ),
        Err(ResolutionProposalError::RemainingUnicode18Conflict {
            first_original_index: 0,
            conflicting_original_index: 2,
        })
    );
}

#[test]
fn two_independent_conflict_groups_can_be_resolved_independently() {
    let original = synthetic_entries(&[
        ("straße", "Street"),
        ("STRASSE", "Avenue"),
        ("é", "Accent A"),
        ("e\u{301}", "Accent B"),
    ]);
    let assessment = validate_resolution_proposal(
        &original,
        &decisions(&[
            ProposedDisposition::Retain,
            ProposedDisposition::OmitFromProposedCollection,
            ProposedDisposition::OmitFromProposedCollection,
            ProposedDisposition::Retain,
        ]),
    )
    .expect("one entry is explicitly retained from each independent pair");
    assert_eq!(assessment.retained_original_indices, vec![0, 3]);
    assert_eq!(assessment.omitted_original_indices, vec![1, 2]);
}

#[test]
fn unresolved_second_group_remains_a_conflict_after_first_group_is_resolved() {
    let original = synthetic_entries(&[
        ("straße", "Street"),
        ("STRASSE", "Avenue"),
        ("é", "Accent A"),
        ("e\u{301}", "Accent B"),
    ]);
    assert_eq!(
        validate_resolution_proposal(
            &original,
            &decisions(&[
                ProposedDisposition::Retain,
                ProposedDisposition::OmitFromProposedCollection,
                ProposedDisposition::Retain,
                ProposedDisposition::Retain,
            ]),
        ),
        Err(ResolutionProposalError::RemainingUnicode18Conflict {
            first_original_index: 2,
            conflicting_original_index: 3,
        })
    );
}

#[test]
fn german_sharp_s_conflict_uses_unicode18_collection_validation() {
    let original = synthetic_entries(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    assert!(matches!(
        validate_resolution_proposal(&original, &decisions(&[ProposedDisposition::Retain; 2])),
        Err(ResolutionProposalError::RemainingUnicode18Conflict {
            first_original_index: 0,
            conflicting_original_index: 1,
        })
    ));
}

#[test]
fn composed_and_decomposed_accent_conflict_uses_unicode18_validation() {
    let original = synthetic_entries(&[("é", "Accent A"), ("e\u{301}", "Accent B")]);
    assert!(matches!(
        validate_resolution_proposal(&original, &decisions(&[ProposedDisposition::Retain; 2])),
        Err(ResolutionProposalError::RemainingUnicode18Conflict {
            first_original_index: 0,
            conflicting_original_index: 1,
        })
    ));
}

#[test]
fn greek_sigma_conflict_uses_unicode18_validation() {
    let original = synthetic_entries(&[("σ", "Sigma"), ("ς", "Final sigma")]);
    assert!(matches!(
        validate_resolution_proposal(&original, &decisions(&[ProposedDisposition::Retain; 2])),
        Err(ResolutionProposalError::RemainingUnicode18Conflict {
            first_original_index: 0,
            conflicting_original_index: 1,
        })
    ));
}

#[test]
fn duplicate_decisions_are_rejected() {
    let original = synthetic_entries(&[("north", "North")]);
    assert_eq!(
        validate_resolution_proposal(
            &original,
            &[
                ResolutionDecision {
                    original_index: 0,
                    disposition: ProposedDisposition::Retain,
                },
                ResolutionDecision {
                    original_index: 0,
                    disposition: ProposedDisposition::OmitFromProposedCollection,
                },
            ],
        ),
        Err(ResolutionProposalError::DuplicateDecisionIndex { original_index: 0 })
    );
}

#[test]
fn missing_decisions_are_rejected() {
    let original = synthetic_entries(&[("north", "North"), ("south", "South")]);
    assert_eq!(
        validate_resolution_proposal(
            &original,
            &[ResolutionDecision {
                original_index: 0,
                disposition: ProposedDisposition::Retain,
            }],
        ),
        Err(ResolutionProposalError::MissingDecision { original_index: 1 })
    );
}

#[test]
fn out_of_range_decision_indices_are_rejected() {
    let original = synthetic_entries(&[("north", "North")]);
    assert_eq!(
        validate_resolution_proposal(
            &original,
            &[
                ResolutionDecision {
                    original_index: 0,
                    disposition: ProposedDisposition::Retain,
                },
                ResolutionDecision {
                    original_index: 1,
                    disposition: ProposedDisposition::OmitFromProposedCollection,
                },
            ],
        ),
        Err(ResolutionProposalError::OutOfRangeDecisionIndex {
            original_index: 1,
            original_entry_count: 1,
        })
    );
}

#[test]
fn decision_input_order_does_not_reorder_retained_entries() {
    let original = synthetic_entries(&[("north", "North"), ("east", "East"), ("west", "West")]);
    let assessment = validate_resolution_proposal(
        &original,
        &[
            ResolutionDecision {
                original_index: 2,
                disposition: ProposedDisposition::Retain,
            },
            ResolutionDecision {
                original_index: 0,
                disposition: ProposedDisposition::Retain,
            },
            ResolutionDecision {
                original_index: 1,
                disposition: ProposedDisposition::Retain,
            },
        ],
    )
    .expect("decision order has no effect on valid collection order");
    assert_eq!(assessment.retained_original_indices, vec![0, 1, 2]);
}

#[test]
fn conflict_indices_map_back_to_original_positions_after_omissions() {
    let original = synthetic_entries(&[
        ("north", "North"),
        ("straße", "Street"),
        ("south", "South"),
        ("STRASSE", "Avenue"),
    ]);
    assert_eq!(
        validate_resolution_proposal(
            &original,
            &decisions(&[
                ProposedDisposition::OmitFromProposedCollection,
                ProposedDisposition::Retain,
                ProposedDisposition::OmitFromProposedCollection,
                ProposedDisposition::Retain,
            ]),
        ),
        Err(ResolutionProposalError::RemainingUnicode18Conflict {
            first_original_index: 1,
            conflicting_original_index: 3,
        })
    );
}

#[test]
fn validating_a_proposal_leaves_original_vocabulary_unchanged() {
    let owned = vec![
        ("straße".to_owned(), "Street".to_owned()),
        ("STRASSE".to_owned(), "Avenue".to_owned()),
        ("North Parkade".to_owned(), "North Parkade".to_owned()),
    ];
    let before = owned.clone();
    let original = borrow_owned_entries(&owned);
    let assessment = validate_resolution_proposal(
        &original,
        &decisions(&[
            ProposedDisposition::Retain,
            ProposedDisposition::OmitFromProposedCollection,
            ProposedDisposition::Retain,
        ]),
    )
    .expect("explicit omission leaves a valid candidate");

    assert_eq!(assessment.retained_original_indices, vec![0, 2]);
    assert_eq!(owned, before);
    assert_eq!(original[0].spoken, "straße");
    assert_eq!(original[1].spoken, "STRASSE");
    assert_eq!(original[2].spoken, "North Parkade");
}

#[test]
fn exactly_five_hundred_valid_retained_entries_are_accepted() {
    let owned = owned_unique_entries(MAX_VOCABULARY_ENTRIES);
    let original = borrow_owned_entries(&owned);
    let assessment = validate_resolution_proposal(
        &original,
        &decisions(&vec![ProposedDisposition::Retain; MAX_VOCABULARY_ENTRIES]),
    )
    .expect("the existing maximum is inclusive");
    assert_eq!(
        assessment.retained_original_indices.len(),
        MAX_VOCABULARY_ENTRIES
    );
    assert_eq!(assessment.original_entry_count, MAX_VOCABULARY_ENTRIES);
}

#[test]
fn over_capacity_original_collection_is_rejected_with_core_error() {
    let owned = owned_unique_entries(MAX_VOCABULARY_ENTRIES + 1);
    let original = borrow_owned_entries(&owned);
    assert_eq!(
        validate_resolution_proposal(&original, &[]),
        Err(ResolutionProposalError::InvalidOriginalCollection(
            VocabularyMigrationError::TooManyEntries {
                count: MAX_VOCABULARY_ENTRIES + 1,
                maximum: MAX_VOCABULARY_ENTRIES,
            }
        ))
    );
}

#[test]
fn invalid_original_spoken_or_written_fields_preserve_core_error_types() {
    let invalid_spoken = synthetic_entries(&[(" ", "valid replacement")]);
    assert_eq!(
        validate_resolution_proposal(
            &invalid_spoken,
            &[ResolutionDecision {
                original_index: 0,
                disposition: ProposedDisposition::Retain,
            }],
        ),
        Err(ResolutionProposalError::InvalidOriginalCollection(
            VocabularyMigrationError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Spoken),
            }
        ))
    );

    let invalid_written = synthetic_entries(&[("valid spoken", "")]);
    assert_eq!(
        validate_resolution_proposal(
            &invalid_written,
            &[ResolutionDecision {
                original_index: 0,
                disposition: ProposedDisposition::Retain,
            }],
        ),
        Err(ResolutionProposalError::InvalidOriginalCollection(
            VocabularyMigrationError::InvalidEntry {
                index: 0,
                source: VocabularyValidationError::Empty(VocabularyField::Written),
            }
        ))
    );
}

#[test]
fn explicitly_omitting_every_entry_validates_an_empty_hypothetical_collection() {
    let original = synthetic_entries(&[("north", "North"), ("south", "South")]);
    let assessment = validate_resolution_proposal(
        &original,
        &decisions(&[
            ProposedDisposition::OmitFromProposedCollection,
            ProposedDisposition::OmitFromProposedCollection,
        ]),
    )
    .expect("explicit all-omitted proposal is a valid empty candidate");
    assert_eq!(assessment.retained_original_indices, Vec::<usize>::new());
    assert_eq!(assessment.omitted_original_indices, vec![0, 1]);
    assert_eq!(assessment.original_entry_count, 2);
}

#[test]
fn assessment_contains_no_provenance_or_migration_authorization() {
    let original = synthetic_entries(&[("north", "North")]);
    let assessment = validate_resolution_proposal(
        &original,
        &[ResolutionDecision {
            original_index: 0,
            disposition: ProposedDisposition::Retain,
        }],
    )
    .expect("proposal is valid");

    assert_eq!(assessment.original_entry_count, 1);
    assert_eq!(assessment.retained_original_indices, vec![0]);
    assert!(assessment.omitted_original_indices.is_empty());
}

#[test]
fn errors_are_typed_and_do_not_include_vocabulary_values() {
    let original = synthetic_entries(&[("private spoken sentinel", "private written sentinel")]);
    let error = validate_resolution_proposal(
        &original,
        &[ResolutionDecision {
            original_index: 1,
            disposition: ProposedDisposition::Retain,
        }],
    )
    .expect_err("out-of-range index is rejected");
    let rendered = format!("{error} {error:?}");
    assert!(matches!(
        error,
        ResolutionProposalError::OutOfRangeDecisionIndex { .. }
    ));
    assert!(!rendered.contains("private spoken sentinel"));
    assert!(!rendered.contains("private written sentinel"));
}

#[test]
fn new_review_is_active_and_does_not_change_original_unicode_entries() {
    let owned = unicode_review_values();
    let snapshot = owned.clone();
    let original = borrow_owned_entries(&owned);
    let review = ResolutionReviewSession::new(&original);

    assert_eq!(review.state(), ReviewSessionState::Active);
    assert!(review.draft_decisions().is_empty());
    assert_original_matches_snapshot(&original, &owned, &snapshot);
}

#[test]
fn draft_changes_revise_only_decisions_without_selecting_a_winner() {
    let owned = unicode_review_values();
    let snapshot = owned.clone();
    let original = borrow_owned_entries(&owned);
    let mut review = ResolutionReviewSession::new(&original);

    review
        .change_decision(2, ProposedDisposition::OmitFromProposedCollection)
        .expect("valid original index can be drafted");
    assert_eq!(
        review.draft_decisions(),
        &[ResolutionDecision {
            original_index: 2,
            disposition: ProposedDisposition::OmitFromProposedCollection,
        }]
    );

    review
        .change_decision(2, ProposedDisposition::Retain)
        .expect("an earlier decision can be revised");
    assert_eq!(
        review.draft_decisions(),
        &[ResolutionDecision {
            original_index: 2,
            disposition: ProposedDisposition::Retain,
        }]
    );
    assert_original_matches_snapshot(&original, &owned, &snapshot);
}

#[test]
fn incomplete_review_cancels_without_validation_and_fresh_review_starts_clean() {
    let owned = vec![
        ("first".to_owned(), "First".to_owned()),
        ("second".to_owned(), "Second".to_owned()),
        ("third".to_owned(), "Third".to_owned()),
    ];
    let snapshot = owned.clone();
    let original = borrow_owned_entries(&owned);
    let mut abandoned = ResolutionReviewSession::new(&original);
    abandoned
        .change_decision(0, ProposedDisposition::Retain)
        .expect("first explicit decision is accepted");
    assert_eq!(abandoned.draft_decisions().len(), 1);

    assert_eq!(abandoned.cancel(), CancellationOutcome::Cancelled);
    assert_eq!(abandoned.state(), ReviewSessionState::Cancelled);
    assert!(abandoned.draft_decisions().is_empty());
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    drop(abandoned);
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    let fresh = ResolutionReviewSession::new(&original);
    assert_eq!(fresh.state(), ReviewSessionState::Active);
    assert!(fresh.draft_decisions().is_empty());
    assert_original_matches_snapshot(&original, &owned, &snapshot);
}

#[test]
fn valid_review_can_be_cancelled_after_validation() {
    let owned = vec![("north parkade".to_owned(), "North Parkade".to_owned())];
    let snapshot = owned.clone();
    let original = borrow_owned_entries(&owned);
    let mut review = ResolutionReviewSession::new(&original);
    review
        .change_decision(0, ProposedDisposition::Retain)
        .expect("valid original index can be drafted");
    let draft_before_validation = review.draft_decisions().to_vec();

    let assessment = review
        .validate()
        .expect("real proposal validation succeeds");
    assert_eq!(assessment.retained_original_indices, vec![0]);
    assert_eq!(review.draft_decisions(), draft_before_validation);
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    assert_eq!(review.cancel(), CancellationOutcome::Cancelled);
    assert!(review.draft_decisions().is_empty());
    assert_original_matches_snapshot(&original, &owned, &snapshot);
}

#[test]
fn invalid_review_can_be_cancelled_after_validation_failure() {
    let owned = vec![
        ("straße".to_owned(), "Street".to_owned()),
        ("STRASSE".to_owned(), "Avenue".to_owned()),
    ];
    let snapshot = owned.clone();
    let original = borrow_owned_entries(&owned);
    let mut review = ResolutionReviewSession::new(&original);
    for original_index in 0..original.len() {
        review
            .change_decision(original_index, ProposedDisposition::Retain)
            .expect("valid original index can be drafted");
    }
    let draft_before_validation = review.draft_decisions().to_vec();

    assert!(matches!(
        review.validate(),
        Err(ReviewSessionError::Proposal(
            ResolutionProposalError::RemainingUnicode18Conflict { .. }
        ))
    ));
    assert_eq!(review.state(), ReviewSessionState::Active);
    assert_eq!(review.draft_decisions(), draft_before_validation);
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    assert_eq!(review.cancel(), CancellationOutcome::Cancelled);
    assert!(review.draft_decisions().is_empty());
    assert_original_matches_snapshot(&original, &owned, &snapshot);
}

#[test]
fn failed_unicode18_validation_allows_a_corrected_retry_and_cancellation() {
    let owned = unicode_review_values();
    let snapshot = owned.clone();
    let original = borrow_owned_entries(&owned);
    let mut review = ResolutionReviewSession::new(&original);
    for original_index in 0..original.len() {
        review
            .change_decision(original_index, ProposedDisposition::Retain)
            .expect("valid original index can be drafted");
    }
    let draft_before_failure = review.draft_decisions().to_vec();

    assert_eq!(
        review.validate(),
        Err(ReviewSessionError::Proposal(
            ResolutionProposalError::RemainingUnicode18Conflict {
                first_original_index: 1,
                conflicting_original_index: 2,
            }
        ))
    );
    assert_eq!(review.state(), ReviewSessionState::Active);
    assert_eq!(review.draft_decisions(), draft_before_failure);
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    for original_index in [2, 4, 6] {
        review
            .change_decision(
                original_index,
                ProposedDisposition::OmitFromProposedCollection,
            )
            .expect("corrected explicit decision is accepted");
    }
    let corrected_draft = review.draft_decisions().to_vec();
    let assessment = review
        .validate()
        .expect("corrected proposal is revalidated through the existing validator");
    assert_eq!(assessment.original_entry_count, 7);
    assert_eq!(assessment.retained_original_indices, vec![0, 1, 3, 5]);
    assert_eq!(assessment.omitted_original_indices, vec![2, 4, 6]);
    assert_eq!(review.draft_decisions(), corrected_draft);
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    assert_eq!(review.cancel(), CancellationOutcome::Cancelled);
    assert!(review.draft_decisions().is_empty());
    assert_original_matches_snapshot(&original, &owned, &snapshot);
}

#[test]
fn cancelled_review_is_terminal_and_repeated_cancellation_is_idempotent() {
    let owned = unicode_review_values();
    let snapshot = owned.clone();
    let original = borrow_owned_entries(&owned);
    let mut review = ResolutionReviewSession::new(&original);
    review
        .change_decision(0, ProposedDisposition::Retain)
        .expect("valid original index can be drafted");

    assert_eq!(review.cancel(), CancellationOutcome::Cancelled);
    assert_eq!(review.cancel(), CancellationOutcome::AlreadyCancelled);
    assert_eq!(review.state(), ReviewSessionState::Cancelled);
    assert!(review.draft_decisions().is_empty());
    assert_eq!(
        review.validate(),
        Err(ReviewSessionError::CannotValidateAfterCancellation)
    );
    assert_eq!(
        review.change_decision(0, ProposedDisposition::OmitFromProposedCollection),
        Err(ReviewSessionError::CannotChangeAfterCancellation)
    );
    assert_original_matches_snapshot(&original, &owned, &snapshot);
}

#[test]
fn session_reports_out_of_range_change_with_existing_typed_proposal_error() {
    let owned = vec![("north".to_owned(), "North".to_owned())];
    let original = borrow_owned_entries(&owned);
    let mut review = ResolutionReviewSession::new(&original);

    assert_eq!(
        review.change_decision(1, ProposedDisposition::Retain),
        Err(ReviewSessionError::Proposal(
            ResolutionProposalError::OutOfRangeDecisionIndex {
                original_index: 1,
                original_entry_count: 1,
            }
        ))
    );
    assert!(review.draft_decisions().is_empty());
}

#[test]
fn session_validation_keeps_incomplete_decisions_as_a_typed_proposal_error() {
    let owned = vec![
        ("north".to_owned(), "North".to_owned()),
        ("south".to_owned(), "South".to_owned()),
    ];
    let original = borrow_owned_entries(&owned);
    let mut review = ResolutionReviewSession::new(&original);
    review
        .change_decision(0, ProposedDisposition::Retain)
        .expect("valid original index can be drafted");

    assert_eq!(
        review.validate(),
        Err(ReviewSessionError::Proposal(
            ResolutionProposalError::MissingDecision { original_index: 1 }
        ))
    );
    assert_eq!(review.state(), ReviewSessionState::Active);
    assert_eq!(review.draft_decisions().len(), 1);
}

#[test]
fn review_cancellation_and_retry_demonstration() {
    let owned = unicode_review_values();
    let snapshot = owned.clone();
    let original = borrow_owned_entries(&owned);

    let mut incomplete = ResolutionReviewSession::new(&original);
    incomplete
        .change_decision(0, ProposedDisposition::Retain)
        .expect("first explicit decision is accepted");
    let incomplete_cancelled = incomplete.cancel() == CancellationOutcome::Cancelled;
    assert!(incomplete_cancelled);
    assert!(incomplete.draft_decisions().is_empty());
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    let mut review = ResolutionReviewSession::new(&original);
    for original_index in 0..original.len() {
        review
            .change_decision(original_index, ProposedDisposition::Retain)
            .expect("valid original index can be drafted");
    }
    let invalid_proposal_rejected = matches!(
        review.validate(),
        Err(ReviewSessionError::Proposal(
            ResolutionProposalError::RemainingUnicode18Conflict { .. }
        ))
    );
    assert!(invalid_proposal_rejected);
    assert_eq!(review.state(), ReviewSessionState::Active);
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    for original_index in [2, 4, 6] {
        review
            .change_decision(
                original_index,
                ProposedDisposition::OmitFromProposedCollection,
            )
            .expect("corrected explicit decision is accepted");
    }
    let corrected_proposal_valid = review
        .validate()
        .is_ok_and(|assessment| assessment.retained_original_indices == [0, 1, 3, 5]);
    assert!(corrected_proposal_valid);
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    let cancellation_after_validation = review.cancel() == CancellationOutcome::Cancelled;
    assert!(cancellation_after_validation);
    let repeated_cancellation_is_idempotent =
        review.cancel() == CancellationOutcome::AlreadyCancelled;
    assert!(repeated_cancellation_is_idempotent);
    let draft_discarded = review.draft_decisions().is_empty();
    assert!(draft_discarded);
    assert_eq!(review.state(), ReviewSessionState::Cancelled);
    assert_original_matches_snapshot(&original, &owned, &snapshot);

    println!("Outflow vocabulary review cancellation checks");
    println!();
    println!(
        "Incomplete draft cancellation: {}",
        if incomplete_cancelled { "PASS" } else { "FAIL" }
    );
    println!(
        "Invalid proposal rejected: {}",
        if invalid_proposal_rejected {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "Corrected proposal validation: {}",
        if corrected_proposal_valid {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "Cancellation after validation: {}",
        if cancellation_after_validation {
            "PASS"
        } else {
            "FAIL"
        }
    );
    println!(
        "Repeated cancellation: {}",
        if repeated_cancellation_is_idempotent {
            "IDEMPOTENT"
        } else {
            "FAILED"
        }
    );
    println!("Original vocabulary: UNCHANGED");
    println!(
        "Draft decisions after cancellation: {}",
        if draft_discarded {
            "DISCARDED"
        } else {
            "PRESENT"
        }
    );
    println!();
    println!("Settings files modified: NO");
    println!("Migration applied: NO");
    println!("Persistent rollback tested: NO");
    println!("Production schema-v2 support: NOT IMPLEMENTED");
}

#[test]
fn resolution_proposal_validation_summary() {
    let two_conflict = synthetic_entries(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let two_resolved = validate_resolution_proposal(
        &two_conflict,
        &decisions(&[
            ProposedDisposition::Retain,
            ProposedDisposition::OmitFromProposedCollection,
        ]),
    );
    assert!(two_resolved.is_ok());

    let three_conflict = synthetic_entries(&[
        ("straße", "Street"),
        ("STRASSE", "Avenue"),
        ("Strasse", "Road"),
    ]);
    let three_resolved = validate_resolution_proposal(
        &three_conflict,
        &decisions(&[
            ProposedDisposition::Retain,
            ProposedDisposition::OmitFromProposedCollection,
            ProposedDisposition::OmitFromProposedCollection,
        ]),
    );
    assert!(three_resolved.is_ok());

    let two_retained =
        validate_resolution_proposal(&two_conflict, &decisions(&[ProposedDisposition::Retain; 2]));
    assert!(matches!(
        two_retained,
        Err(ResolutionProposalError::RemainingUnicode18Conflict { .. })
    ));

    let incomplete = validate_resolution_proposal(
        &two_conflict,
        &[ResolutionDecision {
            original_index: 0,
            disposition: ProposedDisposition::Retain,
        }],
    );
    assert!(matches!(
        incomplete,
        Err(ResolutionProposalError::MissingDecision { .. })
    ));

    let duplicate = validate_resolution_proposal(
        &two_conflict,
        &[
            ResolutionDecision {
                original_index: 0,
                disposition: ProposedDisposition::Retain,
            },
            ResolutionDecision {
                original_index: 0,
                disposition: ProposedDisposition::OmitFromProposedCollection,
            },
            ResolutionDecision {
                original_index: 1,
                disposition: ProposedDisposition::Retain,
            },
        ],
    );
    assert!(matches!(
        duplicate,
        Err(ResolutionProposalError::DuplicateDecisionIndex { .. })
    ));

    let filtered_conflict = synthetic_entries(&[
        ("north", "North"),
        ("straße", "Street"),
        ("south", "South"),
        ("STRASSE", "Avenue"),
    ]);
    let remapped_conflict = validate_resolution_proposal(
        &filtered_conflict,
        &decisions(&[
            ProposedDisposition::OmitFromProposedCollection,
            ProposedDisposition::Retain,
            ProposedDisposition::OmitFromProposedCollection,
            ProposedDisposition::Retain,
        ]),
    );
    assert_eq!(
        remapped_conflict,
        Err(ResolutionProposalError::RemainingUnicode18Conflict {
            first_original_index: 1,
            conflicting_original_index: 3,
        })
    );

    let original_snapshot = two_conflict.to_vec();
    assert_eq!(two_conflict, original_snapshot);
    assert!(matches!(
        two_resolved,
        Ok(ResolutionProposalAssessment {
            retained_original_indices,
            ..
        }) if retained_original_indices == vec![0]
    ));
    assert!(matches!(
        three_resolved,
        Ok(ResolutionProposalAssessment {
            retained_original_indices,
            ..
        }) if retained_original_indices == vec![0]
    ));

    println!("Outflow vocabulary resolution proposal validation");
    println!("\nTwo-entry conflict, one retained: VALID PROPOSAL");
    println!("Three-entry conflict, one retained: VALID PROPOSAL");
    println!("Two conflicting entries retained: REJECTED");
    println!("Incomplete decisions: REJECTED");
    println!("Duplicate decision: REJECTED");
    println!("Original indices: PRESERVED");
    println!("Original vocabulary: UNCHANGED");
    println!("\nResolution applied: NO");
    println!("Entries deleted: NO");
    println!("Settings files modified: NO");
    println!("Migration authorized: NO");
    println!("Production schema-v2 support: NOT IMPLEMENTED");
}
