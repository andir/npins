use anyhow::{Context, Result};

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
