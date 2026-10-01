//! A refused flow (DESIGN.md, Components: Refusal): the ✕ on the line into
//! the refusing card, the arrowhead after it in the refusal colour, and the
//! refusal travelling back along the flow; in the still image, the ✕ and
//! the refusing card's border, which stay where nothing moves.

use crate::geometry::Rect;
use crate::motion::{Entry, State, Timeline, fade};
use crate::render::edge::Drawn;
use crate::render::scene::{GroupOf, Item};
use crate::render::signal::KeyTrack;
use crate::render::svg::num;
use crate::tokens::{
    ARROWHEAD_LENGTH, REFUSAL_MARK, REFUSAL_MARK_GAP, ROUNDED_CARD, STROKE_CONNECTOR,
};

/// Every refusal's marks and way back, for the moving picture. On an edge
/// `labelled` says has a label, the way back fades out round it as a
/// signal does (`signal::label_gap`).
pub fn refusals(timeline: &Timeline, drawn: &[Drawn], labelled: &[bool]) -> Vec<Item> {
    let period = timeline.period;
    let f = fade();
    let mut items = Vec::new();
    for r in &timeline.refusals {
        let d = &drawn[r.edge];
        let gone = (r.end + f).min(period);
        items.push(Item::Motion(format!(
            r#"<g class="refusal" opacity="0">{}{}</g>"#,
            seen(r.start, gone, period),
            cross(d)
        )));
        let chevron = d.chevron();
        items.push(Item::Motion(format!(
            r#"<g class="refusal" opacity="0">{}<path d="{chevron}"/></g>"#,
            seen(r.back, gone, period)
        )));
    }
    for back in &timeline.returns {
        // The way back stays drawn while the card it reached is refused.
        let until = timeline
            .lit
            .iter()
            .filter(|l| {
                l.state == State::Refused
                    && matches!(l.entry, Entry::Back(_))
                    && l.start >= back.end
            })
            .min_by_key(|l| l.start)
            .map_or(back.end, |l| l.end);
        let mut t = KeyTrack::new(period);
        t.key(0, "-1000")
            .key(back.start, "-1000")
            .key(back.end, "0")
            .key(period, "0");
        let along = format!(
            r#"<animate attributeName="stroke-dashoffset" {}/>"#,
            t.attributes("values")
        );
        let d = drawn[back.edge].d();
        let line = |class: &str| {
            format!(
                r#"<path class="{class}" d="{d}" pathLength="1000" stroke-dasharray="1000 1000">{along}</path>"#
            )
        };
        let mut body = line("back");
        if labelled.get(back.edge).copied().unwrap_or(false) {
            body = format!(
                r#"<g mask="url(#{})">{body}</g>"#,
                crate::render::signal::GAP
            );
        }
        items.push(Item::Motion(format!(
            r#"<g class="refusal" opacity="0">{}{body}</g>"#,
            seen(back.start, (until + f).min(period), period)
        )));
    }
    items
}

/// What stays of each refusal where nothing moves: its ✕, and the refusing
/// card's border in the refusal colour. `cards` holds each node's front
/// card on the canvas.
pub fn still(timeline: &Timeline, drawn: &[Drawn], cards: &[Rect]) -> Option<Item> {
    let mut items = Vec::new();
    for r in &timeline.refusals {
        items.push(Item::Motion(format!(
            r#"<g class="refusal">{}</g>"#,
            cross(&drawn[r.edge])
        )));
        let refused = timeline.lit.iter().find(|l| {
            l.state == State::Refused && l.start == r.start && l.entry == Entry::Arrow(r.edge)
        });
        if let Some(l) = refused {
            let c = cards[l.node];
            items.push(Item::Motion(format!(
                r#"<rect class="border refused" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
                num(c.x),
                num(c.y),
                num(c.w),
                num(c.h),
                num(ROUNDED_CARD.min(c.w / 2.0).min(c.h / 2.0))
            )));
        }
    }
    (!items.is_empty()).then_some(Item::Group {
        of: GroupOf::Class("still-refusal"),
        items,
    })
}

/// The ✕ of `refusal.mark` on the line `d`, `refusal.mark-gap` clear of
/// the arrowhead's base, over a patch of the canvas's colour so the line
/// does not cross it.
/// How far the ✕'s patch of canvas reaches beyond the mark: a stroke's
/// width and a half (`.refusal .patch`).
const PATCH: f64 = 1.5 * STROKE_CONNECTOR;

/// How far back from the tip the ✕ on a line reaches, its patch included.
#[must_use]
pub fn mark_reach() -> f64 {
    ARROWHEAD_LENGTH + REFUSAL_MARK_GAP + REFUSAL_MARK + PATCH
}

/// The box the ✕ on `d` covers, its patch of canvas included.
#[must_use]
pub fn mark_box(d: &Drawn) -> Rect {
    let h = REFUSAL_MARK / 2.0;
    let c = d.behind_tip(ARROWHEAD_LENGTH + REFUSAL_MARK_GAP + h);
    let reach = h + PATCH;
    Rect {
        x: c.x - reach,
        y: c.y - reach,
        w: 2.0 * reach,
        h: 2.0 * reach,
    }
}

fn cross(d: &Drawn) -> String {
    let h = REFUSAL_MARK / 2.0;
    let c = d.behind_tip(ARROWHEAD_LENGTH + REFUSAL_MARK_GAP + h);
    let x = format!(
        "M{} {}L{} {}M{} {}L{} {}",
        num(c.x - h),
        num(c.y - h),
        num(c.x + h),
        num(c.y + h),
        num(c.x + h),
        num(c.y - h),
        num(c.x - h),
        num(c.y + h)
    );
    format!(r#"<path class="patch" d="{x}"/><path d="{x}"/>"#)
}

/// Seen from `start` to `end`, fading in and out over `motion.fade`.
fn seen(start: u32, end: u32, period: u32) -> String {
    let f = fade().min(end.saturating_sub(start) / 2);
    let mut t = KeyTrack::new(period);
    t.key(0, "0")
        .key(start, "0")
        .key(start + f, "1")
        .key(end - f, "1")
        .key(end, "0")
        .key(period, "0");
    format!(
        r#"<animate attributeName="opacity" {}/>"#,
        t.attributes("values")
    )
}
