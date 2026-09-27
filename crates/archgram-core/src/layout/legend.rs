//! The legend (DESIGN.md, Components: Legend): generated from what the
//! diagram uses, one entry per category present in a fixed order, then one
//! per variant present. It is drawn when the spec asks for it and it says
//! something: two categories or more, or a variant beyond the single one.
//! Its entries run left to right under the diagram, `spacing.legend` below
//! it, and wrap into rows no wider than the diagram.

use crate::font::text_width;
use crate::geometry::{Rect, Size};
use crate::spec::{Category, Spec, Still, Variant};
use crate::tokens::{
    LEGEND_SWATCH, LEGEND_SWATCH_GAP, SPACING_LEGEND, SPACING_LEGEND_ENTRY, SPACING_LEGEND_ROW,
    TYPOGRAPHY_LEGEND,
};

/// What an entry's swatch shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Swatch {
    /// A category: a small square outlined in its icon hue.
    Category(Category),
    /// Several instances: a small card with two copies behind it.
    Multi,
    /// A system we do not own: a small dashed card.
    External,
}

/// One entry: its swatch and its text, each in its box.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub swatch: Swatch,
    pub swatch_box: Rect,
    pub text: &'static str,
    pub text_box: Rect,
}

/// A flow in words under the legend: its name, then its steps.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowLine {
    pub text: String,
    pub text_box: Rect,
}

/// The flows in words, when the still image lists them.
#[must_use]
pub fn flow_texts(spec: &Spec) -> Vec<String> {
    if spec.still != Still::Legend {
        return Vec::new();
    }
    spec.flows
        .iter()
        .map(|f| format!("{}: {}", f.name.trim(), spec.flow_words(f)))
        .collect()
}

/// The categories in the order the legend lists them, with their names.
const CATEGORIES: [(Category, &str); 4] = [
    (Category::Core, "Core"),
    (Category::Ai, "AI and LLM"),
    (Category::Build, "Build and tooling"),
    (Category::Client, "Clients"),
];

/// The entries a spec's legend has, in order; none when it draws no legend.
#[must_use]
pub fn entries(spec: &Spec) -> Vec<(Swatch, &'static str)> {
    let used = |c: Category| spec.nodes.iter().any(|n| n.kind.category() == c);
    let has = |v: Variant| spec.nodes.iter().any(|n| n.variant == v);
    let mut out: Vec<(Swatch, &'static str)> = CATEGORIES
        .iter()
        .filter(|(c, _)| used(*c))
        .map(|&(c, name)| (Swatch::Category(c), name))
        .collect();
    let categories = out.len();
    if has(Variant::Multi) {
        out.push((Swatch::Multi, "Several instances"));
    }
    if has(Variant::External) {
        out.push((Swatch::External, "External"));
    }
    if !spec.legend || (categories < 2 && out.len() == categories) {
        return Vec::new();
    }
    out
}

/// How much a small card's copies step up and to the right, a sixth of the
/// swatch, as `card.multi-offset` is to a card.
#[must_use]
pub fn stack_step() -> f64 {
    LEGEND_SWATCH / 6.0
}

/// A swatch's size: a square for a category; a card half as wide again as
/// it is tall for a variant, with its copies' reach for several instances.
fn swatch_size(s: Swatch) -> Size {
    match s {
        Swatch::Category(_) => Size {
            w: LEGEND_SWATCH,
            h: LEGEND_SWATCH,
        },
        Swatch::External => Size {
            w: 1.5 * LEGEND_SWATCH,
            h: LEGEND_SWATCH,
        },
        Swatch::Multi => Size {
            w: 1.5 * LEGEND_SWATCH + 2.0 * stack_step(),
            h: LEGEND_SWATCH + 2.0 * stack_step(),
        },
    }
}

/// Lays the legend out under a drawing of `size`, left-aligned, in rows no
/// wider than it (or than the widest entry), then the flows in words, one a
/// line. Returns the entries, the flow lines and the drawing's new size.
#[must_use]
pub fn place(spec: &Spec, size: Size) -> (Vec<Entry>, Vec<FlowLine>, Size) {
    let (entries, size, placed) = place_entries(spec, size);
    let texts = flow_texts(spec);
    if texts.is_empty() {
        return (entries, Vec::new(), size);
    }
    let line = TYPOGRAPHY_LEGEND.size * TYPOGRAPHY_LEGEND.line_height;
    let mut y = size.h
        + if placed {
            SPACING_LEGEND_ROW
        } else {
            SPACING_LEGEND
        };
    let mut lines = Vec::with_capacity(texts.len());
    for text in texts {
        let w = text_width(&text, &TYPOGRAPHY_LEGEND);
        lines.push(FlowLine {
            text,
            text_box: Rect {
                x: 0.0,
                y,
                w,
                h: line,
            },
        });
        y += line + SPACING_LEGEND_ROW;
    }
    let right = lines.iter().map(|l| l.text_box.right()).fold(size.w, f64::max);
    let bottom = y - SPACING_LEGEND_ROW;
    (entries, lines, Size { w: right, h: bottom })
}

/// The entries alone, and whether there were any.
fn place_entries(spec: &Spec, size: Size) -> (Vec<Entry>, Size, bool) {
    let list = entries(spec);
    if list.is_empty() {
        return (Vec::new(), size, false);
    }
    let line = TYPOGRAPHY_LEGEND.size * TYPOGRAPHY_LEGEND.line_height;
    let widths: Vec<f64> = list
        .iter()
        .map(|&(s, text)| swatch_size(s).w + LEGEND_SWATCH_GAP + text_width(text, &TYPOGRAPHY_LEGEND))
        .collect();
    let limit = widths.iter().copied().fold(size.w, f64::max);
    let row_height = list.iter().map(|&(s, _)| swatch_size(s).h).fold(line, f64::max);
    let (mut x, mut y) = (0.0, size.h + SPACING_LEGEND);
    let mut out = Vec::with_capacity(list.len());
    for (&(swatch, text), &w) in list.iter().zip(&widths) {
        if x > 0.0 && x + w > limit {
            x = 0.0;
            y += row_height + SPACING_LEGEND_ROW;
        }
        let s = swatch_size(swatch);
        let middle = y + row_height / 2.0;
        let swatch_box = Rect {
            x,
            y: (middle - s.h / 2.0).round(),
            w: s.w,
            h: s.h,
        };
        let text_box = Rect {
            x: x + s.w + LEGEND_SWATCH_GAP,
            y: middle - line / 2.0,
            w: w - s.w - LEGEND_SWATCH_GAP,
            h: line,
        };
        out.push(Entry {
            swatch,
            swatch_box,
            text,
            text_box,
        });
        x += w + SPACING_LEGEND_ENTRY;
    }
    let right = out.iter().map(|e| e.text_box.right()).fold(size.w, f64::max);
    let bottom = y + row_height;
    (out, Size { w: right, h: bottom }, true)
}
