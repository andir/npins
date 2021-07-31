use anyhow::{Context, Result};
use tokio::process::Command;

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
