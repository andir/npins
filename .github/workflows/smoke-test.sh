#! /usr/bin/env bash

# This is just a simple smoke test until we have a more elaborate test setup.
# You should NOT start adding test cases or helper functions to this.
# The purpose is to verify a very simple use case of npins. Testing exactly one
# nixpkgs pin and importing it.

set -ex

NPINS=$(nix-build default.nix --no-out-link)/bin/npins

$NPINS --help

TMPDIR=$(mktemp -d)

trap 'rm -rf $TMPDIR' EXIT

$NPINS -d $TMPDIR init
$NPINS -d $TMPDIR update
test $($NPINS -d $TMPDIR show | grep nixpkgs | wc -l) -gt 0 || {
  echo "Failed to find nixpkgs in npins output";
  exit 1;
}


nix-instantiate --expr "(import (import $TMPDIR).nixpkgs {}).hello" || {
  echo "Failed to build the hello package with the pinned nixpkgs";
  exit 1
}
