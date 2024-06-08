# npins

Simple and convenient dependency pinning for Nix

<!-- badges -->
[![License][license-shield]][license-url]
[![Contributors][contributors-shield]][contributors-url]
[![Issues][issues-shield]][issues-url]
[![PRs][pr-shield]][pr-url]
[![Tests][test-shield]][test-url]
[![Matrix][matrix-image]][matrix-url]

## About

`npins` is a simple tool for handling different types of dependencies in a Nix project. It is inspired by and comparable to [Niv](https://github.com/nmattia/niv).

### Features

- Track git branches
- Track git release tags
  - Tags must roughly follow SemVer
  - GitHub/GitLab releases are intentionally ignored
- For git repositories hosted on GitHub or GitLab, `fetchTarball` is used instead of `fetchGit`
- Track Nix channels
  - Unlike tracking a channel from its git branch, this gives you access to the `programs.sqlite` database
- Track PyPi packages

## Getting Started

### Installation

`npins` should readily be available in all sufficiently new `nixpkgs`:

```sh
nix-shell -p npins
```

You can easily get a nightly if you want to (requires newstyle Nix commands):

```sh
nix shell -f https://github.com/andir/npins/archive/master.tar.gz
```

You could also install it to your profile using `nix-env` (not recommended, but might be useful for bootstrapping):

```sh
nix-env -f https://github.com/andir/npins/archive/master.tar.gz -i
```

### Quickstart

```
$ npins init
[INFO ] Welcome to npins!
[INFO ] Writing default.nix
[INFO ] Writing initial sources.json with nixpkgs entry (need to fetch latest commit first)
[INFO ] Successfully written initial files to 'npins'.

$ tree
.
└── npins
    ├── default.nix
    └── sources.json

1 directory, 2 files

$ npins show
nixpkgs: (Nix channel)
    name: nixpkgs-unstable
    url: https://releases.nixos.org/nixpkgs/nixpkgs-22.05pre378171.ff691ed9ba2/nixexprs.tar.xz
    hash: 04xggrc0qz5sq39mxdhqh0d2mljg9wmmn8nbv71x3vblam1wyp9b

$ cat npins/sources.json
{
  "pins": {
    "nixpkgs": {
      "type": "Channel",
      "name": "nixpkgs-unstable",
      "url": "https://releases.nixos.org/nixpkgs/nixpkgs-22.05pre378171.ff691ed9ba2/nixexprs.tar.xz",
      "hash": "04xggrc0qz5sq39mxdhqh0d2mljg9wmmn8nbv71x3vblam1wyp9b"
    }
  },
  "version": 2
}
```

In Nix, you may then use it like this:

```nix
let
  sources = import ./npins;
  pkgs = import sources.nixpkgs {};
in
  …
```

You may also use attributes from the JSON file, they are exposed 1:1. For example, `sources.myPackage.version` should work for many pin types (provided that that pin actually tracks some version). Note however that the available attribute may change over time; see `npins upgrade` below.

## Usage

```console
$ npins help
npins 0.2.4

USAGE:
    npins [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]

SUBCOMMANDS:
    add             Adds a new pin entry
    help            Prints this message or the help of the given subcommand(s)
    import-flake    Try to import entries from flake.lock
    import-niv      Try to import entries from Niv
    init            Intializes the npins directory. Running this multiple times will restore/upgrade the
                    `default.nix` and never touch your sources.json
    remove          Removes one pin entry
    show            Lists the current pin entries
    update          Updates all or the given pin to the latest version
    upgrade         Upgrade the sources.json and default.nix to the latest format version. This may occasionally
                    break Nix evaluation!
```

### Initialization

In order to start using `npins` to track any dependencies you need to first [initialize](#npins-help) the project:

```sh
npins init
```

This will create an `npins` folder with a `default.nix` and `sources.json` within. By default, the `nixpkgs-unstable` channel will be added as pin.

```console
$ npins help init
npins-init 0.2.4
Intializes the npins directory. Running this multiple times will restore/upgrade the `default.nix` and never touch your
sources.json

USAGE:
    npins init [FLAGS] [OPTIONS]

FLAGS:
        --bare    Don't add an initial `nixpkgs` entry
    -h, --help    Prints help information

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]
```

### Migrate from Niv

You can import your pins from Niv:

```sh
npins import-niv nix/sources.json
npins update
```

In your Nix configuration, simply replace `import ./nix/sources.nix` with `import ./npins` — it should be a drop-in replacement.

Note that the import functionality is minimal and only preserves the necessary information to identify the dependency, but not the actual pinned values themselves. Therefore, migrating must always come with an update (unless you do it manually).

```console
$ npins help import-niv
npins-import-niv 0.2.4
Try to import entries from Niv

USAGE:
    npins import-niv [OPTIONS] [path]

FLAGS:
    -h, --help    Prints help information

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]
    -n, --name <name>           Only import one entry from Niv

ARGS:
    <path>     [default: nix/sources.json]
```

### Adding dependencies

Some common usage examples:

```sh
npins add channel nixos-21.11
# Remove -b to fetch the latest release
npins add git https://gitlab.com/simple-nixos-mailserver/nixos-mailserver.git -b "nixos-21.11"
npins add github ytdl-org youtube-dl
npins add github ytdl-org youtube-dl -b master # Track nightly
npins add github ytdl-org youtube-dl -b master --at c7965b9fc2cae54f244f31f5373cb81a40e822ab # We want *that* commit
npins add gitlab simple-nixos-mailserver nixos-mailserver --at v2.3.0 # We want *that* tag (note: tag, not version)
npins add gitlab my-org my-private-repo --token H_BRqzV3NcaPvXcYs2Xf # Use a token to access a private repository
npins add pypi streamlit # Use latest version
npins add pypi streamlit --at 1.9.0 # We want *that* version
npins add pypi streamlit --upper-bound 2.0.0 # We only want 1.X
```

Depending on what kind of dependency you are adding, different arguments must be provided. You always have the option to specify a version (or hash, depending on the type) you want to pin to. Otherwise, the latest available version will be fetched for you. Not all features are present on all pin types.

```console
$ npins help add
npins-add 0.2.4
Adds a new pin entry

USAGE:
    npins add [FLAGS] [OPTIONS] <SUBCOMMAND>

FLAGS:
    -n, --dry-run    Don't actually apply the changes
    -h, --help       Prints help information

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]
        --name <name>           Custom name for the pin entry

SUBCOMMANDS:
    channel    Track a Nix channel
    git        Track a git repository
    github     Track a GitHub repository
    gitlab     Track a GitLab repository
    help       Prints this message or the help of the given subcommand(s)
    pypi       Track a package on PyPi
```

There are several options for tracking git branches, releases and tags:

```console
$ npins help add git
npins-add-git 0.2.4
Track a git repository

USAGE:
    npins add git [FLAGS] [OPTIONS] <url>

FLAGS:
    -h, --help            Prints help information
        --pre-releases    Also track pre-releases. Conflicts with the --branch option

OPTIONS:
        --at <tag or rev>                    Use a specific commit/release instead of the latest. This may be a tag
                                             name, or a git revision when --branch is set
    -b, --branch <branch>                    Track a branch instead of a release
    -d, --directory <folder>                 Base folder for sources.json and the boilerplate default.nix [env:
                                             NPINS_DIRECTORY=]  [default: npins]
        --release-prefix <release-prefix>    Optional prefix required for each release name / tag. For example, setting
                                             this to "release/" will only consider those that start with that string
        --upper-bound <version>              Bound the version resolution. For example, setting this to "2" will
                                             restrict updates to 1.X versions. Conflicts with the --branch option

ARGS:
    <url>    The git remote URL. For example <https://github.com/andir/ate.git>
```

### Removing dependencies

```console
$ npins help remove
npins-remove 0.2.4
Removes one pin entry

USAGE:
    npins remove [OPTIONS] <name>

FLAGS:
    -h, --help    Prints help information

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]

ARGS:
    <name>    
```

### Show current entries

This will print the currently pinned dependencies in a human readable format. The machine readable `sources.json` may be accessed directly, but make sure to always check the format version (see below).

```console
$ npins help show
npins-show 0.2.4
Lists the current pin entries

USAGE:
    npins show [OPTIONS]

FLAGS:
    -h, --help    Prints help information

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]
```

### Updating dependencies

You can decide to update only selected dependencies, or all at once. For some pin types, we distinguish between "find out the latest version" and "fetch the latest version". These can be controlled with the `--full` and `--partial` flags.

```console
$ npins help update
npins-update 0.2.4
Updates all or the given pin to the latest version

USAGE:
    npins update [FLAGS] [OPTIONS] [names]...

FLAGS:
    -n, --dry-run    Print the diff, but don't write back the changes
    -f, --full       Re-fetch hashes even if the version hasn't changed. Useful to make sure the derivations are in the
                     Nix store
    -h, --help       Prints help information
    -p, --partial    Don't update versions, only re-fetch hashes

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]

ARGS:
    <names>...    Update only those pins
```

### Upgrading the pins file

To ensure compatibility across releases, the `npins/sources.json` and `npins/default.nix` are versioned. Whenever the format changes (i.e. because new pin types are added), the version number is increased. Use `npins upgrade` to automatically apply the necessary changes to the `sources.json` and to replace the `default.nix` with one for the current version. No stability guarantees are made on the Nix side across versions.

```console
$ npins help upgrade
npins-upgrade 0.2.4
Upgrade the sources.json and default.nix to the latest format version. This may occasionally break Nix evaluation!

USAGE:
    npins upgrade [OPTIONS]

FLAGS:
    -h, --help    Prints help information

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]
```

### Using private GitLab repositories

There are two ways of specifying the access token (not deploy token!), either via an environment variable or via a parameter.
The access token needs at least the `read_api` and `read_repository` scopes and the `Reporter` role.
The `read_api` scope is not available for deploy tokens, hence they are not usable for npins.

Specifying the token via environment variable means that npins will use the token for adding/updating the pin but not write it to sources.json.
To update the repository in the future, the variable needs to be set again and nix needs to be configured accordingly to be able to fetch it (see the `netrc-file` option).
Environment example:
```console
$ GITLAB_TOKEN=H_BRqzV3NcaPvXcYs2Xf npins add gitlab my-org my-private-repo
```

When specifying the token via the `--token` parameter, the token is written to sources.json so future invocations of npins will use it as well.
The token is also embedded into the URL that nix downloads, so no further nix configuration is necessary.
As npins adds the token to your sources.json, this feature is not advised for publicly available repositories.
When a pin has specified a token, the `GITLAB_TOKEN` environment variable is ignored.
Parameter example:
```console
$ npins add gitlab my-org my-private-repo --token H_BRqzV3NcaPvXcYs2Xf
```

## Contributing

Contributions to this project are welcome in the form of GitHub Issues or PRs. Please consider the following before creating PRs:

- This project has several commit hooks configured in the `shell.nix`, make sure you have these enabled and they are passing
- This readme is templated, edit [README.md.in](./README.md.in) instead (the commit hook will take care of the rest)
- Consider discussing major features or changes in an issue first

<!-- MARKDOWN LINKS & IMAGES -->

[contributors-shield]: https://img.shields.io/github/contributors/andir/npins.svg?style=for-the-badge
[contributors-url]: https://github.com/andir/npins/graphs/contributors
[issues-shield]: https://img.shields.io/github/issues/andir/npins.svg?style=for-the-badge
[issues-url]: https://github.com/andir/npins/issues
[license-shield]: https://img.shields.io/github/license/andir/npins.svg?style=for-the-badge
[license-url]: https://github.com/andir/npins/blob/master/LICENSE
[test-shield]: https://img.shields.io/github/actions/workflow/status/andir/npins/test.yml?branch=master&style=for-the-badge
[test-url]: https://github.com/andir/npins/actions
[pr-shield]: https://img.shields.io/github/issues-pr/andir/npins.svg?style=for-the-badge
[pr-url]: https://github.com/andir/npins/pulls
[matrix-image]: https://img.shields.io/matrix/npins:kack.it?label=Chat%20on%20Matrix&server_fqdn=matrix.org&style=for-the-badge
[matrix-url]: https://matrix.to/#/#npins:kack.it
