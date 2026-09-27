//! Orthogonal edge routing for the layered layout (ARCHITECTURE.md, Route).
//!
//! Every edge, split into hops between adjacent layers, runs through the
//! gaps between layers, where no card stands:
//!
//! - a hop whose two ends are level is one straight segment;
//! - otherwise it leaves its start, turns in the gap onto a vertical
//!   segment (a *track*), and turns again into its end: a symmetric Z.
//!
//! Vertical segments in one gap that overlap get separate tracks. Their
//! order is chosen pair by pair to cross fewer of the other hops' horizontal
//! segments, then each segment takes the lowest track free of the segments
//! it overlaps. A gap with many tracks is widened by the layout.
//!
//! Coordinates here are in the layout's frame: "main" runs along the flow,
//! "cross" across it.

use std::collections::BTreeSet;

/// One hop's vertical part, in one gap: it arrives at `from` (cross) and
/// leaves at `to` (cross). A bundle of hops that share a port shares one
/// riser, spanning from its lowest end to its highest.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Riser {
    pub from: f64,
    pub to: f64,
    low: f64,
    high: f64,
}

impl Riser {
    /// One hop's riser.
    #[cfg(test)]
    pub fn new(from: f64, to: f64) -> Self {
        Self {
            from,
            to,
            low: from.min(to),
            high: from.max(to),
        }
    }

    /// A bundle's riser: from its shared port to each of `ends`. `to` is the
    /// end farthest from the port, which decides the crossings it makes.
    pub fn bundle(from: f64, ends: &[f64]) -> Self {
        let to = ends
            .iter()
            .copied()
            .max_by(|a, b| (a - from).abs().total_cmp(&(b - from).abs()))
            .unwrap_or(from);
        let low = ends.iter().copied().fold(from, f64::min);
        let high = ends.iter().copied().fold(from, f64::max);
        Self { from, to, low, high }
    }

    fn low(&self) -> f64 {
        self.low
    }

    fn high(&self) -> f64 {
        self.high
    }

    /// Whether two risers share any stretch of the gap, and so need tracks
    /// of their own. A shared end counts: two segments may not touch.
    fn overlaps(&self, other: &Riser, clearance: f64) -> bool {
        self.low() <= other.high() + clearance && other.low() <= self.high() + clearance
    }
}

/// Whether `y` lies strictly inside the riser's span.
fn inside(y: f64, r: &Riser) -> bool {
    y > r.low() && y < r.high()
}

/// How many crossings putting `a` on an earlier track than `b` makes: `a`'s
/// outgoing horizontal crosses `b`'s riser, and `b`'s incoming horizontal
/// crosses `a`'s riser.
fn crossings_if_before(a: &Riser, b: &Riser) -> u8 {
    u8::from(inside(a.to, b)) + u8::from(inside(b.from, a))
}

/// A track for each riser in one gap (0 is nearest the gap's start), and how
/// many tracks the gap needs. `clearance` is the least distance between two
/// risers' spans that still lets them share a track.
pub fn tracks(risers: &[Riser], clearance: f64) -> (Vec<usize>, usize) {
    let n = risers.len();
    // Preferences between overlapping risers: an edge a -> b means "a before b".
    let mut before: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for a in 0..n {
        for b in a + 1..n {
            if !risers[a].overlaps(&risers[b], clearance) {
                continue;
            }
            let (ab, ba) = (
                crossings_if_before(&risers[a], &risers[b]),
                crossings_if_before(&risers[b], &risers[a]),
            );
            if ab <= ba {
                before[a].insert(b);
            } else {
                before[b].insert(a);
            }
        }
    }
    // An order that follows the preferences; a cycle is broken at its lowest riser.
    let mut indeg = vec![0usize; n];
    for set in &before {
        for &b in set {
            indeg[b] += 1;
        }
    }
    let mut done = vec![false; n];
    let mut order = Vec::with_capacity(n);
    while order.len() < n {
        let next = (0..n)
            .find(|&i| !done[i] && indeg[i] == 0)
            .or_else(|| (0..n).find(|&i| !done[i]))
            .expect("a riser remains");
        done[next] = true;
        order.push(next);
        for &b in &before[next] {
            indeg[b] = indeg[b].saturating_sub(1);
        }
    }
    // Lowest track above every earlier riser it overlaps.
    let mut track = vec![0usize; n];
    for (k, &i) in order.iter().enumerate() {
        track[i] = order[..k]
            .iter()
            .filter(|&&j| risers[i].overlaps(&risers[j], clearance))
            .map(|&j| track[j] + 1)
            .max()
            .unwrap_or(0);
    }
    let count = track.iter().map(|t| t + 1).max().unwrap_or(0);
    (track, count)
}

/// Where ports go along a side centred at `centre`, one per entry of
/// `reach`, in order: one in the middle, several spread around it, never
/// nearer the side's ends than `margin`. Neighbours are `step` apart, or
/// further when a label sits on one of their lines near the card: `reach`
/// is how far that label reaches across the line on either side (0 for a
/// line without one), and a label keeps half a `step` clear of the next
/// line. When the side is too short, every gap shrinks in proportion.
pub fn ports(reach: &[f64], centre: f64, side: f64, step: f64, margin: f64) -> Vec<f64> {
    if reach.len() <= 1 {
        return vec![centre; reach.len()];
    }
    let gaps: Vec<f64> = reach
        .windows(2)
        .map(|w| {
            let labelled = w[0] + w[1];
            if labelled > 0.0 {
                step.max(labelled + step / 2.0)
            } else {
                step
            }
        })
        .collect();
    let total: f64 = gaps.iter().sum();
    let room = (side - 2.0 * margin).max(0.0);
    let scale = if total > room { room / total } else { 1.0 };
    let mut at = centre - total * scale / 2.0;
    let mut out = vec![at];
    for g in gaps {
        at += g * scale;
        out.push(at);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn risers_that_do_not_overlap_share_a_track() {
        let r = [Riser::new(0.0, 10.0), Riser::new(40.0, 60.0)];
        assert_eq!(tracks(&r, 12.0), (vec![0, 0], 1));
    }

    #[test]
    fn overlapping_risers_get_their_own_tracks() {
        let r = [Riser::new(0.0, 50.0), Riser::new(20.0, 70.0)];
        let (t, count) = tracks(&r, 12.0);
        assert_eq!(count, 2);
        assert_ne!(t[0], t[1]);
    }

    #[test]
    fn the_order_avoids_a_crossing_it_can_avoid() {
        // Both go down: a from 0 to 50, b from 20 to 70. With a on the
        // earlier track, a's outgoing horizontal at 50 crosses b's riser
        // (20..70), and b's incoming horizontal at 20 crosses a's riser
        // (0..50): two crossings. With b first, b's outgoing at 70 passes
        // below a's riser and a's incoming at 0 above b's: none. So b takes
        // the earlier track.
        let r = [Riser::new(0.0, 50.0), Riser::new(20.0, 70.0)];
        assert_eq!(crossings_if_before(&r[0], &r[1]), 2);
        assert_eq!(crossings_if_before(&r[1], &r[0]), 0);
        let (t, _) = tracks(&r, 12.0);
        assert!(t[1] < t[0]);
    }

    #[test]
    fn ports_sit_in_the_middle_or_spread_around_it() {
        assert_eq!(ports(&[0.0], 28.0, 56.0, 12.0, 10.0), vec![28.0]);
        assert_eq!(ports(&[0.0; 3], 28.0, 56.0, 12.0, 10.0), vec![16.0, 28.0, 40.0]);
        // Too many for the step: squeezed inside the margins.
        let p = ports(&[0.0; 6], 28.0, 56.0, 12.0, 10.0);
        assert!(p[0] >= 10.0 - 1e-9 && p[5] <= 46.0 + 1e-9, "{p:?}");
    }

    #[test]
    fn a_label_keeps_its_neighbours_clear() {
        // The first line carries a label reaching 30 either side: the next
        // line is 30 + 6 away, not 12.
        let p = ports(&[30.0, 0.0], 100.0, 200.0, 12.0, 10.0);
        assert_eq!(p, vec![82.0, 118.0]);
    }
}
