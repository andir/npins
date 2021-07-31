use std::io::Write;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use structopt::StructOpt;
use url::Url;

mod github;
mod nix;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitHubPin {
    pub repository: String,
    pub owner: String,
    pub branch: String,
    pub revision: Option<String>,
    pub hash: Option<String>,
}

impl GitHubPin {
    pub async fn update(&self) -> Result<Self> {
        let latest = github::get_latest_commit(&self.owner, &self.repository, &self.branch).await?;

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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitPin {
    pub repoistory_url: Url,
    pub branch: String,
    pub revision: Option<String>,
    pub hash: Option<String>,
}

impl GitPin {
    pub async fn update(&self) -> Result<Self> {
        Ok(self.clone())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Pin {
    GitHub(GitHubPin),
    Git(GitPin),
    Url,
}

impl Pin {
    async fn update(&self) -> Result<Pin> {
        match self {
            Self::GitHub(gh) => gh.update().await.map(Self::GitHub),
            Self::Git(g) => g.update().await.map(Self::Git),
            Self::Url => Ok(Self::Url),
        }
    }
}

impl std::fmt::Display for Pin {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::GitHub(gh) => write!(fmt, "{:?}", gh),
            Self::Git(g) => write!(fmt, "{:?}", g),
            Url => write!(fmt, "Url..."),
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
pub enum AddOpts {
    #[structopt(name = "github")]
    GitHub(GitHubAddOpts),
}

impl AddOpts {
    fn run(&self) -> Result<(String, Pin)> {
        match self {
            Self::GitHub(gh) => gh.add(),
        }
    }
}

#[derive(Debug, StructOpt)]
pub struct RemoveOpts {
    pub name: String,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    Init,
    Add(AddOpts),
    Show,
    Update,
    Remove(RemoveOpts),
}

#[derive(Debug, StructOpt)]
pub struct Opts {
    #[structopt(default_value = "npins")]
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
        serde_json::to_writer(fh, pins)?;
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

    async fn update(&self) -> Result<()> {
        let pins = self.read_pins()?;
        let mut new_pins = NixPins::default();

        for (name, pin) in pins.pins.iter() {
            println!("Updating {}", name);
            let p = pin.update().await?;
            new_pins.pins.insert(name.clone(), p);
        }

        println!("old pins: {:?}", pins);
        println!("new pins: {:?}", new_pins);

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
            Command::Update => self.update().await?,
            Command::Remove(r) => self.remove(r)?,
        };

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::from_args();
    opts.run().await?;
    Ok(())
}
