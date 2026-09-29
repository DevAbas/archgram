# The project's colours

archgram draws in black and white by default: every icon, a flow's signal
and a lit card in the text colour, and colour only in the technology
logos. A project with a design of its own gets its colours through a
mapping file, `archgram.theme.json` at the project's root, passed to
`build` with `--theme-file archgram.theme.json`. Its format is in
`npx --yes archgram spec`, under "Theme file"; read that section
before writing one.

The mapping sits at the root because archgram reads a theme's files only
under the mapping's own folder: a mapping in `docs/diagrams/` could not
reach tokens in `design-system/tokens/`.

## Contents

- Where to look, in order
- Mapping the roles
- Colours from the styling code
- Checking the theme

## Where to look, in order

1. **Design tokens (DTCG).** A resolver (`*.resolver.json`) and token files
   (`*.tokens.json`). Point the mapping's `resolver` at it and map each
   role to a token id; the light and dark inputs are the resolver's
   modifier contexts.
2. **The styling code.** CSS custom properties (`--background`,
   `--foreground`, `--border`), a Tailwind v4 `@theme` block, a Tailwind v3
   `tailwind.config` theme, Sass variables, a theme object in TypeScript.
   Read the light and dark values, and write them as DTCG tokens (below).
3. **Colours the user gave** in the request. Write them as tokens the same
   way.
4. **None of these.** Keep archgram's own colours, pass no `--theme-file`,
   and say so in the report.

## Mapping the roles

Map by meaning, not by name. The roles, and what fills them:

| Role | Take the project's |
|---|---|
| `canvas` | page background |
| `card` | raised surface: a card, a panel |
| `badge` | subtle surface: a muted background, a hover |
| `card-edge` | border |
| `connector`, `frame` | a stronger border or muted line, readable as a line |
| `text` | main text |
| `text-muted` | secondary text |
| `signal-core` | the card surface |
| `icon-core`, `icon-ai`, `icon-build`, `icon-client` | main text, to keep the diagram monochrome as archgram draws it; a category colour only when the user asks for one |

A role the mapping leaves out keeps archgram's own colour.

## Colours from the styling code

Write the colours you read as one resolver with the tokens inline, at
`docs/diagrams/theme/archgram.resolver.json`, beside the spec, so a later
build reads the same values:

```json
{
  "version": "2025.10",
  "modifiers": {
    "theme": {
      "contexts": {
        "light": [{ "color": { "$type": "color",
          "canvas": { "$value": "#fafaf9" }, "card": { "$value": "#ffffff" },
          "badge": { "$value": "#f5f5f4" }, "border": { "$value": "#d6d3d1" },
          "line": { "$value": "#78716c" }, "text": { "$value": "#1c1917" },
          "muted": { "$value": "#57534e" } } }],
        "dark": [{ "color": { "$type": "color",
          "canvas": { "$value": "#0c0a09" }, "card": { "$value": "#1c1917" },
          "badge": { "$value": "#292524" }, "border": { "$value": "#44403c" },
          "line": { "$value": "#78716c" }, "text": { "$value": "#fafaf9" },
          "muted": { "$value": "#a8a29e" } } }]
      },
      "default": "light"
    }
  },
  "resolutionOrder": [{ "$ref": "#/modifiers/theme" }]
}
```

and the mapping at the root:

```json
{
  "version": 1,
  "resolver": "docs/diagrams/theme/archgram.resolver.json",
  "roles": {
    "canvas": "color.canvas", "card": "color.card", "badge": "color.badge",
    "card-edge": "color.border", "connector": "color.line", "frame": "color.line",
    "text": "color.text", "text-muted": "color.muted", "signal-core": "color.card",
    "icon-core": "color.text", "icon-ai": "color.text",
    "icon-build": "color.text", "icon-client": "color.text"
  },
  "themes": {
    "light": { "inputs": { "theme": "light" } },
    "dark": { "inputs": { "theme": "dark" } }
  }
}
```

A project with one theme only gives its colours to both contexts; say in
the report that dark mode repeats light.

## Checking the theme

```bash
npx --yes archgram theme check archgram.theme.json
```

It prints each role's colour in both themes, or every problem at its file
and JSON pointer. A pair below archgram's contrast (text on a card, a line
on the canvas) is refused: map a stronger token, and if the project has
none, keep archgram's colour for that role and tell the user which pair
fell short and by how much.
