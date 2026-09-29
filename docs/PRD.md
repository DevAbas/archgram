# PRD: archgram

| Field   | Value      |
|---------|------------|
| Version | 0.11       |
| Date    | 2026-09-29 |
| Status  | Draft      |
| Owner   | Abas Turabli |

This document describes what archgram is and why it exists. How it is built
lives in `ARCHITECTURE.md`, the input format in `docs/SPEC.md`, the visual
rules in `DESIGN.md` and the values in `design-system/tokens/`. Each fact
lives in one of them; the others refer to it. If a requirement changes,
this document changes first.

## 1. Problem

A software architecture diagram in a README is read in about thirty seconds
by someone who did not build the system. Today it is made in one of two
ways, and both fail that reader.

Drawn by hand in Figma, draw.io or Excalidraw, it looks good on the day it
is made and drifts from the code afterwards, because every change means
opening a tool and moving boxes. Generated from Mermaid or a similar text
format, it stays in the repository, but the output is generic: no visual
vocabulary for what a box is, weak layout for nested groups such as a VPC
or a trust boundary, no dark mode of its own, no way to show the order in
which things happen.

AI coding agents hit the same wall from the other side. Asked for a
diagram, an agent spends most of its time placing coordinates by hand. In
a test of three diagram tasks, agents took on average more than nine
minutes each, most of it on layout.

## 2. Users

| User | What they do with archgram |
|---|---|
| A developer documenting a system | Writes or edits a spec next to the code and regenerates the diagram when the system changes |
| An AI coding agent | Reads the code, writes the spec, runs archgram, checks the result; archgram removes the layout work |
| The reader of the diagram | Never touches archgram. Everything is judged by what they understand in thirty seconds |

## 3. Product promise

Describe the system, not the drawing. A spec lists typed nodes (a service,
a database, a queue, a model), the frames that group them, the edges
between them and, optionally, the flows through them. archgram lays it out,
routes every edge around the boxes, draws it in one consistent minimalist
visual language, in light and dark, animated when asked, and produces the
same file every time.

## 4. Principles

1. Reader first. Every default serves the person reading the diagram,
   not the person writing the spec.
2. Semantic, not visual. The spec says what a thing is; archgram decides
   how it looks. No coordinates in the spec.
3. Deterministic. The same spec gives a byte-identical file on every
   machine, so diagrams diff cleanly in git.
4. Our own engine. Layout and edge routing are written in-house. Papers
   and open-source engines are read as examples of good practice, not
   copied and not treated as the source of truth.
5. Few, trusted dependencies. The core depends on three crates; every
   other capability is a separate, optional module.
6. Minimalist. Neutral shapes, colour only where it carries meaning.
7. Accessible. Text meets WCAG 2.1 AA contrast in both themes, motion
   stops under reduced motion, every SVG carries a title and a
   description.
8. Themeable. Switching palette is one line in the spec or one flag on
   the command line. A project's own design system is used only when
   asked for.

## 5. Scope by version

| Version | Delivers |
|---|---|
| 0.1 | JSON spec; every node kind in section 7 with its icon; horizontal and vertical cards; layered layout without frames; orthogonal edge routing; one static SVG with light and dark; the mono palette, with the token structure ready for more palettes; text measured with an embedded font; the `archgram build` command |
| 0.2 | Frames, nested; single, multi-node and external variants; an automatic legend; the YAML module; technology logos from Simple Icons; separate light and dark SVG files |
| 0.3 | Flows and their animation, timed automatically; importing a project's DTCG design tokens as a theme; the `archgram` command on npm for Node, a native binary per platform; open-source release under MIT |
| 0.4 | An edge's label lights with its signal; an optional "by archgram" credit; a spec named `<name>.archgram.yaml` draws `<name>.svg`, into a folder created when missing; the archgram plugin for Claude Code with its `archgram:draw` skill, the archgram repository being its marketplace |
| Later | The WASM package, for the browser; the PNG module, drawn from the same scene as the SVG by archgram's own rasterizer |

## 6. Functional requirements

### 6.1 Spec
- Input is JSON in the core and YAML through the optional module; both map
  to the same types.
- Unknown fields and unknown node kinds are errors, reported with their
  location, never ignored.
- The spec holds no coordinates. It may carry layout hints: direction,
  which nodes share a column or a row, and the order of nodes within one.

### 6.2 Layout and routing
- Edges flow in one main direction, left to right or top to bottom.
- No two boxes overlap. No edge passes through a box it does not start
  or end at.
- Edges are orthogonal, with as few bends and crossings as the layout
  allows.
- Frames contain their nodes, and an edge may cross a frame's border.

### 6.3 Rendering
- Each node kind has its own shape, so a reader recognises it before
  reading its label.
- Each node category has its own hue, and only the lines of its icons take
  it; cards, frames and connectors stay neutral.
- One SVG carries both themes and follows `prefers-color-scheme`; on
  request archgram writes one file per theme.
- On request, a project's own DTCG design tokens fill archgram's colour
  roles, through a mapping file; the colours must keep the same contrast
  as archgram's own, in both themes.
- Output is self-contained: no script, no external file, no web font.
- A small "by archgram" credit sits in the drawing's bottom-right corner,
  faint and hidden from screen readers; the spec turns it off.

### 6.4 Animation
- Off by default. A flow in the spec turns it on for that path. A flow
  may branch: one step may reach several nodes at once.
- A signal travelling a flow takes the colour of the node it leaves. The
  spec picks how it is drawn from a fixed set of styles; by default the
  line fills with that colour.
- A card is lit from the moment a signal reaches it until every signal it
  sends has arrived: its border and a faint fill take its category's hue.
  Then it returns to its usual look.
- An edge's label is lit while its signal travels the edge: it takes the
  signal's colour, or the theme's text colour where the signal's colour
  would fall short of text contrast.
- archgram computes the timeline: branches start together, converging
  paths arrive together, the last node of a flow lights last.
- Native SMIL only; every animation stops under
  `prefers-reduced-motion`, and the still image shows what the spec
  chooses: nothing more, the flows in words under the legend, or each
  step's number on its lines. A screen reader hears each flow in words.

### 6.5 Outputs and interfaces
- A command-line tool, one native binary per platform, installed by hand
  or from npm (`npm install archgram`), so a Node project's scripts run
  it with no Rust toolchain.
- A spec named `<name>.archgram.yaml` (or `.yml`, `.json`) draws `<name>.svg`
  beside it, so a project keeps each spec next to its drawing; a folder
  given for the output is created when it does not exist.
- A library, usable from Rust and, later through WASM, from the browser
  (§5).
- PNG, one file per theme, through the optional module (later, §5).
- A plugin for Claude Code, `archgram`, whose marketplace is the archgram
  repository itself, at the same version as the command (§6.6).

### 6.6 The `archgram:draw` skill
- It draws a project's architecture for its README and docs, from the
  code and the documentation, so the agent writes the spec and archgram
  does the drawing. It replaces the `drawing-architecture-diagrams`
  skill.
- The user calls it (`/archgram:draw`, with instructions such as which
  flow or which direction), or Claude chooses it when asked for an
  architecture diagram. It never runs by itself.
- A part is drawn only when a file in the project backs it, and the skill
  notes that file. Where the code and the documentation disagree, or the
  architecture is unclear, it asks the user; it asks for no other
  approval.
- It writes `docs/diagrams/<name>.archgram.yaml`, creating the folder when
  missing, and draws `<name>.svg` beside it with `npx archgram@<version>`,
  the plugin's own version. The spec is kept, so a later call changes it
  and draws again, and says what changed.
- It draws in the project's own colours, found in this order: the
  project's DTCG tokens, through a mapping file; else the colours its
  code styles with (CSS custom properties, a Tailwind theme, Sass
  variables, a theme object), which the skill reads and writes as DTCG
  tokens and a mapping beside the spec; else colours the user gives in
  the request; else archgram's own palette. archgram's contrast check
  holds either way; a colour that fails it is reported with the reason.
- When done, `archgram check` passes, the SVG is opened with the
  system's own viewer (`open`, `xdg-open`, `start`), and the skill lists
  each part with the file behind it.

## 7. Node vocabulary

Every kind has three variants: single, multi-node (several instances,
drawn as stacked boxes) and external (a system we do not own, drawn with
a dashed border).

| Group | Kinds |
|---|---|
| Core | service, database, queue, cache, storage, users |
| AI and LLM | model, vector store, tool, agent |
| Build and tooling | file, script, generated file, check |
| Clients | browser, mobile, desktop |

A node may name its technology (`tech: postgresql`, a Simple Icons slug);
archgram then shows the technology's logo, in one of four places the
diagram chooses: the card's corner, before the note (or the technology's
name) on the card's second line, a chip on the icon, or in place of the
kind's icon. Logos come from
a pinned Simple Icons release, CC0 data; a logo under a licence of its
own is left out. Each is drawn in its brand's own colour, in every place
and whether or not a flow lights its card, so a reader knows the
technology at a glance; a brand colour that would not show on the card in
a theme is the text colour there. The brands' guidelines stay with
whoever publishes a diagram, and archgram records each logo's source and
guidelines for them.

## 8. Success criteria

- The two README diagrams of ai-powered-cv-screener are reproduced from
  specs with no coordinates, and a reviewer prefers or accepts them.
- On a test set of specs: no overlapping boxes, no edge through a box,
  byte-identical output across runs and across macOS, Linux and Windows.
- Every text pair passes WCAG 2.1 AA in both themes.
- Layout and SVG for a 100-node spec take under 50 ms in the native CLI.
- The core WASM module, when there is one, stays under 350 KB gzipped.
- With archgram, an agent produces an approved diagram in less than half
  the time it took without it (baseline: 9.3 minutes on average).

## 9. Non-goals

- An interactive editor or any GUI.
- Free-form drawing: a box at a chosen position, a line with a chosen
  path.
- Other diagram types: sequence, entity-relationship, class, Gantt.
- A hand-drawn or sketch style.
- Graphs larger than about 300 nodes.
- A hosted rendering service.

## 10. Open questions

None.

## 11. Changelog

| Version | Date       | Change |
|---------|------------|--------|
| 0.1     | 2026-09-27 | First draft: problem, users, principles, scope by version, node vocabulary, success criteria, non-goals. |
| 0.2     | 2026-09-27 | The mono palette, colour only in icon lines; horizontal and vertical cards; logos in the corner or on a chip (§5, §6.3, §7). Palettes removed from the open questions. |
| 0.3     | 2026-09-27 | The font is Geist, measured and embedded as a subset; removed from the open questions. |
| 0.4     | 2026-09-27 | Logos: a pinned Simple Icons release, CC0 data only, drawn neutral, sources and guidelines recorded (§7); removed from the open questions. |
| 0.5     | 2026-09-27 | Four places for a logo: corner, inline, chip, icon (§7); the brand's colour while a signal is at the card (§6.4). |
| 0.6     | 2026-09-27 | Flows branch; seven signal styles, the line filling by default; a lit card takes its hue until its signals arrive; the still image lists or numbers the flows on request (§6.4). |
| 0.7     | 2026-09-27 | A project's DTCG tokens as the theme, through a mapping file, held to the same contrast (§6.3). |
| 0.8     | 2026-09-28 | The PNG module moves after 0.3 (§5, §6.5). |
| 0.9     | 2026-09-28 | 0.3 ships the command on npm for Node, a native binary per platform; the WASM package, for the browser, moves after 0.3 (§5, §6.5, §8). |
| 0.10    | 2026-09-29 | 0.4: an edge's label lit with its signal (§6.4), an optional credit (§6.3), `<name>.archgram.yaml` draws `<name>.svg` into a folder created when missing (§6.5), and the archgram plugin for Claude Code with its `archgram:draw` skill, replacing `drawing-architecture-diagrams` (§5, §6.5, §6.6). 0.3 was the open-source release (§5). The trademark check is done, removed from the open questions: no "archgram" mark in the USPTO or TMview searches of 2026-09-29, classes 9 and 42 (TMview's one hit, "searchgram", is a different word, registered in Korea for point, billing and big-data software). |
| 0.11    | 2026-09-29 | A logo is always in its brand's colour, not only while its card is lit (§6.4, §7). |
