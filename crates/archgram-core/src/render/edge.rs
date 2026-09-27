//! Edges (DESIGN.md, Components: Connector): the router's orthogonal line,
//! an open chevron at its end, and its label, if any, on its longest
//! straight segment.

use crate::font::text_width;
use crate::geometry::Point;
use crate::render::svg::{Svg, escape, num};
use crate::spec::{Edge, EdgeStyle};
use crate::tokens::{CARD_PADDING, TYPOGRAPHY_SUBTITLE};

/// Draws one edge along `path`, which already starts and ends on the sides
/// of its cards.
pub fn edge(svg: &mut Svg, e: &Edge, path: &[Point]) {
    if path.len() < 2 {
        return;
    }
    let d = path
        .iter()
        .enumerate()
        .map(|(i, p)| format!("{}{} {}", if i == 0 { "M" } else { "L" }, num(p.x), num(p.y)))
        .collect::<Vec<_>>()
        .join(" ");
    let class = match e.style {
        EdgeStyle::Solid => "edge",
        EdgeStyle::Dashed => "edge dashed",
    };
    svg.line(&format!(
        r#"<path class="{class}" d="{d}" marker-end="url(#arrow)"/>"#
    ));
    if let Some(label) = &e.label {
        edge_label(svg, label, path);
    }
}

/// The label centred on the path's longest segment, over a patch of canvas
/// so the line does not run through the text.
fn edge_label(svg: &mut Svg, label: &str, path: &[Point]) {
    let (a, b) = path
        .windows(2)
        .map(|w| (w[0], w[1]))
        .max_by(|(a, b), (c, d)| length(*a, *b).total_cmp(&length(*c, *d)))
        .expect("a path of two or more points");
    let centre = Point {
        x: f64::midpoint(a.x, b.x),
        y: f64::midpoint(a.y, b.y),
    };
    let width = text_width(label, &TYPOGRAPHY_SUBTITLE);
    let height = TYPOGRAPHY_SUBTITLE.size * TYPOGRAPHY_SUBTITLE.line_height;
    let pad = CARD_PADDING / 2.0;
    svg.line(&format!(
        r#"<rect class="label-patch" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
        num(centre.x - width / 2.0 - pad),
        num(centre.y - height / 2.0),
        num(width + 2.0 * pad),
        num(height),
        num(pad)
    ));
    let baseline = centre.y - height / 2.0 + crate::font::baseline_in_line(&TYPOGRAPHY_SUBTITLE);
    svg.line(&format!(
        r#"<text class="sub" x="{}" y="{}" text-anchor="middle">{}</text>"#,
        num(centre.x),
        num(baseline),
        escape(label)
    ));
}

fn length(a: Point, b: Point) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
