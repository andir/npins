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
pub mod versions;

#[async_trait::async_trait]
pub trait Updatable:
    Serialize + Deserialize<'static> + std::fmt::Debug + Clone + PartialEq + Eq + std::hash::Hash
{
    /// The pinned hashes
    type Output: diff::Diff
        + Serialize
        + Deserialize<'static>
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Eq;

    /// Fetch the latest applicable commit data
    async fn update(&self) -> Result<Self::Output>;
}

/// Create the `Pin` type
///
/// We need a type to unify over all possible way to pin a dependency. Normally, this would be done with a trait
/// and trait objects. However, designing such a trait to be object-safe turns out to be highly non-trivial.
/// (We'd need the `serde_erase` crate for `Deserialize` alone). Since writing this as an enum is extremely repetitive,
/// this macro does the work for you.
///
/// For each pin type, call it with `(Name, lowename, InputType, OutputType)`. `Name` will be the name of the enum variant,
/// `lower_name` will be used for the constructor.
/// `InputType` and `OutputType` must adhere to the following requirements:
/// - `InputType: Updatable + Serialize + Deserialize + Debug + Clone + Eq + PartialEq + Hash`
/// - `OutputType: Serialize + Deserialize + Debug + Clone + Eq + PartialEq`
/// - Both types serialize to a map/dictionary
/// - **The serialized dictionaries of both are disjoint** (unchecked invariant at the moment)
macro_rules! mkPin {
    ( $(( $name:ident, $lower_name:ident, $input_name:path, $output_name:path )),* $(,)? ) => {
        /* The type declaration */
        /// Enum over all possible pin types
        ///
        /// Every pin type has two parts, an `input` and an `output`. The input implements [`Updatable`], which
        /// will generate output in its most up-to-date form.
        #[derive(Debug, Serialize, Deserialize, Clone)]
        #[serde(tag = "type")]
        pub enum Pin {
            $(
                /* One variant per type. input and output are serialized to a common JSON dict using `flatten`. Output is optional. */
                $name {
                    #[serde(flatten)]
                    input: $input_name,
                    #[serde(flatten)]
                    output: Option<$output_name>,
                }
            ),*
        }

        impl Pin {
            /* Constructors */
            $(fn $lower_name(input: $input_name) -> Self {
                Self::$name { input, output: None }
            })*

            /* If an error is returned, `self` remains unchanged */
            async fn update(&mut self) -> Result<Vec<diff::Difference>> {
                Ok(match self {
                    $(Self::$name { input, output } => {
                        /* Use very explicit syntax to force the correct types and get good compile errors */
                        let new_output: $output_name = <$input_name as Updatable>::update(input).await?;
                        output.insert_diffed(new_output)
                    }),*
                })
            }
        }

        impl std::fmt::Display for Pin {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    $(Self::$name { input, output } => write!(fmt, "{:?} -> {:?}", input, output)),*
                }
            }
        }

        // Each variant holds exactly one distinct type, so we can easily create convenient type wrappers that simply call the constructor
        $(
            impl From<$input_name> for Pin {
                fn from(input: $input_name) -> Self {
                    Self::$lower_name(input)
                }
            }
        )*
    };
}

mkPin! {
    (GitHub, github, github::PinInput, github::PinOutput),
    (GitHubRelease, github_release, github::ReleasePinInput, github::ReleasePinOutput),
    (Git, git, git::PinInput, git::PinOutput),
    (PyPi, pypi, pypi::PinInput, pypi::PinOutput),
}

/// The main struct the CLI operates on
///
/// For serialization purposes, use the `NixPinsVersioned` wrapper instead.
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
    /// `default.nix` and never touch your sources.json.
    Init(InitOpts),

    /// Adds a new pin entry.
    Add(AddOpts),

    /// Query some release information and then print out the entry
    Fetch(AddOpts),

    /// Lists the current pin entries.
    Show,

    /// Updates all or the given pin to the latest version.
    Update(UpdateOpts),

    /// Upgrade the sources.json and default.nix to the latest format version. This may occasionally break Nix evaluation!
    Upgrade,

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
    /// Base folder for sources.json and the boilerplate default.nix
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
        let path = self.folder.join("sources.json");
        let fh = std::io::BufReader::new(std::fs::File::open(&path).with_context(move || {
            format!(
                "Failed to open {}. You must initialize npins before you can show current pins.",
                path.display()
            )
        })?);
        versions::from_value_versioned(serde_json::from_reader(fh)?)
            .context("Failed to deserialize sources.json")
    }

    fn write_pins(&self, pins: &NixPins) -> Result<()> {
        if !self.folder.exists() {
            std::fs::create_dir(&self.folder)?;
        }
        let path = self.folder.join("sources.json");
        let fh = std::fs::File::create(&path)
            .with_context(move || format!("Failed to open {} for writing.", path.display()))?;
        serde_json::to_writer_pretty(fh, &versions::to_value_versioned(&pins))?;
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
        if self.folder.join("sources.json").exists() {
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
            println!("{}:", name);
            println!("\t{}", pin);
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

    fn upgrade(&self) -> Result<()> {
        anyhow::ensure!(
            self.folder.exists(),
            "Could not find npins folder at {}",
            self.folder.display(),
        );

        let nix_path = self.folder.join("default.nix");
        let nix_file = include_str!("../npins/default.nix");
        if std::fs::read_to_string(&nix_path)? == nix_file {
            log::info!("default.nix is already up to date");
        } else {
            log::info!("Replacing default.nix with an up to date version");
            std::fs::write(&nix_path, nix_file).context("Failed to create npins default.nix")?;
        }

        log::info!("Updating sources.json to the newest format version");
        let path = self.folder.join("sources.json");
        let fh = std::io::BufReader::new(std::fs::File::open(&path).with_context(move || {
            format!(
                "Failed to open {}. You must initialize npins before you can show current pins.",
                path.display()
            )
        })?);

        let pins_raw: serde_json::Map<String, serde_json::Value> = serde_json::from_reader(fh)
            .context("sources.json must be a valid JSON file with an object as top level")?;

        let pins_raw = versions::update(pins_raw).context("Upgrading failed")?;

        let pins: NixPins = serde_json::from_value(pins_raw)?;
        self.write_pins(&pins.into())
    }

    fn remove(&self, r: &RemoveOpts) -> Result<()> {
        let pins = self.read_pins()?;

        if !pins.pins.contains_key(&r.name) {
            return Err(anyhow::anyhow!("No such pin entry found."));
        }

        let mut new_pins = pins.clone();
        new_pins.pins.remove(&r.name);

        self.write_pins(&new_pins.into())?;

        Ok(())
    }

    async fn run(&self) -> Result<()> {
        match &self.command {
            Command::Init(o) => self.init(o)?,
            Command::Show => self.show()?,
            Command::Add(a) => self.add(a).await?,
            Command::Fetch(a) => self.fetch(a).await?,
            Command::Update(o) => self.update(o).await?,
            Command::Upgrade => self.upgrade()?,
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
