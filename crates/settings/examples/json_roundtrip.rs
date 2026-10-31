use outflow_settings::Settings;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let original = Settings::default();
    let json = original.to_json()?;
    let decoded = Settings::from_json(&json)?;
    decoded.validate()?;
    assert_eq!(decoded, original);

    println!("Outflow settings JSON round-trip passed");
    println!("Schema version: {}", decoded.schema_version);
    println!("Validation: passed");

    Ok(())
}
