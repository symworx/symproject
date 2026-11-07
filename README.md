# symproject

Scaffold a **research project** for a multi-paper line: shared library, per-paper analysis, and generated results.

Agent packs come from [symworx/symkit](https://github.com/symworx/symkit). This tool writes the directory tree. `symkit` installs the research harness and does not copy its own workspace stubs.

## Install

Published on [crates.io](https://crates.io/crates/symproject). The installed binary is `symproject`. It uses the catalog and templates embedded at that version.

```bash
cargo install --locked symproject
```

From a clone, `./cli/symproject` builds the debug binary and runs it. In this checkout the binary reads `catalog.yaml` and `templates/` from the tree.

```bash
cargo install --locked --path .
```

The examples below use `./cli/symproject` from a clone. After the crates.io install, run `symproject` with the same arguments.

## Quick start

```bash
./cli/symproject list

./cli/symproject new demo-study \
  --title "Demo study" \
  --no-agents

# copy the lab template, edit the copy, then restamp a project
cp templates/lab.yml lab.yml
./cli/symproject set-lab lab.yml demo-study

# stamp a lab while scaffolding
./cli/symproject new chp-coupling \
  --title "CHP coupling" \
  --package coupling \
  --lab lab.yml

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

No `--scaffold` and no `--docs`. Layout is this tool's job. `new` writes `docs/design.md`, `docs/funding/`, `docs/methods/`, and `analysis/`. `--docs` would add `docs/aims.md`, which this layout does not use. If `symkit` is missing, the tree is still written and the install command is printed. `--no-agents` skips the nested install.

## Catalog

`catalog.yaml` is the source of truth for shape, runtime, and the default org. Do not hardcode those ids in the scaffolder. Real lab names belong in a copy of `templates/lab.yml`.

| Flag | Default | Meaning |
|:-|:--|:----|
| `--shape` | `paper-line` | `papers/`, `docs/`, `analysis/`, `src/<package>/`, `scripts/`, `data/`, `results/` |
| `--runtime` | `uv` | `uv` writes `pyproject.toml` and `uv.lock` when `uv` is on `PATH`. `none` skips both |
| `--org` | `lab` | `lab` writes a replaceable lab line and clone URL. `none` leaves the lab line blank |
| `--preset` | (none) | Named combo of shape, runtime, and org. Other flags override |
| `--github-org` | empty | Clone URLs. Overrides `--lab` and the org |
| `--lab` | (none) | `lab.yml` overlay on `new` |
| `--paper` | off | Also write one paper stub during `new` |
| `--package` | slug with `-` → `_` | Directory under `src/` |

Later layers override the same destination: shape, then runtime, then org. `--lab` overrides the org vars. `--github-org` overrides the GitHub org from the lab file.

`symproject set-lab lab.yml [project]` replaces the recorded lab line, issue line, and clone URL. It reads `.symproject.yaml` and rewrites `README.md` and `CONTRIBUTING.md`. `--dry-run` prints the replacements and writes nothing. `new` leaves files that already exist.

## What it writes

`new` writes this tree and runs `git init -b main`. `<package>` is `--package`, or the project name with `-` replaced by `_`. `pyproject.toml` is written for `--runtime uv`. `uv.lock` is written when `uv` is on `PATH`. `.symproject.yaml` records the lab holdings `set-lab` replaces.

```text
<project>/
├── README.md                 # program aim and the paper-line table
├── CONTRIBUTING.md
├── AGENTS.md
├── .gitignore
├── .symproject.yaml
├── pyproject.toml            # runtime uv
├── docs/
│   ├── README.md             # idea, funding science, methods
│   ├── design.md
│   ├── funding/
│   │   └── README.md
│   └── methods/
│       └── README.md
├── analysis/
│   └── README.md             # shared data pipeline
├── papers/
│   └── README.md             # index of papers/<slug>/
├── src/<package>/            # shared library for every paper
│   ├── README.md
│   └── __init__.py
├── scripts/                  # short entry points
│   └── README.md
├── data/                     # README and samples only
│   ├── README.md
│   └── samples/
│       └── .gitkeep
├── results/                  # generated artifacts, one folder per slug
│   └── README.md
└── tests/
    └── README.md
```

`symproject paper <slug> --title "..."` adds one paper and its results folder. It prints a row for the README table. Create the git branch `nb/<slug>` yourself. The root README stays as written.

```text
papers/<slug>/
├── README.md
├── manuscript.md
├── notes/
│   ├── claims.md             # locked methods for this paper
│   └── pipeline.md
└── analysis/
    └── README.md             # purpose, inputs, outputs, rerun
results/<slug>/
└── .gitkeep
```

Program aim stays in the root README. The longer plan stays in `docs/design.md`. Per-paper truth stays in `papers/<slug>/notes/claims.md`, which cites the funding and methods files it locked. Root `analysis/` builds the shared database. Later papers add another `papers/<slug>/` and reuse `src/<package>/` and that database.

Generated projects gitignore `data/private/`, `data/local/`, `data/open/`, and `results/**`. READMEs and `.gitkeep` under `results/` stay tracked. Those `data/` extract directories are not created.

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
