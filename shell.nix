{
  system ? builtins.currentSystem,
}:
let
  pins = import ./npins;
  pkgs = import pins.nixpkgs {
    inherit system;
    overlays = [
      (self: super: {
        nixfmt = super.nixfmt.overrideAttrs (old: {
          src = pins.nixfmt;
        });
      })
    ];
  };
  inherit (pkgs) stdenv lib;

  pre-commit = (import pins."pre-commit-hooks.nix").run {
    src = ./.;
    tools.nixfmt = pkgs.nixfmt; # Why don't they just take it from our pkgs?
    settings.nixfmt.width = 100;
    hooks = {
      nixfmt.enable = true;
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
      clippy
      rustc
      rust-analyzer
      rustfmt
      nixfmt
      nix_2_3
      nix-prefetch-git
      git
    ]
    ++ (lib.optionals stdenv.isDarwin [
      pkgs.libiconv
      pkgs.darwin.apple_sdk.frameworks.Security
    ]);

  inherit (pre-commit) shellHook;
}
