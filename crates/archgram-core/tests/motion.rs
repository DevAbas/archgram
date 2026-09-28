//! Flows and their animation: random specs with random flows, branches
//! included, drawn in every signal style, keep SMIL's rules (ARCHITECTURE.md,
//! Invariants) and stay well-formed. The generator is a fixed xorshift, so a
//! failing seed fails the same way every time.

use archgram_core::build;
use archgram_core::render::{Mode, Options};

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

const STYLES: [&str; 7] = ["wire", "spark", "arc", "comet", "dot", "pulse", "current"];
const STILL: [&str; 3] = ["none", "legend", "numbers"];
const KINDS: [&str; 5] = ["service", "database", "model", "script", "browser"];

/// A random DAG and flows along it, some steps branching, as a JSON spec.
fn random_spec(seed: u64) -> String {
    let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let n = 3 + rng.below(10);
    let nodes: Vec<String> = (0..n)
        .map(|i| {
            format!(
                r#"{{ "id": "n{i}", "kind": "{}", "label": "Node {i}" }}"#,
                KINDS[rng.below(KINDS.len())]
            )
        })
        .collect();
    let mut out: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (a, list) in out.iter_mut().enumerate() {
        for b in a + 1..n {
            if rng.below(3) == 0 || b == a + 1 && rng.below(2) == 0 {
                list.push(b);
            }
        }
    }
    let edges: Vec<String> = out
        .iter()
        .enumerate()
        .flat_map(|(a, bs)| {
            bs.iter()
                .map(move |b| format!(r#"{{ "from": "n{a}", "to": "n{b}" }}"#))
        })
        .collect();
    let mut flows = Vec::new();
    for f in 0..=rng.below(3) {
        let starts: Vec<usize> = (0..n).filter(|&a| !out[a].is_empty()).collect();
        if starts.is_empty() {
            break;
        }
        let mut step = vec![starts[rng.below(starts.len())]];
        let mut steps = vec![step.clone()];
        for _ in 0..=rng.below(4) {
            // Each node of the step sends to one of its successors, and
            // sometimes to a second: every branch leads on, every node of
            // the next step is reached.
            let mut next: Vec<usize> = Vec::new();
            for &a in &step {
                if out[a].is_empty() {
                    continue;
                }
                for _ in 0..=usize::from(rng.below(3) == 0) {
                    let b = out[a][rng.below(out[a].len())];
                    if !next.contains(&b) {
                        next.push(b);
                    }
                }
            }
            if next.is_empty() || step.iter().any(|&a| out[a].is_empty()) {
                break;
            }
            steps.push(next.clone());
            step = next;
        }
        if steps.len() < 2 {
            continue;
        }
        let words: Vec<String> = steps
            .iter()
            .map(|s| match s.as_slice() {
                [one] => format!(r#""n{one}""#),
                many => format!(
                    "[{}]",
                    many.iter()
                        .map(|x| format!(r#""n{x}""#))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })
            .collect();
        flows.push(format!(
            r#"{{ "name": "flow {f}", "steps": [{}] }}"#,
            words.join(", ")
        ));
    }
    format!(
        r#"{{ "archgram": 1, "title": "random {seed}", "description": "d", "signal": "{}", "still": "{}", "nodes": [{}], "edges": [{}], "flows": [{}] }}"#,
        STYLES[rng.below(STYLES.len())],
        STILL[rng.below(STILL.len())],
        nodes.join(", "),
        edges.join(", "),
        flows.join(", ")
    )
}

/// Each tag's attributes, as written: name and value.
fn tags(svg: &str) -> Vec<(String, Vec<(String, String)>)> {
    let mut out = Vec::new();
    for raw in svg.split('<').skip(1) {
        let Some(end) = raw.find('>') else { continue };
        let tag = &raw[..end];
        if tag.starts_with('/') || tag.starts_with('!') || tag.starts_with('?') {
            continue;
        }
        let name = tag
            .split_whitespace()
            .next()
            .unwrap_or("")
            .trim_end_matches('/')
            .to_owned();
        let mut attrs = Vec::new();
        let mut rest = &tag[name.len()..];
        while let Some(eq) = rest.find("=\"") {
            let key = rest[..eq].trim().to_owned();
            let after = &rest[eq + 2..];
            let close = after.find('"').expect("a closed attribute");
            attrs.push((key, after[..close].to_owned()));
            rest = &after[close + 1..];
        }
        out.push((name, attrs));
    }
    out
}

fn attr<'a>(attrs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

/// SMIL's rules on every animation, attributes written once per tag, and
/// every path a signal follows present.
fn check(seed: u64, svg: &str) {
    let tags = tags(svg);
    let ids: Vec<&str> = tags.iter().filter_map(|(_, a)| attr(a, "id")).collect();
    for (name, attrs) in &tags {
        let mut keys: Vec<&str> = attrs.iter().map(|(k, _)| k.as_str()).collect();
        keys.sort_unstable();
        let before = keys.len();
        keys.dedup();
        assert_eq!(
            keys.len(),
            before,
            "seed {seed}: <{name}> repeats an attribute"
        );
        if name == "mpath" {
            let href = attr(attrs, "href").expect("mpath href");
            assert!(
                ids.contains(&href.trim_start_matches('#')),
                "seed {seed}: {href} missing"
            );
        }
        let Some(times) = attr(attrs, "keyTimes") else {
            continue;
        };
        let written: Vec<&str> = times.split(';').collect();
        assert_eq!(written[0], "0", "seed {seed}: keyTimes start at 0");
        assert_eq!(
            *written.last().unwrap(),
            "1",
            "seed {seed}: keyTimes end at 1"
        );
        let times: Vec<f64> = written.iter().map(|t| t.parse().unwrap()).collect();
        assert!(
            times.windows(2).all(|w| w[0] <= w[1]),
            "seed {seed}: keyTimes go back"
        );
        let values = attr(attrs, "values")
            .or_else(|| attr(attrs, "keyPoints"))
            .expect("values");
        assert_eq!(
            values.split(';').count(),
            times.len(),
            "seed {seed}: one value per time"
        );
        if let Some(splines) = attr(attrs, "keySplines") {
            assert_eq!(
                splines.split(';').count(),
                times.len() - 1,
                "seed {seed}: one spline per interval"
            );
        }
    }
}

#[test]
fn random_flows_keep_smils_rules() {
    let mut animated = 0;
    for seed in 1..=300 {
        let spec = random_spec(seed);
        for mode in [Mode::Auto, Mode::Dark] {
            let svg = build(
                &spec,
                Options {
                    mode,
                    ..Options::default()
                },
            )
            .unwrap_or_else(|e| panic!("seed {seed}: {e:?}\n{spec}"));
            check(seed, &svg);
            if svg.contains("<animateMotion") || svg.contains("stroke-dashoffset") {
                animated += 1;
            }
            assert_eq!(
                build(
                    &spec,
                    Options {
                        mode,
                        ..Options::default()
                    }
                )
                .unwrap(),
                svg,
                "seed {seed}"
            );
        }
    }
    assert!(animated > 300, "only {animated} drawings moved");
}

#[test]
fn a_diagram_without_flows_has_no_motion() {
    let spec = r#"{ "archgram": 1, "title": "t", "description": "d", "signal": "spark", "still": "numbers",
        "nodes": [{ "id": "a", "kind": "service", "label": "A" }, { "id": "b", "kind": "database", "label": "B" }],
        "edges": [{ "from": "a", "to": "b" }] }"#;
    let svg = build(spec, Options::default()).unwrap();
    for absent in [
        "<animate",
        "signal",
        "lit",
        "steps",
        "glow",
        "prefers-reduced-motion",
    ] {
        assert!(!svg.contains(absent), "{absent}");
    }
}

#[test]
fn the_still_image_lists_or_numbers_the_flows() {
    let with = |still: &str| {
        format!(
            r#"{{ "archgram": 1, "title": "t", "description": "d", "still": "{still}",
            "nodes": [{{ "id": "a", "kind": "service", "label": "A" }}, {{ "id": "b", "kind": "database", "label": "B" }}, {{ "id": "c", "kind": "cache", "label": "C" }}],
            "edges": [{{ "from": "a", "to": "b" }}, {{ "from": "a", "to": "c" }}],
            "flows": [{{ "name": "fan", "steps": ["a", ["b", "c"]] }}] }}"#
        )
    };
    let listed = build(&with("legend"), Options::default()).unwrap();
    assert!(listed.contains("fan: A \u{2192} B, C</text>"));
    assert!(
        archgram_core::uncovered_characters(
            &archgram_core::parse_spec(&with("legend")).unwrap(),
            &archgram_core::logos::NoLogos
        )
        .is_empty()
    );
    let numbered = build(&with("numbers"), Options::default()).unwrap();
    assert_eq!(numbered.matches(r#"<rect class="step""#).count(), 2);
    assert_eq!(numbered.matches(">1</text>").count(), 2);
    let plain = build(&with("none"), Options::default()).unwrap();
    assert!(!plain.contains("class=\"step") && !plain.contains("B, C</text>"));
    // The screen reader hears the flow whatever the still image shows.
    assert!(plain.contains("fan: A \u{2192} B, C.</desc>"));
}
