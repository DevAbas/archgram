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
//! every signal it sent has arrived, its border drawn over `motion.hop-min`;
//! the last step's cards stay lit until their border has closed.
//!
//! A flow that stops is refused at its last step: `motion.fade` after the
//! signal arrives, the refusal travels back along every line the flow took,
//! `motion.refusal-hop` for each step, to the flow's first step. The
//! refusing card stays refused until a later flow lights it, the first
//! step's until the next flow starts. Flows play one after another, and
//! `motion.rest` passes before the next one and before the cycle repeats.

use std::collections::BTreeMap;

use crate::spec::Spec;
use crate::tokens::{
    MOTION_EASE, MOTION_FADE_MS, MOTION_HOP_GAP_MS, MOTION_HOP_MAX_MS, MOTION_HOP_MIN_MS,
    MOTION_REFUSAL_HOP_MS, MOTION_REST_MS, MOTION_SPEED,
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

/// What a lit card's border says (DESIGN.md, Components: Signal, Refusal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    /// A signal passes the card.
    Pass,
    /// A step refused the flow: the refusing card, or the card the refusal
    /// came back to.
    Refused,
}

/// Where a lit card's border starts, by an edge's index in the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Entry {
    /// At the arrowhead of the edge whose signal reached the card.
    Arrow(usize),
    /// Opposite the start of the edge the card's signal leaves by, closing
    /// there as it leaves: a flow's first card, which no arrow reaches.
    Leave(usize),
    /// At the start of the edge a refusal comes back along.
    Back(usize),
}

/// A time a card is lit: from `start`, as its border is drawn over `trace`,
/// until `end`, after which it fades out over `motion.fade`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lit {
    pub node: usize,
    pub start: u32,
    pub end: u32,
    pub state: State,
    pub entry: Entry,
    /// How long its border takes to close.
    pub trace: u32,
    /// When the card's own signal leaves, if it sends one.
    pub leaves: Option<u32>,
}

/// A refusal travelling back along one edge, against its direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Return {
    pub edge: usize,
    pub start: u32,
    pub end: u32,
}

/// The ✕ on the edge into a refusing card: shown from the signal's arrival
/// until the card is lit again (or the cycle ends); its arrowhead turns the
/// refusal colour as the refusal leaves, at `back`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refusal {
    pub edge: usize,
    pub start: u32,
    pub back: u32,
    pub end: u32,
}

/// The whole cycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timeline {
    /// How long the cycle lasts before it repeats.
    pub period: u32,
    pub hops: Vec<Hop>,
    /// Each card's lit times, by node then time. Times of one state never
    /// come closer than two fades, so one fade ends before the next begins;
    /// a refused time may end as a passing one starts.
    pub lit: Vec<Lit>,
    pub returns: Vec<Return>,
    pub refusals: Vec<Refusal>,
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

/// `motion.hop-gap`, in whole milliseconds: how long a signal waits at its
/// node, and how long what it reaches stays in its colour.
#[must_use]
pub fn hop_gap() -> u32 {
    ms(MOTION_HOP_GAP_MS)
}

/// `motion.hop-min`, in whole milliseconds: how long a lit card's border
/// takes to close, so it never moves faster than a signal.
#[must_use]
pub fn trace() -> u32 {
    ms(MOTION_HOP_MIN_MS)
}

/// How long a hop along an edge `length` pixels long lasts.
#[must_use]
pub fn hop_duration(length: f64) -> u32 {
    ms((1000.0 * length / MOTION_SPEED).clamp(MOTION_HOP_MIN_MS, MOTION_HOP_MAX_MS))
}

/// When a hop's signal is `fraction` of the way along its edge, in whole
/// milliseconds: its progress is eased by `motion.ease`, a cubic Bézier
/// from time to distance, as SMIL's `keySplines` draw it. The curve is
/// solved by halving, with only `+ - * /`, so the moment is the same on
/// every machine.
#[must_use]
pub fn reached(hop: &Hop, fraction: f64) -> u32 {
    let [x1, y1, x2, y2] = MOTION_EASE;
    let bezier = |a: f64, b: f64, s: f64| {
        let u = 1.0 - s;
        3.0 * u * u * s * a + 3.0 * u * s * s * b + s * s * s
    };
    let want = fraction.clamp(0.0, 1.0);
    // Distance never falls back as time goes on: halve toward the want.
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..60 {
        let mid = f64::midpoint(lo, hi);
        if bezier(y1, y2, mid) < want {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let time = bezier(x1, x2, f64::midpoint(lo, hi));
    hop.start + ms(time * f64::from(hop.end - hop.start)).min(hop.end - hop.start)
}

/// The timing of a validated spec's flows, with `lengths` each edge's drawn
/// length in spec order. `None` when the spec has no flows.
#[must_use]
pub fn timeline(spec: &Spec, lengths: &[f64]) -> Option<Timeline> {
    if spec.flows.is_empty() {
        return None;
    }
    let (node, edge) = indices(spec);
    let mut cycle = Cycle::default();
    // The first card fades in from the cycle's start.
    let mut t0 = fade();
    for flow in &spec.flows {
        let stages: Vec<Vec<usize>> = flow
            .steps
            .iter()
            .map(|s| s.nodes().iter().map(|id| node[id.as_str()]).collect())
            .collect();
        let moves: Vec<Vec<Move>> = stages
            .windows(2)
            .map(|pair| {
                pair[0]
                    .iter()
                    .flat_map(|&a| pair[1].iter().map(move |&b| (a, b)))
                    .filter_map(|(a, b)| edge.get(&(a, b)).map(|&e| (a, b, e)))
                    .collect()
            })
            .collect();
        let refused = flow.stop.as_ref().map(|id| node[id.as_str()]);
        let mut end = cycle.play(&stages[0], &moves, lengths, refused, t0);
        if refused.is_some() {
            end = cycle.refuse(&stages[0], &moves, end);
        }
        t0 = end + ms(MOTION_REST_MS);
        for i in cycle.until_next.drain(..) {
            cycle.lit[i].end = t0;
        }
    }
    Some(cycle.settle(t0))
}

/// One move of a step: from a node, to a node, along an edge.
type Move = (usize, usize, usize);

/// The cycle as it is laid out, flow by flow.
#[derive(Default)]
struct Cycle {
    hops: Vec<Hop>,
    lit: Vec<Lit>,
    returns: Vec<Return>,
    refusals: Vec<Refusal>,
    /// Refused times that end when the card is next lit as passing.
    until_lit: Vec<usize>,
    /// Refused times that end when the next flow starts.
    until_next: Vec<usize>,
}

impl Cycle {
    /// Plays a flow from `t0`: its hops and lit times, the last step's
    /// refused when `refused` names it. Returns when its last step is reached.
    fn play(
        &mut self,
        first: &[usize],
        moves: &[Vec<Move>],
        lengths: &[f64],
        refused: Option<usize>,
        t0: u32,
    ) -> u32 {
        let (gap, trace) = (ms(MOTION_HOP_GAP_MS), trace());
        // When each node of the current step was reached, and by which edge.
        let mut reached: BTreeMap<usize, (u32, Option<usize>)> =
            first.iter().map(|&n| (n, (t0, None))).collect();
        let mut end = t0;
        for step in moves {
            let mut arrive: BTreeMap<usize, (u32, Option<usize>)> = BTreeMap::new();
            for &(a, b, e) in step {
                let at = reached[&a].0 + gap + hop_duration(lengths[e]);
                let slot = arrive.entry(b).or_insert((at, Some(e)));
                slot.0 = slot.0.max(at);
            }
            for &(a, b, e) in step {
                self.hops.push(Hop {
                    edge: e,
                    from: a,
                    start: reached[&a].0 + gap,
                    end: arrive[&b].0,
                });
            }
            for (&a, &(at, by)) in &reached {
                let sent = step.iter().filter(|m| m.0 == a);
                let Some(leave) = sent.clone().next().map(|m| m.2) else {
                    continue;
                };
                self.lit.push(Lit {
                    node: a,
                    start: at,
                    end: sent.map(|m| arrive[&m.1].0).max().unwrap_or(at + gap),
                    state: State::Pass,
                    // A flow's first card closes its border as its signal leaves.
                    entry: by.map_or(Entry::Leave(leave), Entry::Arrow),
                    trace: if by.is_some() { trace } else { gap },
                    leaves: Some(at + gap),
                });
            }
            end = arrive.values().map(|r| r.0).max().unwrap_or(end);
            reached = arrive;
        }
        for (&n, &(at, by)) in &reached {
            let Some(by) = by else { continue };
            let state = if refused == Some(n) {
                self.until_lit.push(self.lit.len());
                State::Refused
            } else {
                State::Pass
            };
            self.lit.push(Lit {
                node: n,
                start: at,
                // The last card stays lit until its border has closed.
                end: at + gap.max(trace),
                state,
                entry: Entry::Arrow(by),
                trace,
                leaves: None,
            });
        }
        end
    }

    /// A refusal from the last step, reached at `end`, back along every line
    /// the flow took, the last step first, to its first step. Returns when
    /// it is back.
    fn refuse(&mut self, first: &[usize], moves: &[Vec<Move>], end: u32) -> u32 {
        let back = end + fade();
        let hop = ms(MOTION_REFUSAL_HOP_MS);
        let mut at = back;
        for step in moves.iter().rev() {
            for &(_, _, e) in step {
                self.returns.push(Return {
                    edge: e,
                    start: at,
                    end: at + hop,
                });
            }
            at += hop;
        }
        for &(_, _, e) in moves.last().into_iter().flatten() {
            self.refusals.push(Refusal {
                edge: e,
                start: end,
                back,
                end,
            });
        }
        for &n in first {
            let Some(&(_, _, e)) = moves.first().and_then(|s| s.iter().find(|m| m.0 == n)) else {
                continue;
            };
            self.until_next.push(self.lit.len());
            self.lit.push(Lit {
                node: n,
                start: at,
                end: at,
                state: State::Refused,
                entry: Entry::Back(e),
                trace: trace(),
                leaves: None,
            });
        }
        at
    }

    /// The cycle, `period` long: a refusing card stays refused until a
    /// later flow lights it, or until the cycle ends.
    fn settle(mut self, period: u32) -> Timeline {
        let last = period - fade();
        for &i in &self.until_lit {
            let Lit { node, start, .. } = self.lit[i];
            let next = self
                .lit
                .iter()
                .filter(|l| l.node == node && l.state == State::Pass && l.start > start)
                .map(|l| l.start)
                .min();
            self.lit[i].end = next.unwrap_or(last);
        }
        for l in &mut self.lit {
            l.end = l.end.min(last);
        }
        for r in &mut self.refusals {
            let marked = self.lit.iter().find(|l| {
                l.state == State::Refused && l.start == r.start && l.entry == Entry::Arrow(r.edge)
            });
            r.end = marked.map_or(r.start, |l| l.end);
        }
        Timeline {
            period,
            hops: self.hops,
            lit: merged(self.lit, 2 * fade()),
            returns: self.returns,
            refusals: self.refusals,
        }
    }
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

/// Each node's lit times in order, joined where two of one state come
/// closer than `gap`: the first's border stays, and it leaves when the
/// later one does.
fn merged(mut lit: Vec<Lit>, gap: u32) -> Vec<Lit> {
    lit.sort_by_key(|l| (l.node, l.start, l.state, l.end));
    let mut out: Vec<Lit> = Vec::with_capacity(lit.len());
    for l in lit {
        let near = out
            .iter_mut()
            .rev()
            .find(|o| o.node == l.node && o.state == l.state);
        match near {
            Some(last) if l.start <= last.end + gap => {
                last.end = last.end.max(l.end);
                last.leaves = l.leaves.or(last.leaves);
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
        let spans: Vec<_> = tl.lit.iter().map(|l| (l.node, l.start, l.end)).collect();
        // The last card stays lit until its border has closed.
        assert_eq!(spans, [(0, f, b), (1, b, c), (2, c, c + trace())]);
        // The first card's border closes where its signal leaves, as it
        // leaves; every other starts at the arrowhead that reached it.
        let borders: Vec<_> = tl
            .lit
            .iter()
            .map(|l| (l.entry, l.trace, l.leaves))
            .collect();
        assert_eq!(
            borders,
            [
                (Entry::Leave(0), gap, Some(f + gap)),
                (Entry::Arrow(0), trace(), Some(b + gap)),
                (Entry::Arrow(1), trace(), None),
            ]
        );
        assert!(tl.lit.iter().all(|l| l.state == State::Pass));
        assert!(tl.returns.is_empty() && tl.refusals.is_empty());
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
            (tl.lit[0].node, tl.lit[0].start, tl.lit[0].end),
            (0, f, f + gap + long)
        );
    }

    #[test]
    fn a_refusal_goes_back_and_holds_until_a_later_flow_passes() {
        let spec = diagram(
            &["a", "b", "c"],
            &[("a", "b"), ("b", "c")],
            r#"[{ "name": "f", "steps": ["a", "b", "c"], "stop": "c" }, { "name": "g", "steps": ["a", "b", "c"] }]"#,
        );
        let tl = timeline(&spec, &[0.0, 0.0]).unwrap();
        let (f, gap, short) = (fade(), ms(MOTION_HOP_GAP_MS), ms(MOTION_HOP_MIN_MS));
        let (rest, back_hop) = (ms(MOTION_REST_MS), ms(MOTION_REFUSAL_HOP_MS));
        let c = f + 2 * (gap + short);
        // The refusal leaves a fade after the signal arrives, and goes back
        // one step at a time, the last line first.
        let back = c + f;
        assert_eq!(
            tl.returns,
            [
                Return {
                    edge: 1,
                    start: back,
                    end: back + back_hop
                },
                Return {
                    edge: 0,
                    start: back + back_hop,
                    end: back + 2 * back_hop
                },
            ]
        );
        // The next flow starts a rest after the refusal is back.
        let g = back + 2 * back_hop + rest;
        assert_eq!(tl.hops[2].start, g + gap);
        let c_again = g + 2 * (gap + short);
        // c is refused until g lights it; a from the refusal's return until g starts.
        let refused: Vec<_> = tl
            .lit
            .iter()
            .filter(|l| l.state == State::Refused)
            .map(|l| (l.node, l.start, l.end, l.entry))
            .collect();
        assert_eq!(
            refused,
            [
                (0, back + 2 * back_hop, g, Entry::Back(0)),
                (2, c, c_again, Entry::Arrow(1)),
            ]
        );
        assert_eq!(
            tl.refusals,
            [Refusal {
                edge: 1,
                start: c,
                back,
                end: c_again
            }]
        );
        assert_eq!(tl.period, c_again + rest);
    }

    #[test]
    fn a_card_never_passed_again_stays_refused_until_the_cycle_ends() {
        let spec = diagram(
            &["a", "b"],
            &[("a", "b")],
            r#"[{ "name": "f", "steps": ["a", "b"], "stop": "b" }]"#,
        );
        let tl = timeline(&spec, &[0.0]).unwrap();
        let b = tl.lit.iter().find(|l| l.node == 1).unwrap();
        assert_eq!((b.state, b.end), (State::Refused, tl.period - fade()));
        assert_eq!(tl.refusals[0].end, tl.period - fade());
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
    fn a_signal_reaches_a_point_on_its_eased_way() {
        let hop = Hop {
            edge: 0,
            from: 0,
            start: 1000,
            end: 2000,
        };
        assert_eq!(reached(&hop, 0.0), 1000);
        assert_eq!(reached(&hop, 1.0), 2000);
        // The easing is symmetric: halfway along at half the time.
        assert_eq!(reached(&hop, 0.5), 1500);
        // Slow to start, so the first tenth takes longer than a tenth.
        assert!(reached(&hop, 0.1) > 1100);
        assert!(reached(&hop, 0.9) < 1900);
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
            if w[0].node == w[1].node && w[0].state == w[1].state {
                assert!(w[1].start > w[0].end + 2 * fade());
            }
        }
    }
}
