# Development

`./cli/symproject` builds `target/debug/symproject` when that binary is missing, then runs it. Inside this checkout the binary reads `catalog.yaml` and `templates/` from the tree. A binary installed elsewhere uses the copies embedded at compile time.

`git init` needs `git`. `--runtime uv` writes `uv.lock` when `uv` is on `PATH`. Nested agent install runs `symkit` when it is on `PATH`.

```bash
cargo +nightly fmt
cargo test
cargo clippy --all-targets -- -D warnings
./tests/smoke.sh
```

`cargo +nightly fmt` is the formatter (`rustfmt.toml`). Extend `./tests/smoke.sh` when scaffold behavior that a project tree can see changes.

Generated projects use `git init -b main`. This repository's default branch is `worx`.
