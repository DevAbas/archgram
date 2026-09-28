//! Crossing reduction (ARCHITECTURE.md, Layout, step 3).
//!
//! Orders the vertices of each layer to keep edges from crossing:
//!
//! - the starting order follows the spec: a depth-first walk down the edges
//!   from each vertex in index order, so a chain stays in line;
//! - sweeps go down and up the layers, sorting each layer by the weighted
//!   median of its neighbours' positions in the layer just placed
//!   (Gansner et al. 1993, section 3); a vertex with no such neighbours
//!   keeps its place;
//! - after each sweep, neighbours in a layer are swapped while that removes
//!   crossings (the transpose heuristic, same paper);
//! - the best order seen is kept, the earlier one on a tie.
//!
//! `order` hints hold throughout: within a layer, the hinted vertices keep
//! the hinted order among the places they occupy.
//!
//! Frames stay whole (dagre's `sortSubgraph`, after Forster 2002): a layer
//! is sorted frame by frame, a frame's vertices and child frames among
//! themselves between its two borders, a child frame as one item at the
//! mean of its vertices' keys. Frames side by side keep the order they have
//! in the layer just placed, so their boxes do not cross, and a swap only
//! trades two vertices of the same frame.

use std::collections::BTreeMap;

/// Sweeps, as in Graphviz's `dot`: enough for small graphs to settle.
const SWEEPS: usize = 24;

/// The frames the vertices sit in (`frames::Frames`).
pub struct Clusters<'a> {
    /// Each vertex's innermost frame, border vertices included.
    pub frame_of: &'a [Option<usize>],
    /// Each frame's parent.
    pub parent: &'a [Option<usize>],
    /// For a border vertex, its frame and whether it is the first border.
    pub border: &'a [Option<(usize, bool)>],
}

/// The layers, each ordered; `edges` join a layer to the next.
pub fn order(
    layers: &[Vec<usize>],
    edges: &[(usize, usize)],
    n: usize,
    groups: &[Vec<usize>],
    clusters: &Clusters<'_>,
) -> Vec<Vec<usize>> {
    let mut up = vec![Vec::new(); n];
    let mut down = vec![Vec::new(); n];
    for &(a, b) in edges {
        down[a].push(b);
        up[b].push(a);
    }
    let layer_of = {
        let mut l = vec![0; n];
        for (i, layer) in layers.iter().enumerate() {
            for &v in layer {
                l[v] = i;
            }
        }
        l
    };

    let mut current = initial(layers, &down, &layer_of, n);
    for i in 0..current.len() {
        let pos = positions(&current[i], n);
        let reference = if i > 0 {
            Some(current[i - 1].clone())
        } else {
            None
        };
        current[i] = arrange(&current[i], &|v| pos[v], clusters, reference.as_deref(), n);
        apply_groups(&mut current[i], groups);
    }
    let mut best = current.clone();
    let mut best_crossings = crossings(&best, &down, n);
    for sweep in 0..SWEEPS {
        let going_down = sweep % 2 == 0;
        let indices: Vec<usize> = if going_down {
            (1..current.len()).collect()
        } else {
            (0..current.len().saturating_sub(1)).rev().collect()
        };
        for i in indices {
            let reference = if going_down { i - 1 } else { i + 1 };
            let pos = positions(&current[reference], n);
            let neighbours = if going_down { &up } else { &down };
            let mut key = vec![0.0; n];
            for (place, &v) in current[i].iter().enumerate() {
                #[allow(clippy::cast_precision_loss)] // positions are small
                let k = median(&neighbours[v].iter().map(|&u| pos[u]).collect::<Vec<_>>())
                    .unwrap_or(place as f64);
                key[v] = k;
            }
            current[i] = arrange(
                &current[i],
                &|v| key[v],
                clusters,
                Some(&current[reference]),
                n,
            );
            apply_groups(&mut current[i], groups);
        }
        transpose(&mut current, &down, n, groups, clusters);
        let c = crossings(&current, &down, n);
        if c < best_crossings {
            best_crossings = c;
            best.clone_from(&current);
        }
        if best_crossings == 0 {
            break;
        }
    }
    best
}

/// Depth-first from each vertex in index order, appending each vertex to its
/// layer when first reached, so connected vertices start close together.
fn initial(
    layers: &[Vec<usize>],
    down: &[Vec<usize>],
    layer_of: &[usize],
    n: usize,
) -> Vec<Vec<usize>> {
    let mut out = vec![Vec::new(); layers.len()];
    let mut seen = vec![false; n];
    let mut all: Vec<usize> = layers.iter().flatten().copied().collect();
    all.sort_unstable();
    for start in all {
        let mut stack = vec![start];
        while let Some(v) = stack.pop() {
            if seen[v] {
                continue;
            }
            seen[v] = true;
            out[layer_of[v]].push(v);
            for &w in down[v].iter().rev() {
                if !seen[w] {
                    stack.push(w);
                }
            }
        }
    }
    out
}

fn positions(layer: &[usize], n: usize) -> Vec<f64> {
    let mut pos = vec![0.0; n];
    for (i, &v) in layer.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        {
            pos[v] = i as f64;
        }
    }
    pos
}

/// The weighted median of Gansner et al. (1993): the middle position, or for
/// an even count, the two middle positions weighted towards the side whose
/// neighbours lie closer together.
fn median(p: &[f64]) -> Option<f64> {
    let mut p = p.to_vec();
    p.sort_by(f64::total_cmp);
    let m = p.len() / 2;
    match p.len() {
        0 => None,
        len if len % 2 == 1 => Some(p[m]),
        2 => Some(f64::midpoint(p[0], p[1])),
        len => {
            let left = p[m - 1] - p[0];
            let right = p[len - 1] - p[m];
            if left + right == 0.0 {
                Some(f64::midpoint(p[m - 1], p[m]))
            } else {
                Some((p[m - 1] * right + p[m] * left) / (left + right))
            }
        }
    }
}

/// One layer sorted by `key`, frame by frame: each frame's vertices and child
/// frames sorted among themselves between its borders, a child frame at the
/// mean key of the vertices inside it (its first border's key when it has
/// none here). Frames side by side that `reference` also holds keep its
/// order. Ties keep the layer's order.
fn arrange(
    layer: &[usize],
    key: &dyn Fn(usize) -> f64,
    clusters: &Clusters<'_>,
    reference: Option<&[usize]>,
    n: usize,
) -> Vec<usize> {
    let place = positions(layer, n);
    let tie = |a: &(f64, f64), b: &(f64, f64)| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1));
    if clusters.parent.is_empty() {
        let mut out = layer.to_vec();
        out.sort_by(|&a, &b| tie(&(key(a), place[a]), &(key(b), place[b])));
        return out;
    }
    // Each frame's borders in this layer, and the keys inside it.
    let mut first = BTreeMap::new();
    let mut last = BTreeMap::new();
    let mut sums: BTreeMap<usize, (f64, f64)> = BTreeMap::new();
    let mut items: BTreeMap<Option<usize>, Vec<Item>> = BTreeMap::new();
    for &v in layer {
        match clusters.border[v] {
            Some((f, true)) => {
                first.insert(f, v);
                items
                    .entry(clusters.parent[f])
                    .or_default()
                    .push(Item::Frame(f));
            }
            Some((f, false)) => {
                last.insert(f, v);
            }
            None => {
                items
                    .entry(clusters.frame_of[v])
                    .or_default()
                    .push(Item::Vertex(v));
                let mut at = clusters.frame_of[v];
                while let Some(f) = at {
                    let s = sums.entry(f).or_insert((0.0, 0.0));
                    *s = (s.0 + key(v), s.1 + 1.0);
                    at = clusters.parent[f];
                }
            }
        }
    }
    let item_key = |item: &Item| match *item {
        Item::Vertex(v) => (key(v), place[v]),
        Item::Frame(frame) => {
            let b = first[&frame];
            let k = sums
                .get(&frame)
                .map_or_else(|| key(b), |&(s, count)| s / count);
            (k, place[b])
        }
    };
    let reference_place = reference.map(|r| positions(r, n));
    for list in items.values_mut() {
        list.sort_by(|a, b| tie(&item_key(a), &item_key(b)));
        // Frames the reference layer holds keep their order from it.
        if let Some(rp) = &reference_place {
            let held = |item: &Item| match *item {
                Item::Frame(f) => reference
                    .expect("with its places")
                    .iter()
                    .any(|&v| clusters.border[v] == Some((f, true))),
                Item::Vertex(_) => false,
            };
            let slots: Vec<usize> = (0..list.len()).filter(|&i| held(&list[i])).collect();
            let mut framed: Vec<Item> = slots.iter().map(|&i| list[i]).collect();
            let at = |item: &Item| match *item {
                Item::Frame(f) => reference
                    .expect("with its places")
                    .iter()
                    .find(|&&v| clusters.border[v] == Some((f, true)))
                    .map_or(0.0, |&v| rp[v]),
                Item::Vertex(_) => 0.0,
            };
            framed.sort_by(|a, b| at(a).total_cmp(&at(b)));
            for (slot, item) in slots.into_iter().zip(framed) {
                list[slot] = item;
            }
        }
    }
    let mut out = Vec::with_capacity(layer.len());
    emit(None, &items, &first, &last, &mut out);
    out
}

/// One thing a frame holds in a layer: a vertex, or a child frame.
#[derive(Clone, Copy)]
enum Item {
    Vertex(usize),
    Frame(usize),
}

/// Writes `cluster`'s items in order, each frame between its borders.
fn emit(
    cluster: Option<usize>,
    items: &BTreeMap<Option<usize>, Vec<Item>>,
    first: &BTreeMap<usize, usize>,
    last: &BTreeMap<usize, usize>,
    out: &mut Vec<usize>,
) {
    if let Some(f) = cluster {
        out.push(first[&f]);
    }
    for &item in items.get(&cluster).map_or(&[][..], Vec::as_slice) {
        match item {
            Item::Vertex(v) => out.push(v),
            Item::Frame(g) => emit(Some(g), items, first, last, out),
        }
    }
    if let Some(f) = cluster {
        out.push(last[&f]);
    }
}

/// Puts each hint group's members that share this layer into the group's order,
/// in the places those members already occupy.
fn apply_groups(layer: &mut [usize], groups: &[Vec<usize>]) {
    for group in groups {
        let slots: Vec<usize> = layer
            .iter()
            .enumerate()
            .filter(|(_, v)| group.contains(v))
            .map(|(i, _)| i)
            .collect();
        let members: Vec<usize> = group
            .iter()
            .copied()
            .filter(|v| layer.contains(v))
            .collect();
        for (slot, member) in slots.into_iter().zip(members) {
            layer[slot] = member;
        }
    }
}

/// Whether two neighbours may trade places: two vertices of the same frame,
/// neither a border, not both in one hint group.
fn swappable(a: usize, b: usize, groups: &[Vec<usize>], c: &Clusters<'_>) -> bool {
    let framed = c.parent.is_empty()
        || (c.border[a].is_none() && c.border[b].is_none() && c.frame_of[a] == c.frame_of[b]);
    framed && !groups.iter().any(|g| g.contains(&a) && g.contains(&b))
}

/// Swaps neighbours in each layer while that reduces the crossings on both sides.
fn transpose(
    layers: &mut [Vec<usize>],
    down: &[Vec<usize>],
    n: usize,
    groups: &[Vec<usize>],
    clusters: &Clusters<'_>,
) {
    let mut improved = true;
    let mut rounds = 0;
    while improved && rounds < 64 {
        improved = false;
        rounds += 1;
        for i in 0..layers.len() {
            for j in 0..layers[i].len().saturating_sub(1) {
                let (v, w) = (layers[i][j], layers[i][j + 1]);
                if !swappable(v, w, groups, clusters) {
                    continue;
                }
                let before = local_crossings(layers, i, down, n);
                layers[i].swap(j, j + 1);
                let after = local_crossings(layers, i, down, n);
                if after < before {
                    improved = true;
                } else {
                    layers[i].swap(j, j + 1);
                }
            }
        }
    }
}

/// Crossings between layer `i` and its two neighbours.
fn local_crossings(layers: &[Vec<usize>], i: usize, down: &[Vec<usize>], n: usize) -> usize {
    let mut c = 0;
    if i > 0 {
        c += bilayer(&layers[i - 1], &layers[i], down, n);
    }
    if i + 1 < layers.len() {
        c += bilayer(&layers[i], &layers[i + 1], down, n);
    }
    c
}

/// All crossings in the layering.
pub fn crossings(layers: &[Vec<usize>], down: &[Vec<usize>], n: usize) -> usize {
    layers
        .windows(2)
        .map(|w| bilayer(&w[0], &w[1], down, n))
        .sum()
}

/// Crossings between two adjacent layers: pairs of edges whose ends are in
/// opposite orders.
fn bilayer(upper: &[usize], lower: &[usize], down: &[Vec<usize>], n: usize) -> usize {
    let pos = {
        let mut p = vec![usize::MAX; n];
        for (i, &v) in lower.iter().enumerate() {
            p[v] = i;
        }
        p
    };
    let mut ends: Vec<(usize, usize)> = Vec::new();
    for (i, &u) in upper.iter().enumerate() {
        for &w in &down[u] {
            if pos[w] != usize::MAX {
                ends.push((i, pos[w]));
            }
        }
    }
    let mut c = 0;
    for a in 0..ends.len() {
        for b in a + 1..ends.len() {
            let ((u1, l1), (u2, l2)) = (ends[a], ends[b]);
            if (u1 < u2 && l1 > l2) || (u1 > u2 && l1 < l2) {
                c += 1;
            }
        }
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: Clusters<'static> = Clusters {
        frame_of: &[],
        parent: &[],
        border: &[],
    };

    #[test]
    fn a_crossing_that_can_be_undone_is_undone() {
        // Layer 0: a b; layer 1: c d; edges a->d, b->c cross in the start order.
        let layers = vec![vec![0, 1], vec![3, 2]];
        let edges = [(0, 2), (1, 3)];
        let ordered = order(&layers, &edges, 4, &[], &NONE);
        let mut down = vec![Vec::new(); 4];
        for &(a, b) in &edges {
            down[a].push(b);
        }
        assert_eq!(crossings(&ordered, &down, 4), 0);
    }

    #[test]
    fn a_hint_group_keeps_its_order() {
        // Without the hint, c and d would swap to follow a and b.
        let layers = vec![vec![0, 1], vec![2, 3]];
        let edges = [(0, 3), (1, 2)];
        let ordered = order(&layers, &edges, 4, &[vec![2, 3]], &NONE);
        let pos2 = ordered[1].iter().position(|&v| v == 2).unwrap();
        let pos3 = ordered[1].iter().position(|&v| v == 3).unwrap();
        assert!(pos2 < pos3);
    }

    #[test]
    fn the_weighted_median() {
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[3.0]), Some(3.0));
        assert_eq!(median(&[1.0, 3.0]), Some(2.0));
        assert_eq!(median(&[0.0, 1.0, 5.0]), Some(1.0));
    }
}
