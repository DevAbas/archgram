//! Box sizes (ARCHITECTURE.md, Measure). A card's height comes from its
//! style's template in the tokens; its width grows with its measured text,
//! never below the template's minimum (DESIGN.md, Layout).

use crate::font::text_width;
use crate::geometry::Size;
use crate::spec::{CardStyle, LogoPlace, Node, Spec, Variant};
use crate::tokens::{
    CARD_HORIZONTAL_BADGE, CARD_HORIZONTAL_HEIGHT, CARD_HORIZONTAL_MIN_WIDTH, CARD_LOGO_CORNER,
    CARD_MULTI_OFFSET, CARD_PADDING, CARD_VERTICAL_HEIGHT, CARD_VERTICAL_MIN_WIDTH, TYPOGRAPHY_FRAME_LABEL,
    TYPOGRAPHY_SUBTITLE, TYPOGRAPHY_TITLE,
};

/// The width of a card's widest line of text.
fn text_width_of(node: &Node) -> f64 {
    let title_width = text_width(&node.label, &TYPOGRAPHY_TITLE);
    let note_width = node
        .note
        .as_deref()
        .map_or(0.0, |n| text_width(n, &TYPOGRAPHY_SUBTITLE));
    title_width.max(note_width)
}

/// Where a horizontal card's text starts, from the card's left edge: padding,
/// the badge, padding.
#[must_use]
pub fn horizontal_text_inset() -> f64 {
    CARD_PADDING + CARD_HORIZONTAL_BADGE + CARD_PADDING
}

/// The size of one node's footprint: its card, and for several instances
/// the two copies stepping up and to the right behind it, `card.multi-offset`
/// apart (DESIGN.md, Components: Variants).
#[must_use]
pub fn card_size(node: &Node, style: CardStyle, logo: LogoPlace) -> Size {
    let card = front_card_size(node, style, logo);
    if node.variant == Variant::Multi {
        Size {
            w: card.w + 2.0 * CARD_MULTI_OFFSET,
            h: card.h + 2.0 * CARD_MULTI_OFFSET,
        }
    } else {
        card
    }
}

/// The size of the card a reader sees first.
fn front_card_size(node: &Node, style: CardStyle, logo: LogoPlace) -> Size {
    let text = text_width_of(node);
    // The text's right side gets twice the padding its left side has, so the
    // card does not look full; with a logo in the corner, room for the logo
    // between two paddings, so it never meets the title.
    let after = if node.tech.is_some() && logo == LogoPlace::Corner {
        CARD_PADDING + CARD_LOGO_CORNER + CARD_PADDING
    } else {
        2.0 * CARD_PADDING
    };
    match style {
        CardStyle::Horizontal => Size {
            w: CARD_HORIZONTAL_MIN_WIDTH
                .max(horizontal_text_inset() + text + after)
                .ceil(),
            h: CARD_HORIZONTAL_HEIGHT,
        },
        CardStyle::Vertical => Size {
            w: CARD_VERTICAL_MIN_WIDTH.max(text + 2.0 * CARD_PADDING).ceil(),
            h: CARD_VERTICAL_HEIGHT,
        },
    }
}

/// The patch an edge's label sits on: its text on one line, with half a
/// card's padding on either side (DESIGN.md, Components: Connector).
#[must_use]
pub fn label_size(label: &str) -> Size {
    Size {
        w: text_width(label, &TYPOGRAPHY_SUBTITLE) + CARD_PADDING,
        h: TYPOGRAPHY_SUBTITLE.size * TYPOGRAPHY_SUBTITLE.line_height,
    }
}

/// Every piece of text the drawing shows, with the weight it is set in, in
/// drawing order: node titles and notes, edge labels, frame names in the
/// capitals they are drawn in, then the legend's entries. The font subset
/// and the check for characters the font lacks both read this list, so
/// neither can miss a text the other covers.
#[must_use]
pub fn text_runs(spec: &Spec) -> Vec<(u16, std::borrow::Cow<'_, str>)> {
    let mut runs = Vec::new();
    for node in &spec.nodes {
        runs.push((TYPOGRAPHY_TITLE.weight, node.label.as_str().into()));
        if let Some(note) = &node.note {
            runs.push((TYPOGRAPHY_SUBTITLE.weight, note.as_str().into()));
        }
    }
    for edge in &spec.edges {
        if let Some(label) = &edge.label {
            runs.push((TYPOGRAPHY_SUBTITLE.weight, label.as_str().into()));
        }
    }
    for frame in &spec.frames {
        runs.push((TYPOGRAPHY_FRAME_LABEL.weight, frame.label.to_uppercase().into()));
    }
    for (_, text) in crate::layout::legend::entries(spec) {
        runs.push((crate::tokens::TYPOGRAPHY_LEGEND.weight, text.into()));
    }
    runs
}

/// The size of every node's card, in spec order.
#[must_use]
pub fn card_sizes(spec: &Spec) -> Vec<Size> {
    spec.nodes
        .iter()
        .map(|n| card_size(n, spec.card, spec.logo))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Kind, Variant};

    fn node(label: &str, note: Option<&str>) -> Node {
        Node {
            id: "n".into(),
            kind: Kind::Service,
            label: label.into(),
            note: note.map(Into::into),
            tech: None,
            variant: Variant::Single,
            frame: None,
        }
    }

    #[test]
    #[allow(clippy::float_cmp)] // both sides are the same token, unchanged
    fn short_titles_keep_the_minimum_width() {
        assert_eq!(
            card_size(&node("API", None), CardStyle::Horizontal, LogoPlace::Corner).w,
            CARD_HORIZONTAL_MIN_WIDTH
        );
        assert_eq!(
            card_size(&node("API", None), CardStyle::Vertical, LogoPlace::Corner).w,
            CARD_VERTICAL_MIN_WIDTH
        );
    }

    #[test]
    fn long_titles_widen_the_card_to_fit() {
        let label = "count_candidates_by_seniority_and_city";
        let size = card_size(&node(label, None), CardStyle::Horizontal, LogoPlace::Corner);
        assert!(size.w > CARD_HORIZONTAL_MIN_WIDTH);
        assert!(size.w >= horizontal_text_inset() + text_width(label, &TYPOGRAPHY_TITLE) + CARD_PADDING);
    }

    #[test]
    fn a_long_note_widens_the_card_too() {
        let short = card_size(&node("API", Some("x")), CardStyle::Horizontal, LogoPlace::Corner).w;
        let long = card_size(
            &node(
                "API",
                Some("three replicas behind the load balancer in eu-west-1"),
            ),
            CardStyle::Horizontal,
            LogoPlace::Corner,
        )
        .w;
        assert!(long > short);
    }

    #[test]
    fn a_corner_logo_keeps_its_room_beside_the_title() {
        let long = "Billing service with a long name";
        let plain = card_size(&node(long, None), CardStyle::Horizontal, LogoPlace::Corner).w;
        let mut with_logo = node(long, None);
        with_logo.tech = Some("postgresql".into());
        let corner = card_size(&with_logo, CardStyle::Horizontal, LogoPlace::Corner).w;
        let chip = card_size(&with_logo, CardStyle::Horizontal, LogoPlace::Chip).w;
        // Padding, logo, padding instead of two paddings: the logo's width more.
        assert!(
            (corner - plain - CARD_LOGO_CORNER).abs() <= 1.0,
            "{plain} {corner}"
        );
        assert!((chip - plain).abs() < 1e-9);
    }
}
