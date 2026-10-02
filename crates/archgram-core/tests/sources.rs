//! The code behind a diagram (docs/SPEC.md, Sources): a source's form is
//! checked with the spec, its presence against what the caller finds, and
//! it is never drawn.

use std::cell::RefCell;

use archgram_core::render::Options;
use archgram_core::sources::{Found, check, count};
use archgram_core::{build, parse_spec};

fn example(name: &str) -> String {
    let path = format!("{}/../../examples/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Two nodes and an edge between them, each with the given `source` JSON.
fn spec(api: &str, db: &str, edge: &str) -> String {
    format!(
        r#"{{ "archgram": 1, "title": "t", "description": "d",
  "nodes": [
    {{ "id": "api", "kind": "service", "label": "API", "source": {api} }},
    {{ "id": "db", "kind": "database", "label": "DB", "source": {db} }}
  ],
  "edges": [ {{ "from": "api", "to": "db", "source": {edge} }} ] }}"#
    )
}

/// A project with `src/api.ts`, holding one call, and the folder `src/db`.
fn project(path: &str) -> Found {
    match path {
        "src/api.ts" => Found::File("export const save = () => db.insert(order);\n".into()),
        "src/db" => Found::Folder,
        "src/lock" => Found::Unreadable("permission denied".into()),
        _ => Found::Nothing,
    }
}

#[test]
fn a_source_is_a_path_from_the_spec_folder_with_one_line_of_text() {
    let ok = spec(
        r#""../../src/api.ts""#,
        r#"["src/db", "src/db/orders.sql"]"#,
        r#""src/api.ts#db.insert(""#,
    );
    parse_spec(&ok).expect("well-formed sources");
    let errors = parse_spec(&spec(
        r#""/src/api.ts""#,
        r#"["src/db", "src\\db"]"#,
        r#""src/api.ts#""#,
    ))
    .expect_err("ill-formed sources");
    let at: Vec<String> = errors.iter().map(|e| e.location.to_string()).collect();
    assert_eq!(
        at,
        ["/nodes/0/source", "/nodes/1/source/1", "/edges/0/source"]
    );
    let errors = parse_spec(&spec("[]", r#""src/db""#, r#""src/api.ts""#)).expect_err("empty");
    assert!(errors[0].message.contains("names no source"), "{errors:?}");
}

#[test]
fn every_source_the_code_lacks_is_named_where_it_is() {
    let s = parse_spec(&spec(
        r#""src/gone.ts""#,
        r#"["src/db", "src/lock", "src/db#orders"]"#,
        r#""src/api.ts#db.delete(""#,
    ))
    .unwrap();
    let errors = check(&s, &project);
    let found: Vec<(String, &str)> = errors
        .iter()
        .map(|e| (e.location.to_string(), e.message.as_str()))
        .collect();
    assert_eq!(
        found,
        [
            (
                "/nodes/0/source".into(),
                "node api: `src/gone.ts` does not exist"
            ),
            (
                "/nodes/1/source/1".into(),
                "node db: `src/lock` cannot be read: permission denied"
            ),
            (
                "/nodes/1/source/2".into(),
                "node db: `src/db` is a folder; the text after `#` is looked for in a file"
            ),
            (
                "/edges/0/source".into(),
                "edge api \u{2192} db: `src/api.ts` does not hold `db.delete(`"
            ),
        ]
    );
}

#[test]
fn code_that_is_there_passes_each_path_looked_up_once() {
    let s = parse_spec(&spec(
        r#""src/api.ts""#,
        r#""src/db""#,
        r#""src/api.ts#db.insert(""#,
    ))
    .unwrap();
    let asked = RefCell::new(Vec::new());
    let find = |path: &str| {
        asked.borrow_mut().push(path.to_owned());
        project(path)
    };
    assert_eq!(check(&s, &find), []);
    assert_eq!(*asked.borrow(), ["src/api.ts", "src/db"]);
    assert_eq!(count(&s), 3);
}

#[test]
fn a_spec_without_sources_asks_for_nothing() {
    let s = parse_spec(&example("linkshort.json")).unwrap();
    let find = |path: &str| -> Found { panic!("looked up {path}") };
    assert_eq!(check(&s, &find), []);
    assert_eq!(count(&s), 0);
}

#[test]
fn sources_are_never_drawn() {
    let plain = example("linkshort.json");
    let with = plain
        .replace(
            r#""label": "Worker""#,
            r#""label": "Worker", "source": "../worker/main.py""#,
        )
        .replace(
            r#"{ "from": "worker", "to": "postgres" }"#,
            r#"{ "from": "worker", "to": "postgres", "source": "../worker/main.py#UPDATE links" }"#,
        );
    assert_ne!(plain, with, "the sources are in the spec");
    assert_eq!(
        build(&with, Options::default()).unwrap(),
        build(&plain, Options::default()).unwrap()
    );
}
