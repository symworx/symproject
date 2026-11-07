# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-11-07

### Added

- `symproject help` keeps the project commands in the command list and adds a Labs section for the lab commands: `labs`, `create lab`, `edit lab`, and `delete lab`.
- `symproject labs` is the menu for local lab files. In a terminal the arrow keys move between `new`, `edit`, `delete`, and `quit`. Enter selects. After `edit` or `delete`, a second arrow list picks the lab. `q` or Ctrl-C quits. Without a terminal it prints the list and the shortcuts, then exits.
- A lab file stores the institution, the lab name, the GitHub org, an optional GitHub URL, the lab URL, and investigators. The file name is `<github-org>.yml` under `$SYMPROJECT_LABS`, `$XDG_CONFIG_HOME/symproject/labs`, or `~/.config/symproject/labs`.
- The GitHub org is a name, such as `cog-neuro`. A pasted `https://github.com/<org>` URL is accepted and stored as that name. When the org is only a name, the next prompt is the GitHub URL, and a blank answer uses `https://github.com/<org>`. `--github-url` overrides that page. An omitted `github_url` still clones from `https://github.com/<org>`.
- The lab URL is the lab home page. With an institution, the stamped line is `Lab: [Lab name](lab URL), Institution.` Investigators stay in the lab file.

## [0.1.0] - 2026-11-04

### Added

- Initial release.

## Version Links

[0.1.1]: https://github.com/symworx/symproject/releases/tag/v0.1.1
[0.1.0]: https://github.com/symworx/symproject/releases/tag/v0.1.0
