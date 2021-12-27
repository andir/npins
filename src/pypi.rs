//! Pin a PyPi package

use crate::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct PinInput {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PinOutput {
    pub version: String,
    pub hash: String,
    pub url: String,
}

impl diff::Diff for PinOutput {
    fn diff(&self, other: &Self) -> Vec<diff::Difference> {
        diff::d(&[
            diff::Difference::new("version", &self.version, &other.version),
            diff::Difference::new("hash", &self.hash, &other.hash),
            diff::Difference::new("url", &self.url, &other.url),
        ])
    }
}

#[async_trait::async_trait]
impl Updatable for PinInput {
    type Output = PinOutput;

    async fn update(&self) -> Result<PinOutput> {
        /* Fetch the JSON metadata for a Pypi package.
         * Url template: `https://pypi.org/pypi/$pname/json`
         * JSON schema (as in the returned value): https://warehouse.pypa.io/api-reference/json.html
         */
        let metadata: PyPiMetadata =
            get_and_deserialize(format!("https://pypi.org/pypi/{}/json", &self.name))
                .await
                .context("Could not fetch Pypi metadata")?;

        let mut latest_source: PyPiUrlMetadata = metadata.urls
            .into_iter()
            /* Of all files for the latest release, we only care about source tarballs */
            .filter(|file_meta| file_meta.python_version == "source")
            .next()
            .ok_or_else(|| anyhow::format_err!(
                "JSON metadata is invalid: must contain exactly one entry with \"python_version\": \"source\"",
            ))?;

        let hash = latest_source.digests.remove("sha256").ok_or_else(|| {
            anyhow::format_err!(
                "JSON metadata is invalid: must contain a `sha256` entry within `digests`",
            )
        })?;

        Ok(PinOutput {
            version: metadata.info.version,
            hash,
            url: latest_source.url,
        })
    }
}

/// The actual JSON file is rather large, we only deserialize what we are interested in,
/// and only up to the granularity we are interested in.
/// JSON API specification: <https://warehouse.pypa.io/api-reference/json.html>
#[derive(Debug, Deserialize)]
struct PyPiMetadata {
    pub info: PyPiInfoMetadata,
    /// This contains all data for the latest release
    pub urls: Vec<PyPiUrlMetadata>,
}

// Again, this is not complete
#[derive(Debug, Deserialize)]
struct PyPiUrlMetadata {
    digests: HashMap<String, String>,
    filename: String,
    python_version: String,
    url: String,
}

#[derive(Debug, Deserialize)]
struct PyPiInfoMetadata {
    version: String,
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_pypi_update() -> Result<()> {
        /* Last release has been in 2016, there are separate packages for major releases.
         * There's no way this will get an update anymore.
         */
        let pin = PinInput {
            name: "gaiatest".into(),
        };
        let output = pin.update().await?;
        assert_eq!(
            output,
            PinOutput {
                version: "0.34".into(),
                hash: "3953b158b7b690642d68cd6beb1d59f6e10526f2ee10a6fb4636a913cc95e718".into(),
                url: "https://files.pythonhosted.org/packages/d1/d5/0c270c22d61ff6b883d0f24956f13e904b131b5ac2829e0af1cda99d70b1/gaiatest-0.34.tar.gz".into(),
            }
        );
        Ok(())
    }
}
