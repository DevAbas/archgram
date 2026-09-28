# Fonts

`source/Geist-Regular.ttf` and `source/Geist-Medium.ttf` are the static
TrueType files of Geist v1.7.2, from the release archive at
https://github.com/vercel/geist-font/releases/tag/v1.7.2
(`geist-font-v1.7.2.zip`, SHA-256
`7fc800d2ac6b92844895196e5041aca55d814c15db70c44f79b3b83ab82b04e2`), path
`geist-font/Geist/ttf/`. They are licensed under the SIL Open Font License
1.1, in `OFL.txt`; the licence declares no Reserved Font Name.

archgram measures text with these files and embeds a subset of them in each
SVG. The subset keeps the font's copyright and licence entries in its `name`
table, as the licence asks.

The files archgram embeds, `Geist-Regular.ttf` and `Geist-Medium.ttf` here,
are written from those by `cargo xtask fonts`: every character each maps,
with only the tables a renderer needs, so kerning, ligatures and the glyphs
only they reach, glyph names and hinting are left out. Each is about 49 KB
instead of 126 KB. `tests/fonts.rs` checks that every character keeps its
advance, its outline and the line's metrics, and that the files are what
the task writes. They are a Modified Version under the licence, which they
carry in their `name` table.
