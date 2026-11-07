# Analysis

Shared data pipeline for every paper in this line. This tree loads and conforms data and builds the database the papers read (SymWorx, Postgres, or sqlite).

It does not answer a paper's question. Paper analysis stays in `papers/<slug>/analysis/` and writes `results/<slug>/`.

## Database

Name the engine and the database. Keep connection strings in the environment, not in this file.

## Schema

Commit schema SQL here. Do not commit database files or dumps.

## Rerun

```bash
# document the command that rebuilds the catalog
```

`scripts/` may hold a short entry point that calls this tree. Do not keep a second copy of the load there.
