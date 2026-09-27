//! The layout's invariants (ARCHITECTURE.md, Invariants) on specs from a
//! seeded generator: no two cards overlap, every edge spans layers, and the
//! same spec lays out the same way. The generator is a fixed xorshift, so a
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
    let edge_json: Vec<String> = pairs
        .iter()
        .map(|(a, b)| format!(r#"{{ "from": "n{a}", "to": "n{b}" }}"#))
        .collect();
    format!(
        r#"{{ "archgram": 1, "title": "random {seed}", "description": "generated", "nodes": [{}], "edges": [{}] }}"#,
        node_json.join(", "),
        edge_json.join(", ")
    )
}

#[test]
fn random_specs_keep_the_invariants() {
    for seed in 1..=150u64 {
        let nodes = 2 + usize::try_from(seed % 40).unwrap();
        let spec = parse_spec(&random_spec(seed, nodes)).unwrap_or_else(|e| panic!("seed {seed}: {e:?}"));
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
        assert_eq!(place(&spec, &sizes).unwrap(), p, "seed {seed}: not deterministic");
    }
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
