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

/// Sweeps, as in Graphviz's `dot`: enough for small graphs to settle.
const SWEEPS: usize = 24;

/// The layers, each ordered; `edges` join a layer to the next.
pub fn order(
    layers: &[Vec<usize>],
    edges: &[(usize, usize)],
    n: usize,
    groups: &[Vec<usize>],
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
    for layer in &mut current {
        apply_groups(layer, groups);
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
            let keys: Vec<(usize, f64)> = current[i]
                .iter()
                .enumerate()
                .map(|(place, &v)| {
                    #[allow(clippy::cast_precision_loss)] // positions are small
                    let key = median(&neighbours[v].iter().map(|&u| pos[u]).collect::<Vec<_>>())
                        .unwrap_or(place as f64);
                    (v, key)
                })
                .collect();
            let mut sorted = keys;
            sorted.sort_by(|a, b| a.1.total_cmp(&b.1));
            current[i] = sorted.into_iter().map(|(v, _)| v).collect();
            apply_groups(&mut current[i], groups);
        }
        transpose(&mut current, &down, n, groups);
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
fn initial(layers: &[Vec<usize>], down: &[Vec<usize>], layer_of: &[usize], n: usize) -> Vec<Vec<usize>> {
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
        let members: Vec<usize> = group.iter().copied().filter(|v| layer.contains(v)).collect();
        for (slot, member) in slots.into_iter().zip(members) {
            layer[slot] = member;
        }
    }
}

/// Whether two vertices may trade places: not when both belong to one hint group.
fn swappable(a: usize, b: usize, groups: &[Vec<usize>]) -> bool {
    !groups.iter().any(|g| g.contains(&a) && g.contains(&b))
}

/// Swaps neighbours in each layer while that reduces the crossings on both sides.
fn transpose(layers: &mut [Vec<usize>], down: &[Vec<usize>], n: usize, groups: &[Vec<usize>]) {
    let mut improved = true;
    let mut rounds = 0;
    while improved && rounds < 64 {
        improved = false;
        rounds += 1;
        for i in 0..layers.len() {
            for j in 0..layers[i].len().saturating_sub(1) {
                let (v, w) = (layers[i][j], layers[i][j + 1]);
                if !swappable(v, w, groups) {
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
    layers.windows(2).map(|w| bilayer(&w[0], &w[1], down, n)).sum()
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

    #[test]
    fn a_crossing_that_can_be_undone_is_undone() {
        // Layer 0: a b; layer 1: c d; edges a->d, b->c cross in the start order.
        let layers = vec![vec![0, 1], vec![3, 2]];
        let edges = [(0, 2), (1, 3)];
        let ordered = order(&layers, &edges, 4, &[]);
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
        let ordered = order(&layers, &edges, 4, &[vec![2, 3]]);
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
