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

/// The most sources one spec may name: far more than a drawing a reader
/// takes in (about ten nodes and twelve edges), and few enough that looking
/// them all up stays quick whatever the spec asks.
pub const MOST_SOURCES: usize = 1000;

/// What the caller found at a source's path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// A file: for each of the words asked about, in the order asked,
    /// whether the file holds them; or why it could not be searched. Asked
    /// about no words, the file need not be read at all.
    File(Result<Vec<bool>, String>),
    /// A folder.
    Folder,
    /// Nothing at that path.
    Nothing,
    /// A path the caller does not read, with the reason: outside the
    /// project, through a symbolic link, or neither a file nor a folder.
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

/// Names archgram never reads, in a source or a theme: git's own folder,
/// and files and folders that commonly hold secrets rather than code. With
/// the reason, when `path` (`/`-separated) has one of them.
#[must_use]
pub fn private_part(path: &str) -> Option<String> {
    for part in path.split('/') {
        let name = part.to_ascii_lowercase();
        if name == ".git" {
            return Some(format!(
                "`{path}` is inside `.git`, which archgram never reads: it is git's own, not the project's code"
            ));
        }
        let secret = matches!(
            name.as_str(),
            ".env"
                | ".npmrc"
                | ".pypirc"
                | ".netrc"
                | ".git-credentials"
                | ".ssh"
                | ".aws"
                | ".gnupg"
                | ".docker"
                | "id_rsa"
                | "id_dsa"
                | "id_ecdsa"
                | "id_ed25519"
        ) || name.starts_with(".env.")
            || [".pem", ".key", ".p12", ".pfx"]
                .iter()
                .any(|end| name.ends_with(end));
        if secret {
            return Some(format!(
                "`{path}` names `{part}`, which commonly holds secrets rather than code, and archgram never reads it"
            ));
        }
    }
    None
}

/// A source's path in one spelling: no `.`, no empty part, `..` only at its
/// start. Two spellings of one file give the same, so it is looked up once.
/// None when a `..` follows a name, which `form` refuses.
fn plain(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." if parts.iter().any(|p| *p != "..") => return None,
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
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
    let Some(plain) = plain(path) else {
        return Some(format!(
            "`{path}` goes into a folder and back out with `..`; write the path without the detour"
        ));
    };
    if plain.is_empty() || plain.split('/').all(|p| p == "..") {
        return Some(format!(
            "`{path}` names only the spec's folder or one above it, which says nothing about the code; name the file or folder behind the part"
        ));
    }
    if let Some(why) = private_part(&plain) {
        return Some(why);
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
/// nothing there, or words its file does not hold. `find` is asked once
/// per file, however its path is spelt, with the path in one spelling (`..`
/// only at its start, from the spec's folder) and every distinct set of
/// words any source wants from it; a file asked about no words need not be
/// read. The spec must have passed validation.
#[must_use]
pub fn check(spec: &Spec, find: &dyn Fn(&str, &[&str]) -> Found) -> Vec<SpecError> {
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
    // Each file in one spelling, with the words wanted from it, before any
    // is looked up.
    let mut wants: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for (_, _, sources) in &owned {
        for source in sources.iter().flat_map(Sources::all) {
            let (path, text) = split(source);
            let words = wants.entry(plain(path).unwrap_or_default()).or_default();
            if let Some(text) = text
                && !words.contains(&text)
            {
                words.push(text);
            }
        }
    }
    let found: BTreeMap<&str, Found> = wants
        .iter()
        .map(|(path, words)| (path.as_str(), find(path, words)))
        .collect();
    let mut errors = Vec::new();
    for (pointer, owner, sources) in &owned {
        let Some(sources) = sources else { continue };
        for (j, source) in sources.all().iter().enumerate() {
            let (path, text) = split(source);
            let key = plain(path).unwrap_or_default();
            let problem = match (&found[key.as_str()], text) {
                (Found::Nothing, _) => Some(format!("`{path}` does not exist")),
                (Found::Unreadable(why), _) => Some(format!("`{path}` cannot be read: {why}")),
                (Found::Folder, Some(_)) => Some(format!(
                    "`{path}` is a folder; the text after `#` is looked for in a file"
                )),
                (Found::File(Err(why)), Some(_)) => {
                    Some(format!("`{path}` cannot be searched: {why}"))
                }
                (Found::File(Ok(held)), Some(text)) => {
                    let at = wants[&key].iter().position(|w| *w == text);
                    let holds = at.and_then(|i| held.get(i)).copied().unwrap_or(false);
                    (!holds).then(|| format!("`{path}` does not hold `{text}`"))
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
        for good in [
            "src/a.rs",
            "../../src/a.rs",
            "src",
            "./src",
            "a.rs#links.insert(",
            "a.rs#a bc",
            ".github/workflows/ci.yml",
            "../../src/./a.rs",
            "src//a.rs",
            "config/environment.ts",
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
            "../../docs/..",
            "src/x/../a.rs",
            "../../.env",
            "../../.env.production",
            "keys/server.pem",
            "../../.ssh/id_ed25519",
            "a.rs#",
            "a.rs#  ",
            "a.rs#m",
            "a.rs# a b ",
            "a.rs#one\ntwo",
        ] {
            assert!(form(bad).is_some(), "{bad:?}");
        }
    }

    #[test]
    fn spellings_of_one_path_are_one() {
        assert_eq!(plain("./src//a.rs").as_deref(), Some("src/a.rs"));
        assert_eq!(plain("../../src/./a.rs").as_deref(), Some("../../src/a.rs"));
        assert_eq!(plain("src/x/../a.rs"), None);
    }
}
