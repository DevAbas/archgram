//! Layering (ARCHITECTURE.md, Layout, step 2): network simplex.
//!
//! Every vertex gets a layer so that each edge points to a later layer and
//! the total length of the edges, weighted, is as small as possible. This is
//! the linear programme Gansner, Koutsofios, North and Vo (1993) solve with
//! the network simplex method:
//!
//! 1. start from a feasible layering (longest path from the sources);
//! 2. grow a spanning tree of tight edges (edges exactly one layer long),
//!    shifting the tree when no tight edge reaches a new vertex;
//! 3. while some tree edge has a negative cut value, swap it for the non-tree
//!    edge with the least slack that crosses its cut the other way, then
//!    recompute the layers from the tree.
//!
//! Choices are made in index order, so the result is the same every time.
//! Components are solved one at a time and each starts at layer 0.

// Names follow the paper's notation: vertices a, b, u, v; edge e; tree t.
#![allow(clippy::many_single_char_names, clippy::similar_names)]

/// A directed edge with a weight; its minimum length is one layer.
pub type Edge = (usize, usize, u32);

/// A layer for each of `n` vertices, the smallest in each component being 0.
pub fn layers(n: usize, edges: &[Edge]) -> Vec<i64> {
    let mut rank = vec![0i64; n];
    for component in components(n, edges) {
        let inside: Vec<Edge> = edges
            .iter()
            .copied()
            .filter(|&(a, _, _)| component.contains(&a))
            .collect();
        solve(&component, &inside, &mut rank);
    }
    rank
}

/// The root of `v`'s set in a union-find forest, compressing the path.
fn find(p: &mut [usize], v: usize) -> usize {
    let mut r = v;
    while p[r] != r {
        r = p[r];
    }
    let mut v = v;
    while p[v] != r {
        let next = p[v];
        p[v] = r;
        v = next;
    }
    r
}

/// The connected components, ignoring direction, each sorted, in order of
/// their smallest vertex.
fn components(n: usize, edges: &[Edge]) -> Vec<Vec<usize>> {
    let mut parent: Vec<usize> = (0..n).collect();
    for &(a, b, _) in edges {
        let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
        if ra != rb {
            parent[ra.max(rb)] = ra.min(rb);
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> = std::collections::BTreeMap::new();
    for v in 0..n {
        let r = find(&mut parent, v);
        groups.entry(r).or_default().push(v);
    }
    groups.into_values().collect()
}

fn slack(rank: &[i64], (a, b, _): Edge) -> i64 {
    rank[b] - rank[a] - 1
}

/// Solves one connected component in place.
fn solve(vertices: &[usize], edges: &[Edge], rank: &mut [i64]) {
    longest_path(vertices, edges, rank);
    if vertices.len() == 1 {
        rank[vertices[0]] = 0;
        return;
    }
    let mut tree = feasible_tree(vertices, edges, rank);
    // Each exchange lowers the objective or keeps it; the cap guards against
    // cycling on degenerate problems, which the tests would reveal.
    let cap = 16 * edges.len() + 64;
    for _ in 0..cap {
        let Some((leave, cut)) = negative_cut(vertices, edges, &tree) else {
            break;
        };
        let tail = split(vertices, edges, &tree, leave);
        // The entering edge crosses from the head side to the tail side.
        let enter = (0..edges.len())
            .filter(|i| !tree.contains(i))
            .filter(|&i| !tail[edges[i].0] && tail[edges[i].1])
            .min_by_key(|&i| (slack(rank, edges[i]), i));
        let Some(enter) = enter else { break };
        debug_assert!(cut < 0);
        tree.retain(|&e| e != leave);
        tree.push(enter);
        tree.sort_unstable();
        ranks_from_tree(vertices, edges, &tree, rank);
    }
    let min = vertices.iter().map(|&v| rank[v]).min().unwrap_or(0);
    for &v in vertices {
        rank[v] -= min;
    }
}

/// Sources at 0, every other vertex one past its latest predecessor.
fn longest_path(vertices: &[usize], edges: &[Edge], rank: &mut [i64]) {
    let mut indeg: std::collections::BTreeMap<usize, usize> = vertices.iter().map(|&v| (v, 0)).collect();
    for &(_, b, _) in edges {
        *indeg.get_mut(&b).expect("edge inside the component") += 1;
    }
    let mut ready: std::collections::BTreeSet<usize> =
        indeg.iter().filter(|(_, d)| **d == 0).map(|(v, _)| *v).collect();
    for &v in vertices {
        rank[v] = 0;
    }
    while let Some(v) = ready.pop_first() {
        for &(a, b, _) in edges.iter().filter(|e| e.0 == v) {
            rank[b] = rank[b].max(rank[a] + 1);
            let d = indeg.get_mut(&b).expect("edge inside the component");
            *d -= 1;
            if *d == 0 {
                ready.insert(b);
            }
        }
    }
}

/// A spanning tree of tight edges, shifting ranks where needed (edge indices, sorted).
fn feasible_tree(vertices: &[usize], edges: &[Edge], rank: &mut [i64]) -> Vec<usize> {
    let mut in_tree: std::collections::BTreeSet<usize> = std::collections::BTreeSet::from([vertices[0]]);
    let mut tree = Vec::new();
    loop {
        // Grow along tight edges as far as they reach.
        let mut grew = true;
        while grew {
            grew = false;
            for (i, &e) in edges.iter().enumerate() {
                let (a, b, _) = e;
                if in_tree.contains(&a) != in_tree.contains(&b) && slack(rank, e) == 0 {
                    in_tree.insert(a);
                    in_tree.insert(b);
                    tree.push(i);
                    grew = true;
                }
            }
        }
        if in_tree.len() == vertices.len() {
            break;
        }
        // Shift the tree to make the least-slack edge leaving it tight.
        let (i, _) = edges
            .iter()
            .enumerate()
            .filter(|(_, e)| in_tree.contains(&e.0) != in_tree.contains(&e.1))
            .min_by_key(|(i, e)| (slack(rank, **e), *i))
            .expect("the component is connected");
        let (a, _, _) = edges[i];
        let delta = if in_tree.contains(&a) {
            slack(rank, edges[i])
        } else {
            -slack(rank, edges[i])
        };
        for &v in &in_tree {
            rank[v] += delta;
        }
    }
    tree.sort_unstable();
    tree
}

/// For tree edge `e`, which vertices lie on its tail's side once it is removed.
fn split(vertices: &[usize], edges: &[Edge], tree: &[usize], e: usize) -> Vec<bool> {
    let size = vertices.iter().max().map_or(0, |m| m + 1);
    let mut side = vec![false; size];
    let (tail, _, _) = edges[e];
    side[tail] = true;
    let mut stack = vec![tail];
    while let Some(v) = stack.pop() {
        for &t in tree {
            if t == e {
                continue;
            }
            let (a, b, _) = edges[t];
            let other = if a == v {
                b
            } else if b == v {
                a
            } else {
                continue;
            };
            if !side[other] {
                side[other] = true;
                stack.push(other);
            }
        }
    }
    side
}

/// The tree edge with the most negative cut value, if any, and that value.
fn negative_cut(vertices: &[usize], edges: &[Edge], tree: &[usize]) -> Option<(usize, i64)> {
    let mut best: Option<(usize, i64)> = None;
    for &t in tree {
        let tail = split(vertices, edges, tree, t);
        let cut: i64 = edges
            .iter()
            .map(|&(a, b, w)| match (tail[a], tail[b]) {
                (true, false) => i64::from(w),
                (false, true) => -i64::from(w),
                _ => 0,
            })
            .sum();
        if cut < 0 && best.is_none_or(|(_, c)| cut < c) {
            best = Some((t, cut));
        }
    }
    best
}

/// Ranks that make every tree edge tight, from the component's first vertex.
fn ranks_from_tree(vertices: &[usize], edges: &[Edge], tree: &[usize], rank: &mut [i64]) {
    let size = vertices.iter().max().map_or(0, |m| m + 1);
    let mut known = vec![false; size];
    known[vertices[0]] = true;
    rank[vertices[0]] = 0;
    let mut stack = vec![vertices[0]];
    while let Some(v) = stack.pop() {
        for &t in tree {
            let (a, b, _) = edges[t];
            if a == v && !known[b] {
                rank[b] = rank[a] + 1;
                known[b] = true;
                stack.push(b);
            } else if b == v && !known[a] {
                rank[a] = rank[b] - 1;
                known[a] = true;
                stack.push(a);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total_length(rank: &[i64], edges: &[Edge]) -> i64 {
        edges
            .iter()
            .map(|&(a, b, w)| (rank[b] - rank[a]) * i64::from(w))
            .sum()
    }

    fn feasible(rank: &[i64], edges: &[Edge]) -> bool {
        edges.iter().all(|&e| slack(rank, e) >= 0)
    }

    #[test]
    fn a_chain_takes_one_layer_per_vertex() {
        let edges = [(0, 1, 1), (1, 2, 1), (2, 3, 1)];
        assert_eq!(layers(4, &edges), [0, 1, 2, 3]);
    }

    #[test]
    fn a_short_branch_moves_next_to_where_it_ends() {
        // 0 -> 1 -> 2 -> 3 and 4 -> 3: longest path puts 4 at 0 (length 3);
        // the optimum puts it at 2 (length 1).
        let edges = [(0, 1, 1), (1, 2, 1), (2, 3, 1), (4, 3, 1)];
        let rank = layers(5, &edges);
        assert!(feasible(&rank, &edges));
        assert_eq!(rank[4], 2);
        assert_eq!(total_length(&rank, &edges), 4);
    }

    #[test]
    fn the_textbook_example_reaches_its_optimum() {
        // Gansner et al. 1993, figure 2-1: a..h; the optimum length is 10.
        let (a, b, c, d, e, f, g, h) = (0, 1, 2, 3, 4, 5, 6, 7);
        let edges = [
            (a, b, 1),
            (b, c, 1),
            (c, d, 1),
            (d, h, 1),
            (a, e, 1),
            (a, f, 1),
            (e, g, 1),
            (f, g, 1),
            (g, h, 1),
        ];
        let rank = layers(8, &edges);
        assert!(feasible(&rank, &edges));
        assert_eq!(total_length(&rank, &edges), 10);
    }

    #[test]
    fn components_start_at_zero_each() {
        let edges = [(0, 1, 1), (2, 3, 1)];
        assert_eq!(layers(4, &edges), [0, 1, 0, 1]);
    }
}
