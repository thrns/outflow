use outflow_settings::{Settings, SettingsStorageError, load_settings};
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

struct ExampleDirectory(PathBuf);

impl ExampleDirectory {
    fn create() -> Result<Self, Box<dyn Error>> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        for attempt in 0..128_u32 {
            let path = std::env::temp_dir().join(format!(
                "outflow-load-example-{}-{timestamp}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err("could not create an isolated example directory".into())
    }

    fn settings_path(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for ExampleDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = ExampleDirectory::create()?;
    let expected = Settings::default();
    let valid_path = directory.settings_path("valid.json");
    fs::write(&valid_path, expected.to_json()?)?;

    if load_settings(&valid_path)? != Some(expected) {
        return Err("valid schema v1 settings did not match the fixture".into());
    }
    println!("Outflow settings load ready");
    println!("Valid v1: loaded");

    let missing_path = directory.settings_path("missing.json");
    if load_settings(&missing_path)?.is_some() || missing_path.exists() {
        return Err("missing settings file was not recognized without side effects".into());
    }
    println!("Missing file: defaults available");

    let invalid_path = directory.settings_path("invalid.json");
    fs::write(&invalid_path, b"{ malformed settings")?;
    if !matches!(
        load_settings(&invalid_path),
        Err(SettingsStorageError::InvalidDocument { .. })
    ) {
        return Err("malformed settings did not return an invalid-document error".into());
    }
    println!("Invalid file: rejected");
    Ok(())
}
