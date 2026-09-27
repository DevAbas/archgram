//! Placing the boxes (ARCHITECTURE.md, Layout).
//!
//! Until the layered layout exists (M4), nodes are placed in a plain grid in
//! spec order, so rendering can be built and looked at. The grid ignores
//! edges and frames on purpose: it is scaffolding, not a layout.

use crate::geometry::{Rect, Size};
use crate::spec::{Direction, Spec};
use crate::tokens::{SPACING_LAYER_LAYER, SPACING_NODE_NODE};

/// Where every node goes, in spec order, and the size of the whole drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub nodes: Vec<Rect>,
    pub size: Size,
}

/// Nodes per row (or per column when the flow runs down) in the placeholder grid.
const GRID_RUN: usize = 6;

/// Lays out the nodes with the sizes `sizes` (one per node, in spec order).
#[must_use]
pub fn place(spec: &Spec, sizes: &[Size]) -> Placement {
    let cell_w = sizes.iter().map(|s| s.w).fold(0.0, f64::max);
    let cell_h = sizes.iter().map(|s| s.h).fold(0.0, f64::max);
    let nodes: Vec<Rect> = sizes
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (along, across) = (i % GRID_RUN, i / GRID_RUN);
            let (col, row) = match spec.direction {
                Direction::Right => (along, across),
                Direction::Down => (across, along),
            };
            #[allow(clippy::cast_precision_loss)] // grid indices are tiny
            let (col, row) = (col as f64, row as f64);
            Rect {
                x: col * (cell_w + SPACING_LAYER_LAYER),
                y: row * (cell_h + SPACING_NODE_NODE),
                w: s.w,
                h: s.h,
            }
        })
        .collect();
    let w = nodes.iter().map(Rect::right).fold(0.0, f64::max);
    let h = nodes.iter().map(Rect::bottom).fold(0.0, f64::max);
    Placement {
        nodes,
        size: Size { w, h },
    }
}
