//! When each flow's signals move and which cards they light (DESIGN.md,
//! Motion). Pure arithmetic on whole milliseconds, so the same spec keeps
//! the same timing on every machine.
//!
//! A flow is a list of steps, each one node or several reached at once. A
//! signal leaves every node of a step for each node of the next step it has
//! an edge to. It waits `motion.hop-gap` at its node, then travels at
//! `motion.speed`, a hop lasting between `motion.hop-min` and
//! `motion.hop-max`. Signals meeting at one node arrive together, when the
//! slowest does. A card is lit from the moment a signal reaches it until
//! every signal it sent has arrived; the last step's cards stay lit for
//! `motion.hop-gap`. Flows play one after another, and `motion.rest` passes
//! before the cycle repeats.

use std::collections::BTreeMap;

use crate::spec::Spec;
use crate::tokens::{
    MOTION_FADE_MS, MOTION_HOP_GAP_MS, MOTION_HOP_MAX_MS, MOTION_HOP_MIN_MS, MOTION_REST_MS,
    MOTION_SPEED,
};

/// One signal's move along one edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hop {
    /// The edge, by its index in the spec.
    pub edge: usize,
    /// The node it leaves, by its index in the spec.
    pub from: usize,
    pub start: u32,
    pub end: u32,
}

/// A time a card is lit, fully: it fades in over `motion.fade` before
/// `start` and out over `motion.fade` after `end`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lit {
    pub node: usize,
    pub start: u32,
    pub end: u32,
}

/// The whole cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    /// How long the cycle lasts before it repeats.
    pub period: u32,
    pub hops: Vec<Hop>,
    /// Each card's lit times, by node then time, never closer than two fades
    /// so one fade ends before the next begins.
    pub lit: Vec<Lit>,
}

/// A token's milliseconds as a whole number.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn ms(v: f64) -> u32 {
    v.round().max(0.0) as u32
}

/// `motion.fade`, in whole milliseconds.
#[must_use]
pub fn fade() -> u32 {
    ms(MOTION_FADE_MS)
}

/// How long a hop along an edge `length` pixels long lasts.
#[must_use]
pub fn hop_duration(length: f64) -> u32 {
    ms((1000.0 * length / MOTION_SPEED).clamp(MOTION_HOP_MIN_MS, MOTION_HOP_MAX_MS))
}

/// The timing of a validated spec's flows, with `lengths` each edge's drawn
/// length in spec order. `None` when the spec has no flows.
#[must_use]
pub fn timeline(spec: &Spec, lengths: &[f64]) -> Option<Timeline> {
    if spec.flows.is_empty() {
        return None;
    }
    let (node, edge) = indices(spec);
    let (gap, rest, fade) = (ms(MOTION_HOP_GAP_MS), ms(MOTION_REST_MS), fade());
    let mut hops = Vec::new();
    let mut lit = Vec::new();
    // The first card fades in from the cycle's start.
    let mut t0 = fade;
    for flow in &spec.flows {
        let stages: Vec<Vec<usize>> = flow
            .steps
            .iter()
            .map(|s| s.nodes().iter().map(|id| node[id.as_str()]).collect())
            .collect();
        // When each node of the current step was reached.
        let mut reached: BTreeMap<usize, u32> = stages[0].iter().map(|&n| (n, t0)).collect();
        let mut end = t0;
        for pair in stages.windows(2) {
            let (from, to) = (&pair[0], &pair[1]);
            let moves: Vec<(usize, usize, usize)> = from
                .iter()
                .flat_map(|&a| to.iter().map(move |&b| (a, b)))
                .filter_map(|(a, b)| edge.get(&(a, b)).map(|&e| (a, b, e)))
                .collect();
            let mut arrive: BTreeMap<usize, u32> = BTreeMap::new();
            for &(a, b, e) in &moves {
                let at = reached[&a] + gap + hop_duration(lengths[e]);
                let slot = arrive.entry(b).or_insert(at);
                *slot = (*slot).max(at);
            }
            for &(a, b, e) in &moves {
                hops.push(Hop {
                    edge: e,
                    from: a,
                    start: reached[&a] + gap,
                    end: arrive[&b],
                });
            }
            for &a in from {
                let sent = moves
                    .iter()
                    .filter(|m| m.0 == a)
                    .map(|m| arrive[&m.1])
                    .max();
                lit.push(Lit {
                    node: a,
                    start: reached[&a],
                    end: sent.unwrap_or(reached[&a] + gap),
                });
            }
            end = arrive.values().copied().max().unwrap_or(end);
            reached = arrive;
        }
        for (&n, &at) in &reached {
            lit.push(Lit {
                node: n,
                start: at,
                end: at + gap,
            });
        }
        t0 = end + rest;
    }
    Some(Timeline {
        period: t0,
        hops,
        lit: merged(lit, 2 * fade),
    })
}

/// Each node's index by its id, and each edge's by its two nodes.
type Indices<'a> = (BTreeMap<&'a str, usize>, BTreeMap<(usize, usize), usize>);

fn indices(spec: &Spec) -> Indices<'_> {
    let node: BTreeMap<&str, usize> = spec
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let edge = spec
        .edges
        .iter()
        .enumerate()
        .map(|(i, e)| ((node[e.from.as_str()], node[e.to.as_str()]), i))
        .collect();
    (node, edge)
}

/// Each edge's step numbers for the still image, in spec order: every move
/// from one step to the next is numbered, counting on from one flow to the
/// next in the order they play; an edge several moves take carries each
/// number, and an edge no flow takes none.
#[must_use]
pub fn step_numbers(spec: &Spec) -> Vec<Vec<u32>> {
    let (node, edge) = indices(spec);
    let mut out = vec![Vec::new(); spec.edges.len()];
    let mut n = 0;
    for flow in &spec.flows {
        for pair in flow.steps.windows(2) {
            n += 1;
            for a in pair[0].nodes() {
                for b in pair[1].nodes() {
                    if let Some(&e) = edge.get(&(node[a.as_str()], node[b.as_str()])) {
                        out[e].push(n);
                    }
                }
            }
        }
    }
    out
}

/// Each node's lit times in order, joined where they come closer than `gap`.
fn merged(mut lit: Vec<Lit>, gap: u32) -> Vec<Lit> {
    lit.sort_by_key(|l| (l.node, l.start, l.end));
    let mut out: Vec<Lit> = Vec::with_capacity(lit.len());
    for l in lit {
        match out.last_mut() {
            Some(last) if last.node == l.node && l.start <= last.end + gap => {
                last.end = last.end.max(l.end);
            }
            _ => out.push(l),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(json: &str) -> Spec {
        crate::parse_spec(json).unwrap()
    }

    fn nodes(ids: &[&str]) -> String {
        ids.iter()
            .map(|id| format!(r#"{{ "id": "{id}", "kind": "service", "label": "{id}" }}"#))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn edges(pairs: &[(&str, &str)]) -> String {
        pairs
            .iter()
            .map(|(a, b)| format!(r#"{{ "from": "{a}", "to": "{b}" }}"#))
            .collect::<Vec<_>>()
            .join(", ")
    }

    fn diagram(ids: &[&str], pairs: &[(&str, &str)], flows: &str) -> Spec {
        spec(&format!(
            r#"{{ "archgram": 1, "title": "t", "description": "d", "nodes": [{}], "edges": [{}], "flows": {flows} }}"#,
            nodes(ids),
            edges(pairs)
        ))
    }

    #[test]
    fn no_flows_no_timeline() {
        let spec = diagram(&["a", "b"], &[("a", "b")], "[]");
        assert_eq!(timeline(&spec, &[100.0]), None);
    }

    #[test]
    fn a_hop_lasts_in_proportion_to_its_length_within_bounds() {
        assert_eq!(hop_duration(0.0), ms(MOTION_HOP_MIN_MS));
        assert_eq!(hop_duration(1e6), ms(MOTION_HOP_MAX_MS));
        let mid = f64::midpoint(MOTION_HOP_MIN_MS, MOTION_HOP_MAX_MS);
        assert_eq!(hop_duration(mid * MOTION_SPEED / 1000.0), ms(mid));
    }

    #[test]
    fn a_chain_lights_each_card_until_its_signal_arrives() {
        let spec = diagram(
            &["a", "b", "c"],
            &[("a", "b"), ("b", "c")],
            r#"[{ "name": "f", "steps": ["a", "b", "c"] }]"#,
        );
        let tl = timeline(&spec, &[0.0, 1e6]).unwrap();
        let (f, gap) = (fade(), ms(MOTION_HOP_GAP_MS));
        let (short, long) = (ms(MOTION_HOP_MIN_MS), ms(MOTION_HOP_MAX_MS));
        let b = f + gap + short;
        let c = b + gap + long;
        assert_eq!(
            tl.hops,
            [
                Hop {
                    edge: 0,
                    from: 0,
                    start: f + gap,
                    end: b
                },
                Hop {
                    edge: 1,
                    from: 1,
                    start: b + gap,
                    end: c
                },
            ]
        );
        assert_eq!(
            tl.lit,
            [
                Lit {
                    node: 0,
                    start: f,
                    end: b
                },
                Lit {
                    node: 1,
                    start: b,
                    end: c
                },
                Lit {
                    node: 2,
                    start: c,
                    end: c + gap
                },
            ]
        );
        assert_eq!(tl.period, c + ms(MOTION_REST_MS));
    }

    #[test]
    fn branches_leave_together_and_meet_together() {
        let spec = diagram(
            &["a", "x", "y", "z"],
            &[("a", "x"), ("a", "y"), ("x", "z"), ("y", "z")],
            r#"[{ "name": "f", "steps": ["a", ["x", "y"], "z"] }]"#,
        );
        let tl = timeline(&spec, &[0.0, 1e6, 1e6, 0.0]).unwrap();
        let (f, gap) = (fade(), ms(MOTION_HOP_GAP_MS));
        let (short, long) = (ms(MOTION_HOP_MIN_MS), ms(MOTION_HOP_MAX_MS));
        // Both branches start at once; each keeps its own length.
        assert_eq!(tl.hops[0].start, tl.hops[1].start);
        assert_eq!(tl.hops[0].end, f + gap + short);
        assert_eq!(tl.hops[1].end, f + gap + long);
        // They meet at z when the slower one gets there.
        let meet = f + gap + long + gap + short;
        assert_eq!(tl.hops[2].end, meet);
        assert_eq!(tl.hops[3].end, meet);
        // a stays lit until both of its signals have arrived.
        assert_eq!(
            tl.lit[0],
            Lit {
                node: 0,
                start: f,
                end: f + gap + long
            }
        );
    }

    #[test]
    fn steps_are_numbered_across_flows() {
        let spec = diagram(
            &["a", "x", "y", "z"],
            &[("a", "x"), ("a", "y"), ("x", "z"), ("y", "z"), ("z", "a")],
            r#"[{ "name": "f", "steps": ["a", ["x", "y"], "z"] }, { "name": "g", "steps": ["z", "a", "x"] }]"#,
        );
        assert_eq!(
            step_numbers(&spec),
            [vec![1, 4], vec![1], vec![2], vec![2], vec![3]]
        );
    }

    #[test]
    fn flows_play_in_turn_and_close_lit_times_join() {
        let spec = diagram(
            &["a", "b"],
            &[("a", "b")],
            r#"[{ "name": "f", "steps": ["a", "b"] }, { "name": "g", "steps": ["a", "b"] }]"#,
        );
        let tl = timeline(&spec, &[0.0]).unwrap();
        let first_end = tl.hops[0].end;
        assert_eq!(
            tl.hops[1].start,
            first_end + ms(MOTION_REST_MS) + ms(MOTION_HOP_GAP_MS)
        );
        // Every node's lit times are apart by more than two fades.
        for w in tl.lit.windows(2) {
            if w[0].node == w[1].node {
                assert!(w[1].start > w[0].end + 2 * fade());
            }
        }
    }
}
