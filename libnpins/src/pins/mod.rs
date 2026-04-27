//! The npins `Pin` base type
//! [`Pin`] is the main type for all pins, and then the variants are implemented in submodules.

use crate::*;
use anyhow::Context;
use serde::{Deserialize, Serialize};

pub mod channel;
pub mod container;
pub mod git;
pub mod pypi;
pub mod urlpin;

/// Create the `Pin` type
///
/// We need a type to unify over all possible way to pin a dependency. Normally, this would be done with a trait
/// and trait objects. However, designing such a trait to be object-safe turns out to be highly non-trivial.
/// (We'd need the `serde_erase` crate for `Deserialize` alone). Since writing this as an enum is extremely repetitive,
/// this macro does the work for you.
///
/// For each pin type, call it with `(Name, lower_name, human readable name, Type)`. `Name` will be the name of the enum variant,
/// `lower_name` will be used for the constructor.
macro_rules! mk_pin {
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
                    // TODO(piegames): This is the only field which is independent of the pin type and equal for all pins,
                    // eventually it should be factored out (especially once a second field of that kind is added)
                    #[serde(default, skip_serializing_if="Frozen::is_default")]
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
            pub async fn update(&mut self) -> ::anyhow::Result<Vec<diff::DiffEntry>> {
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
            pub async fn fetch(&mut self) -> ::anyhow::Result<Vec<diff::DiffEntry>> {
                Ok(match self {
                    $(Self::$name { input, version, hashes, .. } => {
                        let version = version.as_ref()
                            .context("No version information available, call `update` first or manually set one")?;
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

            /// Unfreeze a pin
            pub fn unfreeze(&mut self) {
                match self {
                    $(Self::$name { frozen, .. } => frozen.unfreeze()),*
                }
            }

            /// Freeze a pin
            pub fn freeze(&mut self) {
                match self {
                    $(Self::$name { frozen, .. } => frozen.freeze()),*
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

mk_pin! {
    (Git, git, "git repository", git::GitPin),
    (GitRelease, git_release, "git release tag", git::GitReleasePin),
    (PyPi, pypi, "pypi package", pypi::Pin),
    (Channel, channel, "Nix channel", channel::Pin),
    (Url, url, "url", urlpin::UrlPin),
    (MutableUrl, mutable_url, "mutable url", urlpin::MutableUrlPin),
    (Container, container, "OCI Container", container::Pin),
}
