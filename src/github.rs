use crate::{diff, nix};
use anyhow::{Context, Result};
use hubcaps::{Credentials, Github};
use serde::{Deserialize, Serialize};

/// GitHubPin tracks a given branch on GitHub and always uses the latest commit
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubPin {
    pub repository: String,
    pub owner: String,
    pub branch: String,
    pub revision: Option<String>,
    pub hash: Option<String>,
}

impl diff::Diff for GitHubPin {
    fn diff(&self, other: &Self) -> Vec<diff::Difference> {
        diff::d(&[
            diff::Difference::new("repository", &self.repository, &other.repository),
            diff::Difference::new("owner", &self.owner, &other.owner),
            diff::Difference::new("branch", &self.branch, &other.branch),
            diff::Difference::new("revision", &self.revision, &other.revision),
            diff::Difference::new("hash", &self.hash, &other.hash),
        ])
    }
}

impl GitHubPin {
    pub async fn update(&self) -> Result<Self> {
        let latest = get_latest_commit(&self.owner, &self.repository, &self.branch)
            .await
            .context("Couldn't fetch the latest commit")?;

        let tarball_url = format!(
            "https://github.com/{owner}/{repo}/archive/{revision}.tar.gz",
            owner = self.owner,
            repo = self.repository,
            revision = latest.revision,
        );

        let hash = nix::nix_prefetch_tarball(tarball_url).await?;

        Ok(Self {
            revision: Some(latest.revision),
            hash: Some(hash),
            ..self.clone()
        })
    }
}

/// GitHubReleasePin tries to follow the latest release of the given project
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubReleasePin {
    pub repository: String,
    pub owner: String,
    pub tarball_url: Option<String>,
    pub release_name: Option<String>,
    pub hash: Option<String>,
}

impl diff::Diff for GitHubReleasePin {
    fn diff(&self, other: &Self) -> Vec<diff::Difference> {
        diff::d(&[
            diff::Difference::new("repository", &self.repository, &other.repository),
            diff::Difference::new("owner", &self.owner, &other.owner),
            diff::Difference::new("tarball_url", &self.tarball_url, &other.tarball_url),
            diff::Difference::new("release_name", &self.release_name, &other.release_name),
            diff::Difference::new("hash", &self.hash, &other.hash),
        ])
    }
}

impl GitHubReleasePin {
    pub async fn update(&self) -> Result<Self> {
        let latest = get_latest_release(&self.owner, &self.repository)
            .await
            .context("Couldn't fetch the latest release")?;
        let hash = nix::nix_prefetch_tarball(&latest.tarball_url).await?;
        Ok(Self {
            tarball_url: Some(latest.tarball_url),
            release_name: Some(latest.release_name),
            hash: Some(hash),
            ..self.clone()
        })
    }
}

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
