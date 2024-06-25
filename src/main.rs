use std::path::PathBuf;

use anyhow::Result;
use diff::{Diff, OptionExt};
use reqwest::IntoUrl;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use structopt::StructOpt;

pub mod channel;
pub mod cli;
pub mod diff;
pub mod flake;
pub mod git;
pub mod niv;
pub mod nix;
pub mod pypi;
pub mod versions;

/// Helper method to build you a client.
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
    async fn update(&self, old: Option<&Self::Version>) -> Result<Self::Version>;

    /// Fetch hashes for a given version
    async fn fetch(&self, version: &Self::Version) -> Result<Self::Hashes>;
}

/// Create the `Pin` type
///
/// We need a type to unify over all possible way to pin a dependency. Normally, this would be done with a trait
/// and trait objects. However, designing such a trait to be object-safe turns out to be highly non-trivial.
/// (We'd need the `serde_erase` crate for `Deserialize` alone). Since writing this as an enum is extremely repetitive,
/// this macro does the work for you.
///
/// For each pin type, call it with `(Name, lower_name, human readable name, Type)`. `Name` will be the name of the enum variant,
/// `lower_name` will be used for the constructor.
macro_rules! mkPin {
    ( $(( $name:ident, $lower_name:ident, $human_name:expr, $input_name:path )),* $(,)? ) => {
        /* The type declaration */
        /// Enum over all possible pin types
        ///
        /// Every pin type has two parts, an `input` and an `output`. The input implements [`Updatable`], which
        /// will generate output in its most up-to-date form.
        #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
        #[serde(tag = "type")]
        pub enum Pin {
            $(
                /* One variant per type. input and output are serialized to a common JSON dict using `flatten`. Output is optional. */
                $name {
                    #[serde(flatten)]
                    input: $input_name,
                    #[serde(flatten)]
                    version: Option<<$input_name as Updatable>::Version>,
                    #[serde(flatten)]
                    hashes: Option<<$input_name as Updatable>::Hashes>,
		    #[serde(default)]
		    frozen: Frozen,
                }
            ),*
        }

        impl Pin {
            /* Constructors */
            $(fn $lower_name(input: $input_name, version: Option<<$input_name as Updatable>::Version>) -> Self {
                Self::$name { input, version, hashes: None, frozen: Frozen::default() }
            })*

            /* If an error is returned, `self` remains unchanged */
            async fn update(&mut self) -> Result<Vec<diff::DiffEntry>> {
                Ok(match self {
                    $(Self::$name { input, version, .. } => {
                        /* Use very explicit syntax to force the correct types and get good compile errors */
                        let new_version = <$input_name as Updatable>::update(input, version.as_ref()).await?;
                        version.insert_diffed(new_version)
                    }),*
                })
            }

            /* If an error is returned, `self` remains unchanged. This returns a double result: the outer one
             * indicates that `update` should be called first, the inner is from the actual operation.
             */
            async fn fetch(&mut self) -> Result<Vec<diff::DiffEntry>> {
                Ok(match self {
                    $(Self::$name { input, version, hashes, .. } => {
                        let version = version.as_ref()
                            .ok_or_else(|| anyhow::format_err!("No version information available, call `update` first or manually set one"))?;
                        /* Use very explicit syntax to force the correct types and get good compile errors */
                        let new_hashes = <$input_name as Updatable>::fetch(input, &version).await?;
                        hashes.insert_diffed(new_hashes)
                    }),*
                })
            }

            pub fn has_version(&self) -> bool {
                match self {
                    $(Self::$name { version, ..} => version.is_some() ),*
                }
            }

            pub fn has_hashes(&self) -> bool {
                match self {
                    $(Self::$name { hashes, ..} => hashes.is_some() ),*
                }
            }

            /// Human readable name of the pin type
            pub fn pin_type(&self) -> &'static str {
                match self {
                    $(Self::$name { ..} => $human_name ),*
                }
            }


	    /// Thaw a pin
	    pub fn thaw(&mut self) {
		match self {
		    $(Self::$name { ref mut frozen, .. } => frozen.thaw()),*
		}
	    }

	    /// Freeze a pin
	    pub fn freeze(&mut self) {
		match self {
		    $(Self::$name { ref mut frozen, .. } => frozen.freeze()),*
		}
	    }

	    /// Is frozen
	    pub fn is_frozen(&self) -> bool {
		match self {
		    $(Self::$name { frozen, .. } => frozen.is_frozen()),*
		}
	    }
        }

        impl std::fmt::Display for Pin {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    $(Self::$name { input, version, hashes, frozen } => {
                        /* Concat all properties and then print them */
                        let properties = input.properties().into_iter()
                            .chain(version.iter().flat_map(Diff::properties))
                            .chain(hashes.iter().flat_map(Diff::properties))
			    .chain(frozen.properties());
                        for (key, value) in properties {
                            writeln!(fmt, "    {}: {}", key, value)?;
                        }
                        Ok(())
                    }),*
                }
            }
        }

        // Each variant holds exactly one distinct type, so we can easily create convenient type wrappers that simply call the constructor
        $(
            impl From<$input_name> for Pin {
                fn from(input: $input_name) -> Self {
                    Self::$lower_name(input, None)
                }
            }

            impl From<($input_name, Option<<$input_name as Updatable>::Version>)> for Pin {
                fn from((input, version): ($input_name, Option<<$input_name as Updatable>::Version>)) -> Self {
                    Self::$lower_name(input, version)
                }
            }

            impl From<($input_name, <$input_name as Updatable>::Version)> for Pin {
                fn from((input, version): ($input_name, <$input_name as Updatable>::Version)) -> Self {
                    (input, Some(version)).into()
                }
            }
        )*
    };
}

mkPin! {
    (Git, git, "git repository", git::GitPin),
    (GitRelease, git_release, "git release tag", git::GitReleasePin),
    (PyPi, pypi, "pypi package", pypi::Pin),
    (Channel, channel, "Nix channel", channel::Pin),
}

/// The main struct the CLI operates on
///
/// For serialization purposes, use the `NixPinsVersioned` wrapper instead.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct NixPins {
    pins: BTreeMap<String, Pin>,
}

impl NixPins {
    pub fn new_with_nixpkgs() -> Self {
        let mut pins = BTreeMap::new();
        pins.insert(
            "nixpkgs".to_owned(),
            channel::Pin::new("nixpkgs-unstable").into(),
        );
        Self { pins }
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

/// The Frozen field in a Pin
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct Frozen(bool);

impl Frozen {
    fn new(value: bool) -> Self {
        Frozen(value)
    }

    fn freeze(&mut self) {
        self.0 = true;
    }

    fn thaw(&mut self) {
        self.0 = false;
    }

    fn is_frozen(&self) -> bool {
        self.0
    }
}

impl diff::Diff for Frozen {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("frozen".into(), self.0.to_string())]
    }
}

impl std::default::Default for Frozen {
    fn default() -> Self {
        Frozen(false)
    }
}

/// An URL and its hash
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GenericUrlHashes {
    pub url: url::Url,
    pub hash: String,
}

impl diff::Diff for GenericUrlHashes {
    fn properties(&self) -> Vec<(String, String)> {
        vec![
            ("url".into(), self.url.to_string()),
            ("hash".into(), self.hash.clone()),
        ]
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(None)
        .format_target(false)
        .init();

    let opts = cli::Opts::from_args();
    opts.run().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    fn test_frozen() {
        assert!(!Frozen::default().is_frozen());
        assert!(!{
            let mut x = Frozen::default();
            x.thaw();
            x
        }
        .is_frozen());
        assert!({
            let mut x = Frozen::default();
            x.freeze();
            x
        }
        .is_frozen());
        assert!(Frozen::new(true).is_frozen());
        assert!({
            let mut x = Frozen::new(true);
            x.freeze();
            x
        }
        .is_frozen());
        assert!(!{
            let mut x = Frozen::new(true);
            x.thaw();
            x
        }
        .is_frozen());
    }
}
