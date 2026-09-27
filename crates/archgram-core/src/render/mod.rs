//! Drawing the diagram as SVG (ARCHITECTURE.md, Render).

mod card;
mod icons;
pub mod svg;

use crate::geometry::Rect;
use crate::layout::Placement;
use crate::spec::Spec;
use crate::tokens::{
    Colors, FONT_SANS, ROUNDED_CANVAS, SPACING_MARGIN, STROKE_CARD, THEMES, TYPOGRAPHY_SUBTITLE,
    TYPOGRAPHY_TITLE, TextStyle, theme,
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
#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    pub mode: Mode,
}

/// Draws a laid-out diagram. `spec` must have passed validation.
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
    style(&mut svg, &spec.palette, options.mode);
    svg.line(&format!(
        r#"<rect class="canvas" width="{}" height="{}" rx="{}"/>"#,
        num(w),
        num(h),
        num(ROUNDED_CANVAS)
    ));
    for (node, r) in spec.nodes.iter().zip(&placement.nodes) {
        let at = Rect {
            x: r.x + SPACING_MARGIN,
            y: r.y + SPACING_MARGIN,
            ..*r
        };
        card::card(&mut svg, node, at, spec.card);
    }
    svg.close("</svg>");
    svg.finish()
}

/// The style sheet: the theme's roles as custom properties, then the classes
/// that read them (DESIGN.md, front matter: components).
fn style(svg: &mut Svg, palette: &str, mode: Mode) {
    let light = theme(palette, "light").unwrap_or(&THEMES[0]).colors;
    let dark = theme(palette, "dark").unwrap_or(&THEMES[0]).colors;
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
    svg.line(&format!("text {{ font-family: {FONT_SANS}; }}"));
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
