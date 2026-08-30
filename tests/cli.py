# Tests for the CLI and generally user-facing features
# We make a couple of snapshots as we go along, mostly of error messages

with npins_subtest("npins init"):
    # In lockfile mode, no npins/default.nix must be generated
    succeed("npins --lock-file sources.json init --bare")
    fail("test -e npins/default.nix")
    fail("test -e default.nix")
    pins = dump_pins("sources.json")
    assert pins == {}, f"expected empty pins, got {pins!r}"

    # Regular init pins the nixpkgs-unstable channel
    succeed_snapshot("npins init", "init")
    succeed("test -e npins/default.nix")

    pins = dump_pins()
    assert "nixpkgs" in pins, f"expected a nixpkgs pin, got {pins!r}"
    assert pins["nixpkgs"]["name"] == "nixpkgs-unstable"
    assert pins["nixpkgs"]["url"] == CHANNEL_RELEASE_URL

    # Init twice
    succeed_snapshot("npins init --bare", "init_twice")


with npins_subtest("npins show"):
    succeed("npins --lock-file sources.json init --bare")
    # Setting a directory (even the default) should fail in lockfile mode
    fail_snapshot("npins --lock-file sources.json -d npins show", "error_lockfile_and_directory")


with npins_subtest("npins add --dry-run"):
    succeed("npins init --bare")
    succeed_snapshot("npins add -n git http://localhost/untagged-repo.git -b foo", "add_dry_run")
    pins = dump_pins()
    assert pins == {}, f"expected empty pins, got {pins!r}"


# Most `add` tests are currently over at `pin-types.py`, maybe shuffle things around again in the future
with npins_subtest("npins add git"):
    succeed("npins init --bare")
    fail_snapshot("npins add git http://localhost/untagged-repo.git", "add_git_repo_without_releases")


# Test `--lock-file`, and `--directory` while we're at it
with npins_subtest("npins --lock-file add"):
    succeed("npins --lock-file sources2.json init --bare")
    succeed("npins --lock-file sources2.json add git http://localhost/tagged-repo.git -b test-branch")

    pins = dump_pins("sources2.json")
    assert pins["tagged-repo"].get("version") is None
    assert pins["tagged-repo"]["revision"] == ls_remote("http://localhost/tagged-repo.git", "refs/heads/test-branch")
    assert pins["tagged-repo"].get("url") is None

    # Check setting the directory in normal mode still works
    succeed("npins -d testing init --bare")
    succeed("NPINS_DIRECTORY=testing npins show")


# Regression test for https://github.com/andir/npins/issues/75
with npins_subtest("npins add git --at non-rev"):
    succeed("npins init --bare")
    fail_snapshot("npins add git http://localhost/tagged-repo.git --branch test-branch --at v0.2", "git_add_invalid_revision")
    succeed("npins add git http://localhost/tagged-repo.git --at v0.2")
    # Make sure it still evals, because if npins puts in garbage into the lock file it may not necessarily fail until later
    succeed("npins get-path tagged-repo")


# get-path happy case is already thoroughly tested by the pin types test, focus on all unhappy paths
with npins_subtest("npins get-path failures"):
    succeed("npins init --bare")
    fail_snapshot("npins get-path foo", "get_path_no_pin")
    # A pin whose hash has been tampered with must fail at fetch time
    succeed("npins add --name broken url http://localhost/testTarball")
    patch_pin("broken", {"hash": "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="})
    fail_snapshot("npins get-path broken", "get_path_bad_hash")
