{ system ? builtins.currentSystem }:
let
  pins = import ./npins;
  pkgs = import pins.nixpkgs { inherit system; };
  npins = pkgs.callPackage ./default.nix { };
in
pkgs.runCommand "readme"
{
  nativeBuildInputs = [ npins ];
  preferLocalBuild = true;

  raw = ./README.md.in;
} ''
  set -euo pipefail
  content="$(cat $raw)"

  # Match "{{foo}}"
  regex='\{\{([^\}]*)\}\}'

  # Replace "{{foo}}" with "$(foo)", i.e. run the command
  while [[ $content =~ $regex ]]; do
    command="''${BASH_REMATCH[1]}"
    # Run the command, capture failure gracefully
    command="$($command 2>&1 || true)"

    content="''${content/"''${BASH_REMATCH[0]}"/"$command"}"
  done

  echo "$content" >> $out
''
