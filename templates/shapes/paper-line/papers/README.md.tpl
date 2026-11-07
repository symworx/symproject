# Papers

One directory per working paper. The index of the line lives in the repository README.

```text
papers/<slug>/
├── README.md
├── manuscript.md
├── notes/
│   ├── claims.md       # locked for this paper; cites docs/
│   └── pipeline.md
└── analysis/
    └── README.md       # purpose, inputs, outputs, assumptions, rerun
```

```bash
symproject paper <slug> --title "Paper title"
```

That command fills missing files only. It prints a row for the README table. It does not create `nb/<slug>`.
