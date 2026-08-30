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
      nixfmt = {
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
        stages = [ "pre-push" ];
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
      nixfmt
      lix
      nix-prefetch-git
      nix-prefetch-docker
      skopeo
      git
      just
      npins
    ]
    ++ (lib.optionals stdenv.hostPlatform.isDarwin [
      pkgs.libiconv
    ]);

  inherit (pre-commit) shellHook;
}
