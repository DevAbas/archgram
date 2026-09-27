//! Writing SVG text deterministically: numbers always in the same form,
//! attributes always in the order the code writes them, text escaped.

use std::fmt::Write as _;

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
