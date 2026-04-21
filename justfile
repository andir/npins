# Default target, when you just call `just`
[private]
@list:
	just --list

# Run cargo unit tests
test *OPTIONS:
    cargo test --workspace

# Run nix integration tests
# Use e.g. `just test addDryRun` to run only that test
nix-test target='':
    nix-build -A meta.tests{{ if target != '' { "." + target } else { "" } }} --no-out-link

# Some boring passthroughs for convenience and completeness

# Cargo build
build *OPTIONS:
    cargo build {{ OPTIONS }}

# Cargo check
check *OPTIONS:
    check {{ OPTIONS }}
