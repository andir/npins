//! Pin a Nix channel
//!
//! This should be preferred over pinning the equivaleng `nixpkgs` git branch.

use crate::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Pin {
    pub name: String,
}

impl diff::Diff for Pin {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("name".into(), self.name.clone())]
    }
}

impl Pin {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
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
    pub hash: String,
}

impl diff::Diff for ChannelHash {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("hash".into(), self.hash.clone())]
    }
}

#[async_trait::async_trait]
impl Updatable for Pin {
    type Version = ChannelVersion;
    type Hashes = ChannelHash;

    async fn update(&self, _old: Option<&ChannelVersion>) -> Result<ChannelVersion> {
        /* We want to get from something like https://channels.nixos.org/nixos-21.11
         * to https://releases.nixos.org/nixos/21.11/nixos-21.11.335807.df4f1f7cc3f/nixexprs.tar.xz
         */
        let url = build_client()?
            .head(&format!(
                "https://channels.nixos.org/{}/nixexprs.tar.xz",
                self.name
            ))
            .send()
            .await?
            .url()
            .clone();

        Ok(ChannelVersion { url })
    }

    async fn fetch(&self, version: &ChannelVersion) -> Result<ChannelHash> {
        /* Prefetch an URL that looks like
         * https://releases.nixos.org/nixos/21.11/nixos-21.11.335807.df4f1f7cc3f
         */
        let hash = nix::nix_prefetch_tarball(&version.url).await?;

        Ok(ChannelHash { hash })
    }
}
