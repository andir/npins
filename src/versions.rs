//! Versioning support for the save format

use super::*;
use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

/// The current format version
pub const LATEST: u64 = 0;

/// Custom manual deserialize wrapper that checks the version
pub fn from_value_versioned(value: Value) -> Result<NixPins> {
    let map = value.as_object().context("Top level must be an object")?;
    let version = map
        .get("version")
        .context("Top level must contain a version field")?;
    let version = version
        .as_u64()
        .context("Version field must be an integer (and not negative)")?;
    anyhow::ensure!(
        version <= LATEST,
        "Unknown version {}, maybe try updating the application?",
        version,
    );
    anyhow::ensure!(
        version == LATEST,
        "Version {} is too old, you need to run upgrade",
        version,
    );

    Ok(serde_json::from_value(value)?)
}

/// Custom manual serialize wrapper that adds a version field
pub fn to_value_versioned(pins: &NixPins) -> serde_json::Value {
    let mut raw = serde_json::to_value(pins).expect("Serialization should not fail");
    let map = raw
        .as_object_mut()
        .expect("Serialization should yield an object");
    map.insert("version".to_string(), json!(LATEST));

    raw
}

/// Patch the sources.json file to the latest version
///
/// This operates on a JSON value level
pub fn update(pins_raw: Map<String, Value>) -> Result<Value> {
    let version = pins_raw
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| {
            anyhow::format_err!(
                "sources.json must contain a numeric version field at the top level"
            )
        })?;

    /* This is where the upgrading happens (at the moment we don't have any versions to upgrade from) */
    match version {
        0 => {
            log::info!("sources.json is already up to date")
        },
        unknown => {
            anyhow::bail!(
                "Unknown format version {}, maybe try updating the application?",
                unknown
            );
        },
    }

    Ok(serde_json::Value::Object(pins_raw))
}
