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
    pub filename: String,
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
            .get(&format!(
                "https://channels.nixos.org/{}/nixexprs.tar.xz",
                self.name
            ))
            .send()
            .await?
            .url()
            .clone();

        Ok(ChannelVersion {
            url: url.clone(),
            filename: Self::calc_filename(&url)?,
        })
    }

    async fn fetch(&self, version: &ChannelVersion) -> Result<ChannelHash> {
        /* Prefetch an URL that looks like
         * https://releases.nixos.org/nixos/21.11/nixos-21.11.335807.df4f1f7cc3f
         */
        let hash =
            nix::nix_prefetch_tarball(&version.url, Some(Self::calc_filename(&version.url)?)).await?;

        Ok(ChannelHash { hash })
    }
}

impl Pin {
    pub fn calc_filename(url: &url::Url) -> Result<String> {
        let name_parts = url.path_segments().unwrap().collect::<Vec<&str>>();
        let basename = name_parts
            .get(1)
            .ok_or_else(|| anyhow::format_err!("Unsupported channel URL {}", url))?;
        let ending = name_parts
            .last()
            .ok_or_else(|| anyhow::format_err!("Unsupported channel URL {}", url))?
            .split('.')
            .skip(1)
            .collect::<Vec<&str>>();
        Ok(format!("npins-channel-{}.{}", basename, ending.join(".")))
    }
}
