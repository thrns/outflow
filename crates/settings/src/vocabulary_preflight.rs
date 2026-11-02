//! Opt-in, read-only vocabulary compatibility preflight for C-06/C-08.
//!
//! This adapter borrows vocabulary strings from the settings DTO and delegates
//! vocabulary validation and matching-policy assessment to `outflow-core`. It
//! does not load, initialize, run full-settings validation, serialize, migrate,
//! or save settings. Callers must supply trusted source-policy provenance
//! explicitly because schema v1 has no matching-policy marker. Existing
//! settings decode/load paths validate before this API can be called and may
//! reject malformed or legacy-conflicting collections first; this API is not a
//! raw-file recovery mechanism.

use crate::Settings;
use outflow_core::vocabulary::VocabularyEntryRef;
use outflow_core::vocabulary_migration::assess_vocabulary_migration_readiness;

pub use outflow_core::vocabulary::{VocabularyField, VocabularyValidationError};
pub use outflow_core::vocabulary_migration::{
    VocabularyMatchingPolicyId, VocabularyMigrationConflict, VocabularyMigrationConflictKind,
    VocabularyMigrationError, VocabularyMigrationReadiness, VocabularySourcePolicy,
};

/// Assesses an in-memory settings vocabulary against Unicode 18 matching.
///
/// Vocabulary entries are adapted as temporary borrowed views in their
/// original order. The core readiness API applies the shared 500-entry and
/// per-field validation rules and returns index-only conflict reports. The
/// settings DTO and its strings remain unchanged; no comparison keys are
/// retained by this adapter.
///
/// The caller must choose `source_policy` from trusted provenance. Settings
/// schema v1 does not record a matching policy, so schema version, file time,
/// application version, operating system, and absence of detected conflicts
/// do not establish provenance. `Unknown` remains review-required for any
/// nonempty collection.
///
/// This API accepts an already-constructed settings object only. Existing
/// deserialization and load paths validate settings using lowercase-only
/// matching and can reject malformed input or pre-existing lowercase
/// conflicts before this function is available. It neither changes nor
/// bypasses those paths.
///
/// A compatible result means only that the supplied source policy has no
/// conflicts under the assessed rules. It is not a migration, conflict
/// resolution, authorization to save, or evidence that settings were loaded
/// or persisted.
///
/// # Errors
///
/// Returns the core API's typed error for more than 500 entries or the first
/// invalid spoken or written phrase. Error details contain entry indices and
/// field/reason types, never vocabulary text.
pub fn preflight_vocabulary_compatibility(
    settings: &Settings,
    source_policy: VocabularySourcePolicy,
) -> Result<VocabularyMigrationReadiness, VocabularyMigrationError> {
    let entries = settings
        .vocabulary
        .iter()
        .map(|entry| VocabularyEntryRef {
            spoken: &entry.spoken,
            written: &entry.written,
        })
        .collect::<Vec<_>>();

    assess_vocabulary_migration_readiness(&entries, source_policy)
}
