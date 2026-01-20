//! Pin a tarball URL source
//!
//! Optionally (if the host supports it) can use the "Lockable HTTP Tarball Protocol" from flakes.
//! Reference: <https://github.com/nixos/nix/blob/56763ff918eb308db23080e560ed2ea3e00c80a7/doc/manual/src/protocols/tarball-fetcher.md>

use anyhow::{Context, Result};
use reqwest::{
    header::{HeaderName, LOCATION},
    redirect::Policy,
};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::*;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct TarballPin {
    /// URL provided as user input
    pub url: Url,
}

impl diff::Diff for TarballPin {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("url".into(), self.url.to_string())]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct LockedTarball {
    /// If the given URL supports the Lockable Tarball Protocol we store the
    /// flakeref here
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked_url: Option<Url>,
}

impl diff::Diff for LockedTarball {
    fn properties(&self) -> Vec<(String, String)> {
        self.locked_url
            .iter()
            .map(|locked_url| ("locked_url".into(), locked_url.to_string()))
            .collect()
    }
}

#[async_trait::async_trait]
impl Updatable for TarballPin {
    type Version = LockedTarball;
    type Hashes = GenericHash;

    async fn update(&self, old: Option<&LockedTarball>) -> Result<LockedTarball> {
        const LINK: HeaderName = HeaderName::from_static("link");

        // Attempt to use the Lockable HTTP Tarball Protocol, if that fails (the
        // expected Link header is missing) we fail back to using whatever was
        // the input.

        // FIXME: The client is built without the helper function as it currently cannot be
        // configured
        let client = reqwest::Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                " v",
                env!("CARGO_PKG_VERSION")
            ))
            .redirect(Policy::none())
            .build()?;

        let mut target = self.url.as_str();
        let mut res;

        loop {
            res = client.head(target).send().await?;

            let flakerefs = res
                .headers()
                .get_all(LINK)
                .into_iter()
                .filter_map(|header| header.to_str().ok())
                .filter_map(|link| {
                    // Naive parsing of the `Link: <flakeref>; rel="immutable"` header
                    link.strip_suffix(r#">; rel="immutable""#)?
                        .strip_prefix("<")
                })
                .collect::<Vec<_>>();

            if let [flakeref] = flakerefs[..] {
                let locked_url = Some(
                    flakeref
                        .parse::<Url>()
                        .context("immutable link contained an invalid URL")?,
                );

                return Ok(LockedTarball { locked_url });
            }

            if res.status().is_redirection() {
                let error = &format!(
                    "no location given in the redirection response from {}",
                    res.url()
                );

                target = res
                    .headers()
                    .get(LOCATION)
                    .expect(error)
                    .to_str()
                    .expect(error);
            } else {
                break;
            }
        }

        if matches!(old, Some(old) if old.locked_url.is_some()) {
            log::warn!(
                    "url `{url}` of a locked tarball pin did not respond with the expected `Link` header. \
                     if you changed the `url` manually to one that doesn't support this protocol make sure to also remove the `locked_url` field. \
                     https://docs.lix.systems/manual/lix/nightly/protocols/tarball-fetcher.html",
                    url = &self.url,
                );
        }

        Ok(old.unwrap().clone())
    }

    async fn fetch(&self, version: &LockedTarball) -> Result<Self::Hashes> {
        let url = version.locked_url.as_ref().unwrap_or(&self.url);
        let hash = nix::nix_prefetch_tarball(&url).await?;
        Ok(Self::Hashes { hash })
    }
}
