---
# archgram's visual rules, in the DESIGN.md format (google-labs-code/design.md, alpha).
# The values live in the design tokens that `imports:` names (W3C Design Tokens, DTCG 2025.10);
# this file names them by their ids and holds none, as the format's planned token import
# intends (google-labs-code/design.md#13). `components:` is the contract: which roles each
# part of a diagram reads.
version: alpha
name: archgram
description: Minimalist architecture diagrams. Neutral surfaces, colour only in the icon lines, one shape per kind of thing.
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
  signal:
    size: "{signal.dot}"
---

# archgram: Design System

## Overview

A diagram is read in about thirty seconds by someone who did not build the
system. Everything here serves that reader: the eye should find what each
box is, how things connect and where the flow goes, and nothing else should
compete for attention.

The language is minimalist. Surfaces and text are neutral greys; colour
appears only in the lines of the icons, where it says which family a node
belongs to. There are no shadows, no gradients and no glow. The shapes
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

The one palette today is `mono`: neutral greys and four muted hues.

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
  `color.icon-client`): the stroke of a node's icon, by its category.
  Nothing else takes these colours except the flow signal.

Rules:

- Colour lives in icon lines only. Cards, badges, frames and connectors
  stay neutral, whatever the category.
- Hues are muted. No neon, no fully saturated brand colours, no glow.
- A flow's signal takes the icon hue of the node it leaves.
- Technology logos are drawn in `color.text-muted`, never in their brand
  colours.
- Contrast follows WCAG 2.1 AA in both themes: text 4.5:1 against what it
  sits on; icon lines, connectors and frame borders 3:1 against their
  background (1.4.11).
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
- The legend sits under the diagram, left-aligned, and lists only the
  variants and categories the diagram uses.
- `spacing.margin` surrounds everything.

Motion:

- A diagram is still by default. Motion appears only along the flows the
  spec names, to show order: where a request starts, where it branches,
  where it ends.
- A hop lasts between `motion.hop-min` and `motion.hop-max`, in proportion
  to its length, eased with `motion.ease`; consecutive hops are
  `motion.hop-gap` apart; `motion.rest` passes before the cycle repeats.
- Nothing moves for decoration: no ambient background, no pulsing.
- Under `prefers-reduced-motion` the diagram is the still image, and the
  still image must carry the whole meaning.

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

The icon says what the thing is; the category says which hue its lines
take.

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
one of two places, chosen per diagram:

- **Corner** (`logo-corner`, the default): in the card's top-right corner.
- **Chip** (`logo-chip`): in a small round chip on the icon badge's lower
  right corner, edged like a card.

The logo is always `color.text-muted`. It says which technology; the icon
still says which kind of thing.

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
card, so the arrowhead never touches the card's edge. Edges leaving one
side of a card leave from its middle as one trunk and fork in the gap;
edges entering one side merge into one point. An edge with a label near
the card, or one drawn against the flow, keeps a port of its own,
`spacing.edge-edge` or more from the trunk. It never passes through a
card. Where it must change level between two layers it turns twice in the
gap between them, a symmetric step, never a slant; a step shorter than two
radii is one S curve instead, so it never kinks. An edge label, when
there is one, uses `typography.subtitle` on a straight stretch of the line,
in room kept for it: it never covers a card.

### Signal

A flow's moving marker (`signal`): a solid dot of `signal.dot` in the icon
hue of the node it leaves, with a thin ring of `signal.ring` at
`signal.ring-opacity`. No blur and no glow. The node a signal reaches keeps
still; only the signal moves.

### Legend

Generated from what the diagram uses: one entry per variant (multi-node,
external) and per category present, in `typography.legend`.

## Do's and Don'ts

- Do let the icon say what a thing is and the title say which one.
- Do keep colour in icon lines; a coloured card or frame is a new rule,
  written here first.
- Do use the external variant for anything the system calls but does not
  own, including managed services and third-party APIs.
- Do name technologies with `tech`, so the logo appears; do not put a
  logo in place of the kind's icon.
- Don't use brand colours, neon hues, gradients, shadows or glow.
- Don't animate for decoration; if the flow does not need it, the diagram
  is still.
- Don't write a value in this file. A new value is a token first.
