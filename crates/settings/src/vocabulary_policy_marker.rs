//! Isolated, versioned vocabulary matching-policy declarations for C-06/C-08.
//!
//! This module defines a serializable marker and a pure inspection helper. A
//! recognized marker is only a declaration; it does not prove the provenance
//! of settings data or authorize a migration. The production schema v1 DTO
//! and loader do not use this module.

use serde::Serialize;
use serde::de::value::{BorrowedStrDeserializer, Error as ValueError};
use serde::de::{Deserialize, Visitor};
use std::fmt::{self, Formatter};

/// Stable serialized identifier for the approved Unicode 18 canonical-caseless V1 policy.
pub const UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER: &str = "unicode18-canonical-caseless-v1";

/// A vocabulary matching-policy declaration with a stable serialized identifier.
///
/// This value identifies Unicode 18.0.0 canonical-caseless matching using
/// default full non-Turkic case folding and NFD normalization before and after
/// folding. The matching-policy interface version is V1. It contains no
/// vocabulary text or comparison keys and is not evidence of source-policy
/// provenance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum VocabularyPolicyMarker {
    /// Unicode 18.0.0 canonical-caseless matching, matching-policy interface V1.
    #[serde(rename = "unicode18-canonical-caseless-v1")]
    Unicode18CanonicalCaselessV1,
}

impl<'de> Deserialize<'de> for VocabularyPolicyMarker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct MarkerVisitor;

        impl Visitor<'_> for MarkerVisitor {
            type Value = VocabularyPolicyMarker;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str("the exact supported vocabulary policy marker identifier")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER => {
                        Ok(VocabularyPolicyMarker::Unicode18CanonicalCaselessV1)
                    }
                    _ => Err(E::custom("unsupported vocabulary policy marker")),
                }
            }
        }

        deserializer.deserialize_str(MarkerVisitor)
    }
}

impl VocabularyPolicyMarker {
    /// Returns the exact stable identifier serialized for this marker.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::Unicode18CanonicalCaselessV1 => UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER,
        }
    }
}

/// Read-only interpretation of a raw vocabulary-policy marker value.
///
/// `Recognized` means only that the declaration has a supported identifier. It
/// does not establish a [`crate::VocabularySourcePolicy`] or migration
/// eligibility. Unsupported values are intentionally not retained, avoiding
/// accidental disclosure of arbitrary untrusted strings in diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocabularyPolicyMarkerInspection {
    /// No marker value was supplied.
    Absent,
    /// The value exactly matches a supported marker identifier.
    Recognized(VocabularyPolicyMarker),
    /// A value was supplied, but this implementation does not recognize it.
    Unsupported,
}

/// Interprets the presence of a raw marker without establishing provenance.
///
/// Matching is case-sensitive and has no aliases or fallback policy. The
/// function performs no file access, settings mutation, logging, or migration.
///
/// # Examples
///
/// ```
/// use outflow_settings::{
///     VocabularyPolicyMarker, VocabularyPolicyMarkerInspection,
///     inspect_vocabulary_policy_marker,
/// };
///
/// assert_eq!(
///     inspect_vocabulary_policy_marker(Some("unicode18-canonical-caseless-v1")),
///     VocabularyPolicyMarkerInspection::Recognized(
///         VocabularyPolicyMarker::Unicode18CanonicalCaselessV1,
///     ),
/// );
/// assert_eq!(
///     inspect_vocabulary_policy_marker(None),
///     VocabularyPolicyMarkerInspection::Absent,
/// );
/// ```
pub fn inspect_vocabulary_policy_marker(
    raw_marker: Option<&str>,
) -> VocabularyPolicyMarkerInspection {
    let Some(raw_marker) = raw_marker else {
        return VocabularyPolicyMarkerInspection::Absent;
    };

    match VocabularyPolicyMarker::deserialize(BorrowedStrDeserializer::<ValueError>::new(
        raw_marker,
    )) {
        Ok(marker) => VocabularyPolicyMarkerInspection::Recognized(marker),
        Err(_) => VocabularyPolicyMarkerInspection::Unsupported,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER, VocabularyPolicyMarker,
        VocabularyPolicyMarkerInspection, inspect_vocabulary_policy_marker,
    };
    use crate::{
        CURRENT_SCHEMA_VERSION, Settings, VocabularyEntry, VocabularySourcePolicy,
        preflight_vocabulary_compatibility,
    };
    use serde::{Deserialize, Serialize};

    /// Contract-only future shape: deliberately not part of Settings or its loader.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    struct ProposedSettingsV2Fixture {
        schema_version: u32,
        vocabulary_matching_policy: VocabularyPolicyMarker,
    }

    #[test]
    fn vocabulary_policy_marker_serializes_as_the_exact_stable_identifier() {
        let marker = VocabularyPolicyMarker::Unicode18CanonicalCaselessV1;

        assert_eq!(
            marker.identifier(),
            UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER
        );
        assert_eq!(
            serde_json::to_string(&marker).expect("marker serialization succeeds"),
            "\"unicode18-canonical-caseless-v1\""
        );
    }

    #[test]
    fn vocabulary_policy_marker_deserializes_the_exact_stable_identifier() {
        let marker =
            serde_json::from_str::<VocabularyPolicyMarker>("\"unicode18-canonical-caseless-v1\"")
                .expect("stable marker identifier deserializes");

        assert_eq!(marker, VocabularyPolicyMarker::Unicode18CanonicalCaselessV1);
    }

    #[test]
    fn vocabulary_policy_marker_deserialization_is_case_sensitive_and_has_no_aliases() {
        for raw in [
            "\"Unicode18-canonical-caseless-v1\"",
            "\"UNICODE18-CANONICAL-CASELESS-V1\"",
            "\"Unicode18CanonicalCaselessV1\"",
        ] {
            assert!(serde_json::from_str::<VocabularyPolicyMarker>(raw).is_err());
        }
        assert_eq!(
            inspect_vocabulary_policy_marker(Some("Unicode18-canonical-caseless-v1")),
            VocabularyPolicyMarkerInspection::Unsupported
        );
    }

    #[test]
    fn vocabulary_policy_marker_deserialization_rejects_malformed_values() {
        for raw in [
            "null",
            "1",
            "{}",
            "[]",
            "\"\"",
            "\" unicode18-canonical-caseless-v1\"",
        ] {
            assert!(serde_json::from_str::<VocabularyPolicyMarker>(raw).is_err());
        }
        for raw in [
            "",
            " unicode18-canonical-caseless-v1",
            "unicode18-canonical-caseless-v1 ",
        ] {
            assert_eq!(
                inspect_vocabulary_policy_marker(Some(raw)),
                VocabularyPolicyMarkerInspection::Unsupported
            );
        }
    }

    #[test]
    fn vocabulary_policy_marker_rejects_unknown_future_and_other_unicode_versions() {
        for raw in [
            "some-other-policy",
            "unicode18-canonical-caseless-v2",
            "unicode-18.0.0-canonical-caseless-v1",
            "unicode19-canonical-caseless-v1",
            "unicode-19.0.0-canonical-caseless-v1",
            "rust-str-to-lowercase-v1",
        ] {
            assert_eq!(
                inspect_vocabulary_policy_marker(Some(raw)),
                VocabularyPolicyMarkerInspection::Unsupported
            );
            let json_value = serde_json::to_string(raw).expect("synthetic marker JSON encodes");
            assert!(serde_json::from_str::<VocabularyPolicyMarker>(&json_value).is_err());
        }

        let private_marker = "private-untrusted-marker-value";
        let private_json = serde_json::to_string(private_marker).expect("marker JSON encodes");
        let error = serde_json::from_str::<VocabularyPolicyMarker>(&private_json)
            .expect_err("unknown marker is rejected")
            .to_string();
        assert!(!error.contains(private_marker));
    }

    #[test]
    fn vocabulary_policy_marker_missing_value_is_absent() {
        assert_eq!(
            inspect_vocabulary_policy_marker(None),
            VocabularyPolicyMarkerInspection::Absent
        );
    }

    #[test]
    fn vocabulary_policy_marker_unknown_value_has_no_lowercase_fallback() {
        assert_eq!(
            inspect_vocabulary_policy_marker(Some("legacy-lowercase-v1")),
            VocabularyPolicyMarkerInspection::Unsupported
        );
        assert_eq!(
            inspect_vocabulary_policy_marker(Some("unicode18-canonical-caseless-v1 ")),
            VocabularyPolicyMarkerInspection::Unsupported
        );
    }

    #[test]
    fn vocabulary_policy_marker_interpretation_is_deterministic_and_read_only() {
        let raw = String::from("unicode18-canonical-caseless-v1");
        let before = raw.clone();
        let first = inspect_vocabulary_policy_marker(Some(&raw));

        assert_eq!(
            first,
            VocabularyPolicyMarkerInspection::Recognized(
                VocabularyPolicyMarker::Unicode18CanonicalCaselessV1
            )
        );
        for _ in 0..10 {
            assert_eq!(inspect_vocabulary_policy_marker(Some(&raw)), first);
        }
        assert_eq!(raw, before);
    }

    #[test]
    fn vocabulary_policy_marker_stores_no_vocabulary_or_identity_data() {
        let marker = VocabularyPolicyMarker::Unicode18CanonicalCaselessV1;
        let serialized = serde_json::to_string(&marker).expect("marker serialization succeeds");

        assert_eq!(serialized, "\"unicode18-canonical-caseless-v1\"");
        for private_value in [
            "synthetic spoken phrase",
            "synthetic written replacement",
            "comparison key",
            "en-US",
            "device-123",
            "user-456",
            "2026-11-02T00:00:00Z",
        ] {
            assert!(!serialized.contains(private_value));
        }
    }

    #[test]
    fn vocabulary_policy_marker_contract_fixture_uses_a_string_in_proposed_schema_v2() {
        let fixture = ProposedSettingsV2Fixture {
            schema_version: 2,
            vocabulary_matching_policy: VocabularyPolicyMarker::Unicode18CanonicalCaselessV1,
        };
        let json = serde_json::to_string(&fixture).expect("contract fixture serializes");
        let decoded: ProposedSettingsV2Fixture =
            serde_json::from_str(&json).expect("contract fixture deserializes");

        assert_eq!(decoded, fixture);
        assert_eq!(decoded.schema_version, 2);
        assert!(
            json.contains("\"vocabulary_matching_policy\":\"unicode18-canonical-caseless-v1\"")
        );
    }

    #[test]
    fn vocabulary_policy_marker_does_not_establish_verified_source_provenance() {
        let fixture_json = r#"{
            "schema_version": 2,
            "vocabulary_matching_policy": "unicode18-canonical-caseless-v1"
        }"#;
        let fixture: ProposedSettingsV2Fixture =
            serde_json::from_str(fixture_json).expect("contract fixture deserializes");
        assert_eq!(
            inspect_vocabulary_policy_marker(Some(fixture.vocabulary_matching_policy.identifier())),
            VocabularyPolicyMarkerInspection::Recognized(
                VocabularyPolicyMarker::Unicode18CanonicalCaselessV1
            )
        );

        let settings = Settings {
            vocabulary: vec![VocabularyEntry {
                id: "synthetic-entry".to_owned(),
                spoken: "synthetic spoken phrase".to_owned(),
                written: "synthetic written replacement".to_owned(),
            }],
            ..Settings::default()
        };
        assert!(matches!(
            preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown),
            Ok(crate::VocabularyMigrationReadiness::SourcePolicyUnknown { .. })
        ));
    }

    #[test]
    fn vocabulary_policy_marker_does_not_change_schema_v1_serialization() {
        let settings = Settings::default();
        let json = settings.to_json().expect("schema v1 settings serialize");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("settings serializer emits JSON");
        let fields = value
            .as_object()
            .expect("settings JSON is an object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
        assert!(
            !value
                .as_object()
                .expect("root is object")
                .contains_key("vocabulary_matching_policy")
        );
        assert_eq!(
            fields,
            [
                "schema_version",
                "language",
                "recording",
                "cleanup",
                "startup",
                "updates",
                "vocabulary",
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn vocabulary_policy_marker_does_not_change_schema_v1_deserialization() {
        let json = Settings::default()
            .to_json()
            .expect("schema v1 settings serialize");
        let decoded = Settings::from_json(&json).expect("schema v1 settings deserialize");

        assert_eq!(decoded, Settings::default());
        assert!(
            decoded
                .to_json()
                .expect("schema v1 settings reserialize")
                .contains("\"schema_version\":1")
        );
    }

    #[test]
    fn vocabulary_policy_marker_field_remains_unknown_in_schema_v1() {
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("schema v1 settings serialize"),
        )
        .expect("settings JSON parses");
        value["vocabulary_matching_policy"] =
            serde_json::json!(UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER);

        assert!(matches!(
            Settings::from_json(&value.to_string()),
            Err(crate::SettingsJsonError::InvalidDocument { .. })
        ));
    }

    #[test]
    fn vocabulary_policy_marker_does_not_change_unsupported_schema_rejection() {
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("schema v1 settings serialize"),
        )
        .expect("settings JSON parses");
        value["schema_version"] = serde_json::json!(2);

        assert_eq!(
            Settings::from_json(&value.to_string()),
            Err(crate::SettingsJsonError::InvalidSettings(
                crate::SettingsError::UnsupportedSchemaVersion { found: 2 }
            ))
        );
    }

    #[test]
    fn vocabulary_policy_marker_preflight_remains_read_only_and_provenance_explicit() {
        let settings = Settings {
            vocabulary: vec![VocabularyEntry {
                id: "synthetic-entry".to_owned(),
                spoken: "synthetic spoken phrase".to_owned(),
                written: "synthetic written replacement".to_owned(),
            }],
            ..Settings::default()
        };
        let original = settings.clone();
        let original_json = settings
            .to_json()
            .expect("settings serialize before preflight");
        let result = preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown);

        assert!(matches!(
            result,
            Ok(crate::VocabularyMigrationReadiness::SourcePolicyUnknown { .. })
        ));
        assert_eq!(settings, original);
        assert_eq!(
            settings
                .to_json()
                .expect("settings serialize after preflight"),
            original_json
        );
    }

    #[test]
    fn vocabulary_policy_marker_contract_summary_uses_real_interpretation_results() {
        let fixture = ProposedSettingsV2Fixture {
            schema_version: 2,
            vocabulary_matching_policy: VocabularyPolicyMarker::Unicode18CanonicalCaselessV1,
        };
        let fixture_json = serde_json::to_string(&fixture).expect("contract fixture serializes");
        let decoded_fixture: ProposedSettingsV2Fixture =
            serde_json::from_str(&fixture_json).expect("contract fixture deserializes");
        let valid = inspect_vocabulary_policy_marker(Some(
            decoded_fixture.vocabulary_matching_policy.identifier(),
        ));
        let missing = inspect_vocabulary_policy_marker(None);
        let unsupported = inspect_vocabulary_policy_marker(Some("future-policy-v9"));

        let settings = Settings {
            vocabulary: vec![VocabularyEntry {
                id: "synthetic-entry".to_owned(),
                spoken: "synthetic spoken phrase".to_owned(),
                written: "synthetic written replacement".to_owned(),
            }],
            ..Settings::default()
        };
        let settings_before = settings.clone();
        let readiness =
            preflight_vocabulary_compatibility(&settings, VocabularySourcePolicy::Unknown)
                .expect("synthetic settings vocabulary is assessable");
        let verified_provenance_established = !matches!(
            readiness,
            crate::VocabularyMigrationReadiness::SourcePolicyUnknown { .. }
        );
        let migration_applied = settings.schema_version != CURRENT_SCHEMA_VERSION;
        let settings_files_modified = settings != settings_before;

        let label = |inspection| match inspection {
            VocabularyPolicyMarkerInspection::Absent => "ABSENT",
            VocabularyPolicyMarkerInspection::Recognized(_) => "RECOGNIZED",
            VocabularyPolicyMarkerInspection::Unsupported => "REJECTED",
        };
        println!("Outflow vocabulary policy marker contract");
        println!();
        println!("Current identifier: {UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER}");
        println!("Valid marker: {}", label(valid));
        println!("Missing marker: {}", label(missing));
        println!("Unsupported marker: {}", label(unsupported));
        println!(
            "Verified provenance established: {}",
            if verified_provenance_established {
                "YES"
            } else {
                "NO"
            }
        );
        println!();
        println!("Production settings schema: v{CURRENT_SCHEMA_VERSION}");
        println!(
            "Proposed future schema: v{}",
            decoded_fixture.schema_version
        );
        println!(
            "Schema migration applied: {}",
            if migration_applied { "YES" } else { "NO" }
        );
        println!(
            "Settings files modified: {}",
            if settings_files_modified { "YES" } else { "NO" }
        );

        assert_eq!(
            valid,
            VocabularyPolicyMarkerInspection::Recognized(
                VocabularyPolicyMarker::Unicode18CanonicalCaselessV1
            )
        );
        assert_eq!(missing, VocabularyPolicyMarkerInspection::Absent);
        assert_eq!(unsupported, VocabularyPolicyMarkerInspection::Unsupported);
        assert!(matches!(
            readiness,
            crate::VocabularyMigrationReadiness::EmptyCollection
                | crate::VocabularyMigrationReadiness::SourcePolicyUnknown { .. }
        ));
        assert!(!verified_provenance_established);
        assert!(!migration_applied);
        assert!(!settings_files_modified);
    }
}
