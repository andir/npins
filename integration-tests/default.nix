{ system ? builtins.currentSystem
, pins ? import ../npins
, pkgs ? import pins.nixpkgs { inherit system; }
, npins ? pkgs.callPackage ../npins.nix { }
}:
{
  gitRepoBranch =
    let
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
    pkgs.runCommand "git-repo" { nativeBuildInputs = [ npins pkgs.python3 ]; } ''
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
