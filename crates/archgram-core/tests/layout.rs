//! The layout's invariants (ARCHITECTURE.md, Invariants) on specs from a
//! seeded generator: no two cards overlap, every edge spans layers, every
//! edge is orthogonal, starts and ends on its cards and passes through no
//! card, every label sits on its edge and clear of every card, cards in one
//! layer share its depth, and the same spec lays out the same way. The generator is a fixed xorshift, so a
//! failing seed reproduces.

use archgram_core::layout::place;
use archgram_core::measure::card_sizes;
use archgram_core::parse_spec;

/// A small deterministic generator (xorshift64).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn below(&mut self, n: usize) -> usize {
        usize::try_from(self.next() % n as u64).unwrap()
    }
}

const KINDS: [&str; 6] = ["service", "database", "queue", "cache", "model", "browser"];

/// A valid spec: `nodes` nodes, forward edges with some back edges (cycles),
/// possibly several components, labels of varying length.
fn random_spec(seed: u64, nodes: usize) -> String {
    let mut rng = Rng(seed | 1);
    let words = [
        "API",
        "Worker",
        "Postgres",
        "Redis cache",
        "Gateway",
        "Search index",
        "LLM",
        "Billing service",
        "Auth",
    ];
    let node_json: Vec<String> = (0..nodes)
        .map(|i| {
            let label = format!("{} {i}", words[rng.below(words.len())]);
            format!(
                r#"{{ "id": "n{i}", "kind": "{}", "label": "{label}" }}"#,
                KINDS[rng.below(KINDS.len())]
            )
        })
        .collect();
    let mut pairs = std::collections::BTreeSet::new();
    let edges = nodes + rng.below(nodes + 1);
    for _ in 0..edges {
        let (a, b) = (rng.below(nodes), rng.below(nodes));
        if a == b {
            continue;
        }
        // Mostly forward; one in six goes back and may close a cycle.
        let (from, to) = if (a < b) ^ (rng.below(6) == 0) {
            (a, b)
        } else {
            (b, a)
        };
        pairs.insert((from, to));
    }
    // One edge in three has a label, short or long.
    let labels = ["on a miss", "streams", "reads", "writes every row"];
    let edge_json: Vec<String> = pairs
        .iter()
        .map(|(a, b)| {
            if (a + b) % 3 == 0 {
                let label = labels[(a * b) % labels.len()];
                format!(r#"{{ "from": "n{a}", "to": "n{b}", "label": "{label}" }}"#)
            } else {
                format!(r#"{{ "from": "n{a}", "to": "n{b}" }}"#)
            }
        })
        .collect();
    format!(
        r#"{{ "archgram": 1, "title": "random {seed}", "description": "generated", "nodes": [{}], "edges": [{}] }}"#,
        node_json.join(", "),
        edge_json.join(", ")
    )
}

/// Whether `p` lies on the border of `r`, to within half a pixel.
fn on_border(r: &archgram_core::geometry::Rect, p: archgram_core::geometry::Point) -> bool {
    let within = |v: f64, lo: f64, hi: f64| v >= lo - 0.5 && v <= hi + 0.5;
    let near = |v: f64, edge: f64| (v - edge).abs() <= 0.5;
    (within(p.x, r.x, r.right()) && (near(p.y, r.y) || near(p.y, r.bottom())))
        || (within(p.y, r.y, r.bottom()) && (near(p.x, r.x) || near(p.x, r.right())))
}

/// Whether the axis-aligned segment `a`-`b` passes through the inside of `r`
/// (running along its border, or ending on it, does not count).
fn crosses_interior(
    r: &archgram_core::geometry::Rect,
    a: archgram_core::geometry::Point,
    b: archgram_core::geometry::Point,
) -> bool {
    let (x0, x1) = (a.x.min(b.x), a.x.max(b.x));
    let (y0, y1) = (a.y.min(b.y), a.y.max(b.y));
    let e = 0.5;
    x0 < r.right() - e && x1 > r.x + e && y0 < r.bottom() - e && y1 > r.y + e
}

/// Whether `p` lies on the axis-aligned segment `a`-`b`, to within half a pixel.
fn on_segment(
    a: archgram_core::geometry::Point,
    b: archgram_core::geometry::Point,
    p: archgram_core::geometry::Point,
) -> bool {
    let e = 0.5;
    p.x >= a.x.min(b.x) - e && p.x <= a.x.max(b.x) + e && p.y >= a.y.min(b.y) - e && p.y <= a.y.max(b.y) + e
}

#[test]
fn random_specs_keep_the_invariants() {
    check_random(150, "right");
}

#[test]
fn random_specs_flowing_down_keep_the_invariants() {
    check_random(60, "down");
}

fn check_random(seeds: u64, direction: &str) {
    for seed in 1..=seeds {
        let nodes = 2 + usize::try_from(seed % 40).unwrap();
        let json = random_spec(seed, nodes).replacen(
            r#""archgram": 1,"#,
            &format!(r#""archgram": 1, "direction": "{direction}","#),
            1,
        );
        let spec = parse_spec(&json).unwrap_or_else(|e| panic!("seed {seed}: {e:?}"));
        let sizes = card_sizes(&spec);
        let p = place(&spec, &sizes).unwrap_or_else(|e| panic!("seed {seed}: {e:?}"));
        for (i, a) in p.nodes.iter().enumerate() {
            assert!(a.x >= -1e-6 && a.y >= -1e-6, "seed {seed}: node {i} at {a:?}");
            for (j, b) in p.nodes.iter().enumerate().skip(i + 1) {
                assert!(
                    !a.overlaps(b),
                    "seed {seed}: nodes {i} and {j} overlap: {a:?} {b:?}"
                );
            }
        }
        for (i, a) in p.nodes.iter().enumerate() {
            for (j, b) in p.nodes.iter().enumerate().skip(i + 1) {
                let along = |r: &archgram_core::geometry::Rect| if direction == "right" { r.w } else { r.h };
                assert!(
                    p.units[i] != p.units[j]
                        || p.layers[i] != p.layers[j]
                        || (along(a) - along(b)).abs() < 1e-9,
                    "seed {seed}: nodes {i} and {j} share a layer but not its depth"
                );
            }
        }
        for e in &spec.edges {
            let idx = |id: &str| spec.nodes.iter().position(|n| n.id == id).unwrap();
            assert_ne!(
                p.layers[idx(&e.from)],
                p.layers[idx(&e.to)],
                "seed {seed}: edge {} -> {} within one layer",
                e.from,
                e.to
            );
        }
        for (k, (e, path)) in spec.edges.iter().zip(&p.edges).enumerate() {
            let idx = |id: &str| spec.nodes.iter().position(|n| n.id == id).unwrap();
            let (from, to) = (p.nodes[idx(&e.from)], p.nodes[idx(&e.to)]);
            assert!(path.len() >= 2, "seed {seed}: edge {k} has no path");
            assert!(
                on_border(&from, path[0]),
                "seed {seed}: edge {k} does not start on its card: {:?} {from:?}",
                path[0]
            );
            assert!(
                on_border(&to, path[path.len() - 1]),
                "seed {seed}: edge {k} does not end on its card"
            );
            for w in path.windows(2) {
                let (a, b) = (w[0], w[1]);
                assert!(
                    (a.x - b.x).abs() < 1e-6 || (a.y - b.y).abs() < 1e-6,
                    "seed {seed}: edge {k} has a slanted segment {a:?} {b:?}"
                );
                for (i, r) in p.nodes.iter().enumerate() {
                    assert!(
                        !crosses_interior(r, a, b),
                        "seed {seed}: edge {k} ({} -> {}) runs through node {i}",
                        e.from,
                        e.to
                    );
                }
            }
        }
        check_labels(seed, &spec, &p);
        check_units(seed, &spec, &p);
        assert_eq!(place(&spec, &sizes).unwrap(), p, "seed {seed}: not deterministic");
    }
}

/// Every label sits on its edge, covers no card, and no other edge runs
/// under it.
fn check_labels(seed: u64, spec: &archgram_core::spec::Spec, p: &archgram_core::layout::Placement) {
    for (k, (e, label)) in spec.edges.iter().zip(&p.labels).enumerate() {
        assert_eq!(e.label.is_some(), label.is_some(), "seed {seed}: edge {k}");
        let Some(r) = label else { continue };
        let centre = archgram_core::geometry::Point {
            x: r.x + r.w / 2.0,
            y: r.y + r.h / 2.0,
        };
        assert!(
            p.edges[k].windows(2).any(|w| on_segment(w[0], w[1], centre)),
            "seed {seed}: the label of edge {k} is off its path"
        );
        for (i, n) in p.nodes.iter().enumerate() {
            assert!(
                !r.overlaps(n),
                "seed {seed}: the label of edge {k} covers node {i}"
            );
        }
        // An edge sharing a card with the labelled one keeps clear of the
        // label too, unless that card's side is too short for all its
        // ports and every gap shrinks (`route::ports`); any other edge
        // never runs under it.
        let same_card = |j: usize| {
            let (a, b) = (&spec.edges[k], &spec.edges[j]);
            a.from == b.from || a.to == b.to || a.from == b.to || a.to == b.from
        };
        for (j, other) in p.edges.iter().enumerate() {
            if j == k || same_card(j) {
                continue;
            }
            for w in other.windows(2) {
                assert!(
                    !crosses_interior(r, w[0], w[1]),
                    "seed {seed}: edge {j} runs under the label of edge {k}"
                );
            }
        }
    }
}

/// Units share nothing, so their boxes (cards, paths and labels) never meet,
/// and every row of units below the first stays within the widest unit.
fn check_units(seed: u64, spec: &archgram_core::spec::Spec, p: &archgram_core::layout::Placement) {
    use archgram_core::geometry::Rect;
    let count = p.units.iter().copied().max().map_or(0, |m| m + 1);
    let mut boxes: Vec<Option<(f64, f64, f64, f64)>> = vec![None; count];
    let mut grow = |u: usize, x0: f64, y0: f64, x1: f64, y1: f64| {
        let b = boxes[u].get_or_insert((x0, y0, x1, y1));
        *b = (b.0.min(x0), b.1.min(y0), b.2.max(x1), b.3.max(y1));
    };
    for (v, r) in p.nodes.iter().enumerate() {
        grow(p.units[v], r.x, r.y, r.right(), r.bottom());
    }
    let index = |id: &str| spec.nodes.iter().position(|n| n.id == id).unwrap();
    for (k, e) in spec.edges.iter().enumerate() {
        let u = p.units[index(&e.from)];
        for q in &p.edges[k] {
            grow(u, q.x, q.y, q.x, q.y);
        }
        if let Some(Rect { x, y, w, h }) = p.labels[k] {
            grow(u, x, y, x + w, y + h);
        }
    }
    let boxes: Vec<(f64, f64, f64, f64)> = boxes.into_iter().map(|b| b.unwrap()).collect();
    for (a, ba) in boxes.iter().enumerate() {
        for (b, bb) in boxes.iter().enumerate().skip(a + 1) {
            let apart = ba.2 <= bb.0 || bb.2 <= ba.0 || ba.3 <= bb.1 || bb.3 <= ba.1;
            assert!(apart, "seed {seed}: units {a} and {b} meet: {ba:?} {bb:?}");
        }
    }
    let widest = boxes.iter().map(|b| b.2 - b.0).fold(0.0, f64::max);
    for (u, b) in boxes.iter().enumerate() {
        assert!(
            b.2 <= widest + 0.5,
            "seed {seed}: unit {u} runs past the widest unit"
        );
    }
}

#[test]
fn parts_without_edges_go_below_the_flow() {
    let spec = parse_spec(
        r#"{ "archgram": 1, "title": "t", "description": "d",
        "nodes": [{ "id": "a", "kind": "browser", "label": "A" }, { "id": "b", "kind": "service", "label": "B" },
                  { "id": "c", "kind": "database", "label": "C" }, { "id": "x", "kind": "check", "label": "X" },
                  { "id": "y", "kind": "check", "label": "Y" }],
        "edges": [{ "from": "a", "to": "b" }, { "from": "b", "to": "c" }] }"#,
    )
    .unwrap();
    let p = place(&spec, &card_sizes(&spec)).unwrap();
    let flow_bottom = p.nodes[..3]
        .iter()
        .map(archgram_core::geometry::Rect::bottom)
        .fold(0.0, f64::max);
    for i in [3, 4] {
        assert!(
            p.nodes[i].y >= flow_bottom,
            "{:?} is not below the flow",
            p.nodes[i]
        );
    }
    // Side by side in one row, not stacked in the first column.
    assert!((p.nodes[3].y - p.nodes[4].y).abs() < 1e-9);
    assert!(p.nodes[4].x > p.nodes[3].right());
}

#[test]
fn nodes_without_any_edge_form_a_square_grid() {
    let nodes: Vec<String> = (0..9)
        .map(|i| format!(r#"{{ "id": "n{i}", "kind": "service", "label": "N{i}" }}"#))
        .collect();
    let spec = parse_spec(&format!(
        r#"{{ "archgram": 1, "title": "t", "description": "d", "nodes": [{}] }}"#,
        nodes.join(", ")
    ))
    .unwrap();
    let p = place(&spec, &card_sizes(&spec)).unwrap();
    // Three columns of three.
    assert!((p.nodes[2].y - p.nodes[0].y).abs() < 1e-9);
    assert!(p.nodes[3].y > p.nodes[0].bottom());
    assert!((p.nodes[3].x - p.nodes[0].x).abs() < 1e-9);
}

#[test]
fn a_chain_lies_on_one_line() {
    let spec = parse_spec(
        r#"{ "archgram": 1, "title": "t", "description": "d",
        "nodes": [{ "id": "a", "kind": "browser", "label": "A" }, { "id": "b", "kind": "service", "label": "B" }, { "id": "c", "kind": "database", "label": "C" }],
        "edges": [{ "from": "a", "to": "b" }, { "from": "b", "to": "c" }] }"#,
    )
    .unwrap();
    let p = place(&spec, &card_sizes(&spec)).unwrap();
    assert_eq!(p.layers, [0, 1, 2]);
    assert!((p.nodes[0].centre_y() - p.nodes[2].centre_y()).abs() < 1e-9);
}

#[test]
fn hints_are_kept_or_refused() {
    let base = r#""nodes": [{ "id": "a", "kind": "browser", "label": "A" }, { "id": "b", "kind": "service", "label": "B" },
        { "id": "c", "kind": "database", "label": "C" }, { "id": "d", "kind": "cache", "label": "D" }],
        "edges": [{ "from": "a", "to": "b" }, { "from": "b", "to": "c" }, { "from": "a", "to": "d" }]"#;
    let spec = |hints: &str| {
        parse_spec(&format!(
            r#"{{ "archgram": 1, "title": "t", "description": "d", {base}, "hints": {hints} }}"#
        ))
        .unwrap()
    };

    // `d` would sit in layer 1; `last` moves it to the end.
    let s = spec(r#"{ "last": ["d"] }"#);
    let p = place(&s, &card_sizes(&s)).unwrap();
    assert_eq!(p.layers[3], 2);

    let s = spec(r#"{ "sameLayer": [["c", "d"]] }"#);
    let p = place(&s, &card_sizes(&s)).unwrap();
    assert_eq!(p.layers[2], p.layers[3]);

    let s = spec(r#"{ "first": ["b"], "sameLayer": [["a", "b"]] }"#);
    let e = place(&s, &card_sizes(&s)).unwrap_err();
    let where_: Vec<String> = e.iter().map(|x| x.location.to_string()).collect();
    assert_eq!(where_, ["/hints/sameLayer/0", "/hints/first/0"]);
}

/// The budget in ARCHITECTURE.md (Performance budget): layout and SVG for 100
/// nodes in under 50 ms, natively. Timing depends on the build, so this runs
/// on request: `cargo test --release -p archgram-core --test layout -- --ignored`.
#[test]
#[ignore = "a timing check; run in release"]
fn a_hundred_nodes_lay_out_within_the_budget() {
    let json = random_spec(42, 100);
    let start = std::time::Instant::now();
    let runs = 5;
    for _ in 0..runs {
        archgram_core::build(&json, archgram_core::render::Options::default()).unwrap();
    }
    let each = start.elapsed() / runs;
    println!("100 nodes: {each:?} per build");
    assert!(each.as_millis() < 50, "{each:?}");
}
