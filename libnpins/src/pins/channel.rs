//! Pin a Nix channel
//!
//! This should be preferred over pinning the equivaleng `nixpkgs` git branch.

use nix_compat::nixhash::NixHash;
use serde::{Deserialize, Serialize};

use crate::{Updatable, build_client, diff, nix};

/// Stability note: this may change over time as upstream provides other compression algorithms
pub const NIXPKGS_ARTIFACT: &'static str = "nixexprs.tar.xz";

fn default_artifact_path() -> String {
    NIXPKGS_ARTIFACT.into()
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Pin {
    pub name: String,
    #[serde(default = "default_artifact_path")]
    pub artifact: String,
}

impl diff::Diff for Pin {
    fn properties(&self) -> Vec<(String, String)> {
        vec![
            ("name".into(), self.name.clone()),
            ("artifact".into(), self.artifact.clone()),
        ]
    }
}

impl Pin {
    pub fn new(name: impl Into<String>, artifact: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            artifact: artifact.into(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ChannelVersion {
    pub url: url::Url,
}

impl diff::Diff for ChannelVersion {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("url".into(), self.url.to_string())]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ChannelHash {
    pub hash: NixHash,
}

impl diff::Diff for ChannelHash {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("hash".into(), self.hash.to_string())]
    }
}

#[async_trait::async_trait]
impl Updatable for Pin {
    type Version = ChannelVersion;
    type Hashes = ChannelHash;

    async fn update(&self, _old: Option<&ChannelVersion>) -> anyhow::Result<ChannelVersion> {
        /* We want to get from something like https://channels.nixos.org/nixos-21.11
         * to https://releases.nixos.org/nixos/21.11/nixos-21.11.335807.df4f1f7cc3f/nixexprs.tar.xz
         */
        let url = build_client()?
            .head(format!(
                "https://channels.nixos.org/{}/{}",
                self.name, self.artifact,
            ))
            .send()
            .await?
            .url()
            .clone();

        Ok(ChannelVersion { url })
    }

    async fn fetch(&self, version: &ChannelVersion) -> anyhow::Result<Self::Hashes> {
        /* Prefetch an URL that looks like
         * https://releases.nixos.org/nixos/21.11/nixos-21.11.335807.df4f1f7cc3f
         */
        let hash = nix::nix_prefetch_tarball(&version.url).await?;
        Ok(Self::Hashes { hash })
    }
}
