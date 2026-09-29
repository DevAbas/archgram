//! Component packing (ARCHITECTURE.md, Layout): a diagram made of parts
//! that share nothing is laid out part by part, as Graphviz `pack` and
//! ELK's `separateConnectedComponents` do, so a node without edges does not
//! stand in the first column of an unrelated flow.
//!
//! A unit is a set of nodes joined by edges, hint groups (`sameLayer`,
//! `order`) or a top-level frame. A unit with edges gets the full layered
//! layout; a unit without edges is set out as a grid, its cells as wide as
//! its widest card. The unit with the most nodes (the earliest on a tie)
//! comes first; the others follow in the order of their first node, in rows
//! below it no wider than it (or than the widest unit), `spacing.pack`
//! apart. When no unit has edges, all the nodes form one grid, about as
//! many columns as rows.

use crate::geometry::{Point, Rect, Size};
use crate::layout::{Placement, frames};
use crate::spec::Variant;
use crate::spec::{Edge, Hints, Spec};
use crate::tokens::{
    CARD_MULTI_OFFSET, SPACING_FRAME_LABEL, SPACING_FRAME_PADDING, SPACING_NODE_NODE, SPACING_PACK,
};

/// The units, each a sorted list of node indices, in order of their first
/// node.
pub fn units(spec: &Spec) -> Vec<Vec<usize>> {
    let n = spec.nodes.len();
    let index = |id: &str| {
        spec.nodes
            .iter()
            .position(|nd| nd.id == id)
            .expect("validated id")
    };
    let mut parent: Vec<usize> = (0..n).collect();
    let mut join = |a: usize, b: usize| {
        let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
        if ra != rb {
            parent[ra.max(rb)] = ra.min(rb);
        }
    };
    for e in &spec.edges {
        join(index(&e.from), index(&e.to));
    }
    for group in spec.hints.same_layer.iter().chain(&spec.hints.order) {
        for pair in group.windows(2) {
            join(index(&pair[0]), index(&pair[1]));
        }
    }
    // Nodes of one top-level frame stay together.
    let top = |id: &str| {
        let mut f = spec
            .frames
            .iter()
            .find(|f| f.id == id)
            .expect("validated frame");
        while let Some(p) = &f.parent {
            f = spec
                .frames
                .iter()
                .find(|g| &g.id == p)
                .expect("validated frame");
        }
        f.id.clone()
    };
    let mut first_in_frame: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for (v, node) in spec.nodes.iter().enumerate() {
        if let Some(f) = &node.frame {
            let t = top(f);
            match first_in_frame.get(&t) {
                Some(&u) => join(u, v),
                None => {
                    first_in_frame.insert(t, v);
                }
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<usize>> =
        std::collections::BTreeMap::new();
    for v in 0..n {
        let r = root(&mut parent, v);
        groups.entry(r).or_default().push(v);
    }
    groups.into_values().collect()
}

/// The root of `v`'s set, compressing the path.
fn root(parent: &mut [usize], v: usize) -> usize {
    let mut r = v;
    while parent[r] != r {
        r = parent[r];
    }
    let mut v = v;
    while parent[v] != r {
        let next = parent[v];
        parent[v] = r;
        v = next;
    }
    r
}

/// The part of `spec` a unit covers: its nodes, the edges between them and
/// the hints that name them, with the spec indices of its edges.
fn sub_spec(spec: &Spec, unit: &[usize]) -> (Spec, Vec<usize>) {
    let ids: Vec<&str> = unit.iter().map(|&v| spec.nodes[v].id.as_str()).collect();
    let inside = |id: &String| ids.contains(&id.as_str());
    let edge_index: Vec<usize> = (0..spec.edges.len())
        .filter(|&k| inside(&spec.edges[k].from))
        .collect();
    let edges: Vec<Edge> = edge_index.iter().map(|&k| spec.edges[k].clone()).collect();
    let keep = |groups: &[Vec<String>]| -> Vec<Vec<String>> {
        groups
            .iter()
            .filter(|g| g.iter().any(inside))
            .cloned()
            .collect()
    };
    let hints = Hints {
        first: spec
            .hints
            .first
            .iter()
            .filter(|id| inside(id))
            .cloned()
            .collect(),
        last: spec
            .hints
            .last
            .iter()
            .filter(|id| inside(id))
            .cloned()
            .collect(),
        same_layer: keep(&spec.hints.same_layer),
        order: keep(&spec.hints.order),
    };
    let sub = Spec {
        nodes: unit.iter().map(|&v| spec.nodes[v].clone()).collect(),
        edges,
        hints,
        ..spec.clone()
    };
    (sub, edge_index)
}

/// Cards in a grid: rows of `columns`, every front card as wide as the
/// widest and each row as tall as the tallest, `spacing.node-node` apart.
/// A stack of several instances (`multi`) reaches up and to the right of
/// its front card, into the space between cells.
fn grid(sizes: &[Size], multi: &[bool], columns: usize) -> (Vec<Rect>, Size) {
    let stack = 2.0 * CARD_MULTI_OFFSET;
    let front = |i: usize| {
        let s = sizes[i];
        if multi[i] {
            Size {
                w: s.w - stack,
                h: s.h - stack,
            }
        } else {
            s
        }
    };
    let cell_w = (0..sizes.len()).map(|i| front(i).w).fold(0.0, f64::max);
    let cell_h = (0..sizes.len()).map(|i| front(i).h).fold(0.0, f64::max);
    let top = if multi.iter().any(|&m| m) { stack } else { 0.0 };
    let columns = columns.clamp(1, sizes.len().max(1));
    #[allow(clippy::cast_precision_loss)] // cell counts are small
    let rects: Vec<Rect> = (0..sizes.len())
        .map(|i| {
            let x = (i % columns) as f64 * (cell_w + SPACING_NODE_NODE);
            let y = top + (i / columns) as f64 * (cell_h + SPACING_NODE_NODE);
            let reach = if multi[i] { stack } else { 0.0 };
            Rect {
                x,
                y: y - reach,
                w: cell_w + reach,
                h: front(i).h + reach,
            }
        })
        .collect();
    let w = rects.iter().map(Rect::right).fold(0.0, f64::max);
    let h = rects.iter().map(Rect::bottom).fold(0.0, f64::max);
    (rects, Size { w, h })
}

/// Whether each of `nodes` is a stack of several instances.
fn stacks(spec: &Spec, nodes: &[usize]) -> Vec<bool> {
    nodes
        .iter()
        .map(|&v| spec.nodes[v].variant == Variant::Multi)
        .collect()
}

/// How many cells of `cell` width fit in `limit`, at least one.
fn columns_within(limit: f64, cell: f64) -> usize {
    let fit = ((limit + SPACING_NODE_NODE) / (cell + SPACING_NODE_NODE)).floor();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // a small, positive count
    let fit = fit.max(1.0) as usize;
    fit
}

/// A unit without edges or frames: a grid no wider than `limit`.
fn plain_grid(spec: &Spec, unit: &[usize], sizes: &[Size], limit: f64) -> Placement {
    let unit_sizes: Vec<Size> = unit.iter().map(|&v| sizes[v]).collect();
    let cell = unit_sizes.iter().map(|s| s.w).fold(0.0, f64::max);
    let (rects, size) = grid(
        &unit_sizes,
        &stacks(spec, unit),
        columns_within(limit, cell),
    );
    Placement {
        nodes: rects,
        edges: Vec::new(),
        labels: Vec::new(),
        layers: vec![0; unit.len()],
        units: vec![0; unit.len()],
        frames: vec![None; spec.frames.len()],
        frame_labels: vec![None; spec.frames.len()],
        legend: Vec::new(),
        flow_lines: Vec::new(),
        credit: None,
        size,
    }
}

/// A unit without edges whose nodes all sit directly in one frame: a grid
/// inside that frame, and inside each frame around it, each with its
/// padding and its name at the top; no frame narrower than its name.
/// `None` when the nodes do not share one frame.
fn framed_grid(spec: &Spec, unit: &[usize], sizes: &[Size], limit: f64) -> Option<Placement> {
    let first = spec.nodes[unit[0]].frame.as_deref()?;
    if unit
        .iter()
        .any(|&v| spec.nodes[v].frame.as_deref() != Some(first))
    {
        return None;
    }
    let index = |id: &str| {
        spec.frames
            .iter()
            .position(|f| f.id == id)
            .expect("validated frame")
    };
    let mut chain = vec![index(first)];
    while let Some(p) = &spec.frames[chain[chain.len() - 1]].parent {
        chain.push(index(p));
    }
    #[allow(clippy::cast_precision_loss)] // nesting is shallow
    let inset = 2.0 * SPACING_FRAME_PADDING * chain.len() as f64;
    let unit_sizes: Vec<Size> = unit.iter().map(|&v| sizes[v]).collect();
    let cell = unit_sizes.iter().map(|s| s.w).fold(0.0, f64::max);
    let (rects, size) = grid(
        &unit_sizes,
        &stacks(spec, unit),
        columns_within(limit - inset, cell),
    );
    let top = SPACING_FRAME_PADDING + SPACING_FRAME_LABEL;
    let mut inner = Rect {
        x: 0.0,
        y: 0.0,
        w: size.w,
        h: size.h,
    };
    let mut boxes = Vec::new();
    for &f in &chain {
        let name = frames::name_width(&spec.frames[f].label);
        inner = Rect {
            x: inner.x - SPACING_FRAME_PADDING,
            y: inner.y - top,
            w: (inner.w + 2.0 * SPACING_FRAME_PADDING).max(name + 2.0 * SPACING_FRAME_PADDING),
            h: inner.h + top + SPACING_FRAME_PADDING,
        };
        boxes.push((f, inner, name));
    }
    let shift = |r: Rect| Rect {
        x: r.x - inner.x,
        y: r.y - inner.y,
        ..r
    };
    let mut frames_out = vec![None; spec.frames.len()];
    let mut labels_out = vec![None; spec.frames.len()];
    for (f, b, name) in boxes {
        let b = shift(b);
        frames_out[f] = Some(b);
        labels_out[f] = Some(frames::name_box(b, name));
    }
    Some(Placement {
        nodes: rects.into_iter().map(shift).collect(),
        edges: Vec::new(),
        labels: Vec::new(),
        layers: vec![0; unit.len()],
        units: vec![0; unit.len()],
        frames: frames_out,
        frame_labels: labels_out,
        legend: Vec::new(),
        flow_lines: Vec::new(),
        credit: None,
        size: Size {
            w: inner.w,
            h: inner.h,
        },
    })
}

/// Where each unit goes: the primary at the origin, the others in order,
/// left to right in rows below it, a row ending before it would pass `limit`.
fn rows(primary: usize, widths: &[f64], heights: &[f64], limit: f64) -> Vec<Point> {
    let mut offset = vec![Point::default(); widths.len()];
    let mut at = Point {
        x: 0.0,
        y: heights[primary] + SPACING_PACK,
    };
    let mut row_height = 0.0f64;
    for i in (0..widths.len()).filter(|&i| i != primary) {
        if at.x > 0.0 && at.x + widths[i] > limit {
            at = Point {
                x: 0.0,
                y: at.y + row_height + SPACING_PACK,
            };
            row_height = 0.0;
        }
        offset[i] = at;
        at.x += widths[i] + SPACING_PACK;
        row_height = row_height.max(heights[i]);
    }
    offset
}

/// The units, with lone nodes that have no edges put together in one, where
/// the first of them was: they share one grid and line up in it, instead of
/// scattering in a row of their own sizes.
fn merge_loose(spec: &Spec, units: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let joined = |u: &[usize]| {
        spec.edges
            .iter()
            .any(|e| u.iter().any(|&v| spec.nodes[v].id == e.from))
    };
    let mut merged: Vec<Vec<usize>> = Vec::new();
    let mut loose: Option<usize> = None;
    for u in units {
        if u.len() == 1 && !joined(u) && spec.nodes[u[0]].frame.is_none() {
            if let Some(i) = loose {
                merged[i].push(u[0]);
            } else {
                loose = Some(merged.len());
                merged.push(u.clone());
            }
        } else {
            merged.push(u.clone());
        }
    }
    merged
}

/// Lays out every unit (units with edges by `lay_out`) and packs them.
pub fn pack(
    spec: &Spec,
    sizes: &[Size],
    units: &[Vec<usize>],
    lay_out: impl Fn(&Spec, &[Size]) -> Placement,
) -> Placement {
    let n = spec.nodes.len();
    let merged = merge_loose(spec, units);
    let units = &merged;
    let has_edges: Vec<bool> = units
        .iter()
        .map(|u| {
            spec.edges
                .iter()
                .any(|e| u.iter().any(|&v| spec.nodes[v].id == e.from))
        })
        .collect();
    let mut out = Placement {
        nodes: vec![Rect::default(); n],
        edges: vec![Vec::new(); spec.edges.len()],
        labels: vec![None; spec.edges.len()],
        layers: vec![0; n],
        units: vec![0; n],
        frames: vec![None; spec.frames.len()],
        frame_labels: vec![None; spec.frames.len()],
        legend: Vec::new(),
        flow_lines: Vec::new(),
        credit: None,
        size: Size::default(),
    };

    // No edges and no frames anywhere: one grid, about as many columns as rows.
    let framed = spec.nodes.iter().any(|node| node.frame.is_some());
    if !has_edges.iter().any(|&e| e) && !framed {
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            clippy::cast_precision_loss
        )]
        let columns = (n as f64).sqrt().ceil() as usize;
        let all: Vec<usize> = (0..n).collect();
        let (rects, size) = grid(sizes, &stacks(spec, &all), columns);
        out.nodes = rects;
        out.size = size;
        return out;
    }

    // The largest unit first; its width bounds the rows below it.
    let primary = (0..units.len())
        .max_by(|&a, &b| units[a].len().cmp(&units[b].len()).then(b.cmp(&a)))
        .expect("two units or more");
    let mut parts: Vec<Option<(Placement, Vec<usize>)>> = vec![None; units.len()];
    for (i, unit) in units.iter().enumerate() {
        if has_edges[i] {
            let (sub, edge_index) = sub_spec(spec, unit);
            let sub_sizes: Vec<Size> = unit.iter().map(|&v| sizes[v]).collect();
            parts[i] = Some((lay_out(&sub, &sub_sizes), edge_index));
        }
    }
    // The width the rows below keep within: the primary unit's, or, when it
    // has no edges either, a square grid's of every node.
    #[allow(clippy::cast_precision_loss)] // node counts are small
    let square = (n as f64).sqrt().ceil()
        * (sizes.iter().map(|s| s.w).fold(0.0, f64::max) + SPACING_NODE_NODE);
    let primary_w = parts[primary].as_ref().map_or(square, |(p, _)| p.size.w);
    for (i, unit) in units.iter().enumerate() {
        if !has_edges[i] {
            // Nodes that share one frame: a grid in it. Other framed nodes:
            // the layered layout, which lays frames out. The rest: a grid.
            let part = framed_grid(spec, unit, sizes, primary_w).unwrap_or_else(|| {
                if unit.iter().any(|&v| spec.nodes[v].frame.is_some()) {
                    let (sub, _) = sub_spec(spec, unit);
                    let sub_sizes: Vec<Size> = unit.iter().map(|&v| sizes[v]).collect();
                    lay_out(&sub, &sub_sizes)
                } else {
                    plain_grid(spec, unit, sizes, primary_w)
                }
            });
            parts[i] = Some((part, Vec::new()));
        }
    }
    let widths: Vec<f64> = parts
        .iter()
        .map(|p| p.as_ref().expect("laid out").0.size.w.ceil())
        .collect();
    let limit = widths.iter().copied().fold(primary_w.ceil(), f64::max);

    let heights: Vec<f64> = parts
        .iter()
        .map(|p| p.as_ref().expect("laid out").0.size.h.ceil())
        .collect();
    let offset = rows(primary, &widths, &heights, limit);

    for (i, part) in parts.into_iter().enumerate() {
        let (part, edge_index) = part.expect("laid out");
        copy_into(&mut out, &part, &units[i], &edge_index, i, offset[i]);
    }
    out
}

/// Copies unit `i`'s placement into the whole one, moved by `d`: its nodes
/// (`unit`) and edges (`edge_index`) at their places in the spec.
fn copy_into(
    out: &mut Placement,
    part: &Placement,
    unit: &[usize],
    edge_index: &[usize],
    i: usize,
    d: Point,
) {
    let shift = |r: Rect| Rect {
        x: r.x + d.x,
        y: r.y + d.y,
        ..r
    };
    for (k, &v) in unit.iter().enumerate() {
        out.nodes[v] = shift(part.nodes[k]);
        out.layers[v] = part.layers[k];
        out.units[v] = i;
    }
    for (k, &e) in edge_index.iter().enumerate() {
        out.edges[e] = part.edges[k]
            .iter()
            .map(|p| Point {
                x: p.x + d.x,
                y: p.y + d.y,
            })
            .collect();
        out.labels[e] = part.labels[k].map(shift);
    }
    for (f, (rect, label)) in part.frames.iter().zip(&part.frame_labels).enumerate() {
        if let Some(r) = rect {
            out.frames[f] = Some(shift(*r));
            out.frame_labels[f] = label.map(shift);
        }
    }
    out.size.w = out.size.w.max(d.x + part.size.w);
    out.size.h = out.size.h.max(d.y + part.size.h);
}
