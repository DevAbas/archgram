//! The design tokens, compiled from `design-system/tokens/` by `build.rs`.
//! Nothing here is written by hand.

#![allow(clippy::unreadable_literal, clippy::doc_markdown, missing_docs)]

include!(concat!(env!("OUT_DIR"), "/tokens.rs"));

/// The theme for a palette in light or dark, when the palette exists.
#[must_use]
pub fn theme(palette: &str, theme: &str) -> Option<&'static Theme> {
    THEMES.iter().find(|t| t.palette == palette && t.theme == theme)
}
