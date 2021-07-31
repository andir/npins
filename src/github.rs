use anyhow::Result;
use hubcaps::{Credentials, Github};

pub struct CommitInfo {
    pub revision: String,
    pub branch: String,
}

fn get_github_client() -> Result<Github> {
    let creds = match std::env::var("GITHUB_TOKEN") {
        Ok(v) => Some(Credentials::Token(v)),
        Err(_) => None,
    };
    let github = Github::new(
        concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
        creds,
    )?;

    Ok(github)
}

pub async fn get_latest_commit(
    owner: impl AsRef<str>,
    repo: impl AsRef<str>,
    branch: impl AsRef<str>,
) -> Result<CommitInfo> {
    let gh = get_github_client()?;

    let commit = gh
        .repo(owner.as_ref(), repo.as_ref())
        .commits()
        .get(branch.as_ref())
        .await?;

    Ok(CommitInfo {
        revision: commit.sha,
        branch: branch.as_ref().to_string(),
    })
}
