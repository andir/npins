use std::io::Write;

use anyhow::{Context, Result};
use diff::OptionExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use structopt::StructOpt;
use url::Url;

pub mod diff;
pub mod git;
pub mod github;
pub mod nix;
pub mod pypi;

#[async_trait::async_trait]
trait Updatable {
    type Output: diff::Diff + Serialize + Deserialize<'static> + std::fmt::Debug;

    async fn update(&self) -> Result<Self::Output>;
}

/// Create the `Pin` type
///
/// We need a type to unify over all possible way to pin a dependency. Normally, this would be done with a trait
/// and trait objects. However, designing such a trait to be object-safe turns out to be highly non-trivial.

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Pin {
    GitHub {
        #[serde(flatten)]
        input: github::PinInput,
        #[serde(flatten)]
        output: Option<github::PinOutput>,
    },

    GitHubRelease {
        #[serde(flatten)]
        input: github::ReleasePinInput,
        #[serde(flatten)]
        output: Option<github::ReleasePinOutput>,
    },

    Git {
        #[serde(flatten)]
        input: git::PinInput,
        #[serde(flatten)]
        output: Option<git::PinOutput>,
    },

    PyPi {
        #[serde(flatten)]
        input: pypi::PinInput,
        #[serde(flatten)]
        output: Option<pypi::PinOutput>,
    },
}

impl Pin {
    fn github(input: github::PinInput) -> Self {
        Self::GitHub {
            input,
            output: None,
        }
    }

    fn github_release(input: github::ReleasePinInput) -> Self {
        Self::GitHubRelease {
            input,
            output: None,
        }
    }

    fn git(input: git::PinInput) -> Self {
        Self::Git {
            input,
            output: None,
        }
    }

    fn pypi(input: pypi::PinInput) -> Self {
        Self::PyPi {
            input,
            output: None,
        }
    }

    async fn update(&mut self) -> Result<Vec<diff::Difference>> {
        /* Use very explicit syntax to force the correct types and get good compile errors */
        Ok(match self {
            Self::GitHub { input, output } => {
                let new_output: github::PinOutput =
                    <github::PinInput as Updatable>::update(input).await?;
                output.insert_diffed(new_output)
            },

            Self::GitHubRelease { input, output } => {
                let new_output: github::ReleasePinOutput =
                    <github::ReleasePinInput as Updatable>::update(input).await?;
                output.insert_diffed(new_output)
            },

            Self::Git { input, output } => {
                let new_output: git::PinOutput =
                    <git::PinInput as Updatable>::update(input).await?;
                output.insert_diffed(new_output)
            },

            Self::PyPi { input, output } => {
                let new_output: pypi::PinOutput =
                    <pypi::PinInput as Updatable>::update(input).await?;
                output.insert_diffed(new_output)
            },
        })
    }
}

impl std::fmt::Display for Pin {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::GitHub { input, output } => write!(
                fmt,
                "GitHub:{}/{}@{}",
                input.repository,
                input.owner,
                output.as_ref().map_or("", |o| &o.revision)
            ),

            Self::GitHubRelease { input, output } => write!(
                fmt,
                "GitHubRelease:{}/{}@{}",
                input.repository,
                input.owner,
                output.as_ref().map_or("", |o| &o.release_name)
            ),

            Self::Git { input, output } => write!(
                fmt,
                "Git:{}@{}",
                input.repository_url,
                output.as_ref().map_or("", |o| &o.revision)
            ),

            Self::PyPi { input, output } => write!(
                fmt,
                "PyPi: {}@{}",
                input.name,
                output.as_ref().map_or("", |o| &o.version)
            ),
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
            Pin::github(github::PinInput {
                repository: "nixpkgs".to_owned(),
                owner: "nixos".to_owned(),
                branch: "nixpkgs-unstable".to_owned(),
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

    #[structopt(short, long, default_value = "master")]
    pub branch: String,
}

impl GitHubAddOpts {
    pub fn add(&self) -> Result<(String, Pin)> {
        Ok((
            self.repository.clone(),
            Pin::github(github::PinInput {
                repository: self.repository.clone(),
                owner: self.owner.clone(),
                branch: self.branch.clone(),
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
        log::warn!("The releases API always gives you the *latest* release, which is probably not what you want!");
        log::warn!("This is a known issue, and will be fixed in the future. That fix might be backwards-incompatible in some way.");
        Ok((
            self.repository.clone(),
            Pin::github_release(github::ReleasePinInput {
                owner: self.owner.clone(),
                repository: self.repository.clone(),
            }),
        ))
    }
}

#[derive(Debug, StructOpt)]
pub struct GitAddOpts {
    /// The git remote URL. For example <https://github.com/andir/ate.git>
    url: String,

    /// Name of the branch to track.
    #[structopt(short, long, default_value = "master")]
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
            Pin::git(git::PinInput {
                repository_url: url,
                branch: self.branch.clone(),
            }),
        ))
    }
}

#[derive(Debug, StructOpt)]
pub struct PyPiAddOpts {
    /// Name of the package at PyPi.org
    pub name: String,
}

impl PyPiAddOpts {
    pub fn add(&self) -> Result<(String, Pin)> {
        Ok((
            self.name.clone(),
            Pin::pypi(pypi::PinInput {
                name: self.name.clone(),
            }),
        ))
    }
}

#[derive(Debug, StructOpt)]
pub enum AddCommands {
    /// Track a branch from a GitHub repository
    #[structopt(name = "github")]
    GitHub(GitHubAddOpts),
    /// Track the latest release from a GitHub repository
    #[structopt(name = "github-release")]
    GitHubRelease(GitHubReleaseAddOpts),
    /// Track a git repository
    #[structopt(name = "git")]
    Git(GitAddOpts),
    /// Track a package on PyPi
    #[structopt(name = "pypi")]
    PyPi(PyPiAddOpts),
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
            AddCommands::PyPi(p) => p.add()?,
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
pub struct InitOpts {
    /// Don't add an initial `nixpkgs` entry
    #[structopt(long)]
    pub bare: bool,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Intializes the npins directory. Running this multiple times will restore/upgrade the
    /// `default.nix` and never touch your pins.json.
    Init(InitOpts),

    /// Adds a new pin entry.
    Add(AddOpts),

    /// Query some release information and then print out the entry
    Fetch(AddOpts),

    /// Lists the current pin entries.
    Show,

    /// Updates all or the given pin to the latest version.
    Update(UpdateOpts),

    /// Removes one pin entry.
    Remove(RemoveOpts),
}

use structopt::clap::AppSettings;

#[derive(Debug, StructOpt)]
#[structopt(
    setting(AppSettings::ArgRequiredElseHelp),
    global_setting(AppSettings::VersionlessSubcommands),
    global_setting(AppSettings::ColoredHelp),
    global_setting(AppSettings::ColorAuto)
)]
pub struct Opts {
    /// Base folder for npins.json and the boilerplate default.nix
    #[structopt(
        global = true,
        short = "d",
        long = "directory",
        default_value = "npins",
        env = "NPINS_DIRECTORY"
    )]
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

    fn init(&self, o: &InitOpts) -> Result<()> {
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

        let initial_pins = if o.bare {
            NixPins::default()
        } else {
            NixPins::new_with_nixpkgs()
        };
        self.write_pins(&initial_pins)?;
        Ok(())
    }

    fn show(&self) -> Result<()> {
        let pins = self.read_pins()?;
        for (name, pin) in pins.pins.iter() {
            println!("\t{:<24}: {}", name, pin);
        }

        Ok(())
    }

    async fn add(&self, opts: &AddOpts) -> Result<()> {
        let mut pins = self.read_pins()?;
        let (name, mut pin) = opts.run()?;
        self.update_one(&mut pin)
            .await
            .context("Failed to fully initialize the pin")?;
        pins.pins.insert(name, pin);
        self.write_pins(&pins)?;

        Ok(())
    }

    async fn fetch(&self, opts: &AddOpts) -> Result<()> {
        let (_name, mut pin) = opts.run()?;
        self.update_one(&mut pin)
            .await
            .context("Failed to fully fetch the pin")?;
        serde_json::to_writer_pretty(std::io::stdout(), &pin)?;
        println!();

        Ok(())
    }

    async fn update_one(&self, pin: &mut Pin) -> Result<()> {
        let diff = pin.update().await?;
        if diff.len() > 0 {
            println!("changes:");
            for d in diff {
                println!("{}", d);
            }
        }

        Ok(())
    }

    async fn update(&self, opts: &UpdateOpts) -> Result<()> {
        let mut pins = self.read_pins()?;

        if let Some(name) = &opts.name {
            match pins.pins.get_mut(name) {
                None => return Err(anyhow::anyhow!("No such pin entry found.")),
                Some(p) => {
                    self.update_one(p).await?;
                },
            }
        } else {
            for (name, pin) in pins.pins.iter_mut() {
                println!("Updating {}", name);
                self.update_one(pin).await?;
            }
        }

        self.write_pins(&pins)?;

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
            Command::Init(o) => self.init(o)?,
            Command::Show => self.show()?,
            Command::Add(a) => self.add(a).await?,
            Command::Fetch(a) => self.fetch(a).await?,
            Command::Update(o) => self.update(o).await?,
            Command::Remove(r) => self.remove(r)?,
        };

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .format_timestamp(None)
        .format_target(false)
        .init();
    let opts = Opts::from_args();
    opts.run().await?;
    Ok(())
}
