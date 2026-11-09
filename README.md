# symproject

Scaffold a **research project** for a multi-paper line: shared library, per-paper analysis, and generated results.

Agent packs come from [symworx/symkit](https://github.com/symworx/symkit). This tool writes the directory tree. `symkit` installs the research harness and does not copy its own workspace stubs.

## Install

Published on [crates.io](https://crates.io/crates/symproject). The installed binary is `symproject`. It uses the catalog and templates embedded at that version.

```bash
cargo install --locked symproject
```

From this directory, install the binary locally:

```bash
cargo install --locked --path .
```

That puts `symproject` on `PATH` and embeds the catalog and templates from this checkout. `./cli/symproject` builds the debug binary and runs it without installing. In this checkout that binary reads `catalog.yaml` and `templates/` from the tree.

The examples below use `./cli/symproject` from a clone. After either install, run `symproject` with the same arguments.

## Quick start

```bash
./cli/symproject list

./cli/symproject new demo-study \
  --title "Demo study" \
  --no-agents

# list labs, then write one into the config directory
./cli/symproject labs
./cli/symproject create lab \
  --institution "State University" \
  --lab-name "Cog Neuro" \
  --github-org cog-neuro \
  --lab-url "https://example.edu/cog" \
  --investigator "Ada Lovelace"
./cli/symproject new chp-coupling \
  --title "CHP coupling" \
  --package coupling \
  --lab cog-neuro \
  --no-agents

# change the lab file, then replace the recorded line in a project
./cli/symproject edit lab cog-neuro --lab-url "https://example.edu/cog-neuro"
./cli/symproject set-lab cog-neuro chp-coupling

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

`new` writes `migration-docs/README.md` and `migration-docs/.gitkeep`. Files dropped there stay on this machine. Back them up elsewhere. To track one file, add its path on its own line in `.gitkeep` and run `new` again. `--no-migration-docs` skips the directory and does not refresh those exceptions. It does not delete an existing `migration-docs/`. The nested `symkit` install is not passed this flag.

## Catalog

`catalog.yaml` is the source of truth for shape, runtime, and org. The default org is `none`, so a new project has no lab line. Do not hardcode lab names in the scaffolder.

| Flag | Default | Meaning |
|:-|:--|:----|
| `--shape` | `paper-line` | `papers/`, `docs/`, `analysis/`, `src/<package>/`, `scripts/`, `data/`, `results/` |
| `--runtime` | `uv` | `uv` writes `pyproject.toml` and `uv.lock` when `uv` is on `PATH`. `none` skips both |
| `--org` | `none` | `none` leaves the lab line blank. `lab` writes a replaceable placeholder line |
| `--preset` | (none) | Named combo of shape, runtime, and org. Other flags override |
| `--github-org` | empty | Clone URLs. Overrides `--lab` and the org |
| `--lab` | (none) | Lab file, or a name in the labs directory, on `new` |
| `--paper` | off | Also write one paper stub during `new` |
| `--package` | slug with `-` → `_` | Directory under `src/` |
| `--no-migration-docs` | off | Skip `migration-docs/`. Do not refresh `.gitkeep` exceptions |

Later layers override the same destination: shape, then runtime, then org. `--lab` overrides the org vars. `--github-org` overrides the GitHub org from the lab file.

`symproject labs` lists local labs. In a terminal, the arrow keys move between `new`, `edit`, `delete`, and `quit`. Enter selects. `q` or Ctrl-C quits. Enter on `edit` or `delete` uses the arrow keys to choose a lab. Without a terminal it prints the list and the shortcuts, then exits.

Labs are stored in `$SYMPROJECT_LABS`, or `$XDG_CONFIG_HOME/symproject/labs`, or `~/.config/symproject/labs`. The file name is `<github-org>.yml`. `--lab <name>` and `set-lab <name>` use that file when `<name>` is not already a path.

`symproject create lab` is the shortcut for a new lab. In a terminal, with no arguments, it prompts for the institution, the lab name, the GitHub org, the GitHub URL, the lab URL, and investigators. The GitHub org is a name, such as `cog-neuro`. A pasted `https://github.com/<org>` URL is accepted. When the org is only a name, the next prompt is the GitHub URL, and a blank answer uses `https://github.com/<org>`. An empty GitHub org is taken from the lab name. The lab URL is the lab home page. Without a terminal, pass `--institution`, `--lab-name`, `--github-org`, and `--lab-url`. `--github-url` overrides the GitHub URL. Repeat `--investigator` for each person. A second write for the same GitHub org fails and leaves the file in place. `create lab [file]` with no identity flags still copies `templates/lab.yml` and skips an existing file.

`symproject edit lab <name>` changes one lab. Pass at least one of `--institution`, `--lab-name`, `--github-org`, `--github-url`, `--lab-url`, `--investigator`, or `--clear-investigators`. In a terminal you can omit the name and the flags and answer the prompts. `--github-org` renames the file when the new name is free.

`symproject delete lab <name>` removes that file. It asks you to type the name again. `--yes` skips the question. Without a terminal, `--yes` is required. Delete does not rewrite a project already stamped with that lab.

Investigators stay in the YAML. The stamped line is `Lab: [Lab name](url), Institution.` A file with no institution keeps `Lab: [Lab name](url).` `--lab` with the default org records a replaceable lab line. `--lab` with `--org none` is an error.

`symproject set-lab <lab> [project]` replaces the recorded lab line, issue line, and clone URL. It reads `.symproject.yaml` and rewrites `README.md` and `CONTRIBUTING.md`. `--dry-run` prints the replacements and writes nothing. A project stamped with `org: none` has no lab line to replace. `new` leaves files that already exist.

`symproject update [project]` fills files the current catalog adds and refreshes the `migration-docs` block in `.gitignore`. The working directory is the project when you omit the path. It reads `.symproject.yaml` for the project name and org. `--package` names the directory under `src/` when that directory is missing or when `src/` has more than one. Existing files stay as they are, including the lab line. It does not run `git init` or `symkit install`. `--title` applies only to files this run creates; otherwise a new file uses the `#` heading in `README.md`. `--no-migration-docs` skips the gitignore refresh. A lab change stays on `set-lab`.

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
├── migration-docs/           # local legacy files; dumps stay gitignored
│   ├── README.md
│   └── .gitkeep
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
