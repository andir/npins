{ system ? builtins.currentSystem }:
let
  pins = import ./npins;
  pkgs = import pins.nixpkgs { inherit system; };

  pre-commit = (import pins."pre-commit-hooks.nix").run {
    src = ./.;
    hooks = {
      nixpkgs-fmt.enable = true;
      rustfmt.enable = true;
    };
  };

in
pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    cargo
    rustc
    rust-analyzer
    rustfmt
    nixpkgs-fmt
    nix
    nix-prefetch-git
  ];

  inherit (pre-commit) shellHook;
}
