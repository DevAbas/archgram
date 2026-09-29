<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/DevAbas/archgram/main/docs/images/logo-dark.svg">
    <img src="https://raw.githubusercontent.com/DevAbas/archgram/main/docs/images/logo.svg" alt="" height="96">
  </picture>
  <h1 align="center">archgram</h1>
</p>

<p align="center">
  <a aria-label="npm version" href="https://www.npmjs.com/package/archgram"><img alt="npm version" src="https://img.shields.io/npm/v/archgram.svg?style=for-the-badge&labelColor=000000"></a>
  <a aria-label="License" href="https://github.com/DevAbas/archgram/blob/main/LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg?style=for-the-badge&labelColor=000000"></a>
</p>

Clean, minimal architecture diagrams from a spec. Animated flows, light and dark.

![A visitor's request goes to the API, which reads the URL cache in Redis, falls back to Postgres on a miss and adds the click to a Redis stream; a worker drains the stream into Postgres. The animation follows the visit.](https://raw.githubusercontent.com/DevAbas/archgram/main/docs/images/linkshort.svg)

You describe the system: its nodes, what connects them and the paths a
request takes. archgram lays it out, routes the lines around the boxes and
draws it, in light and dark, with the flows animated.

## Install

```sh
npm install --save-dev archgram
```

The command is a native binary for macOS, Linux and Windows, on x64 and
arm64. npm installs the one for your machine from an optional dependency,
`@archgram/cli-<os>-<cpu>`; nothing runs or downloads at install time.

If npm was told to leave optional dependencies out (`--omit=optional`),
or the lockfile was written on another platform, the binary is missing:
delete `node_modules` and `package-lock.json` and run `npm install` again.
`ARCHGRAM_BINARY` points the command at another binary.

## Use

Write a spec in JSON or YAML, for example
`docs/diagrams/linkshort.archgram.yaml`:

```yaml
archgram: 1
title: linkshort
description: A visit goes through the API to Postgres.
nodes:
  - { id: browser, kind: browser, label: Visitor }
  - { id: api, kind: service, label: API, tech: fastapi }
  - { id: db, kind: database, label: links, tech: postgresql }
edges:
  - { from: browser, to: api }
  - { from: api, to: db }
flows:
  - { name: a visit, steps: [browser, api, db] }
```

Then draw it:

```sh
npx archgram build docs/diagrams/linkshort.archgram.yaml   # linkshort.svg beside it, light and dark in one file
npx archgram build docs/diagrams/linkshort.archgram.yaml --split-themes
npx archgram check docs/diagrams/linkshort.archgram.yaml   # every problem, at its line and column
```

To draw in your own design system's colours, point archgram at your
design tokens (W3C Design Tokens) with `--theme-file`.

## Draw with Claude Code

The `archgram` skill lets Claude Code draw a project's architecture from
its code: it reads the code and the docs, writes the spec in
`docs/diagrams/`, checks and draws it with `npx archgram`, in the project's
own colours, and lists each part it drew with the file behind it. Install
it for every project:

```sh
mkdir -p ~/.claude/skills && curl -sL https://github.com/DevAbas/archgram/releases/download/v0.5.0/archgram-skill-0.5.0.tar.gz | tar -xz -C ~/.claude/skills
```

or for one project, into its `.claude/skills/` instead. Then ask Claude to
draw the architecture, or type `/archgram`.

The skill's archive carries a signed record of the build that made it:
`gh attestation verify archgram-skill-0.5.0.tar.gz -R DevAbas/archgram`.

## Documentation

- [docs/SPEC.md](https://github.com/DevAbas/archgram/blob/main/docs/SPEC.md): the spec, every field and rule, with examples.
- [DESIGN.md](https://github.com/DevAbas/archgram/blob/main/DESIGN.md): the visual rules.
- [ARCHITECTURE.md](https://github.com/DevAbas/archgram/blob/main/ARCHITECTURE.md): how archgram works inside.
- [examples/](https://github.com/DevAbas/archgram/tree/main/examples): complete specs.

## License

[MIT](https://github.com/DevAbas/archgram/blob/main/LICENSE). The embedded font, Geist, is under the SIL Open Font
License; technology logos come from Simple Icons, CC0.
