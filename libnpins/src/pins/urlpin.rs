//! Pin a URL source
//!
//! It can pin plain URLs, or unpack them with `fetchTarball`.
//!
//! The mutable url pin allows for the same pattern as the "Lockable HTTP Tarball Protocol" in Nix
//! (<https://docs.lix.systems/manual/lix/nightly/protocols/tarball-fetcher.html#lockable-http-tarball-protocol>,
//! https://github.com/nixos/nix/blob/56763ff918eb308db23080e560ed2ea3e00c80a7/doc/manual/src/protocols/tarball-fetcher.md),
//! but without the associated issues.
//! (The protocol is in a fundamental and unresolvable violation of HTTP standards and how HTTP works.
//! The only acceptable resolution, which we implement, is to forgo on the automagic and explicitly expose this as a choice to the user.)

use anyhow::Result;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{GenericHash, Updatable, build_client, diff, nix};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct UrlPin {
    /// The static URL of the tarball
    pub url: Url,
    /// Whether to unpack it (use fetchTarball) or not (use fetchurl)
    pub unpack: bool,
}

impl diff::Diff for UrlPin {
    fn properties(&self) -> Vec<(String, String)> {
        vec![
            ("url".into(), self.url.to_string()),
            ("unpack".into(), self.unpack.to_string()),
        ]
    }
}

#[async_trait::async_trait]
impl Updatable for UrlPin {
    type Version = ();
    type Hashes = GenericHash;

    async fn update(&self, _old: Option<&()>) -> Result<()> {
        // Static URL, no versioning needed
        Ok(())
    }

    async fn fetch(&self, _version: &()) -> Result<Self::Hashes> {
        let hash = nix::nix_prefetch_url(&self.url, self.unpack).await?;
        Ok(Self::Hashes { hash })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct MutableUrlPin {
    /// The "update" URL, which is expected to redirect to a URL immutably pointing to the current content's snapshot.
    pub update_url: Url,
    /// Whether to unpack it (use fetchTarball) or not (use fetchurl)
    pub unpack: bool,
}

impl diff::Diff for MutableUrlPin {
    fn properties(&self) -> Vec<(String, String)> {
        vec![
            ("update_url".into(), self.update_url.to_string()),
            ("unpack".into(), self.unpack.to_string()),
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct LockedTarballVersion {
    /// The locked URL that immutably points to a specific content snapshot
    pub url: Url,
}

impl diff::Diff for LockedTarballVersion {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("url".into(), self.url.to_string())]
    }
}

#[async_trait::async_trait]
impl Updatable for MutableUrlPin {
    type Version = LockedTarballVersion;
    type Hashes = GenericHash;

    async fn update(&self, _old: Option<&LockedTarballVersion>) -> Result<LockedTarballVersion> {
        // HEAD our "mutable" url and follow all redirects to get our actual, "locked" url
        let url = build_client()?
            .head(self.update_url.clone())
            .send()
            .await?
            .url()
            .clone();
        Ok(LockedTarballVersion { url })
    }

    async fn fetch(&self, version: &LockedTarballVersion) -> Result<Self::Hashes> {
        let hash = nix::nix_prefetch_url(&version.url, self.unpack).await?;
        Ok(Self::Hashes { hash })
    }
}
