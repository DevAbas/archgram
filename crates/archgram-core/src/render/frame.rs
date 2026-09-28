//! Frames (DESIGN.md, Components: Frame, Frame label): a dashed, rounded
//! box in `color.frame` with no fill, and its name in capitals at its top
//! left, on a patch of canvas so an edge passing under it does not cut it.

use crate::font::baseline_in_line;
use crate::geometry::Rect;
use crate::render::scene::{Anchor, Item};
use crate::tokens::{ROUNDED_FRAME, TYPOGRAPHY_FRAME_LABEL};

/// A frame's box.
pub fn frame(r: Rect) -> Item {
    Item::Rect {
        class: "frame".into(),
        x: r.x,
        y: r.y,
        w: r.w,
        h: r.h,
        rx: Some(ROUNDED_FRAME),
    }
}

/// A frame's name in its box `at`, set in capitals as it was measured, on
/// its patch of canvas.
pub fn name(label: &str, at: Rect) -> [Item; 2] {
    let pad = TYPOGRAPHY_FRAME_LABEL.size / 2.0;
    [
        Item::Rect {
            class: "label-patch".into(),
            x: at.x - pad,
            y: at.y,
            w: at.w + 2.0 * pad,
            h: at.h,
            rx: None,
        },
        Item::Text {
            class: "frame-label".into(),
            x: at.x,
            y: at.y + baseline_in_line(&TYPOGRAPHY_FRAME_LABEL),
            anchor: Anchor::Start,
            text: label.to_uppercase(),
        },
    ]
}
