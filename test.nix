{ system ? builtins.currentSystem
, pins ? import ./npins
, pkgs ? import pins.nixpkgs { inherit system; }
, npins ? pkgs.callPackage ./npins.nix { }
}:
let
  inherit (pkgs) lib;
  # Generate a git repository hat can be served via HTTP.
  #
  # By default the repository will contain an empty `test.txt`
  # file. For all defined tags the name of the tag is written to that
  # file for the respective commit for the tag.
  mkGitRepo =
    { name ? "git-repo"
    , branchName ? "master"
    , tags ? [ ]
    , extraCommands ? ""
    }:
    pkgs.runCommand "git-repo"
      {
        nativeBuildInputs = [ pkgs.gitMinimal ];
      } ''
      export HOME=$TMP
      git config --global user.email "you@example.com"
      git config --global user.name "Your Name"

      mkdir tmp
      git init tmp
      cd tmp

      git checkout -B '${branchName}'
      touch test.txt
      git add test.txt
      git commit -v -m "init"

      ${pkgs.lib.concatMapStringsSep "\n" (tag: ''
      echo '${tag}' > test.txt
      git add test.txt
      git commit -v -m 'commit for tag ${tag}'
      git tag '${tag}'
      git checkout -B 'master' # TODO remove this and tests fail (:

      ${extraCommands}
      '') tags}

      git update-server-info
      cp -r .git $out
    '';

  gitRepo = mkGitRepo {
    branchName = "test-branch";
    tags = [ "release" "0.1" "v0.2" ];
  };

  testTarball = pkgs.runCommand "test.tar" { } ''
    echo "Hello world" > foo
    tar -zcvf $out foo
  '';

  mkGitTest = { name, commands, gitRepo, repoPath ? "foo" }: pkgs.runCommand name
    {
      nativeBuildInputs = with pkgs; [ npins python3 netcat nix gitMinimal jq ];
    } ''
    set -euo pipefail
    export HOME=$TMPDIR
    export NIX_STATE_DIR=$TMPDIR
    export NIX_DATA_DIR=$TMPDIR
    export NIX_STORE_DIR=$TMPDIR
    export NIX_LOG_DIR=$TMPDIR
    cd $(mktemp -d)
    ln -s ${gitRepo} $(basename ${repoPath})
    python -m http.server 8000 &
    timeout 30 sh -c 'until nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1

    ${commands}

    touch $out
  '';

  mkGithubTest = { name, commands, gitRepo, repoPath ? "foo/bar", apiTarballs ? [ ] }: pkgs.runCommand name
    {
      nativeBuildInputs = with pkgs; [ npins python3 netcat nix gitMinimal jq ];
    } ''
    set -euo pipefail
    export HOME=$TMPDIR
    export NIX_STATE_DIR=$TMPDIR
    export NIX_DATA_DIR=$TMPDIR
    export NIX_STORE_DIR=$TMPDIR
    export NIX_LOG_DIR=$TMPDIR
    export NPINS_GITHUB_HOST=http://localhost:8000
    export NPINS_GITHUB_API_HOST=http://localhost:8000/api

    cd $(mktemp -d)

    # Mock the repository
    mkdir -p $(dirname ${repoPath})
    ln -s ${gitRepo} ${repoPath}.git

    # Mock the releases
    tarballPath="api/repos/foo/bar/tarball"
    mkdir -p $tarballPath
    ${lib.concatMapStringsSep "\n" (path: "ln -s ${testTarball} $tarballPath/${path}") apiTarballs}

    python -m http.server 8000 &
    timeout 30 sh -c 'until nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1

    ${commands}

    touch $out
  '';
in
{
  noDefaultNix = mkGitTest {
    name = "add-dry-run";
    inherit gitRepo;
    commands = ''
      npins init --bare --no-default-nix
      npins add -n git http://localhost:8000/foo -b test-branch

      test -e npins/default.nix && exit 1
      V=$(jq -r .pins npins/sources.json)
      [[ "$V" = "{}" ]]
    '';
  };


  addDryRun = mkGitTest {
    name = "add-dry-run";
    inherit gitRepo;
    commands = ''
      npins init --bare
      npins add -n git http://localhost:8000/foo -b test-branch

      V=$(jq -r .pins npins/sources.json)
      [[ "$V" = "{}" ]]
    '';
  };

  gitDependency = mkGitTest {
    name = "from-git-repo";
    inherit gitRepo;
    commands = ''
      npins init --bare
      npins add git http://localhost:8000/foo -b test-branch
      npins show

      nix-instantiate --eval npins -A foo.outPath
    '';
  };

  gitRepoEmptyFails = mkGitTest {
    name = "from-empty-git-repo";
    gitRepo = mkGitRepo {
      tags = [ ];
      branchName = "foo";
    };
    commands = ''
      npins init --bare
      ! npins add git http://localhost:8000/foo
    '';
  };

  gitTag = mkGitTest {
    name = "from-git-repo-tag";
    inherit gitRepo;
    commands = ''
      npins init --bare
      npins add git http://localhost:8000/foo
      cat npins/sources.json

      git ls-remote http://localhost:8000/foo
      nix-instantiate --eval npins -A foo.outPath

      V=$(jq -r .pins.foo.version npins/sources.json)
      [[ "$V" = "v0.2" ]]
    '';
  };

  githubRelease = mkGithubTest {
    name = "github-release";
    inherit gitRepo;
    apiTarballs = [ "v0.2" ];
    commands = ''
      npins init --bare
      npins add github foo bar
      nix-instantiate --eval npins -A bar.outPath

      V=$(jq -r .pins.bar.version npins/sources.json)
      [[ "$V" = "v0.2" ]]
    '';
  };
}
