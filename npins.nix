{
  lib,
  pkgs,
  rustPlatform,
  nix-gitignore,
  makeWrapper,
  runCommand,
  stdenv,
  darwin,

  # runtime dependencies
  nix, # for nix-prefetch-url
  nix-prefetch-git,
  git, # for git ls-remote
}:
let
  paths = [
    "^/src$"
    "^/src/.+.rs$"
    "^/npins$"
    "^/npins/default.nix$"
    "^/Cargo.lock$"
    "^/Cargo.toml$"
  ];

  extractSource =
    src:
    let
      baseDir = toString src;
    in
    expressions:
    builtins.path {
      path = src;
      filter =
        path:
        let
          suffix = lib.removePrefix baseDir path;
        in
        _: lib.any (r: builtins.match r suffix != null) expressions;
      name = "source";
    };

  src = extractSource ./. paths;

  cargoToml = builtins.fromTOML (builtins.readFile (src + "/Cargo.toml"));
  runtimePath = lib.makeBinPath [
    nix
    nix-prefetch-git
    git
  ];
  self = rustPlatform.buildRustPackage {
    pname = cargoToml.package.name;
    version = cargoToml.package.version;
    cargoLock = {
      lockFile = src + "/Cargo.lock";
    };

    inherit src;

    buildInputs = lib.optional stdenv.isDarwin (
      with darwin.apple_sdk.frameworks;
      [
        Security
        SystemConfiguration
      ]
    );
    nativeBuildInputs = [ makeWrapper ];

    # (Almost) all tests require internet
    doCheck = false;

    postFixup = ''
      wrapProgram $out/bin/npins --prefix PATH : "${runtimePath}"
    '';

    meta.tests = pkgs.callPackage ./test.nix { npins = self; };
  };
in
self
