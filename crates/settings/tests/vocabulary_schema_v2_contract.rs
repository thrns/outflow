use outflow_core::vocabulary::{
    VocabularyCollectionError, VocabularyEntryRef, validate_vocabulary_collection,
};
use outflow_core::vocabulary_matching::{VocabularyMatchingVersion, comparison_key};
use outflow_core::vocabulary_migration::{
    VocabularyMatchingPolicyId, VocabularyMigrationConflict, VocabularyMigrationConflictKind,
    VocabularyMigrationReadiness, VocabularySourcePolicy, assess_vocabulary_migration_readiness,
};
use outflow_settings::{
    CURRENT_SCHEMA_VERSION, Settings, SettingsError, SettingsJsonError,
    UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER, VocabularyPolicyMarker,
    VocabularyPolicyMarkerInspection, inspect_vocabulary_policy_marker,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const PROPOSED_SCHEMA_VERSION: u32 = 2;

/// Partial contract-only shape. This is not a production settings DTO.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProposedSettingsV2Fixture {
    schema_version: u32,
    vocabulary_matching_policy: VocabularyPolicyMarker,
    vocabulary: Vec<FixtureVocabularyEntry>,
}

/// Test-only vocabulary shape with synthetic values and no production IDs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixtureVocabularyEntry {
    spoken: String,
    written: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixtureContractError {
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

/// Parses only this integration test's proposed partial envelope.
///
/// Version selection happens before DTO deserialization so an unknown version
/// cannot be reinterpreted as v1. The marker API reports declarations only;
/// it does not choose a source-policy provenance value.
fn parse_proposed_v2_fixture(
    input: &str,
) -> Result<ProposedSettingsV2Fixture, FixtureContractError> {
    let value: Value =
        serde_json::from_str(input).map_err(|_| FixtureContractError::InvalidJson)?;
    let object = value
        .as_object()
        .ok_or(FixtureContractError::InvalidEnvelope)?;

    let schema_version = object
        .get("schema_version")
        .ok_or(FixtureContractError::MissingSchemaVersion)?
        .as_u64()
        .ok_or(FixtureContractError::MalformedSchemaVersion)?;
    if schema_version > u64::from(PROPOSED_SCHEMA_VERSION) {
        return Err(FixtureContractError::UnsupportedSchemaVersion);
    }
    if schema_version != u64::from(PROPOSED_SCHEMA_VERSION) {
        return Err(FixtureContractError::WrongProposedSchemaVersion);
    }

    match object.get("vocabulary_matching_policy") {
        None => return Err(FixtureContractError::MissingMarker),
        Some(Value::String(raw_marker)) => match inspect_vocabulary_policy_marker(Some(raw_marker))
        {
            VocabularyPolicyMarkerInspection::Recognized(_) => {}
            VocabularyPolicyMarkerInspection::Unsupported => {
                return Err(FixtureContractError::UnsupportedMarker);
            }
            VocabularyPolicyMarkerInspection::Absent => {
                return Err(FixtureContractError::MissingMarker);
            }
        },
        Some(_) => return Err(FixtureContractError::MalformedMarkerType),
    }

    let vocabulary = object
        .get("vocabulary")
        .ok_or(FixtureContractError::MissingVocabulary)?
        .as_array()
        .ok_or(FixtureContractError::MalformedVocabulary)?;
    for entry in vocabulary {
        let entry = entry
            .as_object()
            .ok_or(FixtureContractError::MalformedVocabulary)?;
        if !entry.get("spoken").is_some_and(Value::is_string)
            || !entry.get("written").is_some_and(Value::is_string)
        {
            return Err(FixtureContractError::MalformedVocabulary);
        }
    }

    serde_json::from_value(value).map_err(|_| FixtureContractError::MalformedEnvelopeFields)
}

fn valid_document(entries: &[FixtureVocabularyEntry]) -> Value {
    json!({
        "schema_version": PROPOSED_SCHEMA_VERSION,
        "vocabulary_matching_policy": UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER,
        "vocabulary": entries,
    })
}

fn fixture_entry(spoken: &str, written: &str) -> FixtureVocabularyEntry {
    FixtureVocabularyEntry {
        spoken: spoken.to_owned(),
        written: written.to_owned(),
    }
}

fn parse_value(value: &Value) -> Result<ProposedSettingsV2Fixture, FixtureContractError> {
    parse_proposed_v2_fixture(&value.to_string())
}

fn borrowed_entries(fixture: &ProposedSettingsV2Fixture) -> Vec<VocabularyEntryRef<'_>> {
    fixture
        .vocabulary
        .iter()
        .map(|entry| VocabularyEntryRef {
            spoken: &entry.spoken,
            written: &entry.written,
        })
        .collect()
}

fn expected_new_unicode18_conflict(
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

#[test]
fn proposed_v2_fixture_accepts_the_minimal_valid_envelope() {
    let source = valid_document(&[fixture_entry("North Parkade", "North Parkade")]);
    let fixture = parse_value(&source).expect("valid partial v2 envelope is accepted");

    assert_eq!(fixture.schema_version, 2);
    assert_eq!(
        fixture.vocabulary_matching_policy,
        VocabularyPolicyMarker::Unicode18CanonicalCaselessV1
    );
    assert_eq!(
        fixture.vocabulary[0],
        fixture_entry("North Parkade", "North Parkade")
    );
}

#[test]
fn marker_inspection_recognizes_only_the_exact_unicode18_identifier() {
    assert_eq!(
        VocabularyPolicyMarker::Unicode18CanonicalCaselessV1.identifier(),
        "unicode18-canonical-caseless-v1"
    );
    assert_eq!(
        inspect_vocabulary_policy_marker(Some("unicode18-canonical-caseless-v1")),
        VocabularyPolicyMarkerInspection::Recognized(
            VocabularyPolicyMarker::Unicode18CanonicalCaselessV1
        )
    );
}

#[test]
fn missing_marker_is_inspectable_as_absent_but_fails_the_v2_contract() {
    let mut source = valid_document(&[]);
    source
        .as_object_mut()
        .expect("synthetic document is an object")
        .remove("vocabulary_matching_policy");

    assert_eq!(
        inspect_vocabulary_policy_marker(None),
        VocabularyPolicyMarkerInspection::Absent
    );
    assert_eq!(
        parse_value(&source),
        Err(FixtureContractError::MissingMarker)
    );
}

#[test]
fn unsupported_marker_is_rejected_without_a_legacy_fallback() {
    let marker = "future-policy-v9";
    let source = json!({
        "schema_version": 2,
        "vocabulary_matching_policy": marker,
        "vocabulary": [],
    });

    assert_eq!(
        inspect_vocabulary_policy_marker(Some(marker)),
        VocabularyPolicyMarkerInspection::Unsupported
    );
    assert_eq!(
        parse_value(&source),
        Err(FixtureContractError::UnsupportedMarker)
    );
    assert_eq!(
        inspect_vocabulary_policy_marker(Some("rust-str-to-lowercase-v1")),
        VocabularyPolicyMarkerInspection::Unsupported,
        "legacy lowercase is not an implicit fallback marker"
    );
    let legacy_marker = json!({
        "schema_version": 2,
        "vocabulary_matching_policy": "rust-str-to-lowercase-v1",
        "vocabulary": [],
    });
    assert_eq!(
        parse_value(&legacy_marker),
        Err(FixtureContractError::UnsupportedMarker)
    );
}

#[test]
fn marker_matching_is_case_sensitive_and_has_no_aliases() {
    for marker in [
        "Unicode18-canonical-caseless-v1",
        "UNICODE18-CANONICAL-CASELESS-V1",
        "Unicode18CanonicalCaselessV1",
    ] {
        let source = json!({
            "schema_version": 2,
            "vocabulary_matching_policy": marker,
            "vocabulary": [],
        });
        assert_eq!(
            inspect_vocabulary_policy_marker(Some(marker)),
            VocabularyPolicyMarkerInspection::Unsupported
        );
        assert_eq!(
            parse_value(&source),
            Err(FixtureContractError::UnsupportedMarker)
        );
    }
}

#[test]
fn unsupported_marker_versions_and_unicode_versions_are_rejected() {
    for marker in [
        "unicode18-canonical-caseless-v2",
        "unicode17-canonical-caseless-v1",
        "unicode19-canonical-caseless-v1",
        "unicode-18.0.0-canonical-caseless-v1",
    ] {
        let source = json!({
            "schema_version": 2,
            "vocabulary_matching_policy": marker,
            "vocabulary": [],
        });
        assert_eq!(
            inspect_vocabulary_policy_marker(Some(marker)),
            VocabularyPolicyMarkerInspection::Unsupported
        );
        assert_eq!(
            parse_value(&source),
            Err(FixtureContractError::UnsupportedMarker)
        );
    }
}

#[test]
fn empty_marker_value_is_not_accepted() {
    assert_eq!(
        inspect_vocabulary_policy_marker(Some("")),
        VocabularyPolicyMarkerInspection::Unsupported
    );
    let source = json!({
        "schema_version": 2,
        "vocabulary_matching_policy": "",
        "vocabulary": [],
    });
    assert_eq!(
        parse_value(&source),
        Err(FixtureContractError::UnsupportedMarker)
    );
}

#[test]
fn malformed_marker_json_types_fail_closed() {
    for marker in [json!(null), json!(false), json!(7), json!([]), json!({})] {
        let source = json!({
            "schema_version": 2,
            "vocabulary_matching_policy": marker.clone(),
            "vocabulary": [],
        });
        assert_eq!(
            parse_value(&source),
            Err(FixtureContractError::MalformedMarkerType)
        );
        assert!(serde_json::from_value::<VocabularyPolicyMarker>(marker).is_err());
    }
}

#[test]
fn schema_version_boundaries_are_independent_of_marker_validation() {
    assert!(parse_value(&valid_document(&[])).is_ok());

    let mut v1_with_marker = valid_document(&[]);
    v1_with_marker["schema_version"] = json!(1);
    assert_eq!(
        parse_value(&v1_with_marker),
        Err(FixtureContractError::WrongProposedSchemaVersion)
    );

    let mut v3 = valid_document(&[]);
    v3["schema_version"] = json!(3);
    assert_eq!(
        parse_value(&v3),
        Err(FixtureContractError::UnsupportedSchemaVersion)
    );
}

#[test]
fn missing_and_malformed_schema_versions_are_rejected() {
    let mut missing = valid_document(&[]);
    missing
        .as_object_mut()
        .expect("synthetic document is an object")
        .remove("schema_version");
    assert_eq!(
        parse_value(&missing),
        Err(FixtureContractError::MissingSchemaVersion)
    );

    for schema_version in [json!(null), json!("2"), json!(2.0), json!(-1), json!(true)] {
        let mut malformed = valid_document(&[]);
        malformed["schema_version"] = schema_version;
        assert_eq!(
            parse_value(&malformed),
            Err(FixtureContractError::MalformedSchemaVersion)
        );
    }
}

#[test]
fn missing_or_malformed_vocabulary_fields_are_rejected() {
    let mut missing = valid_document(&[]);
    missing
        .as_object_mut()
        .expect("synthetic document is an object")
        .remove("vocabulary");
    assert_eq!(
        parse_value(&missing),
        Err(FixtureContractError::MissingVocabulary)
    );

    for vocabulary in [
        json!(null),
        json!({}),
        json!([null]),
        json!([{"spoken": "only one field"}]),
        json!([{"spoken": 3, "written": "synthetic"}]),
    ] {
        let mut malformed = valid_document(&[]);
        malformed["vocabulary"] = vocabulary;
        assert_eq!(
            parse_value(&malformed),
            Err(FixtureContractError::MalformedVocabulary)
        );
    }

    let malformed_json = parse_proposed_v2_fixture(
        r#"{"schema_version":2,"vocabulary_matching_policy":"unicode18-canonical-caseless-v1","vocabulary":[{"spoken":"synthetic-private-value","written": }]}"#,
    );
    assert_eq!(malformed_json, Err(FixtureContractError::InvalidJson));
    assert!(!format!("{malformed_json:?}").contains("synthetic-private-value"));
}

#[test]
fn empty_vocabulary_array_is_a_valid_empty_contract_collection() {
    let fixture = parse_value(&valid_document(&[])).expect("empty array is a valid field");
    assert!(fixture.vocabulary.is_empty());
    assert_eq!(
        assess_vocabulary_migration_readiness(
            &borrowed_entries(&fixture),
            VocabularySourcePolicy::Unknown
        ),
        Ok(VocabularyMigrationReadiness::EmptyCollection)
    );
}

#[test]
fn one_entry_vocabulary_validates_under_unicode18() {
    let fixture = parse_value(&valid_document(&[fixture_entry(
        "North Parkade",
        "North Parkade",
    )]))
    .expect("single-entry fixture parses");
    let entries = borrowed_entries(&fixture);

    assert_eq!(validate_vocabulary_collection(&entries), Ok(()));
    assert!(matches!(
        assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts }) if conflicts.is_empty()
    ));
}

#[test]
fn multiple_distinct_vocabulary_entries_validate_under_unicode18() {
    let fixture = parse_value(&valid_document(&[
        fixture_entry("North Parkade", "North Parkade"),
        fixture_entry("Rust", "Rust"),
        fixture_entry("café", "Cafe"),
    ]))
    .expect("multiple-entry fixture parses");

    assert_eq!(
        validate_vocabulary_collection(&borrowed_entries(&fixture)),
        Ok(())
    );
}

#[test]
fn german_sharp_s_collision_is_new_under_unicode18() {
    let fixture = parse_value(&valid_document(&[
        fixture_entry("straße", "Street"),
        fixture_entry("STRASSE", "Avenue"),
    ]))
    .expect("synthetic collision fixture parses");
    let original_fixture = fixture.clone();
    let entries = borrowed_entries(&fixture);

    assert_eq!(
        comparison_key(
            "straße",
            VocabularyMatchingVersion::Unicode18CanonicalCaselessV1
        ),
        comparison_key(
            "STRASSE",
            VocabularyMatchingVersion::Unicode18CanonicalCaselessV1
        )
    );
    assert!(matches!(
        validate_vocabulary_collection(&entries),
        Err(VocabularyCollectionError::DuplicateSpoken {
            first_index: 0,
            duplicate_index: 1,
        })
    ));
    assert_eq!(
        assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
            conflicts: vec![expected_new_unicode18_conflict(0, 1)],
        })
    );
    assert_eq!(fixture.vocabulary[0].written, "Street");
    assert_eq!(fixture.vocabulary[1].written, "Avenue");
    assert_eq!(fixture, original_fixture);
}

#[test]
fn composed_and_decomposed_accent_collision_preserves_original_input() {
    let composed = "\u{00E9}";
    let decomposed = "e\u{0301}";
    let fixture = parse_value(&valid_document(&[
        fixture_entry(composed, "Example A"),
        fixture_entry(decomposed, "Example B"),
    ]))
    .expect("canonically equivalent fixture parses");
    let original_fixture = fixture.clone();
    let entries = borrowed_entries(&fixture);

    assert_eq!(
        comparison_key(
            composed,
            VocabularyMatchingVersion::Unicode18CanonicalCaselessV1
        ),
        comparison_key(
            decomposed,
            VocabularyMatchingVersion::Unicode18CanonicalCaselessV1
        )
    );
    assert_eq!(
        assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
            conflicts: vec![expected_new_unicode18_conflict(0, 1)],
        })
    );
    assert_eq!(fixture.vocabulary[0].spoken, composed);
    assert_eq!(fixture.vocabulary[1].spoken, decomposed);
    assert_ne!(fixture.vocabulary[0].spoken, fixture.vocabulary[1].spoken);
    assert_eq!(fixture, original_fixture);
}

#[test]
fn greek_sigma_collision_is_new_under_unicode18() {
    let fixture = parse_value(&valid_document(&[
        fixture_entry("σ", "Example A"),
        fixture_entry("ς", "Example B"),
    ]))
    .expect("Greek sigma fixture parses");
    let original_fixture = fixture.clone();
    let entries = borrowed_entries(&fixture);

    assert_ne!("σ".to_lowercase(), "ς".to_lowercase());
    assert_eq!(
        comparison_key("σ", VocabularyMatchingVersion::Unicode18CanonicalCaselessV1),
        comparison_key("ς", VocabularyMatchingVersion::Unicode18CanonicalCaselessV1)
    );
    assert_eq!(
        assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
            conflicts: vec![expected_new_unicode18_conflict(0, 1)],
        })
    );
    assert_eq!(fixture, original_fixture);
}

#[test]
fn migration_report_contains_exact_indices_and_conflict_classifications() {
    let fixture = parse_value(&valid_document(&[
        fixture_entry("straße", "Street"),
        fixture_entry("STRASSE", "Avenue"),
        fixture_entry("\u{00E9}", "Example A"),
        fixture_entry("e\u{0301}", "Example B"),
        fixture_entry("σ", "Example C"),
        fixture_entry("ς", "Example D"),
    ]))
    .expect("combined synthetic conflict fixture parses");
    let original_fixture = fixture.clone();
    let entries = borrowed_entries(&fixture);
    let conflicts = vec![
        expected_new_unicode18_conflict(0, 1),
        expected_new_unicode18_conflict(2, 3),
        expected_new_unicode18_conflict(4, 5),
    ];

    assert_eq!(
        assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
            conflicts: conflicts.clone(),
        })
    );
    assert_eq!(
        assess_vocabulary_migration_readiness(
            &entries,
            VocabularySourcePolicy::VerifiedLegacyLowercase
        ),
        Ok(VocabularyMigrationReadiness::ManualResolutionRequired { conflicts })
    );
    assert_eq!(fixture, original_fixture);
}

#[test]
fn recognized_marker_does_not_establish_source_policy_provenance() {
    let fixture = parse_value(&valid_document(&[fixture_entry(
        "North Parkade",
        "North Parkade",
    )]))
    .expect("recognized marker fixture parses");
    assert_eq!(
        inspect_vocabulary_policy_marker(Some(fixture.vocabulary_matching_policy.identifier())),
        VocabularyPolicyMarkerInspection::Recognized(
            VocabularyPolicyMarker::Unicode18CanonicalCaselessV1
        )
    );
    let entries = borrowed_entries(&fixture);

    assert_eq!(
        assess_vocabulary_migration_readiness(&entries, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown {
            conflicts: Vec::new(),
        }),
        "neither the marker, v2 version, nor conflict-free data proves provenance"
    );

    // TEST ASSUMPTION: these verified policies are supplied explicitly by the
    // synthetic caller; neither value is derived from the JSON marker.
    assert_eq!(
        assess_vocabulary_migration_readiness(
            &entries,
            VocabularySourcePolicy::VerifiedLegacyLowercase
        ),
        Ok(VocabularyMigrationReadiness::CompatibleForExplicitMigration)
    );
    assert_eq!(
        assess_vocabulary_migration_readiness(
            &entries,
            VocabularySourcePolicy::VerifiedUnicode18CanonicalCaselessV1
        ),
        Ok(VocabularyMigrationReadiness::AlreadyCurrent)
    );
}

#[test]
fn production_schema_v1_defaults_and_serialization_remain_unchanged() {
    let settings = Settings::default();
    let serialized = settings.to_json().expect("default settings serialize");
    let serialized_value: Value =
        serde_json::from_str(&serialized).expect("production JSON is valid");

    assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    assert_eq!(settings.schema_version, 1);
    assert_eq!(
        serialized_value,
        json!({
            "schema_version": 1,
            "language": "en",
            "recording": {
                "push_to_talk": null,
                "toggle": null,
                "input_device": "system-default"
            },
            "cleanup": {"mode": "clean", "remove_fillers": true},
            "startup": {"run_at_login": false},
            "updates": {"user_initiated_only": true},
            "vocabulary": []
        })
    );
    assert!(serialized_value.get("vocabulary_matching_policy").is_none());
    assert_eq!(Settings::from_json(&serialized), Ok(settings.clone()));
    assert_eq!(
        Settings::from_json(&serialized).unwrap().to_json(),
        Ok(serialized)
    );
}

#[test]
fn production_decoder_rejects_schema_v2_and_keeps_v1_as_the_only_version() {
    let proposed = valid_document(&[fixture_entry("North Parkade", "North Parkade")]);
    let proposed_json = proposed.to_string();
    assert!(parse_proposed_v2_fixture(&proposed_json).is_ok());

    let mut production_v2 = serde_json::from_str::<Value>(
        &Settings::default().to_json().expect("schema v1 serializes"),
    )
    .expect("production schema v1 is JSON");
    production_v2["schema_version"] = json!(2);
    let result = Settings::from_json(&production_v2.to_string());
    assert!(matches!(
        result,
        Err(SettingsJsonError::InvalidSettings(
            SettingsError::UnsupportedSchemaVersion { found: 2 }
        ))
    ));
    assert_eq!(CURRENT_SCHEMA_VERSION, 1);

    // Keep the proposed DTO in memory and unchanged; this test calls no file,
    // save, reset, initialization, or migration API.
    let proposed_before = proposed.clone();
    let parsed_proposed = parse_value(&proposed).expect("v2 fixture remains valid");
    let readiness = assess_vocabulary_migration_readiness(
        &borrowed_entries(&parsed_proposed),
        VocabularySourcePolicy::Unknown,
    );
    assert!(matches!(
        readiness,
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts }) if conflicts.is_empty()
    ));
    assert_eq!(proposed, proposed_before);
}

#[test]
fn contract_summary_reports_only_asserted_in_memory_results() {
    let valid = valid_document(&[fixture_entry("North Parkade", "North Parkade")]);
    let valid_fixture = parse_value(&valid).expect("recognized v2 marker is accepted");
    let valid_marker = matches!(
        inspect_vocabulary_policy_marker(Some(
            valid_fixture.vocabulary_matching_policy.identifier()
        )),
        VocabularyPolicyMarkerInspection::Recognized(
            VocabularyPolicyMarker::Unicode18CanonicalCaselessV1
        )
    );

    let mut missing = valid.clone();
    missing
        .as_object_mut()
        .expect("synthetic document is an object")
        .remove("vocabulary_matching_policy");
    let missing_marker_rejected = parse_value(&missing) == Err(FixtureContractError::MissingMarker);

    let unsupported = json!({
        "schema_version": 2,
        "vocabulary_matching_policy": "future-policy-v9",
        "vocabulary": [],
    });
    let unsupported_marker_rejected =
        parse_value(&unsupported) == Err(FixtureContractError::UnsupportedMarker);

    let malformed = json!({
        "schema_version": 2,
        "vocabulary_matching_policy": null,
        "vocabulary": [],
    });
    let malformed_marker_rejected =
        parse_value(&malformed) == Err(FixtureContractError::MalformedMarkerType);

    let conflict_fixture = parse_value(&valid_document(&[
        fixture_entry("straße", "Street"),
        fixture_entry("STRASSE", "Avenue"),
    ]))
    .expect("synthetic conflict fixture parses");
    let conflicts = borrowed_entries(&conflict_fixture);
    let conflict_detected = matches!(
        assess_vocabulary_migration_readiness(&conflicts, VocabularySourcePolicy::Unknown),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts })
            if conflicts == vec![expected_new_unicode18_conflict(0, 1)]
    );

    let unknown_fixture = parse_value(&valid).expect("valid fixture parses");
    let unknown_provenance_review_required = matches!(
        assess_vocabulary_migration_readiness(
            &borrowed_entries(&unknown_fixture),
            VocabularySourcePolicy::Unknown
        ),
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { conflicts })
            if conflicts.is_empty()
    );

    let mut production_v2 = serde_json::from_str::<Value>(
        &Settings::default().to_json().expect("schema v1 serializes"),
    )
    .expect("production schema v1 is JSON");
    production_v2["schema_version"] = json!(2);
    let production_v2_loading_implemented = Settings::from_json(&production_v2.to_string()).is_ok();

    let fixture_before_assessment = unknown_fixture.clone();
    let settings_before = Settings::default();
    let settings_json_before = settings_before
        .to_json()
        .expect("production settings serialize in memory");
    let readiness = assess_vocabulary_migration_readiness(
        &borrowed_entries(&unknown_fixture),
        VocabularySourcePolicy::Unknown,
    );
    assert!(matches!(
        readiness,
        Ok(VocabularyMigrationReadiness::SourcePolicyUnknown { .. })
    ));
    let settings_json_after = settings_before
        .to_json()
        .expect("readiness assessment did not mutate settings");
    let migration_applied = unknown_fixture.schema_version != PROPOSED_SCHEMA_VERSION
        || unknown_fixture != fixture_before_assessment;
    let settings_files_modified = settings_json_before != settings_json_after;

    assert!(valid_marker);
    assert!(missing_marker_rejected);
    assert!(unsupported_marker_rejected);
    assert!(malformed_marker_rejected);
    assert!(conflict_detected);
    assert!(unknown_provenance_review_required);
    assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    assert_eq!(valid_fixture.schema_version, 2);
    assert!(!production_v2_loading_implemented);
    assert!(!migration_applied);
    assert!(!settings_files_modified);

    println!("Outflow proposed schema-v2 contract");
    println!();
    println!("Valid marker: ACCEPTED");
    println!("Missing marker: REJECTED");
    println!("Unsupported marker: REJECTED");
    println!("Malformed marker: REJECTED");
    println!("Unicode 18 vocabulary conflict: DETECTED");
    println!("Unknown provenance: REVIEW REQUIRED");
    println!();
    println!("Production settings schema: v{CURRENT_SCHEMA_VERSION}");
    println!("Proposed fixture schema: v{}", valid_fixture.schema_version);
    println!("Production schema-v2 loading: NOT IMPLEMENTED");
    println!("Migration applied: NO");
    println!("Settings files modified: NO");
}
