//! The code behind a diagram (docs/SPEC.md, Sources): the form of each
//! node's and edge's `source`, checked with the rest of the spec, and its
//! presence, checked against what the caller finds. The core reads no file:
//! the caller looks each path up (ARCHITECTURE.md, Parse and validate).

use std::collections::BTreeMap;

use crate::error::SpecError;
use crate::spec::{Sources, Spec};

/// What the caller found at a source's path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// A file, with its text.
    File(String),
    /// A folder.
    Folder,
    /// Nothing at that path.
    Nothing,
    /// Something that is neither a file nor a folder, or cannot be read;
    /// with the reason.
    Unreadable(String),
}

/// A source's path, and the text after its first `#`, if any.
#[must_use]
pub fn split(source: &str) -> (&str, Option<&str>) {
    match source.split_once('#') {
        Some((path, text)) => (path, Some(text)),
        None => (source, None),
    }
}

/// What is wrong with a source's form, if anything.
pub(crate) fn form(source: &str) -> Option<String> {
    let (path, text) = split(source);
    let bytes = path.as_bytes();
    if path.is_empty() {
        return Some(format!(
            "`{source}` names no path; a source is a path from the spec's folder, such as `../../src/api.ts`"
        ));
    }
    if path.contains('\\') {
        return Some(format!(
            "`{path}` holds `\\`; a source's folders are separated by `/`, on every system"
        ));
    }
    if path.starts_with('/')
        || (bytes.len() > 1 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
    {
        return Some(format!(
            "`{path}` is an absolute path; a source is a path from the spec's folder, so it holds on every machine"
        ));
    }
    match text {
        Some(text) if text.trim().is_empty() => Some(format!(
            "`{source}` has nothing after `#`; leave out the `#`, or copy a few words from the line of code"
        )),
        Some(text) if text.contains(['\n', '\r']) => Some(
            "the text after `#` spans several lines; copy a few words from one line of code".into(),
        ),
        _ => None,
    }
}

/// Every source the code does not have, located in the spec: a path with
/// nothing there, or a text its file does not hold. `find` looks a path up
/// as written, from the spec's folder, and is asked once per path. The spec
/// must have passed validation.
#[must_use]
pub fn check(spec: &Spec, find: &dyn Fn(&str) -> Found) -> Vec<SpecError> {
    let nodes = spec.nodes.iter().enumerate().map(|(i, n)| {
        (
            format!("/nodes/{i}/source"),
            format!("node {}", n.id),
            &n.source,
        )
    });
    let edges = spec.edges.iter().enumerate().map(|(i, e)| {
        (
            format!("/edges/{i}/source"),
            format!("edge {} \u{2192} {}", e.from, e.to),
            &e.source,
        )
    });
    let mut found: BTreeMap<&str, Found> = BTreeMap::new();
    let mut errors = Vec::new();
    for (pointer, owner, sources) in nodes.chain(edges) {
        let Some(sources) = sources else { continue };
        for (j, source) in sources.all().iter().enumerate() {
            let (path, text) = split(source);
            let problem = match (found.entry(path).or_insert_with(|| find(path)), text) {
                (Found::Nothing, _) => Some(format!("`{path}` does not exist")),
                (Found::Unreadable(why), _) => Some(format!("`{path}` cannot be read: {why}")),
                (Found::Folder, Some(_)) => Some(format!(
                    "`{path}` is a folder; the text after `#` is looked for in a file"
                )),
                (Found::File(body), Some(text)) if !body.contains(text) => {
                    Some(format!("`{path}` does not hold `{text}`"))
                }
                _ => None,
            };
            if let Some(problem) = problem {
                let at = match sources {
                    Sources::One(_) => pointer.clone(),
                    Sources::Many(_) => format!("{pointer}/{j}"),
                };
                errors.push(SpecError::at(at, format!("{owner}: {problem}")));
            }
        }
    }
    errors
}

/// How many sources the spec names.
#[must_use]
pub fn count(spec: &Spec) -> usize {
    let nodes = spec.nodes.iter().map(|n| &n.source);
    let edges = spec.edges.iter().map(|e| &e.source);
    nodes.chain(edges).flatten().map(|s| s.all().len()).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_splits_at_its_first_hash() {
        assert_eq!(split("src/a.rs"), ("src/a.rs", None));
        assert_eq!(split("src/a.rs#x # y"), ("src/a.rs", Some("x # y")));
    }

    #[test]
    fn a_source_is_a_relative_path_with_one_line_of_text() {
        for good in ["src/a.rs", "../../src/a.rs", "src", "a.rs#links.insert("] {
            assert_eq!(form(good), None, "{good}");
        }
        for bad in [
            "",
            "#text",
            "src\\a.rs",
            "/src/a.rs",
            "C:/src/a.rs",
            "a.rs#",
            "a.rs#  ",
            "a.rs#one\ntwo",
        ] {
            assert!(form(bad).is_some(), "{bad:?}");
        }
    }
}
