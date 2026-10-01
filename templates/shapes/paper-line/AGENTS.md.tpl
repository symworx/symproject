# AGENTS.md — paper line

Instructions in this file take precedence over the research harness when they conflict.

## Layout

Map work onto the tree that is already here. Do not add a root `analysis/` or `docs/` directory beside `papers/`.

| Path | Role |
|:-----|:-----|
| `papers/<slug>/` | One paper: `manuscript.md`, `notes/claims.md`, `analysis/` |
| `src/__PACKAGE__/` | Shared library for every paper in this repo |
| `scripts/` | Repo-wide ingest. Paper analysis stays under `papers/<slug>/analysis/` |
| `data/` | Pointers and tiny public or synthetic samples |
| `results/<slug>/` | Generated tables and figures. Gitignored except READMEs |

## Scientific truth

Quote `papers/<slug>/notes/claims.md` for that paper. If a claim is still a placeholder, ask. Label anything not written there as **provisional**.

Do not invent effect sizes, endpoints, sample sizes, or citations.

## Methods

Prefer SymWorx for shared dynamics and signal methods. Do not reimplement an algorithm this repo already documents as coming from there.
