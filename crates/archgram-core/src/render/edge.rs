//! Edges (DESIGN.md, Components: Connector): the router's orthogonal line
//! with its bends rounded, an open arrowhead at its end, and its label, if
//! any, in the box the layout kept for it.

use crate::geometry::{Point, Rect};
use crate::render::svg::{Svg, escape, num};
use crate::spec::{Edge, EdgeStyle};
use crate::tokens::{ARROWHEAD_GAP, CARD_PADDING, ROUNDED_CONNECTOR, TYPOGRAPHY_SUBTITLE};

/// How far the line stops short of the card it points at: the arrowhead's
/// tip is the line's end, so the tip leaves `arrowhead.gap` of air.
const INSET: f64 = ARROWHEAD_GAP;

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
    let p = &points;
    let mut i = 1;
    while i + 1 < count {
        let (from, corner, to) = (p[i - 1], p[i], p[i + 1]);
        // A jog: this bend and the next turn opposite ways across a step
        // shorter than two radii. Two arcs that small would kink, so the
        // jog is one S curve over a run of up to a radius on either side,
        // leaving the half of each segment that a neighbouring bend uses.
        if i + 2 < count {
            let next = p[i + 2];
            let step = length(corner, to);
            if step < 2.0 * ROUNDED_CONNECTOR && turn(from, corner, to) * turn(corner, to, next) < 0.0 {
                let before = length(from, corner);
                let after = length(to, next);
                let room_before = if i == 1 { before } else { before / 2.0 };
                let room_after = if i + 2 == count - 1 { after } else { after / 2.0 };
                let run = ROUNDED_CONNECTOR.min(room_before).min(room_after);
                if run > 0.0 {
                    let start = toward(corner, from, run);
                    let end = toward(to, next, run);
                    d.push(format!("L{} {}", num(start.x), num(start.y)));
                    d.push(format!(
                        "C{} {} {} {} {} {}",
                        num(corner.x),
                        num(corner.y),
                        num(to.x),
                        num(to.y),
                        num(end.x),
                        num(end.y)
                    ));
                    i += 2;
                    continue;
                }
            }
        }
        let (before, after) = (length(from, corner), length(corner, to));
        let radius = ROUNDED_CONNECTOR.min(before / 2.0).min(after / 2.0);
        if radius <= 0.0 {
            d.push(format!("L{} {}", num(corner.x), num(corner.y)));
            i += 1;
            continue;
        }
        let enter = toward(corner, from, radius);
        let leave = toward(corner, to, radius);
        // A clockwise turn on screen (y grows downwards) sweeps positively.
        let sweep = u8::from(turn(from, corner, to) > 0.0);
        d.push(format!("L{} {}", num(enter.x), num(enter.y)));
        d.push(format!(
            "A{r} {r} 0 0 {sweep} {} {}",
            num(leave.x),
            num(leave.y),
            r = num(radius)
        ));
        i += 1;
    }
    d.push(format!(
        "L{} {}",
        num(points[count - 1].x),
        num(points[count - 1].y)
    ));
    d.join(" ")
}

/// Which way the path turns at `b`: positive clockwise on screen, negative
/// counter-clockwise, zero straight on.
fn turn(a: Point, b: Point, c: Point) -> f64 {
    (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x)
}

/// The point `distance` from `from` towards `to` along their segment.
fn toward(from: Point, to: Point, distance: f64) -> Point {
    let whole = length(from, to);
    Point {
        x: from.x + (to.x - from.x) / whole * distance,
        y: from.y + (to.y - from.y) / whole * distance,
    }
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
        assert_eq!(
            d,
            "M0 0 L10 0 A10 10 0 0 1 20 10 L20 24 A16 16 0 0 0 36 40 L57 40"
        );
    }

    #[test]
    fn a_short_jog_is_one_s_curve() {
        // A step of 6 is too short for two bends: one S over a radius of run
        // on either side, 16 here.
        let d = rounded(&[pt(0.0, 0.0), pt(20.0, 0.0), pt(20.0, 6.0), pt(60.0, 6.0)]);
        assert_eq!(d, "M0 0 L4 0 C20 0 20 6 36 6 L57 6");
    }
}
