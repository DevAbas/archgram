# PRD: archgram

| Field   | Value      |
|---------|------------|
| Version | 0.20       |
| Date    | 2026-10-02 |
| Status  | Draft      |
| Owner   | Abas Turabli |

This document describes what archgram is and why it exists. How it is built
lives in `ARCHITECTURE.md`, the input format in `docs/SPEC.md`, the visual
rules in `DESIGN.md` and the values in `design-system/tokens/`. Each fact
lives in one of them; the others refer to it. A feature that needs more
than a few lines here has its own document in `docs/features/`, with its
problem, requirements, limits and success criteria; this one says what it
is and points there. If a requirement changes, this document, or the
feature's own, changes first.

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

A diagram kept as text beside the code is cheap to change, but nothing
says when it must change. A box still names a module deleted a month
ago, a line still shows a call no longer made, and the diagram stays
plausible and wrong. With coding agents changing a project many times a
day, a diagram goes stale faster than anyone reads it closely enough to
notice.

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
| 0.4 | An edge's label lights with its signal; an optional "by archgram" credit; a spec named `<name>.archgram.yaml` draws `<name>.svg`, into a folder created when missing; `archgram spec`, the format the command reads; the `archgram` skill for Claude Code, which draws a project's architecture from its code |
| 0.5 | A flow may stop at a step, and the refusal goes back to where the flow began; a card's border is drawn from the arrow that reaches it, in a colour for passing and one for a refusal, in one of four styles; only a signal glows, faintly |
| 0.6 | A step's number lights as its signal passes it; the skill works with any coding agent that reads the Agent Skills format |
| 0.7 | A node or an edge names the code behind it, and archgram says when that code is gone ([docs/features/sources.md](features/sources.md)) |
| Later | The WASM package, for the browser; the PNG module, drawn from the same scene as the SVG by archgram's own rasterizer |

## 6. Functional requirements

### 6.1 Spec
- Input is JSON in the core and YAML through the optional module; both map
  to the same types.
- Unknown fields and unknown node kinds are errors, reported with their
  location, never ignored.
- The spec holds no coordinates. It may carry layout hints: direction,
  which nodes share a column or a row, and the order of nodes within one.
- A node or an edge may name the code behind it. `archgram check` fails
  and `archgram build` warns when that code is not there; it is never
  drawn. It finds what the code lost, not what it gained
  ([docs/features/sources.md](features/sources.md), Limit).

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
- The diagram is monochrome: every icon, a flow's signal and a lit card are
  in the text colour, black on light and white on dark, and the icon's
  shape tells a node's kind. Colour belongs to the technology logos and to
  what happens on a flow: one colour where a signal passes a card, another
  where a step refuses it (§6.4); cards, frames and connectors stay neutral
  otherwise. A project's own tokens may still give each category a hue of
  its own.
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
  line fills with that colour. An arrowhead takes the colour of the signal
  that reaches it.
- Only a signal glows, faintly, under its line, and more softly in dark,
  where a light line needs less; the spec may turn it off. Nothing else
  glows.
- A card is lit from the moment a signal reaches its arrowhead until every
  signal it sends has arrived. Its border takes the passing colour, drawn
  from the point where the arrow meets the card, both ways round, closing
  on the far side, no faster than the shortest hop. The first card of a
  flow, which no arrow reaches, closes its border where its signal leaves,
  as it leaves. The border keeps the card's own width; the card takes no
  fill, and the icon and the text keep their colours. Then the card
  returns to its usual look.
- The spec picks how the border is drawn: by default a bright head rides
  each growing end (spark); instead it may drain out through the arrow
  that leaves the card (drain), sweep once around from the arrowhead
  (ring), or fade back to the card's own edge while the card is lit
  (afterglow).
- A flow may stop at a step: the node there refuses the signal. A small ✕
  sits on the edge just before the refusing card's arrowhead, and the
  arrowhead and the card's border take the refusal colour. The refusal
  then travels back along the flow to the node where it began, whose
  border takes the refusal colour too. The refusing card keeps it until a
  later flow passes it, which draws its border in the passing colour.
  The spec may show that wait as a slow dashed border instead (pending).
- The passing and the refusal colours each hold 3:1 against the card, in
  both themes, as any graphic does.
- An edge's label is lit while its signal travels the edge: it takes the
  signal's colour, or the theme's text colour where the signal's colour
  would fall short of text contrast.
- Where the still image numbers the steps, the numbers sit above the
  signals, so a line passes under them. While the flows play, a number
  lights as its signal reaches it: its edge takes the passing colour,
  drawn from where the line enters, both ways round, and then fades as an
  arrowhead does. The still image keeps the plain number. A number keeps
  clear of a refusal's ✕, so both stay readable.
- archgram computes the timeline: branches start together, converging
  paths arrive together, the last node of a flow lights last.
- Native SMIL only; every animation stops under
  `prefers-reduced-motion`, and the still image shows what the spec
  chooses: nothing more, the flows in words under the legend, or each
  step's number on its lines. A refused step keeps its ✕ and the refusing
  card its border in the refusal colour. A screen reader hears each flow
  in words, a refused step included.

### 6.5 Outputs and interfaces
- A command-line tool, one native binary per platform, installed by hand
  or from npm (`npm install archgram`), so a Node project's scripts run
  it with no Rust toolchain.
- A spec named `<name>.archgram.yaml` (or `.yml`, `.json`) draws `<name>.svg`
  beside it, so a project keeps each spec next to its drawing; a folder
  given for the output is created when it does not exist.
- `archgram spec` prints the spec format this version reads (docs/SPEC.md,
  carried in the binary), so whoever writes a spec, a person or an agent,
  reads the format of the very command that draws it.
- A library, usable from Rust and, later through WASM, from the browser
  (§5).
- PNG, one file per theme, through the optional module (later, §5).
- A skill, `archgram`, in the repository's `skills/` folder, for any
  coding agent that reads the open Agent Skills format (a folder with a
  `SKILL.md`), Claude Code among them. It is installed with the `skills`
  command (`npx skills add byabas/archgram`), which puts it in the folder
  each agent reads, for a project or for every project; archgram carries no
  installer of its own (§6.6).

### 6.6 The `archgram` skill
- It draws a project's architecture for its README and docs, from the
  code and the documentation, so the agent writes the spec and archgram
  does the drawing. It replaces the `drawing-architecture-diagrams`
  skill.
- The user asks for it in words, with instructions such as which flow or
  which direction, or calls it by name where the agent offers that
  (`/archgram` in Claude Code); the agent also chooses it when asked for an
  architecture diagram. It never runs by itself.
- It is written to the Agent Skills format alone: its frontmatter keeps the
  format's fields, and its steps name no one agent's tools, so every agent
  that reads skills follows the same steps.
- A part is drawn only when a file in the project backs it, and the skill
  notes that file; an edge only where a line of code makes it, and the
  skill notes that line. It finds the parts by walking from the project's
  entry points, keeps one level of detail per diagram, and names each part
  after the file that decides, found by following the imports. Where the
  code and the documentation disagree, or the architecture is unclear, it
  asks the user; it asks for no other approval.
- It writes `docs/diagrams/<name>.archgram.yaml`, creating the folder when
  missing, and draws `<name>.svg` beside it with `npx archgram`: the
  project's own archgram when it has one, the latest otherwise. The spec is
  kept, so a later call changes it and draws again, and says what changed.
- It learns the spec format from that same command (`archgram spec`), so
  the format it writes always matches the command that draws it; it keeps
  no copy of the format that could fall behind.
- It works on its own: everything it needs is in its folder or comes from
  `npx archgram`, so it needs no plugin and nothing installed in the
  project. Its text changes only by review, like the code.
- Its evaluations live beside it: that it is chosen when asked for an
  architecture diagram and not otherwise, that it draws a project it is
  given, in more than one style of architecture, and that it asks when
  the architecture is unclear. They run on
  request, since each run is a paid model call.
- It draws in the project's own colours, found in this order: the
  project's DTCG tokens, through a mapping file; else the colours its
  code styles with (CSS custom properties, a Tailwind theme, Sass
  variables, a theme object), which the skill reads and writes as DTCG
  tokens and a mapping beside the spec; else colours the user gives in
  the request; else archgram's own palette. archgram's contrast check
  holds either way; a colour that fails it is reported with the reason.
- It recognises the style the system follows (layered, ports and
  adapters, microservices, a pipeline, a plugin host and others) and
  looks for that style's parts; a system that follows none is drawn from
  the general rules alone.
- A drawing holds what a reader takes in within thirty seconds, about ten
  nodes and twelve edges: parts with the same relations, which no question
  tells apart, are one node naming them all; what is still over goes to
  a second diagram, or is left out and said so.
- A drawing is at most 1,300 px wide, so its text stays readable in a
  README on GitHub; a wider one is drawn top to bottom or split in two.
- The skill judges a drawing from its spec and its size. It never starts a
  browser or takes a screenshot.
- When done, `archgram check` passes, the SVG is opened with the
  system's own viewer (`open`, `xdg-open`, `start`), and the skill lists
  each part with the file behind it and each edge with its line.

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
- Each feature with a document of its own meets the criteria there:
  [the code behind a diagram](features/sources.md#success-criteria).

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
| 0.12    | 2026-09-29 | `archgram spec` prints the format the command reads (§5, §6.5); the skill learns the format from it, runs one version throughout, and carries its evaluations (§6.6). |
| 0.13    | 2026-09-29 | The diagram is monochrome: icons, signals and lit cards in the text colour, colour for the logos alone; the legend lists the variants only (§6.3). |
| 0.14    | 2026-09-29 | A skill, `archgram`, not a plugin: one folder in `skills/` the user copies into a skills directory; it runs `npx archgram`, the project's own or the latest, not a pinned version (§5, §6.5, §6.6). |
| 0.15    | 2026-09-29 | 0.5: a flow stops at a refusing step and the refusal travels back to where the flow began; a lit card's border is drawn from its arrowhead in a passing or a refusal colour, in one of four styles (spark by default), with no fill and the icon and text unchanged; an arrowhead takes its signal's colour; only a signal glows, faintly, and more softly in dark, unless the spec turns it off; both colours hold 3:1 against the card (§5, §6.3, §6.4). The samples that chose this are in `docs/samples/stop/`. |
| 0.16    | 2026-09-29 | The skill backs every edge with the line of code that makes it, walks from the entry points at one level of detail, names each part after the file that decides, draws no wider than 1,300 px, and never starts a browser (§6.6). |
| 0.17    | 2026-09-29 | The skill recognises the system's style of architecture, merges parts with the same relations into one node, and keeps a drawing to about ten nodes and twelve edges; its evaluations cover a pipeline, ports and adapters, and a plugin host (§6.6). |
| 0.18    | 2026-10-01 | The skill is for any coding agent that reads the Agent Skills format, not Claude Code alone: written to the format's fields, asked for in words, and installed with the `skills` command into each agent's folder; archgram needs no installer of its own (§6.5, §6.6). |
| 0.19    | 2026-10-01 | A step's number sits above the signals and lights as a signal reaches it, traced in the passing colour from where the line enters; the still image keeps the plain number; a number keeps clear of a refusal's ✕ (§6.4). |
| 0.20    | 2026-10-02 | Features with more than a few lines of requirements get their own document in `docs/features/`, which this one points to. The problem names a stale diagram nobody notices (§1). 0.6 is in the scope; 0.7 names the code behind each node and edge, checked by `check` and `build`, with its limit (§5, §6.1, `docs/features/sources.md`). Each feature's own success criteria count here (§8). |
