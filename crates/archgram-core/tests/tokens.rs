//! The compiled tokens keep DESIGN.md's colour rules (Colors: contrast
//! follows WCAG 2.1 AA in both themes).

use archgram_core::color::check;
use archgram_core::tokens::{PALETTES, THEMES};

#[test]
fn every_palette_has_a_light_and_a_dark_theme() {
    for p in PALETTES {
        for t in ["light", "dark"] {
            assert!(archgram_core::tokens::theme(p, t).is_some(), "{p} {t}");
        }
    }
}

#[test]
fn text_reads_at_4_5_to_1_and_lines_at_3_to_1() {
    for t in THEMES {
        let shortfalls = check(&t.colors);
        assert!(
            shortfalls.is_empty(),
            "{} {}: {}",
            t.palette,
            t.theme,
            shortfalls
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ")
        );
    }
}
