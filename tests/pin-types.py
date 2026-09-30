# Test all pin types in here

import re
from dataclasses import dataclass, field

# For each pin type, and each pin type feature, test:
# - add
# - show
# - eval
# - eval with Nixpkgs
# - get-path
# - NPINS_OVERRIDE
# - update (TODO albeit not super thoroughly: this does not fully test partial vs full updates, all kinds of hash mismatches etc.)
with npins_subtest("pin types"):
    succeed("npins init --bare")

    @dataclass
    class PinTestCase:
        name: str
        # The name the pin will have after adding
        attr_name: str
        # Args to execute on `npins add`
        add_args: str
        # Assert some keys on the pin in the `sources.json`.
        # `None` unfortunately means "absent or null", but it'll do.
        result: dict[str, Any] = field(default_factory=dict)
        # Patch the pin with fake data to perform a "downgrade",
        # then check that `npins update` does the correct thing
        downgrade: dict[str, Any] | None = None

    # Fetch some revisions of our test repos for later use
    untagged_head = ls_remote("http://localhost/untagged-repo.git", "refs/heads/foo")
    tagged_v0_2 = ls_remote("http://localhost/tagged-repo.git", "refs/tags/v0.2")
    submoduled_v0_5 = ls_remote("http://localhost/submoduled-repo.git", "refs/tags/v0.5")
    generic_repo_head = ls_remote("https://github.com/generic-org/generic-repo.git", "refs/heads/test-branch")
    dependency_v0_2 = ls_remote("https://github.com/owner/dependency.git", "refs/tags/v0.2")
    main_repo_head = ls_remote("https://github.com/owner/main-repo.git", "refs/heads/main")
    main_repo_v0_5 = ls_remote("https://github.com/owner/main-repo.git", "refs/tags/v0.5")
    forgejo_dep_head = ls_remote("http://forgejo:3000/owner/dependency.git", "refs/heads/test-branch")
    forgejo_dep_v0_2 = ls_remote("http://forgejo:3000/owner/dependency.git", "refs/tags/v0.2")
    forgejo_main_head = ls_remote("http://forgejo:3000/owner/main-repo.git", "refs/heads/main")
    forgejo_main_v0_5 = ls_remote("http://forgejo:3000/owner/main-repo.git", "refs/tags/v0.5")

    test_cases = [
        PinTestCase(
            "channel",
            "nixpkgs-unstable",
            "channel nixpkgs-unstable",
            {
                "type": "Channel",
                "name": "nixpkgs-unstable",
                "artifact": "nixexprs.tar.xz",
                "url": CHANNEL_UNSTABLE_RELEASE_URL,
            },
            downgrade={"url": "https://releases.nixos.org/nixpkgs/outdated/nixexprs.tar.xz"},
        ),
        PinTestCase(
            "channel specific version",
            "nixos-26.05",
            f"channel nixos-26.05 --at {CHANNEL_STABLE_VERSION}",
            {
                "type": "Channel",
                "name": "nixos-26.05",
                "artifact": "nixexprs.tar.xz",
                "url": CHANNEL_STABLE_RELEASE_URL,
            },
            downgrade={"url": "https://releases.nixos.org/nixos/26.05/outdated/nixexprs.tar.xz"},
        ),
        PinTestCase(
            "tarball",
            "test-tarball",
            "--name test-tarball tarball http://localhost/testTarball",
            {"type": "Url", "unpack": True, "url": "http://localhost/testTarball"},
        ),
        PinTestCase(
            "tarball mutable",
            "test-tarball-mutable",
            "--name test-tarball-mutable tarball --mutable http://localhost/latest",
            {
                "type": "MutableUrl",
                "unpack": True,
                "update_url": "http://localhost/latest",
                "url": "http://localhost/testTarball",
            },
            downgrade={"url": "http://localhost/outdated"},
        ),
        # The channel URL is *actually* mutable: it redirects to the latest release
        PinTestCase(
            "tarball channel",
            "channel-tarball",
            "--name channel-tarball tarball --mutable https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.xz",
            {
                "type": "MutableUrl",
                "unpack": True,
                "update_url": "https://channels.nixos.org/nixpkgs-unstable/nixexprs.tar.xz",
                "url": CHANNEL_UNSTABLE_RELEASE_URL,
            },
            downgrade={"url": "https://releases.nixos.org/nixpkgs/outdated/nixexprs.tar.xz"},
        ),
        PinTestCase(
            "url",
            "test-url",
            "--name test-url url http://localhost/testTarball",
            {"type": "Url", "unpack": False, "url": "http://localhost/testTarball"},
        ),
        PinTestCase(
            "url mutable",
            "test-url-mutable",
            "--name test-url-mutable url --mutable http://localhost/latest",
            {
                "type": "MutableUrl",
                "unpack": False,
                "update_url": "http://localhost/latest",
                "url": "http://localhost/testTarball",
            },
            downgrade={"url": "http://localhost/outdated"},
        ),
        PinTestCase(
            "git branch",
            "untagged-repo",
            "git http://localhost/untagged-repo.git -b foo",
            {
                "type": "Git",
                "repository": {"type": "Git", "url": "http://localhost/untagged-repo.git"},
                "branch": "foo",
                "revision": untagged_head,
                "url": None,
            },
            downgrade={"revision": FAKE_GIT_REV},
        ),
        PinTestCase(
            "git release",
            "tagged-repo",
            "git http://localhost/tagged-repo.git",
            {
                "type": "GitRelease",
                "version": "v0.2",
                "revision": tagged_v0_2,
                "url": None,
            },
            downgrade={"version": "0.1", "revision": FAKE_GIT_REV},
        ),
        # For repos with submodules, we test both with `--submodules` and without
        PinTestCase(
            "git submodule",
            "submoduled-repo",
            "git http://localhost/submoduled-repo.git --submodules",
            {
                "type": "GitRelease",
                "version": "v0.5",
                "revision": submoduled_v0_5,
                "submodules": True,
                "url": None,
            },
        ),
        PinTestCase(
            "git unfetched-submodule",
            "submoduled-repo-2",
            "--name submoduled-repo-2 git http://localhost/submoduled-repo.git",
            {
                "type": "GitRelease",
                "version": "v0.5",
                "revision": submoduled_v0_5,
                "submodules": False,
                "url": None,
            },
        ),
        PinTestCase(
            "github branch",
            "generic-repo",
            "github generic-org generic-repo -b test-branch",
            {
                "type": "Git",
                "repository": {"type": "GitHub", "owner": "generic-org", "repo": "generic-repo"},
                "branch": "test-branch",
                "revision": generic_repo_head,
                "url": f"https://github.com/generic-org/generic-repo/archive/{generic_repo_head}.tar.gz",
            },
            downgrade={"revision": FAKE_GIT_REV},
        ),
        PinTestCase(
            "github release",
            "dependency",
            "github owner dependency",
            {
                "type": "GitRelease",
                "version": "v0.2",
                "revision": dependency_v0_2,
                "url": "https://api.github.com/repos/owner/dependency/tarball/refs/tags/v0.2",
            },
            downgrade={"version": "0.1"},
        ),
        # For submodules on GitHub, we test both against release and branch pins,
        # because for GitHub the URLs differ in the general case
        PinTestCase(
            "github branch submodule",
            "main-repo-1",
            "--name main-repo-1 github owner main-repo -b main --submodules",
            {
                "type": "Git",
                "branch": "main",
                "revision": main_repo_head,
                "submodules": True,
                "url": None,
            },
        ),
        PinTestCase(
            "github branch unfetched-submodule",
            "main-repo-2",
            "--name main-repo-2 github owner main-repo -b main",
            {
                "type": "Git",
                "branch": "main",
                "revision": main_repo_head,
                "submodules": False,
                "url": f"https://github.com/owner/main-repo/archive/{main_repo_head}.tar.gz",
            },
        ),
        PinTestCase(
            "github release submodule",
            "main-repo-3",
            "--name main-repo-3 github owner main-repo --submodules",
            {
                "type": "GitRelease",
                "version": "v0.5",
                "revision": main_repo_v0_5,
                "submodules": True,
                "url": None,
            },
        ),
        PinTestCase(
            "github release unfetched-submodule",
            "main-repo-4",
            "--name main-repo-4 github owner main-repo",
            {
                "type": "GitRelease",
                "version": "v0.5",
                "revision": main_repo_v0_5,
                "submodules": False,
                "url": "https://api.github.com/repos/owner/main-repo/tarball/refs/tags/v0.5",
            },
        ),
        PinTestCase(
            "forgejo branch",
            "forgejo-branch",
            "--name forgejo-branch forgejo http://forgejo:3000 owner dependency -b test-branch",
            {
                "type": "Git",
                "repository": {
                    "type": "Forgejo",
                    "server": "http://forgejo:3000/",
                    "owner": "owner",
                    "repo": "dependency",
                },
                "branch": "test-branch",
                "revision": forgejo_dep_head,
                "url": f"http://forgejo:3000/owner/dependency/archive/{forgejo_dep_head}.tar.gz",
            },
            downgrade={"revision": FAKE_GIT_REV},
        ),
        PinTestCase(
            "forgejo release",
            "forgejo-release",
            "--name forgejo-release forgejo http://forgejo:3000 owner dependency",
            {
                "type": "GitRelease",
                "version": "v0.2",
                "revision": forgejo_dep_v0_2,
                "url": "http://forgejo:3000/api/v1/repos/owner/dependency/archive/v0.2.tar.gz",
            },
            downgrade={"version": "0.1"},
        ),
        # Like for GitHub, we test submodules against both branch and release pins,
        # because the tarball URLs differ
        PinTestCase(
            "forgejo branch submodule",
            "forgejo-main-1",
            "--name forgejo-main-1 forgejo http://forgejo:3000 owner main-repo -b main --submodules",
            {
                "type": "Git",
                "branch": "main",
                "revision": forgejo_main_head,
                "submodules": True,
                "url": None,
            },
        ),
        PinTestCase(
            "forgejo branch unfetched-submodule",
            "forgejo-main-2",
            "--name forgejo-main-2 forgejo http://forgejo:3000 owner main-repo -b main",
            {
                "type": "Git",
                "branch": "main",
                "revision": forgejo_main_head,
                "submodules": False,
                "url": f"http://forgejo:3000/owner/main-repo/archive/{forgejo_main_head}.tar.gz",
            },
        ),
        PinTestCase(
            "forgejo release submodule",
            "forgejo-main-3",
            "--name forgejo-main-3 forgejo http://forgejo:3000 owner main-repo --submodules",
            {
                "type": "GitRelease",
                "version": "v0.5",
                "revision": forgejo_main_v0_5,
                "submodules": True,
                "url": None,
            },
        ),
        PinTestCase(
            "forgejo release unfetched-submodule",
            "forgejo-main-4",
            "--name forgejo-main-4 forgejo http://forgejo:3000 owner main-repo",
            {
                "type": "GitRelease",
                "version": "v0.5",
                "revision": forgejo_main_v0_5,
                "submodules": False,
                "url": "http://forgejo:3000/api/v1/repos/owner/main-repo/archive/v0.5.tar.gz",
            },
        ),
    ]

    # Collect all pin paths for later checks
    eval_paths: dict[str, str] = {}

    # This kinda is matrix-shaped, and it is up for debate in which order iterating it first makes more sense
    # The initial idea was to test updating all pins at once with one `update` invocation, but now we've ended
    # up a bit off course and now it's a weird mix and I don't want to flip it around again
    for tc in test_cases:
        with subtest(f"{tc.name} / add"):
            succeed(f"npins add {tc.add_args}")
            pin = dump_pins()[tc.attr_name]
            for key, expected in tc.result.items():
                actual = pin.get(key)
                assert actual == expected, (
                    f"pin '{tc.attr_name}', key '{key}': expected {expected!r}, got {actual!r}"
                )

        with subtest(f"{tc.name} / eval and get-path"):
            path = nix_instantiate(f'(import ./npins)."{tc.attr_name}".outPath')
            path_nixpkgs = nix_instantiate(
                f'toString ((import ./npins)."{tc.attr_name}" {{ pkgs = import <nixpkgs> {{ }}; }})'
            )
            path_get_path = succeed(f"npins get-path {tc.attr_name}").strip()
            assert path.startswith("/nix/store/"), path
            # All three ways of getting the store path must agree
            assert path == path_nixpkgs == path_get_path, (path, path_nixpkgs, path_get_path)
            eval_paths[tc.attr_name] = path

        with subtest(f"{tc.name} / override"):
            sanitized_name = re.sub("[^a-zA-Z0-9]", "_", tc.attr_name)
            overridden = nix_instantiate(
                f'toString (import ./npins)."{tc.attr_name}".outPath',
                env=f"NPINS_OVERRIDE_{sanitized_name}=/override-path",
                flags="--impure",
            )
            assert overridden == "/override-path", overridden

    with subtest("update all"):
        # TODO expansions for future testing:
        # - Downgrade the "version" separately of the "hashes"
        # - Test `update --partial` vs `update --full` on things like impure sources
        # - Inject hash mismatches to blow up at eval time
        # - Test frozen pins
        before = dump_pins()
        for tc in test_cases:
            if tc.downgrade is not None:
                patch_pin(tc.attr_name, tc.downgrade)
        # Forbid concurrent downloads to make the snapshot deterministic
        succeed_snapshot("npins update --max-concurrent-downloads 1", "update")
        after = dump_pins()
        assert after == before, f"pins changed:\n{before!r}\n{after!r}"

    # Fetched submodules must yield different contents than their unfetched counterparts
    assert eval_paths["submoduled-repo"] != eval_paths["submoduled-repo-2"]
    assert eval_paths["main-repo-1"] != eval_paths["main-repo-2"]
    assert eval_paths["main-repo-3"] != eval_paths["main-repo-4"]
    assert eval_paths["forgejo-main-1"] != eval_paths["forgejo-main-2"]
    assert eval_paths["forgejo-main-3"] != eval_paths["forgejo-main-4"]

    succeed_snapshot("npins show", "show_all_pins")

# Container pins are special, because they always require Nixpkgs
# So, they get a special test. Yay!
with npins_subtest("container"):
    succeed("npins init --bare")

    succeed("npins add container --name hello-world registry:5000/hello-world latest")
    pin = dump_pins()["hello-world"]
    assert pin["type"] == "Container", pin
    assert pin["image_name"] == "registry:5000/hello-world"
    assert pin["image_tag"] == "latest"
    # Could test these better but eh whatever
    assert pin["image_digest"].startswith("sha256:"), pin
    assert pin["hash"].startswith("sha256-"), pin

    # Evaluation requires Nixpkgs …
    path = nix_instantiate('toString ((import ./npins)."hello-world" { pkgs = import <nixpkgs> { }; })')
    assert path.startswith("/nix/store/"), path
    fail_snapshot("npins get-path hello-world", "container_needs_pkgs")
    # Overriding still works and bypasses the fetcher
    overridden = nix_instantiate(
        'toString (import ./npins)."hello-world".outPath',
        env="NPINS_OVERRIDE_hello_world=/override-path",
        flags="--impure",
    )
    assert overridden == "/override-path", overridden

    # update test
    before = dump_pins()["hello-world"]
    patch_pin("hello-world", {"image_digest": FAKE_HASH})
    succeed("npins update hello-world")
    after = dump_pins()["hello-world"]
    assert after == before, f"pin changed:\n{before!r}\n{after!r}"


with npins_subtest("npins add git forge autodetection"):
    succeed("npins init --bare")

    # Add pins in various configurations, then check the pinned repository is correct
    succeed("npins add git https://github.com/owner/dependency.git")
    succeed("npins add --name dependency-plain git --forge none https://github.com/owner/dependency.git")
    succeed("npins add --name dependency-forgejo git http://forgejo:3000/owner/dependency.git")
    succeed("npins add git http://localhost/tagged-repo.git")

    pins = dump_pins()
    assert pins["dependency"]["repository"] == {
        "type": "GitHub",
        "owner": "owner",
        "repo": "dependency",
    }, pins["dependency"]
    assert pins["dependency-plain"]["repository"] == {
        "type": "Git",
        "url": "https://github.com/owner/dependency.git",
    }, pins["dependency-plain"]
    assert pins["dependency-forgejo"]["repository"] == {
        "type": "Forgejo",
        "server": "http://forgejo:3000/",
        "owner": "owner",
        "repo": "dependency",
    }, pins["dependency-forgejo"]
    assert pins["tagged-repo"]["repository"] == {
        "type": "Git",
        "url": "http://localhost/tagged-repo.git",
    }, pins["tagged-repo"]

# Now the unhappy cases
with npins_subtest("npins add git forge autodetection wrong forge"):
    succeed("npins init --bare")
    fail_snapshot("npins add git --forge=github http://localhost/tagged-repo.git", "add_git_wrong_forge")
    fail_snapshot("npins add git --forge=forgejo https://github.com/owner/dependency.git", "add_git_wrong_forge_2")


# Check that the pre-fetch used by npins fetches the same derivation as the Nix code later on.
# Derivation name mismatches between fetchers might produce different derivations, and thus double-fetches
# We detect fetches by counting the number of paths in the store
with npins_subtest("fetch only once"):
    succeed("npins init --bare")

    head_rev = ls_remote("http://localhost/untagged-repo.git", "refs/heads/foo")

    succeed("npins add --name by-branch git http://localhost/untagged-repo.git -b foo")
    succeed("npins add --name by-tag git http://localhost/tagged-repo.git --at v0.2")
    succeed(f"npins add --name by-rev git http://localhost/untagged-repo.git -b foo --at {head_rev}")

    succeed("ls /nix/store > store-before")

    for pin_name in ["by-branch", "by-tag", "by-rev"]:
        path = nix_instantiate(f'(import ./npins)."{pin_name}".outPath')
        assert path.startswith("/nix/store/"), path

    succeed("ls /nix/store > store-after")

    succeed("diff store-before store-after")
