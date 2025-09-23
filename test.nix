{
  system ? builtins.currentSystem,
  pins ? import ./npins,
  pkgs ? import pins.nixpkgs { inherit system; },
  npins ? pkgs.callPackage ./npins.nix { },
}:
let
  # utility bash functions used throught the tests
  prelude = pkgs.writeShellScript "prelude" ''
    export HOME=$TMPDIR
    export NIX_STATE_DIR=$TMPDIR
    export NIX_DATA_DIR=$TMPDIR
    export NIX_STORE_DIR=$TMPDIR
    export NIX_LOG_DIR=$TMPDIR

    function eq() {
      local a=$1
      local b=$2
      printf '[[ "%s" = "%s" ]]' "$a" "$b"
      if [[ "$a" = "$b" ]]; then echo " OK"; else echo " FAIL"; exit 1; fi
    }

    function neq() {
      local a=$1
      local b=$2
      printf '[[ "%s" != "%s" ]]' "$a" "$b"
      if [[ "$a" != "$b" ]]; then echo " OK"; else echo " FAIL"; exit 1; fi
    }

    function resolveGitCommit() {
      local repo=$1
      local commitish=''${2:-main}
      git  -C $repo rev-list  -n1 $commitish
    }
  '';

  inherit (pkgs) lib;

  # Generate a git repository that can be served via HTTP.
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
      export GIT_AUTHOR_DATE="1970-01-01 00:00:00 +0000"
      export GIT_COMMITTER_DATE="1970-01-01 00:00:00 +0000"
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

  # Use git-http-backend CGI to serve git repo via http:// for shallow clone capabilities
  gitServe = pkgs.writers.writePython3Bin "git-serve" { } ''
    import os
    from http.server import CGIHTTPRequestHandler, test

    os.environ["GIT_HTTP_EXPORT_ALL"] = "1"


    class GitHandler(CGIHTTPRequestHandler):
        have_fork = False

        def is_cgi(self):
            self.cgi_info = "${pkgs.gitMinimal}/libexec/git-core", "git-http-backend/" + self.path  # noqa: E501
            if "/archive/" in self.path or "/api/" in self.path:
                return False
            return True

        def translate_path(self, path):
            if path.endswith("git-http-backend"):
                return path
            return CGIHTTPRequestHandler.translate_path(self, path)


    if __name__ == "__main__":
        test(GitHandler)
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
          lix
          gitMinimal
          jq
          nix-prefetch-git
        ];
      }
      ''
        set -euo pipefail
        source ${prelude}

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
        # Mark repos as safe for usage with Git cli
        ${lib.pipe repositories [
          (lib.mapAttrsToList (_: gitRepo: "git config --global --add safe.directory ${gitRepo}"))
          (lib.concatStringsSep "\n")
        ]}

        ${gitServe}/bin/git-serve &
        timeout 30 sh -c 'set -e; until nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1

        ${commands}

        touch $out
      '';

  mkForgejoTest =
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
          lix
          gitMinimal
          jq
        ];
      }
      ''
        set -euo pipefail
        source ${prelude}

        echo "Running test ${name}"
        cd $(mktemp -d)

        # Mock the repositories
        ${lib.pipe repositories [
          (lib.mapAttrsToList (
            repoPath: gitRepo: ''
              mkdir -p $(dirname ${repoPath})
              ln -s ${gitRepo} "${repoPath}.git"

              # Mock the releases
              tarballPath="api/v1/repos/${repoPath}/archive"
              mkdir -p $tarballPath
              archivePath="${repoPath}/archive"
              mkdir -p $archivePath
              ${lib.concatMapStringsSep "\n" (path: ''
                ln -s ${testTarball} $tarballPath/${path}.tar.gz
                ln -s ${testTarball} $archivePath/${path}
              '') apiTarballs}

              chmod -R +rw $archivePath
              chmod -R +rw $tarballPath
              pwd
              ls -la $tarballPath
              # For each of the commits in the repo create the tarballs
              git config --global --add safe.directory ${gitRepo}
              echo $(git -C ${gitRepo} log --oneline --format="format:%H")
              git -C ${gitRepo} log --oneline --format="format:%H" | xargs -I XX -n1 git -C ${gitRepo} archive -o $PWD/$tarballPath/XX.tar.gz XX
              git -C ${gitRepo} log --oneline --format="format:%H" | xargs -I XX -n1 git -C ${gitRepo} archive -o $PWD/$archivePath/XX.tar.gz XX
            ''
          ))
          (lib.concatStringsSep "\n")
        ]}

        ${gitServe}/bin/git-serve &
        timeout 30 sh -c 'set -e; until nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1

        ${commands}

        touch $out
      '';

  mkTarballTest =
    {
      name,
      commands,
      tarballs,
      immutableLinks ? { },
    }:
    pkgs.runCommand name
      {
        nativeBuildInputs = with pkgs; [
          npins
          python3
          netcat
          lix
          jq
        ];
      }
      ''
        set -euo pipefail
        source ${prelude}

        echo -e "\n\nRunning test ${name}\n"
        cd $(mktemp -d)

        # Create tarballs
        ${lib.pipe tarballs [
          (builtins.map (path: ''
            mkdir -p $(dirname ${path})
            ln -s ${testTarball} ${path}.tar.gz
          ''))
          (lib.concatStringsSep "\n")
        ]}

        python ${pkgs.writeText "mock_server.py" ''
          import http.server
          import socketserver

          PORT = 8000
          LINK_MAP = {
            ${
              lib.pipe immutableLinks [
                (lib.mapAttrsToList (path: flakeref: ''"${path}": "${flakeref}",''))
                (lib.concatStringsSep "\n")
              ]
            }
          }

          class Handler(http.server.SimpleHTTPRequestHandler):
              def end_headers(self):
                  if self.path in LINK_MAP:
                    self.send_header("Link", f'<{LINK_MAP[self.path]}>; rel="immutable"')
                  super().end_headers()

          with socketserver.TCPServer(("", PORT), Handler) as httpd:
              httpd.serve_forever()
        ''} &
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
          lix
          gitMinimal
          jq
        ];
      }
      ''
        set -euo pipefail
        export NPINS_GITHUB_HOST=http://localhost:8000
        export NPINS_GITHUB_API_HOST=http://localhost:8000/api
        source ${prelude}

        echo "Running test ${name}"
        cd $(mktemp -d)

        # Mock the repositories
        ${lib.pipe repositories [
          (lib.mapAttrsToList (
            repoPath: gitRepo: ''
              mkdir -p $(dirname ${repoPath})
              ln -s ${gitRepo} "${repoPath}.git"

              # Mock the releases
              tarballPath="api/repos/${repoPath}/tarball/refs/tags"
              mkdir -p $tarballPath
              archivePath="${repoPath}/archive"
              mkdir -p $archivePath
              ${lib.concatMapStringsSep "\n" (path: ''
                ln -s ${testTarball} $tarballPath/${path}
                ln -s ${testTarball} $archivePath/${path}.tar.gz
              '') apiTarballs}

              chmod -R +rw $archivePath
              chmod -R +rw $tarballPath
              pwd
              ls -la $tarballPath
              # For each of the commits in the repo create the tarballs
              git config --global --add safe.directory ${gitRepo}
              git -C ${gitRepo} log --oneline --format="format:%H" | xargs -I XX -n1 git -C ${gitRepo} archive -o $PWD/$tarballPath/XX XX
              git -C ${gitRepo} log --oneline --format="format:%H" | xargs -I XX -n1 git -C ${gitRepo} archive -o $PWD/$archivePath/XX.tar.gz XX
            ''
          ))
          (lib.concatStringsSep "\n")
        ]}

        ${gitServe}/bin/git-serve &
        timeout 30 sh -c 'set -e; until nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1

        ${commands}

        touch $out
      '';

  mkContainerTest =
    {
      name,
      images,
      commands,
    }:
    let
      distributionConfig = pkgs.writeText "config.yaml" ''
        version: 0.1
        storage:
            delete:
              enabled: true
            cache:
                blobdescriptor: inmemory
            filesystem:
                rootdirectory: "./store/"
        http:
            addr: :5000
            tls:
                certificate: ${./tests/assets/cert.pem}
                key: ${./tests/assets/key.pem}
      '';
    in
    pkgs.runCommand name
      {
        nativeBuildInputs = with pkgs; [
          npins
          netcat
          lix
          gitMinimal
          jq
          distribution
          crane
        ];
      }
      ''
        set -euo pipefail
        source ${prelude}
        export SSL_CERT_FILE=${./tests/assets/cert.pem}

        echo "Running test ${name}"
        cd $(mktemp -d)

        registry serve ${distributionConfig} &
        timeout 30 sh -c 'set -e; until nc -z 127.0.0.1 5000; do sleep 1; done' || exit 1
        ${lib.pipe images [
          (lib.mapAttrsToList (
            imageName: imagePath: ''
              crane push --insecure ${imagePath} localhost:5000/${imageName}
            ''
          ))
          (lib.concatStringsSep "\n")
        ]}

        ${commands}

        touch $out
      '';

  mkPrefetchGitTest =
    name: npinsArgs:
    mkGitTest {
      name = "nix-prefetch-git-${name}";
      repositories."foo" = gitRepo;
      commands = ''
        npins init --bare
        npins add git http://localhost:8000/foo ${npinsArgs}
        before=$(ls /build)

        nix-instantiate --eval npins -A foo.outPath
        after=$(ls /build)
        cat npins/sources.json

        [[ "$before" = "$after" ]]
      '';
    };
in
{
  initNoDefaultNix = mkGitTest {
    name = "init-no-default-nix";
    repositories."foo" = gitRepo;
    commands = ''
      npins --lock-file sources.json init --bare
      # Setting a custom directory should fail in lockfile mode
      ! npins --lock-file sources.json -d npins2 show
      npins --lock-file sources.json -d npins show
      test -e npins/default.nix && exit 1
      V=$(jq -r .pins sources.json)
      [[ "$V" = "{}" ]]
    '';
  };

  addInLockfileMode = mkGitTest rec {
    name = "add-in-lockfile-mode";
    repositories."foo" = gitRepo;
    commands = ''
      npins --lock-file sources2.json init --bare
      npins --lock-file sources2.json add git http://localhost:8000/foo -b test-branch

      # Check version and url
      eq "$(jq -r .pins.foo.version sources2.json)" "null"
      eq "$(jq -r .pins.foo.revision sources2.json)" "$(resolveGitCommit ${repositories."foo"} HEAD)"
      eq "$(jq -r .pins.foo.url sources2.json)" "null"

      # Check setting the directory in normal mode still works
      npins -d testing init --bare
      NPINS_DIRECTORY=testing npins show
    '';
  };

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

  gitDependency = mkGitTest rec {
    name = "git-dependency";
    repositories."foo" = gitRepo;
    commands = ''
      npins init --bare
      npins add git http://localhost:8000/foo -b test-branch
      npins show

      nix-instantiate --eval npins -A foo.outPath

      # Check version and url
      eq "$(jq -r .pins.foo.version npins/sources.json)" "null"
      eq "$(jq -r .pins.foo.revision npins/sources.json)" "$(resolveGitCommit ${repositories."foo"} HEAD)"
      eq "$(jq -r .pins.foo.url npins/sources.json)" "null"
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

  gitTag = mkGitTest rec {
    name = "from-git-repo-tag";
    repositories."foo" = gitRepo;
    commands = ''
      npins init --bare
      npins add git http://localhost:8000/foo

      git ls-remote http://localhost:8000/foo
      nix-instantiate --eval npins -A foo.outPath

      # Check version and url
      eq "$(jq -r .pins.foo.version npins/sources.json)" "v0.2"
      eq "$(jq -r .pins.foo.revision npins/sources.json)" "$(resolveGitCommit ${repositories."foo"} HEAD)"
      eq "$(jq -r .pins.foo.url npins/sources.json)" "null"
    '';
  };

  # maybe test using forgejo? https://github.com/NixOS/nixpkgs/blob/master/nixos/tests/forgejo.nix
  forgejoRelease = mkForgejoTest {
    name = "forgejo-release";
    repositories."foo/bar" = gitRepo;
    apiTarballs = [ "v0.2" ];
    commands = ''
      npins init --bare
      npins add forgejo http://localhost:8000 foo bar
      nix-instantiate --eval npins -A bar.outPath

      # Check version and url
      eq "$(jq -r .pins.bar.version npins/sources.json)" "v0.2"
      eq "$(jq -r .pins.bar.revision npins/sources.json)" "$(resolveGitCommit ${gitRepo} v0.2)"
      eq "$(jq -r .pins.bar.url npins/sources.json)" "http://localhost:8000/api/v1/repos/foo/bar/archive/v0.2.tar.gz"
    '';
  };

  forgejoBranch = mkForgejoTest {
    name = "forgejo-branch";
    repositories."foo/bar" = gitRepo;
    apiTarballs = [ "v0.2" ];
    commands = ''
      npins init --bare
      npins add forgejo http://localhost:8000 foo bar --branch test-branch
      nix-instantiate --eval npins -A bar.outPath

      # Check version and url
      eq "$(jq -r .pins.bar.version npins/sources.json)" "null"
      eq "$(jq -r .pins.bar.revision npins/sources.json)" "$(resolveGitCommit ${gitRepo} test-branch)"
      eq "$(jq -r .pins.bar.url npins/sources.json)" "http://localhost:8000/foo/bar/archive/$(resolveGitCommit ${gitRepo} test-branch).tar.gz"
    '';
  };

  forgejoSubmodule = mkForgejoTest rec {
    name = "forgejo-submodule";
    apiTarballs = [ "cbbbea814edccc7bf23af61bd620647ed7c0a436" ];
    repositories."owner/bar" = gitRepo;
    repositories."owner/foo" = mkGitRepo {
      name = "repo-with-submodules";
      extraCommands = ''
        git submodule init

        # In order to be able to add the submodule, we need to fake host it
        cd ..
        ${gitServe}/bin/git-serve &
        timeout 30 sh -c 'set -e; until ${pkgs.netcat}/bin/nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1
        mkdir owner
        ln -s ${repositories."owner/bar"} "owner/bar.git"
        cd tmp

        git submodule add "http://localhost:8000/owner/bar.git"
      '';
    };

    commands = ''
      npins init --bare
      npins add forgejo http://localhost:8000 owner foo --branch main
      npins add --name foo2 forgejo http://localhost:8000 owner foo --branch main --submodules

      # Both have the same revision, but only foo has an URL
      eq "$(jq -r .pins.foo.version npins/sources.json)" "null"
      eq "$(jq -r .pins.foo2.version npins/sources.json)" "null"
      eq "$(jq -r .pins.foo.revision npins/sources.json)" "$(resolveGitCommit ${repositories."owner/foo"})"
      eq "$(jq -r .pins.foo2.revision npins/sources.json)" "$(resolveGitCommit ${repositories."owner/foo"})"
      eq "$(jq -r .pins.foo.url npins/sources.json)" "http://localhost:8000/owner/foo/archive/$(resolveGitCommit ${repositories."owner/foo"}).tar.gz"
      eq "$(jq -r .pins.foo2.url npins/sources.json)" "null"
    '';
  };

  tarballLockable = mkTarballTest {
    name = "tarball-lockable";
    tarballs = [
      "foo/bar/baz"
      "locked/baz"
    ];
    immutableLinks = {
      "/foo/bar/baz.tar.gz" = "http://localhost:8000/locked/baz.tar.gz";
    };
    commands = ''
      npins init --bare
      npins add tarball --name bar http://localhost:8000/foo/bar/baz.tar.gz
      nix-instantiate --eval npins -A bar.outPath

      eq "$(jq -r .pins.bar.url npins/sources.json)" "http://localhost:8000/foo/bar/baz.tar.gz"
      eq "$(jq -r .pins.bar.locked_url npins/sources.json)" "http://localhost:8000/locked/baz.tar.gz"

      # make sure update is idempotent
      npins update bar

      eq "$(jq -r .pins.bar.url npins/sources.json)" "http://localhost:8000/foo/bar/baz.tar.gz"
      eq "$(jq -r .pins.bar.locked_url npins/sources.json)" "http://localhost:8000/locked/baz.tar.gz"
    '';
  };

  tarballNotLockable = mkTarballTest {
    name = "tarball-not-lockable";
    tarballs = [ "foo/bar/baz" ];
    commands = ''
      npins init --bare
      npins add tarball --name bar http://localhost:8000/foo/bar/baz.tar.gz
      nix-instantiate --eval npins -A bar.outPath
      jq .pins.bar npins/sources.json

      eq "$(jq -r .pins.bar.url npins/sources.json)" "http://localhost:8000/foo/bar/baz.tar.gz"
      eq "$(jq -r .pins.bar.locked_url npins/sources.json)" "null"

      # make sure update is idempotent
      npins update bar
      jq .pins.bar npins/sources.json

      eq "$(jq -r .pins.bar.url npins/sources.json)" "http://localhost:8000/foo/bar/baz.tar.gz"
      eq "$(jq -r .pins.bar.locked_url npins/sources.json)" "null"
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
      eq "$(jq -r .pins.bar.version npins/sources.json)" "v0.2"
      eq "$(jq -r .pins.bar.revision npins/sources.json)" "$(resolveGitCommit ${gitRepo} v0.2)"
      eq "$(jq -r .pins.bar.url npins/sources.json)" "http://localhost:8000/api/repos/foo/bar/tarball/refs/tags/v0.2"
    '';
  };

  githubBranch = mkGithubTest {
    name = "github-branch";
    repositories."foo/bar" = gitRepo;
    apiTarballs = [ "v0.2" ];
    commands = ''
      npins init --bare
      npins add github foo bar --branch test-branch
      nix-instantiate --eval npins -A bar.outPath

      # Check version and url
      eq "$(jq -r .pins.bar.version npins/sources.json)" "null"
      eq "$(jq -r .pins.bar.revision npins/sources.json)" "$(resolveGitCommit ${gitRepo} test-branch)"
      eq "$(jq -r .pins.bar.url npins/sources.json)" "http://localhost:8000/foo/bar/archive/$(resolveGitCommit ${gitRepo} test-branch).tar.gz"
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
        ${gitServe}/bin/git-serve &
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
      eq "$(jq -r .pins.foo.version npins/sources.json)" "null"
      eq "$(jq -r .pins.foo2.version npins/sources.json)" "null"
      eq "$(jq -r .pins.foo.revision npins/sources.json)" "$(resolveGitCommit ${repositories."foo"})"
      eq "$(jq -r .pins.foo2.revision npins/sources.json)" "$(resolveGitCommit ${repositories."foo"})"
      eq "$(jq -r .pins.foo.url npins/sources.json)" "null"
      eq "$(jq -r .pins.foo2.url npins/sources.json)" "null"

      nix-instantiate --eval npins -A foo.outPath
      nix-instantiate --eval npins -A foo2.outPath
    '';
  };

  container = mkContainerTest {
    name = "container";
    images."hello-world" = ./tests/assets/hello-world-image;
    commands = ''
      npins init --bare
      npins add container --name hello_world localhost:5000/hello-world latest

      eq "$(jq -r .pins.hello_world.image_name npins/sources.json)" "localhost:5000/hello-world"
      eq "$(jq -r .pins.hello_world.image_tag npins/sources.json)" "latest"

      nix-instantiate --eval --expr "((import ./npins).hello_world)"
    '';
  };

  githubSubmoduleFromRelease = mkGithubTest rec {
    name = "github-submodule-from-release";
    apiTarballs = [ "v0.5" ];
    repositories."owner/bar" = gitRepo;
    repositories."owner/foo" = mkGitRepo {
      name = "repo-with-submodules";
      tags = [ "v0.5" ];
      extraCommands = ''
        git submodule init

        # In order to be able to add the submodule, we need to fake host it
        cd ..
        ${gitServe}/bin/git-serve &
        timeout 30 sh -c 'set -e; until ${pkgs.netcat}/bin/nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1
        mkdir owner
        ln -s ${repositories."owner/bar"} "owner/bar.git"
        cd tmp

        git submodule add "http://localhost:8000/owner/bar.git"
      '';
    };

    commands = ''
      npins init --bare
      npins add github owner foo
      npins add --name foo2 github owner foo --submodules

      cat npins/sources.json

      # Both have the same revision, but only foo has an URL
      eq "$(jq -r .pins.foo.version npins/sources.json)" "v0.5"
      eq "$(jq -r .pins.foo2.version npins/sources.json)" "v0.5"
      eq "$(jq -r .pins.foo.revision npins/sources.json)" "$(resolveGitCommit ${repositories."owner/foo"})"
      eq "$(jq -r .pins.foo2.revision npins/sources.json)" "$(resolveGitCommit ${repositories."owner/foo"})"
      eq "$(jq -r .pins.foo.url npins/sources.json)" "http://localhost:8000/api/repos/owner/foo/tarball/refs/tags/v0.5"
      # release pins with submodules don't have a URL
      eq "$(jq -r .pins.foo2.url npins/sources.json)" "null"
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
        ${gitServe}/bin/git-serve &
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
      eq "$(jq -r .pins.foo.version npins/sources.json)" "null"
      eq "$(jq -r .pins.foo2.version npins/sources.json)" "null"
      eq "$(jq -r .pins.foo.revision npins/sources.json)" "$(resolveGitCommit ${repositories."owner/foo"})"
      eq "$(jq -r .pins.foo2.revision npins/sources.json)" "$(resolveGitCommit ${repositories."owner/foo"})"
      eq "$(jq -r .pins.foo.url npins/sources.json)" "http://localhost:8000/owner/foo/archive/$(resolveGitCommit ${repositories."owner/foo"}).tar.gz"
      eq "$(jq -r .pins.foo2.url npins/sources.json)" "null"
    '';
  };

  nixPrefetchBranch = mkPrefetchGitTest "branch" "--branch test-branch";
  nixPrefetchTag = mkPrefetchGitTest "tag" "--at v0.2";
  nixPrefetchHash = mkPrefetchGitTest "hash" "--branch test-branch --at 81289a3c12d4f528d27794b9e47f4ff5cf534a88";

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
      repositories."foo" = gitRepo;
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

  gitDependencyOverride = mkGitTest rec {
    name = "git-dependency-override";
    repositories."foo" = gitRepo;
    commands = ''
      npins init --bare
      npins add git http://localhost:8000/foo -b test-branch
      npins show

      OUTPATH=$(NPINS_OVERRIDE_foo=/foo_overriden nix-instantiate --eval npins -A foo.outPath --impure)
      eq "$OUTPATH" "/foo_overriden"

      OUTPATH=$(nix-instantiate --eval npins -A foo.outPath)
      neq "$OUTPATH" "/foo_overriden"
    '';
  };

  # https://github.com/andir/npins/issues/75
  regression_issue75 = mkGitTest rec {
    name = "regression-issue-75";
    repositories."foo" = gitRepo;
    commands = ''
      npins init --bare
      ! npins add git http://localhost:8000/foo --branch test-branch --at v0.2
      npins add git http://localhost:8000/foo --at v0.2
      nix-instantiate --eval npins -A foo.outPath
    '';
  };

  getPath = mkGitTest rec {
    name = "get-path";
    repositories."foo" = gitRepo;
    commands = ''
      npins init --bare
      npins add git http://localhost:8000/foo -b test-branch
      npins show
      set +x

      eq "$(nix-instantiate --eval npins -A foo.outPath)" "\"$(npins get-path foo)\""
    '';
  };
}
