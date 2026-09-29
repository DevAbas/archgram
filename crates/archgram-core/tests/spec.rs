//! Reading and checking specs: the examples in docs/SPEC.md are valid, and
//! each rule in docs/SPEC.md (Validation) reports its problem where it is.

use archgram_core::spec::{
    BorderStyle, CardStyle, Category, Direction, Kind, SignalStyle, Still, Variant, Wait,
};
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
    let json =
        r#"{ "archgram": 2, "title": " ", "description": "", "palette": "neon", "nodes": [] }"#;
    assert_eq!(
        pointers(&errors(json)),
        ["/archgram", "/title", "/description", "/palette", "/nodes"]
    );
}

/// A text holds only what XML allows, so the SVG opens; a line break or a
/// tab is allowed.
#[test]
fn texts_hold_only_characters_xml_allows() {
    let nodes = r#"{ "id": "api", "kind": "service", "label": "A\u0007PI", "note": "one\ntwo\tthree" },
        { "id": "db", "kind": "database", "label": "DB" }"#;
    let rest = r#", "edges": [{ "from": "api", "to": "db", "label": "reads\u001b[31m" }],
        "flows": [{ "name": "￾", "steps": ["api", "db"] }]"#;
    let json = spec_with(nodes, rest).replace(r#""title": "t""#, r#""title": "t\u0000""#);
    let e = errors(&json);
    assert_eq!(
        pointers(&e),
        [
            "/title",
            "/nodes/0/label",
            "/edges/0/label",
            "/flows/0/name"
        ]
    );
    assert_eq!(
        e[1].message,
        "the text holds U+0007, which XML does not allow; an SVG with it does not open"
    );
}

#[test]
fn ids_are_well_formed_and_unique_across_nodes_and_frames() {
    let nodes = r#"{ "id": "API", "kind": "service", "label": "API" }, { "id": "db", "kind": "database", "label": "DB", "frame": "db" }"#;
    let e = errors(&spec_with(
        nodes,
        r#", "frames": [{ "id": "db", "label": "Data" }]"#,
    ));
    assert_eq!(
        pointers(&e),
        ["/nodes/0/id", "/frames/0/id", "/nodes/1/frame"]
    );
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
    assert_eq!(
        e[0].message,
        "no frame has the id `vpcx`; did you mean `vpc`?"
    );
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
fn a_branch_is_reached_and_leads_on() {
    let nodes = format!(
        r#"{TWO}, {{ "id": "x", "kind": "service", "label": "X" }}, {{ "id": "y", "kind": "service", "label": "Y" }}"#
    );
    let edges = r#""edges": [{ "from": "api", "to": "x" }, { "from": "api", "to": "y" }, { "from": "x", "to": "db" }]"#;
    let ok = format!(r#", {edges}, "flows": [{{ "name": "fan", "steps": ["api", ["x", "y"]] }}]"#);
    let spec = parse_spec(&spec_with(&nodes, &ok)).expect("a branch along edges");
    assert_eq!(spec.flows[0].steps[1].nodes(), ["x", "y"]);
    // y leads nowhere next; db is reached from x; a step lists no nodes; x twice.
    let bad = format!(
        r#", {edges}, "flows": [{{ "name": "f", "steps": ["api", ["x", "y"], "db"] }}, {{ "name": "g", "steps": ["api", [], ["x", "x"]] }}]"#
    );
    let e = errors(&spec_with(&nodes, &bad));
    assert_eq!(
        pointers(&e),
        [
            "/flows/0/steps/1/1",
            "/flows/1/steps/1/0",
            "/flows/1/steps/2/1"
        ]
    );
    assert_eq!(
        e[0].message,
        "flow `f` leaves `y` for `db`, but no edge goes from `y` to any of them"
    );
    assert_eq!(e[1].message, "flow `g` has a step with no nodes");
    assert_eq!(e[2].message, "flow `g` lists `x` twice in one step");
    // A merge reached from none of its step's nodes names them all.
    let merge = format!(r#", {edges}, "flows": [{{ "name": "m", "steps": [["x", "y"], "api"] }}]"#);
    let e = errors(&spec_with(&nodes, &merge));
    assert_eq!(pointers(&e), ["/flows/0/steps/1"]);
    assert_eq!(
        e[0].message,
        "flow `m` reaches `api` from none of `x`, `y`: no edge goes from any of them to `api`"
    );
}

#[test]
fn signal_and_still_take_their_named_values() {
    let spec = parse_spec(&spec_with(TWO, "")).unwrap();
    assert_eq!(spec.signal, SignalStyle::Wire);
    assert_eq!(spec.still, Still::None);
    let spec = parse_spec(&spec_with(
        TWO,
        r#", "signal": "comet", "still": "numbers""#,
    ))
    .unwrap();
    assert_eq!(
        (spec.signal, spec.still),
        (SignalStyle::Comet, Still::Numbers)
    );
    let e = errors(&spec_with(TWO, r#", "signal": "neon""#));
    assert!(
        e[0].message.starts_with("unknown variant `neon`"),
        "{}",
        e[0].message
    );
}

#[test]
fn a_flow_stops_at_its_last_step_a_single_node() {
    let nodes = format!(r#"{TWO}, {{ "id": "x", "kind": "service", "label": "X" }}"#);
    let edges = r#""edges": [{ "from": "api", "to": "db" }, { "from": "api", "to": "x" }]"#;
    let ok = format!(
        r#", {edges}, "flows": [{{ "name": "refused", "steps": ["api", "db"], "stop": "db" }}]"#
    );
    let spec = parse_spec(&spec_with(&nodes, &ok)).expect("a flow stopping at its last step");
    assert_eq!(spec.flows[0].stop.as_deref(), Some("db"));
    assert_eq!(
        spec.flow_words(&spec.flows[0]),
        "API \u{2192} DB, refused at DB"
    );
    // Not the last step; the last step a branch; no such node.
    let bad = format!(
        r#", {edges}, "flows": [{{ "name": "a", "steps": ["api", "db"], "stop": "api" }}, {{ "name": "b", "steps": ["api", ["db", "x"]], "stop": "x" }}, {{ "name": "c", "steps": ["api", "db"], "stop": "dbb" }}]"#
    );
    let e = errors(&spec_with(&nodes, &bad));
    assert_eq!(
        pointers(&e),
        ["/flows/0/stop", "/flows/1/stop", "/flows/2/stop"]
    );
    assert_eq!(
        e[0].message,
        "flow `a` stops at `api`, which is not its last step: a flow stops at its last step"
    );
    assert_eq!(
        e[1].message,
        "flow `b` stops at `x`, but its last step is a branch: a flow stops at a single node"
    );
    assert_eq!(e[2].message, "no node has the id `dbb`; did you mean `db`?");
}

#[test]
fn border_wait_and_glow_take_their_named_values() {
    let spec = parse_spec(&spec_with(TWO, "")).unwrap();
    assert_eq!(
        (spec.border, spec.wait, spec.glow),
        (BorderStyle::Spark, Wait::Solid, false)
    );
    let spec = parse_spec(&spec_with(
        TWO,
        r#", "border": "afterglow", "wait": "pending", "glow": true"#,
    ))
    .unwrap();
    assert_eq!(
        (spec.border, spec.wait, spec.glow),
        (BorderStyle::Afterglow, Wait::Pending, true)
    );
    let e = errors(&spec_with(TWO, r#", "border": "neon""#));
    assert!(
        e[0].message.starts_with("unknown variant `neon`"),
        "{}",
        e[0].message
    );
}

#[test]
fn hints_name_nodes_and_do_not_contradict_each_other() {
    let rest = r#", "hints": { "first": ["api"], "last": ["api", "dbb"], "sameLayer": [["db"]] }"#;
    let e = errors(&spec_with(TWO, rest));
    assert_eq!(
        pointers(&e),
        ["/hints/last/1", "/hints", "/hints/sameLayer/0"]
    );
}

/// `check` refuses what `build` would: a hint the edges' order contradicts,
/// by an edge, along a path, or through another group.
#[test]
fn hints_do_not_contradict_the_edges() {
    let nodes = r#"{ "id": "a", "kind": "service", "label": "A" }, { "id": "b", "kind": "service", "label": "B" },
        { "id": "c", "kind": "service", "label": "C" }, { "id": "d", "kind": "service", "label": "D" }"#;
    let chain = r#", "edges": [{ "from": "a", "to": "b" }, { "from": "b", "to": "c" }]"#;
    let refused = |edges: &str, hints: &str| {
        let e = errors(&spec_with(nodes, &format!(r#"{edges}, "hints": {hints}"#)));
        let messages: Vec<String> = e.iter().map(|x| x.message.clone()).collect();
        (pointers(&e), messages)
    };

    let (at, said) = refused(chain, r#"{ "first": ["b"], "sameLayer": [["a", "b"]] }"#);
    assert_eq!(at, ["/hints/sameLayer/0", "/hints/first/0"]);
    assert_eq!(
        said[0],
        "`a` and `b` cannot share a layer: an edge joins them"
    );

    let (at, said) = refused(chain, r#"{ "sameLayer": [["a", "c"]] }"#);
    assert_eq!(at, ["/hints/sameLayer/0"]);
    assert_eq!(
        said[0],
        "`a` and `c` cannot share a layer: the edges lead from `a` to `c`"
    );

    let crossed = r#", "edges": [{ "from": "a", "to": "b" }, { "from": "c", "to": "d" }]"#;
    let (at, said) = refused(crossed, r#"{ "sameLayer": [["a", "d"], ["b", "c"]] }"#);
    assert_eq!(at, ["/hints/sameLayer/1"]);
    assert_eq!(
        said[0],
        "`c` and `b` cannot share a layer: the edges lead from `c` to `b`"
    );
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
        [
            "/nodes/1/id",
            "/nodes/0/label",
            "/nodes/1/tech",
            "/edges/0/to"
        ]
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
