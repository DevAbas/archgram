//! A node's card (DESIGN.md, Components: Node card and Variants).

use crate::font::baseline_in_line;
use crate::geometry::Rect;
use crate::logos::Logos;
use crate::render::icons::{GRID, icon};
use crate::render::svg::{Svg, escape, num};
use crate::spec::{CardStyle, Category, LogoPlace, Node, Variant};
use crate::tokens::{
    CARD_HORIZONTAL_BADGE, CARD_HORIZONTAL_ICON, CARD_LOGO_CHIP, CARD_LOGO_CHIP_RING, CARD_LOGO_CORNER,
    CARD_LOGO_INLINE, CARD_LOGO_INLINE_GAP, CARD_MULTI_OFFSET, CARD_PADDING, CARD_VERTICAL_BADGE,
    CARD_VERTICAL_ICON, ROUNDED_BADGE, ROUNDED_CARD, SIGNAL_LIT, STROKE_CARD, STROKE_ICON,
    TYPOGRAPHY_SUBTITLE, TYPOGRAPHY_TITLE,
};

/// The CSS class that gives an icon its category's hue (DESIGN.md, Colors).
pub fn category_class(c: Category) -> &'static str {
    match c {
        Category::Core => "core",
        Category::Ai => "ai",
        Category::Build => "build",
        Category::Client => "client",
    }
}

/// The two lines a card may carry: its title and, when given, its note,
/// with the logo that leads the note when logos go inline.
struct Lines<'a> {
    title: &'a str,
    note: Option<&'a str>,
    inline: Option<&'a str>,
    brand: Option<&'a Brand<'a>>,
}

/// A card's lighting while a flow's signal is at it: its opacity animation,
/// and the class that gives its logo the brand's colour when it has one.
#[derive(Debug)]
pub struct Brand<'a> {
    pub animation: &'a str,
    pub class: Option<String>,
}

/// A logo path; on a card a signal lights, a copy in the brand's colour
/// over it that shows only while the card is lit.
fn logo_path(svg: &mut Svg, class: &str, transform: &str, d: &str, brand: Option<&Brand>) {
    svg.line(&format!(
        r#"<path class="{class}" transform="{transform}" d="{}"/>"#,
        escape(d)
    ));
    if let Some(Brand {
        animation,
        class: Some(colour),
    }) = brand
    {
        svg.line(&format!(
            r#"<path class="brand {colour}" transform="{transform}" d="{}" opacity="0">{animation}</path>"#,
            escape(d)
        ));
    }
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

    /// Writes the lines with the block's top at `top`, anchored at `x`. An
    /// inline logo leads the note line, the pair anchored as one.
    fn write(&self, svg: &mut Svg, x: f64, top: f64, anchor: &str) {
        let anchored = if anchor == "start" {
            String::new()
        } else {
            format!(r#" text-anchor="{anchor}""#)
        };
        let title_y = top + baseline_in_line(&TYPOGRAPHY_TITLE);
        svg.line(&format!(
            r#"<text class="title" x="{}" y="{}"{anchored}>{}</text>"#,
            num(x),
            num(title_y),
            escape(self.title)
        ));
        let Some(note) = self.note else { return };
        let note_top = top + Self::title_height();
        let note_y = note_top + baseline_in_line(&TYPOGRAPHY_SUBTITLE);
        let Some(logo) = self.inline else {
            svg.line(&format!(
                r#"<text class="sub" x="{}" y="{}"{anchored}>{}</text>"#,
                num(x),
                num(note_y),
                escape(note)
            ));
            return;
        };
        let lead = CARD_LOGO_INLINE + CARD_LOGO_INLINE_GAP;
        let whole = lead + crate::font::text_width(note, &TYPOGRAPHY_SUBTITLE);
        let start = if anchor == "start" { x } else { x - whole / 2.0 };
        let transform = format!(
            "translate({} {}) scale({})",
            num(start),
            num(note_top + (Self::note_height() - CARD_LOGO_INLINE) / 2.0),
            num(CARD_LOGO_INLINE / GRID)
        );
        logo_path(svg, "logo", &transform, logo, self.brand);
        svg.line(&format!(
            r#"<text class="sub" x="{}" y="{}">{}</text>"#,
            num(start + lead),
            num(note_y),
            escape(note)
        ));
    }
}

/// Draws `node`'s card in `r`, with its technology's logo from `logos`, in
/// the diagram's `place` for logos; lit while a signal is at it when
/// `brand` says how.
pub fn card(
    svg: &mut Svg,
    node: &Node,
    r: Rect,
    (style, place): (CardStyle, LogoPlace),
    logos: &dyn Logos,
    brand: Option<&Brand>,
) {
    let path = node.tech.as_deref().and_then(|t| logos.path(t));
    let in_badge = if place == LogoPlace::Icon { path } else { None };
    let logo = path
        .filter(|_| matches!(place, LogoPlace::Corner | LogoPlace::Chip))
        .map(|p| (p, place));
    svg.open(&format!(r#"<g data-node="{}">"#, escape(&node.id)));
    let class = match node.variant {
        Variant::External => "card external",
        Variant::Single | Variant::Multi => "card",
    };
    // `r` is the footprint. For several instances the front card sits at
    // its lower left and two copies of the outline step up and to the right
    // behind it; the rest of the card is drawn on the front one.
    let r = if node.variant == Variant::Multi {
        let front = Rect {
            x: r.x,
            y: r.y + 2.0 * CARD_MULTI_OFFSET,
            w: r.w - 2.0 * CARD_MULTI_OFFSET,
            h: r.h - 2.0 * CARD_MULTI_OFFSET,
        };
        for step in [2.0, 1.0] {
            let d = step * CARD_MULTI_OFFSET;
            svg.line(&format!(
                r#"<rect class="card" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
                num(front.x + d),
                num(front.y - d),
                num(front.w),
                num(front.h),
                num(ROUNDED_CARD)
            ));
        }
        front
    } else {
        r
    };
    svg.line(&format!(
        r#"<rect class="{class}" x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
        num(r.x),
        num(r.y),
        num(r.w),
        num(r.h),
        num(ROUNDED_CARD)
    ));
    let hue = category_class(node.kind.category());
    if let Some(b) = brand {
        // Its border in the card's hue, `signal.lit` wide on whole pixels
        // just outside the card's own edge, over a tint of that hue.
        let out = SIGNAL_LIT / 2.0 - STROKE_CARD / 2.0;
        svg.line(&format!(
            r#"<rect class="lit {hue}" x="{}" y="{}" width="{}" height="{}" rx="{}" opacity="0">{}</rect>"#,
            num(r.x - out),
            num(r.y - out),
            num(r.w + 2.0 * out),
            num(r.h + 2.0 * out),
            num(ROUNDED_CARD + out),
            b.animation
        ));
    }

    let second = crate::measure::note_line(node, place, logos);
    let lines = Lines {
        title: &node.label,
        note: second.text,
        inline: second.logo,
        brand,
    };
    match style {
        CardStyle::Horizontal => {
            let badge = Rect {
                x: r.x + CARD_PADDING,
                y: r.centre_y() - CARD_HORIZONTAL_BADGE / 2.0,
                w: CARD_HORIZONTAL_BADGE,
                h: CARD_HORIZONTAL_BADGE,
            };
            badge_with_icon(svg, (badge, CARD_HORIZONTAL_ICON), node, hue, in_badge, brand);
            let text_x = r.x + crate::measure::horizontal_text_inset();
            lines.write(svg, text_x, r.centre_y() - lines.height() / 2.0, "start");
            draw_logo(svg, logo, r, badge, brand);
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
            badge_with_icon(svg, (badge, CARD_VERTICAL_ICON), node, hue, in_badge, brand);
            lines.write(svg, r.centre_x(), badge.bottom() + CARD_PADDING, "middle");
            draw_logo(svg, logo, r, badge, brand);
        }
    }
    svg.close("</g>");
}

/// The neutral badge and, centred in it, the kind's icon at `icon_size`, its
/// lines kept at the token's width whatever the scale.
/// The logo on card `r`: in its top-right corner, inside its padding, at
/// `card.logo-corner`; or in a round chip of `card.logo-chip-ring`, filled
/// and edged like a card, centred on the badge's lower-right corner, at
/// `card.logo-chip`. Always in `color.text-muted` (DESIGN.md, Components:
/// Technology logo).
fn draw_logo(svg: &mut Svg, logo: Option<(&str, LogoPlace)>, r: Rect, badge: Rect, brand: Option<&Brand>) {
    let Some((path, place)) = logo else { return };
    let (size, x, y) = match place {
        LogoPlace::Inline | LogoPlace::Icon => return,
        LogoPlace::Corner => (
            CARD_LOGO_CORNER,
            r.right() - CARD_PADDING - CARD_LOGO_CORNER,
            r.y + CARD_PADDING,
        ),
        LogoPlace::Chip => {
            svg.line(&format!(
                r#"<circle class="logo-chip" cx="{}" cy="{}" r="{}"/>"#,
                num(badge.right()),
                num(badge.bottom()),
                num(CARD_LOGO_CHIP_RING / 2.0)
            ));
            (
                CARD_LOGO_CHIP,
                badge.right() - CARD_LOGO_CHIP / 2.0,
                badge.bottom() - CARD_LOGO_CHIP / 2.0,
            )
        }
    };
    let transform = format!("translate({} {}) scale({})", num(x), num(y), num(size / GRID));
    logo_path(svg, "logo", &transform, path, brand);
}

/// The badge and, in it, the kind's icon; or the technology's logo in the
/// category's hue when logos go in place of the icon.
fn badge_with_icon(
    svg: &mut Svg,
    (badge, icon_size): (Rect, f64),
    node: &Node,
    hue: &str,
    logo: Option<&str>,
    brand: Option<&Brand>,
) {
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
    if let Some(path) = logo {
        let transform = format!("translate({} {}) scale({})", num(ix), num(iy), num(scale));
        logo_path(svg, &format!("logo-icon {hue}"), &transform, path, brand);
        return;
    }
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
