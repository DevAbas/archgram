//! Drawing the diagram as SVG (ARCHITECTURE.md, Render).

mod border;
mod card;
mod credit;
mod edge;
mod frame;
mod icons;
mod legend;
mod refusal;
pub mod scene;
pub mod signal;
pub mod styles;
pub mod svg;

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::color::Rgb;
use crate::font;
use crate::geometry::Rect;
use crate::layout::Placement;
use crate::motion::Timeline;
use crate::spec::{SignalStyle, Spec};
use crate::tokens::{
    ARROWHEAD_LENGTH, ARROWHEAD_WIDTH, Colors, FONT_SANS, ROUNDED_CANVAS, SPACING_MARGIN,
    STROKE_CONNECTOR, theme,
};
use scene::{Anchor, GroupOf, Item, Scene, StyleLine};
use svg::num;

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
    /// A project's own colours (`theme::import`), in place of the spec's
    /// palette.
    pub colors: Option<crate::theme::ThemeColors>,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            mode: Mode::Auto,
            embed_font: true,
            colors: None,
        }
    }
}

/// Draws a laid-out diagram as SVG.
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
    svg::write(&scene(spec, placement, options, logos))
}

/// A laid-out diagram as a scene: every shape in drawing order, with its
/// style sheet.
///
/// # Panics
///
/// As [`render`].
#[must_use]
pub fn scene(
    spec: &Spec,
    placement: &Placement,
    options: Options,
    logos: &dyn crate::logos::Logos,
) -> Scene {
    let w = placement.size.w + 2.0 * SPACING_MARGIN;
    let h = placement.size.h + 2.0 * SPACING_MARGIN;
    // Every edge as drawn, moved onto the canvas, and the flows' timing
    // along them.
    let drawn = drawn_edges(placement);
    let lengths: Vec<f64> = drawn.iter().map(edge::Drawn::length).collect();
    let timeline = crate::motion::timeline(spec, &lengths);
    let brands = brands(spec, logos);
    let style = style(spec, options, logos, (timeline.as_ref(), &brands));
    let mut items = vec![
        Item::Canvas {
            width: w,
            height: h,
            rx: ROUNDED_CANVAS,
        },
        // An open chevron in the connector colour and the line's own stroke
        // (DESIGN.md, Components: Connector), its tip at the line's end.
        Item::Arrowhead {
            length: ARROWHEAD_LENGTH,
            width: ARROWHEAD_WIDTH,
        },
    ];
    if timeline.is_some() && spec.glow && signal::glows(spec.signal) {
        items.push(glow_filters(options.mode, (w, h)));
    }
    // Frames first, the outermost below the ones inside it; their names go
    // over the edges, below the cards.
    let at = |r: Rect| Rect {
        x: r.x + OFFSET,
        y: r.y + OFFSET,
        ..r
    };
    let outer_first = frames_outer_first(&mut items, spec, placement, at);
    let label_boxes = label_boxes(placement);
    edges_drawn(
        &mut items,
        spec,
        (&drawn, &label_boxes),
        timeline.as_ref(),
        (w, h),
    );
    for &f in &outer_first {
        if let Some(label) = placement.frame_labels[f] {
            items.extend(frame::name(&spec.frames[f].label, at(label)));
        }
    }
    // The step numbers go above the signals, so a line passes under them.
    let clear_of = || numbers_clear_of(placement, &label_boxes);
    let (steps, badges) = (spec.still == crate::spec::Still::Numbers)
        .then(|| step_numbers(spec, &drawn, &clear_of(), timeline.as_ref()))
        .flatten()
        .unzip();
    cards(&mut items, spec, placement, logos, &brands);
    if let Some(t) = &timeline {
        // Each node's front card on the canvas, whose border a flow draws.
        let fronts: Vec<Rect> = spec
            .nodes
            .iter()
            .zip(&placement.nodes)
            .map(|(node, &r)| card::front(node, at(r)))
            .collect();
        items.push(border::borders(
            t,
            &fronts,
            &drawn,
            (spec.border, spec.wait),
        ));
        let hue = |n: usize| card::category_class(spec.nodes[n].kind.category());
        let labelled: Vec<bool> = spec
            .edges
            .iter()
            .zip(&label_boxes)
            .map(|(e, at)| e.label.is_some() && at.is_some())
            .collect();
        let labels: Vec<Option<(&str, Rect)>> = spec
            .edges
            .iter()
            .zip(&label_boxes)
            .map(|(e, at)| e.label.as_deref().zip(*at))
            .collect();
        items.push(signal::signals(
            (spec.signal, spec.glow),
            t,
            &drawn,
            &labels,
            hue,
            refusal::refusals(t, &drawn, &labelled),
        ));
        items.extend(steps);
        items.extend(badges.map(|b| step_lights(t, &drawn, &b)));
        items.extend(refusal::still(t, &drawn, &fronts));
    } else {
        items.extend(steps);
    }
    items.extend(placed_legend(placement, at));
    if let Some(c) = placement.credit {
        items.push(credit::credit(at(c)));
    }
    Scene {
        width: w,
        height: h,
        title: spec.title.clone(),
        description: description(spec),
        style,
        items,
    }
}

/// The glow's blur, over the whole canvas `size` large: measured against a
/// line's own box, a straight line's zero height would leave it no room at
/// all. The halo has its own, around the head it rides with: the head is
/// moved along its edge, so the canvas measured from the head would start
/// at its centre and cut its glow to one quarter. One of each for each
/// theme the drawing carries, the dark one softer.
fn glow_filters(mode: Mode, (w, h): (f64, f64)) -> Item {
    use crate::tokens::{SIGNAL_BLUR, SIGNAL_BLUR_DARK, SIGNAL_HALO};
    let filter = |id: &str, blur: f64, (x, y, w, h): (f64, f64, f64, f64)| {
        format!(
            r#"<filter id="{id}" filterUnits="userSpaceOnUse" x="{}" y="{}" width="{}" height="{}"><feGaussianBlur stdDeviation="{}"/></filter>"#,
            num(x),
            num(y),
            num(w),
            num(h),
            num(blur)
        )
    };
    // A blur reaches three times its deviation.
    let halo = |id: &str, blur: f64| {
        let r = SIGNAL_HALO + 3.0 * blur;
        filter(id, blur, (-r, -r, 2.0 * r, 2.0 * r))
    };
    let (light, dark) = (
        filter("glow", SIGNAL_BLUR, (0.0, 0.0, w, h)) + &halo("halo-glow", SIGNAL_BLUR),
        filter("glow-dark", SIGNAL_BLUR_DARK, (0.0, 0.0, w, h))
            + &halo("halo-glow-dark", SIGNAL_BLUR_DARK),
    );
    let filters = match mode {
        Mode::Light => light,
        Mode::Dark => dark,
        Mode::Auto => format!("{light}{dark}"),
    };
    Item::Motion(format!("<defs>{filters}</defs>"))
}

/// The legend and the flows in words, moved onto the canvas by `at`.
fn placed_legend(placement: &Placement, at: impl Fn(Rect) -> Rect) -> Option<Item> {
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
    legend::legend(&entries, &flow_lines)
}

/// Each edge's label box, moved onto the canvas as its edge is.
fn label_boxes(placement: &Placement) -> Vec<Option<Rect>> {
    placement
        .labels
        .iter()
        .map(|label| {
            label.map(|r| Rect {
                x: r.x + EDGE_OFFSET,
                y: r.y + EDGE_OFFSET,
                ..r
            })
        })
        .collect()
}

/// The edges with their labels, each a flow takes named for its signal to
/// follow; then, when signals pass labels, the mask that fades them out
/// round the text (`signal::label_gap`) in a drawing `size` large.
fn edges_drawn(
    items: &mut Vec<Item>,
    spec: &Spec,
    (drawn, label_boxes): (&[edge::Drawn], &[Option<Rect>]),
    timeline: Option<&crate::motion::Timeline>,
    (w, h): (f64, f64),
) {
    let followed: BTreeSet<usize> = timeline
        .map(|t| t.hops.iter().map(|h| h.edge).collect())
        .unwrap_or_default();
    let mut edges = Vec::new();
    for (k, ((e, d), label)) in spec.edges.iter().zip(drawn).zip(label_boxes).enumerate() {
        let id = followed.contains(&k).then(|| signal::edge_id(k));
        edges.extend(edge::edge(e, d, *label, id.as_deref()));
    }
    items.push(Item::Group {
        of: GroupOf::Class("edges"),
        items: edges,
    });
    let passed: Vec<Rect> = spec
        .edges
        .iter()
        .zip(label_boxes)
        .enumerate()
        .filter(|(k, (e, _))| followed.contains(k) && e.label.is_some())
        .filter_map(|(_, (_, at))| *at)
        .collect();
    if let Some(gap) = signal::label_gap(&passed, w, h) {
        items.push(Item::Motion(gap));
    }
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

/// Every card, its logo in its brand's colour.
fn cards(
    items: &mut Vec<Item>,
    spec: &Spec,
    placement: &Placement,
    logos: &dyn crate::logos::Logos,
    brands: &BTreeMap<String, Rgb>,
) {
    for (node, r) in spec.nodes.iter().zip(&placement.nodes) {
        let at = Rect {
            x: r.x + OFFSET,
            y: r.y + OFFSET,
            ..*r
        };
        let brand = card::Brand {
            class: node
                .tech
                .as_deref()
                .filter(|t| brands.contains_key(*t))
                .map(brand_class),
        };
        items.push(card::card(
            node,
            at,
            (spec.card, spec.logo),
            logos,
            Some(&brand),
        ));
    }
}

/// Draws the frames, the outermost below the ones inside it, and returns
/// them in that order for their names.
fn frames_outer_first(
    items: &mut Vec<Item>,
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
        items.push(Item::Group {
            of: GroupOf::Class("frames"),
            items: outer_first
                .iter()
                .map(|&f| frame::frame(at(placement.frames[f].expect("placed"))))
                .collect(),
        });
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

/// The style sheet as it is written.
#[derive(Default)]
struct Sheet(Vec<StyleLine>);

impl Sheet {
    fn line(&mut self, text: &str) {
        self.0.push(StyleLine::Raw(text.to_owned()));
    }

    fn rules(&mut self, rules: Vec<scene::Rule>) {
        self.0.extend(rules.into_iter().map(StyleLine::Rule));
    }
}

/// The style sheet: the theme's roles as custom properties, the fonts, then
/// the classes that read them (`styles`).
fn style(
    spec: &Spec,
    options: Options,
    logos: &dyn crate::logos::Logos,
    (timeline, brands): (Option<&Timeline>, &BTreeMap<String, Rgb>),
) -> Vec<StyleLine> {
    let (palette, mode) = (spec.palette.as_str(), options.mode);
    // Validation admits only known palettes; should one slip through, the
    // first palette stands in, in the same theme.
    let pick = |t: &str| {
        theme(palette, t)
            .or_else(|| theme(crate::tokens::PALETTES[0], t))
            .expect("every palette has a light and a dark theme (tests/tokens.rs)")
            .colors
    };
    let (light, dark) = match options.colors {
        Some(c) => (c.light, c.dark),
        None => (pick("light"), pick("dark")),
    };
    let animated = timeline.is_some();
    let mut sheet = Sheet::default();
    theme_vars(&mut sheet, mode, (&light, &dark), animated);
    if options.embed_font && embed_fonts(&mut sheet, spec, logos) {
        // No kerning: the subset carries none and the text was measured without it.
        sheet.line(&format!(
            "text {{ font-family: \"{}\", {FONT_SANS}; font-kerning: none; }}",
            font::FAMILY
        ));
    } else {
        sheet.line(&format!("text {{ font-family: {FONT_SANS}; }}"));
    }
    sheet.rules(styles::shapes());
    brand_rules(&mut sheet, brands, (&light, &dark, mode));
    if animated {
        motion_style(&mut sheet, (spec.signal, spec.glow), (&light, &dark, mode));
    }
    if spec.still == crate::spec::Still::Numbers && !spec.flows.is_empty() {
        sheet.rules(styles::steps());
    }
    if spec.credit {
        sheet.rules(styles::credit());
    }
    if placement_has_legend(spec) {
        sheet.rules(styles::legend());
    }
    sheet.0
}

/// The theme's roles for `mode`: light, dark, or light switching to dark
/// under `prefers-color-scheme: dark`.
fn theme_vars(svg: &mut Sheet, mode: Mode, (light, dark): (&Colors, &Colors), animated: bool) {
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
    !crate::layout::legend::entries(spec).is_empty()
        || !crate::layout::legend::flow_texts(spec).is_empty()
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

fn dash(d: &[f64]) -> String {
    d.iter().map(|v| num(*v)).collect::<Vec<_>>().join(" ")
}

/// One `@font-face` per weight the diagram uses, each a subset holding only
/// the characters set in that weight. False when a subset cannot be made, in
/// which case the text falls back to the system font.
fn embed_fonts(svg: &mut Sheet, spec: &Spec, logos: &dyn crate::logos::Logos) -> bool {
    let mut by_weight: std::collections::BTreeMap<u16, BTreeSet<char>> =
        std::collections::BTreeMap::new();
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
    numbers
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",")
}

/// Each step's number on the lines the flows take, for the still image
/// (`still: numbers`): a badge where the step arrives, just before the
/// arrowhead (and behind the ✕ on a line a flow stops on), or else as near
/// to it along the line as clears every card, edge label, frame name and
/// badge in `blocked`, and every ✕ in `timeline`. Lines that meet before a
/// card share their last stretch, and show all their numbers there in one
/// badge. With the badges, each as drawn with the edges whose lines it
/// sits on.
fn step_numbers(
    spec: &Spec,
    drawn: &[edge::Drawn],
    blocked: &[Rect],
    timeline: Option<&crate::motion::Timeline>,
) -> Option<(Item, Vec<Badge>)> {
    use crate::tokens::{SIGNAL_NUMBER, TYPOGRAPHY_LEGEND};
    let numbers = crate::motion::step_numbers(spec);
    if numbers.iter().all(Vec::is_empty) {
        return None;
    }
    // The lines flows stop on, whose ✕ the numbers keep clear of.
    let refused: Vec<usize> = timeline
        .iter()
        .flat_map(|t| &t.refusals)
        .map(|r| r.edge)
        .collect();
    let mut blocked = blocked.to_vec();
    blocked.extend(refused.iter().map(|&e| refusal::mark_box(&drawn[e])));
    // Lines ending at one point share their last stretch: one badge on the
    // first of them carries every number, in order.
    let mut groups: Vec<(usize, Vec<u32>, Vec<usize>)> = Vec::new();
    for (e, list) in numbers.iter().enumerate().filter(|(_, n)| !n.is_empty()) {
        let tip = drawn[e].tip();
        let meets = |g: usize| {
            let t = drawn[g].tip();
            (t.x - tip.x).abs() < 0.5 && (t.y - tip.y).abs() < 0.5
        };
        match groups.iter_mut().find(|(g, _, _)| meets(*g)) {
            Some((_, all, edges)) => {
                all.extend(list);
                all.sort_unstable();
                all.dedup();
                edges.push(e);
            }
            None => groups.push((e, list.clone(), vec![e])),
        }
    }
    let mut placed: Vec<(Rect, String)> = Vec::new();
    let mut badges: Vec<Badge> = Vec::new();
    let mut items = Vec::new();
    for (e, list, edges) in &groups {
        let (e, text) = (*e, step_label(list));
        let h = SIGNAL_NUMBER;
        // As wide as the text and the room either side a single digit has.
        let w = h
            .max((font::text_width(&text, &TYPOGRAPHY_LEGEND) + h - TYPOGRAPHY_LEGEND.size).ceil());
        let badge = |c: crate::geometry::Point| Rect {
            x: c.x - w / 2.0,
            y: c.y - h / 2.0,
            w,
            h,
        };
        let stops = edges.iter().any(|g| refused.contains(g));
        let spots = badge_spots(&drawn[e], w, stops);
        // The same numbers already there: that badge is this line's too.
        if let Some(i) = spots.first().and_then(|&c| {
            placed
                .iter()
                .position(|(r, t)| *t == text && overlaps(badge(c), *r, 0.0))
        }) {
            badges[i].edges.extend(edges);
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
        let outline = Rect {
            x: b.x + HALF_PIXEL,
            y: b.y + HALF_PIXEL,
            w: b.w - 1.0,
            h: b.h - 1.0,
        };
        items.push(Item::Rect {
            class: "step".into(),
            x: outline.x,
            y: outline.y,
            w: outline.w,
            h: outline.h,
            rx: Some((h - 1.0) / 2.0),
        });
        badges.push(Badge {
            outline,
            centre,
            edges: edges.clone(),
        });
        let line = TYPOGRAPHY_LEGEND.size * TYPOGRAPHY_LEGEND.line_height;
        items.push(Item::Text {
            class: "step-text".into(),
            x: centre.x,
            y: centre.y - line / 2.0 + font::baseline_in_line(&TYPOGRAPHY_LEGEND),
            anchor: Anchor::Middle,
            text: text.clone(),
        });
        placed.push((b, text));
    }
    Some((
        Item::Group {
            of: GroupOf::Class("steps"),
            items,
        },
        badges,
    ))
}

/// A step number's badge as drawn: its outline, its centre on the line,
/// and the edges whose lines it sits on.
struct Badge {
    outline: Rect,
    centre: crate::geometry::Point,
    edges: Vec<usize>,
}

/// What a step's number keeps clear of: every card, edge label and
/// frame's name, on the canvas.
fn numbers_clear_of(placement: &Placement, label_boxes: &[Option<Rect>]) -> Vec<Rect> {
    let at = |r: Rect| Rect {
        x: r.x + OFFSET,
        y: r.y + OFFSET,
        ..r
    };
    let mut blocked: Vec<Rect> = placement.nodes.iter().map(|&r| at(r)).collect();
    blocked.extend(label_boxes.iter().flatten());
    blocked.extend(placement.frame_labels.iter().flatten().map(|&r| at(r)));
    blocked
}

/// The badges lit as the signals pass them (`border::step_lights`): each
/// badge's outline, where its line enters it, and each moment a signal
/// reaches it there, from the hops along its edges, each where its own
/// line passes the badge.
fn step_lights(
    timeline: &crate::motion::Timeline,
    drawn: &[edge::Drawn],
    badges: &[Badge],
) -> Item {
    let mut out = Vec::new();
    for b in badges {
        let mut entry = None;
        let mut reached = Vec::new();
        for &e in &b.edges {
            let Some((centre, u)) = drawn[e].along(b.centre) else {
                continue;
            };
            // From the centre back to the badge's edge, along the line.
            let half = u.x.abs() * b.outline.w / 2.0 + u.y.abs() * b.outline.h / 2.0;
            entry.get_or_insert(crate::geometry::Point {
                x: b.centre.x - u.x * half,
                y: b.centre.y - u.y * half,
            });
            let fraction = (centre - half).max(0.0) / drawn[e].length().max(1.0);
            reached.extend(
                timeline
                    .hops
                    .iter()
                    .filter(|h| h.edge == e)
                    .map(|h| crate::motion::reached(h, fraction)),
            );
        }
        if let Some(entry) = entry.filter(|_| !reached.is_empty()) {
            out.push((b.outline, entry, reached));
        }
    }
    border::step_lights(&out, timeline.period)
}

/// Where a badge `width` wide may sit on a line, best first: on each
/// straight stretch from the last back to the first, near its end (past the
/// arrowhead on the last one, and past the ✕ too on a line a flow `stops`
/// on), then at its middle, `arrowhead.gap` clear of the bends; last,
/// should no stretch have that room, the middle of the longest, so a number
/// is never lost.
fn badge_spots(drawn: &edge::Drawn, width: f64, stops: bool) -> Vec<crate::geometry::Point> {
    use crate::tokens::{ARROWHEAD_GAP, ARROWHEAD_LENGTH};
    let straights = drawn.straights();
    let span = |a: crate::geometry::Point, b: crate::geometry::Point| {
        (b.x - a.x).abs().max((b.y - a.y).abs())
    };
    let mut spots = Vec::new();
    for (k, &(a, b)) in straights.iter().enumerate().rev() {
        let length = span(a, b);
        let tip = match (k + 1 == straights.len(), stops) {
            (true, true) => refusal::mark_reach(),
            (true, false) => ARROWHEAD_LENGTH,
            (false, _) => 0.0,
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

/// The brand colours the logos show: each technology's slug with its
/// colour, for every card's technology whose logo set gives a colour.
fn brands(spec: &Spec, logos: &dyn crate::logos::Logos) -> BTreeMap<String, Rgb> {
    spec.nodes
        .iter()
        .filter_map(|n| n.tech.as_deref())
        .filter(|slug| logos.path(slug).is_some())
        .filter_map(|slug| {
            let hex = logos.colour(slug)?;
            Rgb::parse(hex).map(|rgb| (slug.to_owned(), rgb))
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
fn brand_fill(brand: Rgb, card: Rgb) -> String {
    if crate::color::contrast(brand, card) >= 3.0 {
        brand.to_string()
    } else {
        "var(--text)".into()
    }
}

/// The signal, the lit border, the refusal and the lit label; under
/// `prefers-reduced-motion` only what stays of a refusal, where the diagram
/// is still.
fn motion_style(
    svg: &mut Sheet,
    (style, glow): (SignalStyle, bool),
    (light, dark, mode): (&Colors, &Colors, Mode),
) {
    use crate::tokens::{
        SIGNAL_BOLT_WIDTH, SIGNAL_DASH, SIGNAL_RING_OPACITY, SIGNAL_TRAIL_OPACITY, STROKE_ICON,
    };
    for hue in ["core", "ai", "build", "client"] {
        svg.line(&format!(".signal.{hue} {{ color: var(--icon-{hue}); }}"));
    }
    svg.line(".signal path { fill: none; stroke: currentColor; stroke-linecap: round; }");
    let rules: Vec<String> = match style {
        SignalStyle::Wire => vec![format!(
            ".signal .fill {{ stroke-width: {}; stroke-linecap: butt; }}",
            num(STROKE_CONNECTOR)
        )],
        SignalStyle::Spark => vec![
            dots(),
            format!(
                ".signal .trail {{ stroke-width: {}; opacity: {}; }}",
                num(STROKE_CONNECTOR),
                num(SIGNAL_TRAIL_OPACITY)
            ),
            format!(
                ".signal .bolt {{ stroke-width: {}; }}",
                num(SIGNAL_BOLT_WIDTH)
            ),
        ],
        SignalStyle::Arc => vec![
            dots(),
            format!(
                ".signal .bolt {{ stroke-width: {}; }}",
                num(SIGNAL_BOLT_WIDTH)
            ),
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
    svg.line(&format!(
        ".signal .chevron {{ stroke-width: {}; stroke-linejoin: round; }}",
        num(STROKE_CONNECTOR)
    ));
    border_style(svg);
    if glow {
        glow_style(svg, style, mode);
    }
    lit_text_rules(svg, (light, dark, mode));
    svg.line(".still-refusal { display: none; }");
    svg.line("@media (prefers-reduced-motion: reduce) { .signals, .borders { display: none; } .still-refusal { display: inline; } }");
}

/// A lit card's border and a refusal's marks.
fn border_style(svg: &mut Sheet) {
    use crate::tokens::{REFUSAL_PENDING, STROKE_CARD};
    // A lit card's border, on the card's own edge at its own width.
    svg.line(&format!(
        ".border {{ fill: none; stroke-width: {}; stroke-linejoin: round; }} .border.pass {{ stroke: var(--signal-pass); }} .border.refused {{ stroke: var(--signal-refusal); }}",
        num(STROKE_CARD)
    ));
    svg.line(".border-head.pass { fill: var(--signal-pass); } .border-head.refused { fill: var(--signal-refusal); }");
    svg.line(&format!(
        ".border.pending {{ stroke-dasharray: {}; }}",
        dash(REFUSAL_PENDING)
    ));
    // A refusal: the ✕ on a patch of canvas, its arrowhead, its way back.
    svg.line(&format!(
        ".refusal path {{ fill: none; stroke: var(--signal-refusal); stroke-width: {}; stroke-linecap: round; stroke-linejoin: round; }} .refusal .back {{ stroke-linecap: butt; }}",
        num(STROKE_CONNECTOR)
    ));
    // The patch covers the line a stroke's width either side of the ✕.
    svg.line(&format!(
        ".refusal .patch {{ stroke: var(--canvas); stroke-width: {}; }}",
        num(3.0 * STROKE_CONNECTOR)
    ));
}

/// A signal's glow, in the styles that have one: in light `signal.glow` at
/// `signal.glow-opacity`, blurred by `signal.blur`; in dark, where a light
/// line needs less, their `-dark` tokens (DESIGN.md, Components: Signal).
fn glow_style(svg: &mut Sheet, style: SignalStyle, mode: Mode) {
    use crate::tokens::{
        SIGNAL_GLOW, SIGNAL_GLOW_DARK, SIGNAL_GLOW_OPACITY, SIGNAL_GLOW_OPACITY_DARK,
    };
    // Written after the lines they widen, so they win over them.
    let rules = |width: f64, opacity: f64, filter: &str| -> String {
        let mut out = Vec::new();
        match style {
            SignalStyle::Wire | SignalStyle::Spark => out.push(format!(
                ".signal .glowing {{ stroke-width: {}; opacity: {}; filter: url(#{filter}); }}",
                num(width),
                num(opacity)
            )),
            SignalStyle::Arc => out.push(format!(
                ".signal .wire-glow {{ stroke-width: {}; filter: url(#{filter}); }}",
                num(width)
            )),
            _ => {}
        }
        if matches!(style, SignalStyle::Spark | SignalStyle::Arc) {
            out.push(format!(
                ".signal .halo {{ fill: currentColor; filter: url(#halo-{filter}); }}"
            ));
        }
        out.join(" ")
    };
    let light = rules(SIGNAL_GLOW, SIGNAL_GLOW_OPACITY, "glow");
    let dark = rules(SIGNAL_GLOW_DARK, SIGNAL_GLOW_OPACITY_DARK, "glow-dark");
    if light.is_empty() {
        return;
    }
    match mode {
        Mode::Light => svg.line(&light),
        Mode::Dark => svg.line(&dark),
        Mode::Auto => {
            svg.line(&light);
            svg.line(&format!("@media (prefers-color-scheme: dark) {{ {dark} }}"));
        }
    }
}

/// An edge's label under its signal takes the signal's colour, or the text
/// colour in a theme where the signal's hue would fall short of text
/// contrast on the canvas: 4.5:1 (DESIGN.md, Colors).
fn lit_text_rules(svg: &mut Sheet, (light, dark, mode): (&Colors, &Colors, Mode)) {
    svg.line(".signal .lit-text { fill: currentColor; }");
    let short = |c: &Colors| -> Vec<String> {
        [
            ("core", c.icon_core),
            ("ai", c.icon_ai),
            ("build", c.icon_build),
            ("client", c.icon_client),
        ]
        .iter()
        .filter(|(_, hue)| crate::color::contrast(*hue, c.canvas) < 4.5)
        .map(|(name, _)| format!(".signal.{name} .lit-text {{ fill: var(--text); }}"))
        .collect()
    };
    match mode {
        Mode::Auto => {
            let (l, d) = (short(light), short(dark));
            for rule in &l {
                svg.line(rule);
            }
            // The dark theme's rules replace the light theme's: a hue short
            // in light but not in dark goes back to the signal's colour.
            if l != d {
                let back: Vec<String> = l
                    .iter()
                    .filter(|r| !d.contains(r))
                    .map(|r| r.replace("var(--text)", "currentColor"))
                    .collect();
                let rules: Vec<&String> = back.iter().chain(&d).collect();
                svg.line(&format!(
                    "@media (prefers-color-scheme: dark) {{ {} }}",
                    rules
                        .iter()
                        .map(|r| r.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }
        }
        Mode::Light => short(light).iter().for_each(|r| svg.line(r)),
        Mode::Dark => short(dark).iter().for_each(|r| svg.line(r)),
    }
}

/// Each brand colour's rule, for the theme or themes the drawing carries.
fn brand_rules(
    svg: &mut Sheet,
    brands: &BTreeMap<String, Rgb>,
    (light, dark, mode): (&Colors, &Colors, Mode),
) {
    let fills = |c: &Colors| -> Vec<String> {
        brands
            .iter()
            .map(|(slug, hex)| {
                // As strong as a logo's own rule, `.logo-icon.core` in the
                // badge too, and after it, so the brand's colour stands.
                format!(
                    ".logo.{class}, .logo-icon.{class} {{ fill: {fill}; }}",
                    class = brand_class(slug),
                    fill = brand_fill(*hex, c.card)
                )
            })
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
                    changed
                        .iter()
                        .map(|r| r.as_str())
                        .collect::<Vec<_>>()
                        .join(" ")
                ));
            }
        }
        Mode::Light => fills(light).iter().for_each(|r| svg.line(r)),
        Mode::Dark => fills(dark).iter().for_each(|r| svg.line(r)),
    }
}

/// A signal's dot, and the bright core some styles light it with.
fn dots() -> String {
    ".signal .dot { fill: currentColor; } .signal .core { fill: var(--signal-core); }".into()
}
