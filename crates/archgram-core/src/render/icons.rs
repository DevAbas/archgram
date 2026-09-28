//! The icon of each node kind: line art on a 24-unit square, drawn for
//! archgram (DESIGN.md, Components: Node card). Stroked, never filled, so
//! the category hue is the only colour an icon carries.

use crate::render::scene::Shape;
use crate::spec::Kind;

/// The side of the square every icon is drawn on.
pub const GRID: f64 = 24.0;

/// The icon's shapes, in icon units.
// A table, one arm per kind.
#[allow(clippy::too_many_lines)]
#[must_use]
pub fn icon(kind: Kind) -> &'static [Shape] {
    match kind {
        Kind::Service => &[
            Shape::Rect {
                x: 3.0,
                y: 5.0,
                w: 18.0,
                h: 14.0,
                rx: 2.0,
            },
            Shape::Path("M3 9h18"),
            Shape::Path("M10 12.5l-2 2 2 2M14 12.5l2 2-2 2"),
        ],
        Kind::Database => &[
            Shape::Ellipse {
                cx: 12.0,
                cy: 6.0,
                rx: 7.0,
                ry: 2.6,
            },
            Shape::Path("M5 6v12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6V6"),
            Shape::Path("M5 12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6"),
        ],
        Kind::Queue => &[
            Shape::Rect {
                x: 2.5,
                y: 8.0,
                w: 19.0,
                h: 8.0,
                rx: 4.0,
            },
            Shape::Path("M9 8v8M15 8v8"),
        ],
        Kind::Cache => &[Shape::Path("M13 3L6 13.5h5L10 21l7-10.5h-5z")],
        Kind::Storage => &[
            Shape::Path("M4 7.5h16l-1.6 11.2a2 2 0 0 1-2 1.8H7.6a2 2 0 0 1-2-1.8z"),
            Shape::Ellipse {
                cx: 12.0,
                cy: 7.5,
                rx: 8.0,
                ry: 2.5,
            },
        ],
        Kind::Users => &[
            Shape::Circle {
                cx: 12.0,
                cy: 8.0,
                r: 3.5,
            },
            Shape::Path("M5 20c.6-3.6 3.4-6 7-6s6.4 2.4 7 6"),
        ],
        Kind::Model => &[Shape::Path(
            "M12 3c.8 5 4 8.2 9 9-5 .8-8.2 4-9 9-.8-5-4-8.2-9-9 5-.8 8.2-4 9-9z",
        )],
        Kind::VectorStore => &[
            Shape::Ellipse {
                cx: 12.0,
                cy: 6.0,
                rx: 7.0,
                ry: 2.6,
            },
            Shape::Path("M5 6v12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6V6"),
            Shape::Circle {
                cx: 9.0,
                cy: 12.0,
                r: 0.9,
            },
            Shape::Circle {
                cx: 14.5,
                cy: 11.0,
                r: 0.9,
            },
            Shape::Circle {
                cx: 12.0,
                cy: 15.5,
                r: 0.9,
            },
        ],
        Kind::Tool => &[Shape::Path(
            "M14.5 4a5 5 0 0 0-4.3 6.9L4 17.1V20h2.9l6.2-6.2A5 5 0 0 0 20 9.5l-3 .9-2.4-2.4.9-3z",
        )],
        Kind::Agent => &[
            Shape::Path("M19 12a7 7 0 1 1-2.1-5"),
            Shape::Path("M19.2 4.5v3.2H16"),
            Shape::Circle {
                cx: 12.0,
                cy: 12.0,
                r: 2.0,
            },
        ],
        Kind::File => &[
            Shape::Path("M6.5 3h7.5l4.5 4.5V21h-12z"),
            Shape::Path("M14 3v4.5h4.5"),
        ],
        Kind::Script => &[
            Shape::Rect {
                x: 3.0,
                y: 5.0,
                w: 18.0,
                h: 14.0,
                rx: 2.0,
            },
            Shape::Path("M7 10l3 2-3 2M12.5 15H17"),
        ],
        Kind::Generated => &[
            Shape::Path("M6.5 3h7.5l4.5 4.5V21h-12z"),
            Shape::Path("M14 3v4.5h4.5"),
            Shape::Path(
                "M12.5 10.5c.3 1.9 1.4 3 3.3 3.3-1.9.3-3 1.4-3.3 3.3-.3-1.9-1.4-3-3.3-3.3 1.9-.3 3-1.4 3.3-3.3z",
            ),
        ],
        Kind::Check => &[
            Shape::Path("M12 3l7 3v5.2c0 4.3-2.9 7.9-7 9.8-4.1-1.9-7-5.5-7-9.8V6z"),
            Shape::Path("M9 12l2.2 2.2L15.5 10"),
        ],
        Kind::Browser => &[
            Shape::Rect {
                x: 3.0,
                y: 4.0,
                w: 18.0,
                h: 16.0,
                rx: 2.0,
            },
            Shape::Path("M3 8.5h18"),
            Shape::Circle {
                cx: 6.0,
                cy: 6.3,
                r: 0.6,
            },
            Shape::Circle {
                cx: 8.2,
                cy: 6.3,
                r: 0.6,
            },
        ],
        Kind::Mobile => &[
            Shape::Rect {
                x: 7.0,
                y: 3.0,
                w: 10.0,
                h: 18.0,
                rx: 2.2,
            },
            Shape::Path("M11 17.5h2"),
        ],
        Kind::Desktop => &[
            Shape::Rect {
                x: 3.0,
                y: 4.0,
                w: 18.0,
                h: 12.5,
                rx: 1.8,
            },
            Shape::Path("M9 20.5h6M12 16.5v4"),
        ],
    }
}
