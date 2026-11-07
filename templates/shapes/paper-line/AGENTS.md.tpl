# AGENTS.md — paper line

Instructions in this file take precedence over the research harness when they conflict.

## Layout

Map work onto the tree that is already here. The root `analysis/` tree builds the shared database. A paper's estimate stays in `papers/<slug>/analysis/`.

| Path | Role |
|:-----|:-----|
| `docs/` | Written record for the line: `design.md`, `funding/`, `methods/` |
| `analysis/` | Shared data pipeline. Does not write `results/<slug>/` |
| `papers/<slug>/` | One paper: `manuscript.md`, `notes/claims.md`, `analysis/` |
| `src/__PACKAGE__/` | Shared library for every paper in this repo |
| `scripts/` | Short entry points. The pipeline lives in `analysis/` |
| `data/` | Pointers and tiny public or synthetic samples |
| `results/<slug>/` | Generated tables and figures. Gitignored except READMEs |

## Scientific truth

For a paper, quote `papers/<slug>/notes/claims.md`. If a claim is still a placeholder, ask. Label anything not written there as **provisional**.

`docs/design.md` is the planned study. A dated file in `docs/funding/` is what a sponsor was told. A file in `docs/methods/` is a procedure or a planned estimate, and only the file the claim names. When these disagree, leave them as written and say so.

The program aim stays in the root README. Do not add `docs/aims.md`. Do not run `symkit` with `--docs`.

Do not invent effect sizes, endpoints, sample sizes, or citations.

## Methods

Prefer SymWorx for shared dynamics and signal methods. Do not reimplement an algorithm this repo already documents as coming from there.
