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
  nix-prefetch-git,
  nix-prefetch-docker,
  git, # for git ls-remote
}:
let
  paths = [
    "^/src$"
    "^/src/.+$"
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
    nix-prefetch-git
    nix-prefetch-docker
    git
  ];
  self = rustPlatform.buildRustPackage {
    pname = cargoToml.package.name;
    version = cargoToml.package.version;
    cargoLock = {
      lockFile = src + "/Cargo.lock";

      outputHashes = {
        "nix-compat-0.1.0" = "sha256-w8N09ZUm0V+6LdjGyL6g0mb3COoNWONfmr/M3S9C+do=";
      };
    };

    inherit src;

    nativeBuildInputs = [ makeWrapper ];

    cargoBuildFlags = [
      "--bin"
      "npins"
      "--features"
      "clap,crossterm,env_logger"
    ];

    # (Almost) all tests require internet
    doCheck = false;

    postFixup = ''
      wrapProgram $out/bin/npins --prefix PATH : "${runtimePath}"
    '';

    meta.tests = pkgs.callPackage ./test.nix { npins = self; };
    meta.mainProgram = cargoToml.package.name;
  };
in
self
