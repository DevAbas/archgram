# Changelog

Each release's changes, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions
follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- A spec named `<name>.archgram.yaml` (or `.yml`, `.json`) draws
  `<name>.svg` beside it, so a project keeps each spec next to its
  drawing under a plain name.
- `-o` into a folder that does not exist creates the folder.
- An edge's label lights with its signal: while the signal is seen, the
  label's text takes the signal's colour, and the signal and its glow fade
  out round it, so nothing crosses or boxes the text. A colour too faint
  for text on the canvas shows as the text colour instead.
- A small, quiet "by [mark] archgram" in the drawing's bottom-right
  corner, with archgram's mark between the words; `credit: false` turns
  it off.
- `archgram spec` prints the spec format this archgram reads, so whoever
  writes a spec reads the format of the very command that draws it.
- The `archgram` skill for Claude Code, in `skills/archgram`: it draws a
  project's architecture from its code and docs, in the project's own
  colours, and lists each part with the file behind it.

### Changed

- A diagram is monochrome: icons, signals and lit cards are in the text
  colour, black on light and white on dark; the technology logos keep
  their brands' colours. The legend lists only the variants, since colour
  no longer tells a category apart.
- A technology logo is always in its brand's own colour, not only while a
  flow's signal lights its card. A brand colour that would not show on the
  card in a theme is the text colour there.
- A technology logo in a card's corner is larger, 18px instead of 14, so
  it reads at a glance.

## [0.3.0] - 2026-09-28

### Added

- Flows animate along the lines. A step may branch: several nodes reached
  at once. The timing follows each line's length; branches leave together
  and paths that meet arrive together.
- Seven signal styles (`signal`): the line filling (the default), spark,
  arc, comet, dot, pulse and current.
- A card is lit while a signal is at it: its border and a tint in its
  hue, and its technology logo in its brand's colour.
- What the still image shows of the flows (`still`): nothing more, the
  flows in words under the legend, or each step's number on its lines.
  Under `prefers-reduced-motion` nothing moves. A screen reader hears each
  flow in words.
- A project's own design tokens (W3C Design Tokens 2025.10) as the theme,
  through a mapping file: `--theme-file` and `archgram theme check`. The
  colours must keep the same contrast as archgram's own.
- The command on npm for Node: `npm install archgram` installs the native
  binary for macOS, Linux or Windows, on x64 or arm64.

### Changed

- The embedded fonts carry only the tables a renderer needs: about 49 KB
  each instead of 126 KB, with every character unchanged.
- A crash says it is a bug in archgram and where to report it.

### Fixed

- `check` refuses a layout hint the edges contradict, as `build` does. A
  `sameLayer` group whose nodes a path of edges joins is refused, where
  the layout used to crash.
- A text holding a character XML does not allow is refused: the SVG would
  not open.

### Security

- The drawing is written to a new file and renamed into place, so a
  symlink at the output is replaced, never written through.
- A theme reads only regular files under its mapping file's folder, by
  their real paths.
- A problem prints a spec's control characters as escapes, so a spec
  cannot drive the terminal or a CI log.

## [0.2.0] - 2026-09-27

### Added

- Frames, nested, around groups of nodes.
- Variants: several instances, and external systems.
- A legend of the categories and variants a diagram uses.
- Technology logos from Simple Icons, in four places: the card's corner,
  before its note, a chip on its badge, or in place of its icon.
- YAML specs, with every problem at its line and column.
- `--split-themes`: one file per theme from one layout.

## [0.1.0] - 2026-09-27

### Added

- The JSON spec, validated with every problem at its JSON pointer.
- Every node kind with its own icon; horizontal and vertical cards.
- A layered layout and orthogonal edge routing, with rounded bends and
  open arrowheads.
- One SVG with light and dark, text measured and drawn in an embedded
  subset of Geist.
- `archgram build` and `archgram check`.

[Unreleased]: https://github.com/DevAbas/archgram/compare/v0.3.0...HEAD
[0.3.0]: https://github.com/DevAbas/archgram/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/DevAbas/archgram/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/DevAbas/archgram/releases/tag/v0.1.0
