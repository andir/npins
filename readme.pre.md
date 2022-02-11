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
