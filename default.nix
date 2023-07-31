{
  system ? builtins.currentSystem,
  pins ? import ./npins,
  pkgs ? import pins.nixpkgs { inherit system; },
}:
pkgs.callPackage ./npins.nix { }
