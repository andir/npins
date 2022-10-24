{ system ? builtins.currentSystem }:
let
  pins = import ./npins;
  pkgs = import pins.nixpkgs { inherit system; };
  inherit (pkgs) stdenv lib;

  pre-commit = (import pins."pre-commit-hooks.nix").run {
    src = ./.;
    hooks = {
      nixpkgs-fmt.enable = true;
      rustfmt.enable = true;
      update-readme = {
        enable = true;
        files = "((readme\\.pre\\.md|readme\\.post\\.md|^readme\\.nix|Cargo\\.toml)|\\.rs)$";
        entry = toString (pkgs.writeShellScript "update-readme" ''
          ${pkgs.nix}/bin/nix-build ${toString ./readme.nix} -o readme && cp readme README.md
          exec ${pkgs.git}/bin/git diff --quiet --exit-code -- README.md
        '');
      };
    };
  };

in
pkgs.mkShell
{
  nativeBuildInputs = with pkgs; [
    cargo
    rustc
    rust-analyzer
    rustfmt
    nixpkgs-fmt
    nix_2_3
    nix-prefetch-git
    git
  ];

  # https://github.com/cachix/pre-commit-hooks.nix/issues/131
  shellHook =
    if (!(stdenv.isDarwin && stdenv.isAarch64))
    then pre-commit.shellHook
    else "";
}
