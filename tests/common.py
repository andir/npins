# This script contains both common helper functions for tests to easily interact with the client machine,
# and some startup code

import contextlib
import json
import os
import shlex
from pathlib import Path
from typing import Any, Iterator

FAKE_HASH = "sha256:" + "0" * 64
FAKE_GIT_REV = "0" * 40

# See `channelReleaseUrl` in ./vm.nix
# TODO: This is not the prettiest, something more akin to `ls_remote` used for git repos would be preferable
CHANNEL_RELEASE_URL = "https://releases.nixos.org/nixos/unstable/nixos-26.11pre123456.abcdef/nixexprs.tar.xz"

start_all()
channels.wait_for_unit("nginx.service")
channels.wait_for_open_port(80)
channels.wait_for_open_port(443)
github.wait_for_unit("nginx.service")
github.wait_for_open_port(80)
github.wait_for_open_port(443)
# gitlab.wait_for_unit("nginx.service")
# Forgejo gets provisioned by services that use its API to inject testing repositories
forgejo.wait_for_unit("forgejo-provision.service")
forgejo.wait_for_open_port(3000)
registry.wait_for_unit("registry-provision.service")
registry.wait_for_open_port(5000)
client.wait_for_unit("multi-user.target")
# client serves the `localhost` repositories and tarballs to itself
client.wait_for_unit("nginx.service")
client.wait_for_open_port(80)

# $out is always set unless in `driverInteractive`
# https://nixos.org/manual/nixos/stable/#sec-running-nixos-tests-interactively
snapshots = Path(os.environ.get("out", ".")) / "snapshots"
snapshots.mkdir(parents=True, exist_ok=True)

# The `testTarball` is not reproducible, so we look up the current run's hash and then find and replace the value later in snapshots
client.succeed("curl -sSf http://localhost/testTarball -o /tmp/testTarball")
tarball_hash = (
    "sha256-" + client.succeed("nix-hash --type sha256 --flat --base64 /tmp/testTarball").strip()
)

def scrub_snapshot(output: str) -> str:
    """Post-processing of snapshots to replace changing hashes with something fixed"""
    return output.replace(tarball_hash, "sha256-tarballAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=")


def succeed(cmd: str):
    """Wrapper around `client.succeed` that runs the command in our testing directory instead of PWD"""
    # The reason for this is that on VM tests, PWD is $TEMP, which may be polluted with other things we should not be wiping,
    # and in container tests PWD is /, which we should *absolutely* not be wiping
    return client.succeed(f"cd /testdir && {cmd}")

def fail(cmd: str):
    """Wrapper around `client.fail` that runs the command in our testing directory instead of PWD"""
    # The reason for this is that on VM tests, PWD is $TEMP, which may be polluted with other things we should not be wiping,
    # and in container tests PWD is /, which we should *absolutely* not be wiping
    return client.fail(f"cd /testdir && {cmd}")

def succeed_snapshot(cmd: str, name: str) -> None:
    """Wrapper around `client.succeed` that snapshots the output"""
    output = succeed(f"{cmd} 2>&1")
    (snapshots / f"{name}.exp").write_text(scrub_snapshot(f"$ {cmd}\n{output}"))


def fail_snapshot(cmd: str, name: str) -> None:
    """Wrapper around `client.fail` that snapshots the output"""
    output = fail(f"{cmd} 2>&1")
    (snapshots / f"{name}.exp").write_text(scrub_snapshot(f"$ {cmd}\n{output}"))


@contextlib.contextmanager
def npins_subtest(name: str) -> Iterator[None]:
    """Like `subtest`, but cleans the test working directory after the scope."""
    client.succeed("mkdir /testdir")
    with subtest(name):
        yield
    client.succeed("rm -rf /testdir")


def dump_pins(lock_file: str = "npins/sources.json") -> dict[str, Any]:
    return json.loads(succeed(f"cat {lock_file}"))["pins"]


def write_json(path: str, data: Any) -> None:
    succeed(f"echo {shlex.quote(json.dumps(data))} > {path}")


def patch_pin(name: str, patch: dict[str, Any], lock_file: str = "npins/sources.json") -> None:
    """Partially overwrite a pin"""
    # Read the full lock file (not just the pins like `dump_pins` does),
    # so that writing it back preserves the version field
    data = json.loads(succeed(f"cat {lock_file}"))
    data["pins"][name].update(patch)
    write_json(lock_file, data)


def ls_remote(url: str, ref: str) -> str:
    """Resolve a ref of a remote git repository to a commit hash"""
    output = client.succeed(f"git ls-remote {url} {ref}")
    assert output, f"'{ref}' not found in {url}"
    return output.split()[0]


def nix_instantiate(expr: str, env: str = "", flags: str = "") -> str:
    output = succeed(f"{env} nix-instantiate --raw --eval {flags} --expr {shlex.quote(expr)}")
    return output
