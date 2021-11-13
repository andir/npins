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
