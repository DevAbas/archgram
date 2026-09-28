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
//! Before step 1, the spec is split into units that share nothing: no
//! edge, no hint group, no frame (`pack`). Each unit with edges runs the
//! steps on its own; units without edges are set out as grids; the largest
//! unit comes first and the others are packed in rows below it.
//!
//! The layout is computed as if the flow ran right: "main" is along the
//! flow, "cross" across it. A flow running down swaps the two at the end.
//! Frames are laid out within these steps (`frames`, ARCHITECTURE.md,
//! Layout, step 5): border vertices keep each one a rectangle.

mod acyclic;
pub(crate) mod frames;
pub mod legend;
mod order;
mod pack;
mod position;
mod rank;
mod route;

use crate::error::SpecError;
use crate::geometry::{Point, Rect, Size};
use crate::spec::{Direction, Spec, Variant};
use crate::tokens::{
    ARROWHEAD_GAP, ARROWHEAD_LENGTH, CARD_MULTI_OFFSET, ROUNDED_CARD, ROUNDED_CONNECTOR, SPACING_EDGE_EDGE,
    SPACING_FRAME_LABEL, SPACING_FRAME_PADDING, SPACING_LAYER_LAYER, SPACING_NODE_NODE,
};

/// Where everything goes.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    /// Each node's footprint in spec order: its card, and for several
    /// instances the stack behind it (`measure::card_size`).
    pub nodes: Vec<Rect>,
    /// Each edge's path in spec order: an orthogonal line from a side of its
    /// `from` node to a side of its `to` node.
    pub edges: Vec<Vec<Point>>,
    /// Each edge's label box in spec order, on its path; `None` without a label.
    pub labels: Vec<Option<Rect>>,
    /// The layer of each node within its unit, in spec order.
    pub layers: Vec<usize>,
    /// The unit each node belongs to, in spec order: parts of the diagram
    /// that share nothing are laid out apart (`pack`).
    pub units: Vec<usize>,
    /// Each frame's box in spec order; `None` for a frame this placement
    /// does not hold.
    pub frames: Vec<Option<Rect>>,
    /// Each frame's name box, at its top left, in spec order.
    pub frame_labels: Vec<Option<Rect>>,
    /// The legend's entries, under the diagram; none when it has none.
    pub legend: Vec<legend::Entry>,
    /// The flows in words, under the legend, when the still image lists them.
    pub flow_lines: Vec<legend::FlowLine>,
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
pub fn place(spec: &Spec, sizes: &[Size]) -> Result<Placement, Vec<SpecError>> {
    let mut placement = place_units(spec, sizes)?;
    let (entries, flow_lines, size) = legend::place(spec, placement.size);
    placement.legend = entries;
    placement.flow_lines = flow_lines;
    placement.size = size;
    Ok(placement)
}

/// The nodes, edges and frames: one unit laid out, or several packed.
fn place_units(spec: &Spec, sizes: &[Size]) -> Result<Placement, Vec<SpecError>> {
    let units = pack::units(spec);
    if units.len() <= 1 {
        return lay_out(spec, sizes);
    }
    // Hints are checked on the whole spec, so an error points at the spec's
    // own hint. The units would find the same: cycle removal treats each
    // connected part on its own anyway (`acyclic`).
    let (dag, _) = acyclic_edges(spec);
    let errors = hint_errors(spec, &dag);
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(pack::pack(spec, sizes, &units, |sub, sub_sizes| {
        lay_out(sub, sub_sizes).expect("hints checked on the whole spec")
    }))
}

/// Each edge as (from, to) node indices, turned so the graph has no cycle,
/// and whether each was turned.
fn acyclic_edges(spec: &Spec) -> (Vec<(usize, usize)>, Vec<bool>) {
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
    let reversed = acyclic::reversed_edges(spec.nodes.len(), &edges);
    let dag = edges
        .iter()
        .zip(&reversed)
        .map(|(&(a, b), &r)| if r { (b, a) } else { (a, b) })
        .collect();
    (dag, reversed)
}

/// Layout hints that contradict the edges, located in the spec; validation
/// reports them once every edge and hint names a node.
///
/// # Panics
///
/// When an edge or hint names a node that does not exist.
pub(crate) fn contradicted_hints(spec: &Spec) -> Vec<SpecError> {
    let (dag, _) = acyclic_edges(spec);
    hint_errors(spec, &dag)
}

/// Layout hints that contradict the edges, located in the spec.
fn hint_errors(spec: &Spec, dag: &[(usize, usize)]) -> Vec<SpecError> {
    let index = |id: &str| {
        spec.nodes
            .iter()
            .position(|nd| nd.id == id)
            .expect("validated id")
    };
    let mut errors = Vec::new();
    // The groups merged so far, as layering merges them. A group joins only
    // when no path of edges leads from one of its nodes to another, so the
    // merged graph keeps no cycle and every edge still points to a later
    // layer.
    let mut group: Vec<usize> = (0..spec.nodes.len()).collect();
    for (j, members) in spec.hints.same_layer.iter().enumerate() {
        let ids: Vec<usize> = members.iter().map(|m| index(m)).collect();
        let before = errors.len();
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
        if errors.len() > before {
            continue;
        }
        if let Some((u, v)) = ordered_pair(&group, dag, &ids) {
            let (u, v) = (&spec.nodes[u].id, &spec.nodes[v].id);
            errors.push(SpecError::at(
                format!("/hints/sameLayer/{j}"),
                format!("`{u}` and `{v}` cannot share a layer: the edges lead from `{u}` to `{v}`"),
            ));
            continue;
        }
        merge(&mut group, &ids);
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
    errors
}

/// Puts `ids` in one group, named by its lowest-numbered member.
fn merge(group: &mut [usize], ids: &[usize]) {
    let head = ids.iter().map(|&i| group[i]).min().unwrap_or(0);
    for &i in ids {
        let old = group[i];
        for g in group.iter_mut() {
            if *g == old {
                *g = head;
            }
        }
    }
}

/// Two of `ids`, in different groups, the first reaching the second along
/// the edges between groups; the first such pair in the list's order.
fn ordered_pair(group: &[usize], dag: &[(usize, usize)], ids: &[usize]) -> Option<(usize, usize)> {
    for &u in ids {
        let mut reached = vec![false; group.len()];
        let mut stack = vec![group[u]];
        while let Some(g) = stack.pop() {
            for &(a, b) in dag {
                if group[a] == g && group[b] != g && !reached[group[b]] {
                    reached[group[b]] = true;
                    stack.push(group[b]);
                }
            }
        }
        if let Some(&v) = ids.iter().find(|&&v| group[v] != group[u] && reached[group[v]]) {
            return Some((u, v));
        }
    }
    None
}

/// Lays out one connected unit (all of the spec when it is one), steps 1 to 6.
#[allow(clippy::too_many_lines)] // the steps above, in order
fn lay_out(spec: &Spec, sizes: &[Size]) -> Result<Placement, Vec<SpecError>> {
    let n = spec.nodes.len();
    let index = |id: &str| {
        spec.nodes
            .iter()
            .position(|nd| nd.id == id)
            .expect("validated id")
    };
    let (main_size, cross_size): (Vec<f64>, Vec<f64>) = match spec.direction {
        Direction::Right => sizes.iter().map(|s| (s.w, s.h)).unzip(),
        Direction::Down => sizes.iter().map(|s| (s.h, s.w)).unzip(),
    };

    // 1. Cycle removal.
    let (dag, reversed) = acyclic_edges(spec);

    // 2. Layering, with sameLayer groups merged into one vertex each.
    let errors = hint_errors(spec, &dag);
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut group = (0..n).collect::<Vec<_>>();
    for members in &spec.hints.same_layer {
        let ids: Vec<usize> = members.iter().map(|m| index(m)).collect();
        merge(&mut group, &ids);
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
    // Frames: the frame of each dummy, and border vertices on every layer a
    // frame spans (`frames`).
    let fr = frames::build(spec, &layer, &chains, &mut vertex_layer);
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
    // Each vertex's anchor is the middle of its cross extent, except for a
    // stack of several instances: its anchor is the middle of its front card,
    // so a straight edge meets the card a reader sees first. The stack steps
    // up and to the right, so the extra extent lies before the anchor when
    // the flow runs right (up) and after it when the flow runs down (right).
    let stack: Vec<f64> = (0..total)
        .map(|v| {
            if v < n && spec.nodes[v].variant == Variant::Multi {
                match spec.direction {
                    Direction::Right => CARD_MULTI_OFFSET,
                    Direction::Down => -CARD_MULTI_OFFSET,
                }
            } else {
                0.0
            }
        })
        .collect();
    let cross_lo: Vec<f64> = (0..total).map(|v| cross_sizes[v] / 2.0 + stack[v]).collect();
    let cross_hi: Vec<f64> = (0..total).map(|v| cross_sizes[v] / 2.0 - stack[v]).collect();
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
    let layers = order::order(
        &layers,
        &short,
        total,
        &groups,
        &order::Clusters {
            frame_of: &fr.frame_of,
            parent: &fr.parent,
            border: &fr.border,
        },
    );

    // 5. Coordinates.
    let kind: Vec<position::Kind> = (0..total)
        .map(|v| match fr.border[v] {
            Some((f, true)) => position::Kind::First(f),
            Some((f, false)) => position::Kind::Last(f),
            None if v < n => position::Kind::Card,
            None => position::Kind::Dummy,
        })
        .collect();
    // A frame's name sits at its top: across the layers when the flow runs
    // right, so the padding before its contents makes room for it; along
    // them when the flow runs down (the gaps make room, below).
    let name_width: Vec<f64> = spec.frames.iter().map(|f| frames::name_width(&f.label)).collect();
    let name_room = SPACING_FRAME_LABEL;
    let pad_lo: Vec<f64> = spec
        .frames
        .iter()
        .map(|_| match spec.direction {
            Direction::Right => SPACING_FRAME_PADDING + name_room,
            Direction::Down => SPACING_FRAME_PADDING,
        })
        .collect();
    let pad_hi = vec![SPACING_FRAME_PADDING; spec.frames.len()];
    let mut border_up = vec![None; total];
    let mut border_down = vec![None; total];
    let mut extra = Vec::new();
    for (f, pairs) in fr.borders.iter().enumerate() {
        for w in pairs.windows(2) {
            border_up[w[1].0] = Some(w[0].0);
            border_up[w[1].1] = Some(w[0].1);
            border_down[w[0].0] = Some(w[1].0);
            border_down[w[0].1] = Some(w[1].1);
        }
        // Flowing down, the frame is at least as wide as its name.
        if let (Some(&(first, last)), Direction::Down) = (pairs.first(), spec.direction) {
            extra.push((first, last, name_width[f] + 2.0 * SPACING_FRAME_PADDING));
        }
    }
    let cross = position::coordinates(
        &layers,
        &short,
        &position::Vertices {
            lo: &cross_lo,
            hi: &cross_hi,
            kind: &kind,
            node_gap: SPACING_NODE_NODE,
            edge_gap: SPACING_EDGE_EDGE,
            pad_lo: &pad_lo,
            pad_hi: &pad_hi,
            border_up: &border_up,
            border_down: &border_down,
            extra: &extra,
        },
    );
    // Along the flow, a stack of several instances reaches past its front
    // card: to the right when the flow runs right, up when it runs down.
    let multi = |v: usize| v < n && spec.nodes[v].variant == Variant::Multi;
    let stack_before = |v: usize| {
        if multi(v) && spec.direction == Direction::Down {
            2.0 * CARD_MULTI_OFFSET
        } else {
            0.0
        }
    };
    let stack_after = |v: usize| {
        if multi(v) && spec.direction == Direction::Right {
            2.0 * CARD_MULTI_OFFSET
        } else {
            0.0
        }
    };
    // Cards in one layer share its deepest front card's size along the
    // flow, the width of a column when the flow runs right, so the sides of
    // the cards a reader sees line up; stacks reach past them.
    let card_depth: Vec<f64> = layers
        .iter()
        .map(|l| {
            l.iter()
                .filter(|&&v| v < n)
                .map(|&v| main_size[v] - stack_before(v) - stack_after(v))
                .fold(0.0, f64::max)
        })
        .collect();
    let reach_before: Vec<f64> = layers
        .iter()
        .map(|l| l.iter().map(|&v| stack_before(v)).fold(0.0, f64::max))
        .collect();
    let reach_after: Vec<f64> = layers
        .iter()
        .map(|l| l.iter().map(|&v| stack_after(v)).fold(0.0, f64::max))
        .collect();
    let depth: Vec<f64> = layers
        .iter()
        .enumerate()
        .map(|(l, vs)| {
            vs.iter()
                .map(|&v| main_sizes[v])
                .fold(card_depth[l] + reach_before[l] + reach_after[l], f64::max)
        })
        .collect();

    // 6. Routing: ports on each card's sides, then each hop between two layers
    //    straight or as a Z through a track in the gap (`route`).
    let hops: Vec<(usize, usize, usize)> = chains
        .iter()
        .enumerate()
        .flat_map(|(e, chain)| chain.windows(2).map(move |w| (e, w[0], w[1])))
        .collect();
    // For a card, its hops on one side, ordered by where their other ends lie
    // across the layers, get ports spread around the side's middle. The
    // plain ones share a single port, as a bundle: edges leaving a side
    // leave as one trunk and fork in the gap; edges entering a side merge
    // into one point. An edge with its label near the card, and an edge
    // drawn against the flow (its arrowhead would sit among lines leaving),
    // keeps a port of its own.
    let mut out_port = vec![0.0; hops.len()];
    let mut in_port = vec![0.0; hops.len()];
    let mut out_bundle: Vec<Option<usize>> = vec![None; hops.len()];
    let mut in_bundle: Vec<Option<usize>> = vec![None; hops.len()];
    for v in 0..total {
        for (outgoing, ports_of) in [(true, &mut out_port), (false, &mut in_port)] {
            let mine: Vec<usize> = (0..hops.len())
                .filter(|&h| if outgoing { hops[h].1 == v } else { hops[h].2 == v })
                .collect();
            if v >= n {
                for h in mine {
                    ports_of[h] = cross[v];
                }
                continue;
            }
            let other = |h: usize| {
                if outgoing {
                    cross[hops[h].2]
                } else {
                    cross[hops[h].1]
                }
            };
            // A label on an edge between neighbouring layers sits just past
            // the card it leaves, across its line: its port leaves it room.
            let reach = |h: usize| {
                let e = hops[h].0;
                match label_extent[e] {
                    Some((_, across)) if outgoing && chains[e].len() == 2 => across / 2.0,
                    _ => 0.0,
                }
            };
            let alone = |h: usize| reach(h) > 0.0 || reversed[hops[h].0];
            // One entry per port: the bundle, then each hop on its own,
            // ordered by where their other ends lie (a bundle by its middle).
            let bundle: Vec<usize> = mine.iter().copied().filter(|&h| !alone(h)).collect();
            let mut entries: Vec<(f64, Vec<usize>)> = mine
                .iter()
                .copied()
                .filter(|&h| alone(h))
                .map(|h| (other(h), vec![h]))
                .collect();
            if !bundle.is_empty() {
                let lo = bundle.iter().map(|&h| other(h)).fold(f64::INFINITY, f64::min);
                let hi = bundle.iter().map(|&h| other(h)).fold(f64::NEG_INFINITY, f64::max);
                entries.push((f64::midpoint(lo, hi), bundle.clone()));
                if bundle.len() > 1 {
                    for &h in &bundle {
                        if outgoing {
                            out_bundle[h] = Some(v);
                        } else {
                            in_bundle[h] = Some(v);
                        }
                    }
                }
            }
            entries.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1[0].cmp(&b.1[0])));
            let reach: Vec<f64> = entries.iter().map(|(_, hs)| reach(hs[0])).collect();
            // Ports stay on the front card's side, centred on its anchor.
            let places = route::ports(
                &reach,
                cross[v],
                2.0 * cross_lo[v].min(cross_hi[v]),
                SPACING_EDGE_EDGE,
                ROUNDED_CARD,
            );
            for ((_, hs), p) in entries.iter().zip(places) {
                for &h in hs {
                    ports_of[h] = p;
                }
            }
        }
    }
    // Tracks per gap; a gap with more tracks than its width holds is widened.
    let level = |h: usize| (out_port[h] - in_port[h]).abs() < 0.5;
    let gaps = layers.len().saturating_sub(1);
    let mut track = vec![0usize; hops.len()];
    let mut track_count = vec![0usize; gaps];
    for (g, count) in track_count.iter_mut().enumerate() {
        // A bundle turns at one track, so its trunk forks, or its branches
        // merge, in one place. A hop in both kinds follows the one leaving.
        let mut units: Vec<Vec<usize>> = Vec::new();
        let mut unit_of_bundle: std::collections::BTreeMap<(bool, usize), usize> =
            std::collections::BTreeMap::new();
        for h in (0..hops.len()).filter(|&h| vertex_layer[hops[h].1] == g && !level(h)) {
            let key = match (out_bundle[h], in_bundle[h]) {
                (Some(b), _) => Some((true, b)),
                (None, Some(b)) => Some((false, b)),
                (None, None) => None,
            };
            match key {
                Some(b) => {
                    let u = *unit_of_bundle.entry(b).or_insert_with(|| {
                        units.push(Vec::new());
                        units.len() - 1
                    });
                    units[u].push(h);
                }
                None => units.push(vec![h]),
            }
        }
        let risers: Vec<route::Riser> = units
            .iter()
            .map(|hs| {
                if hs.len() > 1 && out_bundle[hs[0]].is_none() {
                    // Branches merging: the shared port is where they arrive.
                    let starts: Vec<f64> = hs.iter().map(|&h| out_port[h]).collect();
                    route::Riser::bundle(in_port[hs[0]], &starts)
                } else {
                    let ends: Vec<f64> = hs.iter().map(|&h| in_port[h]).collect();
                    route::Riser::bundle(out_port[hs[0]], &ends)
                }
            })
            .collect();
        let (t, c) = route::tracks(&risers, SPACING_EDGE_EDGE);
        for (hs, t) in units.iter().zip(t) {
            for &h in hs {
                track[h] = t;
            }
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
    // Tracks keep clear of the cards on both sides of the gap: a bend's
    // radius after the card an edge leaves (or its label's room), and before
    // the card it points at a bend's radius, twice the arrowhead's length
    // and its gap, so the last bend is whole and the arrowhead sits on a
    // straight run as long as itself.
    //
    // Frames along the flow: a frame keeps its padding before its first
    // layer (and its name, when the flow runs down) and after its last, in
    // the gaps; a frame nested in one that starts or ends on the same layer
    // adds its own inside. Flowing right, a frame shorter than its name
    // grows at its end. Tracks stay outside the frames starting and ending
    // in their gap.
    let approach = ROUNDED_CONNECTOR + 2.0 * ARROWHEAD_LENGTH + ARROWHEAD_GAP;
    let mut inner_first: Vec<usize> = (0..spec.frames.len()).filter(|&f| fr.present[f]).collect();
    inner_first.sort_by_key(|&f| (std::cmp::Reverse(fr.depth(f)), f));
    let mut stretch = vec![0.0f64; spec.frames.len()];
    let (mut start, mut gap_width, mut depart, mut approach_to) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut start_depth = vec![0.0f64; spec.frames.len()];
    let mut end_depth = vec![0.0f64; spec.frames.len()];
    for _ in 0..3 {
        start_depth.fill(0.0);
        end_depth.fill(0.0);
        for &f in &inner_first {
            let name = if spec.direction == Direction::Down {
                name_room
            } else {
                0.0
            };
            start_depth[f] += SPACING_FRAME_PADDING + name;
            end_depth[f] += SPACING_FRAME_PADDING + stretch[f];
            if let Some(p) = fr.parent[f] {
                if fr.span[p].0 == fr.span[f].0 {
                    start_depth[p] = start_depth[p].max(start_depth[f]);
                }
                if fr.span[p].1 == fr.span[f].1 {
                    end_depth[p] = end_depth[p].max(end_depth[f]);
                }
            }
        }
        let mut start_region = vec![0.0f64; layers.len()];
        let mut end_region = vec![0.0f64; layers.len()];
        for &f in &inner_first {
            start_region[fr.span[f].0] = start_region[fr.span[f].0].max(start_depth[f]);
            end_region[fr.span[f].1] = end_region[fr.span[f].1].max(end_depth[f]);
        }
        depart = (0..gaps)
            .map(|g| lead[g].max(ROUNDED_CONNECTOR) + end_region[g])
            .collect();
        approach_to = (0..gaps).map(|g| start_region[g + 1] + approach).collect();
        #[allow(clippy::cast_precision_loss)] // track counts are small
        let widths: Vec<f64> = (0..gaps)
            .map(|g| {
                let tracks = track_count[g].saturating_sub(1) as f64 * SPACING_EDGE_EDGE;
                SPACING_LAYER_LAYER.max(depart[g] + tracks + approach_to[g])
            })
            .collect();
        gap_width = widths;
        start = Vec::with_capacity(layers.len());
        let mut at = start_region.first().copied().unwrap_or(0.0);
        for (i, d) in depth.iter().enumerate() {
            start.push(at);
            at += d + gap_width.get(i).copied().unwrap_or(0.0);
        }
        let mut grew = false;
        if spec.direction == Direction::Right {
            for &f in &inner_first {
                let (s0, s1) = fr.span[f];
                let length = start[s1] + depth[s1] + end_depth[f] - (start[s0] - start_depth[f]);
                let need = name_width[f] + 2.0 * SPACING_FRAME_PADDING;
                if length + 0.5 < need {
                    stretch[f] += need - length;
                    grew = true;
                }
            }
        }
        if !grew {
            break;
        }
    }
    // A card's front is centred in its layer with its stacks' reach; a
    // dummy (a long edge's bend, or its label) at the layer's middle.
    let centre_main = |v: usize| {
        let l = vertex_layer[v];
        if v < n {
            let block = card_depth[l] + reach_before[l] + reach_after[l];
            start[l] + (depth[l] - block) / 2.0 + reach_before[l] + card_depth[l] / 2.0
        } else {
            start[l] + depth[l] / 2.0
        }
    };
    // Where a hop leaves and enters, along the flow: a card's far or near
    // side, past its stack; a dummy's centre, so a long edge runs straight
    // through its layers.
    let leave_main = |v: usize| {
        if v < n {
            centre_main(v) + card_depth[vertex_layer[v]] / 2.0 + stack_after(v)
        } else {
            centre_main(v)
        }
    };
    let enter_main = |v: usize| {
        if v < n {
            centre_main(v) - card_depth[vertex_layer[v]] / 2.0 - stack_before(v)
        } else {
            centre_main(v)
        }
    };

    // Into the drawing's frame, the smallest coordinate at 0.
    let cross_min = (0..total)
        .map(|v| cross[v] - cross_lo[v])
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
            let from = point(enter_main(v), cross[v] - cross_lo[v]);
            let to = point(leave_main(v), cross[v] + cross_hi[v]);
            Rect {
                x: from.x,
                y: from.y,
                w: to.x - from.x,
                h: to.y - from.y,
            }
        })
        .collect();
    let mut paths: Vec<Vec<Point>> = vec![Vec::new(); chains.len()];
    for (h, &(e, a, b)) in hops.iter().enumerate() {
        let g = vertex_layer[a];
        let mut pts = vec![(leave_main(a), out_port[h])];
        if !level(h) {
            // Tracks spread over the part of the gap between the two runs,
            // the only one in the middle of it.
            let (first, room) = (
                start[g] + depth[g] + depart[g],
                gap_width[g] - depart[g] - approach_to[g],
            );
            #[allow(clippy::cast_precision_loss)] // track counts are small
            let t = if track_count[g] > 1 {
                first + room * track[h] as f64 / (track_count[g] - 1) as f64
            } else {
                first + room / 2.0
            };
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
                let (_, a, _) = hops[hop];
                // In the room the gap keeps before its tracks, level or not:
                // another edge's riser may stand in the rest of the gap.
                let gap = vertex_layer[a];
                let end = start[gap] + depth[gap] + lead[gap];
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
        let rounded: Vec<Point> = path
            .iter()
            .map(|p| Point {
                x: p.x.round(),
                y: p.y.round(),
            })
            .collect();
        // Rounding can make neighbours meet: merge them again.
        path.clear();
        for p in rounded {
            push_point(path, p);
        }
    }
    // Each frame from its borders across the layers and its room in the gaps
    // along them, on whole pixels; its name at its top left, centred in the
    // room kept at its top.
    let frame_rects: Vec<Option<Rect>> = (0..spec.frames.len())
        .map(|f| {
            if !fr.present[f] {
                return None;
            }
            let (s0, s1) = fr.span[f];
            let lo = fr.borders[f]
                .iter()
                .map(|&(a, _)| cross[a])
                .fold(f64::INFINITY, f64::min);
            let hi = fr.borders[f]
                .iter()
                .map(|&(_, b)| cross[b])
                .fold(f64::NEG_INFINITY, f64::max);
            let from = point(start[s0] - start_depth[f], lo);
            let to = point(start[s1] + depth[s1] + end_depth[f], hi);
            let (x, y) = (from.x.round(), from.y.round());
            Some(Rect {
                x,
                y,
                w: to.x.round() - x,
                h: to.y.round() - y,
            })
        })
        .collect();
    let frame_labels: Vec<Option<Rect>> = frame_rects
        .iter()
        .enumerate()
        .map(|(f, r)| r.map(|r| frames::name_box(r, name_width[f])))
        .collect();
    let w = nodes
        .iter()
        .chain(labels.iter().flatten())
        .chain(frame_rects.iter().flatten())
        .map(Rect::right)
        .chain(paths.iter().flatten().map(|p| p.x))
        .fold(0.0, f64::max);
    let h = nodes
        .iter()
        .chain(labels.iter().flatten())
        .chain(frame_rects.iter().flatten())
        .map(Rect::bottom)
        .chain(paths.iter().flatten().map(|p| p.y))
        .fold(0.0, f64::max);
    Ok(Placement {
        nodes,
        edges: paths,
        labels,
        units: vec![0; layer.len()],
        frames: frame_rects,
        frame_labels,
        legend: Vec::new(),
        flow_lines: Vec::new(),
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
