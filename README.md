# npins

<!-- badges -->
[![License][license-shield]][license-url]
[![Contributors][contributors-shield]][contributors-url]
[![Issues][issues-shield]][issues-url]
[![PRs][pr-shield]][pr-url]
[![Tests][test-shield]][test-url]
![Matrix][matrix-url]

<!-- teaser -->
<br />
<p align="center">
  <h2 align="center">npins</h2>
  <p align="center">
    Simple and convenient pinning of dependencies in Nix
  </p>
</p>

## About

`npins` is a simple tool for handling different types of dependencies in a Nix project.

### Features

The set of features provided by `npins` is minimalistic on purpose:

- Initializing a project
- Adding a dependency
- Updating a dependency

The following types of dependencies are currently supported:

- Git repositories
- Branches in GitHub repositories
- GitHub releases
- PyPi packages

## Getting Started
`npins` can be built and installed like any other Nix based project. This section provides some information on installing `npins` and runs through the different usage scenarios.

### Installation
Ideally you will want to install `npins` through an overlay (eventually managed by `npins` itself). You can however use `nix run` to try it in a shell:

```
$ nix run -f https://github.com/andir/npins/archive/master.tar.gz
```

You could also install it to your profile using `nix-env` (not recommended):
```
nix-env -f https://github.com/andir/npins/archive/master.tar.gz -i
```

### Usage

#### Initialization

In order to start using `npins` to track any dependencies you need to first [initialize](#npins-help) the project:

```
$ npins init
```

This will create an `npins` folder with an internal representation of dependencies and a `default.nix` that can be imported. For more information on initialization please refer to [npins init](#npins-init).

#### Adding dependencies

After initialization you will want to add the dependencies you are interested in. Let's assume you want to track [npmlock2nix](https://github.com/nix-community/npmlock2nix):

```
$ npins add github nix-community npmlock2nix
```

This is going to update the internal representation and make `npmlock2nix` available via `npins/default.nix`. For more information on adding dependencies please refer to [npins add](#npins-add).

#### Updating dependencies

After you added a dependency you will want to update it regularly. Assuming you previously added the `npmlock2nix` you could update it as follows:

```
$ npins update npmlock2nix
```

This is going to inspect the `npmlock2nix` dependency and update it to the latest available revision. For more information on updating dependencies please refer to [npins update](#npins-update).

#### Removing dependencies

If no longer want to track some dependency you can remove it again as follows. Assuming you would want to remove the previously added `npmlock2nix` dependency:

```
$ npins remove npmlock2nix
```

For more information on removing dependencies please refer to [npins remove](#npins-remove).

#### Packaging Example

Below is an example of what an expression might look like for packaging some `foobar` dependency which is tracked using `npins`:
```nix
let
   sources = import ./npins;
   pkgs = import sources.nixpkgs {};
in pkgs.stdenv.mkDerivation {
   # Use the name and owner of the repository as package name
   pname = sources.neovim.owner + "-" + sources.neovim.repository;

   # this will set the version of the package to the git revision
   version = sources.neovim.revision;

   # or, if you are tracking a tag you can use the name of the release as
   # defined on GitHub:
   # version = sources.neovim.release_name;
   src = sources.neovim;
}

```

## Contributing

Contributions to this project are welcome in the form of GitHub Issues or PRs. Please consider the following before creating PRs:

- This project has several commit hooks configured via `pre-commit-hooks`, make sure you have these enabled and they are passing
- If you are planning to make any considerable changes, you should first present your plans in a GitHub issue so it can be discussed
- Simplicity and ease of use is one of the design goals, please keep this in mind when making contributons

## Commands

The section below provides an overview of all available commands and their arguments.
### npins help
```console
$ npins help
npins 0.1.0

USAGE:
    npins [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]

SUBCOMMANDS:
    add           Adds a new pin entry
    fetch         Query some release information and then print out the entry
    help          Prints this message or the help of the given subcommand(s)
    import-niv    Try to import entries from Niv
    init          Intializes the npins directory. Running this multiple times will restore/upgrade the `default.nix`
                  and never touch your sources.json
    remove        Removes one pin entry
    show          Lists the current pin entries
    update        Updates all or the given pin to the latest version
    upgrade       Upgrade the sources.json and default.nix to the latest format version. This may occasionally break
                  Nix evaluation!
```

### npins init
```console
$ npins help init
npins-init 0.1.0
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

### npins add
```console
$ npins help add
npins-add 0.1.0
Adds a new pin entry

USAGE:
    npins add [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help    Prints help information

OPTIONS:
    -d, --directory <folder>    Base folder for sources.json and the boilerplate default.nix [env: NPINS_DIRECTORY=]
                                [default: npins]
    -n, --name <name>           

SUBCOMMANDS:
    git       Track a git repository
    github    Track a GitHub repository
    gitlab    Track a GitLab repository
    help      Prints this message or the help of the given subcommand(s)
    pypi      Track a package on PyPi
```

### npins update
```console
$ npins help update
npins-update 0.1.0
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

### npins remove
```console
$ npins help remove
npins-remove 0.1.0
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

<!-- MARKDOWN LINKS & IMAGES -->

[contributors-shield]: https://img.shields.io/github/contributors/andir/npins.svg?style=for-the-badge
[contributors-url]: https://github.com/andir/npins/graphs/contributors
[issues-shield]: https://img.shields.io/github/issues/andir/npins.svg?style=for-the-badge
[issues-url]: https://github.com/andir/npins/issues
[license-shield]: https://img.shields.io/github/license/andir/npins.svg?style=for-the-badge
[license-url]: https://github.com/andir/npins/blob/master/LICENSE
[test-shield]: https://img.shields.io/github/workflow/status/andir/npins/test/master?style=for-the-badge
[test-url]: https://github.com/andir/npins/actions
[pr-shield]: https://img.shields.io/github/issues-pr/andir/npins.svg?style=for-the-badge
[pr-url]: https://github.com/andir/npins/pulls
[matrix-url]: https://img.shields.io/matrix/npins:kack.it?label=Chat%20on%20Matrix?style=for-the-badge
