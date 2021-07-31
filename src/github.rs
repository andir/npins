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

pub struct ReleaseInfo {
    pub tarball_url: String,
    pub release_name: String,
}

pub async fn get_latest_release(
    owner: impl AsRef<str>,
    repo: impl AsRef<str>,
) -> Result<ReleaseInfo> {
    let gh = get_github_client()?;

    let release = gh
        .repo(owner.as_ref(), repo.as_ref())
        .releases()
        .latest()
        .await?;

    Ok(ReleaseInfo {
        tarball_url: release.tarball_url,
        release_name: release.name,
    })
}
