use outflow_settings::{
    CleanupMode, CleanupSettings, Settings, SettingsStorageError, StartupSettings, load_settings,
    reset_settings_to_defaults,
};
use std::error::Error;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct TemporaryDirectory(PathBuf);

impl TemporaryDirectory {
    fn new() -> io::Result<Self> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for attempt in 0..128_u32 {
            let path = std::env::temp_dir().join(format!(
                "outflow-settings-reset-example-{}-{timestamp}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::other(
            "could not create an isolated settings example directory",
        ))
    }

    fn settings_path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn require(condition: bool, message: &'static str) -> io::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message))
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = TemporaryDirectory::new()?;
    let custom_path = directory.settings_path("custom-settings.json");
    let custom = Settings {
        cleanup: CleanupSettings {
            mode: CleanupMode::Verbatim,
            remove_fillers: false,
        },
        startup: StartupSettings { run_at_login: true },
        vocabulary: vec![outflow_settings::VocabularyEntry {
            id: "synthetic-reset-example".to_owned(),
            spoken: "synthetic spoken phrase".to_owned(),
            written: "Synthetic Written Phrase".to_owned(),
        }],
        ..Settings::default()
    };
    fs::write(&custom_path, custom.to_json()?)?;
    let original_bytes = fs::read(&custom_path)?;

    let unconfirmed = reset_settings_to_defaults(&custom_path, false);
    require(
        matches!(unconfirmed, Err(SettingsStorageError::ResetNotConfirmed)),
        "unconfirmed reset did not return cancellation",
    )?;
    require(
        fs::read(&custom_path)? == original_bytes,
        "unconfirmed reset modified the configuration",
    )?;

    reset_settings_to_defaults(&custom_path, true)?;
    let restored = load_settings(&custom_path)?;
    require(
        restored == Some(Settings::default()),
        "confirmed reset did not restore defaults",
    )?;

    let malformed_path = directory.settings_path("malformed-settings.json");
    fs::write(&malformed_path, b"{ malformed synthetic settings")?;
    reset_settings_to_defaults(&malformed_path, true)?;
    require(
        load_settings(&malformed_path)? == Some(Settings::default()),
        "malformed configuration was not replaced with valid defaults",
    )?;
    require(
        Settings::default().schema_version == 1,
        "compiled defaults must use schema version 1",
    )?;

    println!("Outflow settings reset ready");
    println!("Unconfirmed reset: cancelled");
    println!("Confirmed reset: defaults restored");
    println!("Invalid configuration: recovered");
    println!("Schema version: 1");
    Ok(())
}
