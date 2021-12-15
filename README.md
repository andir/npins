# npins

Nix dependency pinning.
## npins help
```console
$ npins help
npins 0.1.0

USAGE:
    npins [folder] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <folder>    Base folder for npins.json and the boilerplate default.nix [env: NPINS_FOLDER=]  [default: npins]

SUBCOMMANDS:
    add       Adds a new pin entry
    help      Prints this message or the help of the given subcommand(s)
    init      Intializes the npins directory. Running this multiple times will restore/upgrade the `default.nix` and
              never touch your pins.json
    remove    Removes one pin entry
    show      Lists the current pin entries
    update    Updates all or the given pin to the latest version
```

## npins help init
```console
$ npins help init
npins-init 0.1.0
Intializes the npins directory. Running this multiple times will restore/upgrade the `default.nix` and never touch your
pins.json

USAGE:
    npins init

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

## npins help add
```console
$ npins help add
npins-add 0.1.0
Adds a new pin entry

USAGE:
    npins add [OPTIONS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -n, --name <name>    

SUBCOMMANDS:
    git               Track a git repository
    github            Track a branch from a GitHub repository
    github-release    Track the latest release from a GitHub repository
    help              Prints this message or the help of the given subcommand(s)
    pypi              Track a package on PyPi
```

## npins help update
```console
$ npins help update
npins-update 0.1.0
Updates all or the given pin to the latest version

USAGE:
    npins update [name]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <name>    
```

## npins help remove
```console
$ npins help remove
npins-remove 0.1.0
Removes one pin entry

USAGE:
    npins remove <name>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <name>    
```

## Usage in nix expressions

`npins` creates a `default.nix` file in the target directory that exports each
of the dependencies as an attribute.

Each attribute has the `outPath` property which means it can be used just like
regular results of fetchers in `nixpkgs`.

Example:

```nix
let
   sources = import ./npins;
   pkgs = import sources.nixpkgs {};
in pkgs.mkShell {
   # ...
}
```

Additionally, depending on the type of pin (Git, GitHub, GitHub release, ...)
additional information about the fetched sources are available.

Example:

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
