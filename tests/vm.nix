# This file contains the mocking infrastructure for all tests.
# The tests are VM tests for now, but will be converted to container tests in the future for better performance.
#
# Hosts:
# - client, on which npins and user commands runs, plus some hosted tarballs (`localhost`)
# - channels, which fakes https://channels.nixos.org
# - github, which fakes relevant https://github.com APIs
# - forgejo, running a Forgejo instance
# - registry, running a Docker registry
# Sources provided by the mocks:
# - https://channels.nixos.org/nixos-unstable (channel)
# - https://github.com/generic-org/generic-repo
# - https://github.com/owner/dependency
# - https://github.com/owner/main-repo (has github:owner/dependency as submodule)
# - http://forgejo:3000/owner/dependency
# - http://forgejo:3000/owner/main-repo (has forgejo:owner/dependency as submodule)
# - registry:5000/hello-world:latest (container image)
# - http://localhost/untagged-repo.git
# - http://localhost/tagged-repo.git
# - http://localhost/submoduled-repo.git (has localhost:tagged-repo as submodule)
# - http://localhost/testTarball
# - http://localhost/latest (redirects to /testTarball)
{
  lib,
  runCommand,
  openssl,
  gitMinimal,
  dockerTools,
  npins,
}:
let
  # Note: This derivation is not bit-reproducible if fetched as a url
  testTarball = runCommand "test.tar" { } ''
    echo "Hello world" > foo
    tar -cf $out foo
  '';

  testImage = dockerTools.buildImage {
    name = "hello-world";
    tag = "latest";
    compressor = "none"; # Save RAM in the VM
    config.Cmd = [ "/hello" ];
  };

  # git repository with a few release tags
  dependencyRepo = mkGitRepo {
    name = "dependency-repo";
    branchName = "test-branch";
    tags = [
      "release"
      "0.1"
      "v0.2"
    ];
  };

  # The repositories hosted on the mock GitHub host
  githubRepos = {
    "generic-org/generic-repo" = mkGitRepo {
      branchName = "test-branch";
      tags = [
        "release"
        "0.1"
        "v0.2"
      ];
    };
    "owner/dependency" = dependencyRepo;
    "owner/main-repo" = mkGitRepo {
      name = "github-repo-with-submodule";
      submodules = [
        {
          repo = dependencyRepo;
          url = "https://github.com/owner/dependency.git";
        }
      ];
      tags = [ "v0.5" ];
    };
  };

  # The repositories hosted on the Forgejo host
  forgejoRepos = {
    "owner/dependency" = dependencyRepo;
    "owner/main-repo" = mkGitRepo {
      name = "forgejo-repo-with-submodule";
      submodules = [
        {
          repo = dependencyRepo;
          url = "http://forgejo:3000/owner/dependency.git";
        }
      ];
      tags = [ "v0.5" ];
    };
  };

  # Repositories hosted on the client itself, reachable as http://localhost/<name>.git
  localRepos = {
    # A repo with no tags or releases
    "untagged-repo.git" = mkGitRepo {
      tags = [ ];
      branchName = "foo";
    };
    "tagged-repo.git" = dependencyRepo;
    "submoduled-repo.git" = mkGitRepo {
      name = "local-repo-with-submodule";
      submodules = [
        {
          repo = dependencyRepo;
          url = "http://localhost/tagged-repo.git";
        }
      ];
      tags = [ "v0.5" ];
    };
  };

  # The locked URL behind the nixpkgs-unstable channel of the mock channels host
  channelReleasePath = "/nixos/unstable/nixos-26.11pre123456.abcdef/nixexprs.tar.xz";
  channelReleaseUrl = "https://releases.nixos.org${channelReleasePath}";

  # Fake self-signed TLS certificates.
  # Contains `ca.pem`, `cert.pem` and `key.pem`
  mockCert =
    let
      domains = [
        "channels.nixos.org"
        "releases.nixos.org"
        "github.com"
        "api.github.com"
        "registry"
      ];
    in
    runCommand "npins-test-certificates" { nativeBuildInputs = [ openssl ]; } ''
      mkdir $out
      # Our fake CA
      openssl req -x509 -newkey rsa:2048 -nodes -days 36500 \
        -subj "/CN=npins test CA" \
        -keyout ca.key -out $out/ca.pem
      # Server certificates (we do all-in-one for all domains)
      openssl req -newkey rsa:2048 -nodes \
        -subj "/CN=npins test cert" \
        -addext "subjectAltName=DNS:${lib.concatStringsSep ",DNS:" domains}" \
        -keyout $out/key.pem -out server.csr
      # Sign the server certificate with our CA
      openssl x509 -req -in server.csr -CA $out/ca.pem -CAkey ca.key \
        -days 36500 -copy_extensions copyall -out $out/cert.pem
    '';

  # Generate a git repository that can be served via HTTP.
  #
  # By default the repository will contain an empty `test.txt` file.
  # For all defined tags the name of the tag is written to that file for the respective commit for the tag.
  mkGitRepo =
    {
      name ? "git-repo",
      branchName ? "main",
      tags ? [ ],
      # List of `{ repo, url }`
      submodules ? [ ],
      extraCommands ? "",
    }:
    runCommand name { nativeBuildInputs = [ gitMinimal ]; } ''
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

      ${lib.optionalString (submodules != [ ]) ''
        git config --global --add safe.directory '*'
        git config --global protocol.file.allow always
      ''}
      ${lib.concatMapStringsSep "\n" (submodule: ''
        # Add the submodule with its "real" URL without network access
        git config --global url."${submodule.repo}".insteadOf "${submodule.url}"
        git submodule add '${submodule.url}'
        git commit -m 'Add submodule ${submodule.url}'
      '') submodules}

      ${lib.concatMapStringsSep "\n" (tag: ''
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

  # Magic incantation from the Internetz that hosts a git server (smart git protocol) from nginx via fastcgi/fcgiwrap bridge
  gitHttpBackend = config: repo: ''
    fastcgi_pass unix:${config.services.fcgiwrap.instances.git.socket.address};
    fastcgi_param SCRIPT_FILENAME ${gitMinimal}/libexec/git-core/git-http-backend;
    fastcgi_param GIT_PROJECT_ROOT ${repo};
    fastcgi_param GIT_HTTP_EXPORT_ALL "";
    # The repositories in the store are not owned by the fcgiwrap user
    fastcgi_param GIT_CONFIG_COUNT 1;
    fastcgi_param GIT_CONFIG_KEY_0 safe.directory;
    fastcgi_param GIT_CONFIG_VALUE_0 "*";
    fastcgi_param PATH_INFO $uri;
    fastcgi_param QUERY_STRING $args;
    fastcgi_param REQUEST_METHOD $request_method;
    fastcgi_param CONTENT_TYPE $content_type;
    fastcgi_param CONTENT_LENGTH $content_length;
  '';
in
{
  # A fake channels.nixos.org host, which also serves releases.nixos.org
  channels = {
    networking.firewall.allowedTCPPorts = [
      80
      443
    ];
    services.nginx = {
      enable = true;
      virtualHosts."channels.nixos.org" = {
        addSSL = true;
        sslCertificate = "${mockCert}/cert.pem";
        sslCertificateKey = "${mockCert}/key.pem";
        locations."= /nixos-unstable/nixexprs.tar.xz".return = "302 ${channelReleaseUrl}";
      };
      virtualHosts."releases.nixos.org" = {
        addSSL = true;
        sslCertificate = "${mockCert}/cert.pem";
        sslCertificateKey = "${mockCert}/key.pem";
        locations."= ${channelReleasePath}".alias = "${testTarball}";
      };
    };
  };

  # A fake github.com host, which also serves api.github.com
  github =
    (import ./vm-github.nix {
      inherit
        lib
        runCommand
        gitMinimal
        mockCert
        gitHttpBackend
        ;
    })
      githubRepos;

  # TODO test GitLab
  # gitlab = { };

  # A container registry, like docker.io.
  # Reachable as `registry:5000` (with TLS, because skopeo insists on it for non-localhost hosts)
  registry =
    { pkgs, ... }:
    {
      services.dockerRegistry = {
        enable = true;
        listenAddress = "0.0.0.0";
        port = 5000;
        openFirewall = true;
        extraConfig.http.tls = {
          certificate = "${mockCert}/cert.pem";
          key = "${mockCert}/key.pem";
        };
      };
      # Of course it comes with telemetry enabled by default, which doesn't work in the sandbox …
      systemd.services.docker-registry.environment.OTEL_TRACES_EXPORTER = "none";
      security.pki.certificateFiles = [ "${mockCert}/ca.pem" ];
      # Oneshot service that uploads the image to the registry on boot
      systemd.services.registry-provision = {
        wantedBy = [ "multi-user.target" ];
        requires = [ "docker-registry.service" ];
        after = [ "docker-registry.service" ];
        path = [
          pkgs.skopeo
          pkgs.curl
        ];
        serviceConfig = {
          Type = "oneshot";
          RemainAfterExit = true;
          TimeoutStartSec = "5min";
        };
        script = ''
          # Wait for the registry to accept requests (is there a prettier way?)
          until curl -sSf https://registry:5000/v2/ > /dev/null; do sleep 1; done
          skopeo --insecure-policy copy docker-archive:${testImage} docker://registry:5000/hello-world:latest
        '';
      };
    };

  # A real Forgejo instance, via the NixOS module
  forgejo = (import ./vm-forgejo.nix { inherit lib; }) forgejoRepos;

  # The machine on which npins runs
  client =
    {
      nodes,
      containers,
      config,
      pkgs,
      ...
    }:
    {
      environment.systemPackages = [
        npins
        pkgs.gitMinimal
        pkgs.nix-prefetch-git
        pkgs.curl
        pkgs.niv
        pkgs.lon
      ];
      nix.package = pkgs.lix;
      nix.nixPath = [ "nixpkgs=${pkgs.path}" ]; # for testing pin eval with pkgs
      # There is no internet access, so don't let Nix waste time on trying to reach cache.nixos.org every time
      nix.settings.substituters = lib.mkForce [ ];
      # virtualisation.memorySize = 4096; # for Nixpkgs eval
      security.pki.certificateFiles = [ "${mockCert}/ca.pem" ];
      networking.extraHosts = ''
        ${
          (nodes.channels or containers.channels).networking.primaryIPAddress
        } channels.nixos.org releases.nixos.org
        ${(nodes.github or containers.github).networking.primaryIPAddress} github.com api.github.com
      '';

      # The containers set the remote to "daemon" which is wrong, just hard-code a store in a random directory for everything.
      environment.variables."NIX_REMOTE" = lib.mkForce "/nix-test-store";

      # Serve `localRepos` and the test tarball to ourselves on http://localhost
      services.fcgiwrap.instances.git = {
        socket.user = "nginx";
        socket.group = "nginx";
      };

      services.nginx = {
        enable = true;
        virtualHosts."localhost" = {
          locations."= /testTarball".alias = "${testTarball}";
          # A mutable URL that redirects to an immutable snapshot, like the channels do
          locations."= /latest".return = "302 http://localhost/testTarball";
          # The git repositories
          locations."~ ^/[^/]+\\.git/".extraConfig =
            let
              # The git repositories served by the client to itself
              localGitRepos = runCommand "local-git-repos" { } ''
                mkdir $out
                ${lib.pipe localRepos [
                  (lib.mapAttrsToList (path: repo: "ln -s ${repo} $out/${path}"))
                  (lib.concatStringsSep "\n")
                ]}
              '';
            in
            gitHttpBackend config "${localGitRepos}";
        };
      };
    };
}
