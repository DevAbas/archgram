//! The rules a spec must follow beyond its types (docs/SPEC.md, Validation).
//! Every problem is collected, in the order the spec lists things, so one
//! run reports them all and the same spec always reports them the same way.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::SpecError;
use crate::spec::Spec;

/// The palettes archgram carries (design-system/tokens/palettes/).
pub const PALETTES: &[&str] = &["mono"];

/// The only version of the spec format.
pub const FORMAT_VERSION: u32 = 1;

/// Checks everything the types cannot express. Returns every problem found.
#[must_use]
pub fn validate(spec: &Spec) -> Vec<SpecError> {
    let mut v = Validator {
        spec,
        errors: Vec::new(),
    };
    v.top_level();
    let ids = v.ids();
    v.nodes(&ids);
    v.frames(&ids);
    v.edges(&ids);
    v.flows(&ids);
    v.hints(&ids);
    v.errors
}

/// What an id names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Named {
    Node,
    Frame,
}

struct Validator<'a> {
    spec: &'a Spec,
    errors: Vec<SpecError>,
}

impl<'a> Validator<'a> {
    fn error(&mut self, pointer: String, message: String) {
        self.errors.push(SpecError::at(pointer, message));
    }

    fn top_level(&mut self) {
        let s = self.spec;
        if s.archgram != FORMAT_VERSION {
            self.error(
                "/archgram".into(),
                format!(
                    "unsupported format version {}; this archgram reads version {FORMAT_VERSION}",
                    s.archgram
                ),
            );
        }
        if s.title.trim().is_empty() {
            self.error("/title".into(), "the title is empty".into());
        }
        if s.description.trim().is_empty() {
            self.error(
                "/description".into(),
                "the description is empty; it is what screen readers announce".into(),
            );
        }
        if !PALETTES.contains(&s.palette.as_str()) {
            self.error(
                "/palette".into(),
                format!(
                    "unknown palette `{}`; known palettes: {}",
                    s.palette,
                    PALETTES.join(", ")
                ),
            );
        }
        if s.nodes.is_empty() {
            self.error("/nodes".into(), "a diagram needs at least one node".into());
        }
    }

    /// Every node and frame id, checked for its form and uniqueness.
    fn ids(&mut self) -> BTreeMap<&'a str, Named> {
        let spec: &'a Spec = self.spec;
        let mut ids = BTreeMap::new();
        let nodes = spec
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (format!("/nodes/{i}/id"), n.id.as_str(), Named::Node));
        let frames = spec
            .frames
            .iter()
            .enumerate()
            .map(|(i, f)| (format!("/frames/{i}/id"), f.id.as_str(), Named::Frame));
        let all: Vec<_> = nodes.chain(frames).collect();
        for (pointer, id, named) in all {
            if !is_valid_id(id) {
                self.error(pointer.clone(), format!("`{id}` is not a valid id; use lowercase letters, digits and single hyphens, such as `api` or `vector-store`"));
            }
            // The first use of an id keeps it; a later one is the duplicate.
            if ids.contains_key(id) {
                self.error(
                    pointer,
                    format!("the id `{id}` is used more than once; node and frame ids share one namespace"),
                );
            } else {
                ids.insert(id, named);
            }
        }
        ids
    }

    fn nodes(&mut self, ids: &BTreeMap<&str, Named>) {
        for (i, n) in self.spec.nodes.iter().enumerate() {
            if n.label.trim().is_empty() {
                self.error(
                    format!("/nodes/{i}/label"),
                    format!("node `{}` has an empty label", n.id),
                );
            }
            if let Some(tech) = &n.tech
                && (tech.is_empty() || !tech.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit()))
            {
                self.error(format!("/nodes/{i}/tech"), format!("`{tech}` is not a Simple Icons slug; slugs are lowercase letters and digits, such as `postgresql`"));
            }
            if let Some(frame) = &n.frame {
                let problem = Self::reference(ids, frame, Named::Frame);
                if let Some(message) = problem {
                    self.error(format!("/nodes/{i}/frame"), message);
                }
            }
        }
    }

    fn frames(&mut self, ids: &BTreeMap<&str, Named>) {
        let frames = &self.spec.frames;
        for (i, f) in frames.iter().enumerate() {
            if f.label.trim().is_empty() {
                self.error(
                    format!("/frames/{i}/label"),
                    format!("frame `{}` has an empty label", f.id),
                );
            }
            if let Some(parent) = &f.parent {
                if parent == &f.id {
                    self.error(
                        format!("/frames/{i}/parent"),
                        format!("frame `{}` cannot be its own parent", f.id),
                    );
                } else if let Some(message) = Self::reference(ids, parent, Named::Frame) {
                    self.error(format!("/frames/{i}/parent"), message);
                }
            }
        }
        // A cycle through parents: walk up from each frame; more steps than frames means a loop.
        let parent: BTreeMap<&str, &str> = frames
            .iter()
            .filter_map(|f| f.parent.as_deref().map(|p| (f.id.as_str(), p)))
            .collect();
        let mut reported = BTreeSet::new();
        for (i, f) in frames.iter().enumerate() {
            let mut at = f.id.as_str();
            let mut steps = 0;
            while let Some(&up) = parent.get(at) {
                at = up;
                steps += 1;
                if steps > frames.len() {
                    if reported.insert(f.id.as_str()) {
                        self.error(
                            format!("/frames/{i}/parent"),
                            format!("frame `{}` is inside itself through its parents", f.id),
                        );
                    }
                    break;
                }
            }
        }
        // An empty frame: no node in it and no frame under it.
        for (i, f) in frames.iter().enumerate() {
            let has_node = self.spec.nodes.iter().any(|n| n.frame.as_deref() == Some(&f.id));
            let has_child = frames.iter().any(|c| c.parent.as_deref() == Some(&f.id));
            if !has_node && !has_child {
                self.error(
                    format!("/frames/{i}"),
                    format!("frame `{}` is empty; put a node in it or remove it", f.id),
                );
            }
        }
    }

    fn edges(&mut self, ids: &BTreeMap<&str, Named>) {
        let mut seen = BTreeSet::new();
        for (i, e) in self.spec.edges.iter().enumerate() {
            for (field, id) in [("from", &e.from), ("to", &e.to)] {
                if let Some(message) = Self::reference(ids, id, Named::Node) {
                    self.error(format!("/edges/{i}/{field}"), message);
                }
            }
            if e.from == e.to {
                self.error(
                    format!("/edges/{i}"),
                    format!(
                        "an edge from `{}` to itself says nothing a reader can use",
                        e.from
                    ),
                );
            }
            if !seen.insert((e.from.as_str(), e.to.as_str())) {
                self.error(
                    format!("/edges/{i}"),
                    format!(
                        "a second edge from `{}` to `{}`; use one edge with a label",
                        e.from, e.to
                    ),
                );
            }
        }
    }

    fn flows(&mut self, ids: &BTreeMap<&str, Named>) {
        let edges: BTreeSet<(&str, &str)> = self
            .spec
            .edges
            .iter()
            .map(|e| (e.from.as_str(), e.to.as_str()))
            .collect();
        for (i, f) in self.spec.flows.iter().enumerate() {
            if f.name.trim().is_empty() {
                self.error(format!("/flows/{i}/name"), "the flow has an empty name".into());
            }
            if f.steps.len() < 2 {
                self.error(
                    format!("/flows/{i}/steps"),
                    format!("flow `{}` needs at least two steps", f.name),
                );
            }
            for (j, step) in f.steps.iter().enumerate() {
                if let Some(message) = Self::reference(ids, step, Named::Node) {
                    self.error(format!("/flows/{i}/steps/{j}"), message);
                }
            }
            for (j, pair) in f.steps.windows(2).enumerate() {
                let (a, b) = (pair[0].as_str(), pair[1].as_str());
                if ids.get(a) == Some(&Named::Node)
                    && ids.get(b) == Some(&Named::Node)
                    && !edges.contains(&(a, b))
                {
                    self.error(
                        format!("/flows/{i}/steps/{}", j + 1),
                        format!(
                            "flow `{}` goes from `{a}` to `{b}`, but no edge goes from `{a}` to `{b}`",
                            f.name
                        ),
                    );
                }
            }
        }
    }

    fn hints(&mut self, ids: &BTreeMap<&str, Named>) {
        let h = &self.spec.hints;
        for (field, list) in [("first", &h.first), ("last", &h.last)] {
            for (j, id) in list.iter().enumerate() {
                if let Some(message) = Self::reference(ids, id, Named::Node) {
                    self.error(format!("/hints/{field}/{j}"), message);
                }
            }
        }
        for id in h.first.iter().filter(|id| h.last.contains(id)) {
            self.error("/hints".into(), format!("`{id}` is hinted both first and last"));
        }
        for (field, groups) in [("sameLayer", &h.same_layer), ("order", &h.order)] {
            for (j, group) in groups.iter().enumerate() {
                if group.len() < 2 {
                    self.error(
                        format!("/hints/{field}/{j}"),
                        "a group needs at least two nodes".into(),
                    );
                }
                for (k, id) in group.iter().enumerate() {
                    if let Some(message) = Self::reference(ids, id, Named::Node) {
                        self.error(format!("/hints/{field}/{j}/{k}"), message);
                    }
                }
            }
        }
    }

    /// Checks that `id` names a thing of the wanted sort; the message when it does not.
    fn reference(ids: &BTreeMap<&str, Named>, id: &str, wanted: Named) -> Option<String> {
        let what = match wanted {
            Named::Node => "node",
            Named::Frame => "frame",
        };
        match ids.get(id) {
            Some(&named) if named == wanted => None,
            Some(_) => Some(format!(
                "`{id}` is a {}, not a {what}",
                if wanted == Named::Node { "frame" } else { "node" }
            )),
            None => {
                let candidates = ids.iter().filter(|(_, n)| **n == wanted).map(|(k, _)| *k);
                Some(match nearest(id, candidates) {
                    Some(near) => format!("no {what} has the id `{id}`; did you mean `{near}`?"),
                    None => format!("no {what} has the id `{id}`"),
                })
            }
        }
    }
}

/// Lowercase letters and digits in groups joined by single hyphens.
fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|part| {
            !part.is_empty() && part.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// The closest candidate by edit distance, when it is close enough to be a typo:
/// at most a third of the id's length, and at least one edit.
fn nearest<'a>(id: &str, candidates: impl Iterator<Item = &'a str>) -> Option<&'a str> {
    let limit = (id.chars().count() / 3).max(1);
    candidates
        .map(|c| (edit_distance(id, c), c))
        .filter(|(d, _)| *d <= limit)
        .min()
        .map(|(_, c)| c)
}

/// Levenshtein distance over characters.
fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, &cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = if ca == cb {
                diagonal
            } else {
                1 + diagonal.min(above).min(row[j])
            };
            diagonal = above;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_lowercase_words_joined_by_single_hyphens() {
        for good in ["api", "vector-store", "db2", "a-b-c"] {
            assert!(is_valid_id(good), "{good}");
        }
        for bad in ["", "API", "vector_store", "-api", "api-", "a--b", "a b"] {
            assert!(!is_valid_id(bad), "{bad}");
        }
    }

    #[test]
    fn edit_distance_counts_single_character_edits() {
        assert_eq!(edit_distance("postgres", "postgres"), 0);
        assert_eq!(edit_distance("datbase", "database"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
    }

    #[test]
    fn nearest_offers_only_close_candidates() {
        assert_eq!(nearest("vpcx", ["vpc", "redis"].into_iter()), Some("vpc"));
        assert_eq!(nearest("queue", ["postgres", "api"].into_iter()), None);
    }
}
