//! The credit (DESIGN.md, Components: Credit): "by", archgram's mark and
//! "archgram", quiet, under everything at the drawing's right edge.

use crate::font::baseline_in_line;
use crate::geometry::Rect;
use crate::layout::legend::{CREDIT_BY, CREDIT_NAME, credit_parts};
use crate::render::scene::{Anchor, GroupOf, Item, Place};
use crate::tokens::TYPOGRAPHY_LEGEND;

/// The mark's grid, as in docs/images/logo.svg: a 64 square.
const GRID: f64 = 64.0;

/// The mark's badge: a square of `GRID` with corners of 15, as a path.
const BADGE: &str =
    "M15 0H49A15 15 0 0 1 64 15V49A15 15 0 0 1 49 64H15A15 15 0 0 1 0 49V15A15 15 0 0 1 15 0Z";

/// The mark's bend and chevron, moved as the logo moves them (by 1 and
/// -2.5, to sit optically centred in the badge).
const GLYPH: &str = "M16 19.5H31A9 9 0 0 1 40 28.5V43.5M32 36.5L40 44.5L48 36.5";

/// The credit in its box `at`, one group so its opacity fades it whole.
#[must_use]
pub fn credit(at: Rect) -> Item {
    let [by, space, mark, _] = credit_parts();
    let baseline = at.y + baseline_in_line(&TYPOGRAPHY_LEGEND);
    let word = |x: f64, text: &str| Item::Text {
        class: "credit".into(),
        x,
        y: baseline,
        anchor: Anchor::Start,
        text: text.to_owned(),
    };
    let place = Place {
        x: at.x + by + space,
        y: at.y + (at.h - mark) / 2.0,
        scale: mark / GRID,
    };
    let path = |class: &str, d: &str| Item::Path {
        id: None,
        class: class.to_owned(),
        place: Some(place),
        d: d.to_owned(),
        arrowhead: false,
    };
    Item::Group {
        of: GroupOf::Class("credit-line"),
        items: vec![
            word(at.x, CREDIT_BY),
            path("credit-mark", BADGE),
            path("credit-glyph", GLYPH),
            word(at.x + by + space + mark + space, CREDIT_NAME),
        ],
    }
}
