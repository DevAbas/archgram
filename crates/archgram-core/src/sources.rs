//! The code behind a diagram (docs/SPEC.md, Sources): the form of each
//! node's and edge's `source`, checked with the rest of the spec, and its
//! presence, checked against what the caller finds. The core reads no file:
//! the caller looks each path up (ARCHITECTURE.md, Parse and validate).

use std::collections::BTreeMap;

use crate::error::SpecError;
use crate::spec::{Sources, Spec};

/// The fewest characters, other than spaces, a source's words may have: fewer
/// are found in almost any file, and say nothing about the code.
pub const FEWEST_WORDS: usize = 3;

/// What the caller found at a source's path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// A file; when the caller was asked for its text, that text, or why it
    /// could not be read (a source that wants no words still finds the file).
    File(Option<Result<String, String>>),
    /// A folder.
    Folder,
    /// Nothing at that path.
    Nothing,
    /// A path the caller does not read, with the reason: outside the folder
    /// it may read, through a symbolic link, too large, or neither a file
    /// nor a folder.
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
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    if parts.iter().all(|p| *p == "." || *p == "..") {
        return Some(format!(
            "`{path}` names only the spec's folder or one above it, which says nothing about the code; name the file or folder behind the part"
        ));
    }
    if parts.iter().any(|p| p.eq_ignore_ascii_case(".git")) {
        return Some(format!(
            "`{path}` is inside `.git`, which archgram never reads: it is git's own, not the project's code"
        ));
    }
    match text {
        Some(text) if text.contains(['\n', '\r']) => Some(
            "the text after `#` spans several lines; copy a few words from one line of code".into(),
        ),
        Some(text) if text.chars().filter(|c| !c.is_whitespace()).count() < FEWEST_WORDS => {
            Some(format!(
                "`{source}` has too little after `#`; copy at least {FEWEST_WORDS} characters from the line of code, so they are not found by chance"
            ))
        }
        _ => None,
    }
}

/// Every source the code does not have, located in the spec: a path with
/// nothing there, or a text its file does not hold. `find` looks a path up
/// as written, from the spec's folder, and is asked once per path, with
/// whether any source wants that file's text: a path cited without words
/// need not be read at all. The spec must have passed validation.
#[must_use]
pub fn check(spec: &Spec, find: &dyn Fn(&str, bool) -> Found) -> Vec<SpecError> {
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
    let owned: Vec<_> = nodes.chain(edges).collect();
    // Whether each path's text is wanted, before any is looked up.
    let mut wants: BTreeMap<&str, bool> = BTreeMap::new();
    for (_, _, sources) in &owned {
        for source in sources.iter().flat_map(Sources::all) {
            let (path, text) = split(source);
            *wants.entry(path).or_default() |= text.is_some();
        }
    }
    let found: BTreeMap<&str, Found> = wants
        .iter()
        .map(|(&path, &words)| (path, find(path, words)))
        .collect();
    let mut errors = Vec::new();
    for (pointer, owner, sources) in &owned {
        let Some(sources) = sources else { continue };
        for (j, source) in sources.all().iter().enumerate() {
            let (path, text) = split(source);
            let problem = match (&found[path], text) {
                (Found::Nothing, _) => Some(format!("`{path}` does not exist")),
                (Found::Unreadable(why), _) => Some(format!("`{path}` cannot be read: {why}")),
                (Found::Folder, Some(_)) => Some(format!(
                    "`{path}` is a folder; the text after `#` is looked for in a file"
                )),
                (Found::File(Some(Ok(body))), Some(text)) if !body.contains(text) => {
                    Some(format!("`{path}` does not hold `{text}`"))
                }
                (Found::File(Some(Err(why))), Some(_)) => {
                    Some(format!("`{path}` cannot be searched: {why}"))
                }
                (Found::File(None), Some(_)) => Some(format!(
                    "`{path}` was not read, so `{}` could not be looked for",
                    text.unwrap_or_default()
                )),
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
        for good in [
            "src/a.rs",
            "../../src/a.rs",
            "src",
            "./src",
            "a.rs#links.insert(",
            "a.rs#a bc",
            ".github/workflows/ci.yml",
        ] {
            assert_eq!(form(good), None, "{good}");
        }
        for bad in [
            "",
            "#text",
            "src\\a.rs",
            "/src/a.rs",
            "C:/src/a.rs",
            ".",
            "..",
            "../..",
            "./../",
            ".git/config",
            "../../.GIT/config",
            "a.rs#",
            "a.rs#  ",
            "a.rs#m",
            "a.rs# a b ",
            "a.rs#one\ntwo",
        ] {
            assert!(form(bad).is_some(), "{bad:?}");
        }
    }
}
