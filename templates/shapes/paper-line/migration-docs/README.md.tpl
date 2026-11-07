# migration-docs

This directory lives only on this machine. Back it up somewhere else. Git does not store the files you drop here.

Drop a legacy PDF or Word file in this folder. After you convert it, put the markdown in the paper line.

| Dump | Lands in |
|:-----|:---------|
| Planned study | `docs/design.md` |
| Submitted aims, approach, sample size | `docs/funding/` |
| Protocol, questionnaire, dictionary, analysis plan | `docs/methods/` |
| A manuscript | `papers/<slug>/manuscript.md` |

The converted markdown is the record. This tool does not convert the file.

To track a file from this directory, add its path on its own line in [`.gitkeep`](.gitkeep), then run `symproject new` again. `--no-migration-docs` skips that refresh and does not delete this directory. Leave participant data, filled questionnaires, private scoring keys, and budgets with salaries untracked.
