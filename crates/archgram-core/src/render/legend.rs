//! The legend (DESIGN.md, Components: Legend): each entry's swatch, then
//! its text in `typography.legend` and `color.text-muted`.

use crate::font::baseline_in_line;
use crate::layout::legend::{Entry, Swatch, stack_step};
use crate::render::card::category_class;
use crate::render::svg::{Svg, escape, num};
use crate::tokens::TYPOGRAPHY_LEGEND;

/// Draws the legend's entries.
pub fn legend(svg: &mut Svg, entries: &[Entry]) {
    if entries.is_empty() {
        return;
    }
    svg.open(r#"<g class="legend">"#);
    for e in entries {
        let b = e.swatch_box;
        let rect = |class: &str, x: f64, y: f64, w: f64, h: f64| {
            format!(
                r#"<rect class="{class}" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
                num(x),
                num(y),
                num(w),
                num(h),
                num(h / 4.0)
            )
        };
        match e.swatch {
            Swatch::Category(c) => {
                svg.line(&rect(
                    &format!("swatch {}", category_class(c)),
                    b.x,
                    b.y,
                    b.w,
                    b.h,
                ));
            }
            Swatch::External => svg.line(&rect("card external", b.x, b.y, b.w, b.h)),
            Swatch::Multi => {
                let d = stack_step();
                let (w, h) = (b.w - 2.0 * d, b.h - 2.0 * d);
                for step in [2.0, 1.0, 0.0] {
                    svg.line(&rect("card", b.x + step * d, b.y + (2.0 - step) * d, w, h));
                }
            }
        }
        svg.line(&format!(
            r#"<text class="legend-text" x="{}" y="{}">{}</text>"#,
            num(e.text_box.x),
            num(e.text_box.y + baseline_in_line(&TYPOGRAPHY_LEGEND)),
            escape(e.text)
        ));
    }
    svg.close("</g>");
}
