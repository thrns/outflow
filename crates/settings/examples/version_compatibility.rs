use outflow_settings::Settings;
use serde_json::Value;
use std::error::Error;

fn synthetic_document(schema_version: Option<u32>) -> Result<String, Box<dyn Error>> {
    let mut document: Value = serde_json::from_str(&Settings::default().to_json()?)?;

    match schema_version {
        Some(version) => document["schema_version"] = serde_json::json!(version),
        None => {
            document
                .as_object_mut()
                .ok_or("settings document must be an object")?
                .remove("schema_version");
        }
    }

    Ok(document.to_string())
}

fn require_rejected(document: &str) -> Result<(), Box<dyn Error>> {
    match Settings::from_json(document) {
        Err(_) => Ok(()),
        Ok(_) => Err("unsupported settings document was accepted".into()),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let v1 = synthetic_document(Some(1))?;
    Settings::from_json(&v1)?;

    println!("Outflow settings version policy ready");
    println!("Schema v1: supported");

    for version in [0, 2] {
        let document = synthetic_document(Some(version))?;
        require_rejected(&document)?;
        println!("Schema v{version}: unsupported");
    }

    let missing_version = synthetic_document(None)?;
    require_rejected(&missing_version)?;
    println!("Missing version: rejected");

    Ok(())
}
