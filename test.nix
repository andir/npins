{ system ? builtins.currentSystem
, pins ? import ./npins
, pkgs ? import pins.nixpkgs { inherit system; }
, npins ? pkgs.callPackage ./npins.nix { }
}:
let

  gitRepo = pkgs.runCommand "git-repo" { nativeBuildInputs = [ pkgs.git ]; } ''
    export HOME=$TMP
    git config --global user.email "you@example.com"
    git config --global user.name "Your Name"
    git init $out
    cd $out
    git checkout -B test-branch
    touch test.txt
    git add test.txt
    git commit -v -m "foo"
    git tag v0.1
    git update-server-info
  '';


  mkTest = name: commands: pkgs.runCommand name
    {
      nativeBuildInputs = with pkgs; [ npins python3 netcat nix gitMinimal ];
    } ''
    set -eo pipefail
    export HOME=$TMPDIR
    export NIX_STATE_DIR=$TMPDIR
    export NIX_DATA_DIR=$TMPDIR
    export NIX_STORE_DIR=$TMPDIR
    export NIX_LOG_DIR=$TMPDIR
    (cd $(mktemp -d); ln -s ${gitRepo}/.git foo && python -m http.server 8000) &
    timeout 30 sh -c 'until nc -z 127.0.0.1 8000; do sleep 1; done' || exit 1
    
    ${commands}

    touch $out
  '';
in
{
  gitDependency = mkTest "from-git-repo" ''
    npins init --bare
    npins add git http://localhost:8000/foo -b test-branch
    npins show

    nix-instantiate npins -A foo.outPath
  '';

  gitTag = mkTest "from-git-repo-tag" ''
    npins init --bare
    git ls-remote http://localhost:8000/foo
    npins add git http://localhost:8000/foo
    cat npins/sources.json

    nix-instantiate npins -A foo.outPath
  '';

}
