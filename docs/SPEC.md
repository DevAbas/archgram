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

An edge means data or a call moving in the direction of the arrow. Two
edges between the same pair in the same direction are an error; use one
with a label.

## Flows

```json
{ "name": "a visit", "steps": ["browser", "api", "redis", "worker", "postgres"] }
```

A flow is a path through the diagram that archgram animates, in order.
Every consecutive pair of steps must be an edge, in that direction. A
spec without flows gives a still diagram. When several flows exist they
play one after another, in the order listed.

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

## Validation

A spec is rejected, with every problem listed at once and each located by
its JSON pointer (or its line and column in YAML), when:

- a required field is missing, a field is unknown, or a value is not one
  of those allowed;
- an id is duplicated, or an edge, flow, frame or hint names an id that
  does not exist;
- frames nest in a cycle, or a frame is empty;
- the nodes of an `order` hint do not share a frame (a frame keeps its
  nodes together, so a hint cannot sort them among others);
- a flow step is not followed by an edge to the next step;
- `tech` names a logo archgram does not carry (the error suggests the
  nearest slugs).

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
