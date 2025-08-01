use crate::{check_git_url, check_url};
use anyhow::{Context, Result};
use data_encoding::BASE64;
use serde::Deserialize;
use serde_repr::Deserialize_repr;
use std::{path::Path, process::Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};

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

pub async fn nix_prefetch_tarball(
    url: impl AsRef<str>,
    logging: Option<Box<dyn FnMut(FetchStatus) + Send>>,
) -> Result<String> {
    let url = url.as_ref();
    let result = async {
        log::debug!("Executing `nix store prefetch-file --unpack --name source --hash-type sha256 --log-format internal-json {url}`",);
        let mut child = tokio::process::Command::new("nix")
            .arg("store") // force calculation of the unpacked NAR hash
            .arg("prefetch-file")
            .arg("--unpack") // force calculation of the unpacked NAR hash
            .arg("--name")
            .arg("source") // use the same symbolic store path name as `builtins.fetchTarball` to avoid downloading the source twice
            .arg("--hash-type")
            .arg("sha256")
            .arg("--json")
            .arg("--extra-experimental-features")
            .arg("nix-command flakes")
            .arg("--log-format")
            .arg("internal-json")
            .arg(url)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn nix store prefetch-file for {}", url))?;

        let output = if let Some(mut callback) = logging {
            let mut stderr_output = Vec::<u8>::new();
            let mut stderr =
                BufReader::new(child.stderr.take().context("stderr was not captured")?);
            loop {
                let len = stderr.read_until(b'\n', &mut stderr_output).await?;
                if len == 0 {
                    break;
                }

                if let Some(json) = stderr_output[stderr_output.len() - len..]
                    .trim_ascii_end()
                    .strip_prefix(b"@nix ")
                {
                    let log = serde_json::from_slice::<LogMessage>(json);
                    if let Ok(Some(log)) = log.map(FetchStatus::from_internal_log) {
                        callback(log);
                    }
                }
            }

            std::process::Output {
                stderr: stderr_output,
                ..child.wait_with_output().await?
            }
        } else {
            child.wait_with_output().await?
        };

        // FIXME: handle errors and pipe stderr through
        if !output.status.success() {
            return Err(anyhow::anyhow!(format!(
                "failed to prefetch url: {}\n{}",
                url,
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct PrefetchedOutput {
            pub hash: String,
            #[allow(unused)]
            pub store_path: String,
        }

        let output: PrefetchedOutput = serde_json::from_slice(&output.stdout)?;
        log::debug!("Got hash: {}", output.hash);
        Ok(output.hash)
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

pub enum FetchStatus {
    Progress { downloaded: u64, total: u64 },
    Message(&'static str),
}

impl FetchStatus {
    fn from_internal_log(log: LogMessage) -> Option<Self> {
        match log {
            LogMessage::Result {
                fields,
                id: _,
                type_: ResultType::Progress,
            } => {
                let &[Field::Int(downloaded), Field::Int(total), ..] = fields.as_slice() else {
                    return None;
                };

                if total == 0 && downloaded == 0 {
                    return None;
                }

                Some(Self::Progress { downloaded, total })
            },
            LogMessage::Start {
                text,
                type_: ActivityType::Unknown,
                ..
            } if text.starts_with("unpacking") => Some(Self::Message("unpacking")),
            LogMessage::Start {
                text,
                type_: ActivityType::Unknown,
                ..
            } if text.starts_with("adding") => Some(Self::Message("adding")),
            _ => None,
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "action", rename_all = "lowercase")]
#[allow(unused)]
enum LogMessage {
    Start {
        #[serde(default)]
        fields: Vec<Field>,
        id: u64,
        level: u64,
        parent: u64,
        text: String,
        #[serde(rename = "type")]
        type_: ActivityType,
    },
    Stop {
        id: u64,
    },
    Result {
        fields: Vec<Field>,
        id: u64,
        #[serde(rename = "type")]
        type_: ResultType,
    },
    Msg {
        level: u64,
        msg: String,
    },
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[allow(unused)]
enum Field {
    Int(u64),
    Str(String),
}

#[derive(Deserialize_repr, Debug, PartialEq)]
#[repr(u8)]
enum ActivityType {
    Unknown = 0,
    CopyPath = 100,
    FileTransfer = 101,
    Realise = 102,
    CopyPaths = 103,
    Builds = 104,
    Build = 105,
    OptimiseStore = 106,
    VerifyPaths = 107,
    Substitute = 108,
    QueryPathInfo = 109,
    PostBuildHook = 110,
    BuildWaiting = 111,
    FetchTree = 112,
}

#[derive(Deserialize_repr, Debug)]
#[repr(u8)]
enum ResultType {
    FileLinked = 100,
    BuildLogLine = 101,
    UntrustedPath = 102,
    CorruptedPath = 103,
    SetPhase = 104,
    Progress = 105,
    SetExpected = 106,
    PostBuildLogLine = 107,
    FetchStatus = 108,
}
