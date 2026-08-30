//! The main CLI application

use anyhow::{Context, Result};
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
    path::Path,
};
use url::{ParseError, Url};

use crate::opts::*;
use libnpins::*;

mod opts;

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

impl ChannelAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        Ok((
            Some(if self.artifact == channel::NIXPKGS_ARTIFACT {
                self.channel_name.clone()
            } else {
                format!("{}-{}", self.channel_name, self.artifact)
            }),
            channel::Pin {
                name: self.channel_name.clone(),
                artifact: self.artifact.clone(),
            }
            .into(),
        ))
    }
}

impl GenericGitAddOpts {
    fn add(&self, repository: git::Repository) -> Result<Pin> {
        Ok(match &self.selected {
            GitAddSelection::Branch { branch } => {
                let pin = git::GitPin::new(repository, branch.clone(), self.submodules);
                let version = self
                    .at
                    .as_ref()
                    .map(|at| git::GitRevision::new(at.clone()))
                    .transpose()?;
                (pin, version).into()
            },
            GitAddSelection::Release {
                pre_releases,
                version_upper_bound,
                release_prefix,
            } => {
                let pin = git::GitReleasePin::new(
                    repository,
                    *pre_releases,
                    version_upper_bound.clone(),
                    release_prefix.clone(),
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

impl GitHubAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        let repository = git::Repository::github(&self.owner, &self.repository);

        Ok((Some(self.repository.clone()), self.more.add(repository)?))
    }
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

impl GitAddOpts {
    pub async fn add(&self) -> Result<(Option<String>, Pin)> {
        let url = self.url.clone();
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

impl ContainerAddOpts {
    pub fn add(&self) -> Result<(Option<String>, Pin)> {
        Ok((
            Some(self.image_name.clone()),
            container::Pin {
                image_name: self.image_name.clone(),
                image_tag: self.image_tag.clone(),
                arch: self.arch.clone(),
            }
            .into(),
        ))
    }
}

impl UrlAddOpts {
    pub async fn add(&self, unpack: bool) -> Result<(Option<String>, Pin)> {
        let pin: Pin = if self.mutable {
            urlpin::MutableUrlPin {
                update_url: self.url.clone(),
                unpack,
            }
            .into()
        } else {
            urlpin::UrlPin {
                url: self.url.clone(),
                unpack,
            }
            .into()
        };
        Ok((None, pin))
    }
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
            AddCommands::Tarball(p) => p.add(true).await?,
            AddCommands::Url(p) => p.add(false).await?,
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

pub fn read_pins(path: &Path) -> Result<NixPins> {
    let fh = BufReader::new(File::open(path).with_context(move || {
        format!(
            "Failed to open {}. You must initialize npins before you can show current pins.",
            path.display()
        )
    })?);
    NixPins::from_json_versioned(serde_json::from_reader(fh)?)
        .context("Failed to deserialize sources.json")
}

impl Opts {
    fn read_pins(&self) -> Result<NixPins> {
        read_pins(self.mode.lockfile())
    }

    fn write_pins(&self, pins: &NixPins) -> Result<()> {
        let path = self.mode.lockfile();
        let mut fh = File::create(path)
            .with_context(move || format!("Failed to open {} for writing.", path.display()))?;
        serde_json::to_writer_pretty(&mut fh, &pins.to_value_versioned())?;
        fh.write_all(b"\n")?;
        Ok(())
    }

    async fn init(&self, o: &InitOpts) -> Result<()> {
        log::info!("Welcome to npins!");

        if let SourceMode::Directory {
            default_nix,
            directory,
            ..
        } = &self.mode
        {
            if !directory.exists() {
                log::info!("Creating `{}` directory", directory.display());
                std::fs::create_dir(directory).context("Failed to create npins folder")?;
            }
            log::info!("Writing default.nix");
            let mut fh = File::create(default_nix).context("Failed to create npins default.nix")?;
            fh.write_all(DEFAULT_NIX.as_bytes())?;
        }

        let path = self.mode.lockfile();
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
            path.display()
        );
        Ok(())
    }

    fn show(&self, opts: &ShowOpts) -> Result<()> {
        let pins = self.read_pins()?;

        let print_pin = if opts.plain {
            |name: &str, _: _| println!("{name}")
        } else {
            |name: &str, pin: &Pin| {
                println!("{name}: ({})", pin.pin_type());
                println!("{pin}");
            }
        };

        let mut errors = Vec::new();

        match &opts.names[..] {
            [] => {
                for (name, pin) in pins.pins.iter() {
                    print_pin(name, pin);
                }
            },
            names if opts.exclude => {
                for (name, pin) in pins.pins.iter() {
                    if !names.contains(name) {
                        print_pin(name, pin);
                    }
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

        anyhow::ensure!(
            errors.is_empty(),
            "Couldn't find the following pins: {:?}",
            errors
        );

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
            diff1.into_iter().chain(diff2).collect()
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
                let diff = Self::update_one(name, pin, opts.strategy).await?;
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
        if let SourceMode::Directory {
            default_nix,
            directory,
            ..
        } = &self.mode
        {
            anyhow::ensure!(
                directory.exists(),
                "Could not find npins folder at {}",
                directory.display(),
            );

            if std::fs::read_to_string(default_nix)? == DEFAULT_NIX {
                log::info!("default.nix is already up to date");
            } else {
                log::info!("Replacing default.nix with an up to date version");
                std::fs::write(default_nix, DEFAULT_NIX)
                    .context("Failed to create npins default.nix")?;
            }
        }

        log::info!("Upgrading lock file to the newest format version");
        let path = self.mode.lockfile();
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
        let mut pins = self.read_pins()?;

        let mut errors = Vec::new();

        for name in &r.names {
            if pins.pins.remove(name).is_none() {
                errors.push(name);
            }
        }

        anyhow::ensure!(
            errors.is_empty(),
            "Couldn't find the following pins: {:?}",
            errors
        );

        self.write_pins(&pins)?;
        log::info!("Successfully removed pins.");
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

    async fn import_niv(&self, o: &ImportNivOpts) -> Result<()> {
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

            let pin: import::NivPin = serde_json::from_value(pin.clone())
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

            let pin: import::FlakePin = serde_json::from_value(pin.clone())
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

    async fn import_lon(&self, o: &ImportLonOpts) -> Result<()> {
        let mut pins = self.read_pins()?;

        let lon: BTreeMap<String, serde_json::Value> =
            serde_json::from_reader(File::open(&o.path).context(anyhow::format_err!(
                "Could not open lon.lock at '{}'",
                o.path.canonicalize().unwrap_or_else(|_| o.path.clone()).display()
            ))?)
            .context("Lon file is not a valid JSON dict")?;
        /* Finally a well-versioned format! (though unclear to me why the version is a string …) */
        let version = lon
            .get("version")
            .context("Lockfile must contain a 'version' key")?;
        anyhow::ensure!(
            version.as_str() == Some("1"),
            "Only lon lockfile version \"1\" is supported; found {}",
            version,
        );
        let sources: BTreeMap<String, serde_json::Value> = serde_json::from_value(
            lon.get("sources")
                .context("Lockfile must contain a 'sources' key")?
                .clone(),
        )
        .context("'sources' key is not a valid JSON dict")?;

        pub async fn import(
            name: &str,
            pin: Option<&serde_json::Value>,
            npins: &mut NixPins,
            sources: &BTreeMap<String, serde_json::Value>,
        ) -> anyhow::Result<()> {
            let pin = pin
                .or_else(|| sources.get(name))
                .with_context(|| anyhow::format_err!("Pin '{name}' not found in lon.lock"))?;
            anyhow::ensure!(
                !npins.pins.contains_key(name),
                "Pin '{}' exists in both files, this is a collision. Please delete the entry in one of the files.",
                name
            );

            let pin: import::LonPin = serde_json::from_value(pin.clone())
                .context("Pin is either invalid, or we don't support it")?;
            let mut pin: Pin = pin
                .try_into()
                .context("Could not convert pin to npins format")?;
            let diff = pin.fetch().await.context("Failed to update the pin")?;
            if !diff.is_empty() {
                log::warn!("Imported pin was modified by the import!");
                write_diff(&mut std::io::stderr(), name, &diff);
            }
            npins.pins.insert(name.to_string(), pin);

            Ok(())
        }

        if let Some(name) = &o.name {
            import(name, None, &mut pins, &sources).await?;
        } else {
            for (name, pin) in sources.iter() {
                log::info!("Importing {}", name);
                if let Err(err) = import(name, Some(pin), &mut pins, &sources).await {
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
        {
            let pins = self.read_pins()?;
            anyhow::ensure!(
                pins.pins.contains_key(&o.name),
                "Pin '{}' does not exist.",
                o.name,
            );
        }

        let out_path = nix::nix_eval_pin(self.mode.lockfile(), &o.name)
            .await
            .context("Could not evaluate pin")?;
        /* note(piegames): HMU if you ever find yourself using npins on Windows */
        use std::os::unix::ffi::OsStrExt;
        std::io::stdout()
            .write_all(out_path.as_path().as_os_str().as_bytes())
            .unwrap();
        Ok(())
    }

    pub fn run(&self) -> Result<()> {
        match &self.command {
            Command::Init(o) => start_runtime(self.init(o))?,
            Command::Show(o) => self.show(o)?,
            Command::Add(a) => start_runtime(self.add(a))?,
            Command::Update(o) => start_runtime(self.update(o))?,
            Command::Verify(o) => start_runtime(self.verify(o))?,
            Command::Upgrade => self.upgrade()?,
            Command::Remove(r) => self.remove(r)?,
            Command::ImportNiv(o) => start_runtime(self.import_niv(o))?,
            Command::ImportFlake(o) => start_runtime(self.import_flake(o))?,
            Command::ImportLon(o) => start_runtime(self.import_lon(o))?,
            Command::Freeze(o) => start_runtime(self.freeze(o))?,
            Command::Unfreeze(o) => start_runtime(self.unfreeze(o))?,
            Command::GetPath(o) => start_runtime(self.get_path(o))?,
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
            result_writer(&mut stderr);
            stderr.flush().unwrap();
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

fn main() -> Result<()> {
    let opts = crate::Opts::parser().run();

    env_logger::builder()
        .filter_level(if opts.verbose {
            log::LevelFilter::Debug
        } else {
            log::LevelFilter::Info
        })
        .format_timestamp(None)
        .format_target(false)
        .init();

    opts.run()
}

fn start_runtime(future: impl Future<Output = Result<()>>) -> Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(future)
}
