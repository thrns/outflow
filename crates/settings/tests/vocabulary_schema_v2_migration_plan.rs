use outflow_settings::{
    CURRENT_SCHEMA_VERSION, Settings, SettingsError, SettingsJsonError,
    UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER, VocabularyMatchingPolicyId,
    VocabularyMigrationConflict, VocabularyMigrationConflictKind, VocabularyMigrationError,
    VocabularyPhraseField, VocabularyPolicyMarker, VocabularyPolicyMarkerInspection,
    VocabularySourcePolicy, inspect_vocabulary_policy_marker, preflight_vocabulary_compatibility,
};
use serde::Deserialize;
use serde_json::{Value, json};

const PROPOSED_SCHEMA_VERSION: u32 = 2;

/// Test-only partial contract; this is not a production settings DTO.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposedSettingsV2Fixture {
    schema_version: u32,
    vocabulary_matching_policy: VocabularyPolicyMarker,
    #[serde(rename = "vocabulary")]
    _vocabulary: Vec<ProposedVocabularyEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposedVocabularyEntry {
    #[serde(rename = "spoken")]
    _spoken: String,
    #[serde(rename = "written")]
    _written: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProposedTargetContractError {
    InvalidJson,
    InvalidEnvelope,
    MissingSchemaVersion,
    MalformedSchemaVersion,
    UnsupportedSchemaVersion,
    WrongProposedSchemaVersion,
    MissingMarker,
    UnsupportedMarker,
    MalformedMarkerType,
    MissingVocabulary,
    MalformedVocabulary,
    MalformedEnvelopeFields,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SyntheticPlanningError {
    InvalidTargetContract(ProposedTargetContractError),
    UnsupportedSourceSchemaVersion { found: u32 },
    SourceVocabulary(VocabularyMigrationError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntheticSourcePolicyProvenance {
    Unknown,
    VerifiedLegacyLowercaseSyntheticAssumption,
    VerifiedUnicode18CanonicalCaselessV1SyntheticAssumption,
}

impl From<VocabularySourcePolicy> for SyntheticSourcePolicyProvenance {
    fn from(policy: VocabularySourcePolicy) -> Self {
        match policy {
            VocabularySourcePolicy::Unknown => Self::Unknown,
            VocabularySourcePolicy::VerifiedLegacyLowercase => {
                Self::VerifiedLegacyLowercaseSyntheticAssumption
            }
            VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1 => {
                Self::VerifiedUnicode18CanonicalCaselessV1SyntheticAssumption
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SyntheticMigrationPlanStatus {
    NoVocabularyMigrationNeeded,
    SourceProvenanceReviewRequired,
    ManualConflictResolutionRequired,
    CompatibleForFutureMigrationReview,
    AlreadyCurrentForReview,
}

/// Content-free, in-memory readiness report. It contains no vocabulary text.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SyntheticMigrationPlan {
    source_schema_version: u32,
    proposed_target_schema_version: u32,
    target_matching_policy_identifier: &'static str,
    source_policy_provenance: SyntheticSourcePolicyProvenance,
    readiness_status: SyntheticMigrationPlanStatus,
    conflict_count: usize,
    conflicts: Vec<VocabularyMigrationConflict>,
    migration_applied: bool,
}

/// Parses the proposed contract in this integration test's scope only.
///
/// Schema selection and marker recognition use the same test contract as the
/// Task 50 integration fixture. This parser deliberately returns only typed,
/// content-free errors; malformed input text is never retained in an error.
fn parse_proposed_target(
    input: &str,
) -> Result<ProposedSettingsV2Fixture, ProposedTargetContractError> {
    let value: Value =
        serde_json::from_str(input).map_err(|_| ProposedTargetContractError::InvalidJson)?;
    let object = value
        .as_object()
        .ok_or(ProposedTargetContractError::InvalidEnvelope)?;

    let schema_version = object
        .get("schema_version")
        .ok_or(ProposedTargetContractError::MissingSchemaVersion)?
        .as_u64()
        .ok_or(ProposedTargetContractError::MalformedSchemaVersion)?;
    if schema_version > u64::from(PROPOSED_SCHEMA_VERSION) {
        return Err(ProposedTargetContractError::UnsupportedSchemaVersion);
    }
    if schema_version != u64::from(PROPOSED_SCHEMA_VERSION) {
        return Err(ProposedTargetContractError::WrongProposedSchemaVersion);
    }

    match object.get("vocabulary_matching_policy") {
        None => return Err(ProposedTargetContractError::MissingMarker),
        Some(Value::String(raw_marker)) => {
            match inspect_vocabulary_policy_marker(Some(raw_marker)) {
                VocabularyPolicyMarkerInspection::Recognized(_) => {}
                VocabularyPolicyMarkerInspection::Unsupported => {
                    return Err(ProposedTargetContractError::UnsupportedMarker);
                }
                VocabularyPolicyMarkerInspection::Absent => {
                    return Err(ProposedTargetContractError::MissingMarker);
                }
            }
        }
        Some(_) => return Err(ProposedTargetContractError::MalformedMarkerType),
    }

    let vocabulary = object
        .get("vocabulary")
        .ok_or(ProposedTargetContractError::MissingVocabulary)?
        .as_array()
        .ok_or(ProposedTargetContractError::MalformedVocabulary)?;
    for entry in vocabulary {
        let entry = entry
            .as_object()
            .ok_or(ProposedTargetContractError::MalformedVocabulary)?;
        if !entry.get("spoken").is_some_and(Value::is_string)
            || !entry.get("written").is_some_and(Value::is_string)
        {
            return Err(ProposedTargetContractError::MalformedVocabulary);
        }
    }

    serde_json::from_value(value).map_err(|_| ProposedTargetContractError::MalformedEnvelopeFields)
}

/// Plans against a synthetic schema-v1 DTO and proposed schema-v2 target
/// contract. Conflicting DTOs are constructed in memory because strict schema
/// decoding now rejects them; source-policy provenance remains an explicit
/// test assumption and is never inferred from the target marker or content.
fn plan_synthetic_vocabulary_migration(
    source: &Settings,
    proposed_target_document: &str,
    source_policy: VocabularySourcePolicy,
) -> Result<SyntheticMigrationPlan, SyntheticPlanningError> {
    if source.schema_version != CURRENT_SCHEMA_VERSION {
        return Err(SyntheticPlanningError::UnsupportedSourceSchemaVersion {
            found: source.schema_version,
        });
    }

    let target = parse_proposed_target(proposed_target_document)
        .map_err(SyntheticPlanningError::InvalidTargetContract)?;
    let readiness = preflight_vocabulary_compatibility(source, source_policy)
        .map_err(SyntheticPlanningError::SourceVocabulary)?;
    let (readiness_status, conflicts) = match readiness {
        outflow_settings::VocabularyMigrationReadiness::EmptyCollection => (
            SyntheticMigrationPlanStatus::NoVocabularyMigrationNeeded,
            Vec::new(),
        ),
        outflow_settings::VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts } => (
            SyntheticMigrationPlanStatus::SourceProvenanceReviewRequired,
            conflicts,
        ),
        outflow_settings::VocabularyMigrationReadiness::ManualResolutionRequired { conflicts } => (
            SyntheticMigrationPlanStatus::ManualConflictResolutionRequired,
            conflicts,
        ),
        outflow_settings::VocabularyMigrationReadiness::CompatibleForExplicitMigration => (
            SyntheticMigrationPlanStatus::CompatibleForFutureMigrationReview,
            Vec::new(),
        ),
        outflow_settings::VocabularyMigrationReadiness::AlreadyCurrent => (
            SyntheticMigrationPlanStatus::AlreadyCurrentForReview,
            Vec::new(),
        ),
    };

    Ok(SyntheticMigrationPlan {
        source_schema_version: source.schema_version,
        proposed_target_schema_version: target.schema_version,
        target_matching_policy_identifier: target.vocabulary_matching_policy.identifier(),
        source_policy_provenance: source_policy.into(),
        readiness_status,
        conflict_count: conflicts.len(),
        conflicts,
        migration_applied: false,
    })
}

fn decoded_source(vocabulary: &[(String, String)]) -> Result<Settings, SettingsJsonError> {
    let mut value: Value = serde_json::from_str(
        &Settings::default()
            .to_json()
            .expect("schema-v1 defaults serialize"),
    )
    .expect("schema-v1 defaults are JSON");
    value["vocabulary"] = Value::Array(
        vocabulary
            .iter()
            .enumerate()
            .map(|(index, (spoken, written))| {
                json!({
                    "id": format!("synthetic-{index}"),
                    "spoken": spoken,
                    "written": written,
                })
            })
            .collect(),
    );
    Settings::from_json(&value.to_string())
}

fn source(vocabulary: &[(&str, &str)]) -> Settings {
    Settings {
        vocabulary: vocabulary
            .iter()
            .enumerate()
            .map(
                |(index, (spoken, written))| outflow_settings::VocabularyEntry {
                    id: format!("synthetic-{index}"),
                    spoken: (*spoken).to_owned(),
                    written: (*written).to_owned(),
                },
            )
            .collect(),
        ..Settings::default()
    }
}

fn target_document(source: &Settings) -> String {
    let vocabulary = source
        .vocabulary
        .iter()
        .map(|entry| json!({"spoken": entry.spoken, "written": entry.written}))
        .collect::<Vec<_>>();
    json!({
        "schema_version": PROPOSED_SCHEMA_VERSION,
        "vocabulary_matching_policy": UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER,
        "vocabulary": vocabulary,
    })
    .to_string()
}

fn expected_new_conflict(
    original_index: usize,
    conflicting_index: usize,
) -> VocabularyMigrationConflict {
    VocabularyMigrationConflict {
        original_index,
        conflicting_index,
        classification: VocabularyMigrationConflictKind::NewUnicode18Conflict,
        source_policy: VocabularyMatchingPolicyId::LegacyRustLowercaseV1,
        target_policy: VocabularyMatchingPolicyId::Unicode18CanonicalCaselessV1,
    }
}

fn status_label(status: SyntheticMigrationPlanStatus) -> &'static str {
    match status {
        SyntheticMigrationPlanStatus::NoVocabularyMigrationNeeded => {
            "NO VOCABULARY MIGRATION NEEDED"
        }
        SyntheticMigrationPlanStatus::SourceProvenanceReviewRequired => "REVIEW REQUIRED",
        SyntheticMigrationPlanStatus::ManualConflictResolutionRequired => "MANUAL RESOLUTION",
        SyntheticMigrationPlanStatus::CompatibleForFutureMigrationReview => "COMPATIBLE FOR REVIEW",
        SyntheticMigrationPlanStatus::AlreadyCurrentForReview => "ALREADY CURRENT",
    }
}

fn malformed_source_json(vocabulary: Value) -> String {
    let mut value: Value = serde_json::from_str(
        &Settings::default()
            .to_json()
            .expect("schema-v1 defaults serialize"),
    )
    .expect("schema-v1 defaults are JSON");
    value["vocabulary"] = vocabulary;
    value.to_string()
}

fn plan_from_target_value(
    source: &Settings,
    target: Value,
    source_policy: VocabularySourcePolicy,
) -> Result<SyntheticMigrationPlan, SyntheticPlanningError> {
    plan_synthetic_vocabulary_migration(source, &target.to_string(), source_policy)
}

#[test]
fn accepts_the_recognized_proposed_v2_target_marker() {
    let settings = source(&[("North Parkade", "North Parkade")]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::Unknown,
    )
    .expect("recognized proposed target is assessable");

    assert_eq!(plan.source_schema_version, 1);
    assert_eq!(plan.proposed_target_schema_version, 2);
    assert_eq!(
        plan.target_matching_policy_identifier,
        "unicode18-canonical-caseless-v1"
    );
    assert_eq!(
        inspect_vocabulary_policy_marker(Some(plan.target_matching_policy_identifier)),
        VocabularyPolicyMarkerInspection::Recognized(
            VocabularyPolicyMarker::Unicode18CanonicalCaselessV1
        )
    );
}

#[test]
fn rejects_a_missing_target_marker() {
    let settings = source(&[]);
    let mut target: Value =
        serde_json::from_str(&target_document(&settings)).expect("synthetic target is JSON");
    target
        .as_object_mut()
        .expect("synthetic target is an object")
        .remove("vocabulary_matching_policy");

    assert_eq!(
        plan_from_target_value(&settings, target, VocabularySourcePolicy::Unknown),
        Err(SyntheticPlanningError::InvalidTargetContract(
            ProposedTargetContractError::MissingMarker
        ))
    );
}

#[test]
fn rejects_an_unsupported_target_marker() {
    let settings = source(&[]);
    let mut target: Value =
        serde_json::from_str(&target_document(&settings)).expect("synthetic target is JSON");
    target["vocabulary_matching_policy"] = json!("future-policy-v9");

    assert_eq!(
        plan_from_target_value(&settings, target, VocabularySourcePolicy::Unknown),
        Err(SyntheticPlanningError::InvalidTargetContract(
            ProposedTargetContractError::UnsupportedMarker
        ))
    );
}

#[test]
fn rejects_a_wrong_case_target_marker() {
    let settings = source(&[]);
    let mut target: Value =
        serde_json::from_str(&target_document(&settings)).expect("synthetic target is JSON");
    target["vocabulary_matching_policy"] = json!("Unicode18-canonical-caseless-v1");

    assert_eq!(
        plan_from_target_value(&settings, target, VocabularySourcePolicy::Unknown),
        Err(SyntheticPlanningError::InvalidTargetContract(
            ProposedTargetContractError::UnsupportedMarker
        ))
    );
}

#[test]
fn rejects_a_malformed_target_marker_json_type() {
    let settings = source(&[]);
    let mut target: Value =
        serde_json::from_str(&target_document(&settings)).expect("synthetic target is JSON");
    target["vocabulary_matching_policy"] = json!(null);

    assert_eq!(
        plan_from_target_value(&settings, target, VocabularySourcePolicy::Unknown),
        Err(SyntheticPlanningError::InvalidTargetContract(
            ProposedTargetContractError::MalformedMarkerType
        ))
    );
}

#[test]
fn rejects_an_unsupported_target_schema_version() {
    let settings = source(&[]);
    let mut target: Value =
        serde_json::from_str(&target_document(&settings)).expect("synthetic target is JSON");
    target["schema_version"] = json!(3);

    assert_eq!(
        plan_from_target_value(&settings, target, VocabularySourcePolicy::Unknown),
        Err(SyntheticPlanningError::InvalidTargetContract(
            ProposedTargetContractError::UnsupportedSchemaVersion
        ))
    );
}

#[test]
fn rejects_malformed_target_vocabulary_declarations() {
    let settings = source(&[]);
    let mut target: Value =
        serde_json::from_str(&target_document(&settings)).expect("synthetic target is JSON");
    target["vocabulary"] = json!([{"spoken": "synthetic-only"}]);

    assert_eq!(
        plan_from_target_value(&settings, target, VocabularySourcePolicy::Unknown),
        Err(SyntheticPlanningError::InvalidTargetContract(
            ProposedTargetContractError::MalformedVocabulary
        ))
    );
}

#[test]
fn empty_vocabulary_needs_no_key_migration_but_does_not_authorize_schema_rewrite() {
    let settings = source(&[]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::Unknown,
    )
    .expect("empty synthetic vocabulary is assessable");

    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::NoVocabularyMigrationNeeded
    );
    assert_eq!(
        plan.source_policy_provenance,
        SyntheticSourcePolicyProvenance::Unknown
    );
    assert_eq!(plan.conflict_count, 0);
    assert!(!plan.migration_applied);
}

#[test]
fn conflict_free_nonempty_vocabulary_still_requires_provenance_review() {
    let settings = source(&[("hello", "Hello"), ("world", "World")]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::Unknown,
    )
    .expect("synthetic source vocabulary is assessable");

    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::SourceProvenanceReviewRequired
    );
    assert_eq!(plan.conflict_count, 0);
}

#[test]
fn unknown_provenance_preserves_detected_unicode18_conflicts() {
    let settings = source(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::Unknown,
    )
    .expect("synthetic Unicode 18 collision is assessable");

    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::SourceProvenanceReviewRequired
    );
    assert_eq!(plan.conflict_count, 1);
    assert_eq!(plan.conflicts, vec![expected_new_conflict(0, 1)]);
}

#[test]
fn verified_legacy_vocabulary_without_conflicts_is_compatible_only_for_review() {
    let settings = source(&[("north parkade", "North Parkade")]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("explicit synthetic legacy assumption is assessable");

    assert_eq!(
        plan.source_policy_provenance,
        SyntheticSourcePolicyProvenance::VerifiedLegacyLowercaseSyntheticAssumption
    );
    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::CompatibleForFutureMigrationReview
    );
    assert!(!plan.migration_applied);
}

#[test]
fn verified_legacy_sharp_s_collision_requires_manual_resolution() {
    let settings = source(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("sharp-s collision is assessable under explicit assumption");

    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::ManualConflictResolutionRequired
    );
    assert_eq!(plan.conflicts, vec![expected_new_conflict(0, 1)]);
}

#[test]
fn verified_legacy_canonical_accent_collision_requires_manual_resolution() {
    let settings = source(&[("é", "Example A"), ("e\u{301}", "Example B")]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("canonical accent collision is allowed by schema-v1 decoding");

    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::ManualConflictResolutionRequired
    );
    assert_eq!(plan.conflicts, vec![expected_new_conflict(0, 1)]);
}

#[test]
fn verified_legacy_greek_sigma_collision_requires_manual_resolution() {
    let settings = source(&[("σ", "Example A"), ("ς", "Example B")]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("sigma collision is allowed by schema-v1 decoding");

    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::ManualConflictResolutionRequired
    );
    assert_eq!(plan.conflicts, vec![expected_new_conflict(0, 1)]);
}

#[test]
fn verified_current_vocabulary_without_conflicts_is_already_current_for_review() {
    let settings = source(&[("hello", "Hello")]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1,
    )
    .expect("explicit synthetic current assumption is assessable");

    assert_eq!(
        plan.source_policy_provenance,
        SyntheticSourcePolicyProvenance::VerifiedUnicode18CanonicalCaselessV1SyntheticAssumption
    );
    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::AlreadyCurrentForReview
    );
}

#[test]
fn verified_current_vocabulary_with_conflicts_requires_manual_resolution() {
    let settings = source(&[("σ", "Example A"), ("ς", "Example B")]);
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1,
    )
    .expect("current-policy conflict is assessable");

    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::ManualConflictResolutionRequired
    );
    assert_eq!(plan.conflicts, vec![expected_new_conflict(0, 1)]);
}

#[test]
fn multiple_conflicts_have_stable_ordered_indices_and_classifications() {
    let settings = source(&[
        ("straße", "Street"),
        ("café", "Cafe"),
        ("STRASSE", "Avenue"),
        ("cafe\u{301}", "Coffee shop"),
        ("σ", "Sigma"),
        ("ς", "Final sigma"),
    ]);
    let target = target_document(&settings);
    let first = plan_synthetic_vocabulary_migration(
        &settings,
        &target,
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("multiple synthetic conflicts are assessable");
    let second = plan_synthetic_vocabulary_migration(
        &settings,
        &target,
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("repeat planning call is assessable");

    let expected = vec![
        expected_new_conflict(0, 2),
        expected_new_conflict(1, 3),
        expected_new_conflict(4, 5),
    ];
    assert_eq!(first, second);
    assert_eq!(
        first.readiness_status,
        SyntheticMigrationPlanStatus::ManualConflictResolutionRequired
    );
    assert_eq!(first.conflict_count, 3);
    assert_eq!(first.conflicts, expected);
    assert!(
        first
            .conflicts
            .iter()
            .all(|conflict| conflict.classification
                == VocabularyMigrationConflictKind::NewUnicode18Conflict)
    );
}

#[test]
fn schema_v1_decoder_rejects_invalid_spoken_written_values_and_lowercase_conflicts() {
    for (spoken, written, expected_field) in [
        (" \t", "replacement", VocabularyPhraseField::Spoken),
        ("spoken", "\n  ", VocabularyPhraseField::Written),
    ] {
        assert_eq!(
            decoded_source(&[(spoken.to_owned(), written.to_owned())]),
            Err(SettingsJsonError::InvalidSettings(
                SettingsError::EmptyVocabularyPhrase {
                    entry_index: 0,
                    field: expected_field,
                }
            ))
        );
    }

    let overlong = "x".repeat(129);
    assert_eq!(
        decoded_source(&[(overlong, "replacement".to_owned())]),
        Err(SettingsJsonError::InvalidSettings(
            SettingsError::VocabularyPhraseTooLong {
                entry_index: 0,
                field: VocabularyPhraseField::Spoken,
                max_chars: 128,
            }
        ))
    );

    let lowercase_conflict = malformed_source_json(json!([
        {"id": "synthetic-0", "spoken": "Hello", "written": "First"},
        {"id": "synthetic-1", "spoken": "hello", "written": "Second"}
    ]));
    assert_eq!(
        Settings::from_json(&lowercase_conflict),
        Err(SettingsJsonError::InvalidSettings(
            SettingsError::DuplicateVocabularySource {
                first_index: 0,
                duplicate_index: 1,
            }
        ))
    );
}

#[test]
fn planner_accepts_exactly_500_valid_entries_decoded_as_schema_v1() {
    let entries = (0..500)
        .map(|index| (format!("spoken-{index}"), format!("written-{index}")))
        .collect::<Vec<_>>();
    let settings = decoded_source(&entries).expect("500 entries decode through schema v1");
    let plan = plan_synthetic_vocabulary_migration(
        &settings,
        &target_document(&settings),
        VocabularySourcePolicy::Unknown,
    )
    .expect("500 valid entries are within the readiness limit");

    assert_eq!(settings.vocabulary.len(), 500);
    assert_eq!(plan.conflict_count, 0);
    assert_eq!(
        plan.readiness_status,
        SyntheticMigrationPlanStatus::SourceProvenanceReviewRequired
    );
}

#[test]
fn schema_v1_decoder_rejects_501_entries_before_planning() {
    let entries = (0..501)
        .map(|index| (format!("spoken-{index}"), format!("written-{index}")))
        .collect::<Vec<_>>();

    assert_eq!(
        decoded_source(&entries),
        Err(SettingsJsonError::InvalidSettings(
            SettingsError::TooManyVocabularyEntries { max_entries: 500 }
        ))
    );
}

#[test]
fn planner_preserves_source_and_target_and_report_contains_no_vocabulary_text() {
    let settings = source(&[("synthetic-private-spoken", "synthetic-private-written")]);
    let original_settings = settings.clone();
    let original_json = settings
        .to_json()
        .expect("source serializes before planning");
    let target = target_document(&settings);
    let original_target = target.clone();
    let plan =
        plan_synthetic_vocabulary_migration(&settings, &target, VocabularySourcePolicy::Unknown)
            .expect("synthetic source and target are assessable");
    let report_debug = format!("{plan:?}");

    assert_eq!(settings, original_settings);
    assert_eq!(
        settings.to_json().expect("source still serializes"),
        original_json
    );
    assert_eq!(target, original_target);
    assert_eq!(settings.vocabulary.len(), 1);
    assert!(!report_debug.contains("synthetic-private-spoken"));
    assert!(!report_debug.contains("synthetic-private-written"));
    assert_eq!(plan.conflict_count, plan.conflicts.len());
    assert!(!plan.migration_applied);
}

#[test]
fn target_marker_never_infers_source_provenance() {
    let settings = source(&[("hello", "Hello")]);
    let target = target_document(&settings);

    for _ in 0..2 {
        let plan = plan_synthetic_vocabulary_migration(
            &settings,
            &target,
            VocabularySourcePolicy::Unknown,
        )
        .expect("recognized target declaration is assessable");
        assert_eq!(
            plan.source_policy_provenance,
            SyntheticSourcePolicyProvenance::Unknown
        );
        assert_eq!(
            plan.readiness_status,
            SyntheticMigrationPlanStatus::SourceProvenanceReviewRequired
        );
    }
}

#[test]
fn production_decoder_still_rejects_schema_v2() {
    let mut value: Value = serde_json::from_str(
        &Settings::default()
            .to_json()
            .expect("schema-v1 settings serialize"),
    )
    .expect("schema-v1 settings are JSON");
    value["schema_version"] = json!(2);

    assert_eq!(
        Settings::from_json(&value.to_string()),
        Err(SettingsJsonError::InvalidSettings(
            SettingsError::UnsupportedSchemaVersion { found: 2 }
        ))
    );
    assert_eq!(CURRENT_SCHEMA_VERSION, 1);
}

#[test]
fn planning_summary_uses_actual_parser_preflight_and_planner_results() {
    let empty = source(&[]);
    let unknown_settings = source(&[("hello", "Hello")]);
    let legacy_settings = source(&[("north parkade", "North Parkade")]);
    let conflict_settings = source(&[("straße", "Street"), ("STRASSE", "Avenue")]);
    let current_settings = source(&[("hello", "Hello")]);
    let source_snapshots = [
        empty.clone(),
        unknown_settings.clone(),
        legacy_settings.clone(),
        conflict_settings.clone(),
        current_settings.clone(),
    ];

    let empty_plan = plan_synthetic_vocabulary_migration(
        &empty,
        &target_document(&empty),
        VocabularySourcePolicy::Unknown,
    )
    .expect("empty plan is assessable");
    let unknown_plan = plan_synthetic_vocabulary_migration(
        &unknown_settings,
        &target_document(&unknown_settings),
        VocabularySourcePolicy::Unknown,
    )
    .expect("unknown provenance plan is assessable");
    let legacy_plan = plan_synthetic_vocabulary_migration(
        &legacy_settings,
        &target_document(&legacy_settings),
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("verified synthetic legacy plan is assessable");
    let conflict_plan = plan_synthetic_vocabulary_migration(
        &conflict_settings,
        &target_document(&conflict_settings),
        VocabularySourcePolicy::VerifiedLegacyLowercase,
    )
    .expect("synthetic legacy conflict plan is assessable");
    let current_plan = plan_synthetic_vocabulary_migration(
        &current_settings,
        &target_document(&current_settings),
        VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1,
    )
    .expect("verified synthetic current plan is assessable");

    let unsupported_target = {
        let mut target: Value = serde_json::from_str(&target_document(&unknown_settings))
            .expect("synthetic target is JSON");
        target["vocabulary_matching_policy"] = json!("future-policy-v9");
        plan_from_target_value(&unknown_settings, target, VocabularySourcePolicy::Unknown)
    };
    let production_v2_rejected = production_decoder_rejects_schema_v2();
    let no_source_mutation = [
        &empty,
        &unknown_settings,
        &legacy_settings,
        &conflict_settings,
        &current_settings,
    ]
    .iter()
    .zip(source_snapshots.iter())
    .all(|(after, before)| *after == before);
    let no_vocabulary_entries_deleted = [
        empty.vocabulary.len() == source_snapshots[0].vocabulary.len(),
        unknown_settings.vocabulary.len() == source_snapshots[1].vocabulary.len(),
        legacy_settings.vocabulary.len() == source_snapshots[2].vocabulary.len(),
        conflict_settings.vocabulary.len() == source_snapshots[3].vocabulary.len(),
        current_settings.vocabulary.len() == source_snapshots[4].vocabulary.len(),
    ]
    .into_iter()
    .all(|preserved| preserved);
    let no_migration = [
        &empty_plan,
        &unknown_plan,
        &legacy_plan,
        &conflict_plan,
        &current_plan,
    ]
    .iter()
    .all(|plan| !plan.migration_applied);

    println!("Outflow proposed vocabulary migration plan");
    println!();
    println!("Source schema: v{}", unknown_plan.source_schema_version);
    println!(
        "Proposed target schema: v{}",
        unknown_plan.proposed_target_schema_version
    );
    println!(
        "Target policy: {}",
        unknown_plan.target_matching_policy_identifier
    );
    println!();
    println!(
        "Unknown provenance: {}",
        status_label(unknown_plan.readiness_status)
    );
    println!(
        "Verified legacy, no conflicts: {}",
        status_label(legacy_plan.readiness_status)
    );
    println!(
        "Verified legacy, Unicode 18 conflict: {}",
        status_label(conflict_plan.readiness_status)
    );
    println!(
        "Verified current, no conflicts: {}",
        status_label(current_plan.readiness_status)
    );
    println!(
        "Empty vocabulary: {}",
        status_label(empty_plan.readiness_status)
    );
    println!(
        "Unsupported target marker: {}",
        if matches!(
            unsupported_target,
            Err(SyntheticPlanningError::InvalidTargetContract(
                ProposedTargetContractError::UnsupportedMarker
            ))
        ) {
            "REJECTED"
        } else {
            "ACCEPTED"
        }
    );
    println!();
    println!(
        "Original settings modified: {}",
        if no_source_mutation { "NO" } else { "YES" }
    );
    println!(
        "Vocabulary entries deleted: {}",
        if no_vocabulary_entries_deleted {
            "NO"
        } else {
            "YES"
        }
    );
    println!(
        "Migration applied: {}",
        if no_migration { "NO" } else { "YES" }
    );
    println!("Settings files written: NO");
    println!(
        "Production schema-v2 loading: {}",
        if production_v2_rejected {
            "NOT IMPLEMENTED"
        } else {
            "ACCEPTED"
        }
    );

    assert_eq!(
        unknown_plan.readiness_status,
        SyntheticMigrationPlanStatus::SourceProvenanceReviewRequired
    );
    assert_eq!(
        legacy_plan.readiness_status,
        SyntheticMigrationPlanStatus::CompatibleForFutureMigrationReview
    );
    assert_eq!(
        conflict_plan.readiness_status,
        SyntheticMigrationPlanStatus::ManualConflictResolutionRequired
    );
    assert_eq!(
        current_plan.readiness_status,
        SyntheticMigrationPlanStatus::AlreadyCurrentForReview
    );
    assert_eq!(
        empty_plan.readiness_status,
        SyntheticMigrationPlanStatus::NoVocabularyMigrationNeeded
    );
    assert!(matches!(
        unsupported_target,
        Err(SyntheticPlanningError::InvalidTargetContract(
            ProposedTargetContractError::UnsupportedMarker
        ))
    ));
    assert!(no_source_mutation);
    assert!(no_vocabulary_entries_deleted);
    assert!(no_migration);
    assert!(production_v2_rejected);
}

fn production_decoder_rejects_schema_v2() -> bool {
    let Ok(json) = Settings::default().to_json() else {
        return false;
    };
    let Ok(mut value) = serde_json::from_str::<Value>(&json) else {
        return false;
    };
    value["schema_version"] = json!(2);
    matches!(
        Settings::from_json(&value.to_string()),
        Err(SettingsJsonError::InvalidSettings(
            SettingsError::UnsupportedSchemaVersion { found: 2 }
        ))
    )
}

#[test]
fn target_contract_errors_do_not_include_untrusted_marker_text() {
    let settings = source(&[]);
    let private_marker = "synthetic-private-marker";
    let target = json!({
        "schema_version": 2,
        "vocabulary_matching_policy": private_marker,
        "vocabulary": [],
    })
    .to_string();
    let error =
        plan_synthetic_vocabulary_migration(&settings, &target, VocabularySourcePolicy::Unknown)
            .expect_err("unrecognized marker is rejected");

    assert_eq!(
        error,
        SyntheticPlanningError::InvalidTargetContract(
            ProposedTargetContractError::UnsupportedMarker
        )
    );
    assert!(!format!("{error:?}").contains(private_marker));
}
