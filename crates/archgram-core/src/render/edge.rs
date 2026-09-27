//! Edges (DESIGN.md, Components: Connector). Until the router (M5), an edge
//! is drawn as straight segments through the layout's bends, cut where it
//! meets the two cards, with an open chevron at its end.

use crate::geometry::{Point, Rect};
use crate::render::svg::{Svg, num};
use crate::spec::{Edge, EdgeStyle};

/// Where the segment from the centre of `r` towards `towards` leaves `r`.
fn leave(r: &Rect, towards: Point) -> Point {
    let c = Point {
        x: r.centre_x(),
        y: r.centre_y(),
    };
    let (dx, dy) = (towards.x - c.x, towards.y - c.y);
    if dx == 0.0 && dy == 0.0 {
        return c;
    }
    let tx = if dx == 0.0 {
        f64::INFINITY
    } else {
        (r.w / 2.0) / dx.abs()
    };
    let ty = if dy == 0.0 {
        f64::INFINITY
    } else {
        (r.h / 2.0) / dy.abs()
    };
    let t = tx.min(ty);
    Point {
        x: c.x + dx * t,
        y: c.y + dy * t,
    }
}

/// Draws one edge along `path` (centre to centre) between `from` and `to`.
pub fn edge(svg: &mut Svg, e: &Edge, path: &[Point], from: &Rect, to: &Rect) {
    if path.len() < 2 {
        return;
    }
    let mut points = path.to_vec();
    let first = leave(from, points[1]);
    let last = leave(to, points[points.len() - 2]);
    points[0] = first;
    let end = points.len() - 1;
    points[end] = last;
    let d = points
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
}
