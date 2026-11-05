use outflow_settings::{
    Settings, SettingsStorageError, VocabularyEntry, add_vocabulary_entry, delete_vocabulary_entry,
    edit_vocabulary_entry, load_settings, save_settings_atomic,
};
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
            .map_err(io::Error::other)?
            .as_nanos();
        for attempt in 0..128_u32 {
            let path = std::env::temp_dir().join(format!(
                "outflow-vocabulary-crud-example-{}-{timestamp}-{attempt}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }

        Err(io::Error::other(
            "could not create an isolated vocabulary example directory",
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

fn temporary_save_reload(
    directory: &TemporaryDirectory,
    entries: &[VocabularyEntry],
) -> Result<Option<usize>, Box<dyn Error>> {
    let path = directory.path().join("settings.json");
    let settings = Settings {
        vocabulary: entries.to_vec(),
        ..Settings::default()
    };

    match save_settings_atomic(&path, &settings) {
        Ok(()) => {}
        Err(SettingsStorageError::UnsupportedPlatform) => return Ok(None),
        Err(error) => return Err(error.into()),
    }

    let reloaded = load_settings(&path)?
        .ok_or_else(|| io::Error::other("settings file was missing after save"))?;
    if reloaded.vocabulary != settings.vocabulary {
        return Err(io::Error::other("vocabulary changed during settings round trip").into());
    }
    Ok(Some(reloaded.vocabulary.len()))
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut entries = Vec::new();
    add_vocabulary_entry(
        &mut entries,
        VocabularyEntry {
            id: "synthetic-entry-1".to_owned(),
            spoken: "synthetic spoken phrase".to_owned(),
            written: "Synthetic written phrase".to_owned(),
        },
    )?;
    println!("Add: success (entries: {})", entries.len());

    edit_vocabulary_entry(
        &mut entries,
        "synthetic-entry-1",
        "edited synthetic spoken phrase".to_owned(),
        "Edited synthetic written phrase".to_owned(),
    )?;
    println!("Edit: success (entries: {})", entries.len());

    let directory = TemporaryDirectory::new()?;
    match temporary_save_reload(&directory, &entries)? {
        Some(count) => println!("Temporary save/reload: success (entries: {count})"),
        None => println!(
            "Temporary save/reload: unavailable (entries: {})",
            entries.len()
        ),
    }

    delete_vocabulary_entry(&mut entries, "synthetic-entry-1")?;
    println!("Delete: success (entries: {})", entries.len());
    Ok(())
}
