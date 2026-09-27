# Architecture

How archgram is built and why it is built that way. What it does is in
`docs/PRD.md`; the input format is in `docs/SPEC.md`; the visual rules are
in `DESIGN.md`; the design values are in `design-system/tokens/`.

## Bird's eye view

archgram is a pure function from a spec to a picture:

```
spec (JSON, or YAML through archgram-yaml)
  -> parse and validate      typed Spec, errors with their location
  -> model                   graph IR: nodes, frames, edges, flows, in spec order
  -> measure                 each label measured with the embedded font; box sizes
  -> layout                  layers, order, coordinates, frames
  -> route                   orthogonal edge paths around the boxes
  -> render                  SVG: shapes, theme, legend, optional animation
  -> rasterise (optional)    PNG per theme, through archgram-png
```

Every stage takes the previous stage's output and returns a new value. No
stage reads files, the clock or the environment, so the same code runs
natively, in Node and in the browser, and the same input always gives the
same bytes.

## Code map

The repository is one Cargo workspace.

| Crate | Holds | Depends on |
|---|---|---|
| `archgram-core` | Spec types, validation, IR, measuring, layout, routing, SVG, themes, the embedded font and its subsetter | `serde`, `serde_json`, `skrifa` |
| `archgram-yaml` | YAML to the core's `Spec`, with line and column in errors | `archgram-core`, `saphyr` |
| `archgram-png` | SVG to PNG, one theme at a time | `archgram-core`, `resvg`, `tiny-skia` |
| `archgram-cli` | The `archgram` binary: files, flags, exit codes | the three above |
| `archgram-wasm` | The npm package for Node and the browser | `archgram-core`, `archgram-yaml`, `wasm-bindgen` |
| `xtask` | Repository tasks run with `cargo xtask`, such as the dependency check; never shipped | `serde_json` |

`archgram-core` does no I/O. Anything that touches the file system or the
terminal lives in `archgram-cli`.

### Design tokens in the build

`archgram-core`'s build script reads `design-system/tokens/` at compile
time: it follows the resolver, resolves every alias, and writes one table
of values per palette and theme into the crate. The tokens stay the only
place a value is written; changing one rebuilds the crate, and no colour
or size is typed by hand in the code.

## The pipeline, stage by stage

### Parse and validate

`serde_json` reads the spec into Rust types with `deny_unknown_fields`, so
a misspelt field is an error, not a silent default. Validation then checks
what types cannot express: every edge names existing nodes, frame nesting
has no cycles, flows follow existing edges. Errors carry a JSON pointer in
the core and a line and column through `archgram-yaml`.

`archgram-yaml` walks `saphyr`'s node tree and builds the same `Spec`
itself. There is no serde bridge in between, so the mapping stays small,
readable and ours.

### Model

The IR stores nodes, edges and frames in vectors and refers to them by
index. Iteration always follows spec order. Hash maps are never iterated;
where a lookup is needed, the key is an index or the map is ordered. This
is the first half of determinism.

### Measure

Labels are measured with the advance widths of Geist, read with
`skrifa`. The static Regular and Medium TTF files ship inside the binary
under their OFL licence, so measurements do not depend on the fonts
installed on a machine. A box's size comes from its kind's template (icon
area, padding) and its measured label.

### Layout

A layered layout in the Sugiyama tradition, in these steps:

1. Cycle removal. Edges that point back against the flow are reversed for
   the duration of the layout, chosen with a greedy feedback-arc-set
   heuristic; ties break by spec order.
2. Layering. Each node gets a layer (a column when the flow runs right)
   by network simplex, which keeps edges short. Layout hints fix a
   node's layer. Edges spanning several layers are split by dummy nodes.
3. Crossing reduction. Layers are swept back and forth, ordering nodes by
   the barycentre of their neighbours, then adjacent pairs are swapped
   while that removes crossings. Spec order is the starting order and the
   tie-break; the number of sweeps is fixed.
4. Coordinates. Brandes–Köpf alignment gives straight edges where
   possible and balanced positions elsewhere: four alignments (towards the
   layer above or below, from either end), aligned to the narrowest, each
   vertex at the mean of its two middle values. Blocks are placed by a
   longest-path pass over the graph of blocks rather than the paper's class
   shifts, which its 2020 erratum shows can misplace classes.
5. Frames. A frame's contents are laid out first, then the frame joins its
   parent's layout as one node with the size of its contents and its
   padding. Edges that cross a frame's border are attached to the border
   during the parent's layout and joined to their inner ends when routed.
   This is the hardest part of the engine and gets the most tests.
6. Direction. The layout is computed left to right; top to bottom is a
   transform of the result.

### Route

Edges are routed after the layout has placed the cards and the bends of
long edges, using the layered structure itself (as ELK's layered router
does) rather than a general search:

- Every edge is a chain of hops between adjacent layers. A hop runs through
  the gap between two layers, where no card stands, so an edge cannot pass
  through a card by construction.
- A hop whose two ends are level is one straight segment; otherwise it is a
  symmetric Z: out of its start, onto a vertical segment (a track) in the
  gap, and into its end. A long edge runs straight through the layers it
  crosses, at the place the layout kept for it, `spacing.edge-edge` clear of
  the cards beside it.
- Each card's edges on one side get ports: the side's middle for one edge,
  spread `spacing.edge-edge` apart around it for several, ordered by where
  their other ends lie so they do not cross as they leave.
- Vertical segments that overlap in a gap get separate tracks. For each
  overlapping pair, the order that crosses fewer of the other hop's
  horizontal segments wins; each segment then takes the lowest track clear
  of those it overlaps. A gap needing more tracks than it holds widens.
- An edge label sits on the edge's longest straight segment, on a patch of
  canvas colour.

A general orthogonal router (visibility graph and A*, as in libavoid) is
not needed while every edge follows the layers; it stays an option should
frames or free placement ever call for it.

### Render

The SVG is built as a string with fixed number formatting (two decimals)
and a fixed attribute order, the second half of determinism. Each node
kind has a shape function; the theme becomes CSS custom properties, with
the dark values under `prefers-color-scheme`, or one file per theme on
request. The legend is generated from the kinds and variants the diagram
uses. Technology logos come from a vendored subset of Simple Icons.

The text is drawn in the same font it was measured with. archgram's own
subsetter cuts Geist down to the glyphs the diagram uses (keeping the
tables a renderer needs: `head`, `hhea`, `maxp`, `hmtx`, `cmap`, `loca`,
`glyf`, `post`, `name`, `OS/2`, with composite glyphs followed to their
parts) and embeds the result as a data URI in an `@font-face` rule, so
the text looks the same on every machine. The subsetter handles static
TrueType outlines only; variable fonts are out of scope. `--system-font`
skips the embedding and falls back to the system font stack.

### Animate

Flows turn into a timeline: each hop lasts in proportion to its path
length, within a minimum and a maximum; a branch starts all its signals
together; converging signals arrive together; each node lights when its
signal arrives. The output is SMIL: one cycle shared by every animation,
`keyTimes` checked before writing, everything that moves hidden under
`prefers-reduced-motion`.

### Rasterise

`archgram-png` renders a single-theme SVG with `resvg`. `resvg` reads
neither `@media` rules nor SMIL, so archgram writes a separate SVG per
theme for it and keeps the still frame in the base attribute values.

## Invariants

These hold for every output and are checked by tests on every change.

- The same spec gives byte-identical output on every platform.
- No two boxes overlap; no edge passes through a box it does not touch.
- Every edge is orthogonal.
- Every text pair meets WCAG 2.1 AA in both themes.
- Every `keyTimes` starts at 0, ends at 1 and increases.
- `archgram-core` performs no I/O.
- archgram's own crates contain no `unsafe` code
  (`#![forbid(unsafe_code)]`).

## Dependencies and supply chain

| Crate | Why | Licence |
|---|---|---|
| `serde`, `serde_json` | Reading the spec | MIT or Apache-2.0 |
| `skrifa` | Font metrics; the font parser Chrome uses | MIT or Apache-2.0 |
| `saphyr` | YAML, in the optional module | MIT or Apache-2.0 |
| `resvg`, `tiny-skia` | PNG, in the optional module | Apache-2.0 or MIT; BSD-3-Clause |
| `wasm-bindgen` | The WASM package | MIT or Apache-2.0 |

- `Cargo.lock` is committed and versions are pinned; builds run with
  `--locked`, so no dependency updates itself.
- Each crate enables only the features it needs.
- Adding or updating a dependency needs the owner's approval, after reading
  its licence, its owner and its advisories.
- `cargo xtask deps` checks the whole tree without third-party tools: the
  licence of every package, from `cargo metadata`, against the allowed
  list (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0,
  OFL-1.1 for fonts); the source of every package outside the workspace,
  which must be crates.io; and every version in `Cargo.lock` against the
  RustSec advisory database, fetched as a git repository. Any finding fails
  the check.
- Cargo's own `cargo tree` shows where each indirect dependency comes from.

## Testing

There are no test dependencies. Golden-file comparison and a seeded random
spec generator are small helpers inside the workspace.

- Unit tests per stage, each on small hand-made graphs.
- Property tests of the invariants above on specs from the seeded
  generator.
- Snapshot tests of the SVG for a fixed set of example specs, including
  the two diagrams of ai-powered-cv-screener.
- A determinism job that renders every example on macOS, Linux and
  Windows and compares the bytes.
- Benchmarks of layout and rendering against the budgets below.

## Performance budget

| Measure | Budget |
|---|---|
| Layout and SVG, 100 nodes, native | under 50 ms |
| CLI start to written file, small spec | under 100 ms |
| `archgram-core` WASM, optimised | under 500 KB |
| PNG, 1600 x 1000, native | under 100 ms |

## Distribution

- Native binaries for macOS, Linux and Windows, built in CI and attached
  to each release.
- One npm package built with `wasm-bindgen-cli` and `wasm-opt`, for Node
  and the browser; the PNG module is a separate, optional package.
- Crates on crates.io once the API is stable.

## References

These are read to learn how others solved the same problems. None is a
source of truth for archgram, and no code is copied from any of them.

Papers
- K. Sugiyama, S. Tagawa, M. Toda. Methods for Visual Understanding of
  Hierarchical System Structures. IEEE Trans. SMC, 1981.
- P. Eades, X. Lin, W. F. Smyth. A Fast and Effective Heuristic for the
  Feedback Arc Set Problem. Information Processing Letters, 1993.
- E. R. Gansner, E. Koutsofios, S. C. North, K.-P. Vo. A Technique for
  Drawing Directed Graphs. IEEE Trans. Software Engineering, 1993.
- U. Brandes, B. Köpf. Fast and Simple Horizontal Coordinate Assignment.
  Graph Drawing, 2001.
- G. Sander. Layout of Compound Directed Graphs. Technical report,
  Universität des Saarlandes, 1996.
- M. Wybrow, K. Marriott, P. J. Stuckey. Orthogonal Connector Routing.
  Graph Drawing, 2009.

Engines

| Project | Language | Licence | Read for |
|---|---|---|---|
| ELK (Eclipse Layout Kernel) | Java | EPL-2.0 | Phase structure and options of a layered layout, hierarchy handling |
| Graphviz `dot` | C | EPL-2.0 | Network simplex layering, mincross |
| dagre | JavaScript | MIT | A compact end-to-end layered layout |
| dagro (D2) | Go | MIT | Compound nodes in a layered layout, and its test cases |
| libavoid (Adaptagrams) | C++ | LGPL-2.1 | Orthogonal routing and nudging |
| saphyr, serde-saphyr | Rust | MIT or Apache-2.0 | Walking a YAML tree and reporting positions |
