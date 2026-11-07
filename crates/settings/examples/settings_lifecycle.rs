use outflow_settings::{
    CleanupMode, SettingsChanges, SettingsSaveOutcome, SettingsService, SettingsServiceOpenStatus,
    VocabularyEntry,
};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn Error>> {
    let reset_requested = parse_args()?;
    let directory = create_temporary_directory()?;
    let settings_path = directory.join("settings.json");

    let mut service = SettingsService::open(&settings_path)?;
    print_snapshot("first run", &service);
    if service.open_status() != SettingsServiceOpenStatus::FirstRun {
        return Err("temporary settings path unexpectedly existed".into());
    }

    let clean_outcome = service.apply_changes(SettingsChanges {
        cleanup_mode: Some(CleanupMode::Clean),
        remove_fillers: Some(true),
        run_at_login: Some(false),
    })?;
    print_saved("preferences: clean", clean_outcome);
    let verbatim_outcome = service.apply_changes(SettingsChanges {
        cleanup_mode: Some(CleanupMode::Verbatim),
        remove_fillers: Some(false),
        run_at_login: Some(true),
    })?;
    print_saved("preferences: verbatim", verbatim_outcome);

    for (id, spoken, written) in [
        (
            "demo-entry-1",
            "synthetic north star",
            "Synthetic North Star",
        ),
        (
            "demo-entry-2",
            "synthetic quiet harbor",
            "Synthetic Quiet Harbor",
        ),
    ] {
        let outcome = service.add_vocabulary(VocabularyEntry {
            id: id.to_owned(),
            spoken: spoken.to_owned(),
            written: written.to_owned(),
        })?;
        print_saved("vocabulary add", outcome);
    }
    print_snapshot("saved", &service);

    drop(service);
    let mut service = SettingsService::open(&settings_path)?;
    print_snapshot("reloaded", &service);
    let readiness = service.confirm_current_vocabulary_review()?;
    println!(
        "Explicit vocabulary review acknowledged: YES ({})",
        readiness_label(&readiness)
    );

    let edit_outcome = service.edit_vocabulary(
        "demo-entry-1",
        "synthetic north star edited".to_owned(),
        "Synthetic North Star Edited".to_owned(),
    )?;
    print_saved("vocabulary edit", edit_outcome);
    let delete_outcome = service.delete_vocabulary("demo-entry-2")?;
    print_saved("vocabulary delete", delete_outcome);
    print_snapshot("edited and deleted", &service);

    drop(service);
    let mut service = SettingsService::open(&settings_path)?;
    print_snapshot("reloaded again", &service);
    println!("Temporary settings file: {}", settings_path.display());

    if reset_requested {
        let outcome = service.reset(true)?;
        print_saved("explicit reset", outcome);
        print_snapshot("reset defaults", &service);
    } else {
        println!("Explicit reset: NOT RUN (pass --reset to reset this temporary file)");
    }

    Ok(())
}

fn parse_args() -> Result<bool, Box<dyn Error>> {
    let mut reset_requested = false;
    for argument in env::args().skip(1) {
        if argument == "--reset" && !reset_requested {
            reset_requested = true;
        } else {
            return Err("usage: settings_lifecycle [--reset]".into());
        }
    }
    Ok(reset_requested)
}

fn create_temporary_directory() -> Result<PathBuf, Box<dyn Error>> {
    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let root = env::temp_dir();
    for attempt in 0..128_u32 {
        let directory = root.join(format!(
            "outflow-settings-lifecycle-{}-{timestamp}-{attempt}",
            std::process::id()
        ));
        match fs::create_dir(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.into()),
        }
    }
    Err("could not allocate an isolated temporary directory".into())
}

fn print_snapshot(label: &str, service: &SettingsService) {
    let settings = service.current();
    println!(
        "{label}: status={:?}, cleanup={:?}, remove_fillers={}, run_at_login={}, vocabulary_entries={}",
        service.open_status(),
        settings.cleanup.mode,
        settings.cleanup.remove_fillers,
        settings.startup.run_at_login,
        service.vocabulary().len(),
    );
}

fn print_saved(label: &str, outcome: SettingsSaveOutcome) {
    println!("{label}: {outcome:?}");
}

fn readiness_label(readiness: &outflow_settings::VocabularyMigrationReadiness) -> &'static str {
    match readiness {
        outflow_settings::VocabularyMigrationReadiness::EmptyCollection => "empty collection",
        outflow_settings::VocabularyMigrationReadiness::SourcePolicyUnknown { .. } => {
            "source policy remains unknown"
        }
        outflow_settings::VocabularyMigrationReadiness::ManualResolutionRequired { .. } => {
            "manual resolution required"
        }
        outflow_settings::VocabularyMigrationReadiness::CompatibleForExplicitMigration => {
            "compatible for explicit migration"
        }
        outflow_settings::VocabularyMigrationReadiness::AlreadyCurrent => "already current",
    }
}
