//! The icon of each node kind: line art on a 24-unit square, drawn for
//! archgram (DESIGN.md, Components: Node card). Stroked, never filled, so
//! the category hue is the only colour an icon carries.

use crate::spec::Kind;

/// The side of the square every icon is drawn on.
pub const GRID: f64 = 24.0;

/// The icon's SVG content, in icon units.
#[must_use]
pub fn icon(kind: Kind) -> &'static str {
    match kind {
        Kind::Service => {
            r#"<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M3 9h18"/><path d="M10 12.5l-2 2 2 2M14 12.5l2 2-2 2"/>"#
        }
        Kind::Database => {
            r#"<ellipse cx="12" cy="6" rx="7" ry="2.6"/><path d="M5 6v12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6V6"/><path d="M5 12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6"/>"#
        }
        Kind::Queue => r#"<rect x="2.5" y="8" width="19" height="8" rx="4"/><path d="M9 8v8M15 8v8"/>"#,
        Kind::Cache => r#"<path d="M13 3L6 13.5h5L10 21l7-10.5h-5z"/>"#,
        Kind::Storage => {
            r#"<path d="M4 7.5h16l-1.6 11.2a2 2 0 0 1-2 1.8H7.6a2 2 0 0 1-2-1.8z"/><ellipse cx="12" cy="7.5" rx="8" ry="2.5"/>"#
        }
        Kind::Users => r#"<circle cx="12" cy="8" r="3.5"/><path d="M5 20c.6-3.6 3.4-6 7-6s6.4 2.4 7 6"/>"#,
        Kind::Model => r#"<path d="M12 3c.8 5 4 8.2 9 9-5 .8-8.2 4-9 9-.8-5-4-8.2-9-9 5-.8 8.2-4 9-9z"/>"#,
        Kind::VectorStore => {
            r#"<ellipse cx="12" cy="6" rx="7" ry="2.6"/><path d="M5 6v12c0 1.4 3.1 2.6 7 2.6s7-1.2 7-2.6V6"/><circle cx="9" cy="12" r=".9"/><circle cx="14.5" cy="11" r=".9"/><circle cx="12" cy="15.5" r=".9"/>"#
        }
        Kind::Tool => {
            r#"<path d="M14.5 4a5 5 0 0 0-4.3 6.9L4 17.1V20h2.9l6.2-6.2A5 5 0 0 0 20 9.5l-3 .9-2.4-2.4.9-3z"/>"#
        }
        Kind::Agent => {
            r#"<path d="M19 12a7 7 0 1 1-2.1-5"/><path d="M19.2 4.5v3.2H16"/><circle cx="12" cy="12" r="2"/>"#
        }
        Kind::File => r#"<path d="M6.5 3h7.5l4.5 4.5V21h-12z"/><path d="M14 3v4.5h4.5"/>"#,
        Kind::Script => {
            r#"<rect x="3" y="5" width="18" height="14" rx="2"/><path d="M7 10l3 2-3 2M12.5 15H17"/>"#
        }
        Kind::Generated => {
            r#"<path d="M6.5 3h7.5l4.5 4.5V21h-12z"/><path d="M14 3v4.5h4.5"/><path d="M12.5 10.5c.3 1.9 1.4 3 3.3 3.3-1.9.3-3 1.4-3.3 3.3-.3-1.9-1.4-3-3.3-3.3 1.9-.3 3-1.4 3.3-3.3z"/>"#
        }
        Kind::Check => {
            r#"<path d="M12 3l7 3v5.2c0 4.3-2.9 7.9-7 9.8-4.1-1.9-7-5.5-7-9.8V6z"/><path d="M9 12l2.2 2.2L15.5 10"/>"#
        }
        Kind::Browser => {
            r#"<rect x="3" y="4" width="18" height="16" rx="2"/><path d="M3 8.5h18"/><circle cx="6" cy="6.3" r=".6"/><circle cx="8.2" cy="6.3" r=".6"/>"#
        }
        Kind::Mobile => r#"<rect x="7" y="3" width="10" height="18" rx="2.2"/><path d="M11 17.5h2"/>"#,
        Kind::Desktop => {
            r#"<rect x="3" y="4" width="18" height="12.5" rx="1.8"/><path d="M9 20.5h6M12 16.5v4"/>"#
        }
    }
}
