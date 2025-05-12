# Changelog

## Unreleased

- Changed the hashes to use the SRI format
- Added support for locking `git` pins by passing the hash to `fetchGit` (https://github.com/andir/npins/pull/139)

## 0.3.1

- Fixed `npins update` looking weird when having many pins (https://github.com/andir/npins/pull/138)
- Fixed `npins update` touching the lock file even if there were no updates (https://github.com/andir/npins/issues/101)
- Fixed some bugs with lockfile mode where it would still look for a "sources.json"
- Fixed a regression in the CLI argument parsing with `--name` (https://github.com/andir/npins/issues/128, https://github.com/andir/npins/pull/129)
- Fixed the caching for git prefetching, so it won't download twice anymore (https://github.com/andir/npins/pull/132)
- Fixed another CLI glitch (https://github.com/andir/npins/issues/75)

## 0.3.0

- Pins can now be "frozen", which will skip updating them (https://github.com/andir/npins/pull/78)
- `update` command now fetches in parallel (https://github.com/andir/npins/pull/112)
- Added support for fetching git submodules (https://github.com/andir/npins/pull/46)
- Added Forgejo as a supported git forge (https://github.com/andir/npins/pull/95)
- Added support for local development overrides (https://github.com/andir/npins/pull/99)
- Added `tarball` pins (https://github.com/andir/npins/pull/119)
- Added `--lockfile-mode` which will only use the JSON lock file and ignore the `default.nix` (https://github.com/andir/npins/pull/120)
- Fixed `import-flake` command not recognizing default branches (https://github.com/andir/npins/pull/114)

## 0.2.4

- Added `import-flake` command
- Added `--release-prefix` option to filter tags e.g. in monorepos with separately tagged releases
- Fetching git dependencies only downloads them once instead of twice

## 0.2.0 - 0.2.3

- Added support for private GitLab repositories

## 0.1.0

Initial release
