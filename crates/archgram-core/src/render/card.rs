//! A node's card (DESIGN.md, Components: Node card and Variants).

use crate::font::baseline_in_line;
use crate::geometry::Rect;
use crate::logos::Logos;
use crate::render::icons::{GRID, icon};
use crate::render::scene::{Anchor, GroupOf, Item, Place};
use crate::render::svg::num;
use crate::spec::{CardStyle, Category, LogoPlace, Node, Variant};
use crate::tokens::{
    CARD_HORIZONTAL_BADGE, CARD_HORIZONTAL_ICON, CARD_LOGO_CHIP, CARD_LOGO_CHIP_RING,
    CARD_LOGO_CORNER, CARD_LOGO_INLINE, CARD_LOGO_INLINE_GAP, CARD_MULTI_OFFSET, CARD_PADDING,
    CARD_VERTICAL_BADGE, CARD_VERTICAL_ICON, ROUNDED_BADGE, ROUNDED_CARD, SIGNAL_LIT, STROKE_CARD,
    STROKE_ICON, TYPOGRAPHY_SUBTITLE, TYPOGRAPHY_TITLE,
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

/// A card's lighting while a flow's signal is at it, its opacity animation
/// when a flow reaches it, and the class that gives its logo the brand's
/// colour when the brand has one.
#[derive(Debug)]
pub struct Brand<'a> {
    pub animation: Option<&'a str>,
    pub class: Option<String>,
}

/// A logo path, in its brand's colour when `brand` gives one (DESIGN.md,
/// Components: Technology logo).
fn logo_path(out: &mut Vec<Item>, class: &str, place: Place, d: &str, brand: Option<&Brand>) {
    let class = match brand.and_then(|b| b.class.as_deref()) {
        Some(colour) => format!("{class} {colour}"),
        None => class.to_owned(),
    };
    out.push(Item::Path {
        id: None,
        class,
        place: Some(place),
        d: d.to_owned(),
        arrowhead: false,
    });
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
    fn write(&self, out: &mut Vec<Item>, x: f64, top: f64, anchor: Anchor) {
        let text = |class: &str, x: f64, y: f64, anchor: Anchor, text: &str| Item::Text {
            class: class.to_owned(),
            x,
            y,
            anchor,
            text: text.to_owned(),
        };
        out.push(text(
            "title",
            x,
            top + baseline_in_line(&TYPOGRAPHY_TITLE),
            anchor,
            self.title,
        ));
        let Some(note) = self.note else { return };
        let note_top = top + Self::title_height();
        let note_y = note_top + baseline_in_line(&TYPOGRAPHY_SUBTITLE);
        let Some(logo) = self.inline else {
            out.push(text("sub", x, note_y, anchor, note));
            return;
        };
        let lead = CARD_LOGO_INLINE + CARD_LOGO_INLINE_GAP;
        let whole = lead + crate::font::text_width(note, &TYPOGRAPHY_SUBTITLE);
        let start = if anchor == Anchor::Start {
            x
        } else {
            x - whole / 2.0
        };
        let place = Place {
            x: start,
            y: note_top + (Self::note_height() - CARD_LOGO_INLINE) / 2.0,
            scale: CARD_LOGO_INLINE / GRID,
        };
        logo_path(out, "logo", place, logo, self.brand);
        out.push(text("sub", start + lead, note_y, Anchor::Start, note));
    }
}

/// `node`'s card in `r`, with its technology's logo from `logos`, in the
/// diagram's `place` for logos; lit while a signal is at it when `brand`
/// says how.
pub fn card(
    node: &Node,
    r: Rect,
    (style, place): (CardStyle, LogoPlace),
    logos: &dyn Logos,
    brand: Option<&Brand>,
) -> Item {
    let mut out = Vec::new();
    let path = node.tech.as_deref().and_then(|t| logos.path(t));
    let in_badge = if place == LogoPlace::Icon { path } else { None };
    let logo = path
        .filter(|_| matches!(place, LogoPlace::Corner | LogoPlace::Chip))
        .map(|p| (p, place));
    let r = outline(&mut out, node, r);
    let hue = category_class(node.kind.category());
    if let Some(animation) = brand.and_then(|b| b.animation) {
        // Its border in the card's hue, `signal.lit` wide on whole pixels
        // just outside the card's own edge, over a tint of that hue.
        let o = SIGNAL_LIT / 2.0 - STROKE_CARD / 2.0;
        out.push(Item::Motion(format!(
            r#"<rect class="lit {hue}" x="{}" y="{}" width="{}" height="{}" rx="{}" opacity="0">{}</rect>"#,
            num(r.x - o),
            num(r.y - o),
            num(r.w + 2.0 * o),
            num(r.h + 2.0 * o),
            num(ROUNDED_CARD + o),
            animation
        )));
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
            badge_with_icon(
                &mut out,
                (badge, CARD_HORIZONTAL_ICON),
                node,
                hue,
                in_badge,
                brand,
            );
            let text_x = r.x + crate::measure::horizontal_text_inset();
            lines.write(
                &mut out,
                text_x,
                r.centre_y() - lines.height() / 2.0,
                Anchor::Start,
            );
            draw_logo(&mut out, logo, r, badge, brand);
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
            badge_with_icon(
                &mut out,
                (badge, CARD_VERTICAL_ICON),
                node,
                hue,
                in_badge,
                brand,
            );
            lines.write(
                &mut out,
                r.centre_x(),
                badge.bottom() + CARD_PADDING,
                Anchor::Middle,
            );
            draw_logo(&mut out, logo, r, badge, brand);
        }
    }
    Item::Group {
        of: GroupOf::Node(node.id.clone()),
        items: out,
    }
}

/// The card's outline in its footprint `r`, and the box the rest of the
/// card is drawn on. For several instances the front card sits at the
/// footprint's lower left and two copies of the outline step up and to the
/// right behind it.
fn outline(out: &mut Vec<Item>, node: &Node, r: Rect) -> Rect {
    let class = match node.variant {
        Variant::External => "card external",
        Variant::Single | Variant::Multi => "card",
    };
    let r = if node.variant == Variant::Multi {
        let front = Rect {
            x: r.x,
            y: r.y + 2.0 * CARD_MULTI_OFFSET,
            w: r.w - 2.0 * CARD_MULTI_OFFSET,
            h: r.h - 2.0 * CARD_MULTI_OFFSET,
        };
        for step in [2.0, 1.0] {
            let d = step * CARD_MULTI_OFFSET;
            out.push(Item::Rect {
                class: "card".into(),
                x: front.x + d,
                y: front.y - d,
                w: front.w,
                h: front.h,
                rx: Some(ROUNDED_CARD),
            });
        }
        front
    } else {
        r
    };
    out.push(Item::Rect {
        class: class.into(),
        x: r.x,
        y: r.y,
        w: r.w,
        h: r.h,
        rx: Some(ROUNDED_CARD),
    });
    r
}

/// The neutral badge and, centred in it, the kind's icon at `icon_size`, its
/// lines kept at the token's width whatever the scale.
/// The logo on card `r`: in its top-right corner, inside its padding, at
/// `card.logo-corner`; or in a round chip of `card.logo-chip-ring`, filled
/// and edged like a card, centred on the badge's lower-right corner, at
/// `card.logo-chip`. In its brand's colour (DESIGN.md, Components:
/// Technology logo).
fn draw_logo(
    out: &mut Vec<Item>,
    logo: Option<(&str, LogoPlace)>,
    r: Rect,
    badge: Rect,
    brand: Option<&Brand>,
) {
    let Some((path, place)) = logo else { return };
    let (size, x, y) = match place {
        LogoPlace::Inline | LogoPlace::Icon => return,
        LogoPlace::Corner => (
            CARD_LOGO_CORNER,
            r.right() - CARD_PADDING - CARD_LOGO_CORNER,
            r.y + CARD_PADDING,
        ),
        LogoPlace::Chip => {
            out.push(Item::Circle {
                class: "logo-chip".into(),
                cx: badge.right(),
                cy: badge.bottom(),
                r: CARD_LOGO_CHIP_RING / 2.0,
            });
            (
                CARD_LOGO_CHIP,
                badge.right() - CARD_LOGO_CHIP / 2.0,
                badge.bottom() - CARD_LOGO_CHIP / 2.0,
            )
        }
    };
    logo_path(
        out,
        "logo",
        Place {
            x,
            y,
            scale: size / GRID,
        },
        path,
        brand,
    );
}

/// The badge and, in it, the kind's icon; or the technology's logo, in its
/// brand's colour, when logos go in place of the icon.
fn badge_with_icon(
    out: &mut Vec<Item>,
    (badge, icon_size): (Rect, f64),
    node: &Node,
    hue: &str,
    logo: Option<&str>,
    brand: Option<&Brand>,
) {
    out.push(Item::Rect {
        class: "badge".into(),
        x: badge.x,
        y: badge.y,
        w: badge.w,
        h: badge.h,
        rx: Some(ROUNDED_BADGE),
    });
    let scale = icon_size / GRID;
    let (ix, iy) = (
        badge.centre_x() - icon_size / 2.0,
        badge.centre_y() - icon_size / 2.0,
    );
    let place = Place {
        x: ix,
        y: iy,
        scale,
    };
    if let Some(path) = logo {
        logo_path(out, &format!("logo-icon {hue}"), place, path, brand);
        return;
    }
    out.push(Item::Icon {
        class: format!("icon {hue}"),
        place,
        stroke_width: STROKE_ICON / scale,
        shapes: icon(node.kind),
    });
}
