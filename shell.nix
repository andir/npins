{
  system ? builtins.currentSystem,
}:
let
  pins = import ./npins;
  pkgs = import pins.nixpkgs { inherit system; };
  inherit (pkgs) stdenv lib;

  pre-commit = (import pins."pre-commit-hooks.nix").run {
    src = ./.;
    hooks = {
      nixfmt-rfc-style = {
        enable = true;
        settings.width = 100;
      };
      rustfmt.enable = true;
      update-readme = {
        enable = true;
        files = "((^README\\.md\\.in|^README\\.md|^readme\\.nix|^Cargo\\.toml)|\\.rs)$";
        entry = toString (
          pkgs.writeShellScript "update-readme" ''
            ${pkgs.nix}/bin/nix-build ${toString ./readme.nix} -o readme && cp readme README.md
            exec ${pkgs.git}/bin/git diff --quiet --exit-code -- README.md
          ''
        );
      };
      update-completions = {
        enable = true;
        files = "((^Cargo\\.toml)|\\.rs)$";
        entry = toString (
          pkgs.writeShellScript "update-completions" ''
            ${pkgs.cargo}/bin/cargo run -p npins-completions -- bash > completions/generated/npins.bash
            ${pkgs.cargo}/bin/cargo run -p npins-completions -- fish > completions/generated/npins.fish
            ${pkgs.cargo}/bin/cargo run -p npins-completions -- zsh > completions/generated/npins.zsh
            exec ${pkgs.git}/bin/git diff --quiet --exit-code -- completions/generated/*
          ''
        );
      };
    };
  };
in
pkgs.mkShell {
  nativeBuildInputs =
    with pkgs;
    [
      cargo
      cargo-expand
      clippy
      rustc
      rust-analyzer
      rustfmt
      nixfmt-rfc-style
      lix
      nix-prefetch-git
      nix-prefetch-docker
      git
      npins
    ]
    ++ (lib.optionals stdenv.isDarwin [
      pkgs.libiconv
    ]);

  inherit (pre-commit) shellHook;
}
