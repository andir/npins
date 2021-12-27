//! Pin a PyPi package

use crate::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Pin {
    pub name: String,
}

#[async_trait::async_trait]
impl Updatable for Pin {
    type Version = GenericVersion;
    type Hashes = GenericUrlHashes;

    async fn update(&self, old: Option<&GenericVersion>) -> Result<GenericVersion> {
        /* Fetch the JSON metadata for a Pypi package.
         * Url template: `https://pypi.org/pypi/$pname/json`
         * JSON schema (as in the returned value): https://warehouse.pypa.io/api-reference/json.html
         */
        let metadata: PyPiMetadata =
            get_and_deserialize(format!("https://pypi.org/pypi/{}/json", &self.name))
                .await
                .context("Could not fetch Pypi metadata")?;
        let version = metadata.info.version;

        if let Some(old) = old {
            let old_version =
                lenient_semver_parser::parse::<lenient_version::Version>(&old.version);
            let version = lenient_semver_parser::parse::<lenient_version::Version>(&version);
            match (old_version, version) {
                (Ok(old_version), Ok(version)) => {
                    anyhow::ensure!(
                        version >= old_version,
                        "Failed to ensure version monotonicity, latest found version is {} but current is {}",
                        version,
                        old_version,
                    );
                },
                _ => {
                    log::warn!("This repository does not appear to be following SemVer, so no guarantees on monotonicity can be made.");
                },
            }
        }

        Ok(GenericVersion { version })
    }

    async fn fetch(&self, version: &GenericVersion) -> Result<GenericUrlHashes> {
        /* Fetch the JSON metadata for a Pypi package.
         * Url template: `https://pypi.org/pypi/$pname/json`
         * JSON schema (as in the returned value): https://warehouse.pypa.io/api-reference/json.html
         */
        let mut metadata: PyPiMetadata =
            get_and_deserialize(format!("https://pypi.org/pypi/{}/json", &self.name))
                .await
                .context("Could not fetch Pypi metadata")?;

        let mut latest_source: PyPiUrlMetadata = metadata.releases
            .remove(&version.version)
            .ok_or_else(|| anyhow::format_err!("Could not find requested version {}", version.version))?
            .into_iter()
            /* Of all files for the latest release, we only care about source tarballs */
            .find(|file_meta| file_meta.python_version == "source")
            .ok_or_else(|| anyhow::format_err!(
                "JSON metadata is invalid: must contain exactly one entry with \"python_version\": \"source\"",
            ))?;

        let hash = latest_source.digests.remove("sha256").ok_or_else(|| {
            anyhow::format_err!(
                "JSON metadata is invalid: must contain a `sha256` entry within `digests`",
            )
        })?;

        Ok(GenericUrlHashes {
            hash,
            url: latest_source.url.parse()?,
        })
    }
}

/// The actual JSON file is rather large, we only deserialize what we are interested in,
/// and only up to the granularity we are interested in.
/// JSON API specification: <https://warehouse.pypa.io/api-reference/json.html>
#[allow(unused)]
#[derive(Debug, Deserialize)]
struct PyPiMetadata {
    pub info: PyPiInfoMetadata,
    /// This contains releases
    pub releases: HashMap<String, Vec<PyPiUrlMetadata>>,
    /// This contains all data for the latest release
    pub urls: Vec<PyPiUrlMetadata>,
}

// Again, this is not complete
#[allow(unused)]
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
        let pin = Pin {
            name: "gaiatest".into(),
        };
        let version = pin.update(None).await?;
        assert_eq!(
            version,
            GenericVersion {
                version: "0.34".into(),
            }
        );
        assert_eq!(
            pin.fetch(&version).await?,
            GenericUrlHashes {
                hash: "3953b158b7b690642d68cd6beb1d59f6e10526f2ee10a6fb4636a913cc95e718".into(),
                url: "https://files.pythonhosted.org/packages/d1/d5/0c270c22d61ff6b883d0f24956f13e904b131b5ac2829e0af1cda99d70b1/gaiatest-0.34.tar.gz".parse().unwrap(),
            }
        );
        Ok(())
    }
}
