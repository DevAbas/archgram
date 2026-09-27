//! A node's card (DESIGN.md, Components: Node card and Variants).

use crate::geometry::Rect;
use crate::render::icons::{GRID, icon};
use crate::render::svg::{Svg, escape, num};
use crate::spec::{CardStyle, Category, Node, Variant};
use crate::tokens::{
    CARD_HORIZONTAL_BADGE, CARD_HORIZONTAL_ICON, CARD_MULTI_OFFSET, CARD_PADDING, CARD_VERTICAL_BADGE,
    CARD_VERTICAL_ICON, ROUNDED_BADGE, ROUNDED_CARD, STROKE_ICON, TYPOGRAPHY_SUBTITLE, TYPOGRAPHY_TITLE,
};

/// Where a text baseline sits in its line box, as a share of the line's height
/// from its top. A stand-in for the font's own ascent, which M3 reads from the
/// embedded font.
const BASELINE_IN_LINE: f64 = 0.8;

/// The CSS class that gives an icon its category's hue (DESIGN.md, Colors).
pub fn category_class(c: Category) -> &'static str {
    match c {
        Category::Core => "core",
        Category::Ai => "ai",
        Category::Build => "build",
        Category::Client => "client",
    }
}

/// The two lines a card may carry: its title and, when given, its note.
struct Lines<'a> {
    title: &'a str,
    note: Option<&'a str>,
}

impl Lines<'_> {
    fn title_height() -> f64 {
        TYPOGRAPHY_TITLE.size * TYPOGRAPHY_TITLE.line_height
    }

    fn note_height() -> f64 {
        TYPOGRAPHY_SUBTITLE.size * TYPOGRAPHY_SUBTITLE.line_height
    }

    /// The height of the text block.
    fn height(&self) -> f64 {
        Self::title_height()
            + if self.note.is_some() {
                Self::note_height()
            } else {
                0.0
            }
    }

    /// Writes the lines with the block's top at `top`, anchored at `x`.
    fn write(&self, svg: &mut Svg, x: f64, top: f64, anchor: &str) {
        let anchor = if anchor == "start" {
            String::new()
        } else {
            format!(r#" text-anchor="{anchor}""#)
        };
        let title_y = top + BASELINE_IN_LINE * Self::title_height();
        svg.line(&format!(
            r#"<text class="title" x="{}" y="{}"{anchor}>{}</text>"#,
            num(x),
            num(title_y),
            escape(self.title)
        ));
        if let Some(note) = self.note {
            let note_y = top + Self::title_height() + BASELINE_IN_LINE * Self::note_height();
            svg.line(&format!(
                r#"<text class="sub" x="{}" y="{}"{anchor}>{}</text>"#,
                num(x),
                num(note_y),
                escape(note)
            ));
        }
    }
}

/// Draws `node`'s card in `r`.
pub fn card(svg: &mut Svg, node: &Node, r: Rect, style: CardStyle) {
    svg.open(&format!(r#"<g data-node="{}">"#, escape(&node.id)));
    let class = match node.variant {
        Variant::External => "card external",
        Variant::Single | Variant::Multi => "card",
    };
    if node.variant == Variant::Multi {
        // Two copies of the outline behind, stepping up and to the right.
        for step in [2.0, 1.0] {
            let d = step * CARD_MULTI_OFFSET;
            svg.line(&format!(
                r#"<rect class="card" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
                num(r.x + d),
                num(r.y - d),
                num(r.w),
                num(r.h),
                num(ROUNDED_CARD)
            ));
        }
    }
    svg.line(&format!(
        r#"<rect class="{class}" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
        num(r.x),
        num(r.y),
        num(r.w),
        num(r.h),
        num(ROUNDED_CARD)
    ));

    let lines = Lines {
        title: &node.label,
        note: node.note.as_deref(),
    };
    let hue = category_class(node.kind.category());
    match style {
        CardStyle::Horizontal => {
            let badge = Rect {
                x: r.x + CARD_PADDING,
                y: r.centre_y() - CARD_HORIZONTAL_BADGE / 2.0,
                w: CARD_HORIZONTAL_BADGE,
                h: CARD_HORIZONTAL_BADGE,
            };
            badge_with_icon(svg, badge, CARD_HORIZONTAL_ICON, node, hue);
            let text_x = badge.right() + CARD_PADDING;
            lines.write(svg, text_x, r.centre_y() - lines.height() / 2.0, "start");
        }
        CardStyle::Vertical => {
            let content = CARD_VERTICAL_BADGE + CARD_PADDING + lines.height();
            let top = r.y + (r.h - content) / 2.0;
            let badge = Rect {
                x: r.centre_x() - CARD_VERTICAL_BADGE / 2.0,
                y: top,
                w: CARD_VERTICAL_BADGE,
                h: CARD_VERTICAL_BADGE,
            };
            badge_with_icon(svg, badge, CARD_VERTICAL_ICON, node, hue);
            lines.write(svg, r.centre_x(), badge.bottom() + CARD_PADDING, "middle");
        }
    }
    svg.close("</g>");
}

/// The neutral badge and, centred in it, the kind's icon at `icon_size`, its
/// lines kept at the token's width whatever the scale.
fn badge_with_icon(svg: &mut Svg, badge: Rect, icon_size: f64, node: &Node, hue: &str) {
    svg.line(&format!(
        r#"<rect class="badge" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
        num(badge.x),
        num(badge.y),
        num(badge.w),
        num(badge.h),
        num(ROUNDED_BADGE)
    ));
    let scale = icon_size / GRID;
    let (ix, iy) = (
        badge.centre_x() - icon_size / 2.0,
        badge.centre_y() - icon_size / 2.0,
    );
    let transform = if (scale - 1.0).abs() < f64::EPSILON {
        format!("translate({} {})", num(ix), num(iy))
    } else {
        format!("translate({} {}) scale({})", num(ix), num(iy), num(scale))
    };
    svg.line(&format!(
        r#"<g class="icon {hue}" transform="{transform}" stroke-width="{}">{}</g>"#,
        num(STROKE_ICON / scale),
        icon(node.kind)
    ));
}
