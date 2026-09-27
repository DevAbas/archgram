//! Drawing the diagram as SVG (ARCHITECTURE.md, Render).

mod card;
mod edge;
mod frame;
mod icons;
mod legend;
pub mod signal;
pub mod svg;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::font;
use crate::geometry::Rect;
use crate::layout::Placement;
use crate::motion::Timeline;
use crate::spec::{SignalStyle, Spec};
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
pub fn render(
    spec: &Spec,
    placement: &Placement,
    options: Options,
    logos: &dyn crate::logos::Logos,
) -> String {
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
        escape(&description(spec))
    ));
    // Every edge as drawn, moved onto the canvas, and the flows' timing
    // along them.
    let drawn = drawn_edges(placement);
    let lengths: Vec<f64> = drawn.iter().map(edge::Drawn::length).collect();
    let timeline = crate::motion::timeline(spec, &lengths);
    let brands = brands(spec, timeline.as_ref(), logos);
    style(&mut svg, spec, options, logos, (timeline.as_ref(), &brands));
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
    // The glow's blur, over the whole canvas: measured against a line's own
    // box, a straight line's zero height would leave it no room at all.
    if timeline.is_some() && signal::glows(spec.signal) {
        svg.line(&format!(
            r#"<defs><filter id="glow" filterUnits="userSpaceOnUse" x="0" y="0" width="{}" height="{}"><feGaussianBlur stdDeviation="{}"/></filter></defs>"#,
            num(w),
            num(h),
            num(crate::tokens::SIGNAL_BLUR)
        ));
    }
    // Frames first, the outermost below the ones inside it; their names go
    // over the edges, below the cards.
    let at = |r: Rect| Rect {
        x: r.x + OFFSET,
        y: r.y + OFFSET,
        ..r
    };
    let outer_first = frames_outer_first(&mut svg, spec, placement, at);
    svg.open(r#"<g class="edges">"#);
    let followed: BTreeSet<usize> = timeline
        .as_ref()
        .map(|t| t.hops.iter().map(|h| h.edge).collect())
        .unwrap_or_default();
    for (k, ((e, d), label)) in spec.edges.iter().zip(&drawn).zip(&placement.labels).enumerate() {
        let label = label.map(|r| Rect {
            x: r.x + EDGE_OFFSET,
            y: r.y + EDGE_OFFSET,
            ..r
        });
        let id = followed.contains(&k).then(|| signal::edge_id(k));
        edge::edge(&mut svg, e, d, label, id.as_deref());
    }
    svg.close("</g>");
    for &f in &outer_first {
        if let Some(label) = placement.frame_labels[f] {
            frame::name(&mut svg, &spec.frames[f].label, at(label));
        }
    }
    if spec.still == crate::spec::Still::Numbers {
        let mut blocked: Vec<Rect> = placement.nodes.iter().map(|&r| at(r)).collect();
        blocked.extend(placement.labels.iter().flatten().map(|r| Rect {
            x: r.x + EDGE_OFFSET,
            y: r.y + EDGE_OFFSET,
            ..*r
        }));
        step_numbers(&mut svg, spec, &drawn, &blocked);
    }
    cards(&mut svg, spec, placement, logos, (timeline.as_ref(), &brands));
    if let Some(t) = &timeline {
        let hue = |n: usize| card::category_class(spec.nodes[n].kind.category());
        signal::signals(&mut svg, spec.signal, t, &drawn, hue);
    }
    let entries: Vec<crate::layout::legend::Entry> = placement
        .legend
        .iter()
        .map(|e| crate::layout::legend::Entry {
            swatch_box: at(e.swatch_box),
            text_box: at(e.text_box),
            ..e.clone()
        })
        .collect();
    let flow_lines: Vec<crate::layout::legend::FlowLine> = placement
        .flow_lines
        .iter()
        .map(|l| crate::layout::legend::FlowLine {
            text_box: at(l.text_box),
            ..l.clone()
        })
        .collect();
    legend::legend(&mut svg, &entries, &flow_lines);
    svg.close("</svg>");
    svg.finish()
}

/// Every edge as drawn, moved onto the canvas.
fn drawn_edges(placement: &Placement) -> Vec<edge::Drawn> {
    placement
        .edges
        .iter()
        .map(|path| {
            let moved: Vec<crate::geometry::Point> = path
                .iter()
                .map(|p| crate::geometry::Point {
                    x: p.x + EDGE_OFFSET,
                    y: p.y + EDGE_OFFSET,
                })
                .collect();
            edge::drawn(&moved)
        })
        .collect()
}

/// Every card, each lit while a flow's signal is at it.
fn cards(
    svg: &mut Svg,
    spec: &Spec,
    placement: &Placement,
    logos: &dyn crate::logos::Logos,
    (timeline, brands): (Option<&Timeline>, &BTreeMap<String, String>),
) {
    for (i, (node, r)) in spec.nodes.iter().zip(&placement.nodes).enumerate() {
        let at = Rect {
            x: r.x + OFFSET,
            y: r.y + OFFSET,
            ..*r
        };
        let lighting = timeline.and_then(|t| {
            let lit: Vec<crate::motion::Lit> = t.lit.iter().filter(|l| l.node == i).copied().collect();
            (!lit.is_empty()).then(|| signal::lit_animation(&lit, t.period))
        });
        let brand = lighting.as_deref().map(|animation| card::Brand {
            animation,
            class: node
                .tech
                .as_deref()
                .filter(|t| brands.contains_key(*t))
                .map(brand_class),
        });
        card::card(svg, node, at, (spec.card, spec.logo), logos, brand.as_ref());
    }
}

/// Draws the frames, the outermost below the ones inside it, and returns
/// them in that order for their names.
fn frames_outer_first(
    svg: &mut Svg,
    spec: &Spec,
    placement: &Placement,
    at: impl Fn(Rect) -> Rect,
) -> Vec<usize> {
    let depth = |f: usize| {
        let mut d = 0;
        let mut p = spec.frames[f].parent.as_deref();
        while let Some(id) = p {
            d += 1;
            p = spec
                .frames
                .iter()
                .find(|g| g.id == id)
                .and_then(|g| g.parent.as_deref());
        }
        d
    };
    let mut outer_first: Vec<usize> = (0..spec.frames.len())
        .filter(|&f| placement.frames[f].is_some())
        .collect();
    outer_first.sort_by_key(|&f| (depth(f), f));
    if !outer_first.is_empty() {
        svg.open(r#"<g class="frames">"#);
        for &f in &outer_first {
            frame::frame(svg, at(placement.frames[f].expect("placed")));
        }
        svg.close("</g>");
    }
    outer_first
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
fn style(
    svg: &mut Svg,
    spec: &Spec,
    options: Options,
    logos: &dyn crate::logos::Logos,
    (timeline, brands): (Option<&Timeline>, &BTreeMap<String, String>),
) {
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
    let animated = timeline.is_some();
    svg.open("<style>");
    theme_vars(svg, mode, (&light, &dark), animated);
    if options.embed_font && embed_fonts(svg, spec, logos) {
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
    svg.line(".logo { fill: var(--text-muted); }");
    for hue in ["core", "ai", "build", "client"] {
        svg.line(&format!(".logo-icon.{hue} {{ fill: var(--icon-{hue}); }}"));
    }
    svg.line(&format!(
        ".logo-chip {{ fill: var(--card); stroke: var(--card-edge); stroke-width: {}; }}",
        num(STROKE_CARD)
    ));
    svg.line(&format!(
        ".frame {{ fill: none; stroke: var(--frame); stroke-width: {}; stroke-dasharray: {}; }}",
        num(crate::tokens::STROKE_FRAME),
        dash(crate::tokens::DASH_FRAME)
    ));
    svg.line(&format!(
        ".frame-label {{ {} fill: var(--text-muted); }}",
        text(&crate::tokens::TYPOGRAPHY_FRAME_LABEL)
    ));
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
    if animated {
        motion_style(svg, spec.signal, brands, (&light, &dark, mode));
    }
    if spec.still == crate::spec::Still::Numbers && !spec.flows.is_empty() {
        step_style(svg);
    }
    if !placement_has_legend(spec) {
        return svg.close("</style>");
    }
    svg.line(&format!(
        ".swatch {{ fill: none; stroke-width: {}; }}",
        num(crate::tokens::STROKE_ICON)
    ));
    for hue in ["core", "ai", "build", "client"] {
        svg.line(&format!(".swatch.{hue} {{ stroke: var(--icon-{hue}); }}"));
    }
    svg.line(&format!(
        ".legend-text {{ {} fill: var(--text-muted); }}",
        text(&crate::tokens::TYPOGRAPHY_LEGEND)
    ));
    svg.close("</style>");
}

/// A step's number on a line: a badge edged like a line, its number like text.
fn step_style(svg: &mut Svg) {
    svg.line(&format!(
        ".step {{ fill: var(--card); stroke: var(--connector); stroke-width: {}; }}",
        num(STROKE_CARD)
    ));
    svg.line(&format!(
        ".step-text {{ {} fill: var(--text); }}",
        text(&crate::tokens::TYPOGRAPHY_LEGEND)
    ));
}

/// The theme's roles for `mode`: light, dark, or light switching to dark
/// under `prefers-color-scheme: dark`.
fn theme_vars(svg: &mut Svg, mode: Mode, (light, dark): (&Colors, &Colors), animated: bool) {
    match mode {
        Mode::Auto => {
            svg.line(&format!(":root {{ {} }}", vars(light, animated)));
            svg.line(&format!(
                "@media (prefers-color-scheme: dark) {{ :root {{ {} }} }}",
                vars(dark, animated)
            ));
        }
        Mode::Light => svg.line(&format!(":root {{ {} }}", vars(light, animated))),
        Mode::Dark => svg.line(&format!(":root {{ {} }}", vars(dark, animated))),
    }
}

/// Whether the drawing has a legend, so its classes are needed.
fn placement_has_legend(spec: &Spec) -> bool {
    !crate::layout::legend::entries(spec).is_empty() || !crate::layout::legend::flow_texts(spec).is_empty()
}

/// The theme's roles as custom properties; the signal's own only where
/// something moves, so a still diagram's style is what it always was.
fn vars(c: &Colors, animated: bool) -> String {
    c.roles()
        .iter()
        .filter(|(name, _)| animated || !name.starts_with("signal-"))
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
fn embed_fonts(svg: &mut Svg, spec: &Spec, logos: &dyn crate::logos::Logos) -> bool {
    let mut by_weight: std::collections::BTreeMap<u16, BTreeSet<char>> = std::collections::BTreeMap::new();
    for (weight, text) in crate::measure::text_runs_with(spec, logos) {
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

/// The spec's description, then each flow in words for a screen reader,
/// which sees no motion.
fn description(spec: &Spec) -> String {
    let mut out = spec.description.trim().to_owned();
    for flow in &spec.flows {
        let _ = write!(out, " {}: {}.", flow.name.trim(), spec.flow_words(flow));
    }
    out
}

/// A line's step numbers as its badge shows them.
#[must_use]
pub fn step_label(numbers: &[u32]) -> String {
    numbers.iter().map(u32::to_string).collect::<Vec<_>>().join(",")
}

/// Each step's number on the lines the flows take, for the still image
/// (`still: numbers`): a badge where the step arrives, just before the
/// arrowhead, or else as near to it along the line as clears every card,
/// label and badge in `blocked`. Lines that meet before a card share their
/// last stretch, and show their number there once.
fn step_numbers(svg: &mut Svg, spec: &Spec, drawn: &[edge::Drawn], blocked: &[Rect]) {
    use crate::tokens::{SIGNAL_NUMBER, TYPOGRAPHY_LEGEND};
    let numbers = crate::motion::step_numbers(spec);
    if numbers.iter().all(Vec::is_empty) {
        return;
    }
    let mut placed: Vec<(Rect, String)> = Vec::new();
    svg.open(r#"<g class="steps">"#);
    for (e, list) in numbers.iter().enumerate().filter(|(_, n)| !n.is_empty()) {
        let text = step_label(list);
        let h = SIGNAL_NUMBER;
        // As wide as the text and the room either side a single digit has.
        let w = h.max((font::text_width(&text, &TYPOGRAPHY_LEGEND) + h - TYPOGRAPHY_LEGEND.size).ceil());
        let badge = |c: crate::geometry::Point| Rect {
            x: c.x - w / 2.0,
            y: c.y - h / 2.0,
            w,
            h,
        };
        let spots = badge_spots(&drawn[e], w);
        if spots.first().is_some_and(|&c| {
            placed
                .iter()
                .any(|(r, t)| *t == text && overlaps(badge(c), *r, 0.0))
        }) {
            continue;
        }
        let clear = |c: crate::geometry::Point| {
            let b = badge(c);
            !blocked
                .iter()
                .chain(placed.iter().map(|(r, _)| r))
                .any(|&r| overlaps(b, r, 2.0))
        };
        let Some(&centre) = spots.iter().find(|&&c| clear(c)).or(spots.first()) else {
            continue;
        };
        let b = badge(centre);
        // Its outline on half pixels, so the one-pixel edge is sharp.
        svg.line(&format!(
            r#"<rect class="step" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
            num(b.x + HALF_PIXEL),
            num(b.y + HALF_PIXEL),
            num(b.w - 1.0),
            num(b.h - 1.0),
            num((h - 1.0) / 2.0)
        ));
        let line = TYPOGRAPHY_LEGEND.size * TYPOGRAPHY_LEGEND.line_height;
        svg.line(&format!(
            r#"<text class="step-text" x="{}" y="{}" text-anchor="middle">{}</text>"#,
            num(centre.x),
            num(centre.y - line / 2.0 + font::baseline_in_line(&TYPOGRAPHY_LEGEND)),
            escape(&text)
        ));
        placed.push((b, text));
    }
    svg.close("</g>");
}

/// Where a badge `width` wide may sit on a line, best first: on each
/// straight stretch from the last back to the first, near its end (past the
/// arrowhead on the last one), then at its middle, `arrowhead.gap` clear of
/// the bends; last, should no stretch have that room, the middle of the
/// longest, so a number is never lost.
fn badge_spots(drawn: &edge::Drawn, width: f64) -> Vec<crate::geometry::Point> {
    use crate::tokens::{ARROWHEAD_GAP, ARROWHEAD_LENGTH};
    let straights = drawn.straights();
    let span =
        |a: crate::geometry::Point, b: crate::geometry::Point| (b.x - a.x).abs().max((b.y - a.y).abs());
    let mut spots = Vec::new();
    for (k, &(a, b)) in straights.iter().enumerate().rev() {
        let length = span(a, b);
        let tip = if k + 1 == straights.len() {
            ARROWHEAD_LENGTH
        } else {
            0.0
        };
        if length < tip + width + 2.0 * ARROWHEAD_GAP {
            continue;
        }
        let at = |from_end: f64| crate::geometry::Point {
            x: b.x + (a.x - b.x) / length * from_end,
            y: b.y + (a.y - b.y) / length * from_end,
        };
        spots.push(at(tip + ARROWHEAD_GAP + width / 2.0));
        spots.push(at(f64::midpoint(tip, length)));
    }
    let longest = straights.iter().copied().reduce(|best, s| {
        if span(s.0, s.1) > span(best.0, best.1) {
            s
        } else {
            best
        }
    });
    if let Some((a, b)) = longest {
        spots.push(crate::geometry::Point {
            x: f64::midpoint(a.x, b.x),
            y: f64::midpoint(a.y, b.y),
        });
    }
    spots
}

/// Whether two boxes come within `margin` of each other.
fn overlaps(a: Rect, b: Rect, margin: f64) -> bool {
    a.x < b.right() + margin
        && b.x < a.right() + margin
        && a.y < b.bottom() + margin
        && b.y < a.bottom() + margin
}

/// The brand colours the lit cards show: each technology's slug with its
/// colour, for the technologies of cards a signal reaches whose logo set
/// gives a colour.
fn brands(
    spec: &Spec,
    timeline: Option<&Timeline>,
    logos: &dyn crate::logos::Logos,
) -> BTreeMap<String, String> {
    let Some(t) = timeline else {
        return BTreeMap::new();
    };
    let reached: BTreeSet<usize> = t.lit.iter().map(|l| l.node).collect();
    reached
        .iter()
        .filter_map(|&n| spec.nodes[n].tech.as_deref())
        .filter(|slug| logos.path(slug).is_some())
        .filter_map(|slug| {
            let hex = logos.colour(slug)?;
            crate::color::channels(hex)
                .map(|_| (slug.to_owned(), hex.trim_start_matches('#').to_ascii_lowercase()))
        })
        .collect()
}

/// The class that gives a technology's logo its brand's colour: the slug,
/// kept to the characters a class name takes.
fn brand_class(slug: &str) -> String {
    let safe: String = slug
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    format!("brand-{safe}")
}

/// The brand's colour on a card, or the text colour when the brand's would
/// not show against the card: a line at least 3:1 (DESIGN.md, Colors).
fn brand_fill(hex: &str, card: &str) -> String {
    let colour = format!("#{hex}");
    match crate::color::contrast(&colour, card) {
        Some(ratio) if ratio >= 3.0 => colour,
        _ => "var(--text)".into(),
    }
}

/// The signal, the lit card and the brand colours; none of it under
/// `prefers-reduced-motion`, where the diagram is still.
fn motion_style(
    svg: &mut Svg,
    style: SignalStyle,
    brands: &BTreeMap<String, String>,
    (light, dark, mode): (&Colors, &Colors, Mode),
) {
    use crate::tokens::{
        SIGNAL_BOLT_WIDTH, SIGNAL_DASH, SIGNAL_GLOW, SIGNAL_GLOW_OPACITY, SIGNAL_LIT, SIGNAL_RING_OPACITY,
        SIGNAL_TINT, SIGNAL_TRAIL_OPACITY, STROKE_ICON,
    };
    for hue in ["core", "ai", "build", "client"] {
        svg.line(&format!(".signal.{hue} {{ color: var(--icon-{hue}); }}"));
    }
    svg.line(".signal path { fill: none; stroke: currentColor; stroke-linecap: round; }");
    let rules: Vec<String> = match style {
        SignalStyle::Wire => vec![
            format!(
                ".signal .fill {{ stroke-width: {}; stroke-linecap: butt; }}",
                num(STROKE_CONNECTOR)
            ),
            glowing(SIGNAL_GLOW, SIGNAL_GLOW_OPACITY),
        ],
        SignalStyle::Spark => vec![
            dots(),
            halo(),
            format!(
                ".signal .trail {{ stroke-width: {}; opacity: {}; }}",
                num(STROKE_CONNECTOR),
                num(SIGNAL_TRAIL_OPACITY)
            ),
            glowing(SIGNAL_GLOW, SIGNAL_GLOW_OPACITY),
            format!(".signal .bolt {{ stroke-width: {}; }}", num(SIGNAL_BOLT_WIDTH)),
        ],
        SignalStyle::Arc => vec![
            dots(),
            halo(),
            format!(
                ".signal .wire-glow {{ stroke-width: {}; filter: url(#glow); }}",
                num(SIGNAL_GLOW)
            ),
            format!(".signal .bolt {{ stroke-width: {}; }}", num(SIGNAL_BOLT_WIDTH)),
        ],
        SignalStyle::Comet => vec![dots()],
        SignalStyle::Dot => vec![
            dots(),
            format!(
                ".signal .ring {{ fill: currentColor; opacity: {}; }}",
                num(SIGNAL_RING_OPACITY)
            ),
        ],
        SignalStyle::Pulse => vec![
            dots(),
            format!(
                ".signal .ripple {{ fill: none; stroke: currentColor; stroke-width: {}; }}",
                num(STROKE_ICON)
            ),
        ],
        SignalStyle::Current => vec![
            dots(),
            format!(
                ".signal .flowing {{ stroke-width: {}; stroke-linecap: butt; stroke-dasharray: {}; }}",
                num(STROKE_CONNECTOR),
                dash(SIGNAL_DASH)
            ),
        ],
    };
    for rule in rules {
        svg.line(&rule);
    }
    for hue in ["core", "ai", "build", "client"] {
        svg.line(&format!(
            ".lit.{hue} {{ fill: var(--icon-{hue}); fill-opacity: {}; stroke: var(--icon-{hue}); stroke-width: {}; }}",
            num(SIGNAL_TINT),
            num(SIGNAL_LIT)
        ));
    }
    let fills = |c: &Colors| -> Vec<String> {
        brands
            .iter()
            .map(|(slug, hex)| format!(".{} {{ fill: {}; }}", brand_class(slug), brand_fill(hex, c.card)))
            .collect()
    };
    match mode {
        Mode::Auto => {
            let (l, d) = (fills(light), fills(dark));
            for rule in &l {
                svg.line(rule);
            }
            let changed: Vec<&String> = d.iter().filter(|r| !l.contains(r)).collect();
            if !changed.is_empty() {
                svg.line(&format!(
                    "@media (prefers-color-scheme: dark) {{ {} }}",
                    changed.iter().map(|r| r.as_str()).collect::<Vec<_>>().join(" ")
                ));
            }
        }
        Mode::Light => fills(light).iter().for_each(|r| svg.line(r)),
        Mode::Dark => fills(dark).iter().for_each(|r| svg.line(r)),
    }
    svg.line("@media (prefers-reduced-motion: reduce) { .signals, .lit, .brand { display: none; } }");
}

/// A signal's dot, and the bright core some styles light it with.
fn dots() -> String {
    ".signal .dot { fill: currentColor; } .signal .core { fill: var(--signal-core); }".into()
}

fn halo() -> String {
    ".signal .halo { fill: currentColor; filter: url(#glow); }".into()
}

/// A line's glow: wide, faint and blurred. Written after the lines it
/// widens, so it wins over them.
fn glowing(width: f64, opacity: f64) -> String {
    format!(
        ".signal .glowing {{ stroke-width: {}; opacity: {}; filter: url(#glow); }}",
        num(width),
        num(opacity)
    )
}
