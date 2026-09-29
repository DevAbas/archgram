//! Edges (DESIGN.md, Components: Connector): the router's orthogonal line
//! with its bends rounded, an open arrowhead at its end, and its label, if
//! any, in the box the layout kept for it.

use crate::geometry::{Point, Rect};
use crate::render::scene::{Anchor, Item};
use crate::render::svg::num;
use crate::spec::{Edge, EdgeStyle};
use crate::tokens::{
    ARROWHEAD_GAP, ARROWHEAD_LENGTH, ARROWHEAD_WIDTH, CARD_PADDING, ROUNDED_CONNECTOR,
    TYPOGRAPHY_SUBTITLE,
};

/// How far the line stops short of the card it points at: the arrowhead's
/// tip is the line's end, so the tip leaves `arrowhead.gap` of air.
const INSET: f64 = ARROWHEAD_GAP;

/// One edge along `drawn`, then its label in `label`. An edge a flow
/// takes carries `id`, for the signal to follow.
pub fn edge(e: &Edge, drawn: &Drawn, label: Option<Rect>, id: Option<&str>) -> Vec<Item> {
    if drawn.pieces.is_empty() {
        return Vec::new();
    }
    let class = match e.style {
        EdgeStyle::Solid => "edge",
        EdgeStyle::Dashed => "edge dashed",
    };
    let mut items = vec![Item::Path {
        id: id.map(str::to_owned),
        class: class.into(),
        place: None,
        d: drawn.d(),
        arrowhead: true,
    }];
    if let (Some(text), Some(at)) = (&e.label, label) {
        items.extend(edge_label(text, at, ("label-patch", "sub")));
    }
    items
}

/// One piece of an edge as drawn, from where the one before it ends.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Piece {
    Line(Point),
    /// A quarter circle of radius `r`: every bend of an orthogonal line turns
    /// by a right angle. `clockwise` on screen, where y grows downwards.
    Arc {
        r: f64,
        clockwise: bool,
        to: Point,
    },
    /// The S curve of a short jog.
    Cubic {
        c1: Point,
        c2: Point,
        to: Point,
    },
}

/// An edge as drawn: its start and its pieces.
#[derive(Debug, Clone, PartialEq)]
pub struct Drawn {
    pub start: Point,
    pub pieces: Vec<Piece>,
}

impl Drawn {
    /// The path's `d`.
    #[must_use]
    pub fn d(&self) -> String {
        let mut d = vec![format!("M{} {}", num(self.start.x), num(self.start.y))];
        for piece in &self.pieces {
            d.push(match *piece {
                Piece::Line(to) => format!("L{} {}", num(to.x), num(to.y)),
                Piece::Arc { r, clockwise, to } => format!(
                    "A{r} {r} 0 0 {} {} {}",
                    u8::from(clockwise),
                    num(to.x),
                    num(to.y),
                    r = num(r)
                ),
                Piece::Cubic { c1, c2, to } => format!(
                    "C{} {} {} {} {} {}",
                    num(c1.x),
                    num(c1.y),
                    num(c2.x),
                    num(c2.y),
                    num(to.x),
                    num(to.y)
                ),
            });
        }
        d.join(" ")
    }

    /// The length along the path: straight pieces exactly, a quarter circle
    /// as `π·r/2`, an S curve by Gauss–Legendre quadrature. Only `+ * /` and
    /// `sqrt`, which IEEE 754 rounds the same everywhere, so the length, and
    /// the timing drawn from it, is the same on every machine.
    #[must_use]
    pub fn length(&self) -> f64 {
        let mut at = self.start;
        let mut total = 0.0;
        for piece in &self.pieces {
            total += match *piece {
                Piece::Line(to) => distance(at, to),
                Piece::Arc { r, .. } => std::f64::consts::FRAC_PI_2 * r,
                Piece::Cubic { c1, c2, to } => cubic_length(at, c1, c2, to),
            };
            at = end(*piece);
        }
        total
    }

    /// The straight pieces, each from its start to its end, in order.
    #[must_use]
    pub fn straights(&self) -> Vec<(Point, Point)> {
        let mut at = self.start;
        let mut out = Vec::new();
        for piece in &self.pieces {
            if let Piece::Line(to) = *piece {
                out.push((at, to));
            }
            at = end(*piece);
        }
        out
    }

    /// Where the line ends: the arrowhead's tip.
    #[must_use]
    pub fn tip(&self) -> Point {
        self.pieces.last().map_or(self.start, |&p| end(p))
    }

    /// The way the line points at its tip, as a unit vector: along its last
    /// straight piece, which every edge ends with.
    #[must_use]
    pub fn heading(&self) -> Point {
        let (a, b) = self
            .straights()
            .last()
            .copied()
            .unwrap_or((self.start, self.tip()));
        let l = distance(a, b).max(f64::EPSILON);
        Point {
            x: (b.x - a.x) / l,
            y: (b.y - a.y) / l,
        }
    }

    /// The point `back` pixels behind the tip, along the last straight piece.
    #[must_use]
    pub fn behind_tip(&self, back: f64) -> Point {
        let (tip, u) = (self.tip(), self.heading());
        Point {
            x: tip.x - u.x * back,
            y: tip.y - u.y * back,
        }
    }

    /// The arrowhead as its own path, as the connector's marker draws it:
    /// two legs `arrowhead.length` back along the line, `arrowhead.width`
    /// apart across it, meeting at the tip.
    #[must_use]
    pub fn chevron(&self) -> String {
        let (tip, u) = (self.tip(), self.heading());
        let base = self.behind_tip(ARROWHEAD_LENGTH);
        let (nx, ny) = (-u.y * ARROWHEAD_WIDTH / 2.0, u.x * ARROWHEAD_WIDTH / 2.0);
        format!(
            "M{} {}L{} {}L{} {}",
            num(base.x + nx),
            num(base.y + ny),
            num(tip.x),
            num(tip.y),
            num(base.x - nx),
            num(base.y - ny)
        )
    }
}

fn end(piece: Piece) -> Point {
    match piece {
        Piece::Line(to) | Piece::Arc { to, .. } | Piece::Cubic { to, .. } => to,
    }
}

fn distance(a: Point, b: Point) -> f64 {
    ((b.x - a.x) * (b.x - a.x) + (b.y - a.y) * (b.y - a.y)).sqrt()
}

/// A cubic Bézier's length: five-point Gauss–Legendre quadrature of its
/// speed over `t` in 0..1.
fn cubic_length(p0: Point, c1: Point, c2: Point, p3: Point) -> f64 {
    // Nodes on -1..1 and their weights (Abramowitz and Stegun, table 25.4).
    const NODES: [(f64, f64); 5] = [
        (0.0, 0.568_888_888_888_888_9),
        (-0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
        (0.538_469_310_105_683_1, 0.478_628_670_499_366_5),
        (-0.906_179_845_938_664, 0.236_926_885_056_189_1),
        (0.906_179_845_938_664, 0.236_926_885_056_189_1),
    ];
    let speed = |t: f64| {
        let u = 1.0 - t;
        let (a, b, c) = (3.0 * u * u, 6.0 * u * t, 3.0 * t * t);
        let dx = a * (c1.x - p0.x) + b * (c2.x - c1.x) + c * (p3.x - c2.x);
        let dy = a * (c1.y - p0.y) + b * (c2.y - c1.y) + c * (p3.y - c2.y);
        (dx * dx + dy * dy).sqrt()
    };
    NODES
        .iter()
        .map(|&(x, w)| w * speed(0.5 * x + 0.5))
        .sum::<f64>()
        * 0.5
}

/// The edge along `path`, which starts and ends on the sides of its cards:
/// the line stopped `INSET` short of its end, each bend rounded by
/// `rounded.connector`, never more than half of either segment beside it,
/// so a short step stays a step and does not turn into an S.
#[must_use]
pub fn drawn(path: &[Point]) -> Drawn {
    if path.len() < 2 {
        return Drawn {
            start: path.first().copied().unwrap_or(Point { x: 0.0, y: 0.0 }),
            pieces: Vec::new(),
        };
    }
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
    let mut pieces = Vec::new();
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
            if step < 2.0 * ROUNDED_CONNECTOR
                && turn(from, corner, to) * turn(corner, to, next) < 0.0
            {
                let before = length(from, corner);
                let after = length(to, next);
                let room_before = if i == 1 { before } else { before / 2.0 };
                let room_after = if i + 2 == count - 1 {
                    after
                } else {
                    after / 2.0
                };
                let run = ROUNDED_CONNECTOR.min(room_before).min(room_after);
                if run > 0.0 {
                    let start = toward(corner, from, run);
                    let end = toward(to, next, run);
                    pieces.push(Piece::Line(start));
                    pieces.push(Piece::Cubic {
                        c1: corner,
                        c2: to,
                        to: end,
                    });
                    i += 2;
                    continue;
                }
            }
        }
        let (before, after) = (length(from, corner), length(corner, to));
        let radius = ROUNDED_CONNECTOR.min(before / 2.0).min(after / 2.0);
        if radius <= 0.0 {
            pieces.push(Piece::Line(corner));
            i += 1;
            continue;
        }
        let enter = toward(corner, from, radius);
        let leave = toward(corner, to, radius);
        // A clockwise turn on screen (y grows downwards) sweeps positively.
        pieces.push(Piece::Line(enter));
        pieces.push(Piece::Arc {
            r: radius,
            clockwise: turn(from, corner, to) > 0.0,
            to: leave,
        });
        i += 1;
    }
    pieces.push(Piece::Line(points[count - 1]));
    Drawn {
        start: points[0],
        pieces,
    }
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
/// through the text; `classes` are the patch's and the text's, and a
/// signal's copy of the label takes its colour through the text's
/// (`signal::signals`).
pub fn edge_label(label: &str, at: Rect, (patch, text): (&str, &str)) -> [Item; 2] {
    let pad = CARD_PADDING / 2.0;
    [
        Item::Rect {
            class: patch.into(),
            x: at.x,
            y: at.y,
            w: at.w,
            h: at.h,
            rx: Some(pad),
        },
        Item::Text {
            class: text.into(),
            x: at.x + at.w / 2.0,
            y: at.y + crate::font::baseline_in_line(&TYPOGRAPHY_SUBTITLE),
            anchor: Anchor::Middle,
            text: label.to_owned(),
        },
    ]
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
        let d = drawn(&[pt(0.0, 0.0), pt(20.0, 0.0), pt(20.0, 40.0), pt(60.0, 40.0)]).d();
        assert_eq!(
            d,
            "M0 0 L10 0 A10 10 0 0 1 20 10 L20 24 A16 16 0 0 0 36 40 L57 40"
        );
    }

    #[test]
    fn a_short_jog_is_one_s_curve() {
        // A step of 6 is too short for two bends: one S over a radius of run
        // on either side, 16 here.
        let d = drawn(&[pt(0.0, 0.0), pt(20.0, 0.0), pt(20.0, 6.0), pt(60.0, 6.0)]).d();
        assert_eq!(d, "M0 0 L4 0 C20 0 20 6 36 6 L57 6");
    }

    #[test]
    fn length_counts_lines_arcs_and_curves() {
        // 10 + a quarter of radius 10 + 14 + a quarter of radius 16 + 21.
        let step = drawn(&[pt(0.0, 0.0), pt(20.0, 0.0), pt(20.0, 40.0), pt(60.0, 40.0)]);
        let want = 45.0 + std::f64::consts::FRAC_PI_2 * 26.0;
        assert!((step.length() - want).abs() < 1e-9);
        // A straight cubic is as long as its chord.
        let straight = cubic_length(pt(0.0, 0.0), pt(10.0, 0.0), pt(20.0, 0.0), pt(30.0, 0.0));
        assert!((straight - 30.0).abs() < 1e-9);
        // The jog's S: longer than its chord, shorter than its control polygon.
        let s = cubic_length(pt(4.0, 0.0), pt(20.0, 0.0), pt(20.0, 6.0), pt(36.0, 6.0));
        assert!(s > 32.56 && s < 38.0, "{s}");
    }
}
