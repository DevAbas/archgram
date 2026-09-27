//! The embedded font: Geist, measured with `skrifa` and embedded as a subset
//! (ARCHITECTURE.md, Measure and Render). See `fonts/README.md` for where the
//! files come from and their licence.

mod base64;
pub mod subset;

pub use base64::encode as base64;

use skrifa::prelude::{FontRef, LocationRef, MetadataProvider, Size};

use crate::tokens::TextStyle;

/// Geist Regular, weight 400.
pub static REGULAR: &[u8] = include_bytes!("../../fonts/Geist-Regular.ttf");
/// Geist Medium, weight 500.
pub static MEDIUM: &[u8] = include_bytes!("../../fonts/Geist-Medium.ttf");

/// The family name the embedded subsets take in CSS: our own, so a copy of
/// Geist installed on the reader's machine never stands in for the subset.
pub const FAMILY: &str = "archgram-geist";

/// The file for a CSS weight: Medium from 500 up, Regular below.
#[must_use]
pub fn face(weight: u16) -> &'static [u8] {
    if weight >= 500 { MEDIUM } else { REGULAR }
}

fn font_ref(bytes: &'static [u8]) -> FontRef<'static> {
    FontRef::new(bytes).expect("the embedded font parses; tests check it")
}

/// The width of `text` set in `style`, in pixels: the sum of the glyphs'
/// advances plus the style's letter spacing after each character, as CSS
/// adds it. No kerning: the embedded subset carries none, so the drawing
/// matches this measure exactly.
#[must_use]
pub fn text_width(text: &str, style: &TextStyle) -> f64 {
    let font = font_ref(face(style.weight));
    #[allow(clippy::cast_possible_truncation)] // pixel sizes fit an f32
    let metrics = font.glyph_metrics(Size::new(style.size as f32), LocationRef::default());
    let charmap = font.charmap();
    text.chars()
        .map(|c| {
            let advance = match charmap.map(c) {
                Some(gid) if gid.to_u32() != 0 => f64::from(metrics.advance_width(gid).unwrap_or(0.0)),
                // A character the font lacks is drawn by the reader's font, whose
                // glyph we cannot measure: a full em is as wide as a Han or emoji
                // glyph and wider than most others, so the card never ends up
                // narrower than its text.
                _ => style.size,
            };
            advance + style.letter_spacing
        })
        .sum()
}

/// The characters of `text` that `weight`'s face does not have, in order,
/// without repeats. The renderer leaves them to the reader's font.
#[must_use]
pub fn uncovered(text: &str, weight: u16) -> Vec<char> {
    let font = font_ref(face(weight));
    let charmap = font.charmap();
    let mut seen = std::collections::BTreeSet::new();
    text.chars()
        .filter(|c| !c.is_whitespace() && charmap.map(*c).is_none_or(|g| g.to_u32() == 0))
        .filter(|c| seen.insert(*c))
        .collect()
}

/// Where the baseline sits below the top of a line box of `style`: half the
/// leading, then the font's ascent, the way CSS places a line of text.
#[must_use]
pub fn baseline_in_line(style: &TextStyle) -> f64 {
    let font = font_ref(face(style.weight));
    #[allow(clippy::cast_possible_truncation)]
    let m = font.metrics(Size::new(style.size as f32), LocationRef::default());
    let (ascent, descent) = (f64::from(m.ascent), f64::from(m.descent).abs());
    let line = style.size * style.line_height;
    (line - (ascent + descent)) / 2.0 + ascent
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::{TYPOGRAPHY_SUBTITLE, TYPOGRAPHY_TITLE};

    #[test]
    fn both_faces_parse_and_map_latin_text() {
        for bytes in [REGULAR, MEDIUM] {
            let font = font_ref(bytes);
            for c in "Postgres API, queue: 3 replicas".chars() {
                assert!(font.charmap().map(c).is_some(), "{c:?}");
            }
        }
    }

    #[test]
    fn widths_add_up_and_grow_with_the_text() {
        let one = text_width("API", &TYPOGRAPHY_TITLE);
        let two = text_width("APIAPI", &TYPOGRAPHY_TITLE);
        assert!(one > 10.0 && one < 40.0, "{one}");
        assert!((two - 2.0 * one).abs() < 1e-6, "{one} {two}");
        assert!(
            text_width("API", &TYPOGRAPHY_SUBTITLE) < one,
            "smaller and lighter is narrower"
        );
    }

    #[test]
    fn characters_the_font_lacks_count_a_full_em() {
        let han = text_width("用户", &TYPOGRAPHY_TITLE);
        assert!((han - 2.0 * TYPOGRAPHY_TITLE.size).abs() < 1e-6, "{han}");
        assert_eq!(
            uncovered("API 用户 用 é", TYPOGRAPHY_TITLE.weight),
            vec!['用', '户']
        );
        assert!(uncovered("Şəkillər və sənədlər", TYPOGRAPHY_TITLE.weight).is_empty());
    }

    #[test]
    fn the_baseline_sits_inside_the_line() {
        for style in [TYPOGRAPHY_TITLE, TYPOGRAPHY_SUBTITLE] {
            let b = baseline_in_line(&style);
            assert!(b > style.size * 0.6 && b < style.size * style.line_height, "{b}");
        }
    }
}
