---
# archgram's visual rules, in the DESIGN.md format (google-labs-code/design.md, alpha).
# The values live in the design tokens that `imports:` names (W3C Design Tokens, DTCG 2025.10);
# this file names them by their ids and holds none, as the format's planned token import
# intends (google-labs-code/design.md#13). `components:` is the contract: which roles each
# part of a diagram reads.
version: alpha
name: archgram
description: Minimalist architecture diagrams. Neutral surfaces, colour only in technology logos and in what happens on a flow, one shape per kind of thing.
imports: ./design-system/tokens/design.resolver.json
components:
  canvas:
    backgroundColor: "{color.canvas}"
    rounded: "{rounded.canvas}"
  node-card:
    backgroundColor: "{color.card}"
    textColor: "{color.text}"
    typography: "{typography.title}"
    rounded: "{rounded.card}"
    padding: "{card.padding}"
    height: "{card.horizontal-height}"
  node-card-vertical:
    backgroundColor: "{color.card}"
    textColor: "{color.text}"
    typography: "{typography.title}"
    rounded: "{rounded.card}"
    width: "{card.vertical-min-width}"
    height: "{card.vertical-height}"
  node-subtitle:
    textColor: "{color.text-muted}"
    typography: "{typography.subtitle}"
  node-external:
    backgroundColor: "{color.canvas}"
    textColor: "{color.text}"
    rounded: "{rounded.card}"
  icon-badge:
    backgroundColor: "{color.badge}"
    rounded: "{rounded.badge}"
    size: "{card.horizontal-badge}"
  icon-badge-vertical:
    backgroundColor: "{color.badge}"
    rounded: "{rounded.badge}"
    size: "{card.vertical-badge}"
  icon-core:
    textColor: "{color.icon-core}"
  icon-ai:
    textColor: "{color.icon-ai}"
  icon-build:
    textColor: "{color.icon-build}"
  icon-client:
    textColor: "{color.icon-client}"
  logo-corner:
    textColor: "{color.text-muted}"
    size: "{card.logo-corner}"
  logo-chip:
    backgroundColor: "{color.card}"
    textColor: "{color.text-muted}"
    size: "{card.logo-chip}"
  logo-inline:
    textColor: "{color.text-muted}"
    size: "{card.logo-inline}"
  logo-icon:
    size: "{card.horizontal-icon}"
  frame:
    textColor: "{color.frame}"
    rounded: "{rounded.frame}"
    padding: "{spacing.frame-padding}"
  frame-label:
    textColor: "{color.text-muted}"
    typography: "{typography.frame-label}"
  connector:
    textColor: "{color.connector}"
    rounded: "{rounded.connector}"
    size: "{arrowhead.length}"
  legend:
    textColor: "{color.text-muted}"
    typography: "{typography.legend}"
  credit:
    textColor: "{color.connector}"
    typography: "{typography.legend}"
  signal:
    size: "{signal.dot}"
    backgroundColor: "{color.signal-core}"
  lit-card:
    textColor: "{color.signal-pass}"
    size: "{stroke.card}"
  refused-card:
    textColor: "{color.signal-refusal}"
    size: "{stroke.card}"
  refusal-mark:
    textColor: "{color.signal-refusal}"
    size: "{refusal.mark}"
  step-number:
    backgroundColor: "{color.card}"
    textColor: "{color.text}"
    typography: "{typography.legend}"
    size: "{signal.number}"
  lit-step-number:
    textColor: "{color.signal-pass}"
    size: "{stroke.card}"
---

# archgram: Design System

## Overview

A diagram is read in about thirty seconds by someone who did not build the
system. Everything here serves that reader: the eye should find what each
box is, how things connect and where the flow goes, and nothing else should
compete for attention.

The language is minimalist. Surfaces and text are neutral greys; colour
appears only in technology logos and in what happens on a flow: a card a
signal passes, a step that refuses it. There are no shadows, no gradients
and no glow but a signal's faint one. The shapes
carry the meaning, following the conventions architecture diagrams have
settled on: every kind of thing has its own icon, several instances of a
thing are stacked cards, a thing we do not own has a dashed border, and a
boundary (a VPC, a trust zone) is a dashed frame.

This document holds the rules and the reasons. It holds no values. Every
value is a design token in `design-system/tokens/`, named here by its id
(`color.card`, `rounded.card`, `typography.title`). The front matter holds
two things: `imports:`, which names the tokens, and `components:`, the
contract of which roles each part of a diagram reads.

### Reading the tokens

The tokens come in three tiers.

- Palettes (`palette.light.*`, `palette.dark.*`, in `palettes/<name>.tokens.json`)
  hold values only. The renderer never reads them.
- Roles (`color.*`, in `themes/light.tokens.json` and `themes/dark.tokens.json`)
  say what a colour is for, and point into the palette.
- Components (the front matter above) say which roles each part reads.

`design.resolver.json` resolves them in this order: the `foundation` set
(type, shape, motion: the same everywhere), then the `palette` modifier
(which palette, `mono` by default), then the `theme` modifier (`light` or
`dark`). Palette and theme vary independently: a new palette is one file
defining the same entries, and light and dark work for it at once.

## Colors

The one palette today is `mono`: neutral greys, and its four icon roles
are the text colour, black on light and white on dark, so a diagram is
monochrome and colour belongs to the technology logos and to a flow's
outcome: green where a signal passes a card, red where a step refuses it.

- **Canvas** (`color.canvas`): the diagram's background. Near-white in
  light, near-black in dark, painted by the diagram itself so it reads on
  any page.
- **Card** (`color.card`): the fill of a node. Pure white in light; one
  step lighter than the canvas in dark, so cards rise above it in both.
- **Card edge** (`color.card-edge`): the border of a node. Faint; it
  separates a card from the canvas and never frames it.
- **Badge** (`color.badge`): the neutral square behind a node's icon.
- **Text** (`color.text`) and **text muted** (`color.text-muted`): titles,
  then subtitles, frame names, the legend and technology logos.
- **Connector** (`color.connector`) and **frame** (`color.frame`): edges,
  arrowheads and frame borders. Strong enough to follow, quieter than
  text.
- **Icon hues** (`color.icon-core`, `color.icon-ai`, `color.icon-build`,
  `color.icon-client`): the stroke of a node's icon, by its category. In
  `mono` all four are `color.text`; a project's own tokens may give each
  category a hue of its own. Nothing else takes these colours except a
  flow's signal and the edge label it lights.
- **Signal core** (`color.signal-core`): the bright centre of a signal's
  dot, in the styles that have one. Only the signal takes it.
- **Signal pass** (`color.signal-pass`): the border of a card a flow's
  signal passes, drawn while the card is lit (Components: Signal). Green,
  so passing reads at once.
- **Signal refusal** (`color.signal-refusal`): a step that refuses a flow:
  the ✕ on its line, the arrowhead after it, the refusal on its way back
  and the borders of the cards it marks (Components: Refusal). Red, the
  one colour a reader already takes for "stopped".

Rules:

- Colour lives in technology logos and in a flow's outcome. Icon lines
  and a flow's signal take the icon roles, which in `mono` are the text
  colour; cards, badges, frames and connectors stay neutral, whatever the
  category. A card's border turns `color.signal-pass` while a signal lights
  it and `color.signal-refusal` while a refusal marks it; its fill, icon
  and text never change (Components: Signal, Refusal).
- Hues are muted and there is no neon; a logo's brand colour is its
  brand's own (Components: Technology logo). Only a signal glows, faintly,
  in the styles that have a glow, so it reads against the connector it
  runs along; the spec may turn it off (`glow: false`). A refusal's marks
  and a lit border never glow.
- A flow's signal takes the icon hue of the node it leaves.
- Technology logos are drawn in their brands' own colours, always; a
  logo whose brand gives none is in `color.text-muted`.
- Contrast follows WCAG 2.1 AA in both themes: text 4.5:1 against what it
  sits on; icon lines, connectors, frame borders and a flow's pass and
  refusal colours 3:1 against their background (1.4.11): the pass colour
  against the card, the refusal colour against the card and the canvas.
- A project's own tokens may fill the roles instead of a palette
  (docs/SPEC.md, Theme file); they are held to every rule here, and an
  import that misses a contrast is refused.
- An external node keeps its category's icon hue; its dashed border, not
  a grey icon, says it is not ours.

## Typography

Four styles, all from `font.sans`:

- `typography.title`: a node's name. The one weight step in the diagram,
  so names are read first.
- `typography.subtitle`: a node's detail line and a technology name.
- `typography.frame-label`: a frame's name, in capitals with open letter
  spacing, so it reads as a label for a region, not as a node.
- `typography.legend`: the legend's entries.

Labels are sentence case except frame names. Text is measured with the
font the renderer embeds, so a card is exactly as wide as its title needs.
The renderer embeds a subset of that font, holding only the characters the
diagram shows, so the reader sees the text as it was measured. On request
the text is left to the reader's system font, which is why `font.sans`
lists a fallback stack.

## Layout

- The flow runs one way, left to right by default or top to bottom on
  request. Nodes sit in layers; `spacing.layer-layer` separates layers,
  `spacing.node-node` separates nodes within one, and `spacing.edge-edge`
  separates edges that run side by side or pass a node.
- A card is horizontal by default (icon on the left, text on the right) or
  vertical on request (icon above the text). Horizontal suits wide flows
  and long names; vertical suits few nodes with short names. One diagram
  uses one style.
- A horizontal card has a fixed height (`card.horizontal-height`) and
  grows in width with its title, never below `card.horizontal-min-width`.
  A vertical card has a fixed height (`card.vertical-height`) and grows in
  width with its title, never below `card.vertical-min-width`.
- Cards in one column share the width of its widest card, so their sides
  line up; a column reads as one step of the flow. For several instances
  the front card takes that width and the stack reaches past it.
- Parts of a diagram that share nothing are laid out apart: the largest
  first, the others in rows below it, `spacing.pack` apart. Nodes without
  edges line up in a grid of equal cells instead of standing in the first
  column of an unrelated flow.
- A frame contains its nodes with `spacing.frame-padding` on every side
  and room at the top for its name (`spacing.frame-label`).
- The legend (Components: Legend) sits under the diagram,
  `spacing.legend` below it, left-aligned, its entries
  `spacing.legend-entry` apart, in rows `spacing.legend-row` apart no
  wider than the diagram.
- `spacing.margin` surrounds everything.

Motion:

- A diagram is still by default. Motion appears only along the flows the
  spec names, to show order: where a request starts, where it branches,
  where it ends.
- A signal travels at `motion.speed`, so a hop lasts in proportion to its
  length, never less than `motion.hop-min` nor more than `motion.hop-max`,
  eased with `motion.ease`. It waits `motion.hop-gap` at each card before
  it leaves; `motion.rest` passes before the cycle repeats. A branch's
  signals leave together; signals that meet at a card arrive together.
- A lit card's border is drawn from the arrowhead round the card, over
  `motion.hop-min`, so it never moves faster than a signal; the last card
  of a flow stays lit until its border has closed.
- A refusal travels back along its flow, `motion.refusal-hop` for each
  line, and `motion.rest` passes after it before the next flow.
- Anything that appears or goes, a signal, an arrowhead's colour or a lit
  border, fades over `motion.fade`.
- Nothing moves for decoration: no ambient background, nothing moving
  where no flow goes.
- Under `prefers-reduced-motion` the diagram is the still image, and the
  still image must carry the whole meaning: the spec chooses whether it
  lists the flows under the legend or numbers their steps on the lines
  (Components: Signal).

## Elevation & Depth

Flat. No shadows and no elevation tokens. Depth appears in one place only:
a multi-node card, whose stacked copies (offset by `card.multi-offset`)
say "several instances" rather than "raised".

## Shapes

- `rounded.card` for cards, `rounded.badge` for icon badges,
  `rounded.frame` for frames, `rounded.canvas` for the canvas. Radii grow
  with the size of the thing, so nested shapes stay concentric.
  `rounded.connector` rounds an edge's bends, never by more than half of
  the segment beside it; a step too short for two such bends is drawn as
  one S curve.
- Strokes: `stroke.card` for card edges, `stroke.icon` for icon lines,
  `stroke.connector` for edges, `stroke.frame` for frames.
- Three dash patterns, never mixed up: `dash.external` marks a node we do
  not own; `dash.frame` marks a boundary; `dash.edge` marks an edge taken
  only sometimes. The frame's dash is longer, so a dashed card inside a
  dashed frame stays distinguishable.

## Components

### Node card

The unit of every diagram: a card (`node-card` or `node-card-vertical`)
with an icon in a badge (`icon-badge`, `icon-badge-vertical`), a title and
an optional subtitle (`node-subtitle`).

The icon says what the thing is; the category says which icon role its
lines take (in `mono`, one colour for all).

| Category | Kinds | Icon role |
|---|---|---|
| Core | service, database, queue, cache, storage, users | `icon-core` |
| AI and LLM | model, vector store, tool, agent | `icon-ai` |
| Build and tooling | file, script, generated file, check | `icon-build` |
| Clients | browser, mobile, desktop | `icon-client` |

Each kind has one icon, drawn for archgram as line art on a square grid:
a code window for a service, a cylinder for a database, a segmented pill
for a queue, a bolt for a cache, a bucket for storage, a person for users,
a four-pointed star for a model, a cylinder with points for a vector store,
a wrench for a tool, a looping arrow around a point for an agent, a folded
page for a file, a terminal prompt for a script, a folded page with a star
for a generated file, a shield with a check for a check, a window with a
title bar for a browser, a phone for mobile and a monitor for desktop.

### Variants

- **Single**: the card as described.
- **Multi-node**: two copies of the card's outline behind it, each offset
  by `card.multi-offset` up and to the right. Several instances of the same
  thing: replicas, a cluster, a pool.
- **External** (`node-external`): the card filled with the canvas colour
  and edged with `dash.external`. A system we call but do not own.

### Technology logo

When a node names its technology, its logo (from Simple Icons) appears in
one of four places, chosen per diagram:

- **Corner** (`logo-corner`, the default): in the card's top-right corner.
  The card keeps room for it beside the title, so the two never meet.
- **Inline** (`logo-inline`): leading the card's second line,
  `card.logo-inline-gap` before the note; a card without a note shows the
  technology's name there instead.
- **Chip** (`logo-chip`): in a small round chip on the icon badge's lower
  right corner, edged like a card.
- **Icon** (`logo-icon`): in the badge, in place of the kind's icon, at
  the icon's size and in its category's hue, as the icon would be.

In every place the logo is in its brand's own colour (the logo's `hex` in
Simple Icons), whether or not a flow's signal lights its card, so a reader
knows the technology at a glance; the kind's icon still says which kind of
thing, except in the fourth place, where the logo says both. A brand
colour below 3:1 against the card in a theme (a black logo on a dark card)
shows as `color.text` in that theme instead, and a logo whose brand gives
no colour is in `color.text-muted`.

### Frame

A boundary around a group of nodes (`frame`): a rounded rectangle edged
with `dash.frame` in `color.frame`, no fill, its name at the top left
(`frame-label`), set in capitals, on a patch of canvas so an edge passing
under it does not cut it. Frames nest; an edge may cross a frame's
border. A frame is never shorter or narrower than its name; nodes of a
frame that share no edge line up in a grid inside it.

### Connector

An edge (`connector`): an orthogonal line in `color.connector`, its bends
rounded by `rounded.connector`, ending in an open arrowhead drawn with the
line's own stroke, `arrowhead.length` back along the line and
`arrowhead.width` across it. The tip stops `arrowhead.gap` short of the
card, so the arrowhead never touches the card's edge. The arrowhead takes
the colour of the signal that reaches it, for `motion.hop-gap` from its
arrival, or until a refusal leaves from it. Edges leaving one
side of a card leave from its middle as one trunk and fork in the gap;
edges entering one side merge into one point. An edge with a label near
the card, or one drawn against the flow, keeps a port of its own,
`spacing.edge-edge` or more from the trunk. Two edges between four
different cards never run along one line: where one card's trunk would
run on along a line another card's branch has already joined (two cards
each leading to the same two), the edge leaves its trunk for a port of
its own beside it, twice `rounded.connector` away where the side has room,
so the two lines cross once, square, on straight stretches. It never
passes through a card. Where it must change level between two layers it turns twice in the
gap between them, a symmetric step, never a slant; a step shorter than two
radii is one S curve instead, so it never kinks. An edge label, when
there is one, uses `typography.subtitle` on a straight stretch of the line,
in room kept for it: it never covers a card.

### Signal

A flow's moving marker (`signal`), in the icon hue of the node it leaves.
The spec picks one style for the whole diagram; each keeps the same
timing (Layout: Motion), and lines are drawn with the connector's own
stroke:

- **Wire** (the default): no marker; the line fills with the hue from its
  start to its end. It stays filled while the card it reached is lit.
- **Spark**: a dot of `signal.dot` with a core of `signal.core` in
  `color.signal-core`, in a halo of `signal.halo` that flickers every
  `signal.flicker`; behind it a bright stretch of `signal.bolt` at
  `signal.bolt-width` and a trail of `signal.trail` at
  `signal.trail-opacity`.
- **Arc**: a spark whose whole line glows and flickers every
  `signal.flicker-fast`.
- **Comet**: a dot and a tail of `signal.comet` that thins from
  `signal.bolt-width` and fades toward its end.
- **Dot**: a dot in a ring of `signal.ring` at `signal.ring-opacity`.
- **Pulse**: a dot with a core, sending out rings to `signal.ripple`,
  each over `signal.ripple-period`.
- **Current**: the line runs as dashes of `signal.dash`, moving on by one
  dash and gap every `signal.dash-period`, behind a dot.

The wire, spark and arc styles glow: a second line under the signal, in
light `signal.glow` wide at `signal.glow-opacity`, blurred by
`signal.blur`; in dark, where a light line on a dark canvas needs less,
`signal.glow-dark` at `signal.glow-opacity-dark`, blurred by
`signal.blur-dark`. The other styles have none, and the spec turns the
glow off with `glow: false`.

A card is lit from the moment a signal reaches its arrowhead until every
signal it sends has arrived (`lit-card`): its own border, at its own width
(`stroke.card`), turns `color.signal-pass`, drawn from the point where
the arrow meets the card, both ways round, closing on the far side. The
card takes no fill, and its icon and text keep their colours. The first
card of a flow, which no arrow reaches, draws its border from the far side
and closes it where its signal leaves, as it leaves. The spec picks how
the border is drawn:

- **Spark** (the default): a dot of `signal.core` in the pass colour rides
  each growing end.
- **Drain**: the border drains away toward the arrow that leaves the card,
  as that arrow's signal leaves.
- **Ring**: the border sweeps once round, clockwise from the arrowhead.
- **Afterglow**: once drawn, the border fades back to the card's own edge
  while the card is lit.

### Refusal

A step may refuse its flow. The refusal is marked in
`color.signal-refusal`:

- A ✕ of `refusal.mark` (`refusal-mark`) sits on the line into the
  refusing card, `refusal.mark-gap` before its arrowhead, on a patch of
  the canvas's colour so the line does not cross it; the arrowhead after
  it turns the refusal colour.
- The refusing card's border (`refused-card`) is drawn in the refusal
  colour from the arrowhead, as a lit card's is.
- The refusal then travels back along the flow to the node where the flow
  began, whose border is drawn in the refusal colour from where it
  arrives.
- The refusing card keeps its refusal border until a later flow passes
  it, which draws its border in the pass colour. On request (`wait:
  pending`) it waits as dashes of `refusal.pending` marching round it,
  one step every `motion.pending`.

An edge's label is lit while its signal is seen: the signal's line and
glow fade out round the label, softened by `signal.blur`, as the still line
stops at it, and a copy of the label's text above them takes the signal's
hue, with nothing behind it: no patch, no glow. Where the hue
falls short of text contrast on `color.canvas` in a theme (4.5:1, Colors),
the text is in `color.text` in that theme.

Where nothing moves, the still image shows what the spec chooses: nothing
more than the diagram; each flow in words under the legend, its steps'
labels joined by arrows and a branch's by commas, in the legend's type; or
each step's number on the lines it takes (`step-number`): a pill
`signal.number` high, edged like a line, just before the arrowhead where
the step arrives (behind the ✕ where a flow stops), or as near as it fits
clear of cards, labels, frames' names, a refusal's ✕ and other numbers. Lines that meet before a card share their number there. Steps
are counted on from one flow to the next. A refused step keeps its ✕ and
the refusing card its border in the refusal colour; the flow's words end
with the step that refused it.

The numbers sit above the signals, so a line passes under its pill and
the number stays readable. While the flows play, a signal that reaches a
pill lights it (`lit-step-number`): its edge, at its own width
(`stroke.card`), turns `color.signal-pass`, traced as a lit card's border
is, from where the line enters, both ways round, over `motion.hop-min`;
it holds for `motion.hop-gap`, then fades over `motion.fade`, as an
arrowhead does. A signal that comes while it is lit keeps it lit. The
still image keeps the plain pill.

### Legend

Generated from what the diagram uses: one entry per variant present
(several instances, external), in `typography.legend` and
`color.text-muted`. Colour tells no category apart, and each kind's icon
already says what it is, so categories have no entry. Each entry leads
with a swatch of `legend.swatch`, `legend.swatch-gap` before its text:
several instances a small card with two copies behind; external a small
dashed card. A diagram with no variant draws no legend.

### Credit

"by", archgram's mark, then "archgram", a space apart, small and quieter
than any text: the legend's type, `typography.legend`, in
`color.connector`, with the mark as tall as the type is large, its badge
in that colour and its lines cut out of it in `color.canvas`, so it reads
in either theme, and the whole of it shown faded, as one. It sits on a
line of its own `spacing.legend` below
everything else, against the drawing's right edge. As a logotype it keeps
no text contrast (WCAG 1.4.3 exempts logotypes). It is part of the picture, not of
its meaning, so like every text inside the drawing a screen reader skips
it: the SVG is one image, named by its title and description. The spec
turns it off (`credit: false`), and the drawing is then a line shorter.

## Do's and Don'ts

- Do let the icon say what a thing is and the title say which one.
- Do keep colour to logos and a flow's outcome; a coloured card or frame
  is a new rule, written here first. The ones written so far are a lit
  card's border and a refused card's.
- Do use the external variant for anything the system calls but does not
  own, including managed services and third-party APIs.
- Do name technologies with `tech`, so the logo appears. A logo takes the
  kind's icon's place only when the diagram asks for it (`logo: icon`).
- Don't use neon hues, gradients, shadows or glow; brand colours belong to
  the technology logos alone, and glow to a signal's line.
- Don't animate for decoration; if the flow does not need it, the diagram
  is still.
- Don't write a value in this file. A new value is a token first.
