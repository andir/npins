use anyhow::{Context, Result};

#[allow(unused)]
pub struct PrefetchInfo {
    store_path: String,
    hash: String,
}

pub async fn nix_prefetch_tarball(
    url: impl AsRef<str>,
    filename: Option<impl AsRef<str>>,
) -> Result<String> {
    let url = url.as_ref();
    let mut cmd = tokio::process::Command::new("nix-prefetch-url");
    cmd.arg("--unpack") // force calculation of the unpacked NAR hash
        .arg("--type")
        .arg("sha256");

    if let Some(filename) = filename {
        cmd.arg("--name").arg(filename.as_ref());
    }

    let output = cmd
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

pub async fn nix_prefetch_git(url: impl AsRef<str>, git_ref: impl AsRef<str>) -> Result<String> {
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
    struct NixPrefetchGitResponse {
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

    let info: NixPrefetchGitResponse = serde_json::from_slice(&output.stdout)
        .context("Failed to deserialize nix-pfetch-git JSON response.")?;

    Ok(info.sha256)
}
