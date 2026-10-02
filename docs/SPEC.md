# The archgram spec

The input format: what a diagram is made of and the rules a spec must
follow. Why the format looks like this is in `docs/PRD.md`; how archgram
reads it is in `ARCHITECTURE.md`; how each part is drawn is in `DESIGN.md`.

A spec describes the system, never the drawing. It has no coordinates, no
sizes and no colours. It names things, groups them and connects them;
archgram decides where they go and how they look.

## Formats

The core reads JSON. The optional YAML module reads YAML 1.2 and produces
the same spec, so every example below can be written either way; the
command line tells them apart by the file's extension (`.json`, `.yaml`,
`.yml`). Unknown fields are errors, not ignored, so a misspelt field is
caught instead of silently changing the diagram. In YAML, a key written
twice, a second document, and a tag outside YAML's core schema are errors
too, and a plain word that reads as a number or a boolean (`title: 2026`)
must be quoted to stay text.

A spec named `<name>.archgram.yaml` (or `.archgram.yml`, `.archgram.json`) draws
`<name>.svg` beside it, so each spec sits next to its drawing and the
drawing keeps a plain name; any other spec draws its own name with `.svg`.
`-o` names another file, and creates its folder when it does not exist.

## Top level

```json
{
  "archgram": 1,
  "title": "linkshort",
  "description": "A redirect reads the cache and queues the click; a worker counts clicks into Postgres.",
  "direction": "right",
  "card": "horizontal",
  "logo": "corner",
  "palette": "mono",
  "legend": true,
  "signal": "wire",
  "still": "none",
  "border": "spark",
  "wait": "solid",
  "glow": true,
  "credit": true,
  "nodes": [],
  "frames": [],
  "edges": [],
  "flows": [],
  "hints": {}
}
```

| Field | Required | Values | Default | Meaning |
|---|---|---|---|---|
| `archgram` | yes | `1` | | The spec format's version |
| `title` | yes | text | | The diagram's name; the SVG's `<title>` |
| `description` | yes | text | | The whole diagram in prose; the SVG's `<desc>`, read by screen readers |
| `direction` | no | `right`, `down` | `right` | The direction of the flow |
| `card` | no | `horizontal`, `vertical` | `horizontal` | The card style for every node |
| `logo` | no | `corner`, `inline`, `chip`, `icon` | `corner` | Where technology logos go: the card's corner, before the note (or the technology's name), a chip on the icon, or in place of the icon |
| `palette` | no | a palette name | `mono` | The palette; light and dark are chosen when rendering |
| `legend` | no | `true`, `false` | `true` | Whether to draw the legend; drawn only when it tells something apart: two categories or more, or a node with several instances or not ours |
| `signal` | no | `wire`, `spark`, `arc`, `comet`, `dot`, `pulse`, `current` | `wire` | How a flow's signal is drawn along the lines (DESIGN.md, Components: Signal) |
| `still` | no | `none`, `legend`, `numbers` | `none` | What the still image shows of the flows, where nothing moves: nothing more, each flow in words under the legend, or each step's number on its lines |
| `border` | no | `spark`, `drain`, `ring`, `afterglow` | `spark` | How a lit card's border is drawn from the arrow that reaches it: both ways round with a dot on each growing end, then draining toward the arrow that leaves, once round clockwise, or fading while the card is lit (DESIGN.md, Components: Signal) |
| `wait` | no | `solid`, `pending` | `solid` | How a refused card waits for a later flow to pass it: its refusal border as drawn, or marching round it as dashes (DESIGN.md, Components: Refusal) |
| `glow` | no | `true`, `false` | `true` | Whether a signal glows, faintly, in the styles that have a glow (`wire`, `spark`, `arc`); nothing else glows |
| `credit` | no | `true`, `false` | `true` | Whether to write a small "by archgram" in the drawing's bottom-right corner, hidden from screen readers |
| `nodes` | yes | list | | At least one node |
| `frames`, `edges`, `flows` | no | list | empty | |
| `hints` | no | object | empty | Layout hints, below |

The lists are empty above only to show the shape; complete specs are under
Examples.

## Nodes

```json
{ "id": "api", "kind": "service", "label": "API", "note": "3 replicas",
  "tech": "fastapi", "variant": "multi", "frame": "vpc" }
```

| Field | Required | Meaning |
|---|---|---|
| `id` | yes | Lowercase letters, digits and hyphens; unique among nodes and frames |
| `kind` | yes | One of the kinds below; decides the icon and the category |
| `label` | yes | The card's title |
| `note` | no | The card's subtitle: one short line |
| `tech` | no | A technology, by its Simple Icons slug (`postgresql`, `redis`, `react`); shows its logo. Lowercase letters, digits and `_` |
| `variant` | no | `single` (default), `multi` (several instances), `external` (not ours) |
| `frame` | no | The id of the frame the node sits in |
| `source` | no | The code behind the node: a path, or a list of paths for a node that stands for several parts (Sources, below) |

Kinds, by category:

| Category | Kinds |
|---|---|
| Core | `service`, `database`, `queue`, `cache`, `storage`, `users` |
| AI and LLM | `model`, `vector-store`, `tool`, `agent` |
| Build and tooling | `file`, `script`, `generated`, `check` |
| Clients | `browser`, `mobile`, `desktop` |

## Frames

```json
{ "id": "vpc", "label": "VPC" }
{ "id": "private", "label": "Private subnet", "parent": "vpc" }
```

A frame is a boundary around nodes: a network, a trust zone, a team's
service. `parent` nests one frame in another. A frame with no nodes and no
child frames is an error.

## Edges

```json
{ "from": "api", "to": "redis" }
{ "from": "api", "to": "postgres", "label": "on a miss", "style": "dashed" }
```

| Field | Required | Meaning |
|---|---|---|
| `from`, `to` | yes | Node ids; the edge points from `from` to `to` |
| `label` | no | A few words on the line |
| `style` | no | `solid` (default) or `dashed`, for a path taken only sometimes |
| `source` | no | The code that makes the edge, usually a path with the words of its line after `#` (Sources, below) |

An edge means data or a call moving in the direction of the arrow. Two
edges between the same pair in the same direction are an error; use one
with a label.

## Flows

```json
[
  { "name": "a visit", "steps": ["browser", "api", "redis", "worker", "postgres"] },
  { "name": "a question", "steps": ["chat", "llm", ["find", "count"], "index"] },
  { "name": "a commit with a raw colour", "steps": ["person", "precommit", "core", "checks"], "stop": "checks" }
]
```

A flow is a path through the diagram that archgram animates, in order. A
step is a node id, or a list of ids reached at once: a branch. Every node
of a step must be reached by an edge from a node of the step before it,
and when a step has several nodes, each must have an edge to a node of
the step after it. A spec without flows gives a still diagram. When
several flows exist they play one after another, in the order listed.

A flow may stop: `stop` names the node that refuses it, which must be
the flow's last step and a single node. A ✕ marks the line into it, and
the refusal travels back along the flow to the node where it began
(DESIGN.md, Components: Refusal). The refusing node stays marked until a
later flow passes it, so a refused flow followed by one that passes the
same node shows a retry.

archgram times the flows itself (DESIGN.md, Layout: Motion): a signal's
speed is fixed, so a longer line takes longer; a branch's signals leave
together; signals that meet at a node arrive together. A card is lit
from the moment a signal reaches its arrowhead until every signal it
sends has arrived; the last card of a flow stays lit until its border
has closed. A refused flow's refusal travels back before the next flow
starts. A screen reader hears each flow in words, after the description,
a refused flow ending with the node that refused it.

## Hints

The layout needs no help, but it accepts some.

```json
{
  "hints": {
    "first": ["browser"],
    "last": ["postgres"],
    "sameLayer": [["worker", "redis"]],
    "order": [["api", "worker"]]
  }
}
```

| Hint | Meaning |
|---|---|
| `first`, `last` | These nodes go in the first or last layer |
| `sameLayer` | Each list shares one layer (one column when the flow runs right) |
| `order` | Within a layer, each list keeps this order, top to bottom or left to right |

A hint that contradicts the edges (a node placed before the node that
feeds it) is an error, reported with both nodes named.

## Sources

A node or an edge may name the code behind it, so archgram can say when
that code is gone. Sources are never drawn: a spec gives the same SVG
with or without them. Why, and what the check cannot find, is in
`docs/features/sources.md`.

```yaml
nodes:
  - { id: worker, kind: service, label: Worker, source: ../../src/worker.rs }
  - { id: billing, kind: service, label: Billing, source: [../../src/charge.ts, ../../src/refund.ts] }
edges:
  - { from: worker, to: links, source: "../../src/worker.rs#links.insert(" }
```

A source is a path to a file or a folder, from the folder the spec is in,
with its folders separated by `/` on every system; it is never absolute.
After the first `#`, it may carry a few words copied from one line of
that file: the line that makes the edge, or that defines the part. The
words are looked for as written, anywhere in the file, so the source
holds while the file changes around them. `source` is one source or a
list of them, never an empty list.

`archgram check` reports, as problems, each path with nothing there,
each file that does not hold its source's words, and each folder given
words to look for; `archgram build` warns of the same and still draws.
Only a regular file is read, and nothing of it is printed. A spec
without sources reads no file.

In YAML, quote a source whose words hold ` #` or `: `: unquoted, YAML
reads the first as the start of a comment, dropping the rest of the
words, and the second as a key.

## Validation

A spec is rejected, with every problem listed at once and each located by
its JSON pointer (or its line and column in YAML), when:

- a required field is missing, a field is unknown, or a value is not one
  of those allowed;
- a text (the title, the description, a label, a note or a flow's name)
  holds a character XML does not allow, such as a control character
  other than tab, line feed and carriage return: the SVG would not open;
- an id is duplicated, or an edge, flow, frame or hint names an id that
  does not exist;
- frames nest in a cycle, or a frame is empty;
- the nodes of an `order` hint do not share a frame (a frame keeps its
  nodes together, so a hint cannot sort them among others);
- a hint contradicts the edges: a `sameLayer` group whose nodes an edge
  joins, or a path of edges leads from one to another (through other
  groups too), a `first` node an edge leads into, or a `last` node an
  edge leads out of;
- a flow step is not reached by an edge from the step before it, a node
  of a branching step has no edge to the step after it, a step lists no
  nodes, or a step lists a node twice;
- a flow's `stop` names no node, or a node that is not its last step, or
  its last step is a branch;
- `tech` names a logo archgram does not carry (the error suggests the
  nearest slugs);
- a `source` is empty, absolute, holds `\`, has nothing after its `#` or
  more than one line there, or is an empty list.

`archgram check` then holds each source to the code (Sources).

## Theme file

A project with its own design tokens can draw in its own colours: a
mapping file, `archgram.theme.json` by convention, passed with
`--theme-file` (and checked with `archgram theme check`), names the
project's resolver and which token fills each of archgram's colour roles.

```json
{
  "version": 1,
  "resolver": "design-system/tokens/design.resolver.json",
  "roles": {
    "canvas": "color.surface.page",
    "card": "color.surface.raised",
    "text": "color.text.primary",
    "icon-ai": "{color.accent.violet}"
  },
  "themes": {
    "light": { "inputs": { "theme": "light" } },
    "dark": { "inputs": { "theme": "dark" }, "roles": { "card-edge": "color.border.strong" } }
  }
}
```

| Field | Required | Meaning |
|---|---|---|
| `version` | yes | The mapping format's version. Only `1` exists |
| `resolver` | yes | The project's resolver document (Design Tokens 2025.10), relative to this file, in its folder or below |
| `roles` | no | Each archgram role's token, by its dotted id, with or without braces |
| `themes.light`, `themes.dark` | yes | `inputs`: the resolver's input for each modifier, to pick that theme; `roles`: roles this theme maps differently |

The roles are `badge`, `canvas`, `card`, `card-edge`, `connector`,
`frame`, `icon-ai`, `icon-build`, `icon-client`, `icon-core`,
`signal-core`, `signal-pass`, `signal-refusal`, `text` and `text-muted`
(DESIGN.md, Colors). A role the file
leaves out keeps the mono palette's colour. The file replaces the spec's
`palette`.

archgram reads the resolver's sets, modifiers (an input, else the
modifier's default), its resolution order (later sources win), `$ref` to
files beside it and to its own sets, inline tokens, `{alias}` values and
`$ref` JSON Pointers inside values, and `$type` from the groups above a
token. A colour may be in `srgb`, `srgb-linear`, `hsl`, `hwb`, `oklab`,
`oklch` or `display-p3`; in another space it needs a `hex` to fall back
on. Channels outside sRGB are clipped. A role's colour is opaque.

An import is refused, with every problem listed and each located by file
and JSON pointer, when the mapping or the resolver breaks its format; a
role is not archgram's; an input names no modifier or no context; a
modifier without a default gets no input; a file cannot be read, is a
remote reference, or is outside the mapping file's folder (by its real
path, so neither `..` nor a symlink leads out); a token does not exist (the nearest are suggested), is
not a colour, or aliases in a loop; a group uses `$extends`; a colour
cannot be read; or a pair of roles falls below the contrast DESIGN.md
requires, in either theme.

## Examples

### linkshort (JSON)

```json
{
  "archgram": 1,
  "title": "linkshort",
  "description": "The API creates short links and redirects. A redirect reads the URL from the Redis cache, falls back to Postgres on a miss, adds the click to a Redis stream and returns. A worker drains the stream in batches and adds the counts to Postgres, where the stats route reads them.",
  "nodes": [
    { "id": "browser", "kind": "browser", "label": "Visitor" },
    { "id": "api", "kind": "service", "label": "API", "note": "FastAPI, port 8000", "tech": "fastapi" },
    { "id": "cache", "kind": "cache", "label": "URL cache", "note": "1-day TTL", "tech": "redis", "frame": "redis" },
    { "id": "clicks", "kind": "queue", "label": "Click stream", "tech": "redis", "frame": "redis" },
    { "id": "worker", "kind": "service", "label": "Worker", "note": "up to 500 per read" },
    { "id": "postgres", "kind": "database", "label": "links", "note": "source of truth", "tech": "postgresql" }
  ],
  "frames": [ { "id": "redis", "label": "Redis" } ],
  "edges": [
    { "from": "browser", "to": "api" },
    { "from": "api", "to": "cache" },
    { "from": "api", "to": "postgres", "label": "on a miss", "style": "dashed" },
    { "from": "api", "to": "clicks" },
    { "from": "clicks", "to": "worker" },
    { "from": "worker", "to": "postgres" }
  ],
  "flows": [ { "name": "a visit", "steps": ["browser", "api", "clicks", "worker", "postgres"] } ]
}
```

### linkshort (YAML, the same spec)

```yaml
archgram: 1
title: linkshort
description: >-
  The API creates short links and redirects. A redirect reads the URL from
  the Redis cache, falls back to Postgres on a miss, adds the click to a
  Redis stream and returns. A worker drains the stream in batches and adds
  the counts to Postgres, where the stats route reads them.
nodes:
  - { id: browser, kind: browser, label: Visitor }
  - { id: api, kind: service, label: API, note: "FastAPI, port 8000", tech: fastapi }
  - { id: cache, kind: cache, label: URL cache, note: 1-day TTL, tech: redis, frame: redis }
  - { id: clicks, kind: queue, label: Click stream, tech: redis, frame: redis }
  - { id: worker, kind: service, label: Worker, note: up to 500 per read }
  - { id: postgres, kind: database, label: links, note: source of truth, tech: postgresql }
frames:
  - { id: redis, label: Redis }
edges:
  - { from: browser, to: api }
  - { from: api, to: cache }
  - { from: api, to: postgres, label: on a miss, style: dashed }
  - { from: api, to: clicks }
  - { from: clicks, to: worker }
  - { from: worker, to: postgres }
flows:
  - { name: a visit, steps: [browser, api, clicks, worker, postgres] }
```

### An agentic RAG pipeline (frames, AI kinds, external variants)

```json
{
  "archgram": 1,
  "title": "How an answer comes to be",
  "description": "Before use, the index script has a model extract a profile from each CV, checks every field against the text and embeds each chunk. At question time the chat sends the question to the ask route; the model calls tools over the index and the vectors, ends with present, and the app checks every candidate and page before the answer streams back.",
  "direction": "right",
  "nodes": [
    { "id": "cvs", "kind": "file", "label": "CV PDFs", "note": "30 files", "frame": "prep" },
    { "id": "extract", "kind": "model", "label": "Gemini", "note": "extracts profile", "tech": "googlegemini", "variant": "external", "frame": "prep" },
    { "id": "fieldcheck", "kind": "check", "label": "Field check", "note": "page per fact", "frame": "prep" },
    { "id": "embed", "kind": "model", "label": "Embedding", "note": "each chunk", "variant": "external", "frame": "prep" },
    { "id": "index", "kind": "storage", "label": "data/index", "note": "JSON, in memory" },
    { "id": "vectors", "kind": "vector-store", "label": "Pinecone", "note": "chunk vectors", "variant": "external" },
    { "id": "chat", "kind": "browser", "label": "Chat screen", "frame": "ask" },
    { "id": "route", "kind": "service", "label": "POST /api/ask", "note": "no CV text in the prompt", "frame": "ask" },
    { "id": "llm", "kind": "model", "label": "Gemini", "note": "tool-calling loop", "tech": "googlegemini", "variant": "external", "frame": "ask" },
    { "id": "tools", "kind": "tool", "label": "Exact tools", "note": "find, count, get", "frame": "ask" },
    { "id": "search", "kind": "tool", "label": "search_cv_text", "note": "BM25 + vectors", "frame": "ask" },
    { "id": "answercheck", "kind": "check", "label": "Answer check", "note": "ids and pages", "frame": "ask" }
  ],
  "frames": [
    { "id": "prep", "label": "Before use" },
    { "id": "ask", "label": "At question time" }
  ],
  "edges": [
    { "from": "cvs", "to": "extract" },
    { "from": "extract", "to": "fieldcheck" },
    { "from": "fieldcheck", "to": "index" },
    { "from": "cvs", "to": "embed" },
    { "from": "embed", "to": "vectors" },
    { "from": "chat", "to": "route" },
    { "from": "route", "to": "llm" },
    { "from": "llm", "to": "tools" },
    { "from": "llm", "to": "search" },
    { "from": "tools", "to": "index" },
    { "from": "search", "to": "vectors" },
    { "from": "llm", "to": "answercheck", "label": "present" },
    { "from": "answercheck", "to": "chat", "label": "streams" }
  ],
  "flows": [
    { "name": "a question", "steps": ["chat", "route", "llm", "answercheck", "chat"] }
  ]
}
```
