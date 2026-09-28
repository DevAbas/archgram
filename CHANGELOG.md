# Changelog

Each release's changes, newest first. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions
follow [Semantic Versioning](https://semver.org/).

## [0.3.0] - unreleased

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

### Fixed

- `check` refuses a layout hint the edges contradict, as `build` does. A
  `sameLayer` group whose nodes a path of edges joins is refused, where
  the layout used to crash.
- A text holding a character XML does not allow is refused: the SVG would
  not open.

### Security

- The drawing is written to a new file and renamed into place, so a
  symlink at the output is replaced, never written through.
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

[0.3.0]: https://github.com/DevAbas/archgram/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/DevAbas/archgram/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/DevAbas/archgram/releases/tag/v0.1.0
