//! Reading YAML: the same spec as the JSON, every problem at its YAML line
//! and column, and the refusals docs/SPEC.md and the crate list.

use std::fmt::Write as _;

use archgram_core::SpecError;
use archgram_yaml::{MAX_DEPTH, parse_spec};

fn root() -> String {
    format!("{}/../..", env!("CARGO_MANIFEST_DIR"))
}

fn errors(yaml: &str) -> Vec<SpecError> {
    parse_spec(yaml).expect_err("the spec should be refused")
}

fn places(e: &[SpecError]) -> Vec<String> {
    e.iter().map(|e| e.location.to_string()).collect()
}

#[test]
fn every_example_reads_the_same_as_json() {
    // YAML 1.2 reads JSON as it is, so each example's own text will do.
    for entry in std::fs::read_dir(format!("{}/examples", root())).unwrap() {
        let path = entry.unwrap().path();
        let text = std::fs::read_to_string(&path).unwrap();
        let from_json = archgram_core::parse_spec(&text).unwrap();
        let from_yaml = parse_spec(&text).unwrap_or_else(|e| panic!("{}: {e:?}", path.display()));
        assert_eq!(from_yaml, from_json, "{}", path.display());
    }
}

#[test]
fn the_yaml_example_in_the_spec_is_linkshort() {
    let doc = std::fs::read_to_string(format!("{}/docs/SPEC.md", root())).unwrap();
    let block = doc
        .split("### linkshort (YAML")
        .nth(1)
        .and_then(|rest| rest.split("```yaml\n").nth(1))
        .and_then(|rest| rest.split("\n```").next())
        .expect("a YAML block in docs/SPEC.md");
    let json = std::fs::read_to_string(format!("{}/examples/linkshort.json", root())).unwrap();
    assert_eq!(
        parse_spec(block).unwrap(),
        archgram_core::parse_spec(&json).unwrap()
    );
}

#[test]
fn a_problem_points_at_its_line_and_column() {
    // An unknown field, at its key; the column counts from 1.
    let e = errors(
        "archgram: 1\ntitle: t\ndescription: d\n  \nnodes:\n  - id: a\n    kind: service\n    lable: A\n",
    );
    assert_eq!(places(&e), ["8:5"], "{e:?}");
    // A rule the core checks by pointer, at the value.
    let e = errors(
        "archgram: 1\ntitle: t\ndescription: d\nnodes:\n  - { id: a, kind: service, label: A, frame: nope }\n",
    );
    assert_eq!(places(&e), ["5:46"], "{e:?}");
    // A plain number where text belongs, with the way out.
    let e = errors("archgram: 1\ntitle: 2026\ndescription: d\nnodes: []\n");
    assert_eq!(places(&e), ["2:1"]);
    assert!(e[0].message.contains("quote it"), "{}", e[0].message);
}

#[test]
fn a_key_comes_once() {
    let e = errors("archgram: 1\ntitle: t\ntitle: u\n");
    assert_eq!(places(&e), ["3:1"]);
    assert!(e[0].message.contains("first at 2:1"), "{}", e[0].message);
}

#[test]
fn one_document_only() {
    let e = errors("archgram: 1\n---\narchgram: 1\n");
    assert_eq!(places(&e)[0].split(':').next(), Some("2"));
}

#[test]
fn an_alias_bomb_stops_early() {
    let mut yaml = String::from("a: &a [x, x, x, x, x, x, x, x, x, x]\n");
    for (i, prev) in ["b", "c", "d", "e", "f", "g"]
        .iter()
        .zip(["a", "b", "c", "d", "e", "f"])
    {
        let refs = vec![format!("*{prev}"); 10].join(", ");
        writeln!(yaml, "{i}: &{i} [{refs}]").unwrap();
    }
    let e = errors(&yaml);
    assert!(e[0].message.contains("aliases expand"), "{}", e[0].message);
}

#[test]
fn nesting_has_a_floor() {
    let yaml = format!("{}{}", "[".repeat(MAX_DEPTH + 1), "]".repeat(MAX_DEPTH + 1));
    let e = errors(&yaml);
    assert!(e[0].message.contains("nesting deeper"), "{}", e[0].message);
}

#[test]
fn a_foreign_tag_is_refused() {
    let e = errors("archgram: 1\ntitle: !secret t\n");
    assert!(e[0].message.contains("core schema"), "{}", e[0].message);
}
