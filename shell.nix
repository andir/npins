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
      git
      npins
    ]
    ++ (lib.optionals stdenv.isDarwin [
      pkgs.libiconv
      pkgs.darwin.apple_sdk.frameworks.Security
    ]);

  inherit (pre-commit) shellHook;
}
