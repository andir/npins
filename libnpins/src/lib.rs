//! The npins library
//!
//! Currently, it pretty much exposes the internals of the CLI 1:1, but in the future
//! this is supposed to evolve into a more standalone library.

use anyhow::Context;
use diff::{Diff, OptionExt};
use nix_compat::nixhash::NixHash;
use reqwest::IntoUrl;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use url::Url;

pub mod pins;
pub use pins::*;

pub mod diff;
pub mod flake;
pub mod niv;
pub mod nix;
pub mod versions;

pub const DEFAULT_NIX: &str = include_str!("default.nix");

/// Helper method to build you a client.
// TODO make injectable via a configuration mechanism
pub fn build_client() -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            " v",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
}

/// Helper method for doing various API calls
async fn get_and_deserialize<T, U>(url: U) -> anyhow::Result<T>
where
    T: for<'a> Deserialize<'a> + 'static,
    U: IntoUrl,
{
    let response = build_client()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    Ok(serde_json::from_str(&response)?)
}

/// Issue a http GET request to an URL without actually fetching its output,
/// as a quick sanity check for its validity.
/// This is meant as a check on the unhappy path to improve error messages:
/// If `git ls-remote` or `nix-prefetch` fails for some reason and the URL already fails this simple
/// HTTP check, we can ignore the error message from these tools and replace it with our own.
///
/// If `result` is `Ok`, it is passed on unchanged and nothing is done.
/// If `result` is `Err`, the check will be executed and the error replaced in case of failure.
async fn check_url<T>(result: anyhow::Result<T>, url: &str) -> anyhow::Result<T> {
    if result.is_ok() {
        return result;
    }

    let url: Url = url.parse()?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return result;
    }

    log::debug!("Checking {url}");
    /* Note that *in theory* we should be able to use a HEAD request instead of GET, however
     * several HTTP servers don't comply with that so we have to GET and then throw away the content instead.
     * Some return 405 Method Not Allowed which would be fine, however GitLab for example simply returns
     * 403 Forbidden on HEAD for an URL that is 200 on GET.
     */
    let Err(response_error) = build_client()?.get(url).send().await?.error_for_status() else {
        return result;
    };

    result.context(response_error)
}

/// The git url to a repo has no defined endpoint in the protocol, and thus
/// may not be routed by all web servers. However, $GIT_REMOTE/info/refs is
/// a valid endpoint that MUST be implemented by all git servers.
/// https://git-scm.com/docs/http-protocol
async fn check_git_url<T>(result: anyhow::Result<T>, git_url: &str) -> anyhow::Result<T> {
    check_url(
        result,
        &format!("{git_url}/info/refs?service=git-upload-pack"),
    )
    .await
}

/// The main trait implemented by all pins
///
/// It comes with two associated types, `Version` and `Hashes`. Together, each of these types
/// must satisfy the following invariants:
/// - They serialize to a map/dictionary/object, however you want to call it
/// - **The serialized dictionaries of all are disjoint** (unchecked invariant at the moment)
#[async_trait::async_trait]
pub trait Updatable:
    Serialize
    + Deserialize<'static>
    + std::fmt::Debug
    + Clone
    + PartialEq
    + Eq
    + std::hash::Hash
    + diff::Diff
{
    /// Version information, produced by the [`update`](Self::update) method.
    ///
    /// It should contain information that charactarizes a version, e.g. the release version.
    /// A user should be able to manually specify it, if they want to pin a specific version.
    /// Each version should map to the same set of hashes over time, and violations of this
    /// should only be caused by upstream errors.
    type Version: diff::Diff
        + Serialize
        + Deserialize<'static>
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Eq;

    /// The pinned hashes for a given version, produced by the [`fetch`](Self::fetch) method.
    ///
    /// It may contain multiple different hashes, or download URLs that go with them.
    type Hashes: diff::Diff
        + Serialize
        + Deserialize<'static>
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Eq;

    /// Fetch the latest applicable commit data
    ///
    /// The old version may be passed to help guarantee monotonicity of the versions.
    async fn update(&self, old: Option<&Self::Version>) -> anyhow::Result<Self::Version>;

    /// Fetch hashes for a given version
    async fn fetch(&self, version: &Self::Version) -> anyhow::Result<Self::Hashes>;
}

/// The main struct the CLI operates on
///
/// For serialization purposes, use the `NixPinsVersioned` wrapper instead.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct NixPins {
    pub pins: BTreeMap<String, Pin>,
}

impl NixPins {
    /// Create a new `NixPins` with a pin `nixpkgs` pointing to the `nixpkgs-unstable` channel
    pub fn new_with_nixpkgs() -> Self {
        let mut pins = BTreeMap::new();
        pins.insert(
            "nixpkgs".to_owned(),
            channel::Pin::new("nixpkgs-unstable", channel::NIXPKGS_ARTIFACT).into(),
        );
        Self { pins }
    }

    /// Custom manual deserialize wrapper that checks the version
    pub fn from_json_versioned(value: serde_json::Value) -> anyhow::Result<Self> {
        versions::from_value_versioned(value)
    }

    /// Custom manual serialize wrapper that adds a version field
    pub fn to_value_versioned(&self) -> serde_json::Value {
        versions::to_value_versioned(self)
    }
}

/// Just a version string
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GenericVersion {
    /// Note that "version" must be seen in the context of the pin.
    /// Without that context, it shall be treated as opaque string.
    pub version: String,
}

impl diff::Diff for GenericVersion {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("version".into(), self.version.clone())]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GenericHash {
    pub hash: NixHash,
}

impl diff::Diff for GenericHash {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("hash".into(), self.hash.to_string())]
    }
}

/// The Frozen field in a Pin
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Frozen(pub bool);

impl Frozen {
    fn freeze(&mut self) {
        self.0 = true;
    }

    fn unfreeze(&mut self) {
        self.0 = false;
    }

    fn is_frozen(&self) -> bool {
        self.0
    }

    fn is_default(&self) -> bool {
        self == &Frozen::default()
    }
}

impl diff::Diff for Frozen {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("frozen".into(), self.0.to_string())]
    }
}

/// An URL and its hash
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GenericUrlHashes {
    pub url: url::Url,
    pub hash: NixHash,
}

impl diff::Diff for GenericUrlHashes {
    fn properties(&self) -> Vec<(String, String)> {
        vec![
            ("url".into(), self.url.to_string()),
            ("hash".into(), self.hash.to_string()),
        ]
    }
}

fn elide_to_first_line(input_string: &str) -> String {
    let mut lines = input_string.lines();
    let value = lines.next().unwrap_or("(unknown)");

    // Add the ellipsis at the end of the string if needed.
    match lines.next() {
        // Only one line.
        None => value.to_string(),
        // Multi-line
        Some(_) => {
            format!("{}…", value)
        },
    }
}

/// Formats a command in a shell-safe manner.
///
/// NOTE: Multi-line components will be elided to their first line!
pub(crate) fn format_command(tokio_cmd: &tokio::process::Command) -> anyhow::Result<String> {
    // `tokio::process`'s `Command` doesn't allow introspecting, so let's ignore that
    // and use it as a `std::process::Command`.
    let cmd = tokio_cmd.as_std();

    let mut command_parts: Vec<String> = Vec::new();

    // Format and escape environment
    let envs: Vec<String> =
        cmd.get_envs()
            .map(|(name, value)| {
                let value = elide_to_first_line(
                value
                .expect("Environment variable for command should have an environment variable name")
                .to_str()
                .unwrap_or("(unknown)")
            );

                // Escape the value only.
                let value = shlex::try_quote(&value).unwrap_or("(invalid)".into());

                // The escaping would produce 'NAME=VA LUE', which will not produce a strings that can
                // be used for passing an environment variables to a command.
                let name = name.to_str().unwrap_or("(unknown)");

                format!("{}={}", name, value)
            })
            .collect();

    // Add to the command
    command_parts.extend(envs);

    // Use the basename of the command.
    let exe_name = std::path::Path::new(cmd.get_program())
        .file_name()
        .unwrap_or(std::ffi::OsStr::new("(unknown)"))
        .to_str()
        .unwrap_or("(unknown)");

    command_parts.push(exe_name.into());

    // Escape all args
    let args: Vec<String> = cmd
        .get_args()
        .map(|s| {
            let arg = s.to_str().unwrap_or("(unknown)");
            shlex::try_quote(&elide_to_first_line(arg))
                .unwrap_or("(invalid)".into())
                .into()
        })
        .collect();

    // Add to the command
    command_parts.extend(args);

    Ok(command_parts.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[rustfmt::skip]
    fn test_frozen() {
        assert!(!Frozen::default().is_frozen());
        assert!(!{
            let mut x = Frozen::default();
            x.unfreeze();
            x
        }
        .is_frozen());
        assert!({
            let mut x = Frozen::default();
            x.freeze();
            x
        }
        .is_frozen());
        assert!(Frozen(true).is_frozen());
        assert!({
            let mut x = Frozen(true);
            x.freeze();
            x
        }
        .is_frozen());
        assert!(!{
            let mut x = Frozen(true);
            x.unfreeze();
            x
        }
        .is_frozen());
    }
}
