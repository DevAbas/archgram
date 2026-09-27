//! Coordinates across the layers (ARCHITECTURE.md, Layout, step 4), after
//! Brandes and Köpf (2001), "Fast and Simple Horizontal Coordinate
//! Assignment".
//!
//! Each vertex is aligned with a median neighbour in the layer before it,
//! forming blocks that share one coordinate, so edges run straight wherever
//! they can. This is done four times, looking up or down the layers and
//! from either end of them; the four results are aligned to the narrowest
//! and each vertex takes the mean of its two middle values.
//!
//! Where the paper places blocks through classes and shifts, this places
//! them with a longest-path pass over the graph of blocks, then a second
//! pass that pulls each block towards the blocks after it. The class-shift
//! step as published could misplace classes (Brandes, Walter and Zink,
//! "Erratum: Fast and Simple Horizontal Coordinate Assignment", 2020); the
//! block graph gives the same alignments a valid, compact placement and is
//! easy to check.
//!
//! Vertices have extents across the layers on either side of their anchor
//! (a card's height when the flow runs right, its width when it runs down);
//! the two sides differ for a stack of several instances, whose anchor is
//! its front card. Dummy vertices, the bends of long edges, have none.
//! Neighbours in a layer are kept apart by the facing extents plus a gap:
//! the node gap between two cards, the edge gap when either is a dummy.

// Names follow the paper's notation: vertices u, v, w; layers l; coordinates x.
#![allow(clippy::many_single_char_names)]

use std::collections::{BTreeMap, BTreeSet};

/// What a vertex is, for the space it keeps from its neighbours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A node's card.
    Card,
    /// A long edge's bend, or its label.
    Dummy,
    /// A frame's first border in a layer.
    First(usize),
    /// A frame's last border in a layer.
    Last(usize),
}

/// What `coordinates` needs to know about the vertices.
pub struct Vertices<'a> {
    /// Extent before the anchor, towards the start of the layer.
    pub lo: &'a [f64],
    /// Extent after the anchor, towards the end of the layer.
    pub hi: &'a [f64],
    pub kind: &'a [Kind],
    pub node_gap: f64,
    pub edge_gap: f64,
    /// Each frame's padding after its first border and before its last.
    pub pad_lo: &'a [f64],
    pub pad_hi: &'a [f64],
    /// For a border vertex, the same border in the layer before and after.
    pub border_up: &'a [Option<usize>],
    pub border_down: &'a [Option<usize>],
    /// Further least distances: `(a, b, d)` keeps `b` at least `d` after `a`.
    pub extra: &'a [(usize, usize, f64)],
}

impl Vertices<'_> {
    /// The least distance between the anchors of neighbours `a` and `b`
    /// when `a` comes first. Not symmetric: `a`'s far side faces `b`'s near
    /// side. Inside a frame its padding keeps the borders off what it holds;
    /// elsewhere the node gap, or the edge gap next to a dummy.
    fn separation(&self, a: usize, b: usize) -> f64 {
        match (self.kind[a], self.kind[b]) {
            (Kind::First(f), Kind::Last(g)) if f == g => self.pad_lo[f] + self.pad_hi[f],
            (Kind::First(f), _) => self.pad_lo[f] + self.lo[b],
            (_, Kind::Last(f)) => self.hi[a] + self.pad_hi[f],
            (ka, kb) => {
                let gap = if ka == Kind::Dummy || kb == Kind::Dummy {
                    self.edge_gap
                } else {
                    self.node_gap
                };
                self.hi[a] + self.lo[b] + gap
            }
        }
    }

    fn is_border(&self, v: usize) -> bool {
        matches!(self.kind[v], Kind::First(_) | Kind::Last(_))
    }
}

/// The centre of every vertex across the layers.
pub fn coordinates(layers: &[Vec<usize>], edges: &[(usize, usize)], v: &Vertices<'_>) -> Vec<f64> {
    let n = v.lo.len();
    let mut up = vec![Vec::new(); n];
    let mut down = vec![Vec::new(); n];
    for &(a, b) in edges {
        down[a].push(b);
        up[b].push(a);
    }
    let dummy: Vec<bool> = v.kind.iter().map(|&k| k == Kind::Dummy).collect();
    let conflicts = type_one_conflicts(layers, &up, &dummy, n);

    let mut results: Vec<(Vec<f64>, bool)> = Vec::new();
    for looking_up in [true, false] {
        for from_left in [true, false] {
            // Put the layers and each layer's order in processing order.
            let mut ls: Vec<Vec<usize>> = layers.to_vec();
            if !looking_up {
                ls.reverse();
            }
            if !from_left {
                for l in &mut ls {
                    l.reverse();
                }
            }
            let before = if looking_up { &up } else { &down };
            let chain = if looking_up { v.border_up } else { v.border_down };
            let (root, _) = align(&ls, before, chain, &conflicts, n);
            let mut x = compact(&ls, &root, v, n, !from_left).unwrap_or_else(|| {
                // A cycle among the blocks: the frames' rules and an alignment
                // disagree. Keep the border chains and let every other vertex
                // stand alone, which the layers' order always allows.
                debug_assert!(false, "the alignment made a cycle among the blocks");
                let alone: Vec<usize> = (0..n).map(|i| if v.is_border(i) { root[i] } else { i }).collect();
                compact(&ls, &alone, v, n, !from_left).expect("borders alone keep the order")
            });
            if !from_left {
                for c in &mut x {
                    *c = -*c;
                }
            }
            results.push((x, from_left));
        }
    }

    // Align all four to the narrowest: left-based ones by their minimum,
    // right-based ones by their maximum.
    let span = |x: &[f64]| {
        let (lo, hi) = (0..n).fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), i| {
            (lo.min(x[i] - v.lo[i]), hi.max(x[i] + v.hi[i]))
        });
        (lo, hi)
    };
    let narrowest = (0..results.len())
        .min_by(|&a, &b| {
            let (la, ha) = span(&results[a].0);
            let (lb, hb) = span(&results[b].0);
            (ha - la).total_cmp(&(hb - lb)).then(a.cmp(&b))
        })
        .expect("four results");
    let (target_lo, target_hi) = span(&results[narrowest].0);
    for (x, from_left) in &mut results {
        let (lo, hi) = span(x);
        let shift = if *from_left {
            target_lo - lo
        } else {
            target_hi - hi
        };
        for c in x.iter_mut() {
            *c += shift;
        }
    }

    (0..n)
        .map(|i| {
            let mut four: Vec<f64> = results.iter().map(|(x, _)| x[i]).collect();
            four.sort_by(f64::total_cmp);
            f64::midpoint(four[1], four[2])
        })
        .collect()
}

/// Type 1 conflicts: an edge between two real-or-mixed vertices that crosses
/// an inner segment (an edge between two dummies). Inner segments win, so
/// long edges stay straight. Returned as unordered pairs.
fn type_one_conflicts(
    layers: &[Vec<usize>],
    up: &[Vec<usize>],
    dummy: &[bool],
    n: usize,
) -> BTreeSet<(usize, usize)> {
    let mut marked = BTreeSet::new();
    let mut pos = vec![0usize; n];
    for layer in layers {
        for (i, &x) in layer.iter().enumerate() {
            pos[x] = i;
        }
    }
    for i in 1..layers.len() {
        let (upper, lower) = (&layers[i - 1], &layers[i]);
        let mut k0 = 0usize;
        let mut scan = 0usize;
        for (l1, &v) in lower.iter().enumerate() {
            let inner = if dummy[v] {
                up[v].iter().copied().find(|&u| dummy[u])
            } else {
                None
            };
            if l1 + 1 == lower.len() || inner.is_some() {
                let k1 = inner.map_or(upper.len().saturating_sub(1), |u| pos[u]);
                for &w in &lower[scan..=l1] {
                    for &u in &up[w] {
                        if (pos[u] < k0 || pos[u] > k1) && !(dummy[u] && dummy[w]) {
                            marked.insert((u.min(w), u.max(w)));
                        }
                    }
                }
                scan = l1 + 1;
                k0 = k1;
            }
        }
    }
    marked
}

/// Vertical alignment for layers given in processing order, with `before[v]`
/// the neighbours of `v` in the layer processed just before its own.
/// Returns each vertex's block root and its successor in the block.
/// A border vertex is aligned with its border in the layer before, `chain`,
/// ahead of everything else: a frame's borders form straight blocks, and no
/// other alignment may cross one.
fn align(
    layers: &[Vec<usize>],
    before: &[Vec<usize>],
    chain: &[Option<usize>],
    conflicts: &BTreeSet<(usize, usize)>,
    n: usize,
) -> (Vec<usize>, Vec<usize>) {
    let mut root: Vec<usize> = (0..n).collect();
    let mut next: Vec<usize> = (0..n).collect();
    let mut pos = vec![0usize; n];
    for layer in layers {
        for (i, &x) in layer.iter().enumerate() {
            pos[x] = i;
        }
    }
    for layer in layers.iter().skip(1) {
        let mut fixed: Vec<(usize, usize)> = Vec::new();
        for &v in layer {
            if let Some(u) = chain.get(v).copied().flatten() {
                next[u] = v;
                root[v] = root[u];
                next[v] = root[v];
                fixed.push((pos[v], pos[u]));
            }
        }
        let crosses = |pv: usize, pu: usize| fixed.iter().any(|&(fv, fu)| (pv < fv) != (pu < fu));
        let mut r: isize = -1;
        for &v in layer {
            let mut ns: Vec<usize> = before[v].clone();
            if ns.is_empty() || chain.get(v).copied().flatten().is_some() {
                continue;
            }
            ns.sort_by_key(|&u| pos[u]);
            let d = ns.len();
            let medians = if d % 2 == 1 {
                vec![d / 2]
            } else {
                vec![d / 2 - 1, d / 2]
            };
            for m in medians {
                if next[v] != v {
                    break;
                }
                let u = ns[m];
                let p = pos[u].cast_signed();
                if !conflicts.contains(&(u.min(v), u.max(v))) && r < p && !crosses(pos[v], pos[u]) {
                    next[u] = v;
                    root[v] = root[u];
                    next[v] = root[v];
                    r = p;
                }
            }
        }
    }
    (root, next)
}

/// Places the blocks: as far towards the start as the separations allow,
/// then each pulled towards the blocks after it as far as they allow.
/// `mirrored` layers run from the end of the drawing's layers to their start
/// (the result is negated afterwards), so each neighbour pair is measured
/// the other way round. `None` when the blocks' constraints form a cycle.
fn compact(
    layers: &[Vec<usize>],
    root: &[usize],
    v: &Vertices<'_>,
    n: usize,
    mirrored: bool,
) -> Option<Vec<f64>> {
    // Separation constraints between blocks: block(a) + sep <= block(b).
    let mut sep: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    for layer in layers {
        for w in layer.windows(2) {
            let (a, b) = (w[0], w[1]);
            let key = (root[a], root[b]);
            let s = if mirrored {
                v.separation(b, a)
            } else {
                v.separation(a, b)
            };
            let e = sep.entry(key).or_insert(s);
            *e = e.max(s);
        }
    }
    for &(a, b, d) in v.extra {
        let key = if mirrored {
            (root[b], root[a])
        } else {
            (root[a], root[b])
        };
        if key.0 != key.1 {
            let e = sep.entry(key).or_insert(d);
            *e = e.max(d);
        }
    }
    let blocks: BTreeSet<usize> = (0..n)
        .filter(|&i| layers.iter().any(|l| l.contains(&i)))
        .map(|i| root[i])
        .collect();
    let mut preds: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
    let mut succs: BTreeMap<usize, Vec<(usize, f64)>> = BTreeMap::new();
    for (&(a, b), &s) in &sep {
        preds.entry(b).or_default().push((a, s));
        succs.entry(a).or_default().push((b, s));
    }
    // Topological order of the block graph (Kahn, smallest block first).
    let mut indeg: BTreeMap<usize, usize> = blocks
        .iter()
        .map(|&b| (b, preds.get(&b).map_or(0, Vec::len)))
        .collect();
    let mut ready: BTreeSet<usize> = indeg.iter().filter(|(_, d)| **d == 0).map(|(b, _)| *b).collect();
    let mut topo = Vec::new();
    while let Some(b) = ready.pop_first() {
        topo.push(b);
        for &(c, _) in succs.get(&b).map_or(&[][..], Vec::as_slice) {
            let d = indeg.get_mut(&c).expect("block");
            *d -= 1;
            if *d == 0 {
                ready.insert(c);
            }
        }
    }
    if topo.len() < blocks.len() {
        return None;
    }

    let mut x: BTreeMap<usize, f64> = BTreeMap::new();
    for &b in &topo {
        let v0 = preds
            .get(&b)
            .map_or(0.0, |ps| ps.iter().map(|&(a, s)| x[&a] + s).fold(0.0, f64::max));
        x.insert(b, v0);
    }
    for &b in topo.iter().rev() {
        if let Some(ss) = succs.get(&b) {
            let limit = ss.iter().map(|&(c, s)| x[&c] - s).fold(f64::INFINITY, f64::min);
            if limit.is_finite() && limit > x[&b] {
                x.insert(b, limit);
            }
        }
    }
    Some((0..n).map(|i| x.get(&root[i]).copied().unwrap_or(0.0)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cards 56 across: 28 on either side of the anchor.
    fn cards(n: usize) -> (Vec<f64>, Vec<Kind>) {
        (vec![28.0; n], vec![Kind::Card; n])
    }

    /// A vertex set with no frames and no further constraints.
    fn plain<'a>(lo: &'a [f64], hi: &'a [f64], kind: &'a [Kind]) -> Vertices<'a> {
        Vertices {
            lo,
            hi,
            kind,
            node_gap: 24.0,
            edge_gap: 12.0,
            pad_lo: &[],
            pad_hi: &[],
            border_up: &[],
            border_down: &[],
            extra: &[],
        }
    }

    #[test]
    fn a_chain_is_a_straight_line() {
        let layers = vec![vec![0], vec![1], vec![2]];
        let (half, kind) = cards(3);
        let x = coordinates(&layers, &[(0, 1), (1, 2)], &plain(&half, &half, &kind));
        assert!((x[0] - x[1]).abs() < 1e-9 && (x[1] - x[2]).abs() < 1e-9, "{x:?}");
    }

    #[test]
    fn neighbours_in_a_layer_never_overlap() {
        // a -> c, a -> d, b -> d: c and d share a layer.
        let layers = vec![vec![0, 1], vec![2, 3]];
        let (half, kind) = cards(4);
        let v = plain(&half, &half, &kind);
        let x = coordinates(&layers, &[(0, 2), (0, 3), (1, 3)], &v);
        assert!(x[3] - x[2] >= 56.0 + 24.0 - 1e-9, "{x:?}");
        assert!(x[1] - x[0] >= 56.0 + 24.0 - 1e-9, "{x:?}");
    }

    #[test]
    fn a_fork_centres_its_source_between_the_branches() {
        let layers = vec![vec![0], vec![1, 2]];
        let (half, kind) = cards(3);
        let v = plain(&half, &half, &kind);
        let x = coordinates(&layers, &[(0, 1), (0, 2)], &v);
        assert!((x[0] - f64::midpoint(x[1], x[2])).abs() < 1e-9, "{x:?}");
    }

    #[test]
    fn uneven_sides_keep_their_facing_gap_in_every_run() {
        // `a` reaches 40 past its anchor towards `b`; `b` reaches 5 back
        // towards `a`. Measured the wrong way round in the mirrored runs,
        // the gap would be 50 + 10 instead of 40 + 5.
        let layers = vec![vec![0, 1]];
        let (lo, hi, kind) = ([10.0, 5.0], [40.0, 50.0], [Kind::Card, Kind::Card]);
        let x = coordinates(&layers, &[], &plain(&lo, &hi, &kind));
        assert!((x[1] - x[0] - (40.0 + 5.0 + 24.0)).abs() < 1e-9, "{x:?}");
    }
}
