{ system ? builtins.currentSystem }:
let
  pins = import ./npins;
  pkgs = import pins.nixpkgs { inherit system; };
  npins = pkgs.callPackage ./default.nix { };

  mkCommandOutput = args:
    let
      cmd = pkgs.lib.concatStringsSep " " args;
    in
    pkgs.runCommand "cmd-output"
      {
        nativeBuildInputs = [ npins ];
        inherit cmd;
      } ''

      set -e
      HEADER=$(echo $cmd | cut -d " " -f 2)
      echo "### npins $HEADER" > $out
      echo '```console' >> $out
      echo "\$ npins $cmd" >> $out
      npins $cmd >> $out 2>&1
      echo '```' >> $out
      echo >> $out
    '';


  commands = [
    [ "help" ]
    [ "help" "init" ]
    [ "help" "add" ]
    [ "help" "update" ]
    [ "help" "remove" ]
  ];

in
pkgs.runCommand "readme"
{
  pre = ./readme.pre.md;
  post = ./readme.post.md;
  usage = builtins.map mkCommandOutput commands;
} ''
  cat $pre > $out
  for file in "''${usage[@]}"; do
    cat $file >> $out
  done
  cat $post >> $out
''
