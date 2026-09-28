//! Frames in the layered layout (ARCHITECTURE.md, Layout, step 5), after
//! dagre's compound layout (its `nesting-graph`, `parent-dummy-chains` and
//! `add-border-segments`, on Sander 1996):
//!
//! - a frame spans the layers from its first descendant node to its last;
//! - each dummy vertex of a long edge sits in the innermost frame it passes
//!   through: the walk climbs from the edge's first node through the frames
//!   that end before the dummy's layer, up to the frames both ends share,
//!   then descends into the frames of the edge's last node that have begun;
//! - on every layer it spans, a frame gets a first and a last border vertex,
//!   chained across the layers. Ordering keeps a frame's vertices between
//!   its borders and coordinates keep each chain straight, so a frame is a
//!   rectangle that holds its own and nothing else.

use crate::font::text_width;
use crate::geometry::Rect;
use crate::spec::Spec;
use crate::tokens::{SPACING_FRAME_LABEL, SPACING_FRAME_PADDING, TYPOGRAPHY_FRAME_LABEL};

/// How wide a frame's name is, set in capitals as it is drawn.
pub fn name_width(label: &str) -> f64 {
    text_width(&label.to_uppercase(), &TYPOGRAPHY_FRAME_LABEL)
}

/// Where a frame's name goes: at its top left, inside its padding, centred
/// in the room kept at its top (`spacing.frame-padding` plus
/// `spacing.frame-label`).
pub fn name_box(frame: Rect, width: f64) -> Rect {
    let height = TYPOGRAPHY_FRAME_LABEL.size * TYPOGRAPHY_FRAME_LABEL.line_height;
    Rect {
        x: frame.x + SPACING_FRAME_PADDING,
        y: frame.y + (SPACING_FRAME_PADDING + SPACING_FRAME_LABEL - height) / 2.0,
        w: width,
        h: height,
    }
}

/// The frames of one laid-out spec, and the border vertices added for them.
pub struct Frames {
    /// Each frame's parent.
    pub parent: Vec<Option<usize>>,
    /// Whether the frame holds a node of this spec; a unit's spec keeps
    /// every frame of the whole one.
    pub present: Vec<bool>,
    /// The first and last layer of each present frame.
    pub span: Vec<(usize, usize)>,
    /// Each vertex's innermost frame, border vertices included.
    pub frame_of: Vec<Option<usize>>,
    /// For a border vertex, its frame and whether it is the first border.
    pub border: Vec<Option<(usize, bool)>>,
    /// Each frame's (first, last) border vertices, one pair per layer of its
    /// span, in layer order.
    pub borders: Vec<Vec<(usize, usize)>>,
}

impl Frames {
    /// The frame and its ancestors, innermost first.
    fn ancestors(&self, f: Option<usize>) -> Vec<usize> {
        let mut out = Vec::new();
        let mut at = f;
        while let Some(g) = at {
            out.push(g);
            at = self.parent[g];
        }
        out
    }

    /// Nesting depth: 0 for a frame at the top level.
    pub fn depth(&self, f: usize) -> usize {
        self.ancestors(Some(f)).len() - 1
    }
}

/// Finds each vertex's frame and adds the border vertices, extending
/// `vertex_layer` with their layers. `layer` holds the nodes' layers,
/// `chains` each edge's vertices from its first layer to its last.
pub fn build(
    spec: &Spec,
    layer: &[usize],
    chains: &[Vec<usize>],
    vertex_layer: &mut Vec<usize>,
) -> Frames {
    let count = spec.frames.len();
    let index = |id: &str| {
        spec.frames
            .iter()
            .position(|f| f.id == id)
            .expect("validated frame")
    };
    let mut frames = Frames {
        parent: spec
            .frames
            .iter()
            .map(|f| f.parent.as_deref().map(index))
            .collect(),
        present: vec![false; count],
        span: vec![(usize::MAX, 0); count],
        frame_of: vec![None; vertex_layer.len()],
        border: vec![None; vertex_layer.len()],
        borders: vec![Vec::new(); count],
    };
    for (v, node) in spec.nodes.iter().enumerate() {
        let f = node.frame.as_deref().map(index);
        frames.frame_of[v] = f;
        for a in frames.ancestors(f) {
            frames.present[a] = true;
            let (lo, hi) = frames.span[a];
            frames.span[a] = (lo.min(layer[v]), hi.max(layer[v]));
        }
    }
    for chain in chains.iter().filter(|c| c.len() > 2) {
        let (start, end) = (
            frames.frame_of[chain[0]],
            frames.frame_of[chain[chain.len() - 1]],
        );
        let (up, down) = (frames.ancestors(start), frames.ancestors(end));
        // The frames both ends share, innermost first; the walk climbs `up`
        // to the first of them, then descends `down` from it.
        let shared = up.iter().copied().find(|f| down.contains(f));
        let climb: Vec<Option<usize>> = up
            .iter()
            .copied()
            .take_while(|&f| Some(f) != shared)
            .map(Some)
            .chain(std::iter::once(shared))
            .collect();
        let descend: Vec<Option<usize>> = std::iter::once(shared)
            .chain(
                down.iter()
                    .copied()
                    .take_while(|&f| Some(f) != shared)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .map(Some),
            )
            .collect();
        let (mut i, mut j, mut climbing) = (0, 0, true);
        for &d in &chain[1..chain.len() - 1] {
            let l = vertex_layer[d];
            if climbing {
                while i + 1 < climb.len() && climb[i].is_some_and(|f| frames.span[f].1 < l) {
                    i += 1;
                }
                climbing = i + 1 < climb.len();
            }
            if !climbing {
                while j + 1 < descend.len() && descend[j + 1].is_some_and(|f| frames.span[f].0 <= l)
                {
                    j += 1;
                }
            }
            frames.frame_of[d] = if climbing { climb[i] } else { descend[j] };
        }
    }
    for f in 0..count {
        if !frames.present[f] {
            continue;
        }
        let (lo, hi) = frames.span[f];
        for l in lo..=hi {
            let mut pair = [0usize; 2];
            for (k, first) in [true, false].into_iter().enumerate() {
                vertex_layer.push(l);
                frames.frame_of.push(Some(f));
                frames.border.push(Some((f, first)));
                pair[k] = vertex_layer.len() - 1;
            }
            frames.borders[f].push((pair[0], pair[1]));
        }
    }
    frames
}
