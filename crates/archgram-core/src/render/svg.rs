//! Writing SVG text deterministically: numbers always in the same form,
//! attributes always in the order the code writes them, text escaped.

use std::fmt::Write as _;

use crate::render::scene::{
    Anchor, Decl, GroupOf, Item, Paint, Place, Rule, Scene, Shape, StyleLine,
};

/// A number with at most two decimals and no trailing zeros: `12`, `12.5`, `12.25`.
/// Two decimals is a hundredth of a pixel, below anything a screen shows, and a
/// fixed form keeps the same drawing the same bytes on every machine.
#[must_use]
pub fn num(v: f64) -> String {
    let rounded = (v * 100.0).round() / 100.0;
    // Avoid "-0".
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    let mut s = format!("{rounded:.2}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

/// Text safe inside an SVG element or a double-quoted attribute.
#[must_use]
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(c),
        }
    }
    out
}

/// An SVG document under construction, one element per line.
#[derive(Debug, Default)]
pub struct Svg {
    body: String,
    depth: usize,
}

impl Svg {
    /// Writes one line at the current depth.
    pub fn line(&mut self, text: &str) {
        for _ in 0..self.depth {
            self.body.push_str("  ");
        }
        self.body.push_str(text);
        self.body.push('\n');
    }

    /// Opens a group; `close` ends it.
    pub fn open(&mut self, tag: &str) {
        self.line(tag);
        self.depth += 1;
    }

    pub fn close(&mut self, tag: &str) {
        self.depth -= 1;
        let _ = write!(&mut self.body, "{}", "  ".repeat(self.depth));
        self.body.push_str(tag);
        self.body.push('\n');
    }

    #[must_use]
    pub fn finish(self) -> String {
        self.body
    }
}

/// A number as an icon writes it: `num`'s form without a leading zero, so
/// `0.9` is `.9`.
fn short(v: f64) -> String {
    let n = num(v);
    match n.strip_prefix("0.") {
        Some(rest) => format!(".{rest}"),
        None => n
            .strip_prefix("-0.")
            .map_or(n.clone(), |rest| format!("-.{rest}")),
    }
}

fn place(p: &Place, always_scale: bool) -> String {
    if always_scale || (p.scale - 1.0).abs() >= f64::EPSILON {
        format!(
            "translate({} {}) scale({})",
            num(p.x),
            num(p.y),
            num(p.scale)
        )
    } else {
        format!("translate({} {})", num(p.x), num(p.y))
    }
}

/// A style rule as CSS: `.card { fill: var(--card); … }`.
#[must_use]
pub fn rule(r: &Rule) -> String {
    let paint = |p: &Paint| match p {
        Paint::None => "none".to_owned(),
        Paint::Role(role) => format!("var(--{})", role.name()),
    };
    let decls: Vec<String> = r
        .decls
        .iter()
        .map(|d| match d {
            Decl::Fill(p) => format!("fill: {};", paint(p)),
            Decl::Stroke(p) => format!("stroke: {};", paint(p)),
            Decl::StrokeWidth(w) => format!("stroke-width: {};", num(*w)),
            Decl::Dash(d) => format!(
                "stroke-dasharray: {};",
                d.iter().map(|v| num(*v)).collect::<Vec<_>>().join(" ")
            ),
            Decl::RoundCaps => "stroke-linecap: round;".to_owned(),
            Decl::RoundJoins => "stroke-linejoin: round;".to_owned(),
            Decl::Opacity(o) => format!("opacity: {};", num(*o)),
            Decl::Font(t) => {
                let spacing = if t.letter_spacing == 0.0 {
                    String::new()
                } else {
                    format!(" letter-spacing: {}px;", num(t.letter_spacing))
                };
                format!(
                    "font-size: {}px; font-weight: {};{spacing}",
                    num(t.size),
                    t.weight
                )
            }
        })
        .collect();
    format!("{} {{ {} }}", r.selector, decls.join(" "))
}

fn shape(s: &Shape) -> String {
    match s {
        Shape::Rect { x, y, w, h, rx } => format!(
            r#"<rect x="{}" y="{}" width="{}" height="{}" rx="{}"/>"#,
            short(*x),
            short(*y),
            short(*w),
            short(*h),
            short(*rx)
        ),
        Shape::Ellipse { cx, cy, rx, ry } => format!(
            r#"<ellipse cx="{}" cy="{}" rx="{}" ry="{}"/>"#,
            short(*cx),
            short(*cy),
            short(*rx),
            short(*ry)
        ),
        Shape::Circle { cx, cy, r } => format!(
            r#"<circle cx="{}" cy="{}" r="{}"/>"#,
            short(*cx),
            short(*cy),
            short(*r)
        ),
        Shape::Path(d) => format!(r#"<path d="{d}"/>"#),
    }
}

fn item(svg: &mut Svg, it: &Item) {
    match it {
        Item::Canvas { width, height, rx } => svg.line(&format!(
            r#"<rect class="canvas" width="{}" height="{}" rx="{}"/>"#,
            num(*width),
            num(*height),
            num(*rx)
        )),
        Item::Arrowhead { length, width } => svg.line(&format!(
            r#"<defs><marker id="arrow" viewBox="0 0 {l} {w}" refX="{l}" refY="{ry}" markerWidth="{l}" markerHeight="{w}" markerUnits="userSpaceOnUse" orient="auto-start-reverse" overflow="visible"><path class="arrowhead" d="M0 0L{l} {ry}L0 {w}"/></marker></defs>"#,
            l = num(*length),
            w = num(*width),
            ry = num(*width / 2.0)
        )),
        Item::Group { of, items } => {
            svg.open(&match of {
                GroupOf::Class(c) => format!(r#"<g class="{c}">"#),
                GroupOf::Node(id) => format!(r#"<g data-node="{}">"#, escape(id)),
            });
            for i in items {
                item(svg, i);
            }
            svg.close("</g>");
        }
        Item::Rect { class, x, y, w, h, rx } => {
            let rx = rx.map(|r| format!(r#" rx="{}""#, num(r))).unwrap_or_default();
            svg.line(&format!(
                r#"<rect class="{class}" x="{}" y="{}" width="{}" height="{}"{rx}/>"#,
                num(*x),
                num(*y),
                num(*w),
                num(*h)
            ));
        }
        Item::Circle { class, cx, cy, r } => svg.line(&format!(
            r#"<circle class="{class}" cx="{}" cy="{}" r="{}"/>"#,
            num(*cx),
            num(*cy),
            num(*r)
        )),
        Item::Path {
            id,
            class,
            place: at,
            d,
            arrowhead,
        } => {
            let id = id.as_ref().map(|i| format!(r#"id="{i}" "#)).unwrap_or_default();
            let at = at.map(|p| format!(r#" transform="{}""#, place(&p, true))).unwrap_or_default();
            let marker = if *arrowhead { r#" marker-end="url(#arrow)""# } else { "" };
            svg.line(&format!(r#"<path {id}class="{class}"{at} d="{}"{marker}/>"#, escape(d)));
        }
        Item::Text {
            class,
            x,
            y,
            anchor,
            text,
        } => {
            let anchor = match anchor {
                Anchor::Start => "",
                Anchor::Middle => r#" text-anchor="middle""#,
            };
            svg.line(&format!(
                r#"<text class="{class}" x="{}" y="{}"{anchor}>{}</text>"#,
                num(*x),
                num(*y),
                escape(text)
            ));
        }
        Item::Icon {
            class,
            place: at,
            stroke_width,
            shapes,
        } => {
            let inner: String = shapes.iter().map(shape).collect();
            svg.line(&format!(
                r#"<g class="{class}" transform="{}" stroke-width="{}">{inner}</g>"#,
                place(at, false),
                num(*stroke_width)
            ));
        }
        Item::Motion(markup) => svg.line(markup),
    }
}

/// A few items as one line of SVG, for markup that only motion shows
/// (`Item::Motion`), drawn exactly as the scene draws them.
#[must_use]
pub fn inline(items: &[Item]) -> String {
    let mut svg = Svg::default();
    for i in items {
        item(&mut svg, i);
    }
    svg.finish().lines().collect()
}

/// A scene as SVG text.
#[must_use]
pub fn write(scene: &Scene) -> String {
    let mut svg = Svg::default();
    svg.open(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" role="img" aria-labelledby="title desc">"#,
        w = num(scene.width),
        h = num(scene.height)
    ));
    svg.line(&format!(
        r#"<title id="title">{}</title>"#,
        escape(&scene.title)
    ));
    svg.line(&format!(
        r#"<desc id="desc">{}</desc>"#,
        escape(&scene.description)
    ));
    svg.open("<style>");
    for line in &scene.style {
        match line {
            StyleLine::Rule(r) => svg.line(&rule(r)),
            StyleLine::Raw(text) => svg.line(text),
        }
    }
    svg.close("</style>");
    for i in &scene.items {
        item(&mut svg, i);
    }
    svg.close("</svg>");
    svg.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_have_one_form() {
        assert_eq!(num(12.0), "12");
        assert_eq!(num(12.5), "12.5");
        assert_eq!(num(12.254), "12.25");
        assert_eq!(num(-0.001), "0");
        assert_eq!(num(0.1 + 0.2), "0.3");
    }

    #[test]
    fn text_is_escaped() {
        assert_eq!(
            escape(r#"<a href="x">&</a>"#),
            "&lt;a href=&quot;x&quot;&gt;&amp;&lt;/a&gt;"
        );
    }
}
