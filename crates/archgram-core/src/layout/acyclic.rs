//! Cycle removal (ARCHITECTURE.md, Layout, step 1).
//!
//! A layered layout needs every edge to point the same way. Edges that close
//! a cycle are reversed for the layout's duration, chosen with the greedy
//! heuristic of Eades, Lin and Smyth (1993): peel sinks off the end and
//! sources off the front; when neither is left, take the vertex whose
//! out-degree most exceeds its in-degree. Edges that point backwards in the
//! resulting sequence are the ones to reverse. Ties go to the lowest index,
//! which is spec order, so the same spec reverses the same edges.

/// For each edge `(from, to)`, whether the layout reverses it.
pub fn reversed_edges(n: usize, edges: &[(usize, usize)]) -> Vec<bool> {
    let mut indeg = vec![0usize; n];
    let mut outdeg = vec![0usize; n];
    for &(a, b) in edges {
        outdeg[a] += 1;
        indeg[b] += 1;
    }
    let mut removed = vec![false; n];
    let (mut front, mut back) = (Vec::new(), Vec::new());
    let mut left = n;
    let remove = |v: usize, removed: &mut Vec<bool>, indeg: &mut Vec<usize>, outdeg: &mut Vec<usize>| {
        removed[v] = true;
        for &(a, b) in edges {
            if a == v && !removed[b] {
                indeg[b] -= 1;
            }
            if b == v && !removed[a] {
                outdeg[a] -= 1;
            }
        }
    };
    while left > 0 {
        let mut progressed = true;
        while progressed {
            progressed = false;
            // Sinks go to the back, in order found; sources to the front.
            if let Some(v) = (0..n).find(|&v| !removed[v] && outdeg[v] == 0) {
                back.push(v);
                remove(v, &mut removed, &mut indeg, &mut outdeg);
                left -= 1;
                progressed = true;
                continue;
            }
            if let Some(v) = (0..n).find(|&v| !removed[v] && indeg[v] == 0) {
                front.push(v);
                remove(v, &mut removed, &mut indeg, &mut outdeg);
                left -= 1;
                progressed = true;
            }
        }
        if left == 0 {
            break;
        }
        // Only cycles remain: take the vertex with the largest out-minus-in degree.
        let v = (0..n)
            .filter(|&v| !removed[v])
            .max_by(|&a, &b| {
                let da = outdeg[a].cast_signed() - indeg[a].cast_signed();
                let db = outdeg[b].cast_signed() - indeg[b].cast_signed();
                da.cmp(&db).then(b.cmp(&a))
            })
            .expect("a vertex remains");
        front.push(v);
        remove(v, &mut removed, &mut indeg, &mut outdeg);
        left -= 1;
    }
    back.reverse();
    front.extend(back);
    let mut position = vec![0; n];
    for (i, &v) in front.iter().enumerate() {
        position[v] = i;
    }
    edges.iter().map(|&(a, b)| position[a] > position[b]).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_dag_keeps_every_edge() {
        let edges = [(0, 1), (1, 2), (0, 2), (2, 3)];
        assert_eq!(reversed_edges(4, &edges), [false; 4]);
    }

    #[test]
    fn a_cycle_loses_one_edge_and_becomes_acyclic() {
        let edges = [(0, 1), (1, 2), (2, 0)];
        let rev = reversed_edges(3, &edges);
        assert_eq!(rev.iter().filter(|r| **r).count(), 1);
    }

    #[test]
    fn a_request_and_its_reply_keep_the_request() {
        // chat -> api -> check -> chat: the reply edge back to the start is reversed.
        let edges = [(0, 1), (1, 2), (2, 0)];
        assert_eq!(reversed_edges(3, &edges), [false, false, true]);
    }
}
