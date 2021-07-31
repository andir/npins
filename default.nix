{ lib
, rustPlatform
, nix-gitignore
, makeWrapper

  # runtime dependencies
, nix # for nix-prefetch-url
, nix-prefetch-git
}:
let
  src = nix-gitignore.gitignoreSource [ ] ./.;
  cargoToml = builtins.fromTOML (builtins.readFile (src + "/Cargo.toml"));
  runtimePath = lib.makeBinPath [ nix nix-prefetch-git ];
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
