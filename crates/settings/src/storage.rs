//! Explicit-path loading, user-directed reset, and atomic persistence for C-08 settings.
//!
//! The ordinary loader reads only a caller-supplied regular file, caps input at
//! [`MAX_SETTINGS_FILE_BYTES`], and validates through [`Settings::from_json`].
//! A crate-private review loader uses the same path checks and byte cap but can
//! return only otherwise-valid schema-v1 settings with vocabulary conflicts so
//! the service can report index-only review readiness without writing them.
//! The writer validates and encodes through [`Settings::to_json`], creates a
//! same-directory temporary file, synchronizes it, and atomically renames it
//! over the destination. A confirmed reset requires an existing regular file
//! and writes validated defaults through that same path. Unix files use mode
//! `0600`; non-Unix writer targets fail closed until an approved owner-only ACL
//! implementation is available. Filesystem snapshots and OS backups may retain
//! prior settings; replacement is not secure erasure. This module does not
//! discover paths, migrate versions, or access the network.

use crate::{Settings, SettingsError, SettingsJsonError};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
#[cfg(unix)]
use std::fs::OpenOptions;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;

/// A safe description of a filesystem operation involved in settings storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsStorageOperation {
    /// Inspecting the destination entry without following a symlink.
    InspectDestination,
    /// Inspecting the supplied settings file without following a symlink.
    InspectSource,
    /// Reading bytes from the supplied settings file.
    ReadSettingsFile,
    /// Opening the destination's parent directory.
    OpenParentDirectory,
    /// Creating a new temporary file exclusively.
    CreateTemporaryFile,
    /// Applying restrictive permissions to the temporary file.
    SetTemporaryFilePermissions,
    /// Writing or flushing serialized settings.
    WriteTemporaryFile,
    /// Synchronizing serialized settings to the filesystem.
    SynchronizeTemporaryFile,
    /// Atomically replacing the destination entry.
    ReplaceDestination,
}

impl Display for SettingsStorageOperation {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let operation = match self {
            Self::InspectDestination => "inspect destination",
            Self::InspectSource => "inspect settings file",
            Self::ReadSettingsFile => "read settings file",
            Self::OpenParentDirectory => "open parent directory",
            Self::CreateTemporaryFile => "create temporary file",
            Self::SetTemporaryFilePermissions => "set temporary file permissions",
            Self::WriteTemporaryFile => "write temporary file",
            Self::SynchronizeTemporaryFile => "synchronize temporary file",
            Self::ReplaceDestination => "replace destination",
        };
        formatter.write_str(operation)
    }
}

/// A redacted settings storage error.
///
/// Errors intentionally retain no path, JSON, vocabulary, or shortcut value.
/// [`Self::ReplacementFailed`] reports that the destination-replacement
/// operation returned an error; the variant alone does not establish the
/// destination's state for every filesystem failure. In contrast,
/// [`Self::ReplacementDurabilityUncertain`] is returned after replacement
/// succeeded but parent-directory synchronization failed. The new contents may
/// be visible, but crash durability is unconfirmed and no rollback is attempted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingsStorageError {
    /// A settings configuration violates a schema v1 rule.
    InvalidSettings(SettingsError),
    /// Valid settings could not be encoded by the existing JSON codec.
    SerializationFailed,
    /// The destination path does not name a file entry.
    InvalidDestinationPath,
    /// No existing settings file was available to reset; reset never creates one.
    DestinationMissing,
    /// The caller did not confirm the requested settings reset.
    ResetNotConfirmed,
    /// The supplied source path does not name a file entry.
    InvalidFilePath,
    /// The destination parent does not exist.
    MissingParentDirectory,
    /// The destination parent exists but is not a plain directory.
    InvalidParentDirectory,
    /// The destination is a symbolic link or a non-regular filesystem object.
    UnsafeDestination,
    /// The supplied path is a symbolic link or does not name a regular file.
    UnsafeFile,
    /// The file exceeds the documented maximum size.
    FileTooLarge {
        /// Maximum accepted file size in bytes.
        max_bytes: u64,
    },
    /// The file contents are not valid UTF-8.
    InvalidUtf8,
    /// The file is not valid schema v1 JSON; only parser location is retained.
    InvalidDocument {
        /// One-based line reported by the JSON decoder.
        line: usize,
        /// One-based column reported by the JSON decoder.
        column: usize,
    },
    /// The JSON decoder failed for a reason other than a content error.
    SettingsDecodingFailed,
    /// The file could not be inspected or read for a non-permission reason.
    ReadFailed {
        /// The safe filesystem operation that failed.
        operation: SettingsStorageOperation,
    },
    /// An operation was denied by the filesystem.
    PermissionDenied {
        /// The storage operation that received the permission error.
        operation: SettingsStorageOperation,
    },
    /// A unique same-directory temporary file could not be created.
    TemporaryFileCreationFailed,
    /// The temporary file could not be written, flushed, or cleaned up.
    TemporaryFileWriteFailed,
    /// Restrictive permissions could not be applied to the temporary file.
    TemporaryFilePermissionsFailed,
    /// The temporary file could not be synchronized before replacement.
    TemporaryFileSynchronizationFailed,
    /// The destination-replacement operation returned an error; post-error destination state is not implied.
    ReplacementFailed,
    /// Replacement succeeded, but syncing the parent directory failed; durability is unconfirmed.
    ReplacementDurabilityUncertain,
    /// This target lacks an implemented owner-only atomic replacement strategy.
    UnsupportedPlatform,
}

impl Display for SettingsStorageError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSettings(error) => write!(formatter, "invalid settings: {error}"),
            Self::SerializationFailed => formatter.write_str("settings serialization failed"),
            Self::InvalidDestinationPath => {
                formatter.write_str("destination path must name a file")
            }
            Self::DestinationMissing => {
                formatter.write_str("settings file does not exist; reset did not create one")
            }
            Self::ResetNotConfirmed => {
                formatter.write_str("settings reset was not confirmed")
            }
            Self::InvalidFilePath => formatter.write_str("settings path must name a file"),
            Self::MissingParentDirectory => {
                formatter.write_str("destination parent directory does not exist")
            }
            Self::InvalidParentDirectory => {
                formatter.write_str("destination parent is not a plain directory")
            }
            Self::UnsafeDestination => formatter.write_str(
                "destination must be a regular file and must not be a symbolic link",
            ),
            Self::UnsafeFile => formatter.write_str(
                "settings file must be a regular file and must not be a symbolic link",
            ),
            Self::FileTooLarge { max_bytes } => write!(
                formatter,
                "settings file exceeds the maximum size of {max_bytes} bytes"
            ),
            Self::InvalidUtf8 => formatter.write_str("settings file is not valid UTF-8"),
            Self::InvalidDocument { line, column } => write!(
                formatter,
                "invalid settings JSON document at line {line}, column {column}"
            ),
            Self::SettingsDecodingFailed => formatter.write_str("settings JSON decoding failed"),
            Self::ReadFailed { operation } => {
                write!(formatter, "could not {operation} because of a filesystem error")
            }
            Self::PermissionDenied { operation } => {
                write!(formatter, "permission denied while trying to {operation}")
            }
            Self::TemporaryFileCreationFailed => {
                formatter.write_str("could not create a unique temporary settings file")
            }
            Self::TemporaryFileWriteFailed => {
                formatter.write_str("could not write or clean up the temporary settings file")
            }
            Self::TemporaryFilePermissionsFailed => {
                formatter.write_str("could not restrict temporary settings file permissions")
            }
            Self::TemporaryFileSynchronizationFailed => {
                formatter.write_str("could not synchronize the temporary settings file")
            }
            Self::ReplacementFailed => {
                formatter.write_str("atomic settings replacement operation failed")
            }
            Self::ReplacementDurabilityUncertain => formatter.write_str(
                "settings replacement succeeded, but directory durability could not be confirmed",
            ),
            Self::UnsupportedPlatform => formatter.write_str(
                "atomic settings writing with owner-only permissions is not supported on this platform",
            ),
        }
    }
}

impl Error for SettingsStorageError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidSettings(error) => Some(error),
            _ => None,
        }
    }
}

/// Maximum accepted settings document size: 1 MiB.
///
/// One MiB admits ordinary schema v1 documents while bounding memory and parser
/// work. The schema does not cap the byte length of every string field, so a
/// very large otherwise well-formed document is intentionally rejected here.
pub const MAX_SETTINGS_FILE_BYTES: u64 = 1024 * 1024;

/// Reads and validates settings from the explicitly supplied local file path.
///
/// A genuinely missing file returns `Ok(None)` and is never created. Existing
/// directories, symbolic links, oversized files, invalid UTF-8, malformed or
/// unsupported JSON, and invalid settings return explicit errors. This
/// function never modifies the source file or substitutes defaults. The caller
/// may choose [`Settings::default`] only after receiving `Ok(None)`.
///
/// File metadata is checked without following a symlink before opening, then
/// the opened handle and path are checked again. The standard library does not
/// expose a portable no-follow open operation for every supported target, so
/// an adversary with concurrent filesystem write access can still race path
/// replacement between checks and `File::open`. These checks are not a claim
/// of complete filesystem attack resistance.
///
/// # Errors
///
/// Returns a redacted error for unsafe paths, filesystem failures, oversized
/// or non-UTF-8 files, invalid JSON, unsupported schema versions, or settings
/// that fail validation. Errors never include paths, file contents, or
/// vocabulary and shortcut values.
pub fn load_settings(path: &Path) -> Result<Option<Settings>, SettingsStorageError> {
    load_settings_with_decoder(path, Settings::from_json)
}

/// Reads a schema-v1 file through the same bounded, no-symlink checks while
/// allowing only vocabulary-policy conflicts for explicit service review.
/// This is crate-private so ordinary readers remain fail-closed.
pub(crate) fn load_settings_for_vocabulary_review(
    path: &Path,
) -> Result<Option<Settings>, SettingsStorageError> {
    load_settings_with_decoder(path, Settings::from_json_for_vocabulary_review)
}

fn load_settings_with_decoder(
    path: &Path,
    decode: fn(&str) -> Result<Settings, SettingsJsonError>,
) -> Result<Option<Settings>, SettingsStorageError> {
    if path.file_name().filter(|name| !name.is_empty()).is_none() {
        return Err(SettingsStorageError::InvalidFilePath);
    }

    let path_metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(map_read_error(
                error,
                SettingsStorageOperation::InspectSource,
            ));
        }
    };
    if path_metadata.file_type().is_symlink() || !path_metadata.is_file() {
        return Err(SettingsStorageError::UnsafeFile);
    }
    if path_metadata.len() > MAX_SETTINGS_FILE_BYTES {
        return Err(SettingsStorageError::FileTooLarge {
            max_bytes: MAX_SETTINGS_FILE_BYTES,
        });
    }

    let file = match File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(map_read_error(
                error,
                SettingsStorageOperation::ReadSettingsFile,
            ));
        }
    };
    let opened_metadata = file
        .metadata()
        .map_err(|error| map_read_error(error, SettingsStorageOperation::ReadSettingsFile))?;
    if !opened_metadata.is_file() {
        return Err(SettingsStorageError::UnsafeFile);
    }
    if opened_metadata.len() > MAX_SETTINGS_FILE_BYTES {
        return Err(SettingsStorageError::FileTooLarge {
            max_bytes: MAX_SETTINGS_FILE_BYTES,
        });
    }

    let current_path_metadata = fs::symlink_metadata(path)
        .map_err(|error| map_read_error(error, SettingsStorageOperation::InspectSource))?;
    if current_path_metadata.file_type().is_symlink() || !current_path_metadata.is_file() {
        return Err(SettingsStorageError::UnsafeFile);
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;

        if opened_metadata.dev() != current_path_metadata.dev()
            || opened_metadata.ino() != current_path_metadata.ino()
        {
            return Err(SettingsStorageError::UnsafeFile);
        }
    }
    if current_path_metadata.len() > MAX_SETTINGS_FILE_BYTES {
        return Err(SettingsStorageError::FileTooLarge {
            max_bytes: MAX_SETTINGS_FILE_BYTES,
        });
    }

    // Read at most one byte beyond the cap so growth after the metadata checks
    // is detected without loading an arbitrarily large file into memory.
    let mut bytes = Vec::new();
    file.take(MAX_SETTINGS_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| map_read_error(error, SettingsStorageOperation::ReadSettingsFile))?;
    if bytes.len() as u64 > MAX_SETTINGS_FILE_BYTES {
        return Err(SettingsStorageError::FileTooLarge {
            max_bytes: MAX_SETTINGS_FILE_BYTES,
        });
    }

    let json = String::from_utf8(bytes).map_err(|_| SettingsStorageError::InvalidUtf8)?;
    match decode(&json) {
        Ok(settings) => Ok(Some(settings)),
        Err(SettingsJsonError::InvalidDocument { line, column }) => {
            Err(SettingsStorageError::InvalidDocument { line, column })
        }
        Err(SettingsJsonError::InvalidSettings(error)) => {
            Err(SettingsStorageError::InvalidSettings(error))
        }
        Err(SettingsJsonError::SerializationFailed) => {
            Err(SettingsStorageError::SettingsDecodingFailed)
        }
    }
}

fn map_read_error(error: io::Error, operation: SettingsStorageOperation) -> SettingsStorageError {
    if error.kind() == io::ErrorKind::PermissionDenied {
        SettingsStorageError::PermissionDenied { operation }
    } else {
        SettingsStorageError::ReadFailed { operation }
    }
}

#[cfg(test)]
mod load_tests {
    #[cfg(unix)]
    use super::save_settings_atomic;
    use super::{
        MAX_SETTINGS_FILE_BYTES, SettingsStorageError, SettingsStorageOperation, load_settings,
    };
    #[cfg(unix)]
    use crate::VocabularyEntry;
    use crate::{
        Settings, SettingsError, SettingsInitialization, initialize_settings,
        reset_settings_to_defaults,
    };
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            for attempt in 0..128_u32 {
                let path = std::env::temp_dir().join(format!(
                    "outflow-settings-load-test-{}-{timestamp}-{attempt}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create isolated test directory: {error}"),
                }
            }
            panic!("could not create a unique isolated test directory");
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn valid_settings() -> Settings {
        Settings {
            vocabulary: vec![crate::VocabularyEntry {
                id: "synthetic-load-test-entry".to_owned(),
                spoken: "synthetic spoken phrase".to_owned(),
                written: "Synthetic Written Phrase".to_owned(),
            }],
            ..Settings::default()
        }
    }

    #[test]
    fn valid_v1_file_loads_equal_settings_without_changing_the_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let original = valid_settings();
        let contents = original.to_json().expect("encode synthetic settings");
        fs::write(&path, contents.as_bytes()).expect("write synthetic file");

        assert_eq!(load_settings(&path), Ok(Some(original)));
        assert_eq!(
            fs::read(path).expect("source file remains readable"),
            contents.as_bytes()
        );
    }

    #[test]
    fn missing_file_returns_none_without_creating_a_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("missing-settings.json");

        assert_eq!(load_settings(&path), Ok(None));
        assert!(!path.exists());
    }

    #[test]
    fn initialization_returns_valid_schema_v1_defaults_without_creating_a_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("missing-settings.json");

        let initialization = initialize_settings(&path).expect("missing file is first run");
        let SettingsInitialization::FirstRun(settings) = initialization else {
            panic!("missing file must return first-run defaults");
        };

        assert_eq!(settings.schema_version, crate::CURRENT_SCHEMA_VERSION);
        assert_eq!(settings.schema_version, 1);
        assert!(settings.validate().is_ok());
        assert!(!path.exists());
    }

    #[test]
    fn initialization_loads_existing_custom_settings_without_changing_the_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let original = valid_settings();
        let contents = original.to_json().expect("encode synthetic settings");
        fs::write(&path, contents.as_bytes()).expect("write synthetic file");

        assert_eq!(
            initialize_settings(&path),
            Ok(SettingsInitialization::Loaded(original))
        );
        assert_eq!(
            fs::read(path).expect("source file remains readable"),
            contents.as_bytes()
        );
    }

    #[test]
    fn initialization_rejects_and_preserves_invalid_configurations() {
        #[derive(Clone, Copy)]
        enum InvalidFixture {
            MalformedJson,
            UnsupportedVersion,
            InvalidSettings,
            InvalidUtf8,
            Oversized,
        }

        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let mut unsupported: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("encode default settings"),
        )
        .expect("decode synthetic settings");
        unsupported["schema_version"] = serde_json::json!(7);
        unsupported["vocabulary"] = serde_json::json!([{
            "id": "private-config-value",
            "spoken": "private spoken phrase",
            "written": "private written phrase"
        }]);
        let mut invalid_settings: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("encode default settings"),
        )
        .expect("decode synthetic settings");
        invalid_settings["language"] = serde_json::json!("private-config-value");

        let fixtures = [
            (
                InvalidFixture::MalformedJson,
                br#"{"private":"private-config-value", broken}"#.to_vec(),
            ),
            (
                InvalidFixture::UnsupportedVersion,
                serde_json::to_vec(&unsupported).expect("encode unsupported fixture"),
            ),
            (
                InvalidFixture::InvalidSettings,
                serde_json::to_vec(&invalid_settings).expect("encode invalid fixture"),
            ),
            (
                InvalidFixture::InvalidUtf8,
                b"\xffprivate-config-value".to_vec(),
            ),
            (
                InvalidFixture::Oversized,
                vec![b'x'; (MAX_SETTINGS_FILE_BYTES + 1) as usize],
            ),
        ];

        for (fixture, original_bytes) in fixtures {
            fs::write(&path, &original_bytes).expect("write synthetic invalid configuration");
            let error = initialize_settings(&path)
                .expect_err("invalid existing configuration must not become defaults");

            match fixture {
                InvalidFixture::MalformedJson => {
                    assert!(matches!(
                        error,
                        SettingsStorageError::InvalidDocument { .. }
                    ));
                }
                InvalidFixture::UnsupportedVersion => {
                    assert_eq!(
                        error,
                        SettingsStorageError::InvalidSettings(
                            SettingsError::UnsupportedSchemaVersion { found: 7 }
                        )
                    );
                }
                InvalidFixture::InvalidSettings => {
                    assert_eq!(
                        error,
                        SettingsStorageError::InvalidSettings(SettingsError::UnsupportedLanguage)
                    );
                }
                InvalidFixture::InvalidUtf8 => {
                    assert_eq!(error, SettingsStorageError::InvalidUtf8);
                }
                InvalidFixture::Oversized => {
                    assert_eq!(
                        error,
                        SettingsStorageError::FileTooLarge {
                            max_bytes: MAX_SETTINGS_FILE_BYTES,
                        }
                    );
                }
            }

            assert!(!error.to_string().contains("private-config-value"));
            if let Ok(original_json) = std::str::from_utf8(&original_bytes) {
                assert!(!error.to_string().contains(original_json));
            }
            assert_eq!(
                fs::read(&path).expect("invalid source remains readable"),
                original_bytes
            );
        }
    }

    #[test]
    fn initialization_preserves_filesystem_failures_as_errors() {
        let directory = TestDirectory::new();
        let parent_file = directory.path().join("not-a-directory");
        fs::write(&parent_file, b"synthetic").expect("create non-directory parent");
        let path = parent_file.join("settings.json");

        assert_eq!(
            initialize_settings(&path),
            Err(SettingsStorageError::ReadFailed {
                operation: SettingsStorageOperation::InspectSource,
            })
        );
    }

    #[cfg(unix)]
    #[test]
    fn initialization_reports_permission_denial_without_returning_defaults() {
        use std::fs::File;
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let contents = Settings::default()
            .to_json()
            .expect("encode synthetic defaults");
        fs::write(&path, contents.as_bytes()).expect("write synthetic file");
        let mut permissions = fs::metadata(&path)
            .expect("inspect synthetic file")
            .permissions();
        permissions.set_mode(0o000);
        fs::set_permissions(&path, permissions).expect("deny file read access");

        // Privileged test runners may bypass mode bits; skip the filesystem
        // denial assertion there because no portable permission fixture exists.
        if File::open(&path).is_ok() {
            let mut permissions = fs::metadata(&path)
                .expect("inspect synthetic file")
                .permissions();
            permissions.set_mode(0o600);
            fs::set_permissions(&path, permissions).expect("restore synthetic file access");
            return;
        }

        let result = initialize_settings(&path);
        let mut permissions = fs::metadata(&path)
            .expect("inspect synthetic file")
            .permissions();
        permissions.set_mode(0o600);
        fs::set_permissions(&path, permissions).expect("restore synthetic file access");

        assert!(matches!(
            result,
            Err(SettingsStorageError::PermissionDenied {
                operation: SettingsStorageOperation::ReadSettingsFile,
            })
        ));
    }

    #[test]
    fn malformed_json_is_rejected_without_echoing_or_changing_content() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let original = br#"{"private_marker":"raw-private-json-9917", not-json}"#;
        fs::write(&path, original).expect("write malformed synthetic file");

        let error = load_settings(&path).expect_err("malformed JSON rejected");

        assert!(matches!(
            error,
            SettingsStorageError::InvalidDocument { .. }
        ));
        assert!(!error.to_string().contains("raw-private-json-9917"));
        assert!(
            !error
                .to_string()
                .contains(std::str::from_utf8(original).expect("fixture UTF-8"))
        );
        assert_eq!(fs::read(path).expect("read unchanged source"), original);
    }

    #[test]
    fn unsupported_schema_version_is_explicit_and_preserves_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("encode default settings"),
        )
        .expect("decode fixture JSON");
        value["schema_version"] = serde_json::json!(7);
        value["vocabulary"] = serde_json::json!([{
            "id": "private-id-9917",
            "spoken": "private spoken phrase 9917",
            "written": "private written phrase 9917"
        }]);
        let original = serde_json::to_vec(&value).expect("encode unsupported fixture");
        fs::write(&path, &original).expect("write unsupported fixture");

        let error = load_settings(&path).expect_err("unsupported version rejected");

        assert_eq!(
            error,
            SettingsStorageError::InvalidSettings(SettingsError::UnsupportedSchemaVersion {
                found: 7
            })
        );
        assert!(!error.to_string().contains("private"));
        assert!(
            !error
                .to_string()
                .contains(std::str::from_utf8(&original).expect("fixture UTF-8"))
        );
        assert_eq!(fs::read(path).expect("read unchanged source"), original);
    }

    #[test]
    fn invalid_settings_are_rejected_without_disclosing_values_or_changing_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let mut value: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("encode default settings"),
        )
        .expect("decode fixture JSON");
        value["vocabulary"] = serde_json::json!([{
            "id": "private-id-7712",
            "spoken": "",
            "written": "private vocabulary 7712"
        }]);
        let original = serde_json::to_vec(&value).expect("encode invalid fixture");
        fs::write(&path, &original).expect("write invalid fixture");

        let error = load_settings(&path).expect_err("invalid settings rejected");

        assert!(matches!(error, SettingsStorageError::InvalidSettings(_)));
        assert!(!error.to_string().contains("private"));
        assert!(
            !error
                .to_string()
                .contains(std::str::from_utf8(&original).expect("fixture UTF-8"))
        );
        assert_eq!(fs::read(path).expect("read unchanged source"), original);
    }

    #[test]
    fn invalid_utf8_is_rejected_without_changing_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let original = [b'{', b' ', 0xff, b' ', b'}'];
        fs::write(&path, original).expect("write invalid UTF-8 fixture");

        assert_eq!(load_settings(&path), Err(SettingsStorageError::InvalidUtf8));
        assert_eq!(fs::read(path).expect("read unchanged source"), original);
    }

    #[test]
    fn oversized_file_is_rejected_without_reading_it_fully_or_changing_it() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let original = vec![b' '; MAX_SETTINGS_FILE_BYTES as usize + 1];
        fs::write(&path, &original).expect("write oversized synthetic file");

        assert_eq!(
            load_settings(&path),
            Err(SettingsStorageError::FileTooLarge {
                max_bytes: MAX_SETTINGS_FILE_BYTES
            })
        );
        assert_eq!(
            fs::metadata(&path).expect("source remains").len(),
            original.len() as u64
        );
        assert_eq!(fs::read(path).expect("read unchanged source"), original);
    }

    #[test]
    fn directory_path_is_rejected() {
        let directory = TestDirectory::new();

        assert_eq!(
            load_settings(directory.path()),
            Err(SettingsStorageError::UnsafeFile)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_link_source_is_rejected_without_following_it() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new();
        let target = directory.path().join("real-settings.json");
        let link = directory.path().join("settings.json");
        let contents = Settings::default().to_json().expect("encode settings");
        fs::write(&target, contents.as_bytes()).expect("write target file");
        symlink(&target, &link).expect("create symlink");

        assert_eq!(load_settings(&link), Err(SettingsStorageError::UnsafeFile));
        assert_eq!(
            fs::read(target).expect("target remains unchanged"),
            contents.as_bytes()
        );
        assert!(
            fs::symlink_metadata(link)
                .expect("symlink remains")
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn filesystem_read_failure_is_descriptive_and_redacted() {
        let directory = TestDirectory::new();
        let parent_file = directory.path().join("not-a-directory-private-name");
        fs::write(&parent_file, b"synthetic").expect("create non-directory parent");
        let path = parent_file.join("settings.json");

        let error = load_settings(&path).expect_err("invalid parent fails to inspect");

        assert_eq!(
            error,
            SettingsStorageError::ReadFailed {
                operation: SettingsStorageOperation::InspectSource
            }
        );
        assert!(error.to_string().contains("inspect settings file"));
        assert!(!error.to_string().contains("not-a-directory-private-name"));
    }

    #[test]
    fn reset_without_confirmation_does_not_modify_the_file_or_disclose_content() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = br#"{"vocabulary":[{"spoken":"private reset phrase"}]}"#;
        fs::write(&path, prior).expect("write synthetic settings");

        let error = reset_settings_to_defaults(&path, false)
            .expect_err("unconfirmed reset must be cancelled");

        assert_eq!(error, SettingsStorageError::ResetNotConfirmed);
        assert!(!error.to_string().contains("private reset phrase"));
        assert_eq!(fs::read(path).expect("read unchanged settings"), prior);
    }

    #[cfg(unix)]
    #[test]
    fn confirmed_reset_replaces_vocabulary_and_preferences_with_defaults() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let custom = Settings {
            cleanup: crate::CleanupSettings {
                mode: crate::CleanupMode::Verbatim,
                remove_fillers: false,
            },
            startup: crate::StartupSettings { run_at_login: true },
            vocabulary: vec![crate::VocabularyEntry {
                id: "synthetic-reset-entry".to_owned(),
                spoken: "synthetic reset phrase".to_owned(),
                written: "Synthetic Reset Phrase".to_owned(),
            }],
            ..Settings::default()
        };
        fs::write(&path, custom.to_json().expect("encode synthetic settings"))
            .expect("write custom settings");

        reset_settings_to_defaults(&path, true).expect("confirmed reset succeeds");

        let restored = load_settings(&path)
            .expect("reset file loads and validates")
            .expect("reset file exists");
        assert_eq!(restored, Settings::default());
        assert_eq!(restored.schema_version, 1);
        assert!(restored.vocabulary.is_empty());
        assert_eq!(restored.cleanup, crate::CleanupSettings::default());
        assert_eq!(restored.startup, crate::StartupSettings::default());
        assert_eq!(
            fs::metadata(path)
                .expect("inspect reset file permissions")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }

    #[cfg(unix)]
    #[test]
    fn malformed_configuration_can_be_reset_to_valid_defaults() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        fs::write(&path, b"{ malformed private settings")
            .expect("write malformed synthetic settings");

        reset_settings_to_defaults(&path, true).expect("explicit reset recovers malformed JSON");

        assert_eq!(load_settings(&path), Ok(Some(Settings::default())));
    }

    #[cfg(unix)]
    #[test]
    fn unsupported_schema_configuration_can_be_reset_explicitly() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let mut document: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("encode synthetic defaults"),
        )
        .expect("decode synthetic defaults");
        document["schema_version"] = serde_json::json!(88);
        fs::write(
            &path,
            serde_json::to_vec(&document).expect("encode unsupported schema fixture"),
        )
        .expect("write unsupported synthetic settings");

        reset_settings_to_defaults(&path, true).expect("explicit reset replaces unsupported JSON");

        assert_eq!(load_settings(&path), Ok(Some(Settings::default())));
    }

    #[cfg(unix)]
    #[test]
    fn explicit_settings_reset_is_scoped_and_separate_from_loading() {
        use std::os::unix::fs::PermissionsExt;

        let valid_directory = TestDirectory::new();
        let settings_path = valid_directory.path().join("settings.json");
        let sentinel_path = valid_directory.path().join("unrelated-sentinel.bin");
        let sentinel_bytes = b"Task 64 unrelated sentinel bytes\n";
        fs::write(&sentinel_path, sentinel_bytes).expect("create unrelated sentinel");

        let custom = Settings {
            cleanup: crate::CleanupSettings {
                mode: crate::CleanupMode::Verbatim,
                remove_fillers: false,
            },
            startup: crate::StartupSettings { run_at_login: true },
            vocabulary: vec![
                VocabularyEntry {
                    id: "task64-entry-one".to_owned(),
                    spoken: "synthetic spoken phrase alpha".to_owned(),
                    written: "Synthetic written replacement alpha".to_owned(),
                },
                VocabularyEntry {
                    id: "task64-entry-two".to_owned(),
                    spoken: "synthetic spoken phrase beta".to_owned(),
                    written: "Synthetic written replacement beta".to_owned(),
                },
            ],
            ..Settings::default()
        };
        assert_ne!(custom, Settings::default());

        save_settings_atomic(&settings_path, &custom)
            .expect("save synthetic schema-v1 settings through the public writer");
        assert!(settings_path.starts_with(valid_directory.path()));
        let loaded_before_reset = load_settings(&settings_path)
            .expect("load valid synthetic settings")
            .expect("settings file exists");
        assert_eq!(loaded_before_reset, custom);
        assert_eq!(loaded_before_reset.vocabulary.len(), 2);

        let original_settings_bytes = fs::read(&settings_path).expect("capture settings bytes");
        let original_sentinel_bytes = fs::read(&sentinel_path).expect("capture sentinel bytes");
        assert_eq!(original_sentinel_bytes, sentinel_bytes);
        let entries_before_reset = directory_entry_names(valid_directory.path());
        let mut expected_entries = vec![
            std::ffi::OsString::from("settings.json"),
            std::ffi::OsString::from("unrelated-sentinel.bin"),
        ];
        expected_entries.sort();
        assert_eq!(entries_before_reset, expected_entries);

        let loaded_again = load_settings(&settings_path)
            .expect("ordinary loading succeeds before explicit reset")
            .expect("settings remain present");
        assert_eq!(loaded_again, custom);
        assert_eq!(
            fs::read(&settings_path).expect("settings after load"),
            original_settings_bytes
        );
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after load"),
            original_sentinel_bytes
        );
        assert_eq!(
            directory_entry_names(valid_directory.path()),
            entries_before_reset
        );

        let initialized = initialize_settings(&settings_path)
            .expect("ordinary initialization succeeds before explicit reset");
        assert_eq!(initialized, SettingsInitialization::Loaded(custom.clone()));
        assert_eq!(
            fs::read(&settings_path).expect("settings after initialization"),
            original_settings_bytes
        );
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after initialization"),
            original_sentinel_bytes
        );
        assert_eq!(
            directory_entry_names(valid_directory.path()),
            entries_before_reset
        );

        let unconfirmed_error = reset_settings_to_defaults(&settings_path, false)
            .expect_err("a reset without caller confirmation must be rejected");
        assert_eq!(unconfirmed_error, SettingsStorageError::ResetNotConfirmed);
        let unconfirmed_display = unconfirmed_error.to_string();
        let unconfirmed_debug = format!("{unconfirmed_error:?}");
        for private_value in [
            "synthetic spoken phrase alpha",
            "Synthetic written replacement alpha",
            "synthetic spoken phrase beta",
            "Synthetic written replacement beta",
            std::str::from_utf8(&original_settings_bytes).expect("settings JSON is UTF-8"),
        ] {
            assert!(!unconfirmed_display.contains(private_value));
            assert!(!unconfirmed_debug.contains(private_value));
        }
        assert_eq!(
            fs::read(&settings_path).expect("settings after unconfirmed reset"),
            original_settings_bytes
        );
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after unconfirmed reset"),
            original_sentinel_bytes
        );
        assert_eq!(
            directory_entry_names(valid_directory.path()),
            entries_before_reset
        );

        let malformed_directory = TestDirectory::new();
        let malformed_path = malformed_directory.path().join("settings.json");
        let malformed_sentinel = malformed_directory.path().join("unrelated-sentinel.bin");
        let malformed_bytes = br#"{"schema_version":1,"vocabulary":[{"id":"task64-malformed","spoken":"synthetic malformed spoken phrase","written":"Synthetic malformed replacement"}"#;
        fs::write(&malformed_path, malformed_bytes).expect("write malformed synthetic JSON");
        fs::write(&malformed_sentinel, sentinel_bytes).expect("create malformed-fixture sentinel");
        let malformed_entries = directory_entry_names(malformed_directory.path());
        assert_eq!(malformed_entries, expected_entries);
        let malformed_error =
            load_settings(&malformed_path).expect_err("loader rejects malformed existing JSON");
        assert!(matches!(
            &malformed_error,
            SettingsStorageError::InvalidDocument { .. }
        ));
        let malformed_initialization_error = initialize_settings(&malformed_path)
            .expect_err("initializer rejects malformed existing JSON");
        assert_eq!(malformed_initialization_error, malformed_error);
        for error in [&malformed_error, &malformed_initialization_error] {
            let display = error.to_string();
            let debug = format!("{error:?}");
            for private_value in [
                "synthetic malformed spoken phrase",
                "Synthetic malformed replacement",
                std::str::from_utf8(malformed_bytes).expect("fixture JSON prefix is UTF-8"),
            ] {
                assert!(!display.contains(private_value));
                assert!(!debug.contains(private_value));
            }
        }
        assert_eq!(
            fs::read(&malformed_path).expect("malformed bytes remain"),
            malformed_bytes
        );
        assert_eq!(
            fs::read(&malformed_sentinel).expect("malformed sentinel remains"),
            sentinel_bytes
        );
        assert_eq!(
            directory_entry_names(malformed_directory.path()),
            malformed_entries
        );

        let unsupported_directory = TestDirectory::new();
        let unsupported_path = unsupported_directory.path().join("settings.json");
        let unsupported_sentinel = unsupported_directory.path().join("unrelated-sentinel.bin");
        let mut unsupported_document: serde_json::Value =
            serde_json::from_str(&custom.to_json().expect("encode synthetic v1 settings"))
                .expect("decode synthetic v1 settings");
        unsupported_document["schema_version"] = serde_json::json!(2);
        let unsupported_bytes =
            serde_json::to_vec(&unsupported_document).expect("encode synthetic schema-v2 fixture");
        fs::write(&unsupported_path, &unsupported_bytes).expect("write schema-v2 fixture");
        fs::write(&unsupported_sentinel, sentinel_bytes).expect("create schema-v2 sentinel");
        let unsupported_entries = directory_entry_names(unsupported_directory.path());
        assert_eq!(unsupported_entries, expected_entries);

        let unsupported_error = load_settings(&unsupported_path)
            .expect_err("loader rejects an unsupported schema-v2 file");
        assert_unsupported_schema_error(&unsupported_error, 2);
        let unsupported_initialization_error = initialize_settings(&unsupported_path)
            .expect_err("initializer rejects an unsupported schema-v2 file");
        assert_eq!(unsupported_initialization_error, unsupported_error);
        let unsupported_json =
            std::str::from_utf8(&unsupported_bytes).expect("synthetic schema-v2 fixture is UTF-8");
        for error in [&unsupported_error, &unsupported_initialization_error] {
            let display = error.to_string();
            let debug = format!("{error:?}");
            for private_value in [
                "synthetic spoken phrase alpha",
                "Synthetic written replacement alpha",
                unsupported_json,
            ] {
                assert!(!display.contains(private_value));
                assert!(!debug.contains(private_value));
            }
        }
        assert_eq!(
            fs::read(&unsupported_path).expect("schema-v2 bytes remain"),
            unsupported_bytes
        );
        assert_eq!(
            fs::read(&unsupported_sentinel).expect("schema-v2 sentinel remains"),
            sentinel_bytes
        );
        assert_eq!(
            directory_entry_names(unsupported_directory.path()),
            unsupported_entries
        );

        reset_settings_to_defaults(&settings_path, true)
            .expect("explicit confirmed reset replaces settings with defaults");
        let reset_settings = load_settings(&settings_path)
            .expect("load persisted reset result")
            .expect("reset writes a defaults file instead of deleting it");
        assert_eq!(reset_settings, Settings::default());
        assert_eq!(reset_settings.schema_version, 1);
        assert!(reset_settings.vocabulary.is_empty());
        let reset_bytes = fs::read(&settings_path).expect("capture persisted defaults");
        assert_ne!(reset_bytes, original_settings_bytes);
        assert_eq!(
            fs::metadata(&settings_path)
                .expect("inspect reset file permissions")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after reset"),
            original_sentinel_bytes
        );
        assert_eq!(
            directory_entry_names(valid_directory.path()),
            entries_before_reset
        );
        assert_eq!(
            initialize_settings(&settings_path).expect("initialize after persisted reset"),
            SettingsInitialization::Loaded(Settings::default())
        );

        reset_settings_to_defaults(&settings_path, true)
            .expect("a repeated confirmed reset succeeds on the existing file");
        assert_eq!(
            load_settings(&settings_path).expect("load repeated reset result"),
            Some(Settings::default())
        );
        assert_eq!(
            fs::read(&settings_path).expect("settings after repeated reset"),
            reset_bytes
        );
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after repeated reset"),
            original_sentinel_bytes
        );
        assert_eq!(
            directory_entry_names(valid_directory.path()),
            entries_before_reset
        );

        // The unsupported file is replaced only here, after the preceding loader
        // and initializer assertions proved that ordinary startup preserved it.
        reset_settings_to_defaults(&unsupported_path, true)
            .expect("explicit reset is allowed to replace the separate v2 fixture");
        assert_eq!(
            load_settings(&unsupported_path).expect("load explicitly reset v2 file"),
            Some(Settings::default())
        );
        assert_eq!(
            initialize_settings(&unsupported_path).expect("initialize after explicit v2 reset"),
            SettingsInitialization::Loaded(Settings::default())
        );
        assert_eq!(
            fs::read(&unsupported_sentinel).expect("v2 sentinel after explicit reset"),
            sentinel_bytes
        );
        assert_eq!(
            directory_entry_names(unsupported_directory.path()),
            unsupported_entries
        );

        println!("Outflow explicit settings reset safety test\n");
        println!("Valid settings before reset: VERIFIED");
        println!("Stored vocabulary before reset: VERIFIED");
        println!("Load without reset: NON-DESTRUCTIVE");
        println!("Initialization without reset: NON-DESTRUCTIVE");
        println!("Unconfirmed reset: REJECTED WITHOUT MUTATION");
        println!("Explicit reset invoked: VERIFIED");
        println!("Documented reset result: DEFAULTS PERSISTED AS SCHEMA V1");
        println!("Previous vocabulary after reset: NOT PRESENT IN PERSISTED SETTINGS");
        println!("Unrelated sentinel files: PRESERVED");
        println!("Unexpected temporary, backup, transcript, or audio files: NONE");
        println!("Repeated reset behavior: SUCCEEDED; DEFAULTS UNCHANGED");
        println!("Malformed and unsupported settings during initialization: PRESERVED");
        println!("Explicit schema-v2 reset: DEFAULTS PERSISTED; MIGRATION NOT PERFORMED");
        println!("Automatic reset: NO");
        println!("Schema migration: NO");
        println!("Real user settings accessed: NO");
        println!("User confirmation UI tested: NO");
    }

    #[test]
    fn confirmed_reset_does_not_create_a_missing_configuration() {
        let directory = TestDirectory::new();
        let path = directory.path().join("missing-settings.json");

        assert_eq!(
            reset_settings_to_defaults(&path, true),
            Err(SettingsStorageError::DestinationMissing)
        );
        assert!(!path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn reset_rejects_a_symbolic_link_without_changing_its_target() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new();
        let target = directory.path().join("real-settings.json");
        let link = directory.path().join("settings.json");
        let prior = b"synthetic symlink target";
        fs::write(&target, prior).expect("write synthetic target");
        symlink(&target, &link).expect("create destination symlink");

        assert_eq!(
            reset_settings_to_defaults(&link, true),
            Err(SettingsStorageError::UnsafeDestination)
        );
        assert_eq!(fs::read(target).expect("target remains unchanged"), prior);
        assert!(
            fs::symlink_metadata(link)
                .expect("link remains present")
                .file_type()
                .is_symlink()
        );
    }

    #[cfg(not(unix))]
    #[test]
    fn confirmed_reset_fails_closed_without_changing_files_on_non_unix() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = b"synthetic prior settings";
        fs::write(&path, prior).expect("write synthetic settings");

        assert_eq!(
            reset_settings_to_defaults(&path, true),
            Err(SettingsStorageError::UnsupportedPlatform)
        );
        assert_eq!(fs::read(path).expect("settings remain unchanged"), prior);
    }

    #[test]
    fn initialization_does_not_reset_invalid_configuration_automatically() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = b"{ private malformed settings";
        fs::write(&path, prior).expect("write malformed settings");

        assert!(initialize_settings(&path).is_err());
        assert_eq!(fs::read(path).expect("read preserved settings"), prior);
    }

    #[cfg(unix)]
    fn directory_entry_names(directory: &Path) -> Vec<std::ffi::OsString> {
        let mut names = fs::read_dir(directory)
            .expect("inspect isolated settings directory")
            .map(|entry| {
                entry
                    .expect("read isolated settings directory entry")
                    .file_name()
            })
            .collect::<Vec<_>>();
        names.sort();
        names
    }

    #[cfg(unix)]
    #[derive(Debug, PartialEq, Eq)]
    struct SettingsFileSnapshot {
        bytes: Vec<u8>,
        mode: u32,
        device: u64,
        inode: u64,
        modified: std::time::SystemTime,
        directory_entries: Vec<std::ffi::OsString>,
    }

    #[cfg(unix)]
    fn settings_file_snapshot(path: &Path) -> SettingsFileSnapshot {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let metadata = fs::symlink_metadata(path).expect("inspect settings file snapshot");
        assert!(metadata.file_type().is_file());
        let directory = path.parent().expect("settings path has a parent");

        SettingsFileSnapshot {
            bytes: fs::read(path).expect("read settings file snapshot"),
            mode: metadata.permissions().mode() & 0o777,
            device: metadata.dev(),
            inode: metadata.ino(),
            modified: metadata.modified().expect("read file modification time"),
            directory_entries: directory_entry_names(directory),
        }
    }

    #[cfg(unix)]
    fn assert_unsupported_schema_error(error: &SettingsStorageError, version: u32) {
        assert!(matches!(
            error,
            SettingsStorageError::InvalidSettings(SettingsError::UnsupportedSchemaVersion {
                found
            }) if *found == version
        ));
    }

    #[cfg(unix)]
    fn assert_unsupported_file_is_preserved(
        path: &Path,
        version: u32,
        original: &SettingsFileSnapshot,
    ) {
        let load_error = load_settings(path).expect_err("unsupported schema must fail loading");
        assert_unsupported_schema_error(&load_error, version);
        assert_eq!(&settings_file_snapshot(path), original);

        let initialization_error =
            initialize_settings(path).expect_err("unsupported schema must fail initialization");
        assert_unsupported_schema_error(&initialization_error, version);
        assert_eq!(initialization_error, load_error);
        assert_eq!(&settings_file_snapshot(path), original);

        let repeated_error = initialize_settings(path)
            .expect_err("repeated initialization must keep rejecting unsupported schema");
        assert_unsupported_schema_error(&repeated_error, version);
        assert_eq!(repeated_error, initialization_error);
        assert_eq!(&settings_file_snapshot(path), original);
    }

    #[cfg(unix)]
    #[test]
    fn settings_initialization_preserves_unsupported_schema_files() {
        use crate::{CleanupMode, CleanupSettings, StartupSettings};
        use serde_json::json;

        println!("Outflow settings initialization preservation test\n");

        let missing_directory = TestDirectory::new();
        let missing_path = missing_directory.path().join("settings.json");
        assert!(!missing_path.exists());
        assert_eq!(load_settings(&missing_path), Ok(None));
        println!("Missing settings:\n  Loader returned Ok(None): PASS");

        assert_eq!(
            initialize_settings(&missing_path),
            Ok(SettingsInitialization::FirstRun(Settings::default()))
        );
        println!("  In-memory defaults returned: PASS");
        assert!(!missing_path.exists());
        assert!(directory_entry_names(missing_directory.path()).is_empty());
        println!("  Settings file created: NO");

        let supported_directory = TestDirectory::new();
        let supported_path = supported_directory.path().join("settings.json");
        let stored_settings = Settings {
            cleanup: CleanupSettings {
                mode: CleanupMode::Verbatim,
                remove_fillers: false,
            },
            startup: StartupSettings { run_at_login: true },
            ..Settings::default()
        };
        assert_ne!(stored_settings, Settings::default());
        save_settings_atomic(&supported_path, &stored_settings)
            .expect("write synthetic supported schema-v1 settings");
        let supported_bytes = fs::read(&supported_path).expect("capture schema-v1 bytes");
        let supported_entries = directory_entry_names(supported_directory.path());
        assert_eq!(
            supported_entries,
            [supported_path.file_name().unwrap().to_owned()]
        );

        assert_eq!(
            initialize_settings(&supported_path),
            Ok(SettingsInitialization::Loaded(stored_settings.clone()))
        );
        println!("\nValid schema v1:\n  Existing settings loaded: PASS");
        assert_ne!(stored_settings, Settings::default());
        assert_eq!(stored_settings.cleanup.mode, CleanupMode::Verbatim);
        assert!(!stored_settings.cleanup.remove_fillers);
        assert!(stored_settings.startup.run_at_login);
        println!("  Stored values preserved: PASS");
        assert_eq!(
            fs::read(&supported_path).expect("read unchanged schema-v1 file"),
            supported_bytes
        );
        assert_eq!(
            directory_entry_names(supported_directory.path()),
            supported_entries
        );
        println!("  File bytes unchanged: PASS");

        let schema_v1_document: serde_json::Value = serde_json::from_str(
            &Settings::default()
                .to_json()
                .expect("encode supported schema-v1 fixture"),
        )
        .expect("decode supported schema-v1 fixture");
        let mut schema_v2_document = schema_v1_document;
        schema_v2_document["schema_version"] = json!(2);
        let schema_v2_bytes =
            serde_json::to_vec(&schema_v2_document).expect("encode unsupported schema-v2 fixture");
        serde_json::from_slice::<serde_json::Value>(&schema_v2_bytes)
            .expect("schema-v2 fixture is syntactically valid JSON");

        let schema_v2_directory = TestDirectory::new();
        let schema_v2_path = schema_v2_directory.path().join("settings.json");
        fs::write(&schema_v2_path, &schema_v2_bytes)
            .expect("write synthetic unsupported schema-v2 fixture");
        let schema_v2_snapshot = settings_file_snapshot(&schema_v2_path);
        assert_eq!(schema_v2_snapshot.bytes, schema_v2_bytes);
        assert_eq!(
            schema_v2_snapshot.directory_entries,
            [schema_v2_path.file_name().unwrap().to_owned()]
        );
        assert_unsupported_file_is_preserved(&schema_v2_path, 2, &schema_v2_snapshot);
        println!(
            "\nUnsupported schema v2:\n  UnsupportedSchemaVersion returned: PASS\n  Original bytes preserved: PASS\n  Repeated initialization non-destructive: PASS"
        );

        let future_directory = TestDirectory::new();
        let future_path = future_directory.path().join("settings.json");
        let future_bytes = br#"{"schema_version":999}"#;
        serde_json::from_slice::<serde_json::Value>(future_bytes)
            .expect("minimal future-schema fixture is syntactically valid JSON");
        fs::write(&future_path, future_bytes)
            .expect("write synthetic unsupported future-schema fixture");
        let future_snapshot = settings_file_snapshot(&future_path);
        assert_eq!(future_snapshot.bytes, future_bytes);
        assert_eq!(
            future_snapshot.directory_entries,
            [future_path.file_name().unwrap().to_owned()]
        );
        assert_unsupported_file_is_preserved(&future_path, 999, &future_snapshot);
        println!(
            "\nUnsupported future schema:\n  UnsupportedSchemaVersion returned: PASS\n  Original bytes preserved: PASS"
        );

        println!("\nAutomatic downgrade: NO");
        println!("Automatic reset: NO");
        println!("Schema migration: NO");
        println!("Real user settings accessed: NO");
    }

    #[cfg(unix)]
    fn create_valid_settings_file(
        directory: &TestDirectory,
        settings: &Settings,
    ) -> (PathBuf, Vec<u8>) {
        use std::os::unix::fs::PermissionsExt;

        let path = directory.path().join("settings.json");
        save_settings_atomic(&path, settings).expect("write synthetic schema-v1 settings");
        assert_eq!(
            load_settings(&path),
            Ok(Some(settings.clone())),
            "the valid baseline must load before the test corrupts it"
        );
        assert_eq!(
            fs::metadata(&path)
                .expect("inspect synthetic settings permissions")
                .permissions()
                .mode()
                & 0o777,
            0o600,
            "the existing writer should apply its Unix owner-only mode"
        );
        let original_bytes = fs::read(&path).expect("capture valid baseline bytes");
        (path, original_bytes)
    }

    #[cfg(unix)]
    fn assert_corrupted_file_is_rejected_without_mutation(path: &Path, corrupted_bytes: &[u8]) {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        const PRIVATE_MARKERS: [&str; 3] = [
            "TASK62_SYNTHETIC_VOCAB_ID_91A7",
            "TASK62_SYNTHETIC_SPOKEN_PHRASE_91A7",
            "TASK62_SYNTHETIC_WRITTEN_PHRASE_91A7",
        ];

        // Corruption is deliberately performed by test setup, never by the loader.
        fs::write(path, corrupted_bytes).expect("write synthetic corrupted settings fixture");
        let bytes_before_load = fs::read(path).expect("capture exact corrupted bytes");
        assert_eq!(bytes_before_load, corrupted_bytes);
        let metadata_before_load = fs::metadata(path).expect("inspect corrupted settings file");
        let mode_before_load = metadata_before_load.permissions().mode() & 0o777;
        let identity_before_load = (metadata_before_load.dev(), metadata_before_load.ino());
        let modified_before_load = metadata_before_load
            .modified()
            .expect("read corrupted settings modification time");
        let directory = path.parent().expect("settings path has a parent");
        let entries_before_load = directory_entry_names(directory);
        assert_eq!(entries_before_load, [path.file_name().unwrap().to_owned()]);

        for _ in 0..2 {
            let error = load_settings(path)
                .expect_err("invalid existing settings must not load as settings or missing");
            assert!(
                matches!(&error, SettingsStorageError::InvalidDocument { .. }),
                "syntactically or structurally invalid JSON must retain its parse classification"
            );
            assert_eq!(
                initialize_settings(path),
                Err(error.clone()),
                "initialization must propagate corruption instead of substituting defaults"
            );

            let display = error.to_string();
            let debug = format!("{error:?}");
            for marker in PRIVATE_MARKERS {
                assert!(!display.contains(marker));
                assert!(!debug.contains(marker));
            }
            if !corrupted_bytes.is_empty() {
                let raw_json = std::str::from_utf8(corrupted_bytes)
                    .expect("all synthetic corruption fixtures are UTF-8");
                assert!(!display.contains(raw_json));
                assert!(!debug.contains(raw_json));
            }

            let metadata_after_load =
                fs::symlink_metadata(path).expect("failed load leaves the settings entry present");
            assert!(metadata_after_load.is_file());
            assert_eq!(
                fs::read(path).expect("read unchanged corrupted settings"),
                corrupted_bytes
            );
            assert_eq!(
                metadata_after_load.permissions().mode() & 0o777,
                mode_before_load,
                "failed load must preserve the existing file permissions"
            );
            assert_eq!(
                (metadata_after_load.dev(), metadata_after_load.ino()),
                identity_before_load,
                "failed load must not replace the existing file"
            );
            assert_eq!(
                metadata_after_load
                    .modified()
                    .expect("read unchanged settings modification time"),
                modified_before_load,
                "failed load must not rewrite the existing file"
            );
            assert_eq!(directory_entry_names(directory), entries_before_load);
        }
    }

    #[cfg(unix)]
    #[test]
    fn corrupted_settings_files_are_rejected_and_preserved_without_automatic_reset() {
        use serde_json::json;

        let settings = Settings {
            vocabulary: vec![crate::VocabularyEntry {
                id: "TASK62_SYNTHETIC_VOCAB_ID_91A7".to_owned(),
                spoken: "TASK62_SYNTHETIC_SPOKEN_PHRASE_91A7".to_owned(),
                written: "TASK62_SYNTHETIC_WRITTEN_PHRASE_91A7".to_owned(),
            }],
            ..Settings::default()
        };

        println!("Outflow corrupted settings preservation test\n");

        let truncated_directory = TestDirectory::new();
        let (truncated_path, valid_bytes) =
            create_valid_settings_file(&truncated_directory, &settings);
        println!("Valid schema-v1 settings baseline: PASS");
        let truncated_bytes = valid_bytes[..valid_bytes.len() - 1].to_vec();
        assert_corrupted_file_is_rejected_without_mutation(&truncated_path, &truncated_bytes);
        println!("Truncated JSON: REJECTED");

        let malformed_directory = TestDirectory::new();
        let (malformed_path, _) = create_valid_settings_file(&malformed_directory, &settings);
        let malformed_bytes = format!(
            r#"{{"spoken":"{}","written":"{}",]}}"#,
            settings.vocabulary[0].spoken, settings.vocabulary[0].written
        )
        .into_bytes();
        assert_corrupted_file_is_rejected_without_mutation(&malformed_path, &malformed_bytes);
        println!("Malformed JSON: REJECTED");

        let empty_directory = TestDirectory::new();
        let (empty_path, _) = create_valid_settings_file(&empty_directory, &settings);
        assert_corrupted_file_is_rejected_without_mutation(&empty_path, b"");
        println!("Empty existing file: REJECTED");

        let structural_directory = TestDirectory::new();
        let (structural_path, structural_baseline) =
            create_valid_settings_file(&structural_directory, &settings);
        let mut structural_document: serde_json::Value =
            serde_json::from_slice(&structural_baseline).expect("decode valid schema-v1 baseline");
        structural_document["language"] = json!({"private": "TASK62_SYNTHETIC_SPOKEN_PHRASE_91A7"});
        let structural_bytes =
            serde_json::to_vec(&structural_document).expect("encode structurally invalid JSON");
        serde_json::from_slice::<serde_json::Value>(&structural_bytes)
            .expect("structurally invalid fixture remains syntactically valid JSON");
        assert_corrupted_file_is_rejected_without_mutation(&structural_path, &structural_bytes);
        println!("Structurally invalid JSON: REJECTED");

        let missing_directory = TestDirectory::new();
        let missing_path = missing_directory.path().join("missing-settings.json");
        let entries_before_missing_load = directory_entry_names(missing_directory.path());
        assert_eq!(load_settings(&missing_path), Ok(None));
        assert_eq!(
            initialize_settings(&missing_path),
            Ok(crate::SettingsInitialization::FirstRun(Settings::default()))
        );
        assert!(!missing_path.exists());
        assert_eq!(
            directory_entry_names(missing_directory.path()),
            entries_before_missing_load
        );
        println!("Missing file distinction: VERIFIED");

        println!("\nCorrupted file bytes: PRESERVED");
        println!("Corrupted file permissions: PRESERVED");
        println!("Repeated failed loads: NON-DESTRUCTIVE");
        println!("Unexpected temporary artifacts: NONE");
        println!("Automatic reset performed: NO");
        println!("Original vocabulary recovered automatically: NO");
        println!("\nReal user settings accessed: NO");
        println!("Production schema-v2 loading: NOT IMPLEMENTED");
        println!("Automatic corruption repair: NOT IMPLEMENTED");
    }
}

/// Replaces an existing settings file with validated schema v1 defaults after
/// the caller explicitly confirms the reset.
///
/// `confirmed` represents caller intent only. A future native UI must obtain
/// the user's confirmation before invoking this function. When false, this
/// function returns [`SettingsStorageError::ResetNotConfirmed`] without
/// inspecting or changing the filesystem. When true, `path` must name an
/// existing regular file and must not be a symbolic link. The file's current
/// contents are not read, so malformed JSON and unsupported schema versions
/// can be deliberately replaced. The replacement uses
/// [`save_settings_atomic`], including its Unix owner-only mode and durability
/// behavior; non-Unix writes continue to fail closed.
///
/// Reset removes Outflow's previously stored settings and vocabulary from the
/// replaced file. Filesystem snapshots and operating-system backups may still
/// retain older data, so this operation does not claim secure erasure. It does
/// not create backup copies or automatically retry a result whose replacement
/// durability is uncertain.
///
/// # Errors
///
/// Returns a redacted error if confirmation is absent, the destination is
/// missing or unsafe, defaults fail validation, replacement fails, or the
/// platform lacks the existing atomic writer guarantees. If
/// [`SettingsStorageError::ReplacementDurabilityUncertain`] is returned, the
/// replacement is already visible but crash durability could not be confirmed;
/// callers must not automatically retry it.
pub fn reset_settings_to_defaults(
    path: &Path,
    confirmed: bool,
) -> Result<(), SettingsStorageError> {
    reset_settings_to_defaults_with_hooks(
        path,
        confirmed,
        |file, json| file.write_all(json),
        File::sync_all,
        || Ok(()),
        File::sync_all,
    )
}

fn reset_settings_to_defaults_with_hooks<W, S, B, P>(
    path: &Path,
    confirmed: bool,
    write_json: W,
    synchronize_file: S,
    before_replace: B,
    synchronize_parent: P,
) -> Result<(), SettingsStorageError>
where
    W: FnOnce(&mut File, &[u8]) -> io::Result<()>,
    S: FnOnce(&File) -> io::Result<()>,
    B: FnOnce() -> io::Result<()>,
    P: FnOnce(&File) -> io::Result<()>,
{
    if !confirmed {
        return Err(SettingsStorageError::ResetNotConfirmed);
    }

    // Reset must never turn a missing configuration into a newly created file.
    // The writer checks this condition again immediately before replacement.
    inspect_destination(path, true)?;

    let defaults = Settings::default();
    defaults
        .validate()
        .map_err(SettingsStorageError::InvalidSettings)?;

    save_settings_atomic_with_destination_policy(
        path,
        &defaults,
        true,
        write_json,
        synchronize_file,
        before_replace,
        synchronize_parent,
    )
}

/// Validates and atomically writes schema v1 settings to `path`.
///
/// The path is supplied by the caller; no configuration directory is
/// discovered. On Unix, the temporary file is created with owner-only mode
/// `0600`, fully written, flushed, synchronized, and renamed from the same
/// directory. The destination is never removed before the rename. A failure
/// before replacement leaves the prior destination untouched. A failure to
/// synchronize the parent directory is reported separately because the new
/// file is already visible at that point. A `ReplacementFailed` error reports
/// an error from the replacement operation but does not, by itself, establish
/// the destination state for every OS-level failure. Do not treat an arbitrary
/// writer error as proof that the old settings remain active. A
/// `ReplacementDurabilityUncertain` error means replacement completed but
/// durable persistence could not be confirmed; callers must not report success,
/// claim the old settings were restored, or automatically overwrite the path
/// with stale settings.
///
/// Symbolic-link destinations and existing non-regular destinations are
/// rejected. The same-directory rename does not follow a destination symlink.
/// Windows and other non-Unix targets return [`SettingsStorageError::UnsupportedPlatform`]
/// until the project approves and implements owner-only ACL handling there.
///
/// # Errors
///
/// Returns a redacted error for invalid settings, unsafe paths, filesystem
/// failures, or unsupported platform guarantees. Error text never includes the
/// path, serialized settings, vocabulary, or shortcut contents.
pub fn save_settings_atomic(path: &Path, settings: &Settings) -> Result<(), SettingsStorageError> {
    save_settings_atomic_with_hooks(
        path,
        settings,
        |file, json| file.write_all(json),
        File::sync_all,
        || Ok(()),
        File::sync_all,
    )
}

pub(crate) fn save_settings_atomic_with_hooks<W, S, B, P>(
    path: &Path,
    settings: &Settings,
    write_json: W,
    synchronize_file: S,
    before_replace: B,
    synchronize_parent: P,
) -> Result<(), SettingsStorageError>
where
    W: FnOnce(&mut File, &[u8]) -> io::Result<()>,
    S: FnOnce(&File) -> io::Result<()>,
    B: FnOnce() -> io::Result<()>,
    P: FnOnce(&File) -> io::Result<()>,
{
    save_settings_atomic_with_destination_policy(
        path,
        settings,
        false,
        write_json,
        synchronize_file,
        before_replace,
        synchronize_parent,
    )
}

fn save_settings_atomic_with_destination_policy<W, S, B, P>(
    path: &Path,
    settings: &Settings,
    require_existing_destination: bool,
    write_json: W,
    synchronize_file: S,
    before_replace: B,
    synchronize_parent: P,
) -> Result<(), SettingsStorageError>
where
    W: FnOnce(&mut File, &[u8]) -> io::Result<()>,
    S: FnOnce(&File) -> io::Result<()>,
    B: FnOnce() -> io::Result<()>,
    P: FnOnce(&File) -> io::Result<()>,
{
    save_settings_atomic_with_destination_policy_and_creation(
        path,
        settings,
        require_existing_destination,
        TemporaryFileOperations {
            create_file: create_temporary_file_exclusively,
            write_json,
        },
        synchronize_file,
        before_replace,
        synchronize_parent,
    )
}

#[cfg(test)]
fn save_settings_atomic_with_creation_hook<C, W, S, B, P>(
    path: &Path,
    settings: &Settings,
    create_temporary_file: C,
    write_json: W,
    synchronize_file: S,
    before_replace: B,
    synchronize_parent: P,
) -> Result<(), SettingsStorageError>
where
    C: FnMut(&Path) -> io::Result<File>,
    W: FnOnce(&mut File, &[u8]) -> io::Result<()>,
    S: FnOnce(&File) -> io::Result<()>,
    B: FnOnce() -> io::Result<()>,
    P: FnOnce(&File) -> io::Result<()>,
{
    save_settings_atomic_with_destination_policy_and_creation(
        path,
        settings,
        false,
        TemporaryFileOperations {
            create_file: create_temporary_file,
            write_json,
        },
        synchronize_file,
        before_replace,
        synchronize_parent,
    )
}

#[cfg(test)]
fn save_settings_atomic_with_replacement_hook<W, S, B, R, P>(
    path: &Path,
    settings: &Settings,
    write_json: W,
    synchronize_file: S,
    before_replace: B,
    replace_destination: R,
    synchronize_parent: P,
) -> Result<(), SettingsStorageError>
where
    W: FnOnce(&mut File, &[u8]) -> io::Result<()>,
    S: FnOnce(&File) -> io::Result<()>,
    B: FnOnce() -> io::Result<()>,
    R: FnOnce(&Path, &Path) -> io::Result<()>,
    P: FnOnce(&File) -> io::Result<()>,
{
    save_settings_atomic_with_replacement_operation(
        path,
        settings,
        false,
        TemporaryFileOperations {
            create_file: create_temporary_file_exclusively,
            write_json,
        },
        synchronize_file,
        DestinationReplacementOperations {
            before_replace,
            replace_destination,
        },
        synchronize_parent,
    )
}

struct TemporaryFileOperations<C, W> {
    create_file: C,
    write_json: W,
}

// Keep Task 55's pre-operation interruption distinct from an error returned
// by the replacement operation itself; they protect different failure stages.
struct DestinationReplacementOperations<B, R> {
    before_replace: B,
    replace_destination: R,
}

fn save_settings_atomic_with_destination_policy_and_creation<C, W, S, B, P>(
    path: &Path,
    settings: &Settings,
    require_existing_destination: bool,
    temporary_file_operations: TemporaryFileOperations<C, W>,
    synchronize_file: S,
    before_replace: B,
    synchronize_parent: P,
) -> Result<(), SettingsStorageError>
where
    C: FnMut(&Path) -> io::Result<File>,
    W: FnOnce(&mut File, &[u8]) -> io::Result<()>,
    S: FnOnce(&File) -> io::Result<()>,
    B: FnOnce() -> io::Result<()>,
    P: FnOnce(&File) -> io::Result<()>,
{
    save_settings_atomic_with_replacement_operation(
        path,
        settings,
        require_existing_destination,
        temporary_file_operations,
        synchronize_file,
        DestinationReplacementOperations {
            before_replace,
            replace_destination: |source: &Path, destination: &Path| {
                fs::rename(source, destination)
            },
        },
        synchronize_parent,
    )
}

fn save_settings_atomic_with_replacement_operation<C, W, S, B, R, P>(
    path: &Path,
    settings: &Settings,
    require_existing_destination: bool,
    temporary_file_operations: TemporaryFileOperations<C, W>,
    synchronize_file: S,
    destination_replacement: DestinationReplacementOperations<B, R>,
    synchronize_parent: P,
) -> Result<(), SettingsStorageError>
where
    C: FnMut(&Path) -> io::Result<File>,
    W: FnOnce(&mut File, &[u8]) -> io::Result<()>,
    S: FnOnce(&File) -> io::Result<()>,
    B: FnOnce() -> io::Result<()>,
    R: FnOnce(&Path, &Path) -> io::Result<()>,
    P: FnOnce(&File) -> io::Result<()>,
{
    settings
        .validate()
        .map_err(SettingsStorageError::InvalidSettings)?;
    let json = settings.to_json().map_err(map_json_error)?;

    #[cfg(unix)]
    {
        save_json_unix(
            path,
            json.as_bytes(),
            require_existing_destination,
            temporary_file_operations,
            synchronize_file,
            destination_replacement,
            synchronize_parent,
        )
    }

    #[cfg(not(unix))]
    {
        let _ = (
            path,
            json,
            require_existing_destination,
            temporary_file_operations,
            synchronize_file,
            destination_replacement,
            synchronize_parent,
        );
        Err(SettingsStorageError::UnsupportedPlatform)
    }
}

fn create_temporary_file_exclusively(path: &Path) -> io::Result<File> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
    }

    #[cfg(not(unix))]
    {
        let _ = path;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "atomic settings writing is unsupported on this platform",
        ))
    }
}

fn map_json_error(error: SettingsJsonError) -> SettingsStorageError {
    match error {
        SettingsJsonError::InvalidSettings(error) => SettingsStorageError::InvalidSettings(error),
        SettingsJsonError::InvalidDocument { .. } | SettingsJsonError::SerializationFailed => {
            SettingsStorageError::SerializationFailed
        }
    }
}

#[cfg(unix)]
fn save_json_unix<C, W, S, B, R, P>(
    path: &Path,
    json: &[u8],
    require_existing_destination: bool,
    temporary_file_operations: TemporaryFileOperations<C, W>,
    synchronize_file: S,
    destination_replacement: DestinationReplacementOperations<B, R>,
    synchronize_parent: P,
) -> Result<(), SettingsStorageError>
where
    C: FnMut(&Path) -> io::Result<File>,
    W: FnOnce(&mut File, &[u8]) -> io::Result<()>,
    S: FnOnce(&File) -> io::Result<()>,
    B: FnOnce() -> io::Result<()>,
    R: FnOnce(&Path, &Path) -> io::Result<()>,
    P: FnOnce(&File) -> io::Result<()>,
{
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::time::{SystemTime, UNIX_EPOCH};
    let TemporaryFileOperations {
        mut create_file,
        write_json,
    } = temporary_file_operations;
    let DestinationReplacementOperations {
        before_replace,
        replace_destination,
    } = destination_replacement;

    if path.file_name().filter(|name| !name.is_empty()).is_none() {
        return Err(SettingsStorageError::InvalidDestinationPath);
    }

    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let parent_metadata = fs::symlink_metadata(parent).map_err(map_parent_error)?;
    if parent_metadata.file_type().is_symlink() || !parent_metadata.is_dir() {
        return Err(SettingsStorageError::InvalidParentDirectory);
    }

    let parent_directory = File::open(parent).map_err(|error| {
        map_io_error(
            error,
            SettingsStorageError::InvalidParentDirectory,
            SettingsStorageOperation::OpenParentDirectory,
        )
    })?;

    inspect_destination(path, require_existing_destination)?;

    // Time and process identifiers make names hard to collide in practice;
    // create_new is the actual exclusivity guarantee and retries collisions.
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut temporary = None;
    for attempt in 0..128_u32 {
        let temporary_path = parent.join(format!(
            ".outflow-settings-tmp-{}-{timestamp}-{attempt}",
            std::process::id()
        ));
        // The test-only seam replaces this operation alone; path generation,
        // collision handling, and the creation error mapping stay in this path.
        let opened = create_file(&temporary_path);
        match opened {
            Ok(file) => {
                temporary = Some((temporary_path, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(map_io_error(
                    error,
                    SettingsStorageError::TemporaryFileCreationFailed,
                    SettingsStorageOperation::CreateTemporaryFile,
                ));
            }
        }
    }
    let (temporary_path, mut temporary_file) =
        temporary.ok_or(SettingsStorageError::TemporaryFileCreationFailed)?;
    let mut temporary_guard = TemporaryFileGuard::new(temporary_path);

    let result = (|| {
        temporary_file
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|error| {
                map_io_error(
                    error,
                    SettingsStorageError::TemporaryFilePermissionsFailed,
                    SettingsStorageOperation::SetTemporaryFilePermissions,
                )
            })?;

        write_json(&mut temporary_file, json).map_err(|error| {
            map_io_error(
                error,
                SettingsStorageError::TemporaryFileWriteFailed,
                SettingsStorageOperation::WriteTemporaryFile,
            )
        })?;
        temporary_file.flush().map_err(|error| {
            map_io_error(
                error,
                SettingsStorageError::TemporaryFileWriteFailed,
                SettingsStorageOperation::WriteTemporaryFile,
            )
        })?;
        synchronize_file(&temporary_file).map_err(|error| {
            map_io_error(
                error,
                SettingsStorageError::TemporaryFileSynchronizationFailed,
                SettingsStorageOperation::SynchronizeTemporaryFile,
            )
        })?;

        // Re-check immediately before rename. On Unix rename replaces a
        // destination symlink entry itself rather than following its target.
        inspect_destination(path, require_existing_destination)?;
        let current_parent_metadata = fs::symlink_metadata(parent).map_err(map_parent_error)?;
        let opened_parent_metadata = parent_directory.metadata().map_err(|error| {
            map_io_error(
                error,
                SettingsStorageError::InvalidParentDirectory,
                SettingsStorageOperation::OpenParentDirectory,
            )
        })?;
        if current_parent_metadata.file_type().is_symlink()
            || !current_parent_metadata.is_dir()
            || current_parent_metadata.dev() != opened_parent_metadata.dev()
            || current_parent_metadata.ino() != opened_parent_metadata.ino()
        {
            return Err(SettingsStorageError::InvalidParentDirectory);
        }

        // Keep the private fault seam at the commit boundary; production
        // supplies a no-op after the final path checks.
        before_replace().map_err(|error| {
            map_io_error(
                error,
                SettingsStorageError::ReplacementFailed,
                SettingsStorageOperation::ReplaceDestination,
            )
        })?;

        replace_destination(temporary_guard.path(), path).map_err(|error| {
            map_io_error(
                error,
                SettingsStorageError::ReplacementFailed,
                SettingsStorageOperation::ReplaceDestination,
            )
        })?;
        temporary_guard.disarm();

        synchronize_parent(&parent_directory)
            .map_err(|_| SettingsStorageError::ReplacementDurabilityUncertain)
    })();

    if result.is_err() && temporary_guard.is_armed() {
        temporary_guard
            .remove()
            .map_err(|_| SettingsStorageError::TemporaryFileWriteFailed)?;
    }
    result
}

fn inspect_destination(path: &Path, require_existing: bool) -> Result<(), SettingsStorageError> {
    if path.file_name().filter(|name| !name.is_empty()).is_none() {
        return Err(SettingsStorageError::InvalidDestinationPath);
    }

    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(SettingsStorageError::UnsafeDestination)
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound && require_existing => {
            Err(SettingsStorageError::DestinationMissing)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            Err(SettingsStorageError::PermissionDenied {
                operation: SettingsStorageOperation::InspectDestination,
            })
        }
        // Keep the existing writer's redacted fallback category for metadata
        // errors that do not provide a safe, more specific classification.
        Err(_) => Err(SettingsStorageError::UnsafeDestination),
    }
}

#[cfg(unix)]
fn map_parent_error(error: io::Error) -> SettingsStorageError {
    match error.kind() {
        io::ErrorKind::NotFound => SettingsStorageError::MissingParentDirectory,
        io::ErrorKind::PermissionDenied => SettingsStorageError::PermissionDenied {
            operation: SettingsStorageOperation::OpenParentDirectory,
        },
        io::ErrorKind::NotADirectory => SettingsStorageError::InvalidParentDirectory,
        _ => SettingsStorageError::InvalidParentDirectory,
    }
}

#[cfg(unix)]
fn map_io_error(
    error: io::Error,
    otherwise: SettingsStorageError,
    operation: SettingsStorageOperation,
) -> SettingsStorageError {
    if error.kind() == io::ErrorKind::PermissionDenied {
        SettingsStorageError::PermissionDenied { operation }
    } else {
        otherwise
    }
}

#[cfg(unix)]
struct TemporaryFileGuard {
    path: PathBuf,
    armed: bool,
}

#[cfg(unix)]
impl TemporaryFileGuard {
    fn new(path: PathBuf) -> Self {
        Self { path, armed: true }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn is_armed(&self) -> bool {
        self.armed
    }

    fn disarm(&mut self) {
        self.armed = false;
    }

    fn remove(&mut self) -> io::Result<()> {
        if self.armed {
            match fs::remove_file(&self.path) {
                Ok(()) => self.armed = false,
                Err(error) if error.kind() == io::ErrorKind::NotFound => self.armed = false,
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
impl Drop for TemporaryFileGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::{
        SettingsStorageError, SettingsStorageOperation, reset_settings_to_defaults,
        reset_settings_to_defaults_with_hooks, save_settings_atomic,
        save_settings_atomic_with_creation_hook, save_settings_atomic_with_hooks,
        save_settings_atomic_with_replacement_hook,
    };
    use crate::{Settings, SettingsError, VocabularyEntry, load_settings};
    use std::cell::{Cell, RefCell};
    use std::fs::{self, File};
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            for attempt in 0..128_u32 {
                let path = std::env::temp_dir().join(format!(
                    "outflow-settings-test-{}-{timestamp}-{attempt}",
                    std::process::id()
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("create isolated test directory: {error}"),
                }
            }
            panic!("could not create a unique isolated test directory");
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn temporary_files(directory: &Path) -> Vec<PathBuf> {
        fs::read_dir(directory)
            .expect("read isolated test directory")
            .map(|entry| entry.expect("read directory entry").path())
            .filter(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with(".outflow-settings-tmp-"))
            })
            .collect()
    }

    fn directory_entries(directory: &Path) -> Vec<PathBuf> {
        let mut entries = fs::read_dir(directory)
            .expect("read isolated test directory")
            .map(|entry| entry.expect("read directory entry").path())
            .collect::<Vec<_>>();
        entries.sort();
        entries
    }

    fn valid_settings() -> Settings {
        Settings {
            vocabulary: vec![VocabularyEntry {
                id: "synthetic-test-entry".to_owned(),
                spoken: "synthetic phrase".to_owned(),
                written: "Synthetic Phrase".to_owned(),
            }],
            ..Settings::default()
        }
    }

    #[test]
    fn valid_settings_write_as_schema_v1_and_decode_back() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let original = valid_settings();

        save_settings_atomic(&path, &original).expect("valid settings save");

        let json = fs::read_to_string(path).expect("read written JSON");
        let value: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(value["schema_version"], 1);
        assert_eq!(Settings::from_json(&json).expect("decode JSON"), original);
    }

    #[test]
    fn existing_settings_are_replaced_atomically() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        fs::write(&path, "old settings").expect("write prior settings");
        let settings = valid_settings();

        save_settings_atomic(&path, &settings).expect("replace existing settings");

        assert_eq!(
            Settings::from_json(&fs::read_to_string(path).expect("read replaced settings"))
                .expect("decode replaced settings"),
            settings
        );
    }

    #[test]
    fn invalid_settings_do_not_modify_existing_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = b"preserve prior bytes";
        fs::write(&path, prior).expect("write prior settings");
        let invalid = Settings {
            vocabulary: vec![VocabularyEntry {
                id: "private-id-8391".to_owned(),
                spoken: "".to_owned(),
                written: "private phrase 8391".to_owned(),
            }],
            ..Settings::default()
        };

        let error = save_settings_atomic(&path, &invalid).expect_err("invalid values rejected");

        assert!(matches!(error, SettingsStorageError::InvalidSettings(_)));
        assert_eq!(fs::read(path).expect("read prior file"), prior);
        assert!(temporary_files(directory.path()).is_empty());
        assert!(!error.to_string().contains("private-id-8391"));
        assert!(!error.to_string().contains("private phrase 8391"));
    }

    #[test]
    fn unsupported_schema_version_cannot_be_written() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = b"prior configuration";
        fs::write(&path, prior).expect("write prior settings");
        let unsupported = Settings {
            schema_version: 2,
            ..Settings::default()
        };

        let error = save_settings_atomic(&path, &unsupported).expect_err("version rejected");

        assert_eq!(
            error,
            SettingsStorageError::InvalidSettings(SettingsError::UnsupportedSchemaVersion {
                found: 2
            })
        );
        assert_eq!(fs::read(path).expect("read prior file"), prior);
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn simulated_partial_write_failure_preserves_file_and_cleans_temporary_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = b"prior configuration";
        fs::write(&path, prior).expect("write prior settings");

        let error = save_settings_atomic_with_hooks(
            &path,
            &valid_settings(),
            |file, json| {
                let partial_length = json.len().min(8);
                file.write_all(&json[..partial_length])?;
                Err(io::Error::other("synthetic disk full after partial write"))
            },
            File::sync_all,
            || Ok(()),
            File::sync_all,
        )
        .expect_err("simulated disk-full failure");

        assert_eq!(error, SettingsStorageError::TemporaryFileWriteFailed);
        assert_eq!(fs::read(path).expect("read prior file"), prior);
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn permission_denial_is_reported_without_disclosing_os_details() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = b"prior configuration";
        fs::write(&path, prior).expect("write prior settings");

        let error = save_settings_atomic_with_hooks(
            &path,
            &valid_settings(),
            |_, _| Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            |file| file.sync_all(),
            || Ok(()),
            |directory| directory.sync_all(),
        )
        .expect_err("simulated permission denial");

        assert_eq!(
            error,
            SettingsStorageError::PermissionDenied {
                operation: SettingsStorageOperation::WriteTemporaryFile
            }
        );
        assert_eq!(fs::read(path).expect("read prior file"), prior);
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn simulated_file_sync_failure_preserves_file_and_cleans_temporary_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = b"prior configuration";
        fs::write(&path, prior).expect("write prior settings");

        let error = save_settings_atomic_with_hooks(
            &path,
            &valid_settings(),
            |file, json| file.write_all(json),
            |_| Err(io::Error::other("synthetic file sync failure")),
            || Ok(()),
            |directory| directory.sync_all(),
        )
        .expect_err("simulated file sync failure");

        assert_eq!(
            error,
            SettingsStorageError::TemporaryFileSynchronizationFailed
        );
        assert_eq!(fs::read(path).expect("read prior file"), prior);
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn simulated_pre_replacement_failure_preserves_file_and_cleans_temporary_file() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = b"prior configuration";
        fs::write(&path, prior).expect("write prior settings");

        let error = save_settings_atomic_with_hooks(
            &path,
            &valid_settings(),
            |file, json| file.write_all(json),
            File::sync_all,
            || Err(io::Error::other("synthetic pre-replacement interruption")),
            File::sync_all,
        )
        .expect_err("simulated pre-replacement interruption");

        assert_eq!(error, SettingsStorageError::ReplacementFailed);
        assert_eq!(fs::read(path).expect("read prior file"), prior);
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn atomic_writer_pre_replacement_failure_preserves_valid_settings_and_allows_retry() {
        let directory = TestDirectory::new();
        let test_directory_path = directory.path().to_path_buf();
        let path = directory.path().join("settings.json");
        let original = valid_settings();

        save_settings_atomic(&path, &original).expect("write initial settings");

        let loaded_initial = load_settings(&path).expect("load initial settings");
        assert_eq!(loaded_initial.as_ref(), Some(&original));

        let original_bytes = fs::read(&path).expect("read initial settings bytes");
        let original_document: serde_json::Value =
            serde_json::from_slice(&original_bytes).expect("parse initial settings document");
        assert_eq!(original_document["schema_version"], 1);

        let mut replacement = original.clone();
        replacement.vocabulary[0].id = "synthetic-replacement-entry".to_owned();
        replacement.vocabulary[0].spoken = "synthetic replacement phrase".to_owned();
        replacement.vocabulary[0].written = "Synthetic Replacement Phrase".to_owned();
        assert_ne!(replacement, original);

        let failure_injected = Cell::new(false);
        let error = save_settings_atomic_with_hooks(
            &path,
            &replacement,
            |file, json| file.write_all(json),
            File::sync_all,
            || {
                failure_injected.set(true);
                Err(io::Error::other("synthetic pre-replacement failure"))
            },
            File::sync_all,
        )
        .expect_err("injected pre-replacement failure must reject the write");

        assert!(failure_injected.get());

        assert_eq!(error, SettingsStorageError::ReplacementFailed);
        assert!(!error.to_string().contains("synthetic replacement phrase"));

        assert!(
            fs::symlink_metadata(&path)
                .expect("original destination still exists")
                .file_type()
                .is_file()
        );
        assert_eq!(
            fs::read(&path).expect("read destination after failure"),
            original_bytes
        );
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(
            fs::read_dir(directory.path())
                .expect("read isolated directory after failure")
                .count(),
            1,
            "only the original destination should remain"
        );

        let loaded_after_failure = load_settings(&path)
            .expect("reload original settings after failed write")
            .expect("original settings remain available");
        assert_eq!(loaded_after_failure, original);
        assert_eq!(loaded_after_failure.vocabulary, original.vocabulary);

        save_settings_atomic(&path, &replacement).expect("retry normal settings write");
        let loaded_after_retry = load_settings(&path)
            .expect("load settings after normal retry")
            .expect("retried settings remain available");
        assert_eq!(loaded_after_retry, replacement);
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(
            fs::read_dir(directory.path())
                .expect("read isolated directory after retry")
                .count(),
            1,
            "retry should leave only the settings destination"
        );
        drop(directory);
        assert!(
            !test_directory_path.exists(),
            "isolated test directory should be removed"
        );

        println!("Outflow atomic settings writer recovery test\n");
        println!("Initial settings write: PASS");
        println!("Original settings reload: PASS");
        println!("Pre-replacement failure: INJECTED");
        println!("Write rejected: PASS");
        println!("Original bytes preserved: PASS");
        println!("Original settings reload after failure: PASS");
        println!("Subsequent normal write: PASS");

        println!("\nSchema migration performed: NO");
        println!("Real user settings accessed: NO");
        println!("Power-loss recovery verified: NO");
        println!("Windows ACL verification: NOT RUN");
        if cfg!(target_os = "linux") {
            println!("Linux filesystem verification: LOCAL HOST ONLY");
        } else {
            println!("Linux filesystem verification: NOT RUN");
        }
    }

    #[test]
    fn atomic_writer_temporary_write_failure_preserves_valid_settings_and_allows_retry() {
        let directory = TestDirectory::new();
        let test_directory_path = directory.path().to_path_buf();
        let path = directory.path().join("settings.json");
        let original = Settings::default();

        save_settings_atomic(&path, &original).expect("write initial settings");
        let loaded_initial = load_settings(&path).expect("load initial settings");
        assert_eq!(loaded_initial.as_ref(), Some(&original));
        let original_bytes = fs::read(&path).expect("read initial settings bytes");
        let entries_before_failure = fs::read_dir(directory.path())
            .expect("read isolated directory before failure")
            .map(|entry| entry.expect("read directory entry").path())
            .collect::<Vec<_>>();
        assert_eq!(entries_before_failure, vec![path.clone()]);

        let mut replacement = original.clone();
        replacement.cleanup.remove_fillers = !replacement.cleanup.remove_fillers;
        assert_ne!(replacement, original);

        let temporary_created = Cell::new(false);
        let partial_prefix_length = Cell::new(0);
        let partial_content_verified = Cell::new(false);
        let write_failure_injected = Cell::new(false);
        let replacement_attempted = Cell::new(false);
        let error = save_settings_atomic_with_hooks(
            &path,
            &replacement,
            |file, json| {
                let prefix_length = json.len().min(16);
                if prefix_length == 0 {
                    return Err(io::Error::other("empty synthetic settings document"));
                }

                file.write_all(&json[..prefix_length])?;
                file.flush()?;

                let temporary_paths = temporary_files(directory.path());
                if temporary_paths.len() != 1 {
                    return Err(io::Error::other(
                        "expected one temporary settings replacement",
                    ));
                }
                let temporary_metadata = fs::symlink_metadata(&temporary_paths[0])?;
                temporary_created.set(temporary_metadata.file_type().is_file());
                let partial_bytes = fs::read(&temporary_paths[0])?;
                if partial_bytes != json[..prefix_length] {
                    return Err(io::Error::other(
                        "temporary settings prefix did not match serialized input",
                    ));
                }

                partial_prefix_length.set(prefix_length);
                partial_content_verified.set(true);
                write_failure_injected.set(true);
                Err(io::Error::other("synthetic temporary-write failure"))
            },
            File::sync_all,
            || {
                replacement_attempted.set(true);
                Ok(())
            },
            File::sync_all,
        )
        .expect_err("injected temporary-write failure must reject the write");

        assert!(temporary_created.get());
        assert!(partial_prefix_length.get() > 0);
        assert!(partial_content_verified.get());
        assert!(write_failure_injected.get());
        assert!(!replacement_attempted.get());
        assert_eq!(error, SettingsStorageError::TemporaryFileWriteFailed);
        assert!(!error.to_string().contains("schema_version"));

        assert!(
            fs::symlink_metadata(&path)
                .expect("original destination still exists")
                .file_type()
                .is_file()
        );
        assert_eq!(
            fs::read(&path).expect("read destination after failure"),
            original_bytes
        );
        assert!(temporary_files(directory.path()).is_empty());
        let entries_after_failure = fs::read_dir(directory.path())
            .expect("read isolated directory after failure")
            .map(|entry| entry.expect("read directory entry").path())
            .collect::<Vec<_>>();
        assert_eq!(entries_after_failure, entries_before_failure);

        let loaded_after_failure = load_settings(&path)
            .expect("reload original settings after failed write")
            .expect("original settings remain available");
        assert_eq!(loaded_after_failure, original);

        save_settings_atomic(&path, &replacement).expect("retry normal settings write");
        let loaded_after_retry = load_settings(&path)
            .expect("load settings after normal retry")
            .expect("retried settings remain available");
        assert_eq!(loaded_after_retry, replacement);
        assert!(temporary_files(directory.path()).is_empty());
        let entries_after_retry = fs::read_dir(directory.path())
            .expect("read isolated directory after retry")
            .map(|entry| entry.expect("read directory entry").path())
            .collect::<Vec<_>>();
        assert_eq!(entries_after_retry, vec![path.clone()]);

        drop(directory);
        assert!(
            !test_directory_path.exists(),
            "isolated test directory should be removed"
        );

        println!("Outflow atomic settings temporary-write failure test\n");
        println!("Initial settings write: PASS");
        println!("Temporary replacement created: PASS");
        println!("Partial temporary write: VERIFIED");
        println!("Write failure: INJECTED");
        println!("Destination replacement attempted: NO");
        println!("Original destination bytes: PRESERVED");
        println!("Original settings reload: PASS");
        println!("Partial temporary file cleanup: PASS");
        println!("Subsequent normal write: PASS");
        println!("Stale temporary artifacts: NONE");

        println!("\nSchema migration performed: NO");
        println!("Real user settings accessed: NO");
        println!("Actual disk-full failure tested: NO");
        println!("Crash recovery verified: NO");
        println!("Windows ACL verification: NOT RUN");
        println!("Linux filesystem verification: NOT RUN");
    }

    #[test]
    fn atomic_writer_temporary_sync_failure_preserves_valid_settings_and_allows_retry() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let test_directory_path = directory.path().to_path_buf();
        let path = directory.path().join("settings.json");
        let original = valid_settings();

        save_settings_atomic(&path, &original).expect("write initial settings");
        let loaded_initial = load_settings(&path)
            .expect("load initial settings")
            .expect("initial settings exist");
        assert!(
            loaded_initial == original,
            "initial settings differ from the synthetic fixture"
        );

        let original_bytes = fs::read(&path).expect("read initial settings bytes");
        let original_document: serde_json::Value =
            serde_json::from_slice(&original_bytes).expect("parse initial settings document");
        assert_eq!(original_document["schema_version"], 1);
        let original_mode = fs::metadata(&path)
            .expect("inspect initial settings permissions")
            .permissions()
            .mode()
            & 0o777;
        let entries_before_failure = directory_entries(directory.path());
        assert_eq!(entries_before_failure, vec![path.clone()]);

        let mut replacement = original.clone();
        replacement.vocabulary[0].id = "synthetic-sync-replacement-entry".to_owned();
        replacement.vocabulary[0].spoken = "synthetic sync replacement phrase".to_owned();
        replacement.vocabulary[0].written = "Synthetic Sync Replacement Phrase".to_owned();
        assert_ne!(replacement, original);
        let replacement_json = replacement.to_json().expect("encode replacement settings");
        assert_ne!(replacement_json.as_bytes(), original_bytes);

        let write_completed = Cell::new(false);
        let synchronization_attempts = Cell::new(0_u32);
        let complete_temporary_file_verified = Cell::new(false);
        let temporary_path_at_synchronization = RefCell::new(None);
        let replacement_attempted = Cell::new(false);
        let error = save_settings_atomic_with_hooks(
            &path,
            &replacement,
            |file, json| {
                file.write_all(json)?;
                write_completed.set(true);
                Ok(())
            },
            |file| {
                synchronization_attempts.set(synchronization_attempts.get() + 1);
                assert!(
                    write_completed.get(),
                    "complete write callback must precede sync"
                );

                let temporary_paths = temporary_files(directory.path());
                assert_eq!(temporary_paths.len(), 1);
                let temporary_path = temporary_paths[0].clone();
                assert!(
                    fs::symlink_metadata(&temporary_path)
                        .expect("inspect prepared temporary file")
                        .file_type()
                        .is_file()
                );
                let temporary_bytes = fs::read(&temporary_path)
                    .expect("read complete temporary replacement before sync failure");
                assert_eq!(temporary_bytes, replacement_json.as_bytes());
                let temporary_json = std::str::from_utf8(&temporary_bytes)
                    .expect("temporary replacement is valid UTF-8 JSON");
                let decoded_temporary = Settings::from_json(temporary_json)
                    .expect("temporary replacement is valid schema-v1 settings");
                assert_eq!(decoded_temporary, replacement);
                assert_eq!(decoded_temporary.schema_version, 1);

                // The writer invokes this hook only after its explicit flush.
                // Return the fault here instead of calling sync_all so the
                // failure occurs at the actual pre-replacement sync boundary.
                assert!(
                    file.metadata()
                        .expect("inspect open temporary file")
                        .is_file()
                );
                *temporary_path_at_synchronization.borrow_mut() = Some(temporary_path);
                complete_temporary_file_verified.set(true);
                Err(io::Error::other(
                    "synthetic temporary-file synchronization failure",
                ))
            },
            || {
                replacement_attempted.set(true);
                Ok(())
            },
            File::sync_all,
        )
        .expect_err("injected temporary-file sync failure must reject the write");

        assert_eq!(synchronization_attempts.get(), 1);
        assert!(complete_temporary_file_verified.get());
        assert!(!replacement_attempted.get());
        assert_eq!(
            error,
            SettingsStorageError::TemporaryFileSynchronizationFailed
        );
        assert!(
            !error
                .to_string()
                .contains("synthetic sync replacement phrase")
        );

        let temporary_path = temporary_path_at_synchronization
            .borrow()
            .clone()
            .expect("synchronization hook observed prepared temporary file");
        assert!(
            !temporary_path.exists(),
            "failed temporary file must be removed"
        );
        assert!(
            fs::symlink_metadata(&path)
                .expect("original destination still exists")
                .file_type()
                .is_file()
        );
        assert_eq!(
            fs::read(&path).expect("read destination after sync failure"),
            original_bytes
        );
        let mode_after_failure = fs::metadata(&path)
            .expect("inspect preserved settings permissions")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode_after_failure, original_mode);
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), entries_before_failure);

        let loaded_after_failure = load_settings(&path)
            .expect("reload original settings after sync failure")
            .expect("original settings remain available");
        assert_eq!(loaded_after_failure, original);
        assert_eq!(loaded_after_failure.schema_version, 1);
        assert_eq!(loaded_after_failure.vocabulary, original.vocabulary);

        save_settings_atomic(&path, &replacement).expect("retry normal settings write");
        let loaded_after_retry = load_settings(&path)
            .expect("load settings after normal retry")
            .expect("retried settings remain available");
        assert_eq!(loaded_after_retry, replacement);
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), vec![path.clone()]);

        drop(directory);
        assert!(
            !test_directory_path.exists(),
            "isolated test directory should be removed"
        );

        println!("Outflow atomic settings synchronization failure test\n");
        println!("Initial settings write: PASS");
        println!("Temporary replacement creation: PASS");
        println!("Complete temporary write: VERIFIED");
        println!("Temporary-file synchronization failure: INJECTED");
        println!("Writer returned error: PASS");
        println!("Destination replacement attempted: NO");
        println!("Original destination bytes: PRESERVED");
        println!("Original settings reload: PASS");
        println!("Temporary artifact cleanup: PASS");
        println!("Subsequent normal write: PASS");
        println!("New settings reload: PASS");

        println!("\nSchema migration performed: NO");
        println!("Real user settings accessed: NO");
        println!("Actual OS synchronization failure tested: NO");
        println!("Crash durability verified: NO");
    }

    #[test]
    fn atomic_writer_replacement_failure_preserves_valid_settings_and_allows_retry() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let test_directory_path = directory.path().to_path_buf();
        let path = directory.path().join("settings.json");
        let mut original = valid_settings();
        original.vocabulary.push(VocabularyEntry {
            id: "synthetic-second-entry".to_owned(),
            spoken: "original exact spoken form".to_owned(),
            written: "Original Exact Written Form".to_owned(),
        });

        println!("Outflow atomic settings replacement failure test\n");
        save_settings_atomic(&path, &original).expect("write initial settings");
        println!("Initial settings write: PASS");

        let loaded_initial = load_settings(&path)
            .expect("load initial settings")
            .expect("initial settings exist");
        assert_eq!(loaded_initial, original);
        assert_eq!(loaded_initial.schema_version, 1);
        println!("Original settings reload: PASS");

        let original_bytes = fs::read(&path).expect("read initial settings bytes");
        let original_document: serde_json::Value =
            serde_json::from_slice(&original_bytes).expect("parse initial settings document");
        assert_eq!(original_document["schema_version"], 1);
        let original_mode = fs::metadata(&path)
            .expect("inspect initial settings permissions")
            .permissions()
            .mode()
            & 0o777;
        let entries_before_failure = directory_entries(directory.path());
        assert_eq!(entries_before_failure, vec![path.clone()]);

        let mut replacement = original.clone();
        replacement.cleanup.remove_fillers = !replacement.cleanup.remove_fillers;
        replacement.vocabulary[0].id = "synthetic-replacement-id".to_owned();
        replacement.vocabulary[0].spoken = "synthetic replacement spoken value".to_owned();
        replacement.vocabulary[0].written = "Synthetic Replacement Written Value".to_owned();
        assert_ne!(replacement, original);
        let replacement_json = replacement.to_json().expect("encode replacement settings");
        assert_ne!(replacement_json.as_bytes(), original_bytes);

        let write_completed = Cell::new(false);
        let replacement_callback_calls = Cell::new(0_u32);
        let prepared_replacement_verified = Cell::new(false);
        let temporary_file_synchronized = Cell::new(false);
        let replacement_error_injected = Cell::new(false);
        let replacement_succeeded = Cell::new(false);
        let parent_sync_calls = Cell::new(0_u32);
        let observed_temporary_path = RefCell::new(None);

        let error = save_settings_atomic_with_replacement_hook(
            &path,
            &replacement,
            |file, json| {
                file.write_all(json)?;
                file.flush()?;
                write_completed.set(true);

                let temporary_paths = temporary_files(directory.path());
                assert_eq!(temporary_paths.len(), 1);
                let temporary_path = temporary_paths[0].clone();
                assert_eq!(temporary_path.parent(), Some(directory.path()));
                assert!(
                    temporary_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(".outflow-settings-tmp-"))
                );
                let temporary_bytes =
                    fs::read(&temporary_path).expect("read fully written temporary replacement");
                assert!(
                    temporary_bytes == replacement_json.as_bytes(),
                    "temporary bytes differ from the expected replacement serialization"
                );
                let temporary_json = std::str::from_utf8(&temporary_bytes)
                    .expect("temporary replacement is UTF-8 JSON");
                let decoded_temporary = Settings::from_json(temporary_json)
                    .expect("temporary replacement is valid schema-v1 settings");
                assert!(
                    decoded_temporary == replacement,
                    "temporary document differs from the synthetic replacement"
                );
                assert_eq!(decoded_temporary.schema_version, 1);
                *observed_temporary_path.borrow_mut() = Some(temporary_path);
                prepared_replacement_verified.set(true);
                println!("Temporary replacement fully written: VERIFIED");
                Ok(())
            },
            |file| {
                assert!(write_completed.get(), "write and flush must precede sync");
                file.sync_all()?;
                temporary_file_synchronized.set(true);
                println!("Temporary-file synchronization: COMPLETED");
                Ok(())
            },
            || Ok(()),
            |source, destination| {
                replacement_callback_calls.set(replacement_callback_calls.get() + 1);
                assert!(prepared_replacement_verified.get());
                assert!(temporary_file_synchronized.get());
                assert_eq!(destination, path);
                assert_eq!(source.parent(), Some(directory.path()));
                assert!(
                    source
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(".outflow-settings-tmp-"))
                );
                assert_eq!(observed_temporary_path.borrow().as_deref(), Some(source));
                assert!(
                    fs::symlink_metadata(source)
                        .expect("prepared replacement still exists at replacement operation")
                        .file_type()
                        .is_file()
                );
                let temporary_bytes =
                    fs::read(source).expect("read complete replacement at replacement operation");
                assert!(
                    temporary_bytes == replacement_json.as_bytes(),
                    "prepared bytes differ from the expected replacement serialization"
                );
                let temporary_json = std::str::from_utf8(&temporary_bytes)
                    .expect("prepared replacement remains UTF-8 JSON");
                let decoded_temporary = Settings::from_json(temporary_json)
                    .expect("prepared replacement remains valid schema-v1 settings");
                assert!(
                    decoded_temporary == replacement,
                    "prepared document differs from the synthetic replacement"
                );
                assert_eq!(decoded_temporary.schema_version, 1);

                assert!(
                    fs::symlink_metadata(destination)
                        .expect("original destination still exists before replacement")
                        .file_type()
                        .is_file()
                );
                assert!(
                    fs::read(destination).expect("read original destination before replacement")
                        == original_bytes,
                    "original destination bytes changed before replacement"
                );
                println!("Replacement callback invoked: VERIFIED");
                let replacement_result: io::Result<()> = Err(io::Error::other(
                    "synthetic destination replacement operation failure",
                ));
                assert!(replacement_result.is_err());
                replacement_succeeded.set(replacement_result.is_ok());
                replacement_error_injected.set(replacement_result.is_err());
                println!("Replacement-operation error: INJECTED");
                replacement_result
            },
            |parent| {
                parent_sync_calls.set(parent_sync_calls.get() + 1);
                parent.sync_all()
            },
        )
        .expect_err("injected replacement-operation error must reject the write");

        assert_eq!(replacement_callback_calls.get(), 1);
        assert!(prepared_replacement_verified.get());
        assert!(temporary_file_synchronized.get());
        assert!(replacement_error_injected.get());
        assert!(!replacement_succeeded.get());
        assert_eq!(parent_sync_calls.get(), 0);
        assert_eq!(error, SettingsStorageError::ReplacementFailed);
        assert!(
            !error
                .to_string()
                .contains("synthetic replacement spoken value")
        );
        assert!(!format!("{error:?}").contains("Synthetic Replacement Written Value"));
        println!("Writer returned replacement error: PASS");

        assert!(
            fs::symlink_metadata(&path)
                .expect("original destination remains present")
                .file_type()
                .is_file()
        );
        let bytes_after_failure = fs::read(&path).expect("read destination after failure");
        assert!(
            bytes_after_failure == original_bytes,
            "original destination bytes changed after replacement failure"
        );
        assert!(
            bytes_after_failure != replacement_json.as_bytes(),
            "replacement became visible at the destination despite the error"
        );
        let decoded_after_failure = Settings::from_json(
            std::str::from_utf8(&bytes_after_failure).expect("original bytes remain UTF-8 JSON"),
        )
        .expect("original destination remains valid schema-v1 settings");
        assert!(
            decoded_after_failure == original,
            "destination no longer decodes to the original settings"
        );
        assert_eq!(decoded_after_failure.schema_version, 1);
        assert!(
            decoded_after_failure.vocabulary == original.vocabulary,
            "original vocabulary values or entry order changed"
        );
        assert!(!replacement_succeeded.get());
        println!("Original destination bytes: PRESERVED");

        let loaded_after_failure = load_settings(&path)
            .expect("reload original settings after replacement failure")
            .expect("original settings remain available");
        assert!(
            loaded_after_failure == original,
            "reload after failure differs from the original settings"
        );
        assert_eq!(loaded_after_failure.schema_version, 1);
        assert!(
            loaded_after_failure.vocabulary == original.vocabulary,
            "reloaded original vocabulary values or entry order changed"
        );
        println!("Original settings reload after failure: PASS");

        let mode_after_failure = fs::metadata(&path)
            .expect("inspect preserved settings permissions")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode_after_failure, original_mode);
        println!("Original permissions: PRESERVED");

        let temporary_path = observed_temporary_path
            .borrow()
            .clone()
            .expect("replacement callback received the temporary path");
        assert!(
            !temporary_path.exists(),
            "failed temp replacement was removed"
        );
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), entries_before_failure);
        assert_eq!(directory_entries(directory.path()), vec![path.clone()]);
        assert_eq!(parent_sync_calls.get(), 0);
        println!("Temporary artifact cleanup: PASS");

        save_settings_atomic(&path, &replacement).expect("retry through public settings writer");
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), vec![path.clone()]);
        println!("Subsequent public write: PASS");

        let loaded_after_retry = load_settings(&path)
            .expect("load settings after normal retry")
            .expect("retried settings remain available");
        assert!(
            loaded_after_retry == replacement,
            "public retry did not save the replacement settings"
        );
        assert_eq!(loaded_after_retry.schema_version, 1);
        println!("New settings reload: PASS");

        drop(directory);
        assert!(
            !test_directory_path.exists(),
            "isolated test directory should be removed"
        );

        println!("\nSchema migration performed: NO");
        println!("Real user settings accessed: NO");
        println!("Actual OS rename failure tested: NO");
        println!("Post-replacement rollback verified: NO");
        println!("Crash durability verified: NO");
    }

    #[test]
    fn atomic_writer_parent_directory_sync_failure_preserves_replaced_settings_and_allows_retry() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let test_directory_path = directory.path().to_path_buf();
        let path = directory.path().join("settings.json");
        let mut original = valid_settings();
        original.vocabulary.push(VocabularyEntry {
            id: "synthetic-second-entry".to_owned(),
            spoken: "original exact spoken form".to_owned(),
            written: "Original Exact Written Form".to_owned(),
        });

        println!("Outflow atomic settings parent-directory sync failure test\n");
        save_settings_atomic(&path, &original).expect("write initial settings");
        println!("Initial settings write: PASS");

        let loaded_initial = load_settings(&path)
            .expect("load initial settings")
            .expect("initial settings exist");
        assert_eq!(loaded_initial, original);
        assert_eq!(loaded_initial.schema_version, 1);
        println!("Original settings reload: PASS");

        let original_bytes = fs::read(&path).expect("read initial settings bytes");
        let original_document: serde_json::Value =
            serde_json::from_slice(&original_bytes).expect("parse initial settings document");
        assert_eq!(original_document["schema_version"], 1);
        let original_mode = fs::metadata(&path)
            .expect("inspect initial settings permissions")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(original_mode, 0o600);
        let entries_before_update = directory_entries(directory.path());
        assert_eq!(entries_before_update, vec![path.clone()]);

        let mut replacement = original.clone();
        replacement.cleanup.remove_fillers = !replacement.cleanup.remove_fillers;
        replacement.vocabulary[0].id = "synthetic-replacement-id".to_owned();
        replacement.vocabulary[0].spoken = "synthetic replacement spoken value".to_owned();
        replacement.vocabulary[0].written = "Synthetic Replacement Written Value".to_owned();
        assert_ne!(replacement, original);
        let replacement_json = replacement.to_json().expect("encode replacement settings");
        assert_ne!(replacement_json.as_bytes(), original_bytes);

        let write_completed = Cell::new(false);
        let temporary_file_synchronized = Cell::new(false);
        let destination_replacement_verified = Cell::new(false);
        let parent_sync_calls = Cell::new(0_u32);
        let directory_sync_error_injected = Cell::new(false);
        let observed_temporary_path = RefCell::new(None);

        let error = save_settings_atomic_with_hooks(
            &path,
            &replacement,
            |file, json| {
                assert_eq!(json, replacement_json.as_bytes());
                file.write_all(json)?;
                file.flush()?;
                write_completed.set(true);

                let temporary_paths = temporary_files(directory.path());
                assert_eq!(temporary_paths.len(), 1);
                let temporary_path = temporary_paths[0].clone();
                assert_eq!(temporary_path.parent(), Some(directory.path()));
                let temporary_bytes =
                    fs::read(&temporary_path).expect("read fully written temporary replacement");
                assert_eq!(temporary_bytes, replacement_json.as_bytes());
                let temporary_json = std::str::from_utf8(&temporary_bytes)
                    .expect("temporary replacement is UTF-8 JSON");
                let decoded_temporary = Settings::from_json(temporary_json)
                    .expect("temporary replacement is valid schema-v1 settings");
                assert_eq!(decoded_temporary, replacement);
                assert_eq!(decoded_temporary.schema_version, 1);
                *observed_temporary_path.borrow_mut() = Some(temporary_path);
                println!("Temporary replacement fully written: VERIFIED");
                Ok(())
            },
            |file| {
                assert!(
                    write_completed.get(),
                    "complete write must precede file sync"
                );
                file.sync_all()?;
                temporary_file_synchronized.set(true);
                println!("Temporary-file synchronization: COMPLETED");
                Ok(())
            },
            || {
                assert!(temporary_file_synchronized.get());
                assert_eq!(
                    fs::read(&path).expect("original destination exists before replacement"),
                    original_bytes
                );
                Ok(())
            },
            |parent| {
                parent_sync_calls.set(parent_sync_calls.get() + 1);
                assert!(temporary_file_synchronized.get());
                assert!(parent.metadata().expect("inspect parent handle").is_dir());

                let destination_metadata =
                    fs::symlink_metadata(&path).expect("replaced destination remains present");
                assert!(destination_metadata.file_type().is_file());
                let visible_bytes = fs::read(&path).expect("read destination after replacement");
                assert_eq!(visible_bytes, replacement_json.as_bytes());
                assert_ne!(visible_bytes, original_bytes);
                let visible_json =
                    std::str::from_utf8(&visible_bytes).expect("visible replacement is UTF-8 JSON");
                let decoded_visible =
                    Settings::from_json(visible_json).expect("visible replacement is schema v1");
                assert_eq!(decoded_visible, replacement);
                assert_eq!(decoded_visible.schema_version, 1);

                let temporary_path = observed_temporary_path
                    .borrow()
                    .clone()
                    .expect("complete temporary path was observed before sync");
                assert!(
                    !temporary_path.exists(),
                    "real rename removes the temp path"
                );
                destination_replacement_verified.set(true);
                println!("Destination replacement: COMPLETED");
                println!("Parent-directory sync callback invoked: VERIFIED");

                directory_sync_error_injected.set(true);
                println!("Parent-directory sync error: INJECTED");
                Err(io::Error::other(
                    "synthetic parent-directory synchronization failure",
                ))
            },
        )
        .expect_err("injected parent-directory sync failure must return an error");

        assert!(write_completed.get());
        assert!(temporary_file_synchronized.get());
        assert!(destination_replacement_verified.get());
        assert_eq!(parent_sync_calls.get(), 1);
        assert!(directory_sync_error_injected.get());
        assert_eq!(error, SettingsStorageError::ReplacementDurabilityUncertain);
        let display = error.to_string();
        assert!(display.contains("settings replacement succeeded"));
        assert!(display.contains("directory durability could not be confirmed"));
        assert!(!display.contains("temporary settings file"));
        let debug = format!("{error:?}");
        for entry in &replacement.vocabulary {
            assert!(!display.contains(entry.id.as_str()));
            assert!(!display.contains(entry.spoken.as_str()));
            assert!(!display.contains(entry.written.as_str()));
            assert!(!debug.contains(entry.id.as_str()));
            assert!(!debug.contains(entry.spoken.as_str()));
            assert!(!debug.contains(entry.written.as_str()));
        }
        println!("Writer returned directory sync error: PASS");

        let destination_metadata =
            fs::symlink_metadata(&path).expect("replacement destination still exists");
        assert!(destination_metadata.file_type().is_file());
        let bytes_after_failure = fs::read(&path).expect("read destination after sync error");
        assert_eq!(bytes_after_failure, replacement_json.as_bytes());
        assert_ne!(bytes_after_failure, original_bytes);
        let decoded_after_failure = Settings::from_json(
            std::str::from_utf8(&bytes_after_failure).expect("replacement bytes remain UTF-8"),
        )
        .expect("replacement destination remains readable schema-v1 settings");
        assert_eq!(decoded_after_failure, replacement);
        assert_eq!(decoded_after_failure.schema_version, 1);
        assert_eq!(decoded_after_failure.vocabulary, replacement.vocabulary);
        assert_ne!(decoded_after_failure, original);
        println!("New destination bytes visible: VERIFIED");

        let loaded_after_failure = load_settings(&path)
            .expect("reload replacement after parent-directory sync error")
            .expect("replacement settings remain available");
        assert_eq!(loaded_after_failure, replacement);
        assert_eq!(loaded_after_failure.schema_version, 1);
        assert_eq!(loaded_after_failure.vocabulary, replacement.vocabulary);
        println!("Replacement settings reload: PASS");

        let mode_after_failure = fs::metadata(&path)
            .expect("inspect replacement permissions")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode_after_failure, 0o600);
        assert_eq!(mode_after_failure, original_mode);
        println!("File permissions: VERIFIED");

        let temporary_path = observed_temporary_path
            .borrow()
            .clone()
            .expect("temporary path was recorded");
        assert!(!temporary_path.exists());
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), vec![path.clone()]);
        assert_eq!(parent_sync_calls.get(), 1);
        println!("Unexpected temporary artifacts: NONE");

        let mut third = replacement.clone();
        third.cleanup.remove_fillers = !third.cleanup.remove_fillers;
        third.startup.run_at_login = true;
        third.vocabulary[1].spoken = "third synthetic spoken value".to_owned();
        third.vocabulary[1].written = "Third Synthetic Written Value".to_owned();
        assert_ne!(third, replacement);
        assert_ne!(third, original);
        save_settings_atomic(&path, &third).expect("subsequent public write succeeds");
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), vec![path.clone()]);
        println!("Subsequent public write: PASS");

        let loaded_after_retry = load_settings(&path)
            .expect("load settings after subsequent public write")
            .expect("third settings value remains available");
        assert_eq!(loaded_after_retry, third);
        assert_eq!(loaded_after_retry.schema_version, 1);
        assert_eq!(loaded_after_retry.vocabulary, third.vocabulary);
        println!("Third settings value reload: PASS");

        drop(directory);
        assert!(
            !test_directory_path.exists(),
            "isolated test directory should be removed"
        );

        println!("\nOld configuration rolled back: NO");
        println!("Post-replacement visibility: VERIFIED");
        println!("Crash durability: NOT VERIFIED");
        println!("Actual OS-level directory sync failure: NOT TESTED");
        println!("Real user settings accessed: NO");
        println!("Schema migration performed: NO");
    }

    #[test]
    fn settings_write_error_outcome_contract() {
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        enum TestedWriteOutcome {
            ReplacementNotPerformed,
            ReplacementVisibleDurabilityUncertain,
        }

        fn classify_tested_outcome(
            error: &SettingsStorageError,
            replacement_visible: bool,
        ) -> TestedWriteOutcome {
            match (error, replacement_visible) {
                (SettingsStorageError::ReplacementFailed, false) => {
                    TestedWriteOutcome::ReplacementNotPerformed
                }
                (SettingsStorageError::ReplacementDurabilityUncertain, true) => {
                    TestedWriteOutcome::ReplacementVisibleDurabilityUncertain
                }
                _ => panic!("typed error and observed destination state disagree"),
            }
        }

        fn assert_error_redacted(error: &SettingsStorageError, settings: &Settings) {
            let display = error.to_string();
            let debug = format!("{error:?}");
            let json = settings.to_json().expect("encode synthetic settings");
            assert!(!display.contains(&json));
            assert!(!debug.contains(&json));
            for entry in &settings.vocabulary {
                for value in [&entry.id, &entry.spoken, &entry.written] {
                    assert!(!display.contains(value));
                    assert!(!debug.contains(value));
                }
            }
        }

        let first_directory = TestDirectory::new();
        let first_path = first_directory.path().join("settings.json");
        let first_original = valid_settings();
        save_settings_atomic(&first_path, &first_original).expect("write first schema-v1 file");
        let first_original_bytes = fs::read(&first_path).expect("read first original bytes");
        assert_eq!(
            Settings::from_json(
                std::str::from_utf8(&first_original_bytes).expect("first file is UTF-8")
            )
            .expect("first file is valid schema v1"),
            first_original
        );

        let mut first_replacement = first_original.clone();
        first_replacement.vocabulary[0].id = "task61-first-replacement-entry".to_owned();
        first_replacement.vocabulary[0].spoken = "task61 synthetic spoken phrase".to_owned();
        first_replacement.vocabulary[0].written = "Task61 Synthetic Written Phrase".to_owned();
        first_replacement.startup.run_at_login = true;
        let first_replacement_json = first_replacement
            .to_json()
            .expect("encode first replacement settings");
        assert_ne!(first_replacement_json.as_bytes(), first_original_bytes);

        let first_replacement_called = Cell::new(false);
        let first_parent_sync_called = Cell::new(false);
        let first_error = save_settings_atomic_with_replacement_hook(
            &first_path,
            &first_replacement,
            |file, json| file.write_all(json),
            File::sync_all,
            || Ok(()),
            |source, destination| {
                assert!(!first_replacement_called.replace(true));
                assert_eq!(destination, first_path);
                assert_eq!(
                    fs::read(source).expect("read prepared replacement"),
                    first_replacement_json.as_bytes()
                );
                assert_eq!(
                    fs::read(destination).expect("read original before replacement"),
                    first_original_bytes
                );
                Err(io::Error::other("synthetic replacement-operation error"))
            },
            |_| {
                first_parent_sync_called.set(true);
                Ok(())
            },
        )
        .expect_err("injected replacement-operation error must return Err");

        assert!(first_replacement_called.get());
        assert!(!first_parent_sync_called.get());
        assert_eq!(first_error, SettingsStorageError::ReplacementFailed);
        assert_error_redacted(&first_error, &first_original);
        assert_error_redacted(&first_error, &first_replacement);
        let first_bytes_after_error = fs::read(&first_path).expect("read first destination");
        assert_eq!(first_bytes_after_error, first_original_bytes);
        assert_ne!(first_bytes_after_error, first_replacement_json.as_bytes());
        let first_visible = first_bytes_after_error == first_replacement_json.as_bytes();
        assert_eq!(
            classify_tested_outcome(&first_error, first_visible),
            TestedWriteOutcome::ReplacementNotPerformed
        );
        let first_loaded = load_settings(&first_path)
            .expect("reload first settings")
            .expect("first settings remain present");
        assert_eq!(first_loaded, first_original);
        assert_eq!(first_loaded.schema_version, 1);
        assert!(temporary_files(first_directory.path()).is_empty());

        let second_directory = TestDirectory::new();
        let second_path = second_directory.path().join("settings.json");
        let second_original = valid_settings();
        save_settings_atomic(&second_path, &second_original).expect("write second schema-v1 file");
        let second_original_bytes = fs::read(&second_path).expect("read second original bytes");
        assert_eq!(
            Settings::from_json(
                std::str::from_utf8(&second_original_bytes).expect("second file is UTF-8")
            )
            .expect("second file is valid schema v1"),
            second_original
        );

        let mut second_replacement = second_original.clone();
        second_replacement.vocabulary[0].id = "task61-second-replacement-entry".to_owned();
        second_replacement.vocabulary[0].spoken = "task61 other synthetic phrase".to_owned();
        second_replacement.vocabulary[0].written = "Task61 Other Synthetic Phrase".to_owned();
        second_replacement.cleanup.remove_fillers = !second_replacement.cleanup.remove_fillers;
        let second_replacement_json = second_replacement
            .to_json()
            .expect("encode second replacement settings");
        assert_ne!(second_replacement_json.as_bytes(), second_original_bytes);

        let second_parent_sync_called = Cell::new(false);
        let second_error = save_settings_atomic_with_hooks(
            &second_path,
            &second_replacement,
            |file, json| file.write_all(json),
            File::sync_all,
            || Ok(()),
            |parent| {
                assert!(!second_parent_sync_called.replace(true));
                assert!(parent.metadata().expect("inspect parent").is_dir());
                let bytes_at_sync =
                    fs::read(&second_path).expect("replacement is visible at parent-sync boundary");
                assert_eq!(bytes_at_sync, second_replacement_json.as_bytes());
                assert_ne!(bytes_at_sync, second_original_bytes);
                assert_eq!(
                    Settings::from_json(
                        std::str::from_utf8(&bytes_at_sync).expect("replacement is UTF-8")
                    )
                    .expect("replacement is valid schema v1"),
                    second_replacement
                );
                Err(io::Error::other(
                    "synthetic parent-directory synchronization error",
                ))
            },
        )
        .expect_err("injected parent-directory sync error must return Err");

        assert!(second_parent_sync_called.get());
        assert_eq!(
            second_error,
            SettingsStorageError::ReplacementDurabilityUncertain
        );
        assert_error_redacted(&second_error, &second_original);
        assert_error_redacted(&second_error, &second_replacement);
        let second_bytes_after_error = fs::read(&second_path).expect("read second destination");
        assert_eq!(second_bytes_after_error, second_replacement_json.as_bytes());
        assert_ne!(second_bytes_after_error, second_original_bytes);
        let second_visible = second_bytes_after_error == second_replacement_json.as_bytes();
        assert_eq!(
            classify_tested_outcome(&second_error, second_visible),
            TestedWriteOutcome::ReplacementVisibleDurabilityUncertain
        );
        let second_loaded = load_settings(&second_path)
            .expect("reload visible replacement")
            .expect("replacement settings remain present");
        assert_eq!(second_loaded, second_replacement);
        assert_eq!(second_loaded.schema_version, 1);
        assert!(temporary_files(second_directory.path()).is_empty());

        println!("Outflow settings write error contract\n");
        println!("Replacement-operation error:");
        println!("  Typed error: ReplacementFailed");
        println!("  Original destination: PRESERVED");
        println!("  Replacement committed: NO");
        println!();
        println!("Post-replacement directory-sync error:");
        println!("  Typed error: ReplacementDurabilityUncertain");
        println!("  Replacement destination: VISIBLE");
        println!("  Replacement settings reload: PASS");
        println!("  Crash durability: UNKNOWN");
        println!();
        println!("Both writer operations returned errors: VERIFIED");
        println!("Distinct destination outcomes: VERIFIED");
        println!("Automatic rollback: NO");
        println!("Real user settings accessed: NO");
        println!("Actual OS-level failures reproduced: NO");
    }

    #[test]
    fn atomic_writer_temporary_creation_failure_preserves_valid_settings_and_allows_retry() {
        let directory = TestDirectory::new();
        let test_directory_path = directory.path().to_path_buf();
        let path = directory.path().join("settings.json");
        let original = valid_settings();

        save_settings_atomic(&path, &original).expect("write initial settings");
        let loaded_initial = load_settings(&path)
            .expect("load initial settings")
            .expect("initial settings exist");
        assert_eq!(loaded_initial, original);

        let original_bytes = fs::read(&path).expect("read initial settings bytes");
        let original_document: serde_json::Value =
            serde_json::from_slice(&original_bytes).expect("parse initial settings document");
        assert_eq!(original_document["schema_version"], 1);
        let entries_before_failure = directory_entries(directory.path());
        assert_eq!(entries_before_failure, vec![path.clone()]);

        let mut replacement = original.clone();
        replacement.vocabulary[0].id = "synthetic-replacement-entry".to_owned();
        replacement.vocabulary[0].spoken = "synthetic replacement phrase".to_owned();
        replacement.vocabulary[0].written = "Synthetic Replacement Phrase".to_owned();
        assert_ne!(replacement, original);
        let replacement_json = replacement.to_json().expect("encode replacement settings");
        assert_ne!(replacement_json.as_bytes(), original_bytes);

        let creation_attempts = Cell::new(0_u32);
        let attempted_paths = RefCell::new(Vec::new());
        let write_attempted = Cell::new(false);
        let replacement_boundary_reached = Cell::new(false);
        let error = save_settings_atomic_with_creation_hook(
            &path,
            &replacement,
            |temporary_path| {
                creation_attempts.set(creation_attempts.get() + 1);
                attempted_paths
                    .borrow_mut()
                    .push(temporary_path.to_path_buf());
                assert_eq!(temporary_path.parent(), Some(directory.path()));
                assert!(
                    temporary_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .is_some_and(|name| name.starts_with(".outflow-settings-tmp-"))
                );
                assert!(!temporary_path.exists());
                Err(io::Error::other(
                    "synthetic temporary-file creation failure",
                ))
            },
            |file, json| {
                write_attempted.set(true);
                file.write_all(json)
            },
            File::sync_all,
            || {
                replacement_boundary_reached.set(true);
                Ok(())
            },
            File::sync_all,
        )
        .expect_err("injected temporary-file creation failure must reject the write");

        assert_eq!(
            creation_attempts.get(),
            1,
            "no collision retry should occur"
        );
        let attempted_path = attempted_paths
            .borrow()
            .first()
            .cloned()
            .expect("creation operation received a temporary path");
        assert!(!attempted_path.exists());
        assert!(!write_attempted.get());
        assert!(!replacement_boundary_reached.get());
        assert_eq!(error, SettingsStorageError::TemporaryFileCreationFailed);
        assert!(!error.to_string().contains("synthetic replacement phrase"));

        assert!(
            fs::symlink_metadata(&path)
                .expect("original destination still exists")
                .file_type()
                .is_file()
        );
        assert_eq!(
            fs::read(&path).expect("read destination after failed creation"),
            original_bytes
        );
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), entries_before_failure);

        let loaded_after_failure = load_settings(&path)
            .expect("reload original settings after failed creation")
            .expect("original settings remain available");
        assert_eq!(loaded_after_failure, original);
        assert_eq!(loaded_after_failure.schema_version, 1);
        assert_eq!(loaded_after_failure.vocabulary, original.vocabulary);

        save_settings_atomic(&path, &replacement).expect("retry normal settings write");
        let loaded_after_retry = load_settings(&path)
            .expect("load settings after normal retry")
            .expect("retried settings remain available");
        assert_eq!(loaded_after_retry, replacement);
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), vec![path.clone()]);

        drop(directory);
        assert!(
            !test_directory_path.exists(),
            "isolated test directory should be removed"
        );

        println!("Outflow atomic settings temporary-creation failure test\n");
        println!("Initial settings write: PASS");
        println!("Original settings reload: PASS");
        println!("Temporary-file creation failure: INJECTED");
        println!("Writer returned error: PASS");
        println!("Replacement attempted: NO");
        println!("Original destination bytes: PRESERVED");
        println!("Original settings reload after failure: PASS");
        println!("Unexpected temporary artifacts: NONE");
        println!("Subsequent normal write: PASS");
        println!("New settings reload: PASS");

        println!("\nSchema migration performed: NO");
        println!("Real user settings accessed: NO");
        println!("Actual OS permission failure tested: NO");
        println!("Actual disk-full failure tested: NO");
    }

    #[test]
    fn missing_parent_directory_is_reported() {
        let directory = TestDirectory::new();
        let path = directory.path().join("missing").join("settings.json");

        assert_eq!(
            save_settings_atomic(&path, &Settings::default()),
            Err(SettingsStorageError::MissingParentDirectory)
        );
    }

    #[test]
    fn non_directory_parent_is_rejected() {
        let directory = TestDirectory::new();
        let parent_file = directory.path().join("parent-file");
        fs::write(&parent_file, b"not a directory").expect("create parent file");
        let path = parent_file.join("settings.json");

        assert_eq!(
            save_settings_atomic(&path, &Settings::default()),
            Err(SettingsStorageError::InvalidParentDirectory)
        );
    }

    #[test]
    fn destination_directory_is_rejected_without_cleanup_or_modification() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings-dir");
        fs::create_dir(&path).expect("create non-file destination");

        assert_eq!(
            save_settings_atomic(&path, &Settings::default()),
            Err(SettingsStorageError::UnsafeDestination)
        );
        assert!(fs::metadata(path).expect("destination remains").is_dir());
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn destination_symbolic_link_is_rejected_without_following_it() {
        use std::os::unix::fs::symlink;

        let directory = TestDirectory::new();
        let target = directory.path().join("real-settings.json");
        let link = directory.path().join("settings.json");
        let prior = b"target must remain untouched";
        fs::write(&target, prior).expect("write symlink target");
        symlink(&target, &link).expect("create destination symlink");

        assert_eq!(
            save_settings_atomic(&link, &Settings::default()),
            Err(SettingsStorageError::UnsafeDestination)
        );
        assert_eq!(fs::read(target).expect("read symlink target"), prior);
        assert!(
            fs::symlink_metadata(link)
                .expect("symlink entry remains")
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn saved_file_permissions_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");

        save_settings_atomic(&path, &Settings::default()).expect("save settings");

        let mode = fs::metadata(path)
            .expect("read settings metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn parent_sync_failure_is_distinguished_after_replacement() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let settings = valid_settings();
        fs::write(&path, b"prior configuration").expect("write prior settings");

        let error = save_settings_atomic_with_hooks(
            &path,
            &settings,
            |file, json| file.write_all(json),
            |file| file.sync_all(),
            || Ok(()),
            |_| Err(io::Error::other("synthetic directory sync failure")),
        )
        .expect_err("post-replacement durability failure");

        assert_eq!(error, SettingsStorageError::ReplacementDurabilityUncertain);
        let stored = fs::read_to_string(path).expect("read replaced settings");
        assert_eq!(
            Settings::from_json(&stored).expect("decode replacement"),
            settings
        );
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn reset_pre_replacement_failure_preserves_original_and_redacts_hook_error() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let prior = b"synthetic private configuration phrase";
        fs::write(&path, prior).expect("write prior settings");

        let error = reset_settings_to_defaults_with_hooks(
            &path,
            true,
            |file, json| file.write_all(json),
            File::sync_all,
            || Err(io::Error::other("private configuration phrase")),
            File::sync_all,
        )
        .expect_err("simulated pre-replacement interruption");

        assert_eq!(error, SettingsStorageError::ReplacementFailed);
        assert!(!error.to_string().contains("private configuration phrase"));
        assert_eq!(fs::read(path).expect("original file remains"), prior);
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn confirmed_reset_pre_replacement_failure_preserves_settings_and_allows_retry() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let sentinel_path = directory.path().join("unrelated-sentinel.bin");
        let sentinel_bytes = b"Task 65 unrelated sentinel bytes\n";
        let original = Settings {
            cleanup: crate::CleanupSettings {
                mode: crate::CleanupMode::Verbatim,
                remove_fillers: false,
            },
            startup: crate::StartupSettings { run_at_login: true },
            vocabulary: vec![
                VocabularyEntry {
                    id: "task65-entry-one".to_owned(),
                    spoken: "synthetic spoken phrase alpha".to_owned(),
                    written: "Synthetic written replacement alpha".to_owned(),
                },
                VocabularyEntry {
                    id: "task65-entry-two".to_owned(),
                    spoken: "synthetic spoken phrase beta".to_owned(),
                    written: "Synthetic written replacement beta".to_owned(),
                },
            ],
            ..Settings::default()
        };
        let defaults = Settings::default();
        let defaults_json = defaults.to_json().expect("encode schema-v1 defaults");

        save_settings_atomic(&path, &original).expect("write initial synthetic settings");
        fs::write(&sentinel_path, sentinel_bytes).expect("write unrelated sentinel");
        let loaded_before = load_settings(&path)
            .expect("load initial settings")
            .expect("initial settings exist");
        assert_eq!(loaded_before, original);
        assert_eq!(loaded_before.schema_version, 1);
        assert_eq!(loaded_before.vocabulary, original.vocabulary);

        let original_bytes = fs::read(&path).expect("capture original settings bytes");
        let original_mode = fs::metadata(&path)
            .expect("inspect original settings permissions")
            .permissions()
            .mode()
            & 0o777;
        let sentinel_snapshot = fs::read(&sentinel_path).expect("capture sentinel bytes");
        assert_eq!(sentinel_snapshot, sentinel_bytes);
        let original_entries = directory_entries(directory.path());
        assert_eq!(original_entries.len(), 2);

        let replacement_prepared = Cell::new(false);
        let error = reset_settings_to_defaults_with_hooks(
            &path,
            true,
            |file, json| file.write_all(json),
            File::sync_all,
            || {
                assert!(!replacement_prepared.replace(true));
                let temporary_path = directory_entries(directory.path())
                    .into_iter()
                    .find(|candidate| {
                        candidate
                            .file_name()
                            .and_then(|name| name.to_str())
                            .is_some_and(|name| name.starts_with(".outflow-settings-tmp-"))
                    })
                    .expect("prepared defaults exist in the temporary file");
                let prepared_bytes = fs::read(temporary_path)
                    .expect("read prepared default replacement before replacement");
                assert_eq!(prepared_bytes, defaults_json.as_bytes());
                assert_ne!(
                    fs::read(&path).expect("original remains before replacement"),
                    prepared_bytes
                );
                Err(io::Error::other(
                    "synthetic confirmed-reset pre-replacement failure",
                ))
            },
            File::sync_all,
        )
        .expect_err("confirmed reset must propagate the injected writer failure");

        assert!(replacement_prepared.get());
        assert_eq!(error, SettingsStorageError::ReplacementFailed);
        assert!(!error.to_string().contains("synthetic spoken phrase"));
        let destination_metadata =
            fs::symlink_metadata(&path).expect("original destination remains present");
        assert!(destination_metadata.file_type().is_file());
        assert_eq!(
            fs::read(&path).expect("read original settings after failure"),
            original_bytes
        );
        assert_ne!(original_bytes, defaults_json.as_bytes());
        let loaded_after_failure = load_settings(&path)
            .expect("reload settings after failed reset")
            .expect("original settings remain available");
        assert_eq!(loaded_after_failure, original);
        assert_eq!(loaded_after_failure.vocabulary, original.vocabulary);
        assert_eq!(loaded_after_failure.cleanup, original.cleanup);
        assert_eq!(loaded_after_failure.startup, original.startup);
        assert_eq!(
            destination_metadata.permissions().mode() & 0o777,
            original_mode
        );
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), original_entries);
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after failure"),
            sentinel_snapshot
        );

        reset_settings_to_defaults(&path, true)
            .expect("a new explicitly confirmed reset succeeds after the failed attempt");
        let loaded_after_retry = load_settings(&path)
            .expect("reload settings after explicit retry")
            .expect("successful reset leaves a settings file");
        assert_eq!(loaded_after_retry, defaults);
        assert_eq!(loaded_after_retry.schema_version, 1);
        assert!(loaded_after_retry.vocabulary.is_empty());
        assert_eq!(
            loaded_after_retry.cleanup,
            crate::CleanupSettings::default()
        );
        assert_eq!(
            loaded_after_retry.startup,
            crate::StartupSettings::default()
        );
        assert_eq!(
            fs::read(&path).expect("read persisted defaults after retry"),
            defaults_json.as_bytes()
        );
        assert_eq!(
            fs::metadata(&path)
                .expect("inspect reset file permissions")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(directory_entries(directory.path()), original_entries);
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after retry"),
            sentinel_snapshot
        );

        println!("Outflow confirmed settings reset failure test\n");
        println!("Initial settings write: PASS");
        println!("Initial vocabulary persisted: VERIFIED");
        println!("Confirmed reset requested: VERIFIED");
        println!("Pre-replacement failure: INJECTED");
        println!("Reset returned storage error: PASS");
        println!("Original destination bytes: PRESERVED");
        println!("Original vocabulary: PRESERVED");
        println!("Original preferences: PRESERVED");
        println!("Temporary artifact cleanup: PASS");
        println!("Unrelated sentinel: PRESERVED\n");
        println!("Explicit retry requested: VERIFIED");
        println!("Retry reset succeeded: PASS");
        println!("Schema-v1 defaults persisted: PASS");
        println!("Previous vocabulary cleared: VERIFIED");
        println!("Unrelated sentinel after retry: PRESERVED\n");
        println!("Automatic retry: NO");
        println!("Automatic rollback: NO");
        println!("Real user settings accessed: NO");
        println!("Human confirmation UI tested: NO");
        println!("Actual OS-level failure reproduced: NO");
    }

    #[test]
    fn confirmed_reset_directory_sync_failure_leaves_defaults_visible() {
        use std::os::unix::fs::PermissionsExt;

        let directory = TestDirectory::new();
        let test_directory_path = directory.path().to_path_buf();
        let path = directory.path().join("settings.json");
        let expected_settings_path = directory.path().join("settings.json");
        let sentinel_path = directory.path().join("unrelated-sentinel.bin");
        let sentinel_bytes = b"Task 66 unrelated sentinel bytes\n";
        let original = Settings {
            cleanup: crate::CleanupSettings {
                mode: crate::CleanupMode::Verbatim,
                remove_fillers: false,
            },
            startup: crate::StartupSettings { run_at_login: true },
            vocabulary: vec![
                VocabularyEntry {
                    id: "task66-entry-one".to_owned(),
                    spoken: "synthetic reset phrase alpha".to_owned(),
                    written: "Synthetic Reset Replacement Alpha".to_owned(),
                },
                VocabularyEntry {
                    id: "task66-entry-two".to_owned(),
                    spoken: "synthetic reset phrase beta".to_owned(),
                    written: "Synthetic Reset Replacement Beta".to_owned(),
                },
                VocabularyEntry {
                    id: "task66-entry-three".to_owned(),
                    spoken: "synthetic reset phrase gamma".to_owned(),
                    written: "Synthetic Reset Replacement Gamma".to_owned(),
                },
            ],
            ..Settings::default()
        };
        let defaults = Settings::default();
        let defaults_json = defaults.to_json().expect("encode schema-v1 defaults");
        assert_ne!(original, defaults);

        println!("Outflow confirmed reset durability uncertainty test\n");
        save_settings_atomic(&path, &original).expect("write initial synthetic settings");
        let loaded_before_reset = load_settings(&path)
            .expect("load initial settings")
            .expect("initial settings exist");
        assert_eq!(loaded_before_reset, original);
        assert_eq!(loaded_before_reset.schema_version, 1);
        assert_eq!(loaded_before_reset.vocabulary, original.vocabulary);
        assert_ne!(loaded_before_reset.cleanup, defaults.cleanup);
        assert_ne!(loaded_before_reset.startup, defaults.startup);
        println!("Initial settings write: PASS");
        println!("Original vocabulary persisted: VERIFIED");

        fs::write(&sentinel_path, sentinel_bytes).expect("write unrelated sentinel");
        let original_bytes = fs::read(&path).expect("capture exact original settings bytes");
        let original_settings_json = std::str::from_utf8(&original_bytes)
            .expect("original schema-v1 settings are UTF-8")
            .to_owned();
        let original_mode = fs::metadata(&path)
            .expect("inspect original settings permissions")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(original_mode, 0o600);
        let sentinel_snapshot = fs::read(&sentinel_path).expect("capture sentinel bytes");
        assert_eq!(sentinel_snapshot, sentinel_bytes);
        let original_entries = directory_entries(directory.path());
        assert_eq!(original_entries, vec![path.clone(), sentinel_path.clone()]);

        let reset_writer_reached = Cell::new(false);
        let temporary_file_synchronized = Cell::new(false);
        let before_replace_reached = Cell::new(false);
        let destination_replacement_verified = Cell::new(false);
        let parent_sync_calls = Cell::new(0_u32);
        let directory_sync_error_injected = Cell::new(false);
        let observed_temporary_path = RefCell::new(None::<PathBuf>);
        let replacement_bytes_at_sync = RefCell::new(None::<Vec<u8>>);

        let error = reset_settings_to_defaults_with_hooks(
            &path,
            true,
            |file, json| {
                assert!(!reset_writer_reached.replace(true));
                assert_eq!(json, defaults_json.as_bytes());
                file.write_all(json)?;
                file.flush()?;

                let temporary_paths = temporary_files(directory.path());
                assert_eq!(temporary_paths.len(), 1);
                let temporary_path = temporary_paths[0].clone();
                assert_eq!(temporary_path.parent(), Some(directory.path()));
                let temporary_bytes = fs::read(&temporary_path)
                    .expect("read complete temporary defaults before synchronization");
                assert_eq!(temporary_bytes, defaults_json.as_bytes());
                let temporary_json = std::str::from_utf8(&temporary_bytes)
                    .expect("temporary defaults are UTF-8 JSON");
                let decoded_temporary = Settings::from_json(temporary_json)
                    .expect("temporary replacement is valid schema v1");
                assert_eq!(decoded_temporary, defaults);
                assert_eq!(
                    fs::read(&expected_settings_path)
                        .expect("original destination exists before replacement"),
                    original_bytes
                );
                *observed_temporary_path.borrow_mut() = Some(temporary_path);
                Ok(())
            },
            |file| {
                assert!(reset_writer_reached.get());
                file.sync_all()?;
                temporary_file_synchronized.set(true);
                Ok(())
            },
            || {
                assert!(temporary_file_synchronized.get());
                assert_eq!(path, expected_settings_path);
                assert_eq!(
                    fs::read(&path).expect("original destination remains before rename"),
                    original_bytes
                );
                before_replace_reached.set(true);
                Ok(())
            },
            |parent| {
                parent_sync_calls.set(parent_sync_calls.get() + 1);
                assert!(temporary_file_synchronized.get());
                assert!(before_replace_reached.get());
                assert_eq!(path, expected_settings_path);
                assert!(parent.metadata().expect("inspect parent handle").is_dir());

                let destination_metadata = fs::symlink_metadata(&expected_settings_path)
                    .expect("destination exists at the parent-sync boundary");
                assert!(destination_metadata.file_type().is_file());
                let visible_bytes = fs::read(&expected_settings_path)
                    .expect("read destination after real replacement");
                assert_ne!(visible_bytes, original_bytes);
                let visible_json =
                    std::str::from_utf8(&visible_bytes).expect("visible defaults are UTF-8 JSON");
                let decoded_visible =
                    Settings::from_json(visible_json).expect("visible defaults are schema v1");
                assert_eq!(decoded_visible, defaults);
                assert!(decoded_visible.vocabulary.is_empty());
                assert_eq!(decoded_visible.cleanup, defaults.cleanup);
                assert_eq!(decoded_visible.startup, defaults.startup);

                let temporary_path = observed_temporary_path
                    .borrow()
                    .clone()
                    .expect("temporary path was captured before replacement");
                assert!(
                    !temporary_path.exists(),
                    "real rename consumes the temp path"
                );
                *replacement_bytes_at_sync.borrow_mut() = Some(visible_bytes);
                destination_replacement_verified.set(true);
                directory_sync_error_injected.set(true);
                Err(io::Error::other(
                    "synthetic confirmed-reset directory synchronization failure",
                ))
            },
        )
        .expect_err("confirmed reset must return the injected durability error");

        assert!(reset_writer_reached.get());
        assert!(temporary_file_synchronized.get());
        assert!(before_replace_reached.get());
        assert!(destination_replacement_verified.get());
        assert_eq!(parent_sync_calls.get(), 1);
        assert!(directory_sync_error_injected.get());
        assert_eq!(error, SettingsStorageError::ReplacementDurabilityUncertain);
        assert!(!matches!(
            &error,
            SettingsStorageError::ReplacementFailed | SettingsStorageError::ResetNotConfirmed
        ));
        assert!(matches!(
            &error,
            SettingsStorageError::ReplacementDurabilityUncertain
        ));
        let error_display = error.to_string();
        let error_debug = format!("{error:?}");
        assert!(error_display.contains("directory durability could not be confirmed"));
        assert!(!error_display.contains(&original_settings_json));
        assert!(!error_debug.contains(&original_settings_json));
        for entry in &original.vocabulary {
            assert!(!error_display.contains(&entry.id));
            assert!(!error_display.contains(&entry.spoken));
            assert!(!error_display.contains(&entry.written));
            assert!(!error_debug.contains(&entry.id));
            assert!(!error_debug.contains(&entry.spoken));
            assert!(!error_debug.contains(&entry.written));
        }

        println!("Confirmed reset requested: VERIFIED");
        println!("Temporary replacement synchronized: VERIFIED");
        println!("Real destination replacement: COMPLETED");
        println!("Parent-directory sync callback: VERIFIED");
        println!("Directory synchronization error: INJECTED");
        println!("Reset returned ReplacementDurabilityUncertain: PASS\n");

        let destination_metadata = fs::symlink_metadata(&path)
            .expect("settings destination remains after the reset error");
        assert!(destination_metadata.file_type().is_file());
        let bytes_after_error = fs::read(&path).expect("read complete settings after reset error");
        let observed_replacement = replacement_bytes_at_sync
            .borrow()
            .clone()
            .expect("replacement bytes were observed before injecting the error");
        assert_eq!(bytes_after_error, observed_replacement);
        assert_ne!(bytes_after_error, original_bytes);
        let decoded_after_error = Settings::from_json(
            std::str::from_utf8(&bytes_after_error).expect("visible defaults remain UTF-8"),
        )
        .expect("replacement settings remain readable schema v1");
        assert_eq!(decoded_after_error.schema_version, 1);
        assert_eq!(decoded_after_error, defaults);
        assert!(decoded_after_error.vocabulary.is_empty());
        assert_eq!(decoded_after_error.cleanup, defaults.cleanup);
        assert_eq!(decoded_after_error.startup, defaults.startup);
        println!("Default settings visible after error: VERIFIED");
        println!("Original vocabulary no longer visible: VERIFIED");
        println!("Non-default preferences reset: VERIFIED");

        let loaded_after_error = load_settings(&path)
            .expect("public loader accepts reset defaults after durability error")
            .expect("reset settings file remains available");
        assert_eq!(loaded_after_error.schema_version, 1);
        assert_eq!(loaded_after_error, defaults);
        assert!(loaded_after_error.vocabulary.is_empty());
        println!("Settings reload after error: PASS");

        let mode_after_error = destination_metadata.permissions().mode() & 0o777;
        assert_eq!(mode_after_error, original_mode);
        assert_eq!(mode_after_error, 0o600);
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after reset error"),
            sentinel_snapshot
        );
        println!("File permissions: VERIFIED");
        println!("Unrelated sentinel: PRESERVED");

        let temporary_path = observed_temporary_path
            .borrow()
            .clone()
            .expect("temporary path remains recorded");
        assert!(!temporary_path.exists());
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(
            directory_entries(directory.path()),
            vec![path.clone(), sentinel_path.clone()]
        );
        assert_eq!(parent_sync_calls.get(), 1);
        println!("Unexpected temporary artifacts: NONE\n");

        let third = Settings {
            cleanup: crate::CleanupSettings {
                mode: crate::CleanupMode::Verbatim,
                remove_fillers: true,
            },
            startup: crate::StartupSettings { run_at_login: true },
            vocabulary: vec![VocabularyEntry {
                id: "task66-third-entry".to_owned(),
                spoken: "third synthetic reset phrase".to_owned(),
                written: "Third Synthetic Reset Replacement".to_owned(),
            }],
            ..Settings::default()
        };
        assert_ne!(third, original);
        assert_ne!(third, defaults);
        save_settings_atomic(&path, &third)
            .expect("subsequent distinct public settings write succeeds");
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(
            directory_entries(directory.path()),
            vec![path.clone(), sentinel_path.clone()]
        );
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after subsequent public write"),
            sentinel_snapshot
        );
        assert_eq!(
            fs::metadata(&path)
                .expect("inspect permissions after subsequent public write")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        println!("Subsequent public write: PASS");

        let loaded_third = load_settings(&path)
            .expect("public loader accepts the subsequent settings value")
            .expect("third settings value remains available");
        assert_eq!(loaded_third.schema_version, 1);
        assert_eq!(loaded_third, third);
        assert_eq!(loaded_third.vocabulary, third.vocabulary);
        assert!(temporary_files(directory.path()).is_empty());
        assert_eq!(
            fs::read(&sentinel_path).expect("sentinel after third value reload"),
            sentinel_snapshot
        );
        println!("Third settings value reload: PASS\n");

        println!("Automatic retry: NO");
        println!("Automatic rollback: NO");
        println!("Crash durability verified: NO");
        println!("Actual OS-level sync failure tested: NO");
        println!("Human confirmation UI tested: NO");
        println!("Real user settings accessed: NO");
        println!("Schema migration performed: NO");

        drop(directory);
        assert!(
            !test_directory_path.exists(),
            "isolated test directory should be removed"
        );
    }

    #[test]
    fn reset_post_replacement_sync_failure_is_uncertain_and_not_retried() {
        use std::cell::Cell;

        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        fs::write(
            &path,
            valid_settings()
                .to_json()
                .expect("encode synthetic custom settings"),
        )
        .expect("write custom settings");
        let parent_sync_calls = Cell::new(0);

        let error = reset_settings_to_defaults_with_hooks(
            &path,
            true,
            |file, json| file.write_all(json),
            File::sync_all,
            || Ok(()),
            |_| {
                parent_sync_calls.set(parent_sync_calls.get() + 1);
                Err(io::Error::other("synthetic directory sync failure"))
            },
        )
        .expect_err("post-replacement directory sync is uncertain");

        assert_eq!(error, SettingsStorageError::ReplacementDurabilityUncertain);
        assert_eq!(parent_sync_calls.get(), 1);
        assert_eq!(super::load_settings(&path), Ok(Some(Settings::default())));
        assert!(temporary_files(directory.path()).is_empty());
    }

    #[test]
    fn storage_errors_do_not_disclose_private_configuration_values() {
        let directory = TestDirectory::new();
        let path = directory.path().join("settings.json");
        let settings = Settings {
            vocabulary: vec![VocabularyEntry {
                id: "secret-id-2299".to_owned(),
                spoken: "secret spoken phrase 2299".to_owned(),
                written: "secret written phrase 2299".to_owned(),
            }],
            ..Settings::default()
        };
        let private_value = "secret spoken phrase 2299";

        let error = save_settings_atomic_with_hooks(
            &path,
            &settings,
            |_, _| Err(io::Error::other(private_value)),
            File::sync_all,
            || Ok(()),
            File::sync_all,
        )
        .expect_err("write hook fails");
        let rendered = error.to_string();

        assert!(!rendered.contains(private_value));
        assert!(!rendered.contains("secret-id-2299"));
        assert!(temporary_files(directory.path()).is_empty());
        assert!(matches!(
            error,
            SettingsStorageError::PermissionDenied {
                operation: SettingsStorageOperation::WriteTemporaryFile
            } | SettingsStorageError::TemporaryFileWriteFailed
        ));
    }
}
