{ system ? builtins.currentSystem
, pins ? import ../npins
, pkgs ? import pins.nixpkgs { inherit system; }
, npins ? pkgs.callPackage ../npins.nix { }
}:
let
  smokers = pkgs.rustPlatform.buildRustPackage {
    pname = "smokers";
    version = "git-${pins.smokers.revision}";
    src = pins.smokers;

    cargoLock.lockFile = pins.smokers + "/Cargo.lock";
  };
  mkSmokersTest =
  { name
  , commands ? [
    # { args ? []
    # , stdout ? ""
    # , exit-code ? 1
    # }
  ]
  , preTest ? ""
  , postTest ? ""
  }: let
    specs = builtins.map (c: let
      command = { args = []; stdout = ""; exit-code = 0; } // c;
    in {
      command = [
        "${npins}/bin/npins"
      ] ++ command.args;
      inherit (command) stdout exit-code;
    }) commands;
    specFiles =
      builtins.map (spec:
        pkgs.writeText "${name}-spec.json" (builtins.toJSON spec)) specs;
    in
    pkgs.runCommand name {
        nativeBuildInputs = [ npins smokers ];
    } ''
      ${preTest}
      mkdir $out
      cd $out
      set -ex
      ${pkgs.lib.concatMapStringsSep "\n" (specFile: ''
        smokers ${specFile} > test.log
      '') specFiles}
      set +ex
      ${postTest}
    '';

      gitRepoWithBranch = pkgs.runCommand "git-repo"
        {
          nativeBuildInputs = [ pkgs.gitMinimal ];
        } ''
        export HOME=$TMP
        git config --global user.email "you@example.com"
        git config --global user.name "Your Name"
        git init $out
        cd $out
        git checkout -B test-branch
        touch test.txt
        git add test.txt
        git commit -v -m "foo"
        git update-server-info
      '';
in
{
  no-arguments = mkSmokersTest {
    name = "no-arguments";
    commands = [ { exit-code = 1; } ];
  };

  help = mkSmokersTest {
    name = "help";
    commands = [ {
        args = [ "--help" ];
        stdout = null;
        exit-code = 0;
    } ];
  };

  gitRepoBranch2 = mkSmokersTest {
    name = "gitRepoBranch";
    preTest = ''
      cd ${gitRepoWithBranch}/.git
      ${pkgs.python3}/bin/python -m http.server 8000 &
      sleep 3
    '';
    commands = [
      { args = [ "init" "--bare" ]; }
      { args = [ "add" "git" "http://localhost:8000" "-b" "test-branch" ]; }
      { args = [ "show" ]; }
    ];
  };

  
  gitRepoBranch =
    let
    in
    pkgs.runCommand "git-repo" { nativeBuildInputs = [ npins pkgs.python3 ]; } ''
      set -e
      cd ${gitRepoWithBranch}/.git
      python -m http.server 8000 &
      # FIXME: we must wait for the HTTP port to be open
      sleep 3
      mkdir $out
      cd $out
      npins init --bare
      npins add git http://localhost:8000 -b test-branch
      npins show
    '';

  gitRepoTag =
    let
      gitRepoWithTag = pkgs.runCommand "git-repo-with-tag"
        {
          nativeBuildInputs = [ pkgs.gitMinimal ];
        } ''
        export HOME=$TMP
        git config --global user.email "you@example.com"
        git config --global user.name "Your Name"
        git init $out
        cd $out
        touch test.txt
        git add test.txt
        git commit -v -m "foo"
        git tag v0.1
        git update-server-info
      '';

    in
    pkgs.runCommand "git-repo" { nativeBuildInputs = [ smokers npins pkgs.python3 ]; } ''
      set -e
      cd ${gitRepoWithTag}/.git
      python -m http.server 8000 &
      # FIXME: we must wait for the HTTP port to be open
      sleep 3
      mkdir $out
      cd $out
      npins init --bare
      npins add git http://localhost:8000
      npins show
    '';
}
