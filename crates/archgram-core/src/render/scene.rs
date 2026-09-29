//! The drawing as a list of shapes (ARCHITECTURE.md, Render): what the
//! render stage decides, before it is text. The SVG writer turns a scene
//! into SVG (`svg::write`); a rasterizer draws the same scene into pixels,
//! with the same style table, so the two never drift apart.
//!
//! Each shape names its style by class, as the SVG does; `style` holds the
//! classes' rules. What moves (the flows' animation) is SVG's alone: it is
//! kept as written markup (`Item::Motion`), which starts invisible, so the
//! still image, the one a rasterizer draws, is the scene without it.

use crate::tokens::{Role, TextStyle};

/// A whole drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    pub width: f64,
    pub height: f64,
    /// The diagram's name and its description, for a screen reader.
    pub title: String,
    pub description: String,
    /// The style sheet, in order: the rules shapes name by class, and lines
    /// only the SVG reads (the theme's properties, fonts, motion).
    pub style: Vec<StyleLine>,
    pub items: Vec<Item>,
}

/// One line of the style sheet.
#[derive(Debug, Clone, PartialEq)]
pub enum StyleLine {
    Rule(Rule),
    /// Written as it is: custom properties, `@font-face`, `@media`, motion.
    Raw(String),
}

/// The declarations of one class (or class pair, such as `.card.external`).
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selector: String,
    pub decls: Vec<Decl>,
}

/// One declaration, in the order it is written.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Decl {
    Fill(Paint),
    Stroke(Paint),
    StrokeWidth(f64),
    Dash(&'static [f64]),
    /// Round line ends.
    RoundCaps,
    /// Round line joins.
    RoundJoins,
    /// How much of a group shows, as one: its parts fade together.
    Opacity(f64),
    /// Size, weight and letter spacing.
    Font(TextStyle),
}

/// What a fill or stroke paints with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Paint {
    None,
    Role(Role),
}

/// Where a text's `x` is: its start or its middle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    Start,
    Middle,
}

/// A shape of an icon, in icon units on its 24-unit square.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shape {
    Rect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        rx: f64,
    },
    Ellipse {
        cx: f64,
        cy: f64,
        rx: f64,
        ry: f64,
    },
    Circle {
        cx: f64,
        cy: f64,
        r: f64,
    },
    Path(&'static str),
}

/// A logo's or icon's placement: moved to `x`, `y`, then scaled.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub x: f64,
    pub y: f64,
    pub scale: f64,
}

/// One thing drawn, in drawing order.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    /// The background, from the origin.
    Canvas { width: f64, height: f64, rx: f64 },
    /// The arrowhead every edge ends in: an open chevron `length` long and
    /// `width` across, its tip at the line's end.
    Arrowhead { length: f64, width: f64 },
    Group {
        /// Its class, such as `edges`, or the node it draws.
        of: GroupOf,
        items: Vec<Item>,
    },
    Rect {
        class: String,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        /// Corner radius; none for square corners.
        rx: Option<f64>,
    },
    Circle {
        class: String,
        cx: f64,
        cy: f64,
        r: f64,
    },
    /// A path in SVG path data: an edge (with its arrowhead, and an id when
    /// a signal follows it) or a logo (placed).
    Path {
        id: Option<String>,
        class: String,
        place: Option<Place>,
        d: String,
        arrowhead: bool,
    },
    Text {
        class: String,
        x: f64,
        y: f64,
        anchor: Anchor,
        text: String,
    },
    /// A kind's icon: its shapes stroked `stroke_width` wide (in icon units).
    Icon {
        class: String,
        place: Place,
        stroke_width: f64,
        shapes: &'static [Shape],
    },
    /// SVG markup for what moves or appears only in motion; invisible in
    /// the still image.
    Motion(String),
}

/// What a group gathers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupOf {
    Class(&'static str),
    Node(String),
}
