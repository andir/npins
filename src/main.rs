use std::io::Write;

use anyhow::{Context, Result};
use diff::Diff;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use structopt::StructOpt;
use url::Url;

mod diff;
mod git;
mod github;
mod nix;

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
        let latest = github::Client::new()?
            .get_latest_commit(&self.owner, &self.repository, &self.branch)
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
        let latest = github::Client::new()?
            .get_latest_release(&self.owner, &self.repository)
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitPin {
    pub repository_url: Url,
    pub branch: String,
    pub revision: Option<String>,
    pub hash: Option<String>,
}

impl diff::Diff for GitPin {
    fn diff(&self, other: &Self) -> Vec<diff::Difference> {
        diff::d(&[
            diff::Difference::new(
                "repository_url",
                &self.repository_url,
                &other.repository_url,
            ),
            diff::Difference::new("branch", &self.branch, &other.branch),
            diff::Difference::new("revision", &self.revision, &other.revision),
            diff::Difference::new("hash", &self.hash, &other.hash),
        ])
    }
}

impl GitPin {
    pub async fn update(&self) -> Result<Self> {
        let info = git::fetch_branch_head(&self.repository_url, &self.branch).await?;
        let hash = nix::nix_prefetch_git(&self.repository_url, &info.revision).await?;
        Ok(GitPin {
            revision: Some(info.revision),
            hash: Some(hash),
            ..self.clone()
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Pin {
    GitHub(GitHubPin),
    GitHubRelease(GitHubReleasePin),
    Git(GitPin),
}

impl diff::Diff for Pin {
    fn diff(&self, other: &Self) -> Vec<diff::Difference> {
        use Pin::*;
        match (self, other) {
            (GitHub(a), GitHub(b)) => a.diff(b),
            (GitHubRelease(a), GitHubRelease(b)) => a.diff(b),
            (Git(a), Git(b)) => a.diff(b),

            // impossible/invalid cases
            (GitHub(_), _) => vec![],
            (GitHubRelease(_), _) => vec![],
            (Git(_), _) => vec![],
        }
    }
}

impl Pin {
    async fn update(&self) -> Result<Pin> {
        match self {
            Self::GitHub(gh) => gh.update().await.map(Self::GitHub),
            Self::GitHubRelease(ghr) => ghr.update().await.map(Self::GitHubRelease),
            Self::Git(g) => g.update().await.map(Self::Git),
        }
    }
}

impl std::fmt::Display for Pin {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::GitHub(gh) => write!(fmt, "{:?}", gh),
            Self::GitHubRelease(ghr) => write!(fmt, "{:?}", ghr),
            Self::Git(g) => write!(fmt, "{:?}", g),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NixPins {
    pins: BTreeMap<String, Pin>,
}

impl NixPins {
    pub fn new_with_nixpkgs() -> Self {
        let mut pins = BTreeMap::new();
        pins.insert(
            "nixpkgs".to_owned(),
            Pin::GitHub(GitHubPin {
                repository: "nixpkgs".to_owned(),
                owner: "nixos".to_owned(),
                branch: "nixpkgs-unstable".to_owned(),
                revision: None,
                hash: None,
            }),
        );
        Self { pins }
    }
}

impl Default for NixPins {
    fn default() -> Self {
        Self {
            pins: BTreeMap::new(),
        }
    }
}

#[derive(Debug, StructOpt)]
pub struct GitHubAddOpts {
    pub owner: String,
    pub repository: String,

    #[structopt(default_value = "master")]
    pub branch: String,
}

impl GitHubAddOpts {
    pub fn add(&self) -> Result<(String, Pin)> {
        Ok((
            self.repository.clone(),
            Pin::GitHub(GitHubPin {
                repository: self.repository.clone(),
                owner: self.owner.clone(),
                branch: self.branch.clone(),
                revision: None,
                hash: None,
            }),
        ))
    }
}

#[derive(Debug, StructOpt)]
pub struct GitHubReleaseAddOpts {
    pub owner: String,
    pub repository: String,
}

impl GitHubReleaseAddOpts {
    pub fn add(&self) -> Result<(String, Pin)> {
        Ok((
            self.repository.clone(),
            Pin::GitHubRelease(GitHubReleasePin {
                owner: self.owner.clone(),
                repository: self.repository.clone(),
                hash: None,
                release_name: None,
                tarball_url: None,
            }),
        ))
    }
}

#[derive(Debug, StructOpt)]
pub struct GitAddOpts {
    /// The git remote URL. For example https://github.com/andir/ate.git
    url: String,

    /// Name of the branch to track.
    #[structopt(default_value = "master")]
    branch: String,
}

impl GitAddOpts {
    pub fn add(&self) -> Result<(String, Pin)> {
        let url = Url::parse(&self.url)?;
        let name = match url.path_segments().map(|x| x.rev().next()).flatten() {
            None => return Err(anyhow::anyhow!("Path segment in URL missing.")),
            Some(seg) => seg.to_owned(),
        };
        let name = name.strip_suffix(".git").unwrap_or(&name);

        Ok((
            name.to_owned(),
            Pin::Git(GitPin {
                repository_url: url,
                branch: self.branch.clone(),
                revision: None,
                hash: None,
            }),
        ))
    }
}

#[derive(Debug, StructOpt)]
pub enum AddCommands {
    #[structopt(name = "github")]
    GitHub(GitHubAddOpts),
    #[structopt(name = "github-release")]
    GitHubRelease(GitHubReleaseAddOpts),
    #[structopt(name = "git")]
    Git(GitAddOpts),
}

#[derive(Debug, StructOpt)]
pub struct AddOpts {
    #[structopt(long, short)]
    pub name: Option<String>,

    #[structopt(subcommand)]
    command: AddCommands,
}

impl AddOpts {
    fn run(&self) -> Result<(String, Pin)> {
        let (name, pin) = match &self.command {
            AddCommands::Git(g) => g.add()?,
            AddCommands::GitHub(gh) => gh.add()?,
            AddCommands::GitHubRelease(ghr) => ghr.add()?,
        };

        let name = if let Some(ref n) = self.name {
            n.clone()
        } else {
            name
        };

        Ok((name, pin))
    }
}

#[derive(Debug, StructOpt)]
pub struct RemoveOpts {
    pub name: String,
}

#[derive(Debug, StructOpt)]
pub struct UpdateOpts {
    pub name: Option<String>,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Intializes the npins directory. Running this multiple times will restore/upgrade the
    /// `default.nix` and never touch your pins.json.
    Init,

    /// Adds a new pin entry.
    Add(AddOpts),

    /// Lists the current pin entries.
    Show,

    /// Updates all or the given pin to the latest version.
    Update(UpdateOpts),

    /// Removes one pin entry.
    Remove(RemoveOpts),
}

#[derive(Debug, StructOpt)]
pub struct Opts {
    /// Base folder for npins.json and the boilerplate default.nix
    #[structopt(default_value = "npins", env = "NPINS_FOLDER")]
    folder: std::path::PathBuf,

    #[structopt(subcommand)]
    command: Command,
}

impl Opts {
    fn read_pins(&self) -> Result<NixPins> {
        let path = self.folder.join("pins.json");
        let fh = std::fs::File::open(&path).with_context(move || {
            format!(
                "Failed to open {}. You must initialize npins before you can show current pins.",
                path.display()
            )
        })?;
        let pins: NixPins = serde_json::from_reader(fh)?;
        Ok(pins)
    }

    fn write_pins(&self, pins: &NixPins) -> Result<()> {
        if !self.folder.exists() {
            std::fs::create_dir(&self.folder)?;
        }
        let path = self.folder.join("pins.json");
        let fh = std::fs::File::create(&path)
            .with_context(move || format!("Failed to open {} for writing.", path.display()))?;
        serde_json::to_writer_pretty(fh, pins)?;
        Ok(())
    }

    fn init(&self) -> Result<()> {
        let default_nix = include_bytes!("../npins/default.nix");
        if !self.folder.exists() {
            std::fs::create_dir(&self.folder).context("Failed to create npins folder")?;
        }
        let p = self.folder.join("default.nix");
        let mut fh = std::fs::File::create(&p).context("Failed to create npins default.nix")?;
        fh.write_all(default_nix)?;

        // Only create the pins if the file isn't there yet
        if self.folder.join("pins.json").exists() {
            return Ok(());
        }

        let initial_pins = NixPins::new_with_nixpkgs();
        self.write_pins(&initial_pins)?;
        Ok(())
    }

    fn show(&self) -> Result<()> {
        let pins = self.read_pins()?;
        for (name, pin) in pins.pins.iter() {
            println!("{}:", name);
            println!("\t{}", pin);
        }

        Ok(())
    }

    fn add(&self, opts: &AddOpts) -> Result<()> {
        let mut pins = self.read_pins()?;
        let (name, pin) = opts.run()?;
        pins.pins.insert(name, pin);
        self.write_pins(&pins)?;

        Ok(())
    }

    async fn update_one(&self, pin: &Pin) -> Result<Pin> {
        let p = pin.update().await?;
        let diff = pin.diff(&p);
        if diff.len() > 0 {
            println!("changes:");
            for d in diff {
                println!("{}", d);
            }
        }

        Ok(p)
    }

    async fn update(&self, opts: &UpdateOpts) -> Result<()> {
        let pins = self.read_pins()?;
        let mut new_pins = NixPins::default();

        if let Some(name) = &opts.name {
            new_pins = pins.clone();
            match pins.pins.get(name) {
                None => return Err(anyhow::anyhow!("No such pin entry found.")),
                Some(p) => {
                    let p = self.update_one(p).await?;
                    new_pins.pins.insert(name.clone(), p);
                }
            }
        } else {
            for (name, pin) in pins.pins.iter() {
                println!("Updating {}", name);
                let p = self.update_one(pin).await?;
                new_pins.pins.insert(name.clone(), p);
            }
        }

        self.write_pins(&new_pins)?;

        Ok(())
    }

    fn remove(&self, r: &RemoveOpts) -> Result<()> {
        let pins = self.read_pins()?;

        if !pins.pins.contains_key(&r.name) {
            return Err(anyhow::anyhow!("No such pin entry found."));
        }

        let mut new_pins = pins.clone();
        new_pins.pins.remove(&r.name);

        self.write_pins(&new_pins)?;

        Ok(())
    }

    async fn run(&self) -> Result<()> {
        match &self.command {
            Command::Init => self.init()?,
            Command::Show => self.show()?,
            Command::Add(a) => self.add(a)?,
            Command::Update(o) => self.update(o).await?,
            Command::Remove(r) => self.remove(r)?,
        };

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let opts = Opts::from_args();
    opts.run().await?;
    Ok(())
}
