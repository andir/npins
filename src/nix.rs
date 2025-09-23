use crate::{check_git_url, check_url};
use anyhow::{Context, Result};
use data_encoding::BASE64;
use std::path::Path;

#[allow(unused)]
pub struct PrefetchInfo {
    store_path: String,
    hash: String,
}

pub fn hash_to_sri(s: &str, algo: &str) -> Result<String> {
    let hash = nix_compat::nixhash::from_str(s, Some(algo))?;

    Ok(format!(
        "{}-{}",
        hash.algo(),
        BASE64.encode(hash.digest_as_bytes())
    ))
}

pub async fn nix_prefetch_tarball(url: impl AsRef<str>) -> Result<String> {
    let url = url.as_ref();
    let result = async {
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
        hash_to_sri(&stdout.trim(), "sha256")
    };
    check_url(result.await, url).await
}

pub async fn nix_prefetch_git(
    url: impl AsRef<str>,
    git_ref: impl AsRef<str>,
    submodules: bool,
) -> Result<String> {
    let url = url.as_ref();

    let result = async {
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
            .env("GIT_SSH_COMMAND", "ssh -o StrictHostKeyChecking=yes")
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

        log::debug!(
            "nix-prefetch-git output: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        let info: NixPrefetchGitResponse = serde_json::from_slice(&output.stdout)
            .context("Failed to deserialize nix-pfetch-git JSON response.")?;
        hash_to_sri(&info.sha256, "sha256")
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
    image_digest: Option<&str>,
) -> Result<NixPrefetchDockerResponse> {
    let image_name = image_name.as_ref();
    let image_tag = image_tag.as_ref();

    log::debug!(
        "Executing: `nix-prefetch-docker {} {}{}`",
        image_name,
        image_tag,
        match image_digest {
            Some(x) => format!(" --image-digest {}", x),
            None => "".into(),
        }
    );
    let mut output = tokio::process::Command::new("nix-prefetch-docker");
    let output = output
        .arg(image_name)
        .arg(image_tag)
        .arg("--json")
        .arg("--quiet");
    let output = match image_digest {
        Some(x) => output.arg("--image-digest").arg(x),
        None => output,
    };
    let output = output.output().await.with_context(|| {
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
    Ok(serde_json::from_slice(&output.stdout)
        .context("Failed to deserialize nix-pfetch-git JSON response.")?)
}
pub async fn nix_eval_pin(lockfile_path: &Path, pin: &str) -> Result<std::path::PathBuf> {
    const DEFAULT_NIX: &'static str = include_str!("default.nix");

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

    log::debug!(
        "Executing: `nix-instantiate --eval --json --expr '{{pin}}: (import default.nix).${{pin}}.outPath' --argstr pin '{pin}' --argstr path '{{«snip»}}'`",
    );
    let output = tokio::process::Command::new("nix-instantiate")
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
        .arg(lockfile_path)
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
