//! Placing the boxes (ARCHITECTURE.md, Layout): a layered layout in the
//! Sugiyama tradition, run on the spec's nodes and edges.
//!
//! 1. Cycle removal (`acyclic`): edges closing a cycle are reversed for now.
//! 2. Layering (`rank`): network simplex; `sameLayer` groups are solved as
//!    one vertex, `first` and `last` then move their nodes to the ends.
//! 3. Long edges are split by dummy vertices, one per layer they cross.
//! 4. Crossing reduction (`order`), keeping `order` hints.
//! 5. Coordinates across the layers (`position`, Brandes and Köpf); along
//!    them, each layer is as deep as its deepest card or label, and every
//!    card in it takes the depth of its deepest card, so their sides line up.
//! 6. Routing (`route`): each edge runs orthogonally through the gaps between
//!    layers, which widen when their tracks need the room.
//!
//! An edge's label gets room of its own, as dagre and ELK give it: on a long
//! edge it takes the place of the middle dummy vertex, sized to the label;
//! on an edge between neighbouring layers, the gap reserves room for it
//! before its tracks, on the segment leaving the edge's first card.
//!
//! The layout is computed as if the flow ran right: "main" is along the
//! flow, "cross" across it. A flow running down swaps the two at the end.
//! Frames are not laid out yet (v0.2); their nodes are placed like any other.

mod acyclic;
mod order;
mod position;
mod rank;
mod route;

use crate::error::SpecError;
use crate::geometry::{Point, Rect, Size};
use crate::spec::{Direction, Spec};
use crate::tokens::{ROUNDED_CARD, SPACING_EDGE_EDGE, SPACING_LAYER_LAYER, SPACING_NODE_NODE};

/// Where everything goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Each node's card, in spec order.
    pub nodes: Vec<Rect>,
    /// Each edge's path in spec order: an orthogonal line from a side of its
    /// `from` node to a side of its `to` node.
    pub edges: Vec<Vec<Point>>,
    /// Each edge's label box in spec order, on its path; `None` without a label.
    pub labels: Vec<Option<Rect>>,
    /// The layer of each node, in spec order.
    pub layers: Vec<usize>,
    /// The size of the whole drawing, from the origin: every card, path and label.
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
    let (mut main_size, cross_size): (Vec<f64>, Vec<f64>) = match spec.direction {
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
    // Each label's extent along and across the flow.
    let label_extent: Vec<Option<(f64, f64)>> = spec
        .edges
        .iter()
        .map(|e| {
            e.label.as_deref().map(|l| {
                let s = crate::measure::label_size(l);
                match spec.direction {
                    Direction::Right => (s.w, s.h),
                    Direction::Down => (s.h, s.w),
                }
            })
        })
        .collect();
    // A long edge's label stands in for its middle dummy vertex.
    let label_vertex: Vec<Option<usize>> = chains
        .iter()
        .zip(&label_extent)
        .map(|(chain, ext)| ext.filter(|_| chain.len() > 2).map(|_| chain[chain.len() / 2]))
        .collect();
    let mut main_sizes: Vec<f64> = (0..total)
        .map(|v| if v < n { main_size[v] } else { 0.0 })
        .collect();
    let mut cross_sizes: Vec<f64> = (0..total)
        .map(|v| if v < n { cross_size[v] } else { 0.0 })
        .collect();
    for (v, ext) in label_vertex.iter().zip(&label_extent) {
        if let (Some(v), Some((along, across))) = (v, ext) {
            main_sizes[*v] = *along;
            cross_sizes[*v] = *across;
        }
    }
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
        .map(|l| l.iter().map(|&v| main_sizes[v]).fold(0.0, f64::max))
        .collect();
    // Cards in one layer share its deepest card's size along the flow: the
    // width of a column when the flow runs right, so their sides line up.
    let card_depth: Vec<f64> = layers
        .iter()
        .map(|l| {
            l.iter()
                .filter(|&&v| v < n)
                .map(|&v| main_size[v])
                .fold(0.0, f64::max)
        })
        .collect();
    for (v, size) in main_size.iter_mut().enumerate() {
        *size = card_depth[vertex_layer[v]];
    }

    // 6. Routing: ports on each card's sides, then each hop between two layers
    //    straight or as a Z through a track in the gap (`route`).
    let hops: Vec<(usize, usize, usize)> = chains
        .iter()
        .enumerate()
        .flat_map(|(e, chain)| chain.windows(2).map(move |w| (e, w[0], w[1])))
        .collect();
    // For a card, its hops on one side, ordered by where their other ends lie
    // across the layers, get ports spread around the side's middle.
    let mut out_port = vec![0.0; hops.len()];
    let mut in_port = vec![0.0; hops.len()];
    for v in 0..total {
        for (outgoing, ports_of) in [(true, &mut out_port), (false, &mut in_port)] {
            let mut mine: Vec<usize> = (0..hops.len())
                .filter(|&h| if outgoing { hops[h].1 == v } else { hops[h].2 == v })
                .collect();
            if v >= n {
                for h in mine {
                    ports_of[h] = cross[v];
                }
                continue;
            }
            mine.sort_by(|&a, &b| {
                let other = |h: usize| {
                    if outgoing {
                        cross[hops[h].2]
                    } else {
                        cross[hops[h].1]
                    }
                };
                other(a).total_cmp(&other(b)).then(a.cmp(&b))
            });
            let places = route::ports(
                mine.len(),
                cross[v],
                cross_size[v],
                SPACING_EDGE_EDGE,
                ROUNDED_CARD,
            );
            for (h, p) in mine.into_iter().zip(places) {
                ports_of[h] = p;
            }
        }
    }
    // Tracks per gap; a gap with more tracks than its width holds is widened.
    let level = |h: usize| (out_port[h] - in_port[h]).abs() < 0.5;
    let gaps = layers.len().saturating_sub(1);
    let mut track = vec![0usize; hops.len()];
    let mut track_count = vec![0usize; gaps];
    for (g, count) in track_count.iter_mut().enumerate() {
        let here: Vec<usize> = (0..hops.len())
            .filter(|&h| vertex_layer[hops[h].1] == g && !level(h))
            .collect();
        let risers: Vec<route::Riser> = here
            .iter()
            .map(|&h| route::Riser {
                from: out_port[h],
                to: in_port[h],
            })
            .collect();
        let (t, c) = route::tracks(&risers, SPACING_EDGE_EDGE);
        for (&h, t) in here.iter().zip(t) {
            track[h] = t;
        }
        *count = c;
    }
    // Room for the labels of edges between neighbouring layers, at the start
    // of the gap the edge leaves into, clear of the card and of the tracks.
    let mut lead = vec![0.0f64; gaps];
    for (chain, ext) in chains.iter().zip(&label_extent) {
        if let (2, Some((along, _))) = (chain.len(), ext) {
            let g = vertex_layer[chain[0]];
            lead[g] = lead[g].max(along + 2.0 * SPACING_EDGE_EDGE);
        }
    }
    #[allow(clippy::cast_precision_loss)] // track counts are small
    let gap_width: Vec<f64> = track_count
        .iter()
        .zip(&lead)
        .map(|(&c, &l)| SPACING_LAYER_LAYER.max(l + (c + 1) as f64 * SPACING_EDGE_EDGE))
        .collect();
    let mut start = Vec::with_capacity(layers.len());
    let mut at = 0.0;
    for (i, d) in depth.iter().enumerate() {
        start.push(at);
        at += d + gap_width.get(i).copied().unwrap_or(0.0);
    }
    let centre_main = |v: usize| start[vertex_layer[v]] + depth[vertex_layer[v]] / 2.0;
    // Where a hop leaves and enters, along the flow: a card's far or near
    // side; a dummy's centre, so a long edge runs straight through its layers.
    let leave_main = |v: usize| {
        if v < n {
            centre_main(v) + main_size[v] / 2.0
        } else {
            centre_main(v)
        }
    };
    let enter_main = |v: usize| {
        if v < n {
            centre_main(v) - main_size[v] / 2.0
        } else {
            centre_main(v)
        }
    };

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
            let (w, h) = match spec.direction {
                Direction::Right => (main_size[v], cross_size[v]),
                Direction::Down => (cross_size[v], main_size[v]),
            };
            Rect {
                x: c.x - w / 2.0,
                y: c.y - h / 2.0,
                w,
                h,
            }
        })
        .collect();
    let mut paths: Vec<Vec<Point>> = vec![Vec::new(); chains.len()];
    for (h, &(e, a, b)) in hops.iter().enumerate() {
        let g = vertex_layer[a];
        let mut pts = vec![(leave_main(a), out_port[h])];
        if !level(h) {
            #[allow(clippy::cast_precision_loss)]
            let t = start[g]
                + depth[g]
                + lead[g]
                + (gap_width[g] - lead[g]) * (track[h] + 1) as f64 / (track_count[g] + 1) as f64;
            pts.push((t, out_port[h]));
            pts.push((t, in_port[h]));
        }
        pts.push((enter_main(b), if level(h) { out_port[h] } else { in_port[h] }));
        for (m, c) in pts {
            push_point(&mut paths[e], point(m, c));
        }
    }
    for (path, &r) in paths.iter_mut().zip(&reversed) {
        if r {
            path.reverse();
        }
    }
    let labels: Vec<Option<Rect>> = (0..chains.len())
        .map(|e| {
            let (along, across) = label_extent[e]?;
            let (along_at, across_at) = if let Some(v) = label_vertex[e] {
                (centre_main(v), cross[v])
            } else {
                // The hop's segment leaving its first card, in the room the gap keeps.
                let hop = hops.iter().position(|&(k, _, _)| k == e).expect("one hop");
                let (_, a, b) = hops[hop];
                let gap = vertex_layer[a];
                let end = if level(hop) {
                    enter_main(b)
                } else {
                    start[gap] + depth[gap] + lead[gap]
                };
                (f64::midpoint(leave_main(a), end), out_port[hop])
            };
            let (width, height) = match spec.direction {
                Direction::Right => (along, across),
                Direction::Down => (across, along),
            };
            let centre = point(along_at, across_at);
            let centre = Point {
                x: centre.x.round(),
                y: centre.y.round(),
            };
            Some(Rect {
                x: centre.x - width / 2.0,
                y: centre.y - height / 2.0,
                w: width,
                h: height,
            })
        })
        .collect();
    // Whole pixels: cards and paths land on the pixel grid, so a line one
    // pixel wide covers whole pixels once the renderer shifts the drawing by
    // half of one (render, `HALF_PIXEL`). Card sizes are whole already.
    let nodes: Vec<Rect> = nodes
        .into_iter()
        .map(|r| Rect {
            x: r.x.round(),
            y: r.y.round(),
            ..r
        })
        .collect();
    for path in &mut paths {
        for p in path.iter_mut() {
            *p = Point {
                x: p.x.round(),
                y: p.y.round(),
            };
        }
    }
    let w = nodes
        .iter()
        .chain(labels.iter().flatten())
        .map(Rect::right)
        .chain(paths.iter().flatten().map(|p| p.x))
        .fold(0.0, f64::max);
    let h = nodes
        .iter()
        .chain(labels.iter().flatten())
        .map(Rect::bottom)
        .chain(paths.iter().flatten().map(|p| p.y))
        .fold(0.0, f64::max);
    Ok(Placement {
        nodes,
        edges: paths,
        labels,
        layers: layer,
        size: Size { w, h },
    })
}

/// Appends `p` to an orthogonal path, dropping repeats and merging a point
/// that only continues the previous segment in a straight line.
fn push_point(path: &mut Vec<Point>, p: Point) {
    const EPS: f64 = 1e-6;
    if path
        .last()
        .is_some_and(|q| (q.x - p.x).abs() < EPS && (q.y - p.y).abs() < EPS)
    {
        return;
    }
    if path.len() >= 2 {
        let (a, b) = (path[path.len() - 2], path[path.len() - 1]);
        let straight = ((a.x - b.x).abs() < EPS && (b.x - p.x).abs() < EPS)
            || ((a.y - b.y).abs() < EPS && (b.y - p.y).abs() < EPS);
        if straight {
            path.pop();
        }
    }
    path.push(p);
}
