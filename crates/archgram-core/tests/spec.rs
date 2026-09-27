//! Reading and checking specs: the examples in docs/SPEC.md are valid, and
//! each rule in docs/SPEC.md (Validation) reports its problem where it is.

use archgram_core::spec::{CardStyle, Category, Direction, Kind, Variant};
use archgram_core::{Location, SpecError, parse_spec};

fn example(name: &str) -> String {
    let path = format!("{}/../../examples/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// A minimal valid spec with the given extra top-level JSON members spliced in.
fn spec_with(nodes: &str, rest: &str) -> String {
    format!(r#"{{ "archgram": 1, "title": "t", "description": "d", "nodes": [{nodes}] {rest} }}"#)
}

fn errors(json: &str) -> Vec<SpecError> {
    parse_spec(json).expect_err("the spec should be rejected")
}

fn pointers(errors: &[SpecError]) -> Vec<String> {
    errors.iter().map(|e| e.location.to_string()).collect()
}

const TWO: &str = r#"{ "id": "api", "kind": "service", "label": "API" }, { "id": "db", "kind": "database", "label": "DB" }"#;

#[test]
fn the_spec_examples_are_valid() {
    let linkshort = parse_spec(&example("linkshort.json")).expect("linkshort");
    assert_eq!(linkshort.nodes.len(), 6);
    assert_eq!(linkshort.direction, Direction::Right);
    assert_eq!(linkshort.card, CardStyle::Horizontal);
    assert_eq!(linkshort.palette, "mono");
    let rag = parse_spec(&example("rag.json")).expect("rag");
    assert_eq!(rag.frames.len(), 2);
    assert!(rag.nodes.iter().any(|n| n.variant == Variant::External));
}

#[test]
fn kinds_belong_to_their_categories() {
    assert_eq!(Kind::Queue.category(), Category::Core);
    assert_eq!(Kind::VectorStore.category(), Category::Ai);
    assert_eq!(Kind::Generated.category(), Category::Build);
    assert_eq!(Kind::Mobile.category(), Category::Client);
}

#[test]
fn malformed_json_is_located_by_line_and_column() {
    let e = errors("{\n  \"archgram\": 1,\n  \"title\": \n}");
    assert_eq!(e.len(), 1);
    assert!(
        matches!(e[0].location, Location::LineColumn { line: 4, .. }),
        "{:?}",
        e[0]
    );
}

#[test]
fn an_unknown_field_is_refused() {
    let e = errors(&spec_with(
        r#"{ "id": "api", "kind": "service", "label": "API", "colour": "red" }"#,
        "",
    ));
    assert!(
        e[0].message.contains("unknown field `colour`"),
        "{}",
        e[0].message
    );
}

#[test]
fn an_unknown_kind_lists_the_known_ones() {
    let e = errors(&spec_with(
        r#"{ "id": "db", "kind": "datbase", "label": "DB" }"#,
        "",
    ));
    assert!(
        e[0].message.contains("unknown variant `datbase`"),
        "{}",
        e[0].message
    );
    assert!(e[0].message.contains("`database`"), "{}", e[0].message);
}

#[test]
fn top_level_rules() {
    let json = r#"{ "archgram": 2, "title": " ", "description": "", "palette": "neon", "nodes": [] }"#;
    assert_eq!(
        pointers(&errors(json)),
        ["/archgram", "/title", "/description", "/palette", "/nodes"]
    );
}

#[test]
fn ids_are_well_formed_and_unique_across_nodes_and_frames() {
    let nodes = r#"{ "id": "API", "kind": "service", "label": "API" }, { "id": "db", "kind": "database", "label": "DB", "frame": "db" }"#;
    let e = errors(&spec_with(
        nodes,
        r#", "frames": [{ "id": "db", "label": "Data" }]"#,
    ));
    assert_eq!(pointers(&e), ["/nodes/0/id", "/frames/0/id", "/nodes/1/frame"]);
    assert!(e[1].message.contains("used more than once"));
    assert_eq!(e[2].message, "`db` is a node, not a frame");
}

#[test]
fn a_reference_to_a_missing_id_suggests_a_near_one() {
    let nodes = r#"{ "id": "api", "kind": "service", "label": "API", "frame": "vpcx" }"#;
    let e = errors(&spec_with(
        nodes,
        r#", "frames": [{ "id": "vpc", "label": "VPC" }]"#,
    ));
    assert_eq!(pointers(&e), ["/nodes/0/frame", "/frames/0"]);
    assert_eq!(e[0].message, "no frame has the id `vpcx`; did you mean `vpc`?");
}

#[test]
fn frames_cannot_nest_in_a_cycle_or_stand_empty() {
    let nodes = r#"{ "id": "api", "kind": "service", "label": "API", "frame": "a" }"#;
    let frames = r#", "frames": [{ "id": "a", "label": "A", "parent": "b" }, { "id": "b", "label": "B", "parent": "a" }, { "id": "c", "label": "C" }]"#;
    let e = errors(&spec_with(nodes, frames));
    assert_eq!(
        pointers(&e),
        ["/frames/0/parent", "/frames/1/parent", "/frames/2"]
    );
}

#[test]
fn edges_join_existing_nodes_once() {
    let edges = r#", "frames": [{ "id": "f", "label": "F" }],
        "edges": [{ "from": "api", "to": "db" }, { "from": "api", "to": "db" }, { "from": "api", "to": "api" }, { "from": "api", "to": "f" }]"#;
    let nodes = format!(r#"{TWO}, {{ "id": "x", "kind": "file", "label": "X", "frame": "f" }}"#);
    let e = errors(&spec_with(&nodes, edges));
    assert_eq!(pointers(&e), ["/edges/1", "/edges/2", "/edges/3/to"]);
    assert_eq!(e[2].message, "`f` is a frame, not a node");
}

#[test]
fn a_flow_follows_existing_edges() {
    let rest = r#", "edges": [{ "from": "api", "to": "db" }], "flows": [{ "name": "back", "steps": ["db", "api"] }, { "name": "one", "steps": ["api"] }]"#;
    let e = errors(&spec_with(TWO, rest));
    assert_eq!(pointers(&e), ["/flows/0/steps/1", "/flows/1/steps"]);
    assert_eq!(
        e[0].message,
        "flow `back` goes from `db` to `api`, but no edge goes from `db` to `api`"
    );
}

#[test]
fn hints_name_nodes_and_do_not_contradict_each_other() {
    let rest = r#", "hints": { "first": ["api"], "last": ["api", "dbb"], "sameLayer": [["db"]] }"#;
    let e = errors(&spec_with(TWO, rest));
    assert_eq!(pointers(&e), ["/hints/last/1", "/hints", "/hints/sameLayer/0"]);
}

#[test]
fn every_problem_is_reported_in_one_run() {
    let nodes = r#"{ "id": "api", "kind": "service", "label": "" }, { "id": "api", "kind": "database", "label": "DB", "tech": "Postgres" }"#;
    let e = errors(&spec_with(
        nodes,
        r#", "edges": [{ "from": "api", "to": "nowhere" }]"#,
    ));
    assert_eq!(
        pointers(&e),
        ["/nodes/1/id", "/nodes/0/label", "/nodes/1/tech", "/edges/0/to"]
    );
}

#[test]
fn an_order_hint_stays_within_one_frame() {
    let nodes = r#"{ "id": "a", "kind": "service", "label": "A", "frame": "f" }, { "id": "b", "kind": "service", "label": "B" },
        { "id": "c", "kind": "service", "label": "C", "frame": "f" }"#;
    let rest = r#", "frames": [{ "id": "f", "label": "F" }], "hints": { "order": [["a", "c"], ["a", "b"]] }"#;
    let e = errors(&spec_with(nodes, rest));
    assert_eq!(pointers(&e), ["/hints/order/1"]);
}
