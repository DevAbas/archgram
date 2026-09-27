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
| `archgram-core` | Spec types, validation, IR, measuring, layout, routing, SVG, themes, the flows' timing and animation, the embedded font and its subsetter | `serde`, `serde_json`, `skrifa` |
| `archgram-icons` | Technology logos: a pinned release of Simple Icons as data, written by `cargo xtask icons <tag>`, through the core's `Logos` trait | `archgram-core` |
| `archgram-yaml` | YAML to the core's `Spec`, with line and column in errors | `archgram-core`, `saphyr-parser` |
| `archgram-png` | SVG to PNG, one theme at a time | `archgram-core`, `resvg`, `tiny-skia` |
| `archgram-cli` | The `archgram` binary: files, flags, exit codes | the crates above |
| `archgram-wasm` | The npm package for Node and the browser | `archgram-core`, `archgram-yaml`, `wasm-bindgen` |
| `xtask` | Repository tasks run with `cargo xtask`, such as the dependency check; never shipped | `serde_json` |

`archgram-core` does no I/O. Anything that touches the file system or the
terminal lives in `archgram-cli`. The core carries no logos either: it
draws those it is given through `Logos`, so the WASM package stays small
and can hand over only the logos a diagram names, while the CLI passes
all of `archgram-icons`.

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

`archgram-yaml` reads `saphyr-parser`'s events (YAML 1.2) into its own
tree, each value with its line and column, writes the tree as JSON, one
value per line, and lets the core read and check it. Each problem the
core finds comes back to the YAML: a JSON line maps to its key's position,
a JSON pointer to its value's. So every rule lives once, in the core, and
a YAML author still sees `line:column`. The tree is the crate's own, not
saphyr's loader, because the loader keeps the last of two equal keys and
expands aliases without limit: here a key twice, a second document, a tag
outside the core schema, nesting past 64 levels and aliases expanding
past 10 000 values are errors. Plain scalars follow the core schema
(null, booleans, integers, floats; the rest is text); a plain number
where text belongs is reported with the advice to quote it.

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
area, padding) and its measured label. A node with several instances
takes the footprint of its whole stack, the front card plus
`card.multi-offset` twice in each direction, so the layout keeps the
stack clear of its neighbours; its anchor across the layers is the front
card's middle, so a straight edge meets the card a reader sees first.
Along the flow the front card takes its column's width like any other
card, and the stack reaches past it into the gap (to the right when the
flow runs right, up when it runs down).

One list of text runs, each with its weight (node titles and notes, edge
labels), feeds both the embedded font's subset and the warning for
characters Geist lacks, so neither can miss a text the other covers.

### Layout

A layered layout in the Sugiyama tradition. First the spec is split into
units that share nothing: no edge, no hint group, no top-level frame
(component packing, as Graphviz `pack` and ELK's
`separateConnectedComponents` do). Each unit with edges runs the steps
below on its own. A unit without edges is set out as a grid, and lone
nodes without edges share one grid. The unit with the most nodes comes
first; the others follow in the order of their first node, in rows below
it no wider than the widest unit, `spacing.pack` apart. When no unit has
edges at all, every node goes in one grid with about as many columns as
rows. Hints are checked on the whole spec, so an error points at the
spec's own hint; cycle removal treats each connected part on its own, so
the units would find the same errors. The steps:

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
5. Frames, in the one global layout, as dagre lays out compound graphs
   (Sander 1996; Forster 2002), not frame by frame: laying each frame out
   alone and then treating it as one big node (ELK's separate children)
   makes rigid blocks with long gaps, and edges between frames need ports
   on their borders. Here nodes keep their global layers and:
   - a frame spans the layers from its first descendant node to its last;
     each dummy of a long edge sits in the innermost frame it passes
     through (dagre's `parentDummyChains`);
   - on every layer it spans, a frame has a first and a last border
     vertex, chained across the layers;
   - crossing reduction sorts a layer frame by frame, a child frame as one
     item at the mean key of what it holds, between its borders; frames
     side by side keep the order they had in the layer just placed; a swap
     only trades two vertices of one frame. An `order` hint must stay
     within one frame, so it never breaks one apart;
   - coordinates align each border chain first, as one block, and no other
     alignment may cross it, so a frame is a rectangle holding its own and
     nothing else. Its padding (with room for its name on top, across the
     layers when the flow runs right) separates its borders from what it
     holds. Should the blocks ever form a cycle, every non-border vertex
     stands alone and the layout carries on; debug builds stop there, and
     the tests would;
   - along the flow, the gaps keep each frame's padding where it starts and
     ends, nested frames adding theirs inside, and tracks stay outside
     them. Flowing down, the name's room is part of that padding; a frame
     is never shorter or narrower than its name.
   Layering ignores frames, so a frame whose nodes lie far apart spans the
   layers between them; dagre's nesting edges would pull them together
   and remain an option.
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
- Each card's edges on one side get ports. The plain ones share one port
  as a bundle, and a bundle turns at one track: edges leaving a side leave
  as a trunk that forks in the gap, edges entering a side merge into one
  point along one trunk (the look of hand-drawn flow diagrams). A hop in
  both kinds of bundle follows the one leaving. An edge whose label sits
  just past the card, and an edge reversed against the flow, get ports of
  their own; all ports are spread `spacing.edge-edge` apart around the
  side's middle, ordered by where their other ends lie (a bundle by its
  middle) so they do not cross as they leave. A port whose
  edge carries its label just past the card keeps its neighbours that
  label's reach plus half of `spacing.edge-edge` away. When a side is too
  short for all its ports every gap shrinks in proportion, and a label may
  then touch a neighbouring edge from the same card.
- Vertical segments that overlap in a gap get separate tracks. For each
  overlapping pair, the order that crosses fewer of the other hop's
  horizontal segments wins; each segment then takes the lowest track clear
  of those it overlaps. Tracks keep clear of the cards on both sides: a
  bend's radius (or a label's room) after the card an edge leaves, and
  before the card it points at a bend's radius, twice the arrowhead's
  length and its gap, so the last bend is whole and the arrowhead sits on
  a straight run. A gap needing more room than it has widens.
- An edge label gets room of its own, as in dagre and ELK. On an edge
  longer than one layer it stands in for the middle dummy vertex, sized to
  the label, so crossing reduction and coordinates keep it clear of cards.
  On an edge between neighbouring layers, the gap it leaves into reserves
  the label's length plus `spacing.edge-edge` on each side before its
  tracks, and the label sits on the segment leaving the first card, in
  that room, straight edge or not. The label is drawn on a patch of canvas
  colour.

A general orthogonal router (visibility graph and A*, as in libavoid) is
not needed while every edge follows the layers; it stays an option should
frames or free placement ever call for it.

### Render

The SVG is built as a string with fixed number formatting (two decimals)
and a fixed attribute order, the second half of determinism. Each node
kind has a shape function; the theme becomes CSS custom properties, with
the dark values under `prefers-color-scheme`, or one file per theme on
request. The legend is generated from the categories and variants the
diagram uses and laid out below it (`layout::legend`), in rows no wider
than the diagram; its text is part of the text runs, so the font subset
carries it. A technology logo is its Simple Icons path scaled from the
24 by 24 grid into one of four places: the card's corner, the start of
its second line, a round chip on the badge (all in `color.text-muted`),
or the badge in place of the icon, in the category's hue. A card whose
logo goes in the corner keeps its width of room beside the title; an
inline logo widens the second line, which without a note shows the
technology's name from the logo set. The set also carries each brand's
colour, which the animation shows while a signal lights the card. `tech` is checked against the logos given, with
the nearest slugs suggested; with none given it is neither checked nor
drawn.

The text is drawn in the same font it was measured with. archgram's own
subsetter cuts Geist down to the glyphs the diagram uses (keeping the
tables a renderer needs: `head`, `hhea`, `maxp`, `hmtx`, `cmap`, `loca`,
`glyf`, `post`, `name`, `OS/2`, with composite glyphs followed to their
parts) and embeds the result as a data URI in an `@font-face` rule, so
the text looks the same on every machine. The subsetter handles static
TrueType outlines only; variable fonts are out of scope. `--system-font`
skips the embedding and falls back to the system font stack.

### Animate

Flows turn into a timeline (`motion`), in whole milliseconds. Each hop
lasts in proportion to its edge's length as drawn, within a minimum and
a maximum: the render stage splits every edge into its pieces (lines,
quarter circles, the S curves of short jogs) and measures them with
`+ * / sqrt` only, a curve by Gauss–Legendre quadrature, so the timing
is the same on every machine. A step may branch: its signals leave
together, and signals meeting at a node arrive together, when the slowest
does. A card is lit from its signal's arrival until every signal it
sends has arrived; lit times of one card closer than two fades are
joined, so one fade ends before the next begins.

The output is SMIL, which runs where CSS and scripts do not (an `<img>`,
GitHub): one cycle shared by every animation; each signal follows its
edge's own path by `mpath`; a lit card is a border and tint over the
card, a brand-coloured copy over the logo, each an opacity animation.
Everything starts invisible, so a reader that runs no animation shows
the still diagram. `KeyTrack` is the only writer of `keyTimes`, and holds
SMIL's rules on them (Invariants). A brand colour too faint on a theme's
card falls back to the text colour; the contrast is computed from a
fixed table of the sRGB curve (`color`), not the platform's `powf`.
Under `prefers-reduced-motion` the signals, lit cards and brand colours
are hidden. What the still image shows of the flows is the spec's: the
flows in words, laid out under the legend (`layout::legend`), or each
step's number on its lines, placed by the render stage clear of cards
and labels.

### Rasterise

`archgram-png` renders a single-theme SVG with `resvg`. `resvg` reads
neither `@media` rules nor SMIL, so archgram writes a separate SVG per
theme for it and keeps the still frame in the base attribute values.

## Invariants

These hold for every output and are checked by tests on every change.

- The same spec gives byte-identical output on every platform.
- No two boxes overlap; no edge passes through a box it does not touch.
- Every edge is orthogonal.
- A frame holds its nodes and child frames with its padding around them;
  nothing else reaches into it, and frames that do not nest stay apart.
- Every text pair meets WCAG 2.1 AA in both themes.
- Every `keyTimes` starts at 0, ends at 1 and never goes back, with one
  value per time and one spline per interval.
- `archgram-core` performs no I/O.
- archgram's own crates contain no `unsafe` code
  (`#![forbid(unsafe_code)]`).

## Dependencies and supply chain

| Crate | Why | Licence |
|---|---|---|
| `serde`, `serde_json` | Reading the spec | MIT or Apache-2.0 |
| `skrifa` | Font metrics; the font parser Chrome uses | MIT or Apache-2.0 |
| `saphyr-parser` | YAML events with positions, in the optional module; with it `arraydeque` and `thiserror` | MIT or Apache-2.0 |
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
- Logo data is pinned like a dependency: `archgram-icons/data/RELEASE`
  names the Simple Icons tag and commit, `cargo xtask icons <tag>`
  rewrites the data from that tag alone, and an update is a reviewed
  change. The data is CC0-1.0, on the allowed list for that crate; a logo
  carrying a licence of its own other than CC0 is left out, and
  `provenance.tsv` keeps each logo's source and brand guidelines.

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
- One SVG carries both themes by default. `archgram build --split-themes`
  lays the diagram out once and writes `<name>.light.svg` and
  `<name>.dark.svg`, one theme each, for pages that choose per reader:

  ```html
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="diagram.dark.svg">
    <img alt="…" src="diagram.light.svg">
  </picture>
  ```

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
