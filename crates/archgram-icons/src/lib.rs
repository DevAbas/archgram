//! Technology logos for archgram (ARCHITECTURE.md, Code map), from a pinned
//! release of Simple Icons: `data/icons.tsv`, one logo per line (slug,
//! title, the path on a 24 by 24 grid), sorted by slug and written by
//! `cargo xtask icons <tag>`. `data/RELEASE` names the release and commit;
//! `data/provenance.tsv` each logo's source and brand guidelines. Logos
//! that carry a licence of their own other than CC0 are left out.
//!
//! The logos are brands' trademarks. archgram draws them in one neutral
//! colour to name a technology, as Simple Icons intends; whoever publishes
//! a diagram follows each brand's guidelines (`provenance.tsv`).

use archgram_core::logos::Logos;

static DATA: &str = include_str!("../data/icons.tsv");

/// The Simple Icons release the data comes from.
pub const RELEASE: &str = include_str!("../data/RELEASE");

/// One logo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Icon {
    pub slug: &'static str,
    pub title: &'static str,
    pub path: &'static str,
}

/// Every logo, sorted by slug.
#[derive(Debug, Clone)]
pub struct Icons {
    icons: Vec<Icon>,
}

impl Icons {
    /// Reads the logos from the data this crate carries.
    ///
    /// # Panics
    ///
    /// When the data is malformed; `cargo xtask icons` writes it and the
    /// tests read all of it.
    #[must_use]
    pub fn load() -> Self {
        let icons = DATA
            .lines()
            .map(|line| {
                let mut cells = line.splitn(3, '\t');
                let (slug, title, path) = (cells.next(), cells.next(), cells.next());
                Icon {
                    slug: slug.expect("a slug"),
                    title: title.expect("a title"),
                    path: path.expect("a path"),
                }
            })
            .collect();
        Self { icons }
    }

    /// The logo `slug` names.
    #[must_use]
    pub fn get(&self, slug: &str) -> Option<&Icon> {
        self.icons
            .binary_search_by(|i| i.slug.cmp(slug))
            .ok()
            .map(|at| &self.icons[at])
    }

    /// How many logos there are.
    #[must_use]
    pub fn len(&self) -> usize {
        self.icons.len()
    }

    /// Whether there are none.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.icons.is_empty()
    }
}

impl Logos for Icons {
    fn path(&self, slug: &str) -> Option<&str> {
        self.get(slug).map(|i| i.path)
    }

    fn slugs(&self) -> Vec<&str> {
        self.icons.iter().map(|i| i.slug).collect()
    }
}
