use crate::{diff, nix};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitPin {
    pub repository_url: Url,
    pub branch: String,
    pub revision: Option<String>,
    pub hash: Option<String>,
}

impl diff::Diff for GitPin {
    fn diff(&self, other: &Self) -> Vec<diff::Difference> {
        diff::d(&[
            diff::Difference::new(
                "repository_url",
                &self.repository_url,
                &other.repository_url,
            ),
            diff::Difference::new("branch", &self.branch, &other.branch),
            diff::Difference::new("revision", &self.revision, &other.revision),
            diff::Difference::new("hash", &self.hash, &other.hash),
        ])
    }
}

impl GitPin {
    pub async fn update(&self) -> Result<Self> {
        let info = fetch_branch_head(&self.repository_url, &self.branch).await?;
        let hash = nix::nix_prefetch_git(&self.repository_url, &info.revision).await?;
        Ok(GitPin {
            revision: Some(info.revision),
            hash: Some(hash),
            ..self.clone()
        })
    }
}

#[derive(Debug)]
pub struct GitRevisionInfo {
    pub revision: String,
}

pub async fn fetch_branch_head(url: &url::Url, branch: impl AsRef<str>) -> Result<GitRevisionInfo> {
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
    let (revision, _) = match stdout.split_once('\t') {
        None => {
            return Err(anyhow::anyhow!(
                "git ls-remote output doesn't contain \\t. Can't match revision."
            ))
        }
        Some(v) => v,
    };
    log::warn!("revision: {:?}", revision);

    Ok(GitRevisionInfo {
        revision: revision.to_string(),
    })
}
