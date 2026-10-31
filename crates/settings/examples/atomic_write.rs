use outflow_settings::{Settings, save_settings_atomic};
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
                "outflow-atomic-example-{}-{timestamp}-{attempt}",
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

    fn settings_path(&self) -> PathBuf {
        self.0.join("settings.json")
    }
}

impl Drop for ExampleDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = ExampleDirectory::create()?;
    let path = directory.settings_path();
    let original = Settings::default();

    save_settings_atomic(&path, &original)?;
    let json = fs::read_to_string(path)?;
    let decoded = Settings::from_json(&json)?;
    if decoded != original {
        return Err("stored settings differ from the original configuration".into());
    }

    println!("Outflow atomic settings write passed");
    println!("Settings validation: passed");
    println!("JSON file: verified");
    Ok(())
}
