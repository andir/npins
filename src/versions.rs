//! Versioning support for the save format

use super::*;
use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

/// The current format version
pub const LATEST: u64 = 3;

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
pub fn upgrade(mut pins_raw: Map<String, Value>) -> Result<Value> {
    let version = pins_raw
        .get("version")
        .and_then(Value::as_u64)
        .ok_or_else(|| {
            anyhow::format_err!(
                "sources.json must contain a numeric version field at the top level"
            )
        })?;

    /* This is where the upgrading happens (at the moment we don't have any versions to upgrade from) */
    match version {
        0 => {
            let pins = pins_raw
                .get_mut("pins")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| anyhow::format_err!("sources.json must contain a `pins` object"))?;
            for (name, pin) in pins.iter_mut() {
                upgrade_v0_pin(
                    name,
                    pin.as_object_mut()
                        .ok_or_else(|| anyhow::format_err!("Pin {} must be an object", name))?,
                )
                .context(anyhow::format_err!(
                    "Pin {} could not be upgraded to the latest format version",
                    name
                ))?;
            }
        },
        1 => {
            log::info!("There is nothing to do");
        },
        2 => {
            log::info!("There is nothing to do");
        },
        3 => {
            log::info!("sources.json is already up to date");
        },
        unknown => {
            anyhow::bail!(
                "Unknown format version {}, maybe try updating the application?",
                unknown
            );
        },
    }

    /* Set the new version */
    *pins_raw.get_mut("version").unwrap() = json!(LATEST);

    Ok(serde_json::Value::Object(pins_raw))
}

/* Rename a bunch of keys in a (JSON) map. Keys that are not
 * present will be ignored.
 */
macro_rules! rename {
    ($map:expr, $($old:expr => $new:expr),* $(,)? ) => {{
        $(
            if let Some(val) = $map.remove($old) {
                $map.insert($new.into(), val);
            }
        )*
    }}
}

fn upgrade_v0_pin(name: &str, raw_pin: &mut Map<String, Value>) -> Result<()> {
    log::debug!("Updating {} to v1", name);

    /* Only the fields we care about */
    #[derive(Deserialize)]
    #[serde(tag = "type")]
    enum OldPin {
        GitHub {
            repository: String,
            owner: String,
            revision: Option<String>,
        },
        GitHubRelease {
            repository: String,
            owner: String,
        },
        Git {
            repository_url: url::Url,
        },
        /* Don't care */
        PyPi {},
        #[serde(other)]
        Invalid,
    }
    let pin: OldPin = serde_json::from_value(serde_json::Value::Object(raw_pin.clone()))?;
    match pin {
        OldPin::GitHub {
            owner,
            repository,
            revision,
            ..
        } => {
            raw_pin.insert("type".into(), json!("Git"));
            raw_pin.remove("repository");
            raw_pin.remove("owner");
            raw_pin.insert(
                "repository".into(),
                json!({
                    "type": "GitHub",
                    "owner": owner,
                    "repo": repository,
                }),
            );
            if let Some(revision) = revision {
                raw_pin.insert(
                    "url".into(),
                    json!(format!(
                        "https://github.com/{}/{}/archive/{}.tar.gz",
                        owner, repository, revision
                    )),
                );
            }
        },
        OldPin::GitHubRelease {
            owner, repository, ..
        } => {
            /* Our version parsing has changed between versions. */
            log::warn!("Upgrading pin {} might induce small semantic changes. Please check the diff afterwards and run `update`!", name);

            raw_pin.insert("type".into(), json!("GitRelease"));
            raw_pin.remove("repository");
            raw_pin.remove("owner");
            raw_pin.insert(
                "repository".into(),
                json!({
                    "type": "GitHub",
                    "owner": owner,
                    "repo": repository,
                }),
            );
            rename!(raw_pin, "release_name" => "version");

            /* Remove those fields because we'd need to additionally provide a "revision", which we can't. */
            raw_pin.remove("tarball_url");
            raw_pin.remove("hash");
        },
        OldPin::Git { repository_url, .. } => {
            raw_pin.remove("repository_url");
            raw_pin.insert(
                "repository".into(),
                json!({
                    "type": "Git",
                    "url": repository_url,
                }),
            );
        },
        /* Do nothing here */
        OldPin::PyPi { .. } => {},
        OldPin::Invalid => anyhow::bail!("Unknown pin type {}", raw_pin["type"]),
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::BTreeMap;

    macro_rules! btreemap {
        ( $($key:expr => $val:expr),* $(,)? ) => {{
            #[allow(unused_mut)]
            let mut map = BTreeMap::new();
            $(
                map.insert($key, $val);
            )*
            map
        }}
    }

    fn init_logger() {
        let _ = env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .format_timestamp(None)
            .format_target(false)
            .try_init();
    }

    #[test]
    fn test_v0() {
        init_logger();

        let pins = match json!({
            "pins": {
                "nixos-mailserver": {
                    "type": "Git",
                    "repository_url": "https://gitlab.com/simple-nixos-mailserver/nixos-mailserver.git",
                    "branch": "nixos-21.11",
                    "revision": "6e3a7b2ea6f0d68b82027b988aa25d3423787303",
                    "hash": "1i56llz037x416bw698v8j6arvv622qc0vsycd20lx3yx8n77n44"
                },
                "nixpkgs": {
                    "type": "GitHub",
                    "repository": "nixpkgs",
                    "owner": "nixos",
                    "branch": "nixpkgs-unstable",
                    "revision": "5c37ad87222cfc1ec36d6cd1364514a9efc2f7f2",
                    "hash": "1r74afnalgcbpv7b9sbdfbnx1kfj0kp1yfa60bbbv27n36vqdhbb"
                },
                "streamlit": {
                    "type": "PyPi",
                    "name": "streamlit",
                    "version": "1.3.1",
                    "hash": "adec7935c9cf774b9115b2456cf2f48c4f49b9f67159a97db0fe228357c1afdf",
                    "url": "https://files.pythonhosted.org/packages/c3/9d/ac871992617220442832af12c3808716f4349ab05ff939d695fe8b542f00/streamlit-1.3.1.tar.gz"
                },
                "youtube-dl": {
                    "type": "GitHubRelease",
                    "repository": "youtube-dl",
                    "owner": "ytdl-org",
                    "tarball_url": "https://api.github.com/repos/ytdl-org/youtube-dl/tarball/2021.12.17",
                    "release_name": "youtube-dl 2021.12.17",
                    "hash": "0a0ljx67q0gh8wza84gk33g31v02bd0a7lzawhn33i42iihms2w7"
                }
            },
            "version": 0
        }) {
            Value::Object(pins) => pins,
            _ => unreachable!(),
        };
        let pins = upgrade(pins).expect("Failed to upgrade data");
        let pins = serde_json::from_value::<NixPins>(pins)
            .expect("Upgraded data failed to deserialize with newest code");

        assert_eq!(
            pins,
            NixPins {
                pins: btreemap![
                        "nixos-mailserver".into() => Pin::Git {
                            input: git::GitPin::git("https://gitlab.com/simple-nixos-mailserver/nixos-mailserver.git".parse().unwrap(), "nixos-21.11".into()),
                            version: Some(git::GitRevision { revision: "6e3a7b2ea6f0d68b82027b988aa25d3423787303".into() }),
                            hashes: Some(git::OptionalUrlHashes { url: None, hash: "1i56llz037x416bw698v8j6arvv622qc0vsycd20lx3yx8n77n44".into() } ),
                frozen: Frozen::default(),
                        },
                        "nixpkgs".into() => Pin::Git {
                            input: git::GitPin::github("nixos", "nixpkgs", "nixpkgs-unstable".into()),
                            version: Some(git::GitRevision { revision: "5c37ad87222cfc1ec36d6cd1364514a9efc2f7f2".into() }),
                            hashes: Some(git::OptionalUrlHashes { url: Some("https://github.com/nixos/nixpkgs/archive/5c37ad87222cfc1ec36d6cd1364514a9efc2f7f2.tar.gz".parse().unwrap()), hash: "1r74afnalgcbpv7b9sbdfbnx1kfj0kp1yfa60bbbv27n36vqdhbb".into() }),
                frozen: Frozen::default(),
                        },
                        "streamlit".into() => Pin::PyPi {
                            input: pypi::Pin { name: "streamlit".into(), version_upper_bound: None },
                            version: Some(GenericVersion { version: "1.3.1".into() }),
                            hashes: Some(GenericUrlHashes { url: "https://files.pythonhosted.org/packages/c3/9d/ac871992617220442832af12c3808716f4349ab05ff939d695fe8b542f00/streamlit-1.3.1.tar.gz".parse().unwrap(), hash: "adec7935c9cf774b9115b2456cf2f48c4f49b9f67159a97db0fe228357c1afdf".into() } ),
                frozen: Frozen::default(),
                        },
                        "youtube-dl".into() => Pin::GitRelease {
                            input: git::GitReleasePin::github("ytdl-org", "youtube-dl", false, None, None),
                            version: Some(GenericVersion { version: "youtube-dl 2021.12.17".into() }),
                            hashes: None,
                frozen: Frozen::default(),
                        }
                    ],
            }
        );
    }
}
