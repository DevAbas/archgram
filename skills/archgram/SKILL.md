---
name: archgram
description: Draws a project's software architecture as an animated SVG with archgram, for its README and docs, from the code and documentation. Writes a spec to docs/diagrams/<name>.archgram.yaml, checks and draws it with the archgram command, in the project's own colours, then opens the drawing and lists each part with the file behind it. Use it whenever the user wants an architecture, system, data-flow, pipeline or "how it works" diagram, a diagram for a README, or an existing archgram diagram updated after the code changed, even if they only say "draw how this works".
license: MIT
compatibility: Requires Node 22 or later, with npx.
allowed-tools: Bash(npx --yes archgram *)
---

# Drawing architecture with archgram

archgram lays out the boxes, routes the lines, animates the flows and
draws light and dark in one SVG. Your work is what it cannot do: decide
what the reader must learn, find the facts in the code, and write them as
a spec. If the user said what to draw (a flow, a part of the system, a
direction), draw that.

Every command below runs archgram through npx, so it needs Node 22 or
later: the project's own archgram when its `package.json` has one, pinned by
its lockfile, and the latest otherwise. If `npx` is missing, say so and
stop.

```
Progress:
- [ ] 1. Reader contract
- [ ] 2. Facts, each with its file
- [ ] 3. The spec format, from archgram itself
- [ ] 4. Spec written and checked
- [ ] 5. The project's colours
- [ ] 6. Drawn, critiqued, opened
- [ ] 7. Reported
```

## 1. Decide who reads it

Read `references/reader.md` and write the reader contract: who reads the
diagram, the three to five questions they must answer from it alone, the
one idea it must make obvious, and what is left out on purpose. Everything
after this is checked against it. Take the reader from the context (the
README's audience, the user's words); ask only when nothing tells you.

## 2. Gather the facts

Read `references/architecture.md`, then the code and the docs the diagram
describes. Recognise the style the system follows in the table in
`references/styles.md`, and read that style's own file. Pick one level for
the whole diagram, list the entry points, and follow each the reader cares
about to what it calls, reads and writes. Draw a part only when a file
backs it: a module, a route, a script, a store. Name it after the file
that decides, found by following the imports, not by a name that looks
right. Draw an edge only where a line of code makes it, and note that line
(`src/api.ts:42`). Keep the list of parts with their files and edges with
their lines; the report ends with it, and it catches boxes and lines that
exist only in someone's memory. Then fit it to what a reader takes in:
merge parts with the same relations, and split what is left over about 10
nodes and 12 edges (`references/architecture.md`, How much to draw).

Ask the user, and only then, when the architecture is unclear: the README
describes a part the code does not have, two readings of the code are
equally likely, or the request names something you cannot find. Say what
you found and what you need to know. Everything else you decide yourself.

## 3. Learn the spec format from archgram

```bash
npx --yes archgram spec
```

This prints the format the very archgram you run reads: every field, node kind,
frame, flow, hint and the theme file. Read it before writing, rather than
writing from memory; a field it does not list is an error.

## 4. Write the spec, then check it

The spec lives at `docs/diagrams/<name>.archgram.yaml`, where `<name>` says
what the diagram shows (`architecture`, `request-flow`). Create the folder
when it is missing. When the file already exists, this is an update:
change it rather than starting over, keep what still holds, and note what
you changed for the report.

Map the facts onto the format: each part a node of the kind that fits it,
with its technology's logo (`tech`) when the part is built on one; each
call or data movement an edge; a boundary (a service, a trust zone, the
plugin versus the project) a frame; the path the contract's questions
follow a flow, so it animates. Keep labels to the words a reader needs.

Leave `still` out, so the drawing carries no extra text, unless the user
asks for it or the drawing is meant to be seen still (a PNG, print): then
`numbers` for one or two flows, whose step numbers a reader can follow on
the lines, and `legend` for more, whose flows read better as words. With
four flows, numbers pile up into badges such as `5,8,11,14` that a reader
cannot tie to a flow.

Then check it, and repeat until it passes:

```bash
npx --yes archgram check docs/diagrams/<name>.archgram.yaml
```

Every problem comes with its line and column, and a misspelt id or logo
with the nearest one that exists.

## 5. Draw in the project's own colours

archgram draws in black and white by default, and colour belongs to the
technology logos. When the project has its own design, draw in it: read
`references/theme.md`, which says where to look (design tokens, then the
styling code, then colours the user gave) and how to hand them to
archgram with `--theme-file`. With none of these, keep archgram's own and
say so in the report.

## 6. Draw, critique, open

```bash
npx --yes archgram build docs/diagrams/<name>.archgram.yaml
```

Add `--theme-file archgram.theme.json` when step 5 wrote one. The drawing
lands beside the spec as `docs/diagrams/<name>.svg`.

Check its width: the SVG's first line says it (`width="1231"`). Wider
than 1,300 px, its text shrinks below a comfortable size on GitHub, which
shows a README image about 880 px wide: set `direction: down`, or split it
into two diagrams, and build again until it fits.

Run the critique in `references/reader.md` against the contract and fix
what it finds: a question the picture cannot answer, a box no file backs,
an edge with no line of code behind it, a missing hop. Judge from the spec
and what `archgram build` wrote; never start a browser, headless or not,
and never take a screenshot: the user sees the drawing, and a browser
started in the background alarms them.

Then open the drawing for the user, once, with the system's own viewer:
`open` on macOS, `xdg-open` on Linux, `start ""` on Windows.

## 7. Report

Tell the user, briefly:

- the reader contract, and how the drawing answers it;
- the style of architecture you recognised, or that none fitted;
- each part drawn, with the file behind it, and each edge with the line
  that makes it;
- on an update, what changed in the spec and why;
- where the colours came from;
- what the diagram still does not show;
- the line for the README, with alt text that tells the whole flow in
  words: `![<the flow in one sentence>](docs/diagrams/<name>.svg)`.

Edit the README, or commit, only when the user asks; the project's own
rules (AGENTS.md, CLAUDE.md, CONTRIBUTING) decide how.
