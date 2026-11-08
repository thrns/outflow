//! Typed model, JSON codec, and validation for Outflow's local settings schema v1.
//!
//! This crate owns configuration values, an in-memory schema v1 JSON codec,
//! read-only settings loading and initialization, a transactional settings service,
//! user-directed settings reset, and an explicit-path atomic file writer. It performs no migration,
//! networking, or operating system integration. Its reader accepts schema v1
//! only, rejects unknown fields, and validates decoded values. Schema v0 is
//! unsupported because no historical v0 shape exists; every other unknown
//! version is also rejected until an explicit, tested migration path is defined.
//! Rejected settings are never replaced with defaults unless a caller explicitly
//! confirms the separate reset operation. The data model follows C-08 and
//! `docs/ARCHITECTURE.md` §7.

use outflow_core::vocabulary::{
    VocabularyCollectionError, VocabularyEntryRef, validate_vocabulary_collection,
};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::Path;

/// Maps persisted cleanup preferences to the core text pipeline's filler flag.
pub mod cleanup_policy;
/// Transactional in-memory settings management over the atomic storage API.
pub mod service;
pub mod storage;
/// In-memory operations for stable-ID vocabulary entries.
pub mod vocabulary_crud;
pub mod vocabulary_policy_marker;
pub mod vocabulary_preflight;

pub use cleanup_policy::process_transcript_with_filler_preference;
pub use service::{
    SettingsChanges, SettingsSaveOutcome, SettingsService, SettingsServiceError,
    SettingsServiceOpenStatus,
};
pub use vocabulary_crud::{
    VocabularyManagementError, add_vocabulary_entry, delete_vocabulary_entry, edit_vocabulary_entry,
};

pub use storage::{
    MAX_SETTINGS_FILE_BYTES, SettingsStorageError, SettingsStorageOperation, load_settings,
    reset_settings_to_defaults, save_settings_atomic,
};
pub use vocabulary_policy_marker::{
    UNICODE18_CANONICAL_CASELESS_V1_IDENTIFIER, VocabularyPolicyMarker,
    VocabularyPolicyMarkerInspection, inspect_vocabulary_policy_marker,
};
pub use vocabulary_preflight::{
    VocabularyField, VocabularyMatchingPolicyId, VocabularyMigrationConflict,
    VocabularyMigrationConflictKind, VocabularyMigrationError, VocabularyMigrationReadiness,
    VocabularySourcePolicy, VocabularyValidationError, preflight_vocabulary_compatibility,
};

/// Whether settings were loaded from an existing file or are available as
/// validated in-memory defaults for a first run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsInitialization {
    /// A valid existing schema v1 configuration was loaded.
    Loaded(Settings),
    /// No configuration file existed; defaults are available in memory only.
    FirstRun(Settings),
}

/// Loads settings from `path`, or returns validated defaults when the file is
/// missing.
///
/// Existing files are read and validated by [`load_settings`]. Missing files
/// produce [`SettingsInitialization::FirstRun`] without creating or modifying
/// a file. Invalid or unreadable files return their existing structured,
/// redacted storage error; they are never replaced with defaults or repaired.
///
/// # Errors
///
/// Returns the same redacted errors as [`load_settings`] for unsafe paths,
/// filesystem failures, oversized or non-UTF-8 files, invalid JSON, unsupported
/// schema versions, or invalid settings. It also returns
/// [`SettingsStorageError::InvalidSettings`] if the compiled-in defaults ever
/// fail validation.
pub fn initialize_settings(path: &Path) -> Result<SettingsInitialization, SettingsStorageError> {
    match load_settings(path)? {
        Some(settings) => Ok(SettingsInitialization::Loaded(settings)),
        None => {
            let settings = Settings::default();
            settings
                .validate()
                .map_err(SettingsStorageError::InvalidSettings)?;
            Ok(SettingsInitialization::FirstRun(settings))
        }
    }
}

/// The only settings schema version currently supported by Outflow.
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

/// Maximum number of user-defined vocabulary entries.
pub const MAX_VOCABULARY_ENTRIES: usize = 500;

/// Maximum number of Unicode scalar values in each vocabulary phrase.
pub const MAX_VOCABULARY_PHRASE_CHARS: usize = 128;

/// The only language code supported by the MVP.
pub const MVP_LANGUAGE: &str = "en";

/// JSON marker for selecting the operating system's default input device.
const SYSTEM_DEFAULT_INPUT_DEVICE: &str = "system-default";

/// All user-configurable local settings represented by schema v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct Settings {
    /// Version of the settings shape. Readers must reject unknown versions.
    pub schema_version: u32,
    /// Language code kept as a string so future readers can explicitly reject
    /// unknown values instead of losing them during deserialization.
    pub language: String,
    /// Recording shortcuts and audio input selection.
    pub recording: RecordingSettings,
    /// Deterministic output cleanup preferences.
    pub cleanup: CleanupSettings,
    /// Local startup preference; this model does not register a login item.
    pub startup: StartupSettings,
    /// Update policy; the MVP only permits user-initiated checks.
    pub updates: UpdateSettings,
    /// Explicit vocabulary entries only; transcript-derived data is not part
    /// of this schema.
    pub vocabulary: Vec<VocabularyEntry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            language: MVP_LANGUAGE.to_owned(),
            recording: RecordingSettings::default(),
            cleanup: CleanupSettings::default(),
            startup: StartupSettings::default(),
            updates: UpdateSettings::default(),
            vocabulary: Vec::new(),
        }
    }
}

impl Settings {
    /// Checks that this configuration uses supported MVP values.
    ///
    /// Error messages contain field names and entry positions only. In
    /// particular, they never include vocabulary phrases or shortcut values.
    /// This check does not parse, migrate, save, or apply settings.
    pub fn validate(&self) -> Result<(), SettingsError> {
        self.validate_non_vocabulary_fields()?;
        validate_vocabulary_ids(&self.vocabulary)?;
        validate_vocabulary(&self.vocabulary)
    }

    fn validate_non_vocabulary_fields(&self) -> Result<(), SettingsError> {
        if self.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(SettingsError::UnsupportedSchemaVersion {
                found: self.schema_version,
            });
        }
        if self.language != MVP_LANGUAGE {
            return Err(SettingsError::UnsupportedLanguage);
        }
        if !self.updates.user_initiated_only {
            return Err(SettingsError::AutomaticUpdateChecksNotAllowed);
        }
        validate_shortcuts(&self.recording)
    }

    /// Encodes valid schema v1 settings as compact JSON.
    ///
    /// The value is validated before encoding so an invalid in-memory value
    /// cannot be emitted as a schema v1 document. No file or network access is
    /// performed, and errors never contain settings values.
    pub fn to_json(&self) -> Result<String, SettingsJsonError> {
        self.validate()
            .map_err(SettingsJsonError::InvalidSettings)?;
        serde_json::to_string(self).map_err(|_| SettingsJsonError::SerializationFailed)
    }

    /// Decodes a schema v1 JSON document and validates every settings value.
    ///
    /// Missing required fields, unknown fields, malformed JSON, invalid field
    /// types, and unsupported schema versions are rejected. The reader never
    /// migrates or substitutes defaults. It checks the version header before
    /// decoding the v1 shape, so an unsupported document does not need to
    /// contain fields defined only by this version. The returned error includes
    /// no input text, vocabulary phrase, shortcut, or other private value.
    pub fn from_json(json: &str) -> Result<Self, SettingsJsonError> {
        let header: SettingsSchemaVersionHeader =
            serde_json::from_str(json).map_err(|error| SettingsJsonError::InvalidDocument {
                line: error.line(),
                column: error.column(),
            })?;
        if header.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(SettingsJsonError::InvalidSettings(
                SettingsError::UnsupportedSchemaVersion {
                    found: header.schema_version,
                },
            ));
        }

        let wire: SettingsWire =
            serde_json::from_str(json).map_err(|error| SettingsJsonError::InvalidDocument {
                line: error.line(),
                column: error.column(),
            })?;

        Self::from_wire(wire).map_err(SettingsJsonError::InvalidSettings)
    }

    /// Decodes a schema-v1 document for read-only vocabulary conflict review.
    ///
    /// Unlike [`Self::from_json`], this method accepts a collection whose
    /// spoken phrases conflict under the current Unicode 18 policy. It still
    /// rejects malformed documents, unsupported versions, invalid settings,
    /// oversized collections, and invalid phrases. Only the settings service
    /// uses it after the strict loader reports a vocabulary conflict, so the
    /// original values can be shown for explicit review without rewriting
    /// legacy data. Callers must not treat this as a validated writable
    /// snapshot.
    pub(crate) fn from_json_for_vocabulary_review(json: &str) -> Result<Self, SettingsJsonError> {
        let header: SettingsSchemaVersionHeader =
            serde_json::from_str(json).map_err(|error| SettingsJsonError::InvalidDocument {
                line: error.line(),
                column: error.column(),
            })?;
        if header.schema_version != CURRENT_SCHEMA_VERSION {
            return Err(SettingsJsonError::InvalidSettings(
                SettingsError::UnsupportedSchemaVersion {
                    found: header.schema_version,
                },
            ));
        }
        let wire: SettingsWire =
            serde_json::from_str(json).map_err(|error| SettingsJsonError::InvalidDocument {
                line: error.line(),
                column: error.column(),
            })?;
        Self::from_wire_for_vocabulary_review(wire).map_err(SettingsJsonError::InvalidSettings)
    }

    fn from_wire(wire: SettingsWire) -> Result<Self, SettingsError> {
        let settings = Self::from_wire_fields(wire);
        settings.validate()?;
        Ok(settings)
    }

    fn from_wire_for_vocabulary_review(wire: SettingsWire) -> Result<Self, SettingsError> {
        let settings = Self::from_wire_fields(wire);
        settings.validate_non_vocabulary_fields()?;
        // Source-phrase conflicts can be reviewed by entry index. Duplicate
        // stable IDs cannot: edit/delete-by-ID would be ambiguous, so this
        // path remains fail-closed and leaves the source file untouched.
        validate_vocabulary_ids(&settings.vocabulary)?;
        validate_vocabulary_allowing_conflicts(&settings.vocabulary)?;
        Ok(settings)
    }

    fn from_wire_fields(wire: SettingsWire) -> Self {
        Self {
            schema_version: wire.schema_version,
            language: wire.language,
            recording: wire.recording,
            cleanup: wire.cleanup,
            startup: wire.startup,
            updates: wire.updates,
            vocabulary: wire.vocabulary,
        }
    }
}

impl<'de> Deserialize<'de> for Settings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SettingsWire::deserialize(deserializer)?;
        Self::from_wire(wire).map_err(D::Error::custom)
    }
}

// Unknown fields are intentionally ignored here: the header exists only to
// classify a syntactically valid unsupported version before requiring v1-only
// fields. A supported v1 document is still fully decoded by `SettingsWire`.
#[derive(Deserialize)]
struct SettingsSchemaVersionHeader {
    schema_version: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct SettingsWire {
    schema_version: u32,
    language: String,
    recording: RecordingSettings,
    cleanup: CleanupSettings,
    startup: StartupSettings,
    updates: UpdateSettings,
    vocabulary: Vec<VocabularyEntry>,
}

/// Recording-related values. `None` for either shortcut means unconfigured;
/// `None` for `input_device` means use the system default input device.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct RecordingSettings {
    /// Platform-specific push-to-talk shortcut, if configured.
    pub push_to_talk: Option<String>,
    /// Platform-specific toggle shortcut, if configured.
    pub toggle: Option<String>,
    /// Explicit local device key, or `None` for the system default.
    #[serde(serialize_with = "serialize_input_device")]
    pub input_device: Option<String>,
}

impl<'de> Deserialize<'de> for RecordingSettings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let RecordingSettingsWire {
            push_to_talk,
            toggle,
            input_device,
        } = RecordingSettingsWire::deserialize(deserializer)?;
        let push_to_talk = match push_to_talk {
            RequiredOption::Missing => return Err(D::Error::missing_field("push_to_talk")),
            RequiredOption::Present(value) => value,
        };
        let toggle = match toggle {
            RequiredOption::Missing => return Err(D::Error::missing_field("toggle")),
            RequiredOption::Present(value) => value,
        };

        let input_device = if input_device == SYSTEM_DEFAULT_INPUT_DEVICE {
            None
        } else {
            Some(input_device)
        };

        Ok(Self {
            push_to_talk,
            toggle,
            input_device,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
struct RecordingSettingsWire {
    #[serde(default)]
    push_to_talk: RequiredOption<String>,
    #[serde(default)]
    toggle: RequiredOption<String>,
    input_device: String,
}

/// Makes an optional value required as a JSON key while still allowing null.
/// Serde normally treats a missing `Option<T>` field as `None`; schema v1
/// requires both shortcut keys to be present so missing configuration is not
/// silently invented during decoding.
#[derive(Default)]
enum RequiredOption<T> {
    #[default]
    Missing,
    Present(Option<T>),
}

impl<'de, T> Deserialize<'de> for RequiredOption<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<T>::deserialize(deserializer).map(Self::Present)
    }
}

fn serialize_input_device<S>(
    input_device: &Option<String>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match input_device {
        None => serializer.serialize_str(SYSTEM_DEFAULT_INPUT_DEVICE),
        Some(device) => serializer.serialize_str(device),
    }
}

/// Output cleanup preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct CleanupSettings {
    /// Whether to produce readable cleaned text or preserve the recognized
    /// wording as closely as possible.
    pub mode: CleanupMode,
    /// Whether deterministic cleanup removes isolated filler words.
    pub remove_fillers: bool,
}

impl Default for CleanupSettings {
    fn default() -> Self {
        Self {
            mode: CleanupMode::Clean,
            remove_fillers: true,
        }
    }
}

/// Supported deterministic text output modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupMode {
    /// Apply the configured deterministic cleanup rules.
    Clean,
    /// Preserve recognized wording without cleanup transformations.
    Verbatim,
}

/// Startup preference values.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct StartupSettings {
    /// Whether the user wants Outflow to run when they sign in.
    pub run_at_login: bool,
}

/// Update-check policy values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct UpdateSettings {
    /// Must remain true: the MVP checks for updates only after an explicit user
    /// action. This schema does not perform any update check.
    pub user_initiated_only: bool,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self {
            user_initiated_only: true,
        }
    }
}

/// An explicitly configured spoken phrase and its written replacement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct VocabularyEntry {
    /// Stable local identifier reserved for later vocabulary editing flows.
    pub id: String,
    /// Phrase the recognizer may hear; used as the unique source phrase.
    pub spoken: String,
    /// Phrase to write when the source phrase matches.
    pub written: String,
}

/// The two platform-specific recording shortcuts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutKind {
    /// Push-to-talk shortcut.
    PushToTalk,
    /// Toggle shortcut.
    Toggle,
}

impl Display for ShortcutKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::PushToTalk => formatter.write_str("push-to-talk"),
            Self::Toggle => formatter.write_str("toggle"),
        }
    }
}

/// Which vocabulary phrase failed validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocabularyPhraseField {
    /// Spoken source phrase.
    Spoken,
    /// Written replacement phrase.
    Written,
}

impl Display for VocabularyPhraseField {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spoken => formatter.write_str("spoken source"),
            Self::Written => formatter.write_str("written replacement"),
        }
    }
}

/// A settings validation failure, without user-supplied phrase or shortcut
/// values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsError {
    /// The stored schema version is not supported by this crate.
    UnsupportedSchemaVersion { found: u32 },
    /// The MVP accepts only the English language code `en`.
    UnsupportedLanguage,
    /// Automatic update checks are forbidden by the MVP policy.
    AutomaticUpdateChecksNotAllowed,
    /// The vocabulary collection exceeds the supported entry cap.
    TooManyVocabularyEntries { max_entries: usize },
    /// A configured shortcut is empty or whitespace-only.
    EmptyShortcut { shortcut: ShortcutKind },
    /// Push-to-talk and toggle shortcuts identify the same key combination.
    DuplicateShortcuts,
    /// One required vocabulary phrase is empty or whitespace-only.
    EmptyVocabularyPhrase {
        entry_index: usize,
        field: VocabularyPhraseField,
    },
    /// One vocabulary phrase exceeds the Unicode scalar value limit.
    VocabularyPhraseTooLong {
        entry_index: usize,
        field: VocabularyPhraseField,
        max_chars: usize,
    },
    /// Two vocabulary entries have the same Unicode 18 canonical-caseless source key.
    DuplicateVocabularySource {
        first_index: usize,
        duplicate_index: usize,
    },
    /// Two vocabulary entries share a stable ID, making ID-based edits ambiguous.
    DuplicateVocabularyId {
        first_index: usize,
        duplicate_index: usize,
    },
}

impl Display for SettingsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchemaVersion { found } => write!(
                formatter,
                "unsupported settings schema version {found}; expected {CURRENT_SCHEMA_VERSION}"
            ),
            Self::UnsupportedLanguage => {
                formatter.write_str("unsupported language; the MVP supports English only")
            }
            Self::AutomaticUpdateChecksNotAllowed => {
                formatter.write_str("update checks must be user-initiated")
            }
            Self::TooManyVocabularyEntries { max_entries } => write!(
                formatter,
                "vocabulary exceeds the limit of {max_entries} entries"
            ),
            Self::EmptyShortcut { shortcut } => {
                write!(formatter, "configured {shortcut} shortcut cannot be empty")
            }
            Self::DuplicateShortcuts => {
                formatter.write_str("push-to-talk and toggle shortcuts must be different")
            }
            Self::EmptyVocabularyPhrase { entry_index, field } => write!(
                formatter,
                "vocabulary entry {entry_index} has an empty or whitespace-only {field} phrase"
            ),
            Self::VocabularyPhraseTooLong {
                entry_index,
                field,
                max_chars,
            } => write!(
                formatter,
                "vocabulary entry {entry_index} {field} phrase exceeds {max_chars} Unicode scalar values"
            ),
            Self::DuplicateVocabularySource {
                first_index,
                duplicate_index,
            } => write!(
                formatter,
                "vocabulary source phrases at entries {first_index} and {duplicate_index} conflict under Unicode 18 canonical-caseless matching"
            ),
            Self::DuplicateVocabularyId {
                first_index,
                duplicate_index,
            } => write!(
                formatter,
                "vocabulary identifiers at entries {first_index} and {duplicate_index} must be unique"
            ),
        }
    }
}

impl Error for SettingsError {}

/// A redacted error returned by the schema v1 JSON codec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsJsonError {
    /// The JSON document is malformed or does not have the required schema v1
    /// shape. Only the parser location is retained.
    InvalidDocument { line: usize, column: usize },
    /// The document decoded but violated a supported settings invariant.
    InvalidSettings(SettingsError),
    /// JSON encoding failed. The underlying serializer message is omitted so
    /// the error cannot accidentally expose private settings values.
    SerializationFailed,
}

impl Display for SettingsJsonError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocument { line, column } => write!(
                formatter,
                "invalid settings JSON document at line {line}, column {column}"
            ),
            Self::InvalidSettings(error) => write!(formatter, "invalid settings: {error}"),
            Self::SerializationFailed => formatter.write_str("settings JSON serialization failed"),
        }
    }
}

impl Error for SettingsJsonError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidSettings(error) => Some(error),
            Self::InvalidDocument { .. } | Self::SerializationFailed => None,
        }
    }
}

fn validate_shortcuts(recording: &RecordingSettings) -> Result<(), SettingsError> {
    for (shortcut, kind) in [
        (&recording.push_to_talk, ShortcutKind::PushToTalk),
        (&recording.toggle, ShortcutKind::Toggle),
    ] {
        if shortcut
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(SettingsError::EmptyShortcut { shortcut: kind });
        }
    }

    if let (Some(push_to_talk), Some(toggle)) = (&recording.push_to_talk, &recording.toggle) {
        // Shortcut names are supplied by a later platform adapter. Comparing
        // trimmed lowercase forms catches obvious duplicates without claiming
        // to canonicalize OS-specific shortcut syntax.
        if push_to_talk.trim().to_lowercase() == toggle.trim().to_lowercase() {
            return Err(SettingsError::DuplicateShortcuts);
        }
    }

    Ok(())
}

fn validate_vocabulary(entries: &[VocabularyEntry]) -> Result<(), SettingsError> {
    validate_vocabulary_collection(&borrow_vocabulary(entries)).map_err(map_vocabulary_error)
}

fn validate_vocabulary_ids(entries: &[VocabularyEntry]) -> Result<(), SettingsError> {
    let mut first_index_by_id = HashMap::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if let Some(first_index) = first_index_by_id.insert(entry.id.as_str(), index) {
            return Err(SettingsError::DuplicateVocabularyId {
                first_index,
                duplicate_index: index,
            });
        }
    }
    Ok(())
}

/// Lets the service inspect only the conflicting legacy data needed for a
/// manual decision. All size and per-entry checks remain the core authority.
fn validate_vocabulary_allowing_conflicts(
    entries: &[VocabularyEntry],
) -> Result<(), SettingsError> {
    match validate_vocabulary_collection(&borrow_vocabulary(entries)) {
        Ok(()) | Err(VocabularyCollectionError::DuplicateSpoken { .. }) => Ok(()),
        Err(error) => Err(map_vocabulary_error(error)),
    }
}

fn borrow_vocabulary(entries: &[VocabularyEntry]) -> Vec<VocabularyEntryRef<'_>> {
    entries
        .iter()
        .map(|entry| VocabularyEntryRef {
            spoken: &entry.spoken,
            written: &entry.written,
        })
        .collect()
}

fn map_vocabulary_error(error: VocabularyCollectionError) -> SettingsError {
    match error {
        VocabularyCollectionError::TooManyEntries { maximum, .. } => {
            SettingsError::TooManyVocabularyEntries {
                max_entries: maximum,
            }
        }
        VocabularyCollectionError::InvalidEntry { index, source } => match source {
            outflow_core::vocabulary::VocabularyValidationError::Empty(field) => {
                SettingsError::EmptyVocabularyPhrase {
                    entry_index: index,
                    field: match field {
                        outflow_core::vocabulary::VocabularyField::Spoken => {
                            VocabularyPhraseField::Spoken
                        }
                        outflow_core::vocabulary::VocabularyField::Written => {
                            VocabularyPhraseField::Written
                        }
                    },
                }
            }
            outflow_core::vocabulary::VocabularyValidationError::TooLong(field) => {
                SettingsError::VocabularyPhraseTooLong {
                    entry_index: index,
                    field: match field {
                        outflow_core::vocabulary::VocabularyField::Spoken => {
                            VocabularyPhraseField::Spoken
                        }
                        outflow_core::vocabulary::VocabularyField::Written => {
                            VocabularyPhraseField::Written
                        }
                    },
                    max_chars: MAX_VOCABULARY_PHRASE_CHARS,
                }
            }
        },
        VocabularyCollectionError::DuplicateSpoken {
            first_index,
            duplicate_index,
        } => SettingsError::DuplicateVocabularySource {
            first_index,
            duplicate_index,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CURRENT_SCHEMA_VERSION, CleanupMode, MAX_VOCABULARY_ENTRIES, MAX_VOCABULARY_PHRASE_CHARS,
        MVP_LANGUAGE, RecordingSettings, Settings, SettingsError, SettingsJsonError, ShortcutKind,
        VocabularyEntry, VocabularyPhraseField,
    };

    fn vocabulary_entry(
        id: usize,
        spoken: impl Into<String>,
        written: impl Into<String>,
    ) -> VocabularyEntry {
        VocabularyEntry {
            id: id.to_string(),
            spoken: spoken.into(),
            written: written.into(),
        }
    }

    #[test]
    fn default_schema_version_is_one() {
        assert_eq!(Settings::default().schema_version, 1);
        assert_eq!(CURRENT_SCHEMA_VERSION, 1);
    }

    #[test]
    fn default_language_is_english() {
        assert_eq!(Settings::default().language, MVP_LANGUAGE);
    }

    #[test]
    fn default_cleanup_mode_is_clean() {
        assert_eq!(Settings::default().cleanup.mode, CleanupMode::Clean);
        assert!(Settings::default().cleanup.remove_fillers);
    }

    #[test]
    fn launch_at_login_defaults_to_false() {
        assert!(!Settings::default().startup.run_at_login);
    }

    #[test]
    fn recording_shortcuts_are_unconfigured_and_input_uses_system_default() {
        assert_eq!(Settings::default().recording, RecordingSettings::default());
        assert!(Settings::default().recording.push_to_talk.is_none());
        assert!(Settings::default().recording.toggle.is_none());
        assert!(Settings::default().recording.input_device.is_none());
    }

    #[test]
    fn default_settings_pass_validation() {
        assert!(Settings::default().validate().is_ok());
    }

    #[test]
    fn unsupported_schema_versions_are_rejected_explicitly() {
        for version in [0, 2, 37, u32::MAX] {
            let settings = Settings {
                schema_version: version,
                ..Settings::default()
            };

            assert_eq!(
                settings.validate(),
                Err(SettingsError::UnsupportedSchemaVersion { found: version })
            );
        }
    }

    #[test]
    fn unsupported_languages_are_rejected_without_echoing_the_value() {
        let settings = Settings {
            language: "fr".to_owned(),
            ..Settings::default()
        };

        let error = settings.validate().expect_err("French is outside the MVP");
        assert_eq!(error, SettingsError::UnsupportedLanguage);
        assert!(!error.to_string().contains("fr"));
    }

    #[test]
    fn automatic_update_checks_are_rejected() {
        let mut settings = Settings::default();
        settings.updates.user_initiated_only = false;

        assert_eq!(
            settings.validate(),
            Err(SettingsError::AutomaticUpdateChecksNotAllowed)
        );
    }

    #[test]
    fn empty_vocabulary_source_and_replacement_phrases_are_rejected() {
        for (spoken, written, field) in [
            ("", "replacement", VocabularyPhraseField::Spoken),
            ("  \t", "replacement", VocabularyPhraseField::Spoken),
            ("source", "", VocabularyPhraseField::Written),
        ] {
            let settings = Settings {
                vocabulary: vec![vocabulary_entry(0, spoken, written)],
                ..Settings::default()
            };

            assert_eq!(
                settings.validate(),
                Err(SettingsError::EmptyVocabularyPhrase {
                    entry_index: 0,
                    field,
                })
            );
        }
    }

    #[test]
    fn duplicate_vocabulary_sources_are_rejected_ignoring_unicode_case() {
        let settings = Settings {
            vocabulary: vec![
                vocabulary_entry(0, "Über", "First replacement"),
                vocabulary_entry(1, "über", "Second replacement"),
            ],
            ..Settings::default()
        };

        assert_eq!(
            settings.validate(),
            Err(SettingsError::DuplicateVocabularySource {
                first_index: 0,
                duplicate_index: 1,
            })
        );
        assert!(
            !settings
                .validate()
                .unwrap_err()
                .to_string()
                .contains("Über")
        );
    }

    #[test]
    fn duplicate_vocabulary_ids_are_rejected_by_strict_and_review_decoders() {
        let settings = Settings {
            vocabulary: vec![
                vocabulary_entry(7, "first synthetic source", "First replacement"),
                vocabulary_entry(7, "second synthetic source", "Second replacement"),
            ],
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings)
            .expect("synthetic duplicate-ID settings serialize for decoder tests");
        let expected = SettingsJsonError::InvalidSettings(SettingsError::DuplicateVocabularyId {
            first_index: 0,
            duplicate_index: 1,
        });

        assert_eq!(Settings::from_json(&json), Err(expected.clone()));
        assert_eq!(
            Settings::from_json_for_vocabulary_review(&json),
            Err(expected)
        );
        assert!(
            !settings
                .validate()
                .expect_err("duplicate IDs are invalid")
                .to_string()
                .contains("first synthetic source")
        );
    }

    #[test]
    fn duplicate_ids_are_rejected_even_when_source_conflicts_are_reviewable() {
        let settings = Settings {
            vocabulary: vec![
                vocabulary_entry(4, "straße", "First replacement"),
                vocabulary_entry(4, "STRASSE", "Second replacement"),
            ],
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings)
            .expect("synthetic duplicate-ID source-conflict fixture serializes");

        assert_eq!(
            Settings::from_json_for_vocabulary_review(&json),
            Err(SettingsJsonError::InvalidSettings(
                SettingsError::DuplicateVocabularyId {
                    first_index: 0,
                    duplicate_index: 1,
                }
            ))
        );
    }

    #[test]
    fn schema_v1_validation_uses_the_core_unicode18_canonical_caseless_policy() {
        for (first, second) in [("straße", "STRASSE"), ("café", "cafe\u{301}")] {
            let settings = Settings {
                vocabulary: vec![
                    vocabulary_entry(0, first, "First replacement"),
                    vocabulary_entry(1, second, "Second replacement"),
                ],
                ..Settings::default()
            };

            assert_eq!(
                settings.validate(),
                Err(SettingsError::DuplicateVocabularySource {
                    first_index: 0,
                    duplicate_index: 1,
                })
            );
        }
    }

    #[test]
    fn vocabulary_entry_limit_accepts_500_and_rejects_501() {
        let mut settings = Settings {
            vocabulary: (0..MAX_VOCABULARY_ENTRIES)
                .map(|index| vocabulary_entry(index, format!("term {index}"), "replacement"))
                .collect(),
            ..Settings::default()
        };
        assert!(settings.validate().is_ok());

        settings
            .vocabulary
            .push(vocabulary_entry(500, "term 500", "replacement"));
        assert_eq!(
            settings.validate(),
            Err(SettingsError::TooManyVocabularyEntries {
                max_entries: MAX_VOCABULARY_ENTRIES,
            })
        );
    }

    #[test]
    fn vocabulary_length_counts_unicode_scalar_values() {
        let mut settings = Settings::default();
        let at_limit = "🗣".repeat(MAX_VOCABULARY_PHRASE_CHARS);
        settings
            .vocabulary
            .push(vocabulary_entry(0, &at_limit, "ok"));
        assert!(settings.validate().is_ok());

        settings.vocabulary[0].written = "é".repeat(MAX_VOCABULARY_PHRASE_CHARS + 1);
        assert_eq!(
            settings.validate(),
            Err(SettingsError::VocabularyPhraseTooLong {
                entry_index: 0,
                field: VocabularyPhraseField::Written,
                max_chars: MAX_VOCABULARY_PHRASE_CHARS,
            })
        );
    }

    #[test]
    fn empty_or_whitespace_only_configured_shortcuts_are_rejected() {
        for kind in [ShortcutKind::PushToTalk, ShortcutKind::Toggle] {
            for shortcut in [String::new(), "  \t".to_owned()] {
                let configured = Some(shortcut);
                let recording = match kind {
                    ShortcutKind::PushToTalk => RecordingSettings {
                        push_to_talk: configured,
                        ..RecordingSettings::default()
                    },
                    ShortcutKind::Toggle => RecordingSettings {
                        toggle: configured,
                        ..RecordingSettings::default()
                    },
                };
                let settings = Settings {
                    recording,
                    ..Settings::default()
                };

                assert_eq!(
                    settings.validate(),
                    Err(SettingsError::EmptyShortcut { shortcut: kind })
                );
            }
        }
    }

    #[test]
    fn duplicate_shortcuts_are_rejected_without_echoing_the_value() {
        let settings = Settings {
            recording: RecordingSettings {
                push_to_talk: Some("Ctrl+Alt+D".to_owned()),
                toggle: Some(" ctrl+alt+d ".to_owned()),
                ..RecordingSettings::default()
            },
            ..Settings::default()
        };

        let error = settings.validate().expect_err("shortcuts must be distinct");
        assert_eq!(error, SettingsError::DuplicateShortcuts);
        assert!(!error.to_string().contains("Ctrl+Alt+D"));
    }

    #[test]
    fn default_settings_serialize_successfully() {
        let json = Settings::default()
            .to_json()
            .expect("default settings should serialize");
        assert!(!json.is_empty());
    }

    #[test]
    fn default_json_contains_the_documented_schema_v1_fields() {
        let settings = Settings::default();
        let value: serde_json::Value =
            serde_json::from_str(&settings.to_json().expect("default settings serialize"))
                .expect("serializer emits valid JSON");

        let root_fields = [
            "schema_version",
            "language",
            "recording",
            "cleanup",
            "startup",
            "updates",
            "vocabulary",
        ]
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            value
                .as_object()
                .expect("root is an object")
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            root_fields
        );
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["language"], "en");
        assert!(value["recording"]["push_to_talk"].is_null());
        assert!(value["recording"]["toggle"].is_null());
        assert_eq!(value["recording"]["input_device"], "system-default");
        assert_eq!(value["cleanup"]["mode"], "clean");
        assert_eq!(value["cleanup"]["remove_fillers"], true);
        assert_eq!(value["startup"]["run_at_login"], false);
        assert_eq!(value["updates"]["user_initiated_only"], true);
        assert_eq!(value["vocabulary"], serde_json::json!([]));
    }

    #[test]
    fn default_settings_round_trip_without_changes() {
        let original = Settings::default();
        let json = original.to_json().expect("default settings serialize");
        let decoded = Settings::from_json(&json).expect("default settings decode");

        assert_eq!(decoded, original);
        assert_eq!(decoded.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn modified_valid_settings_round_trip_correctly() {
        let original = Settings {
            recording: RecordingSettings {
                push_to_talk: Some("Ctrl+Alt+Space".to_owned()),
                toggle: Some("Ctrl+Alt+D".to_owned()),
                input_device: Some("Built-in Microphone".to_owned()),
            },
            cleanup: super::CleanupSettings {
                mode: CleanupMode::Verbatim,
                remove_fillers: false,
            },
            startup: super::StartupSettings { run_at_login: true },
            vocabulary: vec![vocabulary_entry(1, "outflow term", "Outflow Term")],
            ..Settings::default()
        };

        let json = original.to_json().expect("valid settings serialize");
        let decoded = Settings::from_json(&json).expect("valid settings decode");

        assert_eq!(decoded, original);
    }

    #[test]
    fn cleanup_modes_use_the_schema_v1_json_values() {
        for (mode, expected) in [
            (CleanupMode::Clean, "clean"),
            (CleanupMode::Verbatim, "verbatim"),
        ] {
            let settings = Settings {
                cleanup: super::CleanupSettings {
                    mode,
                    ..super::CleanupSettings::default()
                },
                ..Settings::default()
            };
            let value: serde_json::Value =
                serde_json::from_str(&settings.to_json().expect("valid settings serialize"))
                    .expect("serializer emits valid JSON");

            assert_eq!(value["cleanup"]["mode"], expected);
        }
    }

    #[test]
    fn unicode_vocabulary_is_preserved_exactly() {
        let original = Settings {
            vocabulary: vec![vocabulary_entry(3, "café 東京 🗣", "Café 東京 🗣")],
            ..Settings::default()
        };
        let json = original.to_json().expect("valid settings serialize");
        let decoded = Settings::from_json(&json).expect("valid settings decode");

        assert_eq!(decoded.vocabulary, original.vocabulary);
    }

    #[test]
    fn malformed_json_is_rejected() {
        assert!(matches!(
            Settings::from_json("{ malformed private input"),
            Err(SettingsJsonError::InvalidDocument { .. })
        ));
    }

    #[test]
    fn unsupported_schema_versions_are_rejected_during_decoding() {
        let original = Settings {
            startup: super::StartupSettings { run_at_login: true },
            ..Settings::default()
        };
        let json = original.to_json().expect("valid settings serialize");

        for version in [0, 2, 37, u32::MAX] {
            let mut value: serde_json::Value =
                serde_json::from_str(&json).expect("serializer emits valid JSON");
            value["schema_version"] = serde_json::json!(version);

            let decoded = Settings::from_json(&value.to_string());
            assert_ne!(decoded, Ok(Settings::default()));
            assert_eq!(
                decoded,
                Err(SettingsJsonError::InvalidSettings(
                    SettingsError::UnsupportedSchemaVersion { found: version }
                ))
            );
        }
    }

    #[test]
    fn invalid_languages_are_rejected_during_decoding() {
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("default settings serialize"),
        )
        .expect("serializer emits valid JSON");
        value["language"] = serde_json::json!("private-language-value");

        let error = Settings::from_json(&value.to_string()).expect_err("language is unsupported");
        assert_eq!(
            error,
            SettingsJsonError::InvalidSettings(SettingsError::UnsupportedLanguage)
        );
        assert!(!error.to_string().contains("private-language-value"));
    }

    #[test]
    fn automatic_update_checking_is_rejected_during_decoding() {
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("default settings serialize"),
        )
        .expect("serializer emits valid JSON");
        value["updates"]["user_initiated_only"] = serde_json::json!(false);

        assert_eq!(
            Settings::from_json(&value.to_string()),
            Err(SettingsJsonError::InvalidSettings(
                SettingsError::AutomaticUpdateChecksNotAllowed
            ))
        );
    }

    #[test]
    fn invalid_vocabulary_is_rejected_after_decoding() {
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("default settings serialize"),
        )
        .expect("serializer emits valid JSON");
        value["vocabulary"] = serde_json::json!([{
            "id": "private-id",
            "spoken": "",
            "written": "private-vocabulary-value"
        }]);

        assert_eq!(
            Settings::from_json(&value.to_string()),
            Err(SettingsJsonError::InvalidSettings(
                SettingsError::EmptyVocabularyPhrase {
                    entry_index: 0,
                    field: VocabularyPhraseField::Spoken,
                }
            ))
        );
    }

    #[test]
    fn invalid_shortcuts_are_rejected_after_decoding() {
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("default settings serialize"),
        )
        .expect("serializer emits valid JSON");
        value["recording"]["push_to_talk"] = serde_json::json!("  ");

        assert_eq!(
            Settings::from_json(&value.to_string()),
            Err(SettingsJsonError::InvalidSettings(
                SettingsError::EmptyShortcut {
                    shortcut: ShortcutKind::PushToTalk,
                }
            ))
        );
    }

    #[test]
    fn missing_required_fields_are_rejected_including_nullable_shortcuts() {
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("default settings serialize"),
        )
        .expect("serializer emits valid JSON");
        value
            .as_object_mut()
            .expect("root is an object")
            .remove("cleanup");
        assert!(matches!(
            Settings::from_json(&value.to_string()),
            Err(SettingsJsonError::InvalidDocument { .. })
        ));

        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("default settings serialize"),
        )
        .expect("serializer emits valid JSON");
        value["recording"]
            .as_object_mut()
            .expect("recording is an object")
            .remove("push_to_talk");
        assert!(matches!(
            Settings::from_json(&value.to_string()),
            Err(SettingsJsonError::InvalidDocument { .. })
        ));
    }

    #[test]
    fn missing_and_malformed_schema_versions_are_rejected() {
        let json = Settings::default()
            .to_json()
            .expect("default settings serialize");

        let mut missing_version: serde_json::Value =
            serde_json::from_str(&json).expect("serializer emits valid JSON");
        missing_version
            .as_object_mut()
            .expect("settings are serialized as an object")
            .remove("schema_version");
        assert!(matches!(
            Settings::from_json(&missing_version.to_string()),
            Err(SettingsJsonError::InvalidDocument { .. })
        ));

        let invalid_versions = [
            serde_json::Value::Null,
            serde_json::json!("1"),
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!(4_294_967_296_u64),
        ];
        for invalid_version in invalid_versions {
            let mut value: serde_json::Value =
                serde_json::from_str(&json).expect("serializer emits valid JSON");
            value["schema_version"] = invalid_version;

            assert!(matches!(
                Settings::from_json(&value.to_string()),
                Err(SettingsJsonError::InvalidDocument { .. })
            ));
        }
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("default settings serialize"),
        )
        .expect("serializer emits valid JSON");
        value["transcript_history"] = serde_json::json!([]);

        assert!(matches!(
            Settings::from_json(&value.to_string()),
            Err(SettingsJsonError::InvalidDocument { .. })
        ));
    }

    #[test]
    fn json_errors_do_not_disclose_vocabulary_or_raw_input() {
        let private_value = "private-vocabulary-value";
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("default settings serialize"),
        )
        .expect("serializer emits valid JSON");
        value["vocabulary"] = serde_json::json!([{
            "id": "private-id",
            "spoken": "",
            "written": private_value
        }]);
        let raw_json = value.to_string();
        let error = Settings::from_json(&raw_json).expect_err("empty vocabulary is invalid");
        assert!(!error.to_string().contains(private_value));
        assert!(!error.to_string().contains(&raw_json));

        let mut invalid_version: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("default settings serialize"),
        )
        .expect("serializer emits valid JSON");
        invalid_version["schema_version"] = serde_json::json!(private_value);
        invalid_version["vocabulary"] = serde_json::json!([{
            "id": "private-id",
            "spoken": "private spoken phrase",
            "written": private_value
        }]);
        let invalid_version_json = invalid_version.to_string();
        let error = Settings::from_json(&invalid_version_json)
            .expect_err("a string schema version is invalid");
        assert!(!error.to_string().contains(private_value));
        assert!(!error.to_string().contains("private spoken phrase"));
        assert!(!error.to_string().contains("private-id"));
        assert!(!error.to_string().contains(&invalid_version_json));

        let malformed_json = format!("{{\"private\":\"{private_value}\"");
        let error = Settings::from_json(&malformed_json).expect_err("JSON is incomplete");
        assert!(!error.to_string().contains(private_value));
        assert!(!error.to_string().contains(&malformed_json));
    }
}
