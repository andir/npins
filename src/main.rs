//! The main CLI application

use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use crossterm::{
    QueueableCommand,
    cursor::MoveToPreviousLine,
    style::{Print, Stylize},
    terminal::{Clear, ClearType},
};
use futures_util::{
    TryStreamExt,
    stream::{self, StreamExt},
};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    fs::File,
    future,
    io::{BufReader, IsTerminal, Write, stderr},
    path::PathBuf,
};
use url::{ParseError, Url};

use libnpins::*;

/// How to handle updates
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum UpdateStrategy {
    /// Fetch latest version, update hashes if necessary
    Normal,
    /// Update hashes of the currently pinned version
    HashesOnly,
    /// Fetch latest version, always update hashes
    Full,
}

impl UpdateStrategy {
    /// Whether the latest version should be fetched
    pub fn should_update(&self) -> bool {
        match self {
            UpdateStrategy::Normal => true,
            UpdateStrategy::HashesOnly => false,
            UpdateStrategy::Full => true,
        }
    }

    /// Whether we want to force-update the hashes
    pub fn must_fetch(&self) -> bool {
        match self {
            UpdateStrategy::Normal => false,
            UpdateStrategy::HashesOnly => true,
            UpdateStrategy::Full => true,
        }
    }
}

#[derive(Debug, Parser)]
pub struct ChannelAddOpts {
    channel_name: String,
}

impl ChannelAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        Ok((
            Some(self.channel_name.clone()),
            channel::Pin {
                name: self.channel_name.clone(),
            }
            .into(),
        ))
    }
}

#[derive(Debug, Parser)]
pub struct GenericGitAddOpts {
    /// Track a branch instead of a release
    #[arg(short, long)]
    pub branch: Option<String>,

    /// Use a specific commit/release instead of the latest.
    /// This may be a tag name, or a git revision when --branch is set.
    #[arg(long, value_name = "tag or rev")]
    pub at: Option<String>,

    /// Also track pre-releases.
    /// Conflicts with the --branch option.
    #[arg(long, conflicts_with = "branch")]
    pub pre_releases: bool,

    /// Bound the version resolution. For example, setting this to "2" will
    /// restrict updates to 1.X versions. Conflicts with the --branch option.
    #[arg(
        long = "upper-bound",
        value_name = "version",
        conflicts_with_all = &["branch", "at"]
    )]
    pub version_upper_bound: Option<String>,

    /// Optional prefix required for each release name / tag. For
    /// example, setting this to "release/" will only consider those
    /// that start with that string.
    #[arg(long = "release-prefix")]
    pub release_prefix: Option<String>,

    /// Also fetch submodules
    #[arg(long)]
    pub submodules: bool,
}

impl GenericGitAddOpts {
    fn add(&self, repository: git::Repository) -> Result<Pin> {
        Ok(match &self.branch {
            Some(branch) => {
                let pin = git::GitPin::new(repository, branch.clone(), self.submodules);
                let version = self
                    .at
                    .as_ref()
                    .map(|at| git::GitRevision::new(at.clone()))
                    .transpose()?;
                (pin, version).into()
            },
            None => {
                let pin = git::GitReleasePin::new(
                    repository,
                    self.pre_releases,
                    self.version_upper_bound.clone(),
                    self.release_prefix.clone(),
                    self.submodules,
                );
                let version = self.at.as_ref().map(|at| GenericVersion {
                    version: at.clone(),
                });
                (pin, version).into()
            },
        })
    }
}

#[derive(Debug, Parser)]
pub struct GitHubAddOpts {
    pub owner: String,
    pub repository: String,

    #[command(flatten)]
    pub more: GenericGitAddOpts,
}

impl GitHubAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        let repository = git::Repository::github(&self.owner, &self.repository);

        Ok((Some(self.repository.clone()), self.more.add(repository)?))
    }
}

#[derive(Debug, Parser)]
pub struct ForgejoAddOpts {
    pub server: String,
    pub owner: String,
    pub repository: String,

    #[command(flatten)]
    pub more: GenericGitAddOpts,
}
impl ForgejoAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        let server_url = Url::parse(&self.server).or_else(|err| match err {
            ParseError::RelativeUrlWithoutBase => {
                Url::parse(&("https://".to_string() + self.server.as_str()))
            },
            _ => Err(err),
        })?;
        let repository = git::Repository::forgejo(server_url, &self.owner, &self.repository);

        Ok((Some(self.repository.clone()), self.more.add(repository)?))
    }
}

#[derive(Debug, Parser)]
pub struct GitLabAddOpts {
    /// Usually just `"owner" "repository"`, but GitLab allows arbitrary folder-like structures.
    #[arg(required = true)] // TODO set min number of values to 2 again
    pub repo_path: Vec<String>,

    #[arg(
        long,
        default_value = "https://gitlab.com/",
        help = "Use a self-hosted GitLab instance instead",
        value_name = "url"
    )]
    pub server: url::Url,

    #[arg(
        long,
        help = "Use a private token to access the repository.",
        value_name = "token"
    )]
    pub private_token: Option<String>,

    #[command(flatten)]
    pub more: GenericGitAddOpts,
}

impl GitLabAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        let repository = git::Repository::gitlab(
            self.repo_path.join("/"),
            Some(self.server.clone()),
            self.private_token.clone(),
        );
        Ok((
            Some(self.repo_path
                .last()
                .context("GitLab repository path must at least have one element (usually two: owner, repo)")?
                .clone()),
            self.more.add(repository)?,
        ))
    }
}

#[derive(Debug, Parser, Clone, Copy, Default, ValueEnum)]
pub enum GitForgeOpts {
    /// A generic git pin, with no further information
    None,
    #[default]
    /// Try to determine the Forge from the given url, potentially by probing the server
    Auto,
    /// A Gitlab forge, e.g. gitlab.com
    Gitlab,
    /// A Github forge, i.e. github.com
    Github,
    /// A Forgejo forge, e.g. forgejo.org
    Forgejo,
}

#[derive(Debug, Parser)]
pub struct GitAddOpts {
    /// The git remote URL. For example <https://github.com/andir/ate.git>
    pub url: String,

    #[arg(long, value_enum, default_value = "auto")]
    pub forge: GitForgeOpts,

    #[command(flatten)]
    pub more: GenericGitAddOpts,
}

impl GitAddOpts {
    pub async fn add(&self) -> Result<(Option<String>, Pin)> {
        let url = Url::parse(&self.url)
            .map_err(|e| {
                match e {
                    url::ParseError::RelativeUrlWithoutBase => {
                        anyhow::format_err!("URL scheme is missing. For git URLs, add the fully qualified scheme like git+ssh://. For local repositories, add file://")
                    },
                    url::ParseError::InvalidPort => {
                        anyhow::format_err!("Invalid port number. For git URLs, try inserting a '/' after the ':' before the user name, like so: git+ssh://git@gitlab-instance.net:/user/repo.git")
                    },
                    e => e.into(),
                }
            })
            .context("Failed to parse repository URL")?;

        if url.scheme().contains('.') {
            log::warn!(
                "Your URL scheme ('{}:') contains a '.', which is unusual. Please double-check its correctness.",
                url.scheme()
            );
            log::warn!(
                "Very likely you forgot to specify the scheme, and the host name parsed as such instead."
            );
        }
        let name = match url.path_segments().and_then(|mut x| x.next_back()) {
            None => anyhow::bail!(
                "Path of URL must start with a '/'. Also make sure that the URL starts with a scheme."
            ),
            Some(seg) => seg.to_owned(),
        };
        let name = name.strip_suffix(".git").unwrap_or(&name);

        use git::Repository;
        let repository = match self.forge {
            GitForgeOpts::Auto => Repository::git_auto(url).await,
            GitForgeOpts::None => Repository::git(url),
            GitForgeOpts::Github => Repository::github_from_url(url)
                .context("Could not parse the URL as GitHub repository")?,
            GitForgeOpts::Gitlab => Repository::gitlab_from_url(url)
                .context("Could not parse the URL as GitLab repository")?,
            GitForgeOpts::Forgejo => Repository::forgejo_from_url(url)
                .context("Could not parse the URL as Forgejo repository")?,
        };

        Ok((Some(name.to_owned()), self.more.add(repository)?))
    }
}

#[derive(Debug, Parser)]
pub struct PyPiAddOpts {
    /// Name of the package at PyPi.org
    pub package_name: String,

    /// Use a specific release instead of the latest.
    #[arg(long, value_name = "version")]
    pub at: Option<String>,

    /// Bound the version resolution. For example, setting this to "2" will
    /// restrict updates to 1.X versions. Conflicts with the --branch option.
    #[arg(long = "upper-bound", value_name = "version", conflicts_with = "at")]
    pub version_upper_bound: Option<String>,
}

impl PyPiAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        Ok((Some(self.package_name.clone()), {
            let pin = pypi::Pin {
                name: self.package_name.clone(),
                version_upper_bound: self.version_upper_bound.clone(),
            };
            let version = self.at.as_ref().map(|at| GenericVersion {
                version: at.clone(),
            });
            (pin, version).into()
        }))
    }
}

#[derive(Debug, Parser)]
pub struct ContainerAddOpts {
    pub image_name: String,
    pub image_tag: String,
}

impl ContainerAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        Ok((
            Some(self.image_name.clone()),
            container::Pin {
                image_name: self.image_name.clone(),
                image_tag: self.image_tag.clone(),
            }
            .into(),
        ))
    }
}

#[derive(Debug, Parser)]
pub struct TarballAddOpts {
    /// Tarball URL
    pub url: Url,
}

impl TarballAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        let url = self.url.clone();
        Ok((None, tarball::TarballPin { url }.into()))
    }
}

#[derive(Debug, Subcommand)]
pub enum AddCommands {
    /// Track a Nix channel
    #[command(name = "channel")]
    Channel(ChannelAddOpts),
    /// Track a GitHub repository
    #[command(name = "github")]
    GitHub(GitHubAddOpts),
    /// Track a Forgejo repository
    #[command(name = "forgejo")]
    Forgejo(ForgejoAddOpts),
    /// Track a GitLab repository
    #[command(name = "gitlab")]
    GitLab(GitLabAddOpts),
    /// Track a git repository
    #[command(name = "git")]
    Git(GitAddOpts),
    /// Track a package on PyPi
    #[command(name = "pypi")]
    PyPi(PyPiAddOpts),
    /// Track an OCI container
    #[command(name = "container")]
    Container(ContainerAddOpts),
    /// Track a tarball
    ///
    /// This can be either a static URL that never changes its contents or a
    /// URL which supports flakes "Lockable HTTP Tarball" API.
    #[command(name = "tarball")]
    Tarball(TarballAddOpts),
}

#[derive(Debug, Parser)]
pub struct AddOpts {
    /// Add the pin with a custom name.
    /// If a pin with that name already exists, it will be overwritten
    #[arg(long, global = true)]
    pub name: Option<String>,
    /// Add the pin as frozen, meaning that it will be ignored by `npins update` by default.
    #[arg(long, global = true)]
    pub frozen: bool,
    /// Don't actually apply the changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,
    #[command(subcommand)]
    command: AddCommands,
}

impl AddOpts {
    async fn run(&self) -> Result<(String, Pin)> {
        let (name, mut pin) = match &self.command {
            AddCommands::Channel(c) => c.add()?,
            AddCommands::Git(g) => g.add().await?,
            AddCommands::GitHub(gh) => gh.add()?,
            AddCommands::Forgejo(fg) => fg.add()?,
            AddCommands::GitLab(gl) => gl.add()?,
            AddCommands::PyPi(p) => p.add()?,
            AddCommands::Tarball(p) => p.add()?,
            AddCommands::Container(p) => p.add()?,
        };

        let name = match (&self.name, name) {
            (Some(user_specified), _) => user_specified.clone(),
            (None, Some(guess_from_pin)) => guess_from_pin,
            (None, None) => {
                anyhow::bail!(
                    "Couldn't pick a Pin name automatically. Use --name to specify one manually"
                )
            },
        };
        if self.frozen {
            pin.freeze();
        }

        Ok((name, pin))
    }
}

#[derive(Debug, Parser)]
pub struct ShowOpts {
    /// Name of the pin to show
    pub names: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct RemoveOpts {
    pub name: String,
}

#[derive(Debug, Parser)]
pub struct UpdateOpts {
    /// Updates only the specified pins.
    pub names: Vec<String>,
    /// Don't update versions, only re-fetch hashes
    #[arg(short, long, conflicts_with = "full")]
    pub partial: bool,
    /// Re-fetch hashes even if the version hasn't changed.
    /// Useful to make sure the derivations are in the Nix store.
    #[arg(short, long, conflicts_with = "partial")]
    pub full: bool,
    /// Print the diff, but don't write back the changes
    #[arg(short = 'n', long, global = true)]
    pub dry_run: bool,
    /// Allow updating frozen pins, which would otherwise be ignored
    #[arg(long = "frozen")]
    pub update_frozen: bool,
    /// Maximum number of simultaneous downloads
    #[structopt(default_value = "5", long)]
    pub max_concurrent_downloads: usize,
}

#[derive(Debug, Parser)]
pub struct VerifyOpts {
    /// Verifies only the specified pins.
    pub names: Vec<String>,
    /// Maximum number of simultaneous downloads
    #[structopt(default_value = "5", long)]
    pub max_concurrent_downloads: usize,
}

#[derive(Debug, Parser)]
pub struct InitOpts {
    /// Don't add an initial `nixpkgs` entry
    #[arg(long)]
    pub bare: bool,
}

#[derive(Debug, Parser)]
pub struct ImportOpts {
    #[arg(default_value = "nix/sources.json")]
    pub path: PathBuf,
    /// Only import one entry from Niv
    #[arg(short, long)]
    pub name: Option<String>,
}

#[derive(Debug, Parser)]
pub struct ImportFlakeOpts {
    #[arg(default_value = "flake.lock")]
    pub path: PathBuf,
    /// Only import one entry from the flake
    #[arg(short, long)]
    pub name: Option<String>,
}

#[derive(Debug, Parser)]
pub struct FreezeOpts {
    /// Names of the pin(s)
    #[structopt(required = true)]
    pub names: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct GetPathOpts {
    /// Name of the pin
    #[structopt(required = true)]
    pub name: String,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Intializes the npins directory. Running this multiple times will restore/upgrade the
    /// `default.nix` and never touch your sources.json.
    Init(InitOpts),

    /// Adds a new pin entry.
    // Boxing AddOpts as it is by far our largest structure, reduces
    // memory requirements for smaller devices (even if maginal)
    Add(Box<AddOpts>),

    /// Lists the current pin entries.
    Show(ShowOpts),

    /// Updates all or the given pins to the latest version.
    Update(UpdateOpts),

    /// Verifies that all or the given pins still have correct hashes. This is like `update --partial --dry-run` and then checking that the diff is empty
    Verify(VerifyOpts),

    /// Upgrade the sources.json and default.nix to the latest format version. This may occasionally break Nix evaluation!
    Upgrade,

    /// Removes one pin entry.
    Remove(RemoveOpts),

    /// Try to import entries from Niv
    ImportNiv(ImportOpts),

    /// Try to import entries from flake.lock
    ImportFlake(ImportFlakeOpts),

    /// Freeze a pin entry
    Freeze(FreezeOpts),

    /// Thaw a pin entry
    Unfreeze(FreezeOpts),

    /// Evaluates the store path to a pin, fetching it if necessary. Don't forget to add a GC root
    GetPath(GetPathOpts),
}

#[derive(Debug, Parser)]
#[command(
    version,
    about,
    arg_required_else_help = true,
    // Confirm clap defaults
    propagate_version = false,
    disable_colored_help = false,
    color = clap::ColorChoice::Auto
)]
pub struct Opts {
    /// Base folder for sources.json and the boilerplate default.nix
    #[arg(
        short = 'd',
        long = "directory",
        default_value = "npins",
        env = "NPINS_DIRECTORY"
    )]
    folder: std::path::PathBuf,

    /// Specifies the path to the sources.json and activates lockfile mode.
    /// In lockfile mode, no default.nix will be generated and --directory will be ignored.
    #[arg(long)]
    lock_file: Option<std::path::PathBuf>,

    /// Print debug messages.
    #[arg(global = true, short = 'v', long = "verbose")]
    pub verbose: bool,

    #[command(subcommand)]
    command: Command,
}

fn write_diff(writer: &mut impl Write, name: &str, diff: &[diff::DiffEntry]) {
    if diff.is_empty() {
        writeln!(writer, "[{name}] No Changes").unwrap();
    } else {
        writeln!(writer, "[{name}] Changes:").unwrap();
        for entry in diff {
            write!(writer, "{entry}").unwrap();
        }
    }
}

impl Opts {
    fn read_pins(&self) -> Result<NixPins> {
        let path = if let Some(lock_file) = self.lock_file.as_ref() {
            lock_file.to_owned()
        } else {
            self.folder.join("sources.json")
        };
        let fh = BufReader::new(File::open(&path).with_context(move || {
            format!(
                "Failed to open {}. You must initialize npins before you can show current pins.",
                path.display()
            )
        })?);
        NixPins::from_json_versioned(serde_json::from_reader(fh)?)
            .context("Failed to deserialize sources.json")
    }

    fn write_pins(&self, pins: &NixPins) -> Result<()> {
        let path = if let Some(lock_file) = &self.lock_file {
            lock_file.to_owned()
        } else {
            if !self.folder.exists() {
                std::fs::create_dir(&self.folder)?;
            }
            self.folder.join("sources.json")
        };
        let mut fh = File::create(&path)
            .with_context(move || format!("Failed to open {} for writing.", path.display()))?;
        serde_json::to_writer_pretty(&mut fh, &pins.to_value_versioned())?;
        fh.write_all(b"\n")?;
        Ok(())
    }

    async fn init(&self, o: &InitOpts) -> Result<()> {
        log::info!("Welcome to npins!");

        // Skip the entire default.nix and convenience creating folders bit in lockfile mode
        if self.lock_file.is_none() {
            let default_nix = DEFAULT_NIX;
            if !self.folder.exists() {
                log::info!("Creating `{}` directory", self.folder.display());
                std::fs::create_dir(&self.folder).context("Failed to create npins folder")?;
            }
            log::info!("Writing default.nix");
            let p = self.folder.join("default.nix");
            let mut fh = File::create(&p).context("Failed to create npins default.nix")?;
            fh.write_all(default_nix.as_bytes())?;
        }

        let sources_json = self.folder.join("sources.json");
        let path = self.lock_file.as_ref().unwrap_or(&sources_json);
        // Only create the pins if the file isn't there yet
        if path.exists() {
            log::info!(
                "The file '{}' already exists; nothing to do.",
                path.display()
            );
            return Ok(());
        }

        let initial_pins = if o.bare {
            log::info!("Writing initial lock file (empty)");
            NixPins::default()
        } else {
            log::info!(
                "Writing initial lock file with nixpkgs entry (need to fetch latest commit first)"
            );
            let mut pin = NixPins::new_with_nixpkgs();
            Self::update_one(
                "nixpkgs",
                pin.pins.get_mut("nixpkgs").unwrap(),
                UpdateStrategy::Full,
            )
            .await
            .context("Failed to fetch initial nixpkgs entry")?;
            pin
        };
        self.write_pins(&initial_pins)?;
        log::info!(
            "Successfully written initial files to '{}'.",
            self.lock_file
                .as_ref()
                .unwrap_or(&self.folder.join("sources.json"))
                .display()
        );
        Ok(())
    }

    fn show(&self, opts: &ShowOpts) -> Result<()> {
        let pins = self.read_pins()?;

        let print_pin = |name: &str, pin: &Pin| {
            println!("{}: ({})", name, pin.pin_type());
            println!("{}", pin);
        };

        let mut errors = vec![];

        match &opts.names[..] {
            [] => {
                for (name, pin) in pins.pins.iter() {
                    print_pin(name, pin);
                }
            },
            names => {
                for name in names {
                    match pins.pins.get(name) {
                        None => {
                            errors.push(name.clone());
                        },
                        Some(pin) => {
                            print_pin(name, pin);
                        },
                    }
                }
            },
        }

        if !errors.is_empty() {
            return Err(anyhow::anyhow!(
                "Couldn't find the following pins: {:?}",
                errors
            ));
        }

        Ok(())
    }

    async fn add(&self, opts: &AddOpts) -> Result<()> {
        let mut pins = self.read_pins()?;
        let (name, mut pin) = opts.run().await?;
        if opts.frozen {
            log::info!("Adding '{}' (frozen) …", name);
        } else {
            log::info!("Adding '{}' …", name);
        }
        /* Fetch the latest version unless the user specified some */
        let strategy = if pin.has_version() {
            UpdateStrategy::HashesOnly
        } else {
            UpdateStrategy::Full
        };
        Self::update_one(&name, &mut pin, strategy)
            .await
            .context("Failed to fully initialize the pin")?;
        pins.pins.insert(name.clone(), pin.clone());
        if !opts.dry_run {
            self.write_pins(&pins)?;
        }

        println!("{}", pin);
        Ok(())
    }

    async fn update_one(
        name: &str,
        pin: &mut Pin,
        strategy: UpdateStrategy,
    ) -> Result<Vec<diff::DiffEntry>> {
        /* Skip this for partial updates */
        let diff1 = if strategy.should_update() {
            pin.update()
                .await
                .with_context(|| format!("Updating {}", name))?
        } else {
            vec![]
        };

        /* We only need to fetch the hashes if the version changed, or if the flags indicate that we should */
        let diff = if !diff1.is_empty() || strategy.must_fetch() {
            let diff2 = pin
                .fetch()
                .await
                .with_context(|| format!("Fetching {}", name))?;
            diff1.into_iter().chain(diff2.into_iter()).collect()
        } else {
            diff1
        };

        Ok(diff)
    }

    async fn update(&self, opts: &UpdateOpts) -> Result<()> {
        let mut pins = self.read_pins()?;

        let mut selected_pins = BTreeSet::new();
        for name in &opts.names {
            if !selected_pins.insert(name) {
                log::warn!("Ignoring duplicate pin: {name}")
            }
        }
        selected_pins.retain(|&name| match pins.pins.get(name) {
            Some(p) if !opts.update_frozen && p.is_frozen() => {
                log::warn!("Ignoring frozen pin: {name}");
                false
            },
            Some(_) => true,
            None => {
                log::warn!("Specified pin does not exist: {name}");
                false
            },
        });

        let length = if opts.names.is_empty() {
            pins.pins
                .iter()
                .filter(|(_, pin)| opts.update_frozen || !pin.is_frozen())
                .count()
        } else {
            selected_pins.len()
        };

        if length == 0 {
            return Err(anyhow::anyhow!("no valid pin selected for update"));
        }

        let strategy = match (opts.partial, opts.full) {
            (false, false) => UpdateStrategy::Normal,
            (false, true) => UpdateStrategy::Full,
            (true, false) => UpdateStrategy::HashesOnly,
            (true, true) => panic!("partial and full are mutually exclusive"),
        };

        let animation = Animation::new(|stderr, finished| {
            write!(stderr, "Updated {finished}/{length} pins").unwrap()
        });
        let animation = &animation;

        let update_iter = pins
            .pins
            .iter_mut()
            .filter(|(name, pin)| {
                selected_pins.contains(name)
                    || (opts.names.is_empty() && (opts.update_frozen || !pin.is_frozen()))
            })
            .map(|(name, pin)| async move {
                animation.on_pin_start(name);
                let diff = Self::update_one(name, pin, strategy).await?;
                animation.on_pin_finish(name, |stderr| write_diff(stderr, name, &diff));
                anyhow::Result::<_, anyhow::Error>::Ok((name, diff))
            });

        let mut has_diff = false;
        stream::iter(update_iter)
            .buffer_unordered(opts.max_concurrent_downloads)
            .try_for_each(|(_name, diff)| {
                has_diff |= !diff.is_empty();
                future::ready(Ok(()))
            })
            .await
            .inspect_err(|_| {
                /* Flush the status line */
                if length != 0 && stderr().is_terminal() {
                    eprintln!();
                }
            })?;

        /* Flush the status line */
        if length != 0 && stderr().is_terminal() {
            eprintln!();
        }

        if !opts.dry_run {
            if has_diff {
                self.write_pins(&pins)?;
            }
            log::info!("Update successful.");
        } else {
            log::info!("Dry run successful.");
        }

        Ok(())
    }

    async fn verify(&self, opts: &VerifyOpts) -> Result<()> {
        let mut pins = self.read_pins()?;

        let mut selected_pins = BTreeSet::new();
        for name in &opts.names {
            if !selected_pins.insert(name) {
                log::warn!("Ignoring duplicate pin: {name}")
            }
        }
        selected_pins.retain(|&name| match pins.pins.get(name) {
            Some(_) => true,
            None => {
                log::warn!("Specified pin does not exist: {name}");
                false
            },
        });

        let length = if opts.names.is_empty() {
            pins.pins.len()
        } else {
            selected_pins.len()
        };

        const STRATEGY: UpdateStrategy = UpdateStrategy::HashesOnly;

        let animation = Animation::new(|stderr, finished| {
            write!(stderr, "Verified {finished}/{length} pins").unwrap()
        });
        let animation = &animation;

        let update_iter = pins
            .pins
            .iter_mut()
            .filter(|(name, _pin)| selected_pins.contains(name) || opts.names.is_empty())
            .map(|(name, pin)| async move {
                animation.on_pin_start(name);
                let diff_result = Self::update_one(name, pin, STRATEGY).await;
                animation.on_pin_finish(name, |stderr| match &diff_result {
                    Ok(diff) => write_diff(stderr, name, diff),
                    Err(err) => {
                        writeln!(stderr, "[{name}] Failed download").unwrap();
                        writeln!(stderr, "{err:?}").unwrap();
                    },
                });
                (name, diff_result)
            });

        let (differences, failed) = stream::iter(update_iter)
            .buffer_unordered(opts.max_concurrent_downloads)
            /* Filter out empty diffs */
            .filter(|(_, diff_result)| {
                future::ready(
                    diff_result
                        .as_ref()
                        .map(|diff| !diff.is_empty())
                        .unwrap_or(true),
                )
            })
            .fold(
                (vec![], vec![]),
                |(mut differences, mut failed), (name, diff_result)| async move {
                    match diff_result {
                        Ok(_) => differences.push(name),
                        Err(_) => failed.push(name),
                    }
                    (differences, failed)
                },
            )
            .await;

        /* Flush the status line */
        if length != 0 && stderr().is_terminal() {
            eprintln!();
        }

        if differences.is_empty() && failed.is_empty() {
            log::info!("Verification passed.");
            Ok(())
        } else {
            if !differences.is_empty() {
                log::error!(
                    "The {} pins failed verification: {:?}",
                    differences.len(),
                    differences
                );
            }
            if !failed.is_empty() {
                log::error!("The {} pins failed to download: {:?}", failed.len(), failed);
            }
            anyhow::bail!("Verification failed.")
        }
    }

    fn upgrade(&self) -> Result<()> {
        if self.lock_file.is_none() {
            anyhow::ensure!(
                self.folder.exists(),
                "Could not find npins folder at {}",
                self.folder.display(),
            );

            let nix_path = self.folder.join("default.nix");
            let nix_file = DEFAULT_NIX;
            if std::fs::read_to_string(&nix_path)? == nix_file {
                log::info!("default.nix is already up to date");
            } else {
                log::info!("Replacing default.nix with an up to date version");
                std::fs::write(&nix_path, nix_file)
                    .context("Failed to create npins default.nix")?;
            }
        }

        log::info!("Upgrading lock file to the newest format version");
        let sources_json = self.folder.join("sources.json");
        let path = self.lock_file.as_ref().unwrap_or(&sources_json);
        let fh = BufReader::new(File::open(path).with_context(move || {
            format!(
                "Failed to open {}. You must initialize npins first.",
                path.display()
            )
        })?);

        let pins_raw: serde_json::Map<String, serde_json::Value> = serde_json::from_reader(fh)
            .context("lock file must be a valid JSON file with an object as top level")?;

        let pins_raw_new = versions::upgrade(pins_raw.clone(), path).context("Upgrading failed")?;
        let pins: NixPins = serde_json::from_value(pins_raw_new.clone())?;
        if pins_raw_new != serde_json::Value::Object(pins_raw) {
            log::info!(
                "Done. It is recommended to at least run `npins update --partial` afterwards."
            );
        }
        self.write_pins(&pins)
    }

    fn remove(&self, r: &RemoveOpts) -> Result<()> {
        let pins = self.read_pins()?;

        if !pins.pins.contains_key(&r.name) {
            return Err(anyhow::anyhow!("Could not find the pin '{}'", r.name));
        }

        let mut new_pins = pins.clone();
        new_pins.pins.remove(&r.name);

        self.write_pins(&new_pins)?;
        log::info!("Successfully removed pin '{}'.", r.name);
        Ok(())
    }

    async fn freeze(&self, o: &FreezeOpts) -> Result<()> {
        let mut pins = self.read_pins()?;

        for name in o.names.iter() {
            let pin = match pins.pins.get_mut(name) {
                None => return Err(anyhow::anyhow!("Couldn't find the pin {} to freeze.", name)),
                Some(pin) => pin,
            };

            pin.freeze();
            log::info!("Froze pin {}", name);
        }

        self.write_pins(&pins)?;

        Ok(())
    }

    async fn unfreeze(&self, o: &FreezeOpts) -> Result<()> {
        let mut pins = self.read_pins()?;

        for name in o.names.iter() {
            let pin = match pins.pins.get_mut(name) {
                None => return Err(anyhow::anyhow!("Couldn't find the pin {} to thaw.", name)),
                Some(pin) => pin,
            };

            pin.unfreeze();

            log::info!("Thawed pin {}", name);
        }

        self.write_pins(&pins)?;

        Ok(())
    }

    async fn import_niv(&self, o: &ImportOpts) -> Result<()> {
        let mut pins = self.read_pins()?;

        let niv: BTreeMap<String, serde_json::Value> =
            serde_json::from_reader(File::open(&o.path).context(anyhow::format_err!(
                "Could not open sources.json at '{}'",
                o.path.canonicalize().unwrap_or_else(|_| o.path.clone()).display()
            ))?)
            .context("Niv file is not a valid JSON dict")?;
        log::info!(
            "Note that all the imported entries will be updated so they won't necessarily point to the same commits as before!"
        );

        async fn import(
            name: &str,
            pin: Option<&serde_json::Value>,
            npins: &mut NixPins,
            niv: &BTreeMap<String, serde_json::Value>,
        ) -> Result<()> {
            let pin = pin
                .or_else(|| niv.get(name))
                .with_context(|| anyhow::format_err!("Pin '{name}' not found in sources.json"))?;
            anyhow::ensure!(
                !npins.pins.contains_key(name),
                "Pin '{}' exists in both files, this is a collision. Please delete the entry in one of the files.",
                name
            );

            let pin: niv::NivPin = serde_json::from_value(pin.clone())
                .context("Pin is either invalid, or we don't support it")?;
            let mut pin: Pin = pin
                .try_into()
                .context("Could not convert pin to npins format")?;
            pin.update().await.context("Failed to update the pin")?;
            pin.fetch().await.context("Failed to update the pin")?;
            npins.pins.insert(name.to_string(), pin);

            Ok(())
        }

        if let Some(name) = &o.name {
            import(name, None, &mut pins, &niv).await?;
        } else {
            for (name, pin) in niv.iter() {
                log::info!("Importing {}", name);
                if let Err(err) = import(name, Some(pin), &mut pins, &niv).await {
                    log::error!("Failed to import pin '{}'", name);
                    log::error!("{}", err);
                    err.chain()
                        .skip(1)
                        .for_each(|cause| log::error!("\t{}", cause));
                }
            }
        }

        self.write_pins(&pins)?;
        log::info!("Done.");
        Ok(())
    }

    async fn import_flake(&self, o: &ImportFlakeOpts) -> Result<()> {
        let mut pins = self.read_pins()?;

        let flake: serde_json::Value =
            serde_json::from_reader(File::open(&o.path).context(anyhow::format_err!(
                "Could not open flake.lock at '{}'",
                o.path.canonicalize().unwrap_or_else(|_| o.path.clone()).display()
            ))?)
            .context("Nix lock file is not a valid JSON object")?;
        log::info!(
            "Note that all the imported entries will be updated so they won't necessarily point to the same commits as before!"
        );

        let nodes: &serde_json::Map<String, serde_json::Value> = flake
            .get("nodes")
            .context("flake.lock missing key `nodes`")?
            .as_object()
            .context("`nodes` key does not contain an object")?;

        let root_name = flake
            .get("root")
            .context("missing `root` key")?
            .as_str()
            .context("`root` key of flake lockfile is not a string")?;
        let root = nodes
            .get(root_name)
            .context("flake.lock missing key `root`")?
            .get("inputs")
            .context("`root` key missing `inputs` key")?
            .as_object()
            .context("`root` key is not an object")?;

        let inputs: BTreeMap<String, String> = root
            .into_iter()
            .map(|(key, value)| Some((key.to_string(), value.as_str()?.to_string())))
            .collect::<Option<_>>()
            .context(format!(
                "root flake input `{root_name}` had unexpected format and could not be read"
            ))?;

        async fn import(
            name: &str,
            npins: &mut NixPins,
            nodes: &serde_json::Map<String, serde_json::Value>,
        ) -> Result<()> {
            let pin = nodes
                .get(name)
                .with_context(|| anyhow::format_err!("Pin '{name}' not found in flake.lock"))?;
            anyhow::ensure!(
                !npins.pins.contains_key(name),
                "Pin '{}' exists in both files, this is a collision. Please delete the entry in one of the files.",
                name
            );

            let pin: flake::FlakePin = serde_json::from_value(pin.clone())
                .context("Pin is either invalid, or we don't support it")?;

            if pin.is_indirect() {
                log::info!("skipping indirect input {}", name);
                return Ok(());
            }

            let mut pin: Pin = pin
                .try_to_pin()
                .await
                .context("Could not convert pin to npins format")?;

            pin.update().await?;
            pin.fetch().await.context("Failed to update the pin")?;
            npins.pins.insert(name.to_string(), pin);

            Ok(())
        }

        if let Some(name) = &o.name {
            import(
                inputs
                    .get(name)
                    .context(format!("flake input {name} not found"))?,
                &mut pins,
                nodes,
            )
            .await
            .with_context(|| format!("Importing flake input {name}"))?;
        } else {
            for (name, input_name) in inputs.iter() {
                log::info!("Importing {}", name);
                if let Err(err) = import(input_name, &mut pins, nodes).await {
                    log::error!("Failed to import pin '{}'", name);
                    log::error!("{}", err);
                    err.chain()
                        .skip(1)
                        .for_each(|cause| log::error!("\t{}", cause));
                }
            }
        }

        self.write_pins(&pins)?;
        log::info!("Done.");
        Ok(())
    }

    async fn get_path(&self, o: &GetPathOpts) -> Result<()> {
        /* Although redundant, we still parse the lock file here for better error messages */
        self.read_pins()?;

        let path = self
            .lock_file
            .to_owned()
            .unwrap_or(self.folder.join("sources.json"));
        let out_path = nix::nix_eval_pin(&path, &o.name)
            .await
            .context("Could not evaluate pin")?;
        /* note(piegames): HMU if you ever find yourself using npins on Windows */
        use std::os::unix::ffi::OsStrExt;
        std::io::stdout()
            .write_all(out_path.as_path().as_os_str().as_bytes())
            .unwrap();
        Ok(())
    }

    pub async fn run(&self) -> Result<()> {
        if self.lock_file.is_some() && &*self.folder != std::path::Path::new("npins") {
            anyhow::bail!(
                "If --lock-file is set, --directory will be ignored and thus should not be set to a non-default value (which is \"npins\")"
            );
        }
        match &self.command {
            Command::Init(o) => self.init(o).await?,
            Command::Show(o) => self.show(o)?,
            Command::Add(a) => self.add(a).await?,
            Command::Update(o) => self.update(o).await?,
            Command::Verify(o) => self.verify(o).await?,
            Command::Upgrade => self.upgrade()?,
            Command::Remove(r) => self.remove(r)?,
            Command::ImportNiv(o) => self.import_niv(o).await?,
            Command::ImportFlake(o) => self.import_flake(o).await?,
            Command::Freeze(o) => self.freeze(o).await?,
            Command::Unfreeze(o) => self.unfreeze(o).await?,
            Command::GetPath(o) => self.get_path(o).await?,
        };

        Ok(())
    }
}

/// Helper struct for the CLI animation used by `npins update`
struct Animation<'a, F> {
    in_progress: RefCell<BTreeSet<&'a str>>,
    finished: Cell<i32>,
    write_bottom_line: F,
}

impl<'a, F: for<'b> Fn(&'b mut std::io::StderrLock, i32)> Animation<'a, F> {
    pub fn new(write_bottom_line: F) -> Self {
        Self {
            in_progress: Default::default(),
            finished: Default::default(),
            write_bottom_line,
        }
    }

    /// Update the set of in_progress pins, redrawing it in the process.
    /// The closure passed may also print information, which will then be displayed above the
    /// list of in-progress pins.
    fn update_in_progress(
        &'a self,
        stderr: &mut std::io::StderrLock,
        updater: impl for<'b> FnOnce(&'b mut BTreeSet<&'a str>, &mut std::io::StderrLock),
    ) {
        let mut in_progress = self.in_progress.borrow_mut();

        updater(&mut in_progress, stderr);
        for n in in_progress.iter() {
            stderr.queue(Print(n.dark_yellow())).unwrap();
            stderr.write_all(b"\n").unwrap();
        }

        (self.write_bottom_line)(stderr, self.finished.get());
    }

    /// To be called every time a pin starts processing
    pub fn on_pin_start(&'a self, name: &'a str) {
        /* No animations outside a terminal */
        if !stderr().is_terminal() {
            return;
        }

        let mut stderr = stderr().lock();
        self.update_in_progress(&mut stderr, |in_progress, stderr| {
            if !in_progress.is_empty() {
                crossterm::queue!(
                    stderr,
                    MoveToPreviousLine(in_progress.len() as u16),
                    Clear(ClearType::FromCursorDown)
                )
                .unwrap();
            }

            in_progress.insert(name);
        });
        stderr.flush().unwrap();
    }

    /// Removes the pin from being progressed and provide writer to print result
    pub fn on_pin_finish(
        &'a self,
        name: &str,
        result_writer: impl FnOnce(&mut std::io::StderrLock),
    ) {
        let mut stderr = stderr().lock();

        if !stderr.is_terminal() {
            return;
        }

        self.finished.set(self.finished.get() + 1);
        self.update_in_progress(&mut stderr, |in_progress, stderr| {
            crossterm::queue!(
                stderr,
                MoveToPreviousLine(in_progress.len() as u16),
                Clear(ClearType::FromCursorDown)
            )
            .unwrap();

            result_writer(stderr);

            in_progress.remove(name);
        });
        stderr.flush().unwrap();
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Opts::parse();

    env_logger::builder()
        .filter_level(if opts.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .format_timestamp(None)
        .format_target(false)
        .init();

    opts.run().await?;
    Ok(())
}
