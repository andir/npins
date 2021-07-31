let
  data = builtins.fromJSON (builtins.readFile ./pins.json);

  mkSource = spec:
    assert spec ? type;
    if spec.type == "GitHub" then mkGitHubSource spec
    else builtins.throw "Unknown source type ${spec.type}";

  mkGitHubSource = spec:
    let
      url = with spec; "https://github.com/${owner}/${repository}/archive/${revision}.tar.gz";
      path = (builtins.fetchTarball {
        inherit url;
        sha256 = spec.hash; # FIXME: check nix version & use SRI hashes
      });
    in
    spec // { outPath = path; }
  ;

in
builtins.mapAttrs (_: mkSource) data.pins
