//! Convert+Import Nix flake lock files

use crate::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;
use url::Url;

/// Pin entry from a nix flake's lock file
///
/// Flake locks have a two-part structure: the input's specification, and the
/// actual pin itself (under `locked`). We need aspects of both, but ignore the
/// other attributes (e.g. whether an input is a flake or not)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlakePin {
    locked: FlakeLocked,
    original: FlakeOriginal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum FlakeType {
    Gitlab,
    Github,
    Git,
    Path,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlakeLocked {
    /// repository owner on GitHub, or repository prefix on GitLab
    owner: Option<String>,
    /// repository name on GitHub and GitLab
    repo: Option<String>,
    /// the url of a generic git input
    url: Option<Url>,
    #[serde(rename = "type")]
    type_: FlakeType,
    /// git ref in all git input types
    #[serde(rename = "ref")]
    ref_: Option<String>,
    /// the input's hash. not used, but kept here in case we want to implement
    /// also importing the pins themselves
    #[serde(rename = "narHash")]
    nar_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlakeOriginal {
    /// git ref in git input types where a branch is referred to, but locked
    #[serde(rename = "ref")]
    ref_: Option<String>,
    #[serde(rename = "type")]
    type_: String,
}

impl FlakePin {
    pub fn is_indirect(&self) -> bool {
        self.original.type_ == "indirect"
    }
}

impl TryFrom<FlakePin> for Pin {
    type Error = anyhow::Error;

    fn try_from(flake: FlakePin) -> Result<Self> {
        use FlakeType::*;

        // "indirect" inputs (i.e. dependencies of flake dependencies) are
        // not supported for now
        assert_ne!(flake.original.type_, "indirect");

        Ok(match flake.locked.type_ {
            Gitlab => git::GitPin::gitlab(
                format!(
                    "{}/{}",
                    flake
                        .locked
                        .owner
                        .context("missing field owner in gitlab flake input")?,
                    flake
                        .locked
                        .repo
                        .context("missing field repo in gitlab flake input")?
                ),
                // I am not sure if there is any documentation on this format,
                // but if no ref is present, it appears always `master` is meant
                flake.original.ref_.unwrap_or_else(|| "master".to_owned()),
                None,
                None,
                false,
            )
            .into(),
            Github => git::GitPin::github(
                flake
                    .locked
                    .owner
                    .context("missing owner field in github flake input")?,
                flake
                    .locked
                    .repo
                    .context("missing field repo in github flake input")?,
                flake.original.ref_.unwrap_or_else(|| "master".to_owned()),
                false,
            )
            .into(),
            Git => {
                let mut ref_ = flake
                    .locked
                    .ref_
                    .context("missing ref on git flake input")?;
                if let Some(shortened) = ref_.strip_prefix("refs/heads/") {
                    ref_ = shortened.to_string();
                }
                git::GitPin::git(
                    flake.locked.url.context("missing url on git flake input")?,
                    ref_,
                    false,
                )
                .into()
            },
            Path => anyhow::bail!("Path inputs are currently not supported by npins."),
        })
    }
}
