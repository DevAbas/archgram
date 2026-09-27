//! Edges (DESIGN.md, Components: Connector): the router's orthogonal line
//! with its bends rounded, a filled arrowhead at its end, and its label, if
//! any, in the box the layout kept for it.

use crate::geometry::{Point, Rect};
use crate::render::svg::{Svg, escape, num};
use crate::spec::{Edge, EdgeStyle};
use crate::tokens::{ARROWHEAD_LENGTH, CARD_PADDING, ROUNDED_CONNECTOR, TYPOGRAPHY_SUBTITLE};

/// How far the line stops short of its end: halfway into the arrowhead, so
/// the line's end never shows past the tip (as D2 and Mermaid draw it). The
/// arrowhead's marker puts the tip back on the end.
pub const INSET: f64 = ARROWHEAD_LENGTH / 2.0;

/// Draws one edge along `path`, which already starts and ends on the sides
/// of its cards, with its label in `label`.
pub fn edge(svg: &mut Svg, e: &Edge, path: &[Point], label: Option<Rect>) {
    if path.len() < 2 {
        return;
    }
    let d = rounded(path);
    let class = match e.style {
        EdgeStyle::Solid => "edge",
        EdgeStyle::Dashed => "edge dashed",
    };
    svg.line(&format!(
        r#"<path class="{class}" d="{d}" marker-end="url(#arrow)"/>"#
    ));
    if let (Some(text), Some(at)) = (&e.label, label) {
        edge_label(svg, text, at);
    }
}

/// The path's `d`: the line stopped `INSET` short of its end, each bend
/// rounded by `rounded.connector`, never more than half of either segment
/// beside it, so a short step stays a step and does not turn into an S.
fn rounded(path: &[Point]) -> String {
    let mut points = path.to_vec();
    let count = points.len();
    let (before_end, end) = (points[count - 2], points[count - 1]);
    let last = length(before_end, end);
    if last > INSET {
        let t = INSET / last;
        points[count - 1] = Point {
            x: end.x - (end.x - before_end.x) * t,
            y: end.y - (end.y - before_end.y) * t,
        };
    }
    let mut d = vec![format!("M{} {}", num(points[0].x), num(points[0].y))];
    for w in points.windows(3) {
        let (from, corner, to) = (w[0], w[1], w[2]);
        let (before, after) = (length(from, corner), length(corner, to));
        let radius = ROUNDED_CONNECTOR.min(before / 2.0).min(after / 2.0);
        if radius <= 0.0 {
            d.push(format!("L{} {}", num(corner.x), num(corner.y)));
            continue;
        }
        let enter = Point {
            x: corner.x - (corner.x - from.x) / before * radius,
            y: corner.y - (corner.y - from.y) / before * radius,
        };
        let leave = Point {
            x: corner.x + (to.x - corner.x) / after * radius,
            y: corner.y + (to.y - corner.y) / after * radius,
        };
        // A clockwise turn on screen (y grows downwards) sweeps positively.
        let turn = (corner.x - from.x) * (to.y - corner.y) - (corner.y - from.y) * (to.x - corner.x);
        let sweep = u8::from(turn > 0.0);
        d.push(format!("L{} {}", num(enter.x), num(enter.y)));
        d.push(format!(
            "A{r} {r} 0 0 {sweep} {} {}",
            num(leave.x),
            num(leave.y),
            r = num(radius)
        ));
    }
    d.push(format!(
        "L{} {}",
        num(points[count - 1].x),
        num(points[count - 1].y)
    ));
    d.join(" ")
}

/// The length of an axis-aligned segment.
fn length(a: Point, b: Point) -> f64 {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

/// The label in its box, over a patch of canvas so the line does not run
/// through the text.
fn edge_label(svg: &mut Svg, label: &str, at: Rect) {
    let pad = CARD_PADDING / 2.0;
    svg.line(&format!(
        r#"<rect class="label-patch" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
        num(at.x),
        num(at.y),
        num(at.w),
        num(at.h),
        num(pad)
    ));
    let baseline = at.y + crate::font::baseline_in_line(&TYPOGRAPHY_SUBTITLE);
    svg.line(&format!(
        r#"<text class="sub" x="{}" y="{}" text-anchor="middle">{}</text>"#,
        num(at.x + at.w / 2.0),
        num(baseline),
        escape(label)
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt(x: f64, y: f64) -> Point {
        Point { x, y }
    }

    #[test]
    fn a_step_turns_twice_with_opposite_arcs_and_stops_short() {
        // Right, down, right: a clockwise turn, then a counter-clockwise one.
        let d = rounded(&[pt(0.0, 0.0), pt(20.0, 0.0), pt(20.0, 40.0), pt(60.0, 40.0)]);
        assert_eq!(d, "M0 0 L12 0 A8 8 0 0 1 20 8 L20 32 A8 8 0 0 0 28 40 L56 40");
    }

    #[test]
    fn a_short_jog_rounds_by_half_its_length() {
        // The middle segment is 6 long: each bend takes 3, and no more.
        let d = rounded(&[pt(0.0, 0.0), pt(20.0, 0.0), pt(20.0, 6.0), pt(60.0, 6.0)]);
        assert_eq!(d, "M0 0 L17 0 A3 3 0 0 1 20 3 L20 3 A3 3 0 0 0 23 6 L56 6");
    }
}
