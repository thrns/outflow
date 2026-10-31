use outflow_settings::{CleanupMode, Settings};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let settings = Settings::default();
    settings.validate()?;

    let language = match settings.language.as_str() {
        "en" => "English",
        _ => "Unsupported",
    };
    let cleanup = match settings.cleanup.mode {
        CleanupMode::Clean => "Clean",
        CleanupMode::Verbatim => "Verbatim",
    };
    let launch_at_login = if settings.startup.run_at_login {
        "On"
    } else {
        "Off"
    };

    println!("Outflow settings schema v{} ready", settings.schema_version);
    println!("Language: {language}");
    println!("Cleanup: {cleanup}");
    println!("Launch at Login: {launch_at_login}");
    Ok(())
}
