//! Drawing the diagram as SVG (ARCHITECTURE.md, Render).

mod card;
mod edge;
mod icons;
pub mod svg;

use std::collections::BTreeSet;

use crate::font;
use crate::geometry::Rect;
use crate::layout::Placement;
use crate::spec::Spec;
use crate::tokens::{
    ARROWHEAD_LENGTH, ARROWHEAD_WIDTH, Colors, FONT_SANS, ROUNDED_CANVAS, SPACING_MARGIN, STROKE_CARD,
    STROKE_CONNECTOR, TYPOGRAPHY_SUBTITLE, TYPOGRAPHY_TITLE, TextStyle, theme,
};
use svg::{Svg, escape, num};

/// Which theme the SVG carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Light, switching to dark under `prefers-color-scheme: dark`.
    #[default]
    Auto,
    Light,
    Dark,
}

/// Choices that belong to one rendering rather than to the spec.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub mode: Mode,
    /// Embed a subset of the font the text was measured with (the default),
    /// or leave the text to the reader's system font.
    pub embed_font: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Mode::Auto,
            embed_font: true,
        }
    }
}

/// Draws a laid-out diagram.
///
/// # Panics
///
/// When `spec` has not passed validation (an edge names a node that does
/// not exist) or `placement` was made for another spec.
#[must_use]
pub fn render(spec: &Spec, placement: &Placement, options: Options) -> String {
    let w = placement.size.w + 2.0 * SPACING_MARGIN;
    let h = placement.size.h + 2.0 * SPACING_MARGIN;
    let mut svg = Svg::default();
    svg.open(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-labelledby="title desc">"#,
        w = num(w),
        h = num(h)
    ));
    svg.line(&format!(r#"<title id="title">{}</title>"#, escape(&spec.title)));
    svg.line(&format!(
        r#"<desc id="desc">{}</desc>"#,
        escape(spec.description.trim())
    ));
    style(&mut svg, spec, options);
    svg.line(&format!(
        r#"<rect class="canvas" width="{}" height="{}" rx="{}"/>"#,
        num(w),
        num(h),
        num(ROUNDED_CANVAS)
    ));
    // Arrowhead: an open chevron in the connector colour and the line's own
    // stroke (DESIGN.md, Components: Connector), in user space so its size is
    // the tokens' and not scaled by the stroke. Its tip is the line's end;
    // `overflow` keeps the stroke's round ends from being clipped.
    svg.line(&format!(
        r#"<defs><marker id="arrow" viewBox="0 0 {l} {w}" refX="{l}" refY="{ry}" markerWidth="{l}" markerHeight="{w}" markerUnits="userSpaceOnUse" orient="auto-start-reverse" overflow="visible"><path class="arrowhead" d="M0 0L{l} {ry}L0 {w}"/></marker></defs>"#,
        l = num(ARROWHEAD_LENGTH),
        w = num(ARROWHEAD_WIDTH),
        ry = num(ARROWHEAD_WIDTH / 2.0)
    ));
    svg.open(r#"<g class="edges">"#);
    for ((e, path), label) in spec.edges.iter().zip(&placement.edges).zip(&placement.labels) {
        let path: Vec<crate::geometry::Point> = path
            .iter()
            .map(|p| crate::geometry::Point {
                x: p.x + EDGE_OFFSET,
                y: p.y + EDGE_OFFSET,
            })
            .collect();
        let label = label.map(|r| Rect {
            x: r.x + EDGE_OFFSET,
            y: r.y + EDGE_OFFSET,
            ..r
        });
        edge::edge(&mut svg, e, &path, label);
    }
    svg.close("</g>");
    for (node, r) in spec.nodes.iter().zip(&placement.nodes) {
        let at = Rect {
            x: r.x + OFFSET,
            y: r.y + OFFSET,
            ..*r
        };
        card::card(&mut svg, node, at, spec.card);
    }
    svg.close("</svg>");
    svg.finish()
}

/// Half a pixel. The layout puts cards and paths on whole pixels; a card's
/// border one pixel wide centred on a whole pixel would cover two half
/// pixels and blur into a pale band, so the drawing moves by half a pixel to
/// cover one. A line wider than a pixel then covers one whole pixel at its
/// middle.
const HALF_PIXEL: f64 = 0.5;

/// Where the layout's origin lands on the canvas: past the margin, and half a
/// pixel in.
const OFFSET: f64 = SPACING_MARGIN + HALF_PIXEL;

/// Where the layout's origin lands for edges: past the margin, and shifted
/// so the line's stroke covers whole pixels: by half a pixel for a stroke
/// of an odd number of pixels, by none for an even one.
const EDGE_OFFSET: f64 = SPACING_MARGIN + (STROKE_CONNECTOR / 2.0) % 1.0;

/// The style sheet: the theme's roles as custom properties, then the classes
/// that read them (DESIGN.md, front matter: components).
fn style(svg: &mut Svg, spec: &Spec, options: Options) {
    let (palette, mode) = (spec.palette.as_str(), options.mode);
    // Validation admits only known palettes; should one slip through, the
    // first palette stands in, in the same theme.
    let pick = |t: &str| {
        theme(palette, t)
            .or_else(|| theme(crate::tokens::PALETTES[0], t))
            .expect("every palette has a light and a dark theme (tests/tokens.rs)")
            .colors
    };
    let (light, dark) = (pick("light"), pick("dark"));
    svg.open("<style>");
    match mode {
        Mode::Auto => {
            svg.line(&format!(":root {{ {} }}", vars(&light)));
            svg.line(&format!(
                "@media (prefers-color-scheme: dark) {{ :root {{ {} }} }}",
                vars(&dark)
            ));
        }
        Mode::Light => svg.line(&format!(":root {{ {} }}", vars(&light))),
        Mode::Dark => svg.line(&format!(":root {{ {} }}", vars(&dark))),
    }
    if options.embed_font && embed_fonts(svg, spec) {
        // No kerning: the subset carries none and the text was measured without it.
        svg.line(&format!(
            "text {{ font-family: \"{}\", {FONT_SANS}; font-kerning: none; }}",
            font::FAMILY
        ));
    } else {
        svg.line(&format!("text {{ font-family: {FONT_SANS}; }}"));
    }

    svg.line(&format!(
        ".title {{ {} fill: var(--text); }}",
        text(&TYPOGRAPHY_TITLE)
    ));
    svg.line(&format!(
        ".sub {{ {} fill: var(--text-muted); }}",
        text(&TYPOGRAPHY_SUBTITLE)
    ));
    svg.line(".canvas { fill: var(--canvas); }");
    svg.line(&format!(
        ".card {{ fill: var(--card); stroke: var(--card-edge); stroke-width: {}; }}",
        num(STROKE_CARD)
    ));
    svg.line(&format!(
        ".card.external {{ fill: var(--canvas); stroke: var(--connector); stroke-dasharray: {}; }}",
        dash(crate::tokens::DASH_EXTERNAL)
    ));
    svg.line(".badge { fill: var(--badge); }");
    svg.line(".label-patch { fill: var(--canvas); }");
    svg.line(&format!(
        ".edge {{ fill: none; stroke: var(--connector); stroke-width: {}; stroke-linecap: round; stroke-linejoin: round; }}",
        num(crate::tokens::STROKE_CONNECTOR)
    ));
    svg.line(&format!(
        ".edge.dashed {{ stroke-dasharray: {}; }}",
        dash(crate::tokens::DASH_EDGE)
    ));
    svg.line(&format!(
        ".arrowhead {{ fill: none; stroke: var(--connector); stroke-width: {}; stroke-linecap: round; stroke-linejoin: round; }}",
        num(crate::tokens::STROKE_CONNECTOR)
    ));
    svg.line(".icon { fill: none; stroke-linecap: round; stroke-linejoin: round; }");
    for hue in ["core", "ai", "build", "client"] {
        svg.line(&format!(".icon.{hue} {{ stroke: var(--icon-{hue}); }}"));
    }
    svg.close("</style>");
}

fn vars(c: &Colors) -> String {
    c.roles()
        .iter()
        .map(|(name, value)| format!("--{name}: {value};"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn text(t: &TextStyle) -> String {
    let spacing = if t.letter_spacing == 0.0 {
        String::new()
    } else {
        format!(" letter-spacing: {}px;", num(t.letter_spacing))
    };
    format!(
        "font-size: {}px; font-weight: {};{spacing}",
        num(t.size),
        t.weight
    )
}

fn dash(d: &[f64]) -> String {
    d.iter().map(|v| num(*v)).collect::<Vec<_>>().join(" ")
}

/// One `@font-face` per weight the diagram uses, each a subset holding only
/// the characters set in that weight. False when a subset cannot be made, in
/// which case the text falls back to the system font.
fn embed_fonts(svg: &mut Svg, spec: &Spec) -> bool {
    let mut by_weight: std::collections::BTreeMap<u16, BTreeSet<char>> = std::collections::BTreeMap::new();
    for (weight, text) in crate::measure::text_runs(spec) {
        by_weight.entry(weight).or_default().extend(text.chars());
    }
    let mut faces = Vec::new();
    for (weight, chars) in &by_weight {
        match font::subset::subset(font::face(*weight), chars) {
            Ok(bytes) => faces.push(format!(
                "@font-face {{ font-family: \"{}\"; font-weight: {weight}; src: url(data:font/ttf;base64,{}) format(\"truetype\"); }}",
                font::FAMILY,
                font::base64(&bytes)
            )),
            Err(_) => return false,
        }
    }
    for face in faces {
        svg.line(&face);
    }
    true
}
