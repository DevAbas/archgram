//! Technology logos (PRD, section 7; ARCHITECTURE.md, Render). The core
//! carries none: it draws the logos it is given, so the binary that ships
//! it decides which set it needs. The native CLI passes Simple Icons from
//! `archgram-icons`; a web build can pass only the few a diagram names.

use std::collections::BTreeMap;

/// A set of logos, each a single SVG path on a 24 by 24 grid, found by its
/// Simple Icons slug.
pub trait Logos {
    /// The path of the logo `slug` names, if the set has it.
    fn path(&self, slug: &str) -> Option<&str>;
    /// Every slug in the set, for suggesting one when a name is wrong.
    fn slugs(&self) -> Vec<&str>;

    /// The technology's name, shown beside an inline logo on a card without
    /// a note.
    fn title(&self, _slug: &str) -> Option<&str> {
        None
    }

    /// The brand's colour, six hex digits, shown while a flow's signal is at
    /// the card (DESIGN.md, Components: Technology logo).
    fn colour(&self, _slug: &str) -> Option<&str> {
        None
    }
}

/// No logos at all: `tech` is then neither checked nor drawn.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLogos;

impl Logos for NoLogos {
    fn path(&self, _: &str) -> Option<&str> {
        None
    }

    fn slugs(&self) -> Vec<&str> {
        Vec::new()
    }
}

impl Logos for BTreeMap<String, String> {
    fn path(&self, slug: &str) -> Option<&str> {
        self.get(slug).map(String::as_str)
    }

    fn slugs(&self) -> Vec<&str> {
        self.keys().map(String::as_str).collect()
    }
}
