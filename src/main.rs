use anyhow::{Context, Result};
use diff::OptionExt;
use reqwest::IntoUrl;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::Write;
use structopt::StructOpt;

pub mod diff;
pub mod git;
pub mod nix;
pub mod pypi;
pub mod versions;

/// Helper method for doing various API calls
async fn get_and_deserialize<T, U>(url: U) -> anyhow::Result<T>
where
    T: for<'a> Deserialize<'a> + 'static,
    U: IntoUrl,
{
    let response = reqwest::Client::builder()
        .user_agent(concat!(
            env!("CARGO_PKG_NAME"),
            " v",
            env!("CARGO_PKG_VERSION")
        ))
        .build()?
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    Ok(serde_json::from_str(&response)?)
}

/// The main trait implemented by all pins
///
/// It comes with two associated types, `Version` and `Hashes`. Together, each of these types
/// must satisfy the following invariants:
/// - They serialize to a map/dictionary/object, however you want to call it
/// - **The serialized dictionaries of all are disjoint** (unchecked invariant at the moment)
#[async_trait::async_trait]
pub trait Updatable:
    Serialize + Deserialize<'static> + std::fmt::Debug + Clone + PartialEq + Eq + std::hash::Hash
{
    /// Version information, produced by the [`update`](Self::update) method.
    ///
    /// It should contain information that charactarizes a version, e.g. the release version.
    /// A user should be able to manually specify it, if they want to pin a specific version.
    /// Each version should map to the same set of hashes over time, and violations of this
    /// should only be caused by upstream errors.
    type Version: diff::Diff
        + Serialize
        + Deserialize<'static>
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Eq;

    /// The pinned hashes for a given version, produced by the [`fetch`](Self::fetch) method.
    ///
    /// It may contain multiple different hashes, or download URLs that go with them.
    type Hashes: diff::Diff
        + Serialize
        + Deserialize<'static>
        + std::fmt::Debug
        + Clone
        + PartialEq
        + Eq;

    /// Fetch the latest applicable commit data
    ///
    /// The old version may be passed to help guarantee monotonicity of the versions.
    async fn update(&self, old: Option<&Self::Version>) -> Result<Self::Version>;

    /// Fetch hashes for a given version
    async fn fetch(&self, version: &Self::Version) -> Result<Self::Hashes>;
}

/// Create the `Pin` type
///
/// We need a type to unify over all possible way to pin a dependency. Normally, this would be done with a trait
/// and trait objects. However, designing such a trait to be object-safe turns out to be highly non-trivial.
/// (We'd need the `serde_erase` crate for `Deserialize` alone). Since writing this as an enum is extremely repetitive,
/// this macro does the work for you.
///
/// For each pin type, call it with `(Name, lower_name, Type)`. `Name` will be the name of the enum variant,
/// `lower_name` will be used for the constructor.
macro_rules! mkPin {
    ( $(( $name:ident, $lower_name:ident, $input_name:path )),* $(,)? ) => {
        /* The type declaration */
        /// Enum over all possible pin types
        ///
        /// Every pin type has two parts, an `input` and an `output`. The input implements [`Updatable`], which
        /// will generate output in its most up-to-date form.
        #[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
        #[serde(tag = "type")]
        pub enum Pin {
            $(
                /* One variant per type. input and output are serialized to a common JSON dict using `flatten`. Output is optional. */
                $name {
                    #[serde(flatten)]
                    input: $input_name,
                    #[serde(flatten)]
                    version: Option<<$input_name as Updatable>::Version>,
                    #[serde(flatten)]
                    hashes: Option<<$input_name as Updatable>::Hashes>,
                }
            ),*
        }

        impl Pin {
            /* Constructors */
            $(fn $lower_name(input: $input_name, version: Option<<$input_name as Updatable>::Version>) -> Self {
                Self::$name { input, version, hashes: None }
            })*

            /* If an error is returned, `self` remains unchanged */
            async fn update(&mut self) -> Result<Vec<diff::Difference>> {
                Ok(match self {
                    $(Self::$name { input, version, .. } => {
                        /* Use very explicit syntax to force the correct types and get good compile errors */
                        let new_version = <$input_name as Updatable>::update(input, version.as_ref()).await?;
                        version.insert_diffed(new_version)
                    }),*
                })
            }

            /* If an error is returned, `self` remains unchanged. This returns a double result: the outer one
             * indicates that `update` should be called first, the inner is from the actual operation.
             */
            async fn fetch(&mut self) -> Result<Vec<diff::Difference>> {
                Ok(match self {
                    $(Self::$name { input, version, hashes } => {
                        let version = version.as_ref()
                            .ok_or_else(|| anyhow::format_err!("No version information available, call `update` first or manually set one"))?;
                        /* Use very explicit syntax to force the correct types and get good compile errors */
                        let new_hashes = <$input_name as Updatable>::fetch(input, &version).await?;
                        hashes.insert_diffed(new_hashes)
                    }),*
                })
            }

            pub fn has_version(&self) -> bool {
                match self {
                    $(Self::$name { version, ..} => version.is_some() ),*
                }
            }

            pub fn has_hashes(&self) -> bool {
                match self {
                    $(Self::$name { hashes, ..} => hashes.is_some() ),*
                }
            }
        }

        impl std::fmt::Display for Pin {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                match self {
                    $(Self::$name { input, version, hashes } => write!(fmt, "{:?} -> {:?} -> {:?}", input, version, hashes)),*
                }
            }
        }

        // Each variant holds exactly one distinct type, so we can easily create convenient type wrappers that simply call the constructor
        $(
            impl From<$input_name> for Pin {
                fn from(input: $input_name) -> Self {
                    Self::$lower_name(input, None)
                }
            }

            impl From<($input_name, Option<<$input_name as Updatable>::Version>)> for Pin {
                fn from((input, version): ($input_name, Option<<$input_name as Updatable>::Version>)) -> Self {
                    Self::$lower_name(input, version)
                }
            }

            impl From<($input_name, <$input_name as Updatable>::Version)> for Pin {
                fn from((input, version): ($input_name, <$input_name as Updatable>::Version)) -> Self {
                    (input, Some(version)).into()
                }
            }
        )*
    };
}

mkPin! {
    (Git, git, git::GitPin),
    (GitRelease, git_release, git::GitReleasePin),
    (PyPi, pypi, pypi::Pin),
}

/// The main struct the CLI operates on
///
/// For serialization purposes, use the `NixPinsVersioned` wrapper instead.
#[derive(Debug, Serialize, Deserialize, Clone, Default, PartialEq, Eq)]
pub struct NixPins {
    pins: BTreeMap<String, Pin>,
}

impl NixPins {
    pub fn new_with_nixpkgs() -> Self {
        let mut pins = BTreeMap::new();
        pins.insert(
            "nixpkgs".to_owned(),
            git::GitPin::github("nixos", "nixpkgs", "nixpkgs-unstable".to_owned()).into(),
        );
        Self { pins }
    }
}

/// Just a version string
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GenericVersion {
    /// Note that "version" must be seen in the context of the pin.
    /// Without that context, it shall be treated as opaque string.
    pub version: String,
}

impl diff::Diff for GenericVersion {
    fn diff(&self, other: &Self) -> Vec<diff::Difference> {
        diff::d(&[diff::Difference::new(
            "version",
            &self.version,
            &other.version,
        )])
    }
}

/// An URL and its hash
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct GenericUrlHashes {
    pub url: url::Url,
    pub hash: String,
}

impl diff::Diff for GenericUrlHashes {
    fn diff(&self, other: &Self) -> Vec<diff::Difference> {
        diff::d(&[
            diff::Difference::new("hash", &self.hash, &other.hash),
            diff::Difference::new("url", &self.url, &other.url),
        ])
    }
}

use url::Url;

#[derive(Debug, StructOpt)]
pub struct GenericGitAddOpts {
    /// Track a branch instead of a release
    #[structopt(short, long)]
    pub branch: Option<String>,

    /// Use a specific commit/release instead of the latest.
    /// This may be a tag name, or a git revision when --branch is set.
    #[structopt(long, value_name = "tag or rev")]
    pub at: Option<String>,

    /// Also track pre-releases.
    /// Conflicts with the --branch option.
    #[structopt(long, conflicts_with = "branch")]
    pub pre_releases: bool,

    /// Bound the version resolution. For example, setting this to "2" will
    /// restrict updates to 1.X versions. Conflicts with the --branch option.
    #[structopt(
        long = "upper-bound",
        value_name = "version",
        conflicts_with = "branch"
    )]
    pub version_upper_bound: Option<String>,
}

#[derive(Debug, StructOpt)]
pub struct GitHubAddOpts {
    pub owner: String,
    pub repository: String,

    #[structopt(flatten)]
    pub more: GenericGitAddOpts,
}

impl GitHubAddOpts {
    pub fn add(&self) -> Result<(String, Pin)> {
        Ok((
            self.repository.clone(),
            match &self.more.branch {
                Some(branch) => {
                    let pin = git::GitPin::github(&self.repository, &self.owner, branch.clone());
                    let version = self.more.at.as_ref().map(|at| git::GitRevision {
                        revision: at.clone(),
                    });
                    (pin, version).into()
                },
                None => {
                    let pin = git::GitReleasePin::github(
                        &self.owner,
                        &self.repository,
                        self.more.pre_releases,
                        self.more.version_upper_bound.clone(),
                    );
                    let version = self.more.at.as_ref().map(|at| GenericVersion {
                        version: at.clone(),
                    });
                    (pin, version).into()
                },
            },
        ))
    }
}

#[derive(Debug, StructOpt)]
pub struct GitLabAddOpts {
    /// Usually just `"owner" "repository"`, but GitLab allows arbitrary folder-like structures.
    #[structopt(required = true, min_values = 2)]
    pub repo_path: Vec<String>,

    #[structopt(
        long,
        default_value = "https://gitlab.com/",
        help = "Use a self-hosted GitLab instance instead",
        value_name = "url"
    )]
    pub server: url::Url,

    #[structopt(flatten)]
    pub more: GenericGitAddOpts,
}

impl GitLabAddOpts {
    pub fn add(&self) -> Result<(String, Pin)> {
        Ok((
            self.repo_path
                .last()
                .ok_or_else(|| anyhow::format_err!("GitLab repository path must at least have one element (usually two: owner, repo)"))?
                .clone(),
            match &self.more.branch {
                Some(branch) =>{
                    let pin = git::GitPin::gitlab(
                        self.repo_path.join("/"),
                        branch.clone(),
                        Some(self.server.clone()),
                    );
                    let version = self.more.at.as_ref()
                    .map(|at| git::GitRevision {
                        revision: at.clone(),
                    });
                    (pin, version).into()},
                None => {
                    let pin = git::GitReleasePin::gitlab(
                        self.repo_path.join("/"),
                        Some(self.server.clone()),
                        self.more.pre_releases,
                        self.more.version_upper_bound.clone(),
                    );
                    let version = self.more.at.as_ref()
                        .map(|at| GenericVersion {
                            version: at.clone(),
                        });
                    (pin, version).into()
                },
            },
        ))
    }
}

#[derive(Debug, StructOpt)]
pub struct GitAddOpts {
    /// The git remote URL. For example <https://github.com/andir/ate.git>
    pub url: String,

    #[structopt(flatten)]
    pub more: GenericGitAddOpts,
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
            match &self.more.branch {
                Some(branch) => {
                    let pin = git::GitPin::git(url, branch.clone());
                    let version = self.more.at.as_ref().map(|at| git::GitRevision {
                        revision: at.clone(),
                    });
                    (pin, version).into()
                },
                None => {
                    let pin = git::GitReleasePin::git(
                        url,
                        self.more.pre_releases,
                        self.more.version_upper_bound.clone(),
                    );
                    let version = self.more.at.as_ref().map(|at| GenericVersion {
                        version: at.clone(),
                    });
                    (pin, version).into()
                },
            },
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
            pypi::Pin {
                name: self.name.clone(),
            }
            .into(),
        ))
    }
}

#[derive(Debug, StructOpt)]
pub enum AddCommands {
    /// Track a GitHub repository
    #[structopt(name = "github")]
    GitHub(GitHubAddOpts),
    /// Track a GitLab repository
    #[structopt(name = "gitlab")]
    GitLab(GitLabAddOpts),
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
            AddCommands::GitLab(gl) => gl.add()?,
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
    /// Update only those pins
    pub names: Vec<String>,
    /// Don't update versions, only re-fetch hashes
    #[structopt(short, long, conflicts_with = "full")]
    pub partial: bool,
    /// Re-fetch hashes even if the version hasn't changed.
    /// Useful to make sure the derivations are in the Nix store.
    #[structopt(short, long, conflicts_with = "partial")]
    pub full: bool,
    /// Print the diff, but don't write back the changes
    #[structopt(short = "n", long)]
    pub dry_run: bool,
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
        serde_json::to_writer_pretty(fh, &versions::to_value_versioned(pins))?;
        Ok(())
    }

    async fn init(&self, o: &InitOpts) -> Result<()> {
        log::info!("Welcome to npins!");
        let default_nix = include_bytes!("../npins/default.nix");
        if !self.folder.exists() {
            log::info!("Creating `{}` directory", self.folder.display());
            std::fs::create_dir(&self.folder).context("Failed to create npins folder")?;
        }
        log::info!("Writing default.nix");
        let p = self.folder.join("default.nix");
        let mut fh = std::fs::File::create(&p).context("Failed to create npins default.nix")?;
        fh.write_all(default_nix)?;

        // Only create the pins if the file isn't there yet
        if self.folder.join("sources.json").exists() {
            log::info!("Done.");
            return Ok(());
        }

        let initial_pins = if o.bare {
            log::info!("Writing initial sources.json (empty)");
            NixPins::default()
        } else {
            log::info!("Writing initial sources.json with nixpkgs entry (need to fetch latest commit first)");
            let mut pin = NixPins::new_with_nixpkgs();
            self.update_one(pin.pins.get_mut("nixpkgs").unwrap(), false, true)
                .await
                .context("Failed to fetch initial nixpkgs entry")?;
            pin
        };
        self.write_pins(&initial_pins)?;
        log::info!("Done.");
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
        let has_version = pin.has_version();
        self.update_one(&mut pin, has_version, false)
            .await
            .context("Failed to fully initialize the pin")?;
        pins.pins.insert(name, pin);
        self.write_pins(&pins)?;

        Ok(())
    }

    async fn fetch(&self, opts: &AddOpts) -> Result<()> {
        let (_name, mut pin) = opts.run()?;
        let has_version = pin.has_version();
        self.update_one(&mut pin, has_version, false)
            .await
            .context("Failed to fully fetch the pin")?;
        serde_json::to_writer_pretty(std::io::stdout(), &pin)?;
        println!();

        Ok(())
    }

    async fn update_one(&self, pin: &mut Pin, partial: bool, full: bool) -> Result<()> {
        assert!(
            !(partial && full),
            "partial and full are mutually exclusive"
        );

        /* Skip this for partial updates */
        let diff1 = if !partial {
            pin.update().await?
        } else {
            vec![]
        };

        /* We only need to fetch the hashes if the version changed, or if the flags indicate that we should */
        if !diff1.is_empty() || full || partial {
            let diff2 = pin.fetch().await?;

            if diff1.len() + diff2.len() > 0 {
                println!("changes:");
                for d in diff1 {
                    println!("{}", d);
                }
                for d in diff2 {
                    println!("{}", d);
                }
            }
        }

        Ok(())
    }

    async fn update(&self, opts: &UpdateOpts) -> Result<()> {
        let mut pins = self.read_pins()?;

        if opts.names.is_empty() {
            for (name, pin) in pins.pins.iter_mut() {
                println!("Updating {}", name);
                self.update_one(pin, opts.partial, opts.full).await?;
            }
        } else {
            for name in &opts.names {
                match pins.pins.get_mut(name) {
                    None => return Err(anyhow::anyhow!("No such pin entry found.")),
                    Some(pin) => {
                        println!("Updating {}", name);
                        self.update_one(pin, opts.partial, opts.full).await?;
                    },
                }
            }
        }

        if !opts.dry_run {
            self.write_pins(&pins)?;
        }

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

        log::info!("Upgrading sources.json to the newest format version");
        let path = self.folder.join("sources.json");
        let fh = std::io::BufReader::new(std::fs::File::open(&path).with_context(move || {
            format!(
                "Failed to open {}. You must initialize npins before you can show current pins.",
                path.display()
            )
        })?);

        let pins_raw: serde_json::Map<String, serde_json::Value> = serde_json::from_reader(fh)
            .context("sources.json must be a valid JSON file with an object as top level")?;

        let pins_raw_new = versions::upgrade(pins_raw.clone()).context("Upgrading failed")?;
        let pins: NixPins = serde_json::from_value(pins_raw_new.clone())?;
        if pins_raw_new != serde_json::Value::Object(pins_raw) {
            log::info!("Done. It is recommended to at least run `update --partial` afterwards.");
        }
        self.write_pins(&pins)
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

    pub async fn run(&self) -> Result<()> {
        match &self.command {
            Command::Init(o) => self.init(o).await?,
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
