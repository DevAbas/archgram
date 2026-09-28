//! The embedded fonts are Geist's release files cut down by `cargo xtask
//! fonts` (fonts/README.md): every character keeps its advance, its outline
//! and the line's metrics, and the files are exactly what the task writes.

use archgram_core::font::{MEDIUM, REGULAR, characters, subset::subset};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::{FontRef, MetadataProvider};

fn source(name: &str) -> Vec<u8> {
    let path = format!("{}/fonts/source/{name}.ttf", env!("CARGO_MANIFEST_DIR"));
    std::fs::read(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A glyph's outline as its drawing commands, in font units.
#[derive(Default)]
struct Commands(Vec<String>);

impl OutlinePen for Commands {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(format!("M{x} {y}"));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(format!("L{x} {y}"));
    }
    fn quad_to(&mut self, cx0: f32, cy0: f32, x: f32, y: f32) {
        self.0.push(format!("Q{cx0} {cy0} {x} {y}"));
    }
    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        self.0.push(format!("C{cx0} {cy0} {cx1} {cy1} {x} {y}"));
    }
    fn close(&mut self) {
        self.0.push("Z".into());
    }
}

fn outline(font: &FontRef, c: char) -> Vec<String> {
    let gid = font.charmap().map(c).expect("mapped");
    let glyph = font.outline_glyphs().get(gid).expect("an outline");
    let mut pen = Commands::default();
    glyph
        .draw(
            DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
            &mut pen,
        )
        .expect("drawn");
    pen.0
}

#[test]
fn every_character_keeps_its_advance_outline_and_line() {
    for (name, embedded) in [("Geist-Regular", REGULAR), ("Geist-Medium", MEDIUM)] {
        let original = source(name);
        let (was, is) = (FontRef::new(&original).unwrap(), FontRef::new(embedded).unwrap());
        let chars = characters(&original).unwrap();
        assert_eq!(
            characters(embedded).unwrap(),
            chars,
            "{name}: the same characters"
        );
        let was_m = was.glyph_metrics(Size::unscaled(), LocationRef::default());
        let is_m = is.glyph_metrics(Size::unscaled(), LocationRef::default());
        for &c in &chars {
            let (a, b) = (was.charmap().map(c).unwrap(), is.charmap().map(c).unwrap());
            assert_eq!(
                was_m.advance_width(a),
                is_m.advance_width(b),
                "{name} {c:?}: advance"
            );
            assert_eq!(outline(&was, c), outline(&is, c), "{name} {c:?}: outline");
        }
        let line = |f: &FontRef| {
            let m = f.metrics(Size::unscaled(), LocationRef::default());
            (m.units_per_em, m.ascent, m.descent, m.leading)
        };
        assert_eq!(line(&was), line(&is), "{name}: line metrics");
    }
}

#[test]
fn the_fonts_are_what_xtask_fonts_writes() {
    for (name, embedded) in [("Geist-Regular", REGULAR), ("Geist-Medium", MEDIUM)] {
        let original = source(name);
        let written = subset(&original, &characters(&original).unwrap()).unwrap();
        assert!(written == embedded, "{name} is stale; run `cargo xtask fonts`");
    }
}
