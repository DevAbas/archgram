//! The compiled tokens keep DESIGN.md's colour rules (Colors: contrast
//! follows WCAG 2.1 AA in both themes).

use archgram_core::tokens::{PALETTES, THEMES};

/// Relative luminance of an sRGB hex colour (WCAG 2.1).
fn luminance(hex: &str) -> f64 {
    let channel = |i: usize| {
        let c = f64::from(u8::from_str_radix(&hex[i..i + 2], 16).unwrap()) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5)
}

fn contrast(a: &str, b: &str) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

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
        let c = t.colors;
        let pairs = [
            ("text on card", c.text, c.card, 4.5),
            ("text muted on card", c.text_muted, c.card, 4.5),
            ("text on canvas", c.text, c.canvas, 4.5),
            ("text muted on canvas", c.text_muted, c.canvas, 4.5),
            ("icon core on badge", c.icon_core, c.badge, 3.0),
            ("icon ai on badge", c.icon_ai, c.badge, 3.0),
            ("icon build on badge", c.icon_build, c.badge, 3.0),
            ("icon client on badge", c.icon_client, c.badge, 3.0),
            ("icon core on canvas", c.icon_core, c.canvas, 3.0),
            ("icon ai on canvas", c.icon_ai, c.canvas, 3.0),
            ("icon build on canvas", c.icon_build, c.canvas, 3.0),
            ("icon client on canvas", c.icon_client, c.canvas, 3.0),
            ("connector on canvas", c.connector, c.canvas, 3.0),
            ("frame on canvas", c.frame, c.canvas, 3.0),
        ];
        for (what, fg, bg, min) in pairs {
            let ratio = contrast(fg, bg);
            assert!(
                ratio >= min,
                "{} {}: {what} is {ratio:.2}:1, below {min}:1",
                t.palette,
                t.theme
            );
        }
    }
}
