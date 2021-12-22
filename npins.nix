{ lib
, rustPlatform
, nix-gitignore
, makeWrapper
, runCommand

  # runtime dependencies
, nix # for nix-prefetch-url
, nix-prefetch-git
, git # for git ls-remote
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

  extractSource = src:
    let baseDir = toString src; in
    expressions:
    builtins.path {
      path = src;
      filter = path:
        let suffix = lib.removePrefix baseDir path; in
        _: lib.any (r: builtins.match r suffix != null) expressions;
      name = "source";
    };

  src = extractSource ./. paths;

  cargoToml = builtins.fromTOML (builtins.readFile (src + "/Cargo.toml"));
  runtimePath = lib.makeBinPath [ nix nix-prefetch-git git ];
in
rustPlatform.buildRustPackage {
  pname = cargoToml.package.name;
  version = cargoToml.package.version;
  cargoLock = {
    lockFile = src + "/Cargo.lock";
    outputHashes."hubcaps-0.6.2" = "0xxla9d71ar0z9kmilx6qa077d3lq7zi3kjl234yjdmyb56n54iq";
  };

  inherit src;

  nativeBuildInputs = [ makeWrapper ];

  postFixup = ''
    wrapProgram $out/bin/npins --prefix PATH : "${runtimePath}"
  '';
}
