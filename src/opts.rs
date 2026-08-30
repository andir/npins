//! Contains the entire command line interface for npins
//! including the parsing and completions for it, ordered
//! roughly from top to bottom twice for both the types
//! and the implementations on them

use anyhow::Context;
use bpaf::{OptionParser, Parser, ShellComp, construct, long, positional, pure, short};
use core::{cell::OnceCell, convert::Infallible, fmt::Display};
use libnpins::channel;
use std::{
    path::{Path, PathBuf},
    rc::Rc,
};
use url::Url;

#[derive(Debug)]
pub struct Opts {
    pub mode: SourceMode,
    pub verbose: bool,

    pub command: Command,
}

/// Type of file structures we support operating on
#[derive(Debug)]
pub enum SourceMode {
    /// Represents a initalized npins which has our `default.nix` and `sources.json`
    Directory {
        sources: PathBuf,
        default_nix: PathBuf,
        directory: PathBuf,
    },
    /// Represents the path to a npins lockfile with nothing else that is managed by us
    Lockfile(PathBuf),
}

#[derive(Debug)]
pub enum Command {
    Init(InitOpts),
    // Boxing AddOpts as it is by far our largest structure, reduces
    // memory requirements for smaller devices (even if marginal)
    Add(Box<AddOpts>),
    Show(ShowOpts),
    Update(UpdateOpts),
    Verify(VerifyOpts),
    Upgrade,
    Remove(RemoveOpts),
    ImportNiv(ImportNivOpts),
    ImportLon(ImportLonOpts),
    ImportFlake(ImportFlakeOpts),
    Freeze(FreezeOpts),
    Unfreeze(FreezeOpts),
    GetPath(GetPathOpts),
}

#[derive(Debug)]
pub struct InitOpts {
    pub bare: bool,
}

#[derive(Debug)]
pub struct AddOpts {
    pub name: Option<String>,
    pub frozen: bool,
    pub dry_run: bool,

    pub command: AddCommands,
}

#[derive(Debug)]
pub struct ShowOpts {
    pub plain: bool,
    pub exclude: bool,

    pub names: Vec<String>,
}

#[derive(Debug)]
pub struct UpdateOpts {
    pub strategy: UpdateStrategy,
    pub dry_run: bool,
    pub update_frozen: bool,
    pub max_concurrent_downloads: usize,

    pub names: Vec<String>,
}

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

#[derive(Debug)]
pub struct VerifyOpts {
    pub max_concurrent_downloads: usize,

    pub names: Vec<String>,
}

#[derive(Debug)]
pub struct RemoveOpts {
    pub names: Vec<String>,
}

#[derive(Debug)]
pub struct ImportNivOpts {
    pub name: Option<String>,

    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ImportLonOpts {
    pub name: Option<String>,

    pub path: PathBuf,
}

#[derive(Debug)]
pub struct ImportFlakeOpts {
    pub name: Option<String>,

    pub path: PathBuf,
}

#[derive(Debug)]
pub struct FreezeOpts {
    pub names: Vec<String>,
}

#[derive(Debug)]
pub struct GetPathOpts {
    pub name: String,
}

#[derive(Debug)]
pub enum AddCommands {
    Channel(ChannelAddOpts),
    GitHub(GitHubAddOpts),
    Forgejo(ForgejoAddOpts),
    GitLab(GitLabAddOpts),
    Git(GitAddOpts),
    PyPi(PyPiAddOpts),
    Container(ContainerAddOpts),
    Tarball(UrlAddOpts),
    Url(UrlAddOpts),
}

#[derive(Debug)]
pub struct ChannelAddOpts {
    pub channel_name: String,
    pub artifact: String,
}

#[derive(Debug)]
pub struct GitHubAddOpts {
    pub more: GenericGitAddOpts,

    pub owner: String,
    pub repository: String,
}

#[derive(Debug)]
pub struct ForgejoAddOpts {
    pub more: GenericGitAddOpts,

    pub server: String,
    pub owner: String,
    pub repository: String,
}

#[derive(Debug)]
pub struct GitLabAddOpts {
    pub more: GenericGitAddOpts,

    pub server: url::Url,
    pub private_token: Option<String>,

    pub repo_path: Vec<String>,
}

#[derive(Debug)]
pub struct GitAddOpts {
    pub more: GenericGitAddOpts,

    pub forge: GitForgeOpts,

    pub url: Url,
}

#[derive(Debug)]
pub struct PyPiAddOpts {
    pub at: Option<String>,

    // TODO: `at` and `version_upper_bound` were previously mutually exclusive, why?
    pub version_upper_bound: Option<String>,

    pub package_name: String,
}

#[derive(Debug)]
pub struct ContainerAddOpts {
    pub arch: Option<String>,

    pub image_name: String,
    pub image_tag: String,
}

#[derive(Debug)]
pub struct UrlAddOpts {
    pub mutable: bool,

    pub url: Url,
}

#[derive(Debug)]
pub struct GenericGitAddOpts {
    pub at: Option<String>,
    pub selected: GitAddSelection,
    pub submodules: bool,
}

#[derive(Debug)]
pub enum GitAddSelection {
    Branch {
        branch: String,
    },
    Release {
        pre_releases: bool,
        // TODO: `at` and `version_upper_bound` were previously mutually exclusive, why?
        version_upper_bound: Option<String>,
        release_prefix: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Default)]
pub enum GitForgeOpts {
    None,
    #[default]
    Auto,
    Gitlab,
    Github,
    Forgejo,
}

/// Shared path of the lockfile we are parsing
/// Should only be used inside of completions
type CompletionLockfile = Rc<OnceCell<Box<Path>>>;

impl Opts {
    pub fn parser() -> OptionParser<Self> {
        // Look... I know this is very cursed and all but we need a way to share
        // the lockfile to completions and bpaf docs explicitly state that parsers
        // cannot pass information to each other so you have to do this hack.
        // https://docs.rs/bpaf/0.9.26/bpaf/_documentation/_0_intro/index.html#flexibility
        let lockfile_shared = CompletionLockfile::default();
        let lockfile_set = lockfile_shared.clone();

        let mode = SourceMode::parser().map(move |m| {
            let _ = lockfile_set.set(m.lockfile().into());
            m
        });

        let verbose = short('v')
            .long("verbose")
            .help("Prints debug messages")
            .switch();

        let command = Command::parser(lockfile_shared);

        construct!(Opts {
            mode,
            verbose,
            command
        })
        .to_options()
        .descr("Simple and convenient dependency pinning for Nix. All options are available in subcommands.")
        .fallback_to_usage()
        .version(env!("CARGO_PKG_VERSION"))
    }
}

impl SourceMode {
    pub fn parser() -> impl Parser<Self> {
        let lock_file = long("lock-file")
            .help("Specifies the lockfile and operates only on it (lockfile mode)")
            .argument::<PathBuf>("FILE")
            .complete_shell(ShellComp::File { mask: None })
            .map(Self::Lockfile);

        let directory = short('d')
            .long("directory")
            .help("Specifies base folder for sources.json and the boilerplate default.nix")
            .env("NPINS_DIRECTORY")
            .argument::<PathBuf>("FOLDER")
            .complete_shell(ShellComp::Dir { mask: None })
            .fallback_with(|| Result::<_, Infallible>::Ok(PathBuf::from("npins")))
            .format_fallback(|path, f| path.display().fmt(f))
            .map(Self::from_directory);

        construct!([directory, lock_file])
    }

    pub fn from_directory(directory: PathBuf) -> Self {
        Self::Directory {
            sources: directory.join("sources.json"),
            default_nix: directory.join("default.nix"),
            directory,
        }
    }

    pub fn lockfile(&self) -> &Path {
        let (Self::Directory { sources, .. } | Self::Lockfile(sources)) = self;
        sources
    }
}

impl Command {
    pub fn parser(lockfile: CompletionLockfile) -> impl Parser<Self> {
        let init = construct!(InitOpts {
            bare(long("bare").switch().help("Don't add an initial `nixpkgs` entry"))
        })
            .map(Self::Init)
            .to_options()
            .descr("Intializes the npins directory. Running this multiple times will restore/upgrade the `default.nix` and never touch your sources.json")
            .fallback_to_usage()
            .command("init");

        let add = AddOpts::parser(lockfile.clone())
            .map(Box::new)
            .map(Self::Add);

        let plain = long("plain")
            .short('b')
            .switch()
            .help("Prints only pin names");
        let exclude = long("exclude")
            .short('e')
            .switch()
            .help("Prints all the pins not specified");
        let names = positional::<String>("NAMES")
            .help("Names of the pins to show")
            .many()
            .complete(complete_pins(lockfile.clone()));
        let show = construct!(ShowOpts {
            plain,
            exclude,
            names
        })
        .map(Self::Show)
        .to_options()
        .descr("Lists the current pin entries")
        .fallback_to_usage()
        .command("show");

        let strategy = UpdateStrategy::parser();
        let max_concurrent_downloads = long("max-concurrent-downloads")
            .help("Maximum number of simultaneous downloads")
            .argument::<usize>("NUM")
            .fallback(5);
        let dry_run = long("dry-run")
            .short('n')
            .help("Print the diff, but don't write back the changes")
            .switch();
        let update_frozen = long("frozen")
            .help("Allow updating frozen pins, which would otherwise be ignored")
            .switch();
        let names = positional::<String>("NAMES")
            .help("Updates only the specified pins")
            .many()
            .complete(complete_pins(lockfile.clone()));
        let update = construct!(UpdateOpts {
            strategy,
            dry_run,
            update_frozen,
            max_concurrent_downloads,
            names
        })
        .map(Self::Update)
        .to_options()
        .descr("Updates all or the given pins to the latest version")
        .fallback_to_usage()
        .command("update");

        let max_concurrent_downloads = long("max-concurrent-downloads")
            .help("Maximum number of simultaneous downloads")
            .argument::<usize>("NUM")
            .fallback(5);
        let names = positional::<String>("NAMES")
            .help("Verifies only the specified pins")
            .many()
            .complete(complete_pins(lockfile.clone()));
        let verify = construct!(VerifyOpts {
            max_concurrent_downloads,
            names
        })
        .map(Self::Verify)
        .to_options()
        .descr("Verifies that all or the given pins still have correct hashes. This is like `update --partial --dry-run` and then checking that the diff is empty")
        .fallback_to_usage()
        .command("verify");

        let upgrade = pure(())
            .to_options()
            .descr("Upgrade the sources.json and default.nix to the latest format version. This may occasionally break Nix evaluation!")
            .fallback_to_usage()
            .command("upgrade")
            .map(|()| Self::Upgrade);

        let names = positional::<String>("NAMES")
            .help("Names of the pins to remove")
            .some("Need at least one pin entry to remove")
            .complete(complete_pins(lockfile.clone()));
        let remove = construct!(RemoveOpts { names })
            .map(Self::Remove)
            .to_options()
            .descr("Remove pin entries")
            .fallback_to_usage()
            .command("remove");

        let name = long("name")
            .short('n')
            .help("Only import one entry from Niv")
            .argument::<String>("NAME")
            .optional();
        let path = positional::<PathBuf>("FILE")
            .complete_shell(ShellComp::File { mask: None })
            .fallback_with(|| Result::<_, Infallible>::Ok(PathBuf::from("nix/sources.json")))
            .format_fallback(|path, f| path.display().fmt(f));
        let import_niv = construct!(ImportNivOpts { name, path })
            .map(Self::ImportNiv)
            .to_options()
            .descr("Try to import entries from Niv")
            .fallback_to_usage()
            .command("import-niv");

        let name = long("name")
            .short('n')
            .help("Only import one entry from Lon")
            .argument::<String>("NAME")
            .optional();
        let path = positional::<PathBuf>("FILE")
            .complete_shell(ShellComp::File { mask: None })
            .fallback_with(|| Result::<_, Infallible>::Ok(PathBuf::from("lon.lock")))
            .format_fallback(|path, f| path.display().fmt(f));
        let import_lon = construct!(ImportLonOpts { name, path })
            .map(Self::ImportLon)
            .to_options()
            .descr("Try to import entries from Lon")
            .fallback_to_usage()
            .command("import-lon");

        let name = long("name")
            .short('n')
            .help("Only import one entry from the flake")
            .argument::<String>("NAME")
            .optional();
        let path = positional::<PathBuf>("FILE")
            .complete_shell(ShellComp::File { mask: None })
            .fallback_with(|| Result::<_, Infallible>::Ok(PathBuf::from("flake.lock")))
            .format_fallback(|path, f| path.display().fmt(f));
        let import_flake = construct!(ImportFlakeOpts { name, path })
            .map(Self::ImportFlake)
            .to_options()
            .descr("Try to import entries from flake.lock")
            .fallback_to_usage()
            .command("import-flake");

        let names = positional::<String>("NAMES")
            .help("Names of the pins to freeze")
            .some("Need at least one pin entry to freeze")
            .complete(complete_frozen(lockfile.clone(), false));
        let freeze = construct!(FreezeOpts { names })
            .map(Self::Freeze)
            .to_options()
            .descr("Freezes a pin entry, preventing it from being changed during an update")
            .fallback_to_usage()
            .command("freeze");

        let names = positional::<String>("NAMES")
            .help("Names of the pins to unfreeze")
            .some("Need at least one pin entry to unfreeze")
            .complete(complete_frozen(lockfile.clone(), true));
        let unfreeze = construct!(FreezeOpts { names })
            .map(Self::Unfreeze)
            .to_options()
            .descr(
                "Thaws a pin entry, allowing it to be changed during an update like a normal pin",
            )
            .fallback_to_usage()
            .command("unfreeze");

        let name = positional::<String>("NAME")
            .help("Name of the pin")
            .complete(complete_pin(lockfile.clone()));
        let get_path = construct!(GetPathOpts { name })
            .map(Self::GetPath)
            .to_options()
            .descr("Evaluates the store path to a pin, fetching it if necessary. Don't forget to add a GC root")
            .fallback_to_usage()
            .command("get-path");

        construct!([
            init,
            add,
            show,
            update,
            verify,
            upgrade,
            remove,
            import_niv,
            import_lon,
            import_flake,
            freeze,
            unfreeze,
            get_path
        ])
    }
}

impl UpdateStrategy {
    pub fn parser() -> impl Parser<Self> {
        let full = long("full").short('f').help("Re-fetch hashes even if the version hasn't changed.\nUseful to make sure the derivations are in the Nix store.").req_flag(Self::Full);
        let partial = long("partial")
            .short('p')
            .help("Don't update versions, only re-fetch hashes")
            .req_flag(Self::HashesOnly);

        construct!([full, partial]).fallback(Self::Normal)
    }
}

impl AddOpts {
    pub fn parser(lockfile: CompletionLockfile) -> impl Parser<Self> {
        let name = long("name")
            .argument::<String>("NAME")
            .help("Add the pin with a custom name. If a pin with that name already exists, it will be overwritten")
            .complete(complete_pin(lockfile))
            .optional();

        let frozen = long("frozen").switch().help(
            "Add the pin as frozen, meaning that it will be ignored by `npins update` by default.",
        );

        let dry_run = long("dry-run")
            .short('n')
            .switch()
            .help("Don't actually apply the changes");

        let command = AddCommands::parser();

        construct!(AddOpts {
            name,
            frozen,
            dry_run,
            command
        })
        .to_options()
        .descr("Adds a new pin entry.")
        .fallback_to_usage()
        .command("add")
    }
}

impl AddCommands {
    pub fn parser() -> impl Parser<Self> {
        let channel_name = positional("CHANNEL").help("The name of the channel to pin");
        let artifact = positional("ARTIFACT").help("Select a specific artifact from the channel, defaults to Nixpkgs if omitted.
Find valid artifact names on <https://nixos.org/download/> or `nix-shell -p awscli2 --run 'aws s3 ls nix-channels/$CHANNEL'` (unfortunately requires an AWS account).
Common values: `latest-nixos-graphical-x86_64-linux.iso`, `latest-nixos-minimal-aarch64-linux.iso`").fallback_with(|| Result::<_, Infallible>::Ok(String::from(channel::NIXPKGS_ARTIFACT))).display_fallback();
        let channel = construct!(ChannelAddOpts {
            channel_name,
            artifact
        })
        .map(Self::Channel)
        .to_options()
        .descr("Track a Nix channel")
        .fallback_to_usage()
        .command("channel");

        let more = GenericGitAddOpts::parser();
        let owner = positional("OWNER");
        let repository = positional("REPOSITORY");
        let github = construct!(GitHubAddOpts {
            more,
            owner,
            repository
        })
        .map(Self::GitHub)
        .to_options()
        .descr("Track a GitHub repository")
        .fallback_to_usage()
        .command("github");

        let more = GenericGitAddOpts::parser();
        let server = positional("SERVER");
        let owner = positional("OWNER");
        let repository = positional("REPOSITORY");
        let forgejo = construct!(ForgejoAddOpts {
            more,
            server,
            owner,
            repository
        })
        .map(Self::Forgejo)
        .to_options()
        .descr("Track a Forgejo repository")
        .fallback_to_usage()
        .command("forgejo");

        let more = GenericGitAddOpts::parser();
        let server = long("server")
            .argument("URL")
            .help("Use a specific GitLab instance")
            .fallback_with(|| Url::parse("https://gitlab.com/"))
            .display_fallback();
        let private_token = long("private-token")
            .argument("TOKEN")
            .help("Use a private token to access the repository.")
            .optional();
        let repo_path = positional::<String>("REPO PATH")
            .help(r#"Usually just `"owner" "repository"`, but GitLab allows arbitrary folder-like structures."#)
            .many()
            .guard(|r| r.len() >= 2, "Repository path must be contain at least 2 segments");
        let gitlab = construct!(GitLabAddOpts {
            more,
            server,
            private_token,
            repo_path,
        })
        .map(Self::GitLab)
        .to_options()
        .descr("Track a GitLab repository")
        .fallback_to_usage()
        .command("gitlab");

        let more = GenericGitAddOpts::parser();
        let forge = GitForgeOpts::parser();
        let url = positional::<String>("URL")
            .help("The git remote URL. For example <https://github.com/andir/ate.git>")
            .parse(|x| {
                Url::parse(&x)
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
                .context("Failed to parse repository URL")
            });
        let git = construct!(GitAddOpts { more, forge, url })
            .map(Self::Git)
            .to_options()
            .descr("Track a git repository")
            .fallback_to_usage()
            .command("git");

        let at = long("at")
            .argument("VERSION")
            .help("Use a specific release instead of the latest.")
            .optional();
        let version_upper_bound = long("upper-bound")
            .argument("VERSION")
            .help(r#"Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions."#)
            .optional();
        let package_name = positional("PACKAGE").help("Name of the package at PyPi.org");
        let pypi = construct!(PyPiAddOpts {
            at,
            version_upper_bound,
            package_name
        })
        .map(Self::PyPi)
        .to_options()
        .descr("Track a package on PyPi")
        .fallback_to_usage()
        .command("pypi");

        let arch = long("arch").argument("ARCH").optional();
        let image_name = positional("NAME").help("Name of the image");
        let image_tag = positional("TAG").help("Tag of the image");
        let container = construct!(ContainerAddOpts {
            arch,
            image_name,
            image_tag
        })
        .map(Self::Container)
        .to_options()
        .descr("Track an OCI container")
        .fallback_to_usage()
        .command("container");

        let mutable = long("mutable").switch().help("Treat this URL as mutable, and assume it will redirect to an immutable version of the content to be pinned. For example, a HEAD URL redirecting to the currently latest commit");
        let url = positional("URL").help("Tarball URL");
        let tarball = construct!(UrlAddOpts {
            mutable,
            url
        })
        .map(Self::Tarball)
        .to_options()
        .descr("Track a URL")
        .header(r#"This can be either a static URL that never changes its contents or a "mutable" URL that redirects to an immutable snapshot."#)
        .fallback_to_usage()
        .command("tarball");

        let mutable = long("mutable").switch().help("Treat this URL as mutable, and assume it will redirect to an immutable version of the content to be pinned. For example, a HEAD URL redirecting to the currently latest commit");
        let url = positional("URL").help("URL to pin");
        let url = construct!(UrlAddOpts {
            mutable,
            url
        })
        .map(Self::Url)
        .to_options()
        .descr("Track a URL")
        .header(r#"This can be either a static URL that never changes its contents or a "mutable" URL that redirects to an immutable snapshot."#)
        .fallback_to_usage()
        .command("url");

        construct!([
            channel, github, forgejo, gitlab, git, pypi, container, tarball, url
        ])
    }
}

impl GenericGitAddOpts {
    pub fn parser() -> impl Parser<Self> {
        let at = long("at")
            .argument::<String>("TAG OR REV")
            .help("Use a specific commit/release instead of the latest.\nThis may be a tag name, or a git revision when --branch is set.")
            .optional();

        let submodules = long("submodules").switch().help("Also fetch submodules");

        let selected = GitAddSelection::parser();

        construct!(Self {
            at,
            submodules,
            selected
        })
    }
}

impl GitAddSelection {
    pub fn parser() -> impl Parser<Self> {
        let branch = short('b')
            .long("branch")
            .argument::<String>("BRANCH")
            .help("Track a branch instead of a release");

        let branch = construct!(Self::Branch { branch });

        let pre_releases = long("pre-releases")
            .switch()
            .help("Also track pre-releases.\nConflicts with the --branch option.");

        let version_upper_bound = long("upper-bound")
            .argument::<String>("VERSION")
            .help(r#"Bound the version resolution. For example, setting this to "2" will restrict updates to 1.X versions."#)
            .optional();

        let release_prefix = long("release-prefix")
            .argument::<String>("VERSION")
            .help(r#"Optional prefix required for each release name / tag. For example, setting this to "release/" will only consider those that start with that string."#)
            .optional();

        let release = construct!(Self::Release {
            pre_releases,
            version_upper_bound,
            release_prefix
        });

        construct!([branch, release])
    }
}

impl GitForgeOpts {
    pub fn parser() -> impl Parser<Self> {
        long("forge")
            .argument::<String>("FORGE")
            .complete(|_| {
                Vec::from([
                    (
                        "none",
                        Some("A generic git pin, with no further information"),
                    ),
                    (
                        "auto",
                        Some("Try to determine the Forge from the given url, potentially by probing the server"),
                    ),
                    (
                        "gitlab",
                        Some("A Gitlab forge, e.g. gitlab.com"),
                    ),
                    (
                        "github",
                        Some("A Github forge, i.e. github.com"),
                    ),
                    (
                        "forgejo",
                        Some("A Forgejo forge, e.g. codeberg.org"),
                    ),
                ])
            })
            .parse(|f| {
                Ok(match f.as_str() {
                    "none" => Self::None,
                    "auto" => Self::Auto,
                    "gitlab" => Self::Gitlab,
                    "github" => Self::Github,
                    "forgejo" => Self::Forgejo,
                    x => return Err(format!("invalid value '{x}' for forge")),
                })
            })
            .fallback(GitForgeOpts::Auto)
            .display_fallback()
    }
}

impl Display for GitForgeOpts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::None => "none",
            Self::Auto => "auto",
            Self::Gitlab => "gitlab",
            Self::Github => "github",
            Self::Forgejo => "forgejo",
        })
    }
}

fn complete_pin(lockfile: CompletionLockfile) -> impl Fn(&String) -> Vec<(String, Option<String>)> {
    move |incomplete| {
        let Some(Ok(pins)) = lockfile.get().map(AsRef::as_ref).map(crate::read_pins) else {
            return Vec::new();
        };

        pins.pins
            .into_keys()
            .filter(|name| name.starts_with(incomplete))
            .map(|name| (name, None))
            .collect()
    }
}

fn complete_pins(
    lockfile: CompletionLockfile,
) -> impl Fn(&Vec<String>) -> Vec<(String, Option<String>)> {
    move |v| {
        let Some(Ok(mut pins)) = lockfile.get().map(AsRef::as_ref).map(crate::read_pins) else {
            return Vec::new();
        };

        // Last element could contain a pin name which we want the user
        // to know exists and not autocomplete to one longer then it.
        // Think of a case like lix and lix-module, if I type lix and
        // then autocomplete if we didn't do this we'd get only lix-module
        let (incomplete, finished) = v.split_last().unzip();
        for n in finished.unwrap_or(&[]) {
            pins.pins.remove(n);
        }

        let incomplete = incomplete.map(String::as_str).unwrap_or("");
        pins.pins
            .into_keys()
            .filter(|name| name.starts_with(incomplete))
            .map(|name| (name, None))
            .collect()
    }
}

fn complete_frozen(
    lockfile: CompletionLockfile,
    is_frozen: bool,
) -> impl Fn(&Vec<String>) -> Vec<(String, Option<String>)> {
    move |v| {
        let Some(Ok(mut pins)) = lockfile.get().map(AsRef::as_ref).map(crate::read_pins) else {
            return Vec::new();
        };

        // Last element could contain a pin name which we want the user
        // to know exists and not autocomplete to one longer then it.
        // Think of a case like lix and lix-module, if I type lix and
        // then autocomplete if we didn't do this we'd get only lix-module
        let (incomplete, finished) = v.split_last().unzip();
        for n in finished.unwrap_or(&[]) {
            pins.pins.remove(n);
        }

        let incomplete = incomplete.map(String::as_str).unwrap_or("");
        pins.pins
            .into_iter()
            .filter(|(_, p)| p.is_frozen() == is_frozen)
            .filter(|(name, _)| name.starts_with(incomplete))
            .map(|(name, _)| (name, None))
            .collect()
    }
}

#[test]
fn check_invariants() {
    Opts::parser().check_invariants(true)
}
