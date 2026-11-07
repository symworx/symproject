# __PROJECT_TITLE__

Research project for a multi-paper line. Replace this paragraph with the aim. The longer plan lives in `docs/design.md`.

__LAB_LINE__

__ISSUE_LINE__

Repository: __REPO_WEB__

```bash
__CLONE_SNIPPET__
```

## Paper line

| Paper | Issue | Status |
|:------|:------|:-------|
| | | |

Add a paper with `symproject paper <slug> --title "..."`. That creates `papers/<slug>/` and `results/<slug>/`. It does not create the git branch. Do the work on `nb/<slug>`. `main` holds the settled layout and the shared library.

## Layout

```text
__PROJECT_NAME__/
├── docs/                   # design, funding science, methods
├── analysis/               # shared data pipeline
├── papers/<slug>/          # manuscript, notes/claims.md, analysis/
├── src/__PACKAGE__/        # shared library for every paper
├── scripts/                # short entry points
├── data/                   # README and samples only
├── results/<slug>/         # generated artifacts (gitignored)
└── tests/
```

Later papers add `papers/<slug>/`. They reuse `src/__PACKAGE__/` and the database built by `analysis/`. A paper's estimate stays in `papers/<slug>/analysis/` and writes `results/<slug>/`.

Prefer [SymWorx](https://github.com/symworx/symworx) for shared dynamics and signal methods.

__RUNTIME_NOTE__
