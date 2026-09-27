//! Frames (DESIGN.md, Components: Frame, Frame label): a dashed, rounded
//! box in `color.frame` with no fill, and its name in capitals at its top
//! left, on a patch of canvas so an edge passing under it does not cut it.

use crate::font::baseline_in_line;
use crate::geometry::Rect;
use crate::render::svg::{Svg, escape, num};
use crate::tokens::{ROUNDED_FRAME, TYPOGRAPHY_FRAME_LABEL};

/// Draws a frame's box.
pub fn frame(svg: &mut Svg, r: Rect) {
    svg.line(&format!(
        r#"<rect class="frame" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
        num(r.x),
        num(r.y),
        num(r.w),
        num(r.h),
        num(ROUNDED_FRAME)
    ));
}

/// Draws a frame's name in its box `at`, set in capitals as it was measured.
pub fn name(svg: &mut Svg, label: &str, at: Rect) {
    let pad = TYPOGRAPHY_FRAME_LABEL.size / 2.0;
    svg.line(&format!(
        r#"<rect class="label-patch" x="{}" y="{}" width="{}" height="{}"/>"#,
        num(at.x - pad),
        num(at.y),
        num(at.w + 2.0 * pad),
        num(at.h)
    ));
    svg.line(&format!(
        r#"<text class="frame-label" x="{}" y="{}">{}</text>"#,
        num(at.x),
        num(at.y + baseline_in_line(&TYPOGRAPHY_FRAME_LABEL)),
        escape(&label.to_uppercase())
    ));
}
