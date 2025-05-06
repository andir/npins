use crate::check_url;
use anyhow::{Context, Result};
use log::debug;

#[allow(unused)]
pub struct PrefetchInfo {
    store_path: String,
    hash: String,
}

pub async fn nix_prefetch_tarball(url: impl AsRef<str>) -> Result<String> {
    let url = url.as_ref();
    check_url(url).await?;

    log::debug!(
        "Executing `nix-prefetch-url --unpack --name source --type sha256 {}`",
        url
    );
    let output = tokio::process::Command::new("nix-prefetch-url")
        .arg("--unpack") // force calculation of the unpacked NAR hash
        .arg("--name")
        .arg("source") // use the same symbolic store path name as `builtins.fetchTarball` to avoid downloading the source twice
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
    log::debug!("Got hash: {}", stdout);
    Ok(String::from(stdout.trim()))
}

pub async fn nix_prefetch_git(
    url: impl AsRef<str>,
    git_ref: impl AsRef<str>,
    submodules: bool,
) -> Result<String> {
    let url = url.as_ref();
    check_url(url).await?;

    log::debug!(
        "Executing: `nix-prefetch-git {}{} {}`",
        if submodules {
            "--fetch-submodules "
        } else {
            ""
        },
        url,
        git_ref.as_ref()
    );
    let mut output = tokio::process::Command::new("nix-prefetch-git");
    if submodules {
        output.arg("--fetch-submodules");
    }
    let output = output
        // Disable any interactive login attempts, failing gracefully instead
        .env("GIT_TERMINAL_PROMPT", "0")
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

    debug!(
        "nix-prefetch-git output: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let info: NixPrefetchGitResponse = serde_json::from_slice(&output.stdout)
        .context("Failed to deserialize nix-pfetch-git JSON response.")?;

    Ok(info.sha256)
}
