use clap::{Parser, Subcommand, ValueEnum, ValueHint};
use std::path::PathBuf;
use url::Url;

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

#[derive(Debug, Parser)]
pub struct ChannelAddOpts {
    #[arg(value_hint = ValueHint::Other)]
    pub channel_name: String,
}

#[derive(Debug, Parser)]
pub struct GenericGitAddOpts {
    /// Track a branch instead of a release
    #[arg(short, long, value_hint = ValueHint::Other)]
    pub branch: Option<String>,

    /// Use a specific commit/release instead of the latest.
    /// This may be a tag name, or a git revision when --branch is set.
    #[arg(long, value_name = "tag or rev", value_hint = ValueHint::Other)]
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
        conflicts_with_all = &["branch", "at"],
        value_hint = ValueHint::Other
    )]
    pub version_upper_bound: Option<String>,

    /// Optional prefix required for each release name / tag. For
    /// example, setting this to "release/" will only consider those
    /// that start with that string.
    #[arg(long = "release-prefix", value_hint = ValueHint::Other)]
    pub release_prefix: Option<String>,

    /// Also fetch submodules
    #[arg(long)]
    pub submodules: bool,
}

#[derive(Debug, Parser)]
pub struct GitHubAddOpts {
    #[arg(value_hint = ValueHint::Other)]
    pub owner: String,
    #[arg(value_hint = ValueHint::Other)]
    pub repository: String,

    #[command(flatten)]
    pub more: GenericGitAddOpts,
}

#[derive(Debug, Parser)]
pub struct ForgejoAddOpts {
    #[arg(value_hint = ValueHint::Url)]
    pub server: String,
    #[arg(value_hint = ValueHint::Other)]
    pub owner: String,
    #[arg(value_hint = ValueHint::Other)]
    pub repository: String,

    #[command(flatten)]
    pub more: GenericGitAddOpts,
}

#[derive(Debug, Parser)]
pub struct GitLabAddOpts {
    /// Usually just `"owner" "repository"`, but GitLab allows arbitrary folder-like structures.
    // TODO set min number of values to 2 again
    #[arg(required = true, value_hint = ValueHint::Other)]
    pub repo_path: Vec<String>,

    #[arg(
        long,
        default_value = "https://gitlab.com/",
        help = "Use a self-hosted GitLab instance instead",
        value_name = "url",
        value_hint = ValueHint::Url
    )]
    pub server: url::Url,

    #[arg(
        long,
        help = "Use a private token to access the repository.",
        value_name = "token",
        value_hint = ValueHint::Other
    )]
    pub private_token: Option<String>,

    #[command(flatten)]
    pub more: GenericGitAddOpts,
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
    #[arg(value_hint = ValueHint::Url)]
    pub url: String,

    #[arg(long, value_enum, default_value = "auto")]
    pub forge: GitForgeOpts,

    #[command(flatten)]
    pub more: GenericGitAddOpts,
}

#[derive(Debug, Parser)]
pub struct PyPiAddOpts {
    /// Name of the package at PyPi.org
    #[arg(value_hint = ValueHint::Other)]
    pub package_name: String,

    /// Use a specific release instead of the latest.
    #[arg(long, value_name = "version", value_hint = ValueHint::Other)]
    pub at: Option<String>,

    /// Bound the version resolution. For example, setting this to "2" will
    /// restrict updates to 1.X versions. Conflicts with the --branch option.
    #[arg(long = "upper-bound", value_name = "version", conflicts_with = "at", value_hint = ValueHint::Other)]
    pub version_upper_bound: Option<String>,
}

#[derive(Debug, Parser)]
pub struct ContainerAddOpts {
    #[arg(value_hint = ValueHint::Other)]
    pub image_name: String,
    #[arg(value_hint = ValueHint::Other)]
    pub image_tag: String,
}

#[derive(Debug, Parser)]
pub struct TarballAddOpts {
    /// Tarball URL
    #[arg(value_hint = ValueHint::Url)]
    pub url: Url,
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
    #[arg(long, global = true, value_hint = ValueHint::Other)]
    pub name: Option<String>,
    /// Add the pin as frozen, meaning that it will be ignored by `npins update` by default.
    #[arg(long, global = true)]
    pub frozen: bool,
    /// Don't actually apply the changes
    #[arg(short = 'n', long)]
    pub dry_run: bool,
    #[command(subcommand)]
    pub command: AddCommands,
}

#[derive(Debug, Parser)]
pub struct ShowOpts {
    /// Names of the pins to show
    #[arg(value_hint = ValueHint::Other)]
    pub names: Vec<String>,
    /// Prints only pin names
    #[arg(short = 'p', long)]
    pub plain: bool,
    /// Invert [NAMES] to exclude specified pins
    #[arg(short = 'e', long)]
    pub exclude: bool,
}

#[derive(Debug, Parser)]
pub struct RemoveOpts {
    // Names of the pins to remove
    #[arg(value_hint = ValueHint::Other)]
    pub names: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct UpdateOpts {
    /// Updates only the specified pins.
    #[arg(value_hint = ValueHint::Other)]
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
    #[structopt(default_value = "5", long, value_hint = ValueHint::Other)]
    pub max_concurrent_downloads: usize,
}

#[derive(Debug, Parser)]
pub struct VerifyOpts {
    /// Verifies only the specified pins.
    #[arg(value_hint = ValueHint::Other)]
    pub names: Vec<String>,
    /// Maximum number of simultaneous downloads
    #[structopt(default_value = "5", long, value_hint = ValueHint::Other)]
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
    #[arg(default_value = "nix/sources.json", value_hint = ValueHint::FilePath)]
    pub path: PathBuf,
    /// Only import one entry from Niv
    #[arg(short, long, value_hint = ValueHint::Other)]
    pub name: Option<String>,
}

#[derive(Debug, Parser)]
pub struct ImportFlakeOpts {
    #[arg(default_value = "flake.lock", value_hint = ValueHint::FilePath)]
    pub path: PathBuf,
    /// Only import one entry from the flake
    #[arg(short, long, value_hint = ValueHint::Other)]
    pub name: Option<String>,
}

#[derive(Debug, Parser)]
pub struct FreezeOpts {
    /// Names of the pin(s)
    #[structopt(required = true, value_hint = ValueHint::Other)]
    pub names: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct GetPathOpts {
    /// Name of the pin
    #[structopt(required = true, value_hint = ValueHint::Other)]
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
        env = "NPINS_DIRECTORY",
        value_hint = ValueHint::DirPath
    )]
    pub folder: std::path::PathBuf,

    /// Specifies the path to the sources.json and activates lockfile mode.
    /// In lockfile mode, no default.nix will be generated and --directory will be ignored.
    #[arg(long, value_hint = ValueHint::FilePath)]
    pub lock_file: Option<std::path::PathBuf>,

    /// Print debug messages.
    #[arg(global = true, short = 'v', long = "verbose")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}
