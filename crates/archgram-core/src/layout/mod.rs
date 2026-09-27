//! Placing the boxes (ARCHITECTURE.md, Layout): a layered layout in the
//! Sugiyama tradition, run on the spec's nodes and edges.
//!
//! 1. Cycle removal (`acyclic`): edges closing a cycle are reversed for now.
//! 2. Layering (`rank`): network simplex; `sameLayer` groups are solved as
//!    one vertex, `first` and `last` then move their nodes to the ends.
//! 3. Long edges are split by dummy vertices, one per layer they cross.
//! 4. Crossing reduction (`order`), keeping `order` hints.
//! 5. Coordinates across the layers (`position`, Brandes and Köpf); along
//!    them, each layer is as deep as its deepest card, cards centred in it.
//!
//! The layout is computed as if the flow ran right: "main" is along the
//! flow, "cross" across it. A flow running down swaps the two at the end.
//! Frames are not laid out yet (v0.2); their nodes are placed like any other.

mod acyclic;
mod order;
mod position;
mod rank;

use crate::error::SpecError;
use crate::geometry::{Point, Rect, Size};
use crate::spec::{Direction, Spec};
use crate::tokens::{SPACING_EDGE_EDGE, SPACING_LAYER_LAYER, SPACING_NODE_NODE};

/// Where everything goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Each node's card, in spec order.
    pub nodes: Vec<Rect>,
    /// Each edge's path in spec order, from its `from` node's centre through
    /// the bends of the layout to its `to` node's centre. The router (M5)
    /// turns these into orthogonal lines; until then they are drawn as is.
    pub edges: Vec<Vec<Point>>,
    /// The layer of each node, in spec order.
    pub layers: Vec<usize>,
    /// The size of the whole drawing, from the origin.
    pub size: Size,
}

/// Lays out the nodes with the card sizes `sizes` (one per node, in spec order).
///
/// # Errors
///
/// Layout hints that contradict the edges, located in the spec.
///
/// # Panics
///
/// When `spec` has not passed validation (an edge or hint names a node that
/// does not exist), or `sizes` does not hold one size per node.
#[allow(clippy::too_many_lines)] // the steps above, in order
pub fn place(spec: &Spec, sizes: &[Size]) -> Result<Placement, Vec<SpecError>> {
    let n = spec.nodes.len();
    let index = |id: &str| {
        spec.nodes
            .iter()
            .position(|nd| nd.id == id)
            .expect("validated id")
    };
    let edges: Vec<(usize, usize)> = spec
        .edges
        .iter()
        .map(|e| (index(&e.from), index(&e.to)))
        .collect();
    let (main_size, cross_size): (Vec<f64>, Vec<f64>) = match spec.direction {
        Direction::Right => sizes.iter().map(|s| (s.w, s.h)).unzip(),
        Direction::Down => sizes.iter().map(|s| (s.h, s.w)).unzip(),
    };

    // 1. Cycle removal.
    let reversed = acyclic::reversed_edges(n, &edges);
    let dag: Vec<(usize, usize)> = edges
        .iter()
        .zip(&reversed)
        .map(|(&(a, b), &r)| if r { (b, a) } else { (a, b) })
        .collect();

    // 2. Layering, with sameLayer groups merged into one vertex each.
    let mut errors = Vec::new();
    let mut group = (0..n).collect::<Vec<_>>();
    for (j, members) in spec.hints.same_layer.iter().enumerate() {
        let ids: Vec<usize> = members.iter().map(|m| index(m)).collect();
        let head = ids.iter().map(|&i| group[i]).min().unwrap_or(0);
        for &i in &ids {
            let old = group[i];
            for g in &mut group {
                if *g == old {
                    *g = head;
                }
            }
        }
        for (k, &(a, b)) in dag.iter().enumerate() {
            if ids.contains(&a) && ids.contains(&b) {
                let e = &spec.edges[k];
                errors.push(SpecError::at(
                    format!("/hints/sameLayer/{j}"),
                    format!(
                        "`{}` and `{}` cannot share a layer: an edge joins them",
                        e.from, e.to
                    ),
                ));
            }
        }
    }
    let incoming = |v: usize| dag.iter().any(|&(_, b)| b == v);
    let outgoing = |v: usize| dag.iter().any(|&(a, _)| a == v);
    for (j, id) in spec.hints.first.iter().enumerate() {
        if incoming(index(id)) {
            errors.push(SpecError::at(
                format!("/hints/first/{j}"),
                format!("`{id}` cannot come first: an edge leads into it"),
            ));
        }
    }
    for (j, id) in spec.hints.last.iter().enumerate() {
        if outgoing(index(id)) {
            errors.push(SpecError::at(
                format!("/hints/last/{j}"),
                format!("`{id}` cannot come last: an edge leads out of it"),
            ));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut weights: std::collections::BTreeMap<(usize, usize), u32> = std::collections::BTreeMap::new();
    for &(a, b) in &dag {
        if group[a] != group[b] {
            *weights.entry((group[a], group[b])).or_insert(0) += 1;
        }
    }
    let group_edges: Vec<rank::Edge> = weights.iter().map(|(&(a, b), &w)| (a, b, w)).collect();
    let group_rank = rank::layers(n, &group_edges);
    let mut layer: Vec<usize> = (0..n)
        .map(|v| usize::try_from(group_rank[group[v]]).unwrap_or(0))
        .collect();
    let deepest = layer.iter().copied().max().unwrap_or(0);
    for id in &spec.hints.first {
        layer[index(id)] = 0;
    }
    for id in &spec.hints.last {
        layer[index(id)] = deepest;
    }

    // 3. Dummy vertices for edges longer than one layer.
    let mut vertex_layer = layer.clone();
    let mut chains: Vec<Vec<usize>> = Vec::with_capacity(dag.len());
    let mut short: Vec<(usize, usize)> = Vec::new();
    for &(a, b) in &dag {
        let mut chain = vec![a];
        for l in layer[a] + 1..layer[b] {
            vertex_layer.push(l);
            chain.push(vertex_layer.len() - 1);
        }
        chain.push(b);
        for w in chain.windows(2) {
            short.push((w[0], w[1]));
        }
        chains.push(chain);
    }
    let total = vertex_layer.len();
    let mut layers: Vec<Vec<usize>> =
        vec![Vec::new(); vertex_layer.iter().copied().max().map_or(0, |m| m + 1)];
    for (v, &l) in vertex_layer.iter().enumerate() {
        layers[l].push(v);
    }

    // 4. Crossing reduction.
    let groups: Vec<Vec<usize>> = spec
        .hints
        .order
        .iter()
        .map(|g| g.iter().map(|m| index(m)).collect())
        .collect();
    let layers = order::order(&layers, &short, total, &groups);

    // 5. Coordinates.
    let dummy: Vec<bool> = (0..total).map(|v| v >= n).collect();
    let cross_sizes: Vec<f64> = (0..total)
        .map(|v| if v < n { cross_size[v] } else { 0.0 })
        .collect();
    let cross = position::coordinates(
        &layers,
        &short,
        &position::Vertices {
            size: &cross_sizes,
            dummy: &dummy,
            node_gap: SPACING_NODE_NODE,
            edge_gap: SPACING_EDGE_EDGE,
        },
    );
    let depth: Vec<f64> = layers
        .iter()
        .map(|l| {
            l.iter()
                .filter(|&&v| v < n)
                .map(|&v| main_size[v])
                .fold(0.0, f64::max)
        })
        .collect();
    let mut start = Vec::with_capacity(layers.len());
    let mut at = 0.0;
    for d in &depth {
        start.push(at);
        at += d + SPACING_LAYER_LAYER;
    }
    let centre_main = |v: usize| start[vertex_layer[v]] + depth[vertex_layer[v]] / 2.0;

    // Into the drawing's frame, the smallest coordinate at 0.
    let cross_min = (0..total)
        .map(|v| cross[v] - cross_sizes[v] / 2.0)
        .fold(f64::INFINITY, f64::min);
    let point = |main: f64, across: f64| match spec.direction {
        Direction::Right => Point {
            x: main,
            y: across - cross_min,
        },
        Direction::Down => Point {
            x: across - cross_min,
            y: main,
        },
    };
    let nodes: Vec<Rect> = (0..n)
        .map(|v| {
            let c = point(centre_main(v), cross[v]);
            Rect {
                x: c.x - sizes[v].w / 2.0,
                y: c.y - sizes[v].h / 2.0,
                w: sizes[v].w,
                h: sizes[v].h,
            }
        })
        .collect();
    let paths: Vec<Vec<Point>> = chains
        .iter()
        .zip(&reversed)
        .map(|(chain, &r)| {
            let mut p: Vec<Point> = chain.iter().map(|&v| point(centre_main(v), cross[v])).collect();
            if r {
                p.reverse();
            }
            p
        })
        .collect();
    let w = nodes.iter().map(Rect::right).fold(0.0, f64::max);
    let h = nodes.iter().map(Rect::bottom).fold(0.0, f64::max);
    Ok(Placement {
        nodes,
        edges: paths,
        layers: layer,
        size: Size { w, h },
    })
}
