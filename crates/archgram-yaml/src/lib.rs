//! YAML specs for archgram (ARCHITECTURE.md, Parse and validate).
//!
//! The core reads JSON only. This crate reads YAML with `saphyr-parser`'s
//! events (YAML 1.2), builds its own tree with each value's line and column,
//! writes the tree as JSON, one value per line, and lets the core read and
//! check that. Every problem the core finds is then moved back to where the
//! author wrote it in the YAML: a JSON line maps to its value's position,
//! a JSON pointer to its value's position. So the rules live once, in the
//! core, and a YAML author still sees `line:column`.
//!
//! Beyond the parser's own checks, a spec is refused when it holds more than
//! one document, a key that is not plain text, a key twice in one mapping, a
//! tag outside YAML's core schema, nesting deeper than [`MAX_DEPTH`], or
//! aliases that expand past [`MAX_EXPANDED`] values (a "billion laughs").
//! Plain scalars follow YAML 1.2's core schema: `null`, booleans, integers
//! and floats become JSON literals, everything else text.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use archgram_core::{Location, Spec, SpecError};
use saphyr_parser::{Event, Parser, ScalarStyle, Span, Tag};

/// The deepest nesting a spec may have. A spec needs five levels.
pub const MAX_DEPTH: usize = 64;

/// How many values aliases may add in all: a spec repeats little, so a
/// budget far above any real one stops an alias bomb before it grows.
pub const MAX_EXPANDED: usize = 10_000;

/// A line and a column, both counted from 1.
type At = (usize, usize);

/// What to add to `saphyr-parser`'s column to count from 1
/// (`tests/yaml.rs` pins it).
const COLUMN_BASE: usize = 1;

/// Where each part of a spec was written in its YAML.
#[derive(Debug, Clone, Default)]
pub struct Positions {
    /// For each line of the JSON the core read, the YAML position it came from.
    lines: Vec<At>,
    /// For each JSON pointer into the spec, its value's YAML position.
    pointers: BTreeMap<String, At>,
}

impl Positions {
    /// The problem, moved to where it was written in the YAML. A pointer
    /// the tree lacks takes its nearest ancestor's position.
    #[must_use]
    pub fn locate(&self, error: SpecError) -> SpecError {
        let at = match &error.location {
            Location::LineColumn { line, .. } => self.lines.get(line.saturating_sub(1)).copied(),
            Location::Pointer(p) => {
                let mut p = p.as_str();
                loop {
                    if let Some(&at) = self.pointers.get(p) {
                        break Some(at);
                    }
                    match p.rfind('/') {
                        Some(i) => p = &p[..i],
                        None => break None,
                    }
                }
            }
        };
        match at {
            Some((line, column)) => SpecError {
                location: Location::LineColumn { line, column },
                message: error.message,
            },
            None => error,
        }
    }
}

/// Reads and checks a YAML spec.
///
/// # Errors
///
/// Every problem, located by its line and column in the YAML.
pub fn parse_spec(yaml: &str) -> Result<Spec, Vec<SpecError>> {
    parse(yaml).map(|(spec, _)| spec)
}

/// Reads and checks a YAML spec, and keeps where each part was written, so
/// checks made later (such as the logos a build carries) can be located too.
///
/// # Errors
///
/// Every problem, located by its line and column in the YAML.
pub fn parse(yaml: &str) -> Result<(Spec, Positions), Vec<SpecError>> {
    let root = read_tree(yaml).map_err(|e| vec![e])?;
    let mut json = Json::default();
    json.value(&root, None, "");
    let positions = Positions {
        lines: json.at,
        pointers: json.pointers,
    };
    match archgram_core::parse_spec(&json.lines.join("\n")) {
        Ok(spec) => Ok((spec, positions)),
        Err(errors) => Err(errors.into_iter().map(|e| positions.locate(hint(e))).collect()),
    }
}

/// A plain YAML scalar the core wanted as text: say how to keep it text.
fn hint(mut e: SpecError) -> SpecError {
    let plain = ["integer", "floating point", "boolean", "null"]
        .iter()
        .any(|t| e.message.contains(&format!("invalid type: {t}")));
    if plain && e.message.contains("expected a string") {
        e.message.push_str("; in YAML, quote it to keep it as text");
    }
    e
}

/// A value of the tree, with where it starts.
#[derive(Debug, Clone)]
struct Node {
    at: At,
    value: Value,
}

#[derive(Debug, Clone)]
enum Value {
    /// Keys in the order written, each with its own position.
    Map(Vec<(String, At, Node)>),
    Seq(Vec<Node>),
    /// The JSON literal the scalar reads as: `null`, `true`, `12`, `"text"`;
    /// and the text itself, for a key.
    Scalar {
        literal: String,
        text: String,
    },
}

impl Node {
    /// How many values it holds, itself included.
    fn count(&self) -> usize {
        1 + match &self.value {
            Value::Map(entries) => entries.iter().map(|(_, _, n)| n.count()).sum(),
            Value::Seq(items) => items.iter().map(Node::count).sum(),
            Value::Scalar { .. } => 0,
        }
    }
}

/// A mapping or sequence being read.
enum Open {
    Map {
        at: At,
        anchor: usize,
        entries: Vec<(String, At, Node)>,
        key: Option<(String, At)>,
    },
    Seq {
        at: At,
        anchor: usize,
        items: Vec<Node>,
    },
}

fn at(span: &Span) -> At {
    (span.start.line(), span.start.col() + COLUMN_BASE)
}

fn problem(at: At, message: impl Into<String>) -> SpecError {
    SpecError {
        location: Location::LineColumn {
            line: at.0,
            column: at.1,
        },
        message: message.into(),
    }
}

/// Reads the YAML into a tree: one document, plain-text keys, no key twice.
fn read_tree(yaml: &str) -> Result<Node, SpecError> {
    let mut stack: Vec<Open> = Vec::new();
    let mut anchors: BTreeMap<usize, Node> = BTreeMap::new();
    let mut root: Option<Node> = None;
    let mut documents = 0;
    let mut expanded = 0usize;
    for event in Parser::new_from_str(yaml) {
        let (event, span) = event.map_err(|e| {
            let m = e.marker();
            problem((m.line(), m.col() + COLUMN_BASE), e.info().to_owned())
        })?;
        let here = at(&span);
        let done = match event {
            Event::DocumentStart(_) => {
                documents += 1;
                if documents > 1 {
                    return Err(problem(here, "a spec is one YAML document; this starts a second"));
                }
                None
            }
            Event::Scalar(text, style, anchor, tag) => {
                let node = Node {
                    at: here,
                    value: Value::Scalar {
                        literal: scalar(&text, style, tag.as_deref(), here)?,
                        text: text.into_owned(),
                    },
                };
                if anchor > 0 {
                    anchors.insert(anchor, node.clone());
                }
                Some(node)
            }
            Event::Alias(id) => {
                let node = anchors
                    .get(&id)
                    .cloned()
                    .ok_or_else(|| problem(here, "an alias to an anchor not yet defined"))?;
                expanded += node.count();
                if expanded > MAX_EXPANDED {
                    return Err(problem(
                        here,
                        format!("aliases expand to more than {MAX_EXPANDED} values"),
                    ));
                }
                Some(Node { at: here, ..node })
            }
            Event::MappingStart(anchor, tag) => {
                opening(&stack, tag.as_deref(), here)?;
                stack.push(Open::Map {
                    at: here,
                    anchor,
                    entries: Vec::new(),
                    key: None,
                });
                None
            }
            Event::SequenceStart(anchor, tag) => {
                opening(&stack, tag.as_deref(), here)?;
                stack.push(Open::Seq {
                    at: here,
                    anchor,
                    items: Vec::new(),
                });
                None
            }
            Event::MappingEnd | Event::SequenceEnd => {
                let (node, anchor) = close(stack.pop(), here)?;
                if anchor > 0 {
                    anchors.insert(anchor, node.clone());
                }
                Some(node)
            }
            Event::StreamStart | Event::StreamEnd | Event::DocumentEnd | Event::Nothing => None,
        };
        if let Some(node) = done {
            place(&mut stack, &mut root, node)?;
        }
    }
    root.ok_or_else(|| problem((1, 1), "the file holds no spec"))
}

/// Checks a mapping or list about to open: not too deep, no foreign tag,
/// and not a key.
fn opening(stack: &[Open], tag: Option<&Tag>, here: At) -> Result<(), SpecError> {
    if stack.len() >= MAX_DEPTH {
        return Err(problem(here, format!("nesting deeper than {MAX_DEPTH} levels")));
    }
    if let Some(t) = tag
        && !(t.is_yaml_core_schema() && matches!(t.suffix.as_str(), "map" | "seq"))
    {
        return Err(problem(
            here,
            format!(
                "the tag `{}{}` is not part of YAML's core schema",
                t.handle, t.suffix
            ),
        ));
    }
    if let Some(Open::Map { key: None, .. }) = stack.last() {
        return Err(problem(here, "a key must be plain text, not a mapping or a list"));
    }
    Ok(())
}

/// The finished mapping or list, and its anchor.
fn close(open: Option<Open>, here: At) -> Result<(Node, usize), SpecError> {
    match open {
        Some(Open::Map {
            at, anchor, entries, ..
        }) => Ok((
            Node {
                at,
                value: Value::Map(entries),
            },
            anchor,
        )),
        Some(Open::Seq { at, anchor, items }) => Ok((
            Node {
                at,
                value: Value::Seq(items),
            },
            anchor,
        )),
        None => Err(problem(here, "an end without a start")),
    }
}

/// Puts a finished value where it belongs: a key, a mapping's value, a
/// list's item, or the document's root. A key comes once per mapping.
fn place(stack: &mut [Open], root: &mut Option<Node>, node: Node) -> Result<(), SpecError> {
    match stack.last_mut() {
        None => *root = Some(node),
        Some(Open::Seq { items, .. }) => items.push(node),
        Some(Open::Map { entries, key, .. }) => {
            if let Some((k, k_at)) = key.take() {
                entries.push((k, k_at, node));
            } else {
                let Value::Scalar { text, .. } = &node.value else {
                    return Err(problem(node.at, "a key must be plain text"));
                };
                if let Some((_, first, _)) = entries.iter().find(|(k, _, _)| k == text) {
                    return Err(problem(
                        node.at,
                        format!("duplicate key `{text}`; first at {}:{}", first.0, first.1),
                    ));
                }
                *key = Some((text.clone(), node.at));
            }
        }
    }
    Ok(())
}

/// The JSON literal a scalar reads as: quoted scalars and `!!str` are text,
/// plain ones follow the core schema.
fn scalar(text: &str, style: ScalarStyle, tag: Option<&Tag>, here: At) -> Result<String, SpecError> {
    let forced_text = match tag {
        None => false,
        Some(t) if t.is_yaml_core_schema() => match t.suffix.as_str() {
            "str" => true,
            "int" | "float" | "bool" | "null" => false,
            other => {
                return Err(problem(
                    here,
                    format!("the tag `!!{other}` does not fit a scalar"),
                ));
            }
        },
        Some(t) => {
            return Err(problem(
                here,
                format!(
                    "the tag `{}{}` is not part of YAML's core schema",
                    t.handle, t.suffix
                ),
            ));
        }
    };
    if forced_text || style != ScalarStyle::Plain {
        return Ok(json_string(text));
    }
    Ok(core_schema(text).unwrap_or_else(|| json_string(text)))
}

/// YAML 1.2's core schema for a plain scalar: the JSON literal of a null,
/// boolean, integer or float, or `None` for text. Infinity and not-a-number
/// have no JSON form and stay text.
fn core_schema(t: &str) -> Option<String> {
    match t {
        "" | "~" | "null" | "Null" | "NULL" => return Some("null".into()),
        "true" | "True" | "TRUE" => return Some("true".into()),
        "false" | "False" | "FALSE" => return Some("false".into()),
        _ => {}
    }
    let digits = |s: &str, radix: u32| !s.is_empty() && s.chars().all(|c| c.is_digit(radix));
    let unsigned = t.strip_prefix(['-', '+']).unwrap_or(t);
    if digits(unsigned, 10) {
        return t.parse::<i64>().ok().map(|n| n.to_string());
    }
    if let Some(oct) = t.strip_prefix("0o").filter(|s| digits(s, 8)) {
        return i64::from_str_radix(oct, 8).ok().map(|n| n.to_string());
    }
    if let Some(hex) = t.strip_prefix("0x").filter(|s| digits(s, 16)) {
        return i64::from_str_radix(hex, 16).ok().map(|n| n.to_string());
    }
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (unsigned, None),
    };
    let mantissa_ok = match mantissa.split_once('.') {
        Some((whole, part)) => {
            (whole.is_empty() || digits(whole, 10))
                && (part.is_empty() || digits(part, 10))
                && !(whole.is_empty() && part.is_empty())
        }
        None => digits(mantissa, 10),
    };
    let exponent_ok = exponent.is_none_or(|e| digits(e.strip_prefix(['-', '+']).unwrap_or(e), 10));
    if mantissa_ok && exponent_ok && (mantissa.contains('.') || exponent.is_some()) {
        return t
            .parse::<f64>()
            .ok()
            .filter(|f| f.is_finite())
            .map(|f| format!("{f:?}"));
    }
    None
}

/// `s` as a JSON string.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The tree written as JSON, one value per line, with each line's YAML
/// position and each pointer's.
#[derive(Default)]
struct Json {
    lines: Vec<String>,
    at: Vec<At>,
    pointers: BTreeMap<String, At>,
}

impl Json {
    fn push(&mut self, line: String, at: At) {
        self.lines.push(line);
        self.at.push(at);
    }

    /// Writes `node`, its first line led by `key` when it is a mapping's
    /// value. A closing bracket takes its container's position, which is
    /// where the JSON reader reports a missing field.
    fn value(&mut self, node: &Node, key: Option<&str>, pointer: &str) {
        let lead = key.map_or_else(String::new, |k| format!("{}: ", json_string(k)));
        self.pointers.insert(pointer.to_owned(), node.at);
        match &node.value {
            Value::Scalar { literal, .. } => self.push(format!("{lead}{literal}"), node.at),
            Value::Map(entries) => {
                self.push(format!("{lead}{{"), node.at);
                for (i, (k, k_at, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        self.comma();
                    }
                    let escaped = k.replace('~', "~0").replace('/', "~1");
                    // The line holds the key and its value: it takes the key's
                    // position, where an unknown field is and a wrong value
                    // sits beside; the pointer keeps the value's own.
                    let child = Node {
                        at: *k_at,
                        ..v.clone()
                    };
                    self.value(&child, Some(k), &format!("{pointer}/{escaped}"));
                    self.pointers.insert(format!("{pointer}/{escaped}"), v.at);
                }
                self.push("}".into(), node.at);
            }
            Value::Seq(items) => {
                self.push(format!("{lead}["), node.at);
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        self.comma();
                    }
                    self.value(v, None, &format!("{pointer}/{i}"));
                }
                self.push("]".into(), node.at);
            }
        }
    }

    /// Ends the last line with a comma, before the next value.
    fn comma(&mut self) {
        if let Some(last) = self.lines.last_mut() {
            last.push(',');
        }
    }
}
