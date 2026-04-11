//! Pin a plain file URL source
//!
//! Prefetches a single file via `nix-prefetch-url` and stores the resulting
//! hash. The Nix side of the lockfile (see `default.nix`) consumes this with
//! `fetchurl` to materialize the file.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{GenericHash, Updatable, diff, nix};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct FilePin {
    pub url: Url,
}

impl diff::Diff for FilePin {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("url".into(), self.url.to_string())]
    }
}

/// File pins have no meaningful version; the URL is assumed to be static.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct FileVersion {}

impl diff::Diff for FileVersion {
    fn properties(&self) -> Vec<(String, String)> {
        vec![]
    }
}

#[async_trait::async_trait]
impl Updatable for FilePin {
    type Version = FileVersion;
    type Hashes = GenericHash;

    async fn update(&self, _old: Option<&FileVersion>) -> Result<FileVersion> {
        Ok(FileVersion {})
    }

    async fn fetch(&self, _version: &FileVersion) -> Result<Self::Hashes> {
        let hash = nix::nix_prefetch_url(&self.url).await?;
        Ok(Self::Hashes { hash })
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Frozen, GenericHash, NixPins, Pin};
    use nix_compat::nixhash::NixHash;

    #[test]
    fn test_file_pin_roundtrip() {
        let pins = NixPins {
            pins: [(
                "test-file".into(),
                Pin::File {
                    input: FilePin {
                        url: "https://example.com/some-file.iso".parse().unwrap(),
                    },
                    version: Some(FileVersion {}),
                    hashes: Some(GenericHash {
                        hash: NixHash::from_sri(
                            "sha256-K9yBph93OLTNw02Q6e9CYFGrUhvEXnh45vrZqIRWfvQ=",
                        )
                        .unwrap(),
                    }),
                    frozen: Frozen::default(),
                },
            )]
            .into_iter()
            .collect(),
        };

        let serialized = pins.to_value_versioned();
        let deserialized = NixPins::from_json_versioned(serialized).unwrap();
        assert_eq!(pins, deserialized);
    }

    #[test]
    fn test_file_pin_serialization_shape() {
        let pin = Pin::File {
            input: FilePin {
                url: "https://example.com/file.bin".parse().unwrap(),
            },
            version: Some(FileVersion {}),
            hashes: Some(GenericHash {
                hash: NixHash::from_sri("sha256-K9yBph93OLTNw02Q6e9CYFGrUhvEXnh45vrZqIRWfvQ=")
                    .unwrap(),
            }),
            frozen: Frozen::default(),
        };

        let value = serde_json::to_value(&pin).unwrap();
        assert_eq!(value["type"], "File");
        assert_eq!(value["url"], "https://example.com/file.bin");
        assert!(value["hash"].as_str().unwrap().starts_with("sha256-"));
    }

    #[tokio::test]
    async fn test_file_update_and_fetch() {
        // A small, immutable file served from the Nix cache. Its contents
        // (store dir, priority, ...) are part of the cache's public contract
        // and are not expected to change.
        let pin = FilePin {
            url: "https://cache.nixos.org/nix-cache-info".parse().unwrap(),
        };

        let version = pin.update(None).await.unwrap();
        assert_eq!(version, FileVersion {});

        let hashes = pin.fetch(&version).await.unwrap();
        assert_eq!(
            hashes.hash,
            NixHash::from_sri("sha256-LJ3jc651pScWN2NQNERaXNOmrjWsbDBtQMDgZ2R4WJc=").unwrap(),
        );
    }
}
