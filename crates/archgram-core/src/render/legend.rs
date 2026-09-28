//! The legend (DESIGN.md, Components: Legend): each entry's swatch, then
//! its text in `typography.legend` and `color.text-muted`.

use crate::font::baseline_in_line;
use crate::layout::legend::{Entry, FlowLine, Swatch, stack_step};
use crate::render::card::category_class;
use crate::render::scene::{Anchor, GroupOf, Item};
use crate::tokens::TYPOGRAPHY_LEGEND;

/// The legend's entries, then the flows in words; nothing when it has
/// neither.
pub fn legend(entries: &[Entry], flow_lines: &[FlowLine]) -> Option<Item> {
    if entries.is_empty() && flow_lines.is_empty() {
        return None;
    }
    let mut items = Vec::new();
    let text = |x: f64, top: f64, text: &str| Item::Text {
        class: "legend-text".into(),
        x,
        y: top + baseline_in_line(&TYPOGRAPHY_LEGEND),
        anchor: Anchor::Start,
        text: text.to_owned(),
    };
    for e in entries {
        let b = e.swatch_box;
        let rect = |class: &str, x: f64, y: f64, w: f64, h: f64| Item::Rect {
            class: class.to_owned(),
            x,
            y,
            w,
            h,
            rx: Some(h / 4.0),
        };
        match e.swatch {
            Swatch::Category(c) => {
                items.push(rect(
                    &format!("swatch {}", category_class(c)),
                    b.x,
                    b.y,
                    b.w,
                    b.h,
                ));
            }
            Swatch::External => items.push(rect("card external", b.x, b.y, b.w, b.h)),
            Swatch::Multi => {
                let d = stack_step();
                let (w, h) = (b.w - 2.0 * d, b.h - 2.0 * d);
                for step in [2.0, 1.0, 0.0] {
                    items.push(rect("card", b.x + step * d, b.y + (2.0 - step) * d, w, h));
                }
            }
        }
        items.push(text(e.text_box.x, e.text_box.y, e.text));
    }
    for l in flow_lines {
        items.push(text(l.text_box.x, l.text_box.y, &l.text));
    }
    Some(Item::Group {
        of: GroupOf::Class("legend"),
        items,
    })
}
