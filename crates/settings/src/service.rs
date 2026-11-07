//! Transactional settings workflow for explicit local paths (C-08 / TS-08).
//!
//! The service owns one in-memory snapshot and writes each accepted change
//! through the existing atomic storage functions. It has no Tauri, runtime,
//! operating-system, microphone, model, or network dependencies. A nonempty
//! schema-v1 vocabulary has no recorded matching-policy provenance, so opening
//! it requires an explicit per-instance review acknowledgment before any write.
//! That acknowledgment is deliberately not persisted or inferred from a schema
//! version or marker; callers must review again after reopening the file.

use crate::storage::load_settings_for_vocabulary_review;
use crate::{
    CleanupMode, Settings, SettingsError, SettingsInitialization, SettingsJsonError,
    SettingsStorageError, StartupSettings, VocabularyEntry, VocabularyManagementError,
    VocabularyMigrationError, VocabularyMigrationReadiness, VocabularySourcePolicy,
    add_vocabulary_entry, delete_vocabulary_entry, edit_vocabulary_entry, initialize_settings,
    preflight_vocabulary_compatibility, reset_settings_to_defaults, save_settings_atomic,
};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

/// Supported mutable preferences for the schema-v1 settings service.
///
/// A missing value leaves that preference unchanged. `run_at_login` is only a
/// persisted preference here; this service never registers an OS login item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SettingsChanges {
    /// New deterministic cleanup mode.
    pub cleanup_mode: Option<CleanupMode>,
    /// Whether deterministic cleanup removes filler words.
    pub remove_fillers: Option<bool>,
    /// Whether the user prefers the application to run at sign-in.
    pub run_at_login: Option<bool>,
}

/// Result of an atomic replacement operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsSaveOutcome {
    /// The writer completed its atomic replacement and directory sync.
    Saved,
    /// The atomic replacement is visible, but parent-directory durability is
    /// unconfirmed. The live snapshot follows the visible replacement.
    DurabilityUncertain,
}

/// How a service instance obtained its starting snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsServiceOpenStatus {
    /// A missing file produced in-memory defaults; opening did not create it.
    FirstRun,
    /// Strict schema-v1 decoding loaded an existing settings file.
    Loaded,
    /// Existing vocabulary entries need an explicit matching-policy review.
    VocabularyReviewRequired,
}

/// Redacted errors from settings and vocabulary transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsServiceError {
    /// Explicit-path load, save, or reset failed.
    Storage(SettingsStorageError),
    /// A candidate violated a schema-v1 settings rule.
    InvalidSettings(SettingsError),
    /// The existing core vocabulary CRUD operation rejected a candidate.
    Vocabulary(VocabularyManagementError),
    /// Vocabulary review could not validate the current collection.
    VocabularyMigration(VocabularyMigrationError),
    /// The current nonempty schema-v1 vocabulary has not been reviewed in this
    /// service instance. The readiness value contains indices and policy IDs,
    /// never vocabulary phrases.
    VocabularyReviewRequired(VocabularyMigrationReadiness),
}

impl Display for SettingsServiceError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "settings storage failed: {error}"),
            Self::InvalidSettings(error) => write!(formatter, "invalid settings: {error}"),
            Self::Vocabulary(error) => write!(formatter, "invalid vocabulary change: {error}"),
            Self::VocabularyMigration(error) => {
                write!(formatter, "vocabulary review failed: {error}")
            }
            Self::VocabularyReviewRequired(_) => {
                formatter.write_str("existing vocabulary requires explicit policy review")
            }
        }
    }
}

impl Error for SettingsServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::InvalidSettings(error) => Some(error),
            Self::Vocabulary(error) => Some(error),
            Self::VocabularyMigration(error) => Some(error),
            Self::VocabularyReviewRequired(_) => None,
        }
    }
}

/// A reusable transactional service over one explicitly supplied settings path.
///
/// Each mutation is applied to a candidate snapshot, validated, then written
/// atomically before the live snapshot changes. A pre-replacement save failure
/// leaves the previous in-memory snapshot and persisted file intact. If the
/// storage layer reports that rename succeeded but parent-directory sync did
/// not, the service reports [`SettingsSaveOutcome::DurabilityUncertain`] and
/// updates its in-memory snapshot to match the replacement already visible on
/// disk.
pub struct SettingsService {
    path: PathBuf,
    settings: Settings,
    open_status: SettingsServiceOpenStatus,
    vocabulary_reviewed: bool,
}

impl SettingsService {
    /// Opens exactly `path`, loading a strict schema-v1 snapshot or returning
    /// validated defaults in memory when the file is absent.
    ///
    /// A nonempty existing vocabulary is always marked review-required because
    /// schema v1 has no trusted matching-policy provenance. If strict loading
    /// rejects only a Unicode 18 vocabulary conflict, the service performs a
    /// bounded read-only decode so callers can inspect and explicitly repair
    /// the original entries. No open path creates or rewrites a file.
    ///
    /// # Errors
    ///
    /// Returns a redacted storage error for an unsafe, unreadable, corrupt, or
    /// unsupported settings file, or an invalid default configuration.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, SettingsServiceError> {
        let path = path.as_ref().to_path_buf();
        let (settings, open_status, vocabulary_reviewed) = match initialize_settings(&path) {
            Ok(SettingsInitialization::FirstRun(settings)) => {
                (settings, SettingsServiceOpenStatus::FirstRun, true)
            }
            Ok(SettingsInitialization::Loaded(settings)) => {
                if settings.vocabulary.is_empty() {
                    (settings, SettingsServiceOpenStatus::Loaded, true)
                } else {
                    (
                        settings,
                        SettingsServiceOpenStatus::VocabularyReviewRequired,
                        false,
                    )
                }
            }
            Err(
                error @ SettingsStorageError::InvalidSettings(
                    SettingsError::DuplicateVocabularySource { .. },
                ),
            ) => match load_settings_for_vocabulary_review(&path) {
                Ok(Some(settings)) => (
                    settings,
                    SettingsServiceOpenStatus::VocabularyReviewRequired,
                    false,
                ),
                Ok(None) => return Err(SettingsServiceError::Storage(error)),
                Err(review_error) => {
                    return Err(SettingsServiceError::Storage(review_error));
                }
            },
            Err(error) => return Err(SettingsServiceError::Storage(error)),
        };

        Ok(Self {
            path,
            settings,
            open_status,
            vocabulary_reviewed,
        })
    }

    /// Returns the current settings snapshot. Mutations must use this service's
    /// transactional methods so failed saves cannot leak candidate state.
    pub fn current(&self) -> &Settings {
        &self.settings
    }

    /// Returns the current ordered vocabulary without copying or logging it.
    pub fn vocabulary(&self) -> &[VocabularyEntry] {
        &self.settings.vocabulary
    }

    /// Returns the service's first-run, loaded, or review-required status.
    pub const fn open_status(&self) -> SettingsServiceOpenStatus {
        self.open_status
    }

    /// Assesses the current ordered entries without assuming source-policy
    /// provenance from schema version or file contents.
    pub fn vocabulary_readiness(
        &self,
    ) -> Result<VocabularyMigrationReadiness, SettingsServiceError> {
        preflight_vocabulary_compatibility(&self.settings, VocabularySourcePolicy::Unknown)
            .map_err(SettingsServiceError::VocabularyMigration)
    }

    /// Confirms that the caller has explicitly reviewed the current vocabulary.
    ///
    /// This acknowledgment applies only to this in-memory service instance and
    /// does not assert source-policy provenance or write a marker. Any later
    /// candidate still must pass current Unicode 18 validation. Reopening the
    /// file requires review again because schema v1 stores no provenance.
    pub fn confirm_current_vocabulary_review(
        &mut self,
    ) -> Result<VocabularyMigrationReadiness, SettingsServiceError> {
        let readiness = self.vocabulary_readiness()?;
        self.vocabulary_reviewed = true;
        Ok(readiness)
    }

    /// Applies supported preference changes transactionally.
    pub fn apply_changes(
        &mut self,
        changes: SettingsChanges,
    ) -> Result<SettingsSaveOutcome, SettingsServiceError> {
        self.ensure_vocabulary_reviewed()?;
        let mut candidate = self.settings.clone();
        if let Some(mode) = changes.cleanup_mode {
            candidate.cleanup.mode = mode;
        }
        if let Some(remove_fillers) = changes.remove_fillers {
            candidate.cleanup.remove_fillers = remove_fillers;
        }
        if let Some(run_at_login) = changes.run_at_login {
            candidate.startup = StartupSettings { run_at_login };
        }
        self.commit_candidate(candidate)
    }

    /// Adds one stable-ID entry and persists it as one transaction.
    pub fn add_vocabulary(
        &mut self,
        entry: VocabularyEntry,
    ) -> Result<SettingsSaveOutcome, SettingsServiceError> {
        self.ensure_vocabulary_reviewed()?;
        let mut candidate = self.settings.clone();
        add_vocabulary_entry(&mut candidate.vocabulary, entry)
            .map_err(SettingsServiceError::Vocabulary)?;
        self.commit_candidate(candidate)
    }

    /// Edits the spoken and written text for a stable ID, retaining its ID and
    /// ordered position, then persists it as one transaction.
    pub fn edit_vocabulary(
        &mut self,
        id: &str,
        spoken: String,
        written: String,
    ) -> Result<SettingsSaveOutcome, SettingsServiceError> {
        self.ensure_vocabulary_reviewed()?;
        let mut candidate = self.settings.clone();
        edit_vocabulary_entry(&mut candidate.vocabulary, id, spoken, written)
            .map_err(SettingsServiceError::Vocabulary)?;
        self.commit_candidate(candidate)
    }

    /// Deletes exactly one stable ID and persists it as one transaction.
    pub fn delete_vocabulary(
        &mut self,
        id: &str,
    ) -> Result<SettingsSaveOutcome, SettingsServiceError> {
        self.ensure_vocabulary_reviewed()?;
        let mut candidate = self.settings.clone();
        delete_vocabulary_entry(&mut candidate.vocabulary, id)
            .map_err(SettingsServiceError::Vocabulary)?;
        self.commit_candidate(candidate)
    }

    /// Resets an existing file only when `confirmed` is explicitly true.
    ///
    /// Reset delegates to the storage module's existing reset semantics and
    /// never creates a missing file. The in-memory snapshot changes only after
    /// the atomic replacement succeeds or is visible with uncertain directory
    /// durability.
    pub fn reset(&mut self, confirmed: bool) -> Result<SettingsSaveOutcome, SettingsServiceError> {
        match reset_settings_to_defaults(&self.path, confirmed) {
            Ok(()) => {
                self.settings = Settings::default();
                self.open_status = SettingsServiceOpenStatus::Loaded;
                self.vocabulary_reviewed = true;
                Ok(SettingsSaveOutcome::Saved)
            }
            Err(SettingsStorageError::ReplacementDurabilityUncertain) => {
                self.settings = Settings::default();
                self.open_status = SettingsServiceOpenStatus::Loaded;
                self.vocabulary_reviewed = true;
                Ok(SettingsSaveOutcome::DurabilityUncertain)
            }
            Err(error) => Err(SettingsServiceError::Storage(error)),
        }
    }

    fn ensure_vocabulary_reviewed(&self) -> Result<(), SettingsServiceError> {
        if self.settings.vocabulary.is_empty() || self.vocabulary_reviewed {
            return Ok(());
        }
        Err(SettingsServiceError::VocabularyReviewRequired(
            self.vocabulary_readiness()?,
        ))
    }

    fn commit_candidate(
        &mut self,
        candidate: Settings,
    ) -> Result<SettingsSaveOutcome, SettingsServiceError> {
        self.commit_candidate_with(candidate, save_settings_atomic)
    }

    fn commit_candidate_with(
        &mut self,
        candidate: Settings,
        persist: impl FnOnce(&Path, &Settings) -> Result<(), SettingsStorageError>,
    ) -> Result<SettingsSaveOutcome, SettingsServiceError> {
        candidate
            .validate()
            .map_err(SettingsServiceError::InvalidSettings)?;

        let outcome = match persist(&self.path, &candidate) {
            Ok(()) => SettingsSaveOutcome::Saved,
            Err(SettingsStorageError::ReplacementDurabilityUncertain) => {
                SettingsSaveOutcome::DurabilityUncertain
            }
            Err(error) => return Err(SettingsServiceError::Storage(error)),
        };

        // The old snapshot remains live until the writer confirms that the new
        // file is atomically visible. The uncertain-durability variant also
        // means the rename already succeeded, so the candidate is authoritative
        // for this process even though a crash could lose the directory entry.
        self.settings = candidate;
        self.open_status = SettingsServiceOpenStatus::Loaded;
        if self.settings.vocabulary.is_empty() {
            self.vocabulary_reviewed = true;
        }
        Ok(outcome)
    }
}

impl From<SettingsJsonError> for SettingsServiceError {
    fn from(error: SettingsJsonError) -> Self {
        match error {
            SettingsJsonError::InvalidSettings(error) => Self::InvalidSettings(error),
            SettingsJsonError::InvalidDocument { .. } | SettingsJsonError::SerializationFailed => {
                Self::Storage(SettingsStorageError::SettingsDecodingFailed)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CleanupMode, SettingsChanges, SettingsService, SettingsServiceError};
    use crate::storage::save_settings_atomic_with_hooks;
    use std::fs::{self, File};
    use std::io;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn injected_storage_write_failure_preserves_live_snapshot_and_prior_file() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "outflow-settings-service-fault-{}-{timestamp}",
            std::process::id()
        ));
        fs::create_dir(&directory).expect("create isolated settings test directory");
        let path = directory.join("settings.json");
        let mut service = SettingsService::open(&path).expect("open first-run service");
        service
            .apply_changes(SettingsChanges {
                cleanup_mode: Some(CleanupMode::Clean),
                remove_fillers: Some(true),
                run_at_login: Some(false),
            })
            .expect("persist baseline settings");
        let prior_snapshot = service.current().clone();
        let prior_file = fs::read(&path).expect("capture prior settings file");
        let mut candidate = prior_snapshot.clone();
        candidate.cleanup.mode = CleanupMode::Verbatim;
        candidate.cleanup.remove_fillers = false;
        candidate.startup.run_at_login = true;

        let result = service.commit_candidate_with(candidate, |path, settings| {
            save_settings_atomic_with_hooks(
                path,
                settings,
                |_, _| Err(io::Error::other("synthetic injected write failure")),
                File::sync_all,
                || Ok(()),
                File::sync_all,
            )
        });

        assert!(matches!(result, Err(SettingsServiceError::Storage(_))));
        assert_eq!(service.current(), &prior_snapshot);
        assert_eq!(
            fs::read(&path).expect("read preserved settings"),
            prior_file
        );
        assert_eq!(
            crate::load_settings(&path).expect("reload preserved settings"),
            Some(prior_snapshot)
        );

        fs::remove_dir_all(directory).expect("remove isolated settings test directory");
    }
}
