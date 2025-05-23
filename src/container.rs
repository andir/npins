//! Pin an OCI container

use crate::nix::nix_prefetch_docker;
use crate::*;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub struct Pin {
    pub image_name: String,
    pub image_tag: String,
}

impl diff::Diff for Pin {
    fn properties(&self) -> Vec<(String, String)> {
        vec![
            ("image_name".into(), self.image_name.clone()),
            ("image_tag".into(), self.image_tag.clone()),
        ]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ContainerVersion {
    pub image_digest: String,
}

impl diff::Diff for ContainerVersion {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("image_digest".into(), self.image_digest.to_string())]
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct ContainerHash {
    pub hash: String,
}

impl diff::Diff for ContainerHash {
    fn properties(&self) -> Vec<(String, String)> {
        vec![("hash".into(), self.hash.to_string())]
    }
}

#[async_trait::async_trait]
impl Updatable for Pin {
    type Version = ContainerVersion;
    type Hashes = ContainerHash;

    async fn update(&self, _old: Option<&ContainerVersion>) -> Result<ContainerVersion> {
        Ok(ContainerVersion {
            image_digest: nix_prefetch_docker(&self.image_name, &self.image_tag, None)
                .await?
                .image_digest,
        })
    }

    async fn fetch(&self, version: &ContainerVersion) -> Result<ContainerHash> {
        Ok(ContainerHash {
            hash: nix_prefetch_docker(
                &self.image_name,
                &self.image_tag,
                Some(&version.image_digest),
            )
            .await?
            .hash,
        })
    }
}
