use anyhow::{Context, Result};

#[allow(unused)]
pub struct PrefetchInfo {
    store_path: String,
    hash: String,
}

pub async fn nix_prefetch_tarball(url: impl AsRef<str>) -> Result<String> {
    let url = url.as_ref();
    let output = tokio::process::Command::new("nix-prefetch-url")
        .arg("--unpack") // force calculation of the unpacked NAR hash
        .arg("--type")
        .arg("sha256")
        .arg(url)
        .output()
        .await
        .with_context(|| format!("Failed to spawn nix-prefetch-url for {}", url))?;

    // FIXME: handle errors and pipe stderr through
    if !output.status.success() {
        return Err(anyhow::anyhow!(format!(
            "failed to prefetch url: {}\n{}",
            url,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(String::from(stdout.trim()))
}

#[derive(Debug)]
pub struct NixPrefetchGitResponse {
    pub sha256: String,
    pub path: String,
}

pub async fn nix_prefetch_git(
    url: impl AsRef<str>,
    git_ref: impl AsRef<str>,
) -> Result<NixPrefetchGitResponse> {
    let url = url.as_ref();
    let output = tokio::process::Command::new("nix-prefetch-git")
        .arg(url)
        .arg(git_ref.as_ref())
        .output()
        .await
        .with_context(|| {
            format!(
                "Failed to spawn nix-prefetch-git for {} @ {}",
                url,
                git_ref.as_ref()
            )
        })?;

    // FIXME: handle errors and pipe stderr through
    if !output.status.success() {
        return Err(anyhow::anyhow!(format!(
            "failed to prefetch url: {}\n{}",
            url,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    #[allow(unused)]
    #[derive(Debug, serde::Deserialize)]
    struct NixPrefetchGitCliResponse {
        url: String,
        rev: String,
        date: String,
        path: String,
        sha256: String,
        #[serde(rename = "fetchSubmodules")]
        fetch_submodules: bool,
        #[serde(rename = "deepClone")]
        deep_clone: bool,
        #[serde(rename = "leaveDotGit")]
        leave_dot_git: bool,
    }

    let info: NixPrefetchGitCliResponse = serde_json::from_slice(&output.stdout)
        .context("Failed to deserialize nix-pfetch-git JSON response.")?;

    Ok(NixPrefetchGitResponse {
        sha256: info.sha256,
        path: info.path,
    })
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_nix_prefetch_git() {
        let result = super::nix_prefetch_git(
            "https://github.com/left-pad/left-pad.git",
            "2fca6157fcca165438e0f9495cf0e5a4e6f71349",
        )
        .await
        .unwrap();
        assert_eq!(
            result.sha256,
            "06cb6fv6y9giiiljzjf8k9n7qzb7aaibaryhdwr7lb618lhjvwfi"
        );
        assert_eq!(
            result.path,
            "/nix/store/31bxz3mxqhsinhnyvgdpdc13b86j372w-left-pad-2fca615"
        );
    }

    #[tokio::test]
    async fn test_nix_prefetch_tarball() {
        let result = super::nix_prefetch_tarball(
            "https://github.com/left-pad/left-pad/archive/refs/tags/v1.3.0.tar.gz",
        )
        .await
        .unwrap();
        assert_eq!(
            result,
            "0mjvb0b51ivwi9sfkiqnjbj2y1rfblydnb0s4wdk46c7lsf1jisg"
        )
    }
}
