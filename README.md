# symproject

Scaffold a **research project** for a multi-paper line: shared library, per-paper analysis, and generated results.

Agent packs come from [symworx/symkit](https://github.com/symworx/symkit). This tool writes the directory tree. `symkit` installs the research harness and does not copy its own workspace stubs.

## Install

From a clone, `./cli/symproject` builds the debug binary and runs it. In this checkout the binary reads `catalog.yaml` and `templates/` from the tree.

```bash
cargo install --locked --path .
```

## Quick start

```bash
./cli/symproject list

./cli/symproject new demo-study \
  --title "Demo study" \
  --no-agents

# cSYMd lab overlay (paper line + uv + github.com/csymd)
./cli/symproject new chp-coupling \
  --title "CHP coupling" \
  --package coupling \
  --preset csymd

# add one paper to an existing project
cd demo-study
../cli/symproject paper multirate-fusion \
  --title "Multirate fusion predicts GH from HRV"
```

`--title` is required. Existing files are skipped. `--package` is the import name under `src/`. The default replaces `-` in the project name with `_`.

If `symkit` is on `PATH`, `new` then runs:

```bash
symkit install <outdir> --harness research --role researcher --yes
```

No `--scaffold` and no `--docs`. Layout is this tool's job. `--docs` would add `docs/aims.md` beside `papers/`. If `symkit` is missing, the tree is still written and the install command is printed. `--no-agents` skips the nested install.

## Catalog

`catalog.yaml` is the source of truth. Do not hardcode shape, runtime, or org ids in the scaffolder.

| Flag | Default | Meaning |
|:--|:--|:--|
| `--shape` | `paper-line` | `papers/`, `src/<package>/`, `scripts/`, `data/`, `results/` |
| `--runtime` | `uv` | `uv` writes `pyproject.toml` and `uv.lock` when `uv` is on `PATH`. `none` skips both |
| `--org` | `none` | `csymd` sets the lab link and GitHub org `csymd` |
| `--preset` | (none) | `csymd` = paper-line + uv + csymd. Other flags override |
| `--github-org` | empty | Clone URLs. `csymd` defaults to `csymd` |
| `--paper` | off | Also write one paper stub during `new` |
| `--package` | slug with `-` → `_` | Directory under `src/` |

Later layers override the same destination: shape, then runtime, then org.

## What it writes

- `README.md`, `CONTRIBUTING.md`, `AGENTS.md`, `.gitignore`
- `papers/README.md`, `results/README.md`, `data/README.md`, `data/samples/.gitkeep`
- `scripts/README.md`, `tests/README.md`
- `src/<package>/__init__.py` and a short README
- `pyproject.toml` when the runtime is `uv`
- `git init -b main`

`symproject paper <slug> --title "..."` adds:

- `papers/<slug>/README.md`, `manuscript.md`
- `papers/<slug>/notes/claims.md`, `pipeline.md`
- `papers/<slug>/analysis/README.md`
- `results/<slug>/.gitkeep`

It prints a row for the README table. It does not create `nb/<slug>` and it does not rewrite the README.

Generated projects keep restricted extracts and `results/**` gitignored. Program aim stays in the root README. Per-paper truth stays in `papers/<slug>/notes/claims.md`.

## Tests

```bash
cargo +nightly fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
./tests/smoke.sh
```

Command notes: [DEVELOPMENT.md](DEVELOPMENT.md).

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE.md](NOTICE.md).

Copyright (c) 2026, Nathaniel T. Berry.
