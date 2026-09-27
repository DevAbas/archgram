//! Box sizes (ARCHITECTURE.md, Measure). A card's size comes from its
//! style's template in the tokens; from M3 on, a horizontal card also grows
//! with its measured title.

use crate::geometry::Size;
use crate::spec::{CardStyle, Spec};
use crate::tokens::{
    CARD_HORIZONTAL_HEIGHT, CARD_HORIZONTAL_MIN_WIDTH, CARD_VERTICAL_HEIGHT, CARD_VERTICAL_WIDTH,
};

/// The size of every node's card, in spec order.
#[must_use]
pub fn card_sizes(spec: &Spec) -> Vec<Size> {
    let size = match spec.card {
        CardStyle::Horizontal => Size {
            w: CARD_HORIZONTAL_MIN_WIDTH,
            h: CARD_HORIZONTAL_HEIGHT,
        },
        CardStyle::Vertical => Size {
            w: CARD_VERTICAL_WIDTH,
            h: CARD_VERTICAL_HEIGHT,
        },
    };
    vec![size; spec.nodes.len()]
}
