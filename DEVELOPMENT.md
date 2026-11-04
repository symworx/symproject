# Development

`./cli/symproject` builds `target/debug/symproject` when that binary is missing, then runs it. Inside this checkout the binary reads `catalog.yaml` and `templates/` from the tree. A binary installed elsewhere uses the copies embedded at compile time.

`git init` needs `git`. `--runtime uv` writes `uv.lock` when `uv` is on `PATH`. Nested agent install runs `symkit` when it is on `PATH`.

```bash
cargo +nightly fmt
cargo test
cargo clippy --all-targets -- -D warnings
./tests/smoke.sh
```

`cargo +nightly fmt` is the formatter (`rustfmt.toml`). CI runs that check with `--check`, plus the clippy and test commands above. Extend `./tests/smoke.sh` when scaffold behavior that a project tree can see changes.

## CI

`.github/workflows/ci.yml` runs on `worx` and `develop`. Job ids are `fmt` and `rust-checks` (no `name:` on those jobs). `.github/workflows/release.yml` repeats those checks for `release/**` and `v*` tags, and checks the tag against `[package].version` in `Cargo.toml` and a `## [X.Y.Z]` heading in `CHANGELOG.md`. `rust-checks` also runs `./tests/smoke.sh`. A tag push does not publish.

Generated projects use `git init -b main`. This repository's default branch is `worx`.

## Releasing

`./scripts/bump-version.sh` rewrites `[package] version` in `Cargo.toml` and the `symproject` entry in `Cargo.lock`. With no arguments it prints the current version and checks those two sites. `patch`, `minor`, `major`, and `set X.Y.Z` are the bumps. `--dry-run` prints the plan and writes nothing. `--changelog` prepends a `## [X.Y.Z]` stub when `CHANGELOG.md` exists.

On `worx` (or an optional `release/vX.Y.Z` branch): `./scripts/bump-version.sh patch --dry-run`, then the same command with `--changelog`. Commit, open a pull request against `worx`, and after merge tag `vX.Y.Z`. The tag must match `[package] version`. The release workflow checks that tag. It does not publish.
