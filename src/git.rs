use crate::*;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PinInput {
    pub repository_url: Url,
    pub branch: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PinOutput {
    pub revision: String,
    pub hash: String,
}

impl diff::Diff for PinOutput {
    fn diff(&self, other: &Self) -> Vec<diff::Difference> {
        diff::d(&[
            diff::Difference::new("revision", &self.revision, &other.revision),
            diff::Difference::new("hash", &self.hash, &other.hash),
        ])
    }
}

#[async_trait::async_trait]
impl Updatable for PinInput {
    type Output = PinOutput;

    async fn update(&self) -> Result<PinOutput> {
        let info = fetch_branch_head(&self.repository_url, &self.branch).await?;
        let hash = nix::nix_prefetch_git(&self.repository_url, &info.revision).await?;
        Ok(PinOutput {
            revision: info.revision,
            hash,
        })
    }
}

#[derive(Debug)]
pub struct RevisionInfo {
    pub revision: String,
}

pub async fn fetch_branch_head(url: &url::Url, branch: impl AsRef<str>) -> Result<RevisionInfo> {
    let branch = branch.as_ref();
    let git_ref = format!("refs/heads/{}", branch);

    let process = Command::new("git")
        .arg("ls-remote")
        .arg(url.as_str())
        .arg(git_ref)
        .output()
        .await
        .with_context(|| {
            format!(
                "Failed to get revision from remote for {} @ {}",
                url, branch
            )
        })?;
    let stdout = String::from_utf8_lossy(&process.stdout);
    anyhow::ensure!(
        !stdout.is_empty(),
        anyhow::anyhow!(
            "git ls-remote output is empty. Are you sure the requested branch ('{}') exists?",
            branch
        ),
    );
    let revision = stdout
        .split_once('\t')
        .map(|(revision, _)| revision)
        .ok_or_else(|| {
            anyhow::anyhow!("git ls-remote output doesn't contain \\t. Can't match revision.")
        })?;
    log::debug!("revision: {:?}", revision);

    Ok(RevisionInfo {
        revision: revision.to_string(),
    })
}
