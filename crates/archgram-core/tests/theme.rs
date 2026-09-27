//! Importing a project's design tokens as the theme (docs/SPEC.md, Theme
//! file): archgram's own tokens, read through a mapping, give back its own
//! colours and drawings; each rule of the format reports its problem where
//! it is.

use std::collections::BTreeMap;

use archgram_core::render::{Mode, Options};
use archgram_core::theme::{ImportError, Imported, import, join};
use archgram_core::tokens::{Role, theme};

fn root() -> String {
    format!("{}/../..", env!("CARGO_MANIFEST_DIR"))
}

/// Files in memory, by path; the rest from the repository.
fn reader(files: BTreeMap<String, String>) -> impl Fn(&str) -> Option<String> {
    move |path: &str| {
        files
            .get(path)
            .cloned()
            .or_else(|| std::fs::read_to_string(format!("{}/{path}", root())).ok())
    }
}

fn files(list: &[(&str, &str)]) -> BTreeMap<String, String> {
    list.iter()
        .map(|(p, t)| ((*p).to_owned(), (*t).to_owned()))
        .collect()
}

fn run(list: &[(&str, &str)]) -> Result<Imported, Vec<ImportError>> {
    import("archgram.theme.json", &reader(files(list)))
}

fn errors(list: &[(&str, &str)]) -> Vec<String> {
    run(list)
        .expect_err("the import should fail")
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// A mapping of every role to archgram's own token of that name.
fn own_mapping() -> String {
    let roles: Vec<String> = Role::ALL
        .iter()
        .map(|r| format!(r#""{0}": "color.{0}""#, r.name()))
        .collect();
    format!(
        r#"{{ "version": 1, "resolver": "design-system/tokens/design.resolver.json",
        "roles": {{ {} }},
        "themes": {{ "light": {{ "inputs": {{ "palette": "mono", "theme": "light" }} }},
                    "dark": {{ "inputs": {{ "theme": "dark" }} }} }} }}"#,
        roles.join(", ")
    )
}

#[test]
fn archgrams_own_tokens_give_back_its_own_colours() {
    let imported = run(&[("archgram.theme.json", &own_mapping())]).unwrap();
    assert_eq!(imported.colors.light, theme("mono", "light").unwrap().colors);
    assert_eq!(imported.colors.dark, theme("mono", "dark").unwrap().colors);
    // Every role says which token filled it.
    assert!(
        imported
            .light
            .iter()
            .all(|(r, id, _)| id.as_deref() == Some(&*format!("color.{}", r.name())))
    );
}

#[test]
fn a_drawing_in_the_imported_theme_is_the_same_bytes() {
    let imported = run(&[("archgram.theme.json", &own_mapping())]).unwrap();
    let spec = std::fs::read_to_string(format!("{}/examples/kinds.json", root())).unwrap();
    let golden =
        std::fs::read_to_string(format!("{}/crates/archgram-core/tests/golden/kinds.svg", root())).unwrap();
    let svg = archgram_core::build(
        &spec,
        Options {
            mode: Mode::Auto,
            colors: Some(imported.colors),
            ..Options::default()
        },
    )
    .unwrap();
    assert!(svg == golden, "the imported theme draws differently");
}

const RESOLVER: &str = r##"{
  "version": "2025.10",
  "sets": {
    "base": { "sources": [{ "$ref": "palette.json" }] },
    "all": { "sources": [{ "$ref": "#/sets/base" }] }
  },
  "modifiers": {
    "mode": {
      "contexts": {
        "day": [{ "$ref": "roles.json#/day" }],
        "night": [{ "$ref": "roles.json#/night" }]
      },
      "default": "day"
    }
  },
  "resolutionOrder": [
    { "$ref": "#/sets/all" },
    { "$ref": "#/modifiers/mode" },
    { "type": "set", "name": "late", "sources": [{ "ink": { "$type": "color", "$value": "{ramp.900}" } }] }
  ]
}"##;

const PALETTE: &str = r##"{
  "ramp": {
    "$type": "color",
    "0": { "$value": { "colorSpace": "srgb", "components": [1, 1, 1], "hex": "#ffffff" } },
    "50": { "$value": "#f4f4f4" },
    "900": { "$value": { "colorSpace": "oklch", "components": [0.2, 0, "none"] } },
    "blue": { "$value": { "colorSpace": "hsl", "components": [215, 60, 42] } },
    "broken": { "$value": "{ramp.5O}" }
  }
}"##;

const ROLES: &str = r##"{
  "day": {
    "surface": { "$type": "color", "$value": "{ramp.0}" },
    "accent": { "$value": "{ramp.blue}" },
    "blue-first": { "$type": "color", "$value": { "colorSpace": "srgb", "components": [{ "$ref": "#/ramp/blue/$value/components/0" }, 0, 0] } }
  },
  "night": {
    "surface": { "$type": "color", "$value": "{ramp.900}" }
  }
}"##;

fn mapping(roles: &str, themes: &str) -> String {
    format!(
        r#"{{ "version": 1, "resolver": "tokens/resolver.json", "roles": {{ {roles} }}, "themes": {{ {themes} }} }}"#
    )
}

fn project(mapping: &str) -> Vec<(&'static str, String)> {
    vec![
        ("archgram.theme.json", mapping.to_owned()),
        ("tokens/resolver.json", RESOLVER.to_owned()),
        ("tokens/palette.json", PALETTE.to_owned()),
        ("tokens/roles.json", ROLES.to_owned()),
    ]
}

fn run_project(m: &str) -> Result<Imported, Vec<ImportError>> {
    let list = project(m);
    let refs: Vec<(&str, &str)> = list.iter().map(|(p, t)| (*p, t.as_str())).collect();
    run(&refs)
}

fn project_errors(m: &str) -> Vec<String> {
    run_project(m)
        .expect_err("the import should fail")
        .iter()
        .map(ToString::to_string)
        .collect()
}

const BOTH: &str = r#""light": { "inputs": { "mode": "day" }, "roles": { "text": "{ink}" } }, "dark": { "inputs": { "mode": "night" } }"#;

#[test]
fn sets_modifiers_aliases_and_pointers_resolve() {
    let m = mapping(r#""card": "surface""#, BOTH);
    let imported = run_project(&m).unwrap();
    let light = imported.colors.light;
    assert_eq!(light.card.to_string(), "#ffffff");
    assert_eq!(light.text.to_string(), "#161616");
    // The dark theme reads the night context; unmapped roles keep mono's.
    assert_eq!(imported.colors.dark.card.to_string(), "#161616");
    assert_eq!(
        imported.colors.dark.canvas,
        theme("mono", "dark").unwrap().colors.canvas
    );
    assert!(
        imported
            .dark
            .iter()
            .any(|(r, id, _)| *r == Role::Canvas && id.is_none())
    );
    // An untyped alias takes its target's type; a pointer reaches inside a value.
    let m = mapping(
        "",
        r#""light": { "inputs": { "mode": "day" }, "roles": { "icon-core": "accent" } }, "dark": { "inputs": { "mode": "night" } }"#,
    );
    assert_eq!(
        run_project(&m).unwrap().colors.light.icon_core.to_string(),
        "#2b60ab"
    );
}

#[test]
fn each_problem_is_reported_where_it_is() {
    let e = project_errors(&mapping(r#""card": "surfac", "paper": "surface""#, BOTH));
    // A role name is checked first, once.
    assert_eq!(
        e,
        [
            "archgram.theme.json /roles/paper: `paper` is not an archgram role; the roles are badge, canvas, card, card-edge, connector, frame, icon-ai, icon-build, icon-client, icon-core, signal-core, text, text-muted"
        ]
    );
    let e = project_errors(&mapping(r#""card": "surfac""#, BOTH));
    assert_eq!(
        e,
        [
            "archgram.theme.json /roles/card: light: `surfac`: no such token; did you mean `surface`?",
            "archgram.theme.json /roles/card: dark: `surfac`: no such token; did you mean `surface`?",
        ]
    );
    let e = project_errors(&mapping(
        "",
        r#""light": { "inputs": { "mode": "dusk" } }, "dark": { "inputs": { "moode": "night" } }"#,
    ));
    assert_eq!(
        e,
        [
            "archgram.theme.json /themes/light/inputs/mode: modifier `mode` has no context `dusk`; it has `day`, `night`",
            "archgram.theme.json /themes/dark/inputs/moode: the resolver has no modifier `moode`; it has `mode`",
        ]
    );
    // A missing alias target is named, not the role's own token.
    let e = project_errors(&mapping(r#""card": "{ramp.broken}""#, BOTH));
    assert_eq!(
        e[0],
        "archgram.theme.json /roles/card: light: `ramp.broken`: `ramp.broken` aliases `ramp.5O`: no such token; did you mean `ramp.50`, `ramp.0`?"
    );
    // The same contrast DESIGN.md holds the built-in palettes to.
    let plain = r#""light": { "inputs": { "mode": "day" } }, "dark": { "inputs": { "mode": "night" } }"#;
    let e = project_errors(&mapping(r#""text": "ramp.50""#, plain));
    assert_eq!(
        e[0],
        "archgram.theme.json /themes/light: light: `text` on `card` is 1.09:1, below 4.5:1"
    );
}

#[test]
fn loops_remote_files_and_extends_are_refused() {
    let looped = r#"{ "version": "2025.10", "resolutionOrder": [{ "type": "set", "name": "s", "sources": [{
        "a": { "$type": "color", "$value": "{b}" }, "b": { "$type": "color", "$value": "{a}" } }] }] }"#;
    let remote = r#"{ "version": "2025.10", "resolutionOrder": [{ "type": "set", "name": "s", "sources": [{ "$ref": "https://example.com/t.json" }] }] }"#;
    let extends = r#"{ "version": "2025.10", "resolutionOrder": [{ "type": "set", "name": "s", "sources": [{ "g": { "$extends": "{h}" } }] }] }"#;
    let m = |r: &str| {
        format!(
            r#"{{ "version": 1, "resolver": "{r}", "roles": {{ "card": "a" }}, "themes": {{ "light": {{}}, "dark": {{}} }} }}"#
        )
    };
    let cases = [
        ("loop.json", looped, "aliases loop: a → b → a"),
        ("remote.json", remote, "not remote references"),
        ("extends.json", extends, "`$extends` is not supported"),
    ];
    for (name, resolver, want) in cases {
        let mapping = m(name);
        let e = errors(&[("archgram.theme.json", &mapping), (name, resolver)]);
        assert!(e[0].contains(want), "{name}: {e:?}");
    }
}

#[test]
fn paths_join_like_the_files_name_each_other() {
    assert_eq!(join("a/b/theme.json", "tokens/r.json"), "a/b/tokens/r.json");
    assert_eq!(join("a/b/theme.json", "../t/./r.json"), "a/t/r.json");
    assert_eq!(join("theme.json", "r.json"), "r.json");
    assert_eq!(join("/p/theme.json", "t/r.json"), "/p/t/r.json");
    assert_eq!(join("/p/theme.json", "/abs/r.json"), "/abs/r.json");
    assert_eq!(join("theme.json", "../up.json"), "../up.json");
}
