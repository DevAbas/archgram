//! A lit card's border, drawn from the arrow that reaches it (DESIGN.md,
//! Components: Signal and Refusal): the card's own edge, at its own width,
//! traced both ways round from the point the arrow meets it and closing on
//! the far side, in the pass colour or the refusal colour, in the style the
//! spec names. The card's fill, icon and text never change.
//!
//! The outline is walked as its straight sides and its quarter circles,
//! with only `+ - * /`: a trace starts and ends on a straight side, so it
//! always takes a corner whole.

use std::fmt::Write as _;

use crate::geometry::{Point, Rect};
use crate::motion::{Entry, Lit, State, Timeline, fade};
use crate::render::edge::Drawn;
use crate::render::scene::{GroupOf, Item};
use crate::render::signal::KeyTrack;
use crate::render::svg::num;
use crate::spec::{BorderStyle, Wait};
use crate::tokens::{MOTION_PENDING_MS, REFUSAL_PENDING, ROUNDED_CARD, SIGNAL_CORE};

/// Every lit time's border, above the cards. `cards` holds each node's
/// front card on the canvas, `drawn` each edge as drawn.
pub fn borders(
    timeline: &Timeline,
    cards: &[Rect],
    drawn: &[Drawn],
    (style, wait): (BorderStyle, Wait),
) -> Item {
    let items = timeline
        .lit
        .iter()
        .map(|l| {
            Item::Motion(border(
                l,
                timeline.period,
                cards[l.node],
                drawn,
                (style, wait),
            ))
        })
        .collect();
    Item::Group {
        of: GroupOf::Class("borders"),
        items,
    }
}

/// The class a border takes for its state: its colour.
fn state_class(state: State) -> &'static str {
    match state {
        State::Pass => "pass",
        State::Refused => "refused",
    }
}

/// One lit time's border on `card`.
fn border(
    l: &Lit,
    period: u32,
    card: Rect,
    drawn: &[Drawn],
    (style, wait): (BorderStyle, Wait),
) -> String {
    let outline = Outline::of(card);
    let t = Times::of(l, period);
    // Where it starts, and where its two halves meet.
    let (from, to) = match l.entry {
        Entry::Arrow(e) => {
            let s = outline.at(drawn[e].tip());
            (s, outline.opposite(s))
        }
        Entry::Back(e) => {
            let s = outline.at(drawn[e].start);
            (s, outline.opposite(s))
        }
        Entry::Leave(e) => {
            let s = outline.at(drawn[e].start);
            (outline.opposite(s), s)
        }
    };
    let class = state_class(l.state);
    let halves = [outline.walk(from, to), outline.walk_back(from, to)];
    let lines =
        |animation: &str| -> String { halves.iter().map(|d| line(class, d, animation)).collect() };
    let passing = l.state == State::Pass;
    match style {
        BorderStyle::Drain if passing => {
            // Drawn in, then drained toward its far side as its signal
            // leaves, or as it goes.
            let d0 = l.leaves.unwrap_or(t.hold).max(t.t1).min(period);
            let d1 = (d0 + l.trace).min(period);
            let out = t.track(
                &[
                    (t.t0, "1000"),
                    (t.t1, "0"),
                    (d0, "0"),
                    (d1, "-1000"),
                    (period, "-1000"),
                ],
                "1000",
            );
            let shown = t.seen(&[(t.t0, "1"), (d1, "1"), ((d1 + 1).min(period), "0")]);
            return format!(r#"<g opacity="0">{shown}{}</g>"#, lines(&dash_offset(&out)));
        }
        BorderStyle::Afterglow if passing => {
            // Drawn in, then cooling back to the card's own edge.
            let cool = t.gone.max((t.t1 + fade()).min(period));
            let shown = t.seen(&[(t.t0, "1"), (t.t1, "1"), (cool, "0")]);
            return format!(r#"<g opacity="0">{shown}{}</g>"#, lines(&t.draw_in()));
        }
        _ => {}
    }
    let body = match style {
        BorderStyle::Ring => line(
            class,
            &outline.walk(from, from + outline.length),
            &t.draw_in(),
        ),
        BorderStyle::Spark => {
            let heads: String = halves.iter().map(|d| head(d, class, &t)).collect();
            format!("{}{heads}", lines(&t.draw_in()))
        }
        _ => lines(&t.draw_in()),
    };
    // The refused card waits as marching dashes once its border is drawn.
    if wait == Wait::Pending && l.state == State::Refused {
        let until = (t.t1 + fade()).min(period);
        let shown = t.seen(&[(t.t0, "1"), (until, "1"), (until, "0")]);
        return format!(
            r#"<g opacity="0">{shown}{body}</g>{}"#,
            dashes(&outline, class, &t)
        );
    }
    let shown = t.seen(&[(t.t0, "1"), (t.hold, "1"), (t.gone, "0")]);
    format!(r#"<g opacity="0">{shown}{body}</g>"#)
}

/// One half of a border along `d`, drawn by `animation`.
fn line(class: &str, d: &str, animation: &str) -> String {
    format!(
        r#"<path class="border {class}" d="{d}" pathLength="1000" stroke-dasharray="1000 1000">{animation}</path>"#
    )
}

/// A dash offset animation with `attributes`.
fn dash_offset(attributes: &str) -> String {
    format!(r#"<animate attributeName="stroke-dashoffset" {attributes}/>"#)
}

/// A lit time's moments: its border starts at `t0` and closes at `t1`,
/// holds until `hold` and is gone by `gone`, in a cycle `period` long.
struct Times {
    t0: u32,
    t1: u32,
    hold: u32,
    gone: u32,
    period: u32,
}

impl Times {
    fn of(l: &Lit, period: u32) -> Times {
        let t1 = (l.start + l.trace).min(period);
        let hold = l.end.max(t1).min(period);
        Times {
            t0: l.start,
            t1,
            hold,
            gone: (hold + fade()).min(period),
            period,
        }
    }

    /// A track from `first` at 0 through `keys`, ending at the cycle's end.
    fn track(&self, keys: &[(u32, &str)], first: &str) -> String {
        let mut t = KeyTrack::new(self.period);
        t.key(0, first);
        for &(at, v) in keys {
            t.key(at, v);
        }
        if keys.last().is_none_or(|k| k.0 < self.period) {
            let last = keys.last().map_or(first, |k| k.1);
            t.key(self.period, last);
        }
        t.attributes("values")
    }

    /// The half drawn in from `t0` to `t1`.
    fn draw_in(&self) -> String {
        dash_offset(&self.track(&[(self.t0, "1000"), (self.t1, "0")], "1000"))
    }

    /// Unseen until a fade before `t0`, then as `keys` say, then unseen.
    fn seen(&self, keys: &[(u32, &str)]) -> String {
        let mut all = vec![(self.t0.saturating_sub(fade()), "0")];
        all.extend_from_slice(keys);
        if all.last().is_some_and(|k| k.1 != "0") {
            all.push(((all[all.len() - 1].0 + fade()).min(self.period), "0"));
        }
        all.push((self.period, "0"));
        format!(
            r#"<animate attributeName="opacity" {}/>"#,
            self.track(&all, "0")
        )
    }
}

/// A dot riding the growing end of the half `d` while its border is drawn.
fn head(d: &str, class: &str, t: &Times) -> String {
    let along = t.track(&[(t.t0, "0"), (t.t1, "1")], "0");
    let shown = t.track(
        &[
            (t.t0, "0"),
            (t.t0, "1"),
            (t.t1, "1"),
            ((t.t1 + fade()).min(t.period), "0"),
        ],
        "0",
    );
    format!(
        r#"<g class="border-head {class}" opacity="0"><animate attributeName="opacity" {shown}/><circle r="{}"/><animateMotion path="{d}" {} calcMode="linear"/></g>"#,
        num(SIGNAL_CORE),
        along.replacen("values=", "keyPoints=", 1)
    )
}

/// The refused card's border as dashes of `refusal.pending` marching round
/// it, one step every `motion.pending`, from when it is drawn until it goes.
fn dashes(outline: &Outline, class: &str, t: &Times) -> String {
    let step: f64 = REFUSAL_PENDING.iter().sum();
    let shown = t.track(
        &[
            (t.t1, "0"),
            ((t.t1 + fade()).min(t.period), "1"),
            (t.hold, "1"),
            (t.gone, "0"),
        ],
        "0",
    );
    let Rect { x, y, w, h } = outline.card;
    format!(
        r#"<g opacity="0"><animate attributeName="opacity" {shown}/><rect class="border pending {class}" x="{}" y="{}" width="{}" height="{}" rx="{}"><animate attributeName="stroke-dashoffset" values="0;{}" dur="{}ms" repeatCount="indefinite"/></rect></g>"#,
        num(x),
        num(y),
        num(w),
        num(h),
        num(outline.r),
        num(-step),
        num(MOTION_PENDING_MS)
    )
}

/// A card's rounded outline, clockwise from the start of its top side.
/// A place on it is its distance along it from there.
struct Outline {
    card: Rect,
    r: f64,
    /// Each side and corner: where it starts along the outline, its length,
    /// and whether it is a corner.
    parts: [(f64, f64, bool); 8],
    length: f64,
}

impl Outline {
    fn of(card: Rect) -> Outline {
        let r = ROUNDED_CARD.min(card.w / 2.0).min(card.h / 2.0);
        let (across, down) = (card.w - 2.0 * r, card.h - 2.0 * r);
        let corner = std::f64::consts::FRAC_PI_2 * r;
        let lengths = [across, corner, down, corner, across, corner, down, corner];
        let mut parts = [(0.0, 0.0, false); 8];
        let mut at = 0.0;
        for (i, &l) in lengths.iter().enumerate() {
            parts[i] = (at, l, i % 2 == 1);
            at += l;
        }
        Outline {
            card,
            r,
            parts,
            length: at,
        }
    }

    /// The place on a straight side nearest `point`.
    fn at(&self, point: Point) -> f64 {
        let Rect {
            x: left,
            y: top,
            w: width,
            h: height,
        } = self.card;
        let (right, bottom, r) = (left + width, top + height, self.r);
        let along = |v: f64, from: f64, span: f64| (v - from).max(0.0).min(span);
        let (across, down) = (width - 2.0 * r, height - 2.0 * r);
        // Each side: where its straight starts along the outline, how far
        // the point is from the side, and how far along the side it falls.
        let sides = [
            (
                self.parts[0].0,
                (point.y - top).abs(),
                along(point.x, left + r, across),
            ),
            (
                self.parts[2].0,
                (point.x - right).abs(),
                along(point.y, top + r, down),
            ),
            (
                self.parts[4].0,
                (point.y - bottom).abs(),
                along(right - r, point.x, across),
            ),
            (
                self.parts[6].0,
                (point.x - left).abs(),
                along(bottom - r, point.y, down),
            ),
        ];
        let mut best = sides[0];
        for side in &sides[1..] {
            if side.1 < best.1 {
                best = *side;
            }
        }
        best.0 + best.2
    }

    /// The place across the outline from `s`, moved off a corner onto the
    /// nearer side, so a trace ends on a straight.
    fn opposite(&self, s: f64) -> f64 {
        let t = (s + self.length / 2.0) % self.length;
        match self.parts.iter().find(|p| p.2 && t > p.0 && t < p.0 + p.1) {
            Some(&(start, l, _)) if t - start < l / 2.0 => start,
            Some(&(start, l, _)) => (start + l) % self.length,
            None => t,
        }
    }

    /// Where each side and corner starts, clockwise from the top side's.
    fn starts(&self) -> [Point; 8] {
        let Rect {
            x: left,
            y: top,
            w: width,
            h: height,
        } = self.card;
        let (right, bottom, r) = (left + width, top + height, self.r);
        let at = |x, y| Point { x, y };
        [
            at(left + r, top),
            at(right - r, top),
            at(right, top + r),
            at(right, bottom - r),
            at(right - r, bottom),
            at(left + r, bottom),
            at(left, bottom - r),
            at(left, top + r),
        ]
    }

    /// The point at `place`, on a straight side or a corner's end.
    fn point(&self, place: f64) -> Point {
        let place = place.rem_euclid(self.length);
        let i = self.parts.iter().rposition(|p| place >= p.0).unwrap_or(0);
        let (start, length, corner) = self.parts[i];
        let along = place - start;
        let starts = self.starts();
        // A corner is only ever met at its ends.
        if corner {
            return if along < length / 2.0 {
                starts[i]
            } else {
                starts[(i + 1) % 8]
            };
        }
        let (dx, dy) = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)][i / 2];
        Point {
            x: starts[i].x + dx * along,
            y: starts[i].y + dy * along,
        }
    }

    /// The path clockwise from place `from` to place `to`.
    fn walk(&self, from: f64, to: f64) -> String {
        let pieces = self.pieces(from, to);
        let start = self.point(from);
        let mut d = format!("M{} {}", num(start.x), num(start.y));
        for (end, corner) in pieces {
            let p = self.point(end);
            let _ = if corner {
                write!(
                    d,
                    "A{r} {r} 0 0 1 {} {}",
                    num(p.x),
                    num(p.y),
                    r = num(self.r)
                )
            } else {
                write!(d, "L{} {}", num(p.x), num(p.y))
            };
        }
        d
    }

    /// The path counter-clockwise from place `from` to place `to`: the
    /// clockwise path from `to` to `from`, walked back.
    fn walk_back(&self, from: f64, to: f64) -> String {
        let pieces = self.pieces(to, from);
        let start = self.point(from);
        let mut d = format!("M{} {}", num(start.x), num(start.y));
        // Each piece's start is the end of the one before it.
        let mut starts: Vec<f64> = vec![to];
        starts.extend(pieces.iter().map(|p| p.0));
        starts.pop();
        for ((_, corner), &begin) in pieces.iter().rev().zip(starts.iter().rev()) {
            let p = self.point(begin);
            let _ = if *corner {
                write!(
                    d,
                    "A{r} {r} 0 0 0 {} {}",
                    num(p.x),
                    num(p.y),
                    r = num(self.r)
                )
            } else {
                write!(d, "L{} {}", num(p.x), num(p.y))
            };
        }
        d
    }

    /// The pieces clockwise from `from` to `to`, each as the place it ends
    /// and whether it is a corner. A whole turn when they are the same place.
    fn pieces(&self, from: f64, to: f64) -> Vec<(f64, bool)> {
        let mut span = (to - from).rem_euclid(self.length);
        if span < 1e-6 {
            span = self.length;
        }
        let mut out = Vec::new();
        let mut t = from;
        let end = from + span;
        while end - t > 1e-6 {
            let s = t.rem_euclid(self.length);
            let &(start, l, corner) = self
                .parts
                .iter()
                .rev()
                .find(|p| s + 1e-6 >= p.0)
                .unwrap_or(&self.parts[0]);
            let mut part_end = t + (start + l - s);
            if part_end - t < 1e-6 {
                // At the part's very end: go on with the next part.
                t = part_end;
                continue;
            }
            if part_end > end {
                part_end = end;
            }
            out.push((part_end, corner));
            t = part_end;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card() -> Outline {
        Outline::of(Rect {
            x: 100.0,
            y: 50.0,
            w: 200.0,
            h: 60.0,
        })
    }

    #[test]
    fn an_arrow_from_above_starts_the_border_at_the_top_and_meets_at_the_bottom() {
        let o = card();
        let s = o.at(Point { x: 200.0, y: 46.5 });
        assert_eq!(o.point(s), Point { x: 200.0, y: 50.0 });
        assert_eq!(o.point(o.opposite(s)), Point { x: 200.0, y: 110.0 });
        let r = num(ROUNDED_CARD);
        assert_eq!(
            o.walk(s, o.opposite(s)),
            format!("M200 50L290 50A{r} {r} 0 0 1 300 60L300 100A{r} {r} 0 0 1 290 110L200 110")
        );
        assert_eq!(
            o.walk_back(s, o.opposite(s)),
            format!("M200 50L110 50A{r} {r} 0 0 0 100 60L100 100A{r} {r} 0 0 0 110 110L200 110")
        );
    }

    #[test]
    fn a_ring_goes_all_the_way_round() {
        let o = card();
        let s = o.at(Point { x: 303.5, y: 80.0 });
        assert_eq!(o.point(s), Point { x: 300.0, y: 80.0 });
        let ring = o.walk(s, s);
        assert!(ring.starts_with("M300 80L300 100"), "{ring}");
        assert!(ring.ends_with("L300 80"), "{ring}");
        assert_eq!(ring.matches('A').count(), 4);
    }

    #[test]
    fn a_far_side_on_a_corner_moves_onto_a_side() {
        let o = card();
        // Near the top's right end: its opposite would fall in the lower left corner.
        let s = o.at(Point { x: 288.0, y: 50.0 });
        let p = o.point(o.opposite(s));
        assert!(
            (p.y - 110.0).abs() < 1e-9 || (p.x - 100.0).abs() < 1e-9,
            "{p:?}"
        );
    }
}
