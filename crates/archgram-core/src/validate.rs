//! The rules a spec must follow beyond its types (docs/SPEC.md, Validation).
//! Every problem is collected, in the order the spec lists things, so one
//! run reports them all and the same spec always reports them the same way.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::SpecError;
use crate::spec::{Flow, Sources, Spec, Step};

use crate::tokens::PALETTES;

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
    v.texts();
    let ids = v.ids();
    v.nodes(&ids);
    v.frames(&ids);
    v.edges(&ids);
    v.sources();
    v.flows(&ids);
    v.hints(&ids);
    // The hints are held against the edges' order only when every edge and
    // hint names a node and no edge loops: the check follows the edges.
    if follows_edges(spec, &ids) {
        v.errors.extend(crate::layout::contradicted_hints(spec));
    }
    v.errors
}

/// Whether every edge joins two nodes and every hint names a node.
fn follows_edges(spec: &Spec, ids: &BTreeMap<&str, Named>) -> bool {
    let is_node = |id: &String| ids.get(id.as_str()) == Some(&Named::Node);
    let h = &spec.hints;
    spec.edges
        .iter()
        .all(|e| e.from != e.to && is_node(&e.from) && is_node(&e.to))
        && h.first.iter().chain(&h.last).all(is_node)
        && h.same_layer.iter().chain(&h.order).flatten().all(is_node)
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

    /// Every text the drawing carries holds only characters XML allows
    /// (XML 1.0, section 2.2, Characters): an SVG with any other one is
    /// not XML, and a browser shows nothing.
    fn texts(&mut self) {
        let s = self.spec;
        let mut texts: Vec<(String, &str)> = vec![
            ("/title".into(), &s.title),
            ("/description".into(), &s.description),
        ];
        for (i, n) in s.nodes.iter().enumerate() {
            texts.push((format!("/nodes/{i}/label"), &n.label));
            if let Some(note) = &n.note {
                texts.push((format!("/nodes/{i}/note"), note));
            }
        }
        for (i, f) in s.frames.iter().enumerate() {
            texts.push((format!("/frames/{i}/label"), &f.label));
        }
        for (i, e) in s.edges.iter().enumerate() {
            if let Some(label) = &e.label {
                texts.push((format!("/edges/{i}/label"), label));
            }
        }
        for (i, f) in s.flows.iter().enumerate() {
            texts.push((format!("/flows/{i}/name"), &f.name));
        }
        for (pointer, text) in texts {
            if let Some(c) = text.chars().find(|&c| !is_xml_char(c)) {
                self.error(
                    pointer,
                    format!(
                        "the text holds U+{:04X}, which XML does not allow; an SVG with it does not open",
                        u32::from(c)
                    ),
                );
            }
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
                && (tech.is_empty()
                    || !tech
                        .bytes()
                        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'))
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
            let has_node = self
                .spec
                .nodes
                .iter()
                .any(|n| n.frame.as_deref() == Some(&f.id));
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

    /// Each node's and edge's `source`, checked for its form; whether the
    /// code is there is for the caller, who can read it (`sources::check`).
    fn sources(&mut self) {
        let s = self.spec;
        let nodes = s
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (format!("/nodes/{i}/source"), &n.source));
        let edges = s
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| (format!("/edges/{i}/source"), &e.source));
        for (pointer, sources) in nodes.chain(edges) {
            match sources {
                None => {}
                Some(Sources::One(source)) => {
                    if let Some(message) = crate::sources::form(source) {
                        self.error(pointer, message);
                    }
                }
                Some(Sources::Many(list)) if list.is_empty() => self.error(
                    pointer,
                    "the list names no source; leave `source` out instead".into(),
                ),
                Some(Sources::Many(list)) => {
                    for (j, source) in list.iter().enumerate() {
                        if let Some(message) = crate::sources::form(source) {
                            self.error(format!("{pointer}/{j}"), message);
                        }
                    }
                }
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
                self.error(
                    format!("/flows/{i}/name"),
                    "the flow has an empty name".into(),
                );
            }
            if f.steps.len() < 2 {
                self.error(
                    format!("/flows/{i}/steps"),
                    format!("flow `{}` needs at least two steps", f.name),
                );
            }
            self.flow_steps(i, f, ids);
            self.flow_moves(i, f, ids, &edges);
            self.flow_stop(i, f, ids);
        }
    }

    /// A flow stops at a node, the single node of its last step.
    fn flow_stop(&mut self, i: usize, f: &Flow, ids: &BTreeMap<&str, Named>) {
        let Some(stop) = &f.stop else { return };
        if let Some(message) = Self::reference(ids, stop, Named::Node) {
            self.error(format!("/flows/{i}/stop"), message);
            return;
        }
        let message = match f.steps.last().map(Step::nodes) {
            Some([last]) if last == stop => return,
            Some(nodes) if nodes.contains(stop) => format!(
                "flow `{}` stops at `{stop}`, but its last step is a branch: a flow stops at a single node",
                f.name
            ),
            _ => format!(
                "flow `{}` stops at `{stop}`, which is not its last step: a flow stops at its last step",
                f.name
            ),
        };
        self.error(format!("/flows/{i}/stop"), message);
    }

    /// Each step lists nodes, each once.
    fn flow_steps(&mut self, i: usize, f: &Flow, ids: &BTreeMap<&str, Named>) {
        for (j, step) in f.steps.iter().enumerate() {
            let nodes = step.nodes();
            if nodes.is_empty() {
                self.error(
                    step_at(i, f, j, 0),
                    format!("flow `{}` has a step with no nodes", f.name),
                );
            }
            for (k, id) in nodes.iter().enumerate() {
                if let Some(message) = Self::reference(ids, id, Named::Node) {
                    self.error(step_at(i, f, j, k), message);
                } else if nodes[..k].contains(id) {
                    self.error(
                        step_at(i, f, j, k),
                        format!("flow `{}` lists `{id}` twice in one step", f.name),
                    );
                }
            }
        }
    }

    /// Every node of a step is reached by an edge from the step before it,
    /// and every branch of a step leads on to the step after it.
    fn flow_moves(
        &mut self,
        i: usize,
        f: &Flow,
        ids: &BTreeMap<&str, Named>,
        edges: &BTreeSet<(&str, &str)>,
    ) {
        let is_node = |id: &str| ids.get(id) == Some(&Named::Node);
        let first = |list: &[String], k: usize| !list[..k].contains(&list[k]);
        let joined = |list: &[String]| {
            list.iter()
                .map(|id| format!("`{id}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        for j in 0..f.steps.len().saturating_sub(1) {
            let (from, to) = (f.steps[j].nodes(), f.steps[j + 1].nodes());
            // An empty step is reported already, and a repeat once.
            if from.is_empty() || to.is_empty() {
                continue;
            }
            // Every node of a step is reached from the step before it...
            let reported = self.errors.len();
            for (k, b) in to
                .iter()
                .enumerate()
                .filter(|&(k, b)| is_node(b) && first(to, k))
            {
                if from
                    .iter()
                    .filter(|a| is_node(a))
                    .any(|a| edges.contains(&(a.as_str(), b.as_str())))
                {
                    continue;
                }
                let message = match from {
                    [a] => format!(
                        "flow `{}` goes from `{a}` to `{b}`, but no edge goes from `{a}` to `{b}`",
                        f.name
                    ),
                    _ => format!(
                        "flow `{}` reaches `{b}` from none of {}: no edge goes from any of them to `{b}`",
                        f.name,
                        joined(from)
                    ),
                };
                self.error(step_at(i, f, j + 1, k), message);
            }
            // ...and every branch of a step leads on to the next one; a
            // step reached from none of them says so once, above.
            if from.len() < 2 || self.errors.len() > reported {
                continue;
            }
            for (k, a) in from
                .iter()
                .enumerate()
                .filter(|&(k, a)| is_node(a) && first(from, k))
            {
                if to.iter().any(|b| edges.contains(&(a.as_str(), b.as_str()))) {
                    continue;
                }
                self.error(
                    step_at(i, f, j, k),
                    format!(
                        "flow `{}` leaves `{a}` for {}, but no edge goes from `{a}` to any of them",
                        f.name,
                        joined(to)
                    ),
                );
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
            self.error(
                "/hints".into(),
                format!("`{id}` is hinted both first and last"),
            );
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
        // An order hint sorts nodes side by side; frames keep their nodes
        // together, so the nodes of one hint must share a frame.
        for (j, group) in h.order.iter().enumerate() {
            let frame_of = |id: &String| {
                self.spec
                    .nodes
                    .iter()
                    .find(|n| &n.id == id)
                    .map(|n| n.frame.clone())
            };
            let frames: Vec<Option<String>> = group.iter().filter_map(frame_of).collect();
            if frames.windows(2).any(|w| w[0] != w[1]) {
                self.error(
                    format!("/hints/order/{j}"),
                    "the nodes of an order hint must share a frame, or all have none".into(),
                );
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
                if wanted == Named::Node {
                    "frame"
                } else {
                    "node"
                }
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

/// A character XML 1.0 allows in a document (section 2.2, the `Char`
/// production): tab, line feed, carriage return, and every other character
/// from U+0020 on except U+FFFE and U+FFFF. A Rust `char` is never a
/// surrogate, the one other range the production leaves out.
fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r') || (c >= ' ' && c != '\u{FFFE}' && c != '\u{FFFF}')
}

/// Lowercase letters and digits in groups joined by single hyphens.
fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.split('-').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        })
}

/// The closest candidate by edit distance, when it is close enough to be a typo:
/// at most a third of the id's length, and at least one edit.
/// Each `tech` names a logo in `logos` (docs/SPEC.md, Validation); the
/// message suggests the nearest slugs. With no logos at all nothing is
/// checked: the build that draws none cannot tell.
pub fn logos(spec: &Spec, logos: &dyn crate::logos::Logos) -> Vec<SpecError> {
    let slugs = logos.slugs();
    if slugs.is_empty() {
        return Vec::new();
    }
    let mut errors = Vec::new();
    for (i, node) in spec.nodes.iter().enumerate() {
        let Some(tech) = &node.tech else { continue };
        if logos.path(tech).is_some() {
            continue;
        }
        let close = nearest_few(tech, slugs.iter().copied(), 3);
        let hint = match close.as_slice() {
            [] => String::new(),
            [one] => format!("; did you mean `{one}`?"),
            [rest @ .., last] => format!(
                "; did you mean {} or `{last}`?",
                rest.iter()
                    .map(|s| format!("`{s}`"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        };
        errors.push(SpecError::at(
            format!("/nodes/{i}/tech"),
            format!("`{tech}` is not a logo archgram carries{hint}"),
        ));
    }
    errors
}

/// Where node `k` of step `j` of flow `i` is: the step itself, or its place
/// in the step's list.
fn step_at(i: usize, f: &Flow, j: usize, k: usize) -> String {
    match &f.steps[j] {
        Step::One(_) => format!("/flows/{i}/steps/{j}"),
        Step::Many(_) => format!("/flows/{i}/steps/{j}/{k}"),
    }
}

/// Up to `count` candidates close to `id`, nearest first, ties in the
/// candidates' order.
pub(crate) fn nearest_few<'a>(
    id: &str,
    candidates: impl Iterator<Item = &'a str>,
    count: usize,
) -> Vec<&'a str> {
    let limit = (id.chars().count() / 3).max(1);
    let mut close: Vec<(usize, &str)> = candidates
        .map(|c| (edit_distance(id, c), c))
        .filter(|(d, _)| *d <= limit)
        .collect();
    close.sort_unstable();
    close.into_iter().take(count).map(|(_, c)| c).collect()
}

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
