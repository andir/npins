use crate::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PinInput {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
        let metadata = fetch_metadata(&self.name)
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

/// Fetch the JSON metadata for a Pypi package.
///
/// Url template: `https://pypi.org/pypi/$pname/json`
/// JSON schema (as in the returned value): TODO link to documentation?
async fn fetch_metadata(pname: &str) -> Result<PyPiMetadata> {
    let response = reqwest::get(format!("https://pypi.org/pypi/{}/json", pname))
        .await?
        .error_for_status()?;
    Ok(serde_json::from_str(&response.text().await?)?)
}
