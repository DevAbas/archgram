# lintkit

A small linter with plugins. `lintkit init` sets a project up, `check`
finds problems, `fix` repairs what it can and checks again, and `report`
writes a page for the team. Rules are plugins: any file in `plugins/`, or
a package named in `lintkit.config.json`.
