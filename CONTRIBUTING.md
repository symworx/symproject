# Contributing

The default branch is **`worx`**. Open pull requests against **`worx`**.

| Branch | Role |
|:-------|:-----|
| `worx` | Default. Open feature pull requests here. |
| `feature/*` | Short-lived work |

Trees this tool generates use `main` and `nb/<slug>`. That branch model is for research projects, not for this repository.

## This repository

`symproject` scaffolds a research-project shape (layout, runtime, identity overlays). Agent packs come from `symkit`. Do not pass `--scaffold` or `--docs` to `symkit` on a tree this tool created.

`catalog.yaml` lists shapes, runtimes, orgs, presets, and the paper stub. Add a new org by registering it in the catalog. Do not hardcode new ids in the scaffolder (`src/`).

```bash
./cli/symproject list
./cli/symproject new demo-study --title "Demo study" --runtime none --no-agents
./tests/smoke.sh
```

Keep project `AGENTS.md` templates short. Do not commit secrets, credentials, or restricted data.

## License

Apache License 2.0. Copyright (c) 2026, Nathaniel T. Berry.
