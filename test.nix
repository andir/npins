{
  system ? builtins.currentSystem,
  pins ? import ./npins,
  pkgs ? import pins.nixpkgs { inherit system; },
  npins ? pkgs.callPackage ./npins.nix { },
}:
let
  inherit (pkgs) lib;
  # Generate a git repository hat can be served via HTTP.
  #
  # By default the repository will contain an empty `test.txt`
  # file. For all defined tags the name of the tag is written to that
  # file for the respective commit for the tag.
  mkGitRepo =
    {
      name ? "git-repo",
      branchName ? "main",
      tags ? [ ],
      extraCommands ? "",
    }:
    pkgs.runCommand "git-repo" { nativeBuildInputs = [ pkgs.gitMinimal ]; } ''
      export HOME=$TMP
      git config --global user.email "you@example.com"
      git config --global user.name "Your Name"
      git config --global init.defaultBranch main

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
      '') tags}

      git checkout -B '${branchName}' # TODO remove this and tests fail (:
      ${extraCommands}

      git update-server-info
      cp -r .git $out
    '';

  gitRepo = mkGitRepo {
    branchName = "test-branch";
    tags = [
      "release"
      "0.1"
      "v0.2"
    ];
  };

  testTarball = pkgs.runCommand "test.tar" { } ''
    echo "Hello world" > foo
    tar -zcvf $out foo
  '';

  mkGitTest =
    {
      name,
      commands,
      # Repositories to host. key = repo path, value = repo derivation
      repositories,
    }:
    pkgs.runCommand name
      {
        nativeBuildInputs = with pkgs; [
          npins
          python3
          netcat
          nix
          gitMinimal
          jq
          nix-prefetch-git
        ];
      }
      ''
        set -euo pipefail
        export HOME=$TMPDIR
        export NIX_STATE_DIR=$TMPDIR
        export NIX_DATA_DIR=$TMPDIR
        export NIX_STORE_DIR=$TMPDIR
        export NIX_LOG_DIR=$TMPDIR

        echo -e "\n\nRunning test ${name}\n"
        cd $(mktemp -d)

        # Mock the repositories
        ${lib.pipe repositories [
          (lib.mapAttrsToList (
            repoPath: gitRepo: ''
              mkdir -p $(dirname ${repoPath})
              ln -s ${gitRepo} "${repoPath}"
            ''
          ))
          (lib.concatStringsSep "\n")
        ]}

        python -m http.server 8000 &
        timeout 30 sh -c 'set -e; until nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1

        ${commands}

        touch $out
      '';

  mkGithubTest =
    {
      name,
      commands,
      # Repositories to host. key = repo path, value = repo derivation
      repositories,
      # For simplicity, all fake releases will be added to all repositories,
      # and both as "archive" (for refs) and as "tarball" (for releases)
      apiTarballs ? [ ],
    }:
    pkgs.runCommand name
      {
        nativeBuildInputs = with pkgs; [
          npins
          python3
          netcat
          nix
          gitMinimal
          jq
        ];
      }
      ''
        set -euo pipefail
        export HOME=$TMPDIR
        export NIX_STATE_DIR=$TMPDIR
        export NIX_DATA_DIR=$TMPDIR
        export NIX_STORE_DIR=$TMPDIR
        export NIX_LOG_DIR=$TMPDIR
        export NPINS_GITHUB_HOST=http://localhost:8000
        export NPINS_GITHUB_API_HOST=http://localhost:8000/api

        echo "Running test ${name}"
        cd $(mktemp -d)

        # Mock the repositories
        ${lib.pipe repositories [
          (lib.mapAttrsToList (
            repoPath: gitRepo: ''
              mkdir -p $(dirname ${repoPath})
              ln -s ${gitRepo} "${repoPath}.git"

              # Mock the releases
              tarballPath="api/repos/${repoPath}/tarball"
              mkdir -p $tarballPath
              archivePath="${repoPath}/archive"
              mkdir -p $archivePath
              ${lib.concatMapStringsSep "\n" (path: ''
                ln -s ${testTarball} $tarballPath/${path}
                ln -s ${testTarball} $archivePath/${path}.tar.gz
              '') apiTarballs}
            ''
          ))
          (lib.concatStringsSep "\n")
        ]}

        python -m http.server 8000 &
        timeout 30 sh -c 'set -e; until nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1

        ${commands}

        touch $out
      '';
in
{
  addDryRun = mkGitTest {
    name = "add-dry-run";
    repositories."foo" = gitRepo;
    commands = ''
      npins init --bare
      npins add -n git http://localhost:8000/foo -b test-branch

      V=$(jq -r .pins npins/sources.json)
      [[ "$V" = "{}" ]]
    '';
  };

  gitDependency = mkGitTest {
    name = "from-git-repo";
    repositories."foo" = gitRepo;
    commands = ''
      npins init --bare
      npins add git http://localhost:8000/foo -b test-branch
      npins show

      nix-instantiate --eval npins -A foo.outPath

      # Check version and url
      [[ "$(jq -r .pins.foo.version npins/sources.json)" = "null" ]]
      [[ "$(jq -r .pins.foo.revision npins/sources.json)" = "b606f4ab240e230dd5916969b31a44d46e74eea1" ]]
      [[ "$(jq -r .pins.foo.url npins/sources.json)" = "null" ]]
    '';
  };

  gitRepoEmptyFails = mkGitTest {
    name = "from-empty-git-repo";
    repositories."foo" = mkGitRepo {
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
    repositories."foo" = gitRepo;
    commands = ''
      npins init --bare
      npins add git http://localhost:8000/foo

      git ls-remote http://localhost:8000/foo
      nix-instantiate --eval npins -A foo.outPath

      # Check version and url
      [[ "$(jq -r .pins.foo.version npins/sources.json)" = "v0.2" ]]
      [[ "$(jq -r .pins.foo.revision npins/sources.json)" = "b606f4ab240e230dd5916969b31a44d46e74eea1" ]]
      [[ "$(jq -r .pins.foo.url npins/sources.json)" = "null" ]]
    '';
  };

  githubRelease = mkGithubTest {
    name = "github-release";
    repositories."foo/bar" = gitRepo;
    apiTarballs = [ "v0.2" ];
    commands = ''
      npins init --bare
      npins add github foo bar
      nix-instantiate --eval npins -A bar.outPath

      # Check version and url
      [[ "$(jq -r .pins.bar.version npins/sources.json)" = "v0.2" ]]
      [[ "$(jq -r .pins.bar.revision npins/sources.json)" = "b606f4ab240e230dd5916969b31a44d46e74eea1" ]]
      [[ "$(jq -r .pins.bar.url npins/sources.json)" = "http://localhost:8000/api/repos/foo/bar/tarball/v0.2" ]]
    '';
  };

  githubBranch = mkGithubTest {
    name = "github-branch";
    repositories."foo/bar" = gitRepo;
    apiTarballs = [
      "v0.2"
      "b606f4ab240e230dd5916969b31a44d46e74eea1"
    ];
    commands = ''
      npins init --bare
      npins add github foo bar --branch test-branch
      nix-instantiate --eval npins -A bar.outPath

      # Check version and url
      [[ "$(jq -r .pins.bar.version npins/sources.json)" = "null" ]]
      [[ "$(jq -r .pins.bar.revision npins/sources.json)" = "b606f4ab240e230dd5916969b31a44d46e74eea1" ]]
      [[ "$(jq -r .pins.bar.url npins/sources.json)" = "http://localhost:8000/foo/bar/archive/b606f4ab240e230dd5916969b31a44d46e74eea1.tar.gz" ]]
    '';
  };

  gitSubmodule = mkGitTest rec {
    name = "git-submodule";
    repositories."bar" = gitRepo;
    repositories."foo" = mkGitRepo {
      name = "repo-with-submodules";
      extraCommands = ''
        git submodule init

        # In order to be able to add the submodule, we need to fake host it
        cd ..
        ${pkgs.python3}/bin/python -m http.server 8000 &
        timeout 30 sh -c 'set -e; until ${pkgs.netcat}/bin/nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1
        ln -s ${repositories.bar} "bar"
        cd tmp

        git submodule add "http://localhost:8000/bar"
      '';
    };

    commands = ''
      npins init --bare
      npins add git http://localhost:8000/foo --branch main
      npins add --name foo2 git http://localhost:8000/foo --branch main --submodules

      # Both have the same revision and no URL
      [[ "$(jq -r .pins.foo.version npins/sources.json)" = "null" ]]
      [[ "$(jq -r .pins.foo2.version npins/sources.json)" = "null" ]]
      [[ "$(jq -r .pins.foo.revision npins/sources.json)" = "cbbbea814edccc7bf23af61bd620647ed7c0a436" ]]
      [[ "$(jq -r .pins.foo2.revision npins/sources.json)" = "cbbbea814edccc7bf23af61bd620647ed7c0a436" ]]
      [[ "$(jq -r .pins.foo.url npins/sources.json)" = "null" ]]
      [[ "$(jq -r .pins.foo2.url npins/sources.json)" = "null" ]]

      nix-instantiate --eval npins -A foo.outPath
      nix-instantiate --eval npins -A foo2.outPath
    '';
  };

  githubSubmodule = mkGithubTest rec {
    name = "github-submodule";
    apiTarballs = [ "cbbbea814edccc7bf23af61bd620647ed7c0a436" ];
    repositories."owner/bar" = gitRepo;
    repositories."owner/foo" = mkGitRepo {
      name = "repo-with-submodules";
      extraCommands = ''
        git submodule init

        # In order to be able to add the submodule, we need to fake host it
        cd ..
        ${pkgs.python3}/bin/python -m http.server 8000 &
        timeout 30 sh -c 'set -e; until ${pkgs.netcat}/bin/nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1
        mkdir owner
        ln -s ${repositories."owner/bar"} "owner/bar.git"
        cd tmp

        git submodule add "http://localhost:8000/owner/bar.git"
      '';
    };

    commands = ''
      npins init --bare
      npins add github owner foo --branch main
      npins add --name foo2 github owner foo --branch main --submodules

      # Both have the same revision, but only foo has an URL
      [[ "$(jq -r .pins.foo.version npins/sources.json)" = "null" ]]
      [[ "$(jq -r .pins.foo2.version npins/sources.json)" = "null" ]]
      [[ "$(jq -r .pins.foo.revision npins/sources.json)" = "cbbbea814edccc7bf23af61bd620647ed7c0a436" ]]
      [[ "$(jq -r .pins.foo2.revision npins/sources.json)" = "cbbbea814edccc7bf23af61bd620647ed7c0a436" ]]
      [[ "$(jq -r .pins.foo.url npins/sources.json)" = "http://localhost:8000/owner/foo/archive/cbbbea814edccc7bf23af61bd620647ed7c0a436.tar.gz" ]]
      [[ "$(jq -r .pins.foo2.url npins/sources.json)" = "null" ]]
    '';
  };

  nixPrefetch =
    let
      mkPrefetchGitTest =
        name: npinsArgs:
        mkGitTest {
          name = "nix-prefetch-git-${name}";
          inherit gitRepo;
          commands = ''
            npins init --bare
            npins add git http://localhost:8000/foo ${npinsArgs}
            before=$(ls /build)

            nix-instantiate --eval npins -A foo.outPath.outPath
            after=$(ls /build)
            cat npins/sources.json

            [[ "$before" = "$after" ]]
          '';
        };
    in
    {
      branch = mkPrefetchGitTest "branch" "--branch test-branch";
      tag = mkPrefetchGitTest "tag" "--at v0.2";
      hash = mkPrefetchGitTest "hash" "--branch test-branch --at 9ba40d123c3e6adb35c99ad04fd9de6bcdc1c9d5";

      importGitFromFlake =
        let
          flake = pkgs.writeText "flake.nix" ''
            {
              inputs.foo.url = "git+http://localhost:8000/foo?ref=test-branch";
              inputs.foo.flake = false;
              outputs = _: {};
            }
          '';
        in
        mkGitTest {
          name = "from-flake-import-git";
          inherit gitRepo;
          commands = ''
            cp ${flake} flake.nix
            nix --extra-experimental-features flakes --extra-experimental-features nix-command flake update

            npins init --bare
            npins import-flake
            git ls-remote http://localhost:8000/foo
            nix-instantiate --eval npins -A foo.outPath

            cat npins/sources.json

            V=$(jq -r .pins.foo.branch npins/sources.json)
            [[ "$V" = "test-branch" ]]
          '';
        };
    };
}
