use outflow_settings::{Settings, SettingsInitialization, initialize_settings};
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
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
                "outflow-settings-initialize-example-{}-{timestamp}-{attempt}",
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

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = TemporaryDirectory::new()?;
    let missing_path = directory.path().join("missing-settings.json");
    let first_run = initialize_settings(&missing_path)?;
    let SettingsInitialization::FirstRun(defaults) = first_run else {
        return Err(io::Error::other("missing settings did not return defaults").into());
    };
    if defaults.schema_version != 1 || defaults.validate().is_err() || missing_path.exists() {
        return Err(io::Error::other("first-run defaults were not valid and in-memory").into());
    }

    let valid_path = directory.path().join("valid-settings.json");
    let valid_settings = Settings::default();
    fs::write(&valid_path, valid_settings.to_json()?)?;
    if initialize_settings(&valid_path)? != SettingsInitialization::Loaded(valid_settings) {
        return Err(io::Error::other("valid settings were not loaded").into());
    }

    let invalid_path = directory.path().join("invalid-settings.json");
    let invalid_contents = b"{ malformed synthetic settings";
    fs::write(&invalid_path, invalid_contents)?;
    if initialize_settings(&invalid_path).is_ok() {
        return Err(io::Error::other("invalid settings were not rejected").into());
    }
    if fs::read(&invalid_path)? != invalid_contents {
        return Err(io::Error::other("invalid settings were modified").into());
    }

    println!("Outflow settings initialization ready");
    println!("Missing configuration: defaults initialized");
    println!("Valid configuration: loaded");
    println!("Invalid configuration: recovery required");
    println!("Original invalid file: preserved");
    Ok(())
}
