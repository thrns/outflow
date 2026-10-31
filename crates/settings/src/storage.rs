//! Explicit-path loading, user-directed reset, and atomic persistence for C-08 settings.
//!
//! The loader reads only a caller-supplied regular file, caps input at
//! [`MAX_SETTINGS_FILE_BYTES`], and validates through [`Settings::from_json`].
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
/// [`Self::ReplacementDurabilityUncertain`] means the rename already succeeded,
/// but the parent directory could not be synchronized afterward.
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
    /// The atomic replacement operation failed; the existing destination is kept.
    ReplacementFailed,
    /// Replacement succeeded, but syncing the parent directory failed.
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
                formatter.write_str("atomic settings replacement failed; existing settings were kept")
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
    match Settings::from_json(&json) {
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
    use super::{
        MAX_SETTINGS_FILE_BYTES, SettingsStorageError, SettingsStorageOperation, load_settings,
    };
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
/// file is already visible at that point.
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

fn save_settings_atomic_with_hooks<W, S, B, P>(
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
            write_json,
            synchronize_file,
            before_replace,
            synchronize_parent,
        )
    }

    #[cfg(not(unix))]
    {
        let _ = (
            path,
            json,
            require_existing_destination,
            write_json,
            synchronize_file,
            before_replace,
            synchronize_parent,
        );
        Err(SettingsStorageError::UnsupportedPlatform)
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
fn save_json_unix<W, S, B, P>(
    path: &Path,
    json: &[u8],
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
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    use std::time::{SystemTime, UNIX_EPOCH};

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
        let opened = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary_path);
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

        before_replace().map_err(|error| {
            map_io_error(
                error,
                SettingsStorageError::ReplacementFailed,
                SettingsStorageOperation::ReplaceDestination,
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

        fs::rename(temporary_guard.path(), path).map_err(|error| {
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
        SettingsStorageError, SettingsStorageOperation, reset_settings_to_defaults_with_hooks,
        save_settings_atomic, save_settings_atomic_with_hooks,
    };
    use crate::{Settings, SettingsError, VocabularyEntry};
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
