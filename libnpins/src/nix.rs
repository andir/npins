use anyhow::{Context, Result};
use nix_compat::nixhash::NixHash;
use std::path::Path;

use crate::{DEFAULT_NIX, check_git_url, check_url, format_command};

#[allow(unused)]
pub struct PrefetchInfo {
    store_path: String,
    hash: String,
}

pub async fn nix_prefetch_tarball(url: impl AsRef<str>) -> Result<NixHash> {
    nix_prefetch_url(url, true).await
}

pub async fn nix_prefetch_url(url: impl AsRef<str>, unpack: bool) -> Result<NixHash> {
    let url = url.as_ref();
    let result = async {
        let mut command = tokio::process::Command::new("nix-prefetch-url");
        if unpack {
            command.arg("--unpack"); // force calculation of the unpacked NAR hash
        }
        command
            .arg("--name")
            .arg("source") // use the same symbolic store path name as `builtins.fetchTarball` to avoid downloading the source twice
            .arg("--type")
            .arg("sha256")
            .arg(url);

        log::debug!("Executing: {}", format_command(&command)?);

        let output = command
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

        // try to parse the returned hash, trimming the newline at the end
        let hash_digest = output.stdout.trim_ascii_end();
        nix_compat::nixbase32::decode_fixed::<32>(hash_digest)
            .with_context(|| {
                format!(
                    "failed to convert {} to sha256 sri hash",
                    String::from_utf8_lossy(&output.stdout)
                )
            })
            .map(NixHash::Sha256)
    };
    check_url(result.await, url).await
}

pub async fn nix_prefetch_git(
    url: impl AsRef<str>,
    git_ref: impl AsRef<str>,
    submodules: bool,
) -> Result<NixHash> {
    let url = url.as_ref();

    let result = async {
        let mut command = tokio::process::Command::new("nix-prefetch-git");
        if submodules {
            command.arg("--fetch-submodules");
        }
        command
            // Disable any interactive login attempts, failing gracefully instead
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=yes")
            .args(["--hash", "sha256"])
            .arg(url)
            .arg(git_ref.as_ref());

        log::debug!("Executing: {}", format_command(&command)?);

        let output = command.output().await.with_context(|| {
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
            hash: NixHash,
            #[serde(rename = "fetchSubmodules")]
            fetch_submodules: bool,
            #[serde(rename = "deepClone")]
            deep_clone: bool,
            #[serde(rename = "leaveDotGit")]
            leave_dot_git: bool,
        }

        log::debug!(
            "nix-prefetch-git output: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let info: NixPrefetchGitResponse = serde_json::from_slice(&output.stdout)
            .context("Failed to deserialize nix-prefetch-git JSON response.")?;

        Ok(info.hash)
    };
    check_git_url(result.await, url).await
}

#[allow(unused)]
#[derive(Debug, serde::Deserialize)]
pub struct NixPrefetchDockerResponse {
    pub hash: String,
    #[serde(rename = "imageName")]
    pub image_name: String,
    #[serde(rename = "imageDigest")]
    pub image_digest: String,
    #[serde(rename = "finalImageName")]
    pub final_image_name: String,
    #[serde(rename = "finalImageTag")]
    pub final_image_tag: String,
}

pub async fn nix_prefetch_docker(
    image_name: impl AsRef<str>,
    image_tag: impl AsRef<str>,
    arch: &Option<String>,
    image_digest: Option<&str>,
) -> Result<NixPrefetchDockerResponse> {
    let image_name = image_name.as_ref();
    let image_tag = image_tag.as_ref();

    let mut command = tokio::process::Command::new("nix-prefetch-docker");
    command
        .arg(image_name)
        .arg(image_tag)
        .arg("--json")
        .arg("--quiet");

    if let Some(value) = image_digest {
        command.arg("--image-digest").arg(value);
    }

    if let Some(value) = arch {
        command.arg("--arch").arg(value);
    }

    log::debug!("Executing: {}", format_command(&command)?);

    let output = command.output().await.with_context(|| {
        format!(
            "Failed to spawn nix-prefetch-docker for {}:{}",
            image_name, image_tag
        )
    })?;

    // FIXME: handle errors and pipe stderr through
    if !output.status.success() {
        return Err(anyhow::anyhow!(format!(
            "failed to prefetch docker: {}\n{}",
            image_name,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    log::debug!(
        "nix-prefetch-git output: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout)
        .context("Failed to deserialize nix-prefetch-git JSON response.")
}

/// Query the digest of a container image from its registry, without downloading the image.
///
/// This was extracted from `nix-prefetch-docker`, which calls skopeo internally as well
/// https://github.com/NixOS/nixpkgs/blob/master/pkgs/build-support/docker/nix-prefetch-docker#L34-L41
pub async fn container_image_digest(
    image_name: impl AsRef<str>,
    image_tag: impl AsRef<str>,
    arch: Option<&str>,
) -> Result<String> {
    let image_name = image_name.as_ref();
    let image_tag = image_tag.as_ref();

    let mut command = tokio::process::Command::new("skopeo");
    command.arg("--insecure-policy");
    if let Some(arch) = arch {
        command.arg("--override-arch").arg(arch);
    }
    command
        .arg("inspect")
        .arg("--format")
        .arg("{{.Digest}}")
        .arg(format!("docker://{image_name}:{image_tag}"));

    log::debug!("Executing: {}", format_command(&command)?);

    let output = command
        .output()
        .await
        .with_context(|| format!("Failed to spawn skopeo inspect for {image_name}:{image_tag}"))?;

    if !output.status.success() {
        return Err(anyhow::anyhow!(format!(
            "failed to inspect container image {}:{}\n{}",
            image_name,
            image_tag,
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let digest = std::str::from_utf8(&output.stdout)
        .context("skopeo sent invalid utf8")?
        .trim()
        .to_owned();
    anyhow::ensure!(
        !digest.is_empty(),
        "skopeo returned an empty digest for {}:{}",
        image_name,
        image_tag
    );
    Ok(digest)
}

pub async fn nix_eval_pin(lockfile_path: &Path, pin: &str) -> Result<std::path::PathBuf> {
    let lockfile_path = lockfile_path.canonicalize()?;
    let lockfile_path = lockfile_path
        .to_str()
        .context("Lockfile path must be UTF-8")?;

    /* This is the Nix code we evaluate.
     * It is effectively `'{pin, path}: ((import default.nix) { input = builtins.toPath path; }) .${pin}.outPath'`,
     * except that the default.nix is inlined instead of imported (we have the code baked into the binary).
     *
     * The pin's name may contain special characters etc., so instead of splicing it in here with `format!` we
     * do a little dance with a function declaration that we'll then call with `--argstr`. That saves us from
     * one round-trip of a string value into Nix syntax and back.
     *
     * Same with the path, but this also means that we are passing the path in as string, so need to convert it
     * back to a path again.
     */
    let nix_eval_code =
        format!("{{pin, path}}: (({DEFAULT_NIX}) {{ input = /. + path; }}).${{pin}}.outPath");

    let mut command = tokio::process::Command::new("nix-instantiate");
    command
        .arg("--show-trace")
        .arg("--eval")
        .arg("--json")
        .arg("--expr")
        .arg(nix_eval_code)
        .arg("--argstr")
        .arg("pin")
        .arg(pin)
        .arg("--argstr")
        .arg("path")
        .arg(lockfile_path);

    log::debug!("Executing: {}", format_command(&command)?);

    let output = command
        .stdout(std::process::Stdio::piped())
        .spawn()
        .context("Failed to spawn `nix-instantiate`")?
        .wait_with_output()
        .await
        .context("Failed to spawn `nix-instantiate`")?;

    if !output.status.success() {
        anyhow::bail!(
            "Failed to eval pin: '{}'\n{}",
            pin,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    serde_json::from_slice::<std::path::PathBuf>(&output.stdout)
        .context("Failed to deserialize nix-instantiate JSON response.")
}
