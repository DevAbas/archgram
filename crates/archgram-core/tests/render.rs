//! Drawing: each example renders to the SVG kept in `tests/golden/`, the
//! same bytes every time. A change to the drawing shows up as a diff of
//! those files; to accept it, run `ARCHGRAM_BLESS=1 cargo test` and review
//! the diff before committing.

use archgram_core::build;
use archgram_core::render::{Mode, Options};

fn example(name: &str) -> String {
    let path = format!("{}/../../examples/{name}.json", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

fn golden(name: &str, mode: Mode) {
    let svg = build(
        &example(name),
        Options {
            mode,
            ..Options::default()
        },
    )
    .unwrap_or_else(|e| panic!("{name}: {e:?}"));
    let suffix = match mode {
        Mode::Auto => "",
        Mode::Light => ".light",
        Mode::Dark => ".dark",
    };
    let path = format!(
        "{}/tests/golden/{name}{suffix}.svg",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("ARCHGRAM_BLESS").is_some() {
        std::fs::write(&path, &svg).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| panic!("{path} is missing; run ARCHGRAM_BLESS=1 cargo test"));
    assert!(
        svg == expected,
        "{name}{suffix} differs from {path}; if the change is intended, run ARCHGRAM_BLESS=1 cargo test and review the diff"
    );
}

#[test]
fn examples_match_their_golden_files() {
    for name in [
        "linkshort",
        "rag",
        "kinds",
        "kinds-vertical",
        "cv-screener-architecture",
    ] {
        golden(name, Mode::Auto);
    }
    golden("kinds", Mode::Light);
    golden("kinds", Mode::Dark);
}

#[test]
fn the_same_spec_draws_the_same_bytes() {
    let spec = example("rag");
    let first = build(&spec, Options::default()).unwrap();
    for _ in 0..3 {
        assert_eq!(build(&spec, Options::default()).unwrap(), first);
    }
}

#[test]
fn the_svg_carries_its_title_and_description_and_no_invalid_numbers() {
    let svg = build(&example("linkshort"), Options::default()).unwrap();
    assert!(svg.contains(r#"<title id="title">linkshort</title>"#));
    assert!(svg.contains(r#"<desc id="desc">The API creates short links"#));
    assert!(svg.contains(r#"role="img" aria-labelledby="title desc""#));
    for bad in ["NaN", "inf", "<script"] {
        assert!(!svg.contains(bad), "{bad}");
    }
}

#[test]
fn a_single_theme_has_no_media_query() {
    let light = build(
        &example("kinds"),
        Options {
            mode: Mode::Light,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(!light.contains("prefers-color-scheme"));
    let auto = build(
        &example("kinds"),
        Options {
            mode: Mode::Auto,
            ..Options::default()
        },
    )
    .unwrap();
    assert!(auto.contains("@media (prefers-color-scheme: dark)"));
}

#[test]
fn a_logo_goes_in_the_corner_or_on_a_chip() {
    use std::collections::BTreeMap;
    let logos: BTreeMap<String, String> = [("redis".to_owned(), "M2 2h20v20H2z".to_owned())].into();
    let spec = |logo: &str| {
        format!(
            r#"{{ "archgram": 1, "title": "t", "description": "d", "logo": "{logo}",
            "nodes": [{{ "id": "c", "kind": "cache", "label": "Cache", "tech": "redis" }}, {{ "id": "a", "kind": "service", "label": "API" }}] }}"#
        )
    };
    let corner = archgram_core::build_with(&spec("corner"), Options::default(), &logos).unwrap();
    assert_eq!(corner.matches(r#"class="logo""#).count(), 1);
    assert!(!corner.contains(r#"class="logo-chip""#));
    let chip = archgram_core::build_with(&spec("chip"), Options::default(), &logos).unwrap();
    assert_eq!(chip.matches(r#"class="logo-chip""#).count(), 1);
    assert_eq!(chip.matches(r#"class="logo""#).count(), 1);
    // Inline: the logo leads the note line; icon: it takes the icon's place.
    let inline = archgram_core::build_with(&spec("inline"), Options::default(), &logos).unwrap();
    assert_eq!(inline.matches(r#"class="logo""#).count(), 1);
    let icon = archgram_core::build_with(&spec("icon"), Options::default(), &logos).unwrap();
    assert_eq!(icon.matches(r#"class="logo-icon core""#).count(), 1);
    assert_eq!(
        icon.matches(r#"class="icon "#).count(),
        1,
        "one kind icon left, the API's"
    );
    // A slug the set lacks is refused, with the nearest one offered.
    let wrong = spec("corner").replace(r#""tech": "redis""#, r#""tech": "rediss""#);
    let errors = archgram_core::build_with(&wrong, Options::default(), &logos).unwrap_err();
    assert!(
        errors[0].message.contains("did you mean `redis`?"),
        "{}",
        errors[0].message
    );
}

/// Every class a still shape names has a rule in the style table, so a
/// rasterizer drawing the scene finds each one's paint.
#[test]
fn every_still_shape_has_a_style_rule() {
    use archgram_core::render::scene::{Item, StyleLine};
    fn classes(items: &[Item], out: &mut Vec<String>) {
        for it in items {
            match it {
                Item::Group { items, .. } => classes(items, out),
                Item::Rect { class, .. }
                | Item::Circle { class, .. }
                | Item::Path { class, .. }
                | Item::Text { class, .. }
                | Item::Icon { class, .. } => out.push(class.clone()),
                Item::Canvas { .. } | Item::Arrowhead { .. } | Item::Motion(_) => {}
            }
        }
    }
    for name in [
        "linkshort",
        "rag",
        "kinds",
        "kinds-vertical",
        "cv-screener-architecture",
    ] {
        let spec = archgram_core::parse_spec(&example(name)).unwrap();
        let sizes = archgram_core::measure::card_sizes(&spec);
        let placement = archgram_core::layout::place(&spec, &sizes).unwrap();
        let scene = archgram_core::render::scene(
            &spec,
            &placement,
            Options::default(),
            &archgram_core::logos::NoLogos,
        );
        let selectors: Vec<String> = scene
            .style
            .iter()
            .filter_map(|l| match l {
                StyleLine::Rule(r) => Some(r.selector.clone()),
                StyleLine::Raw(_) => None,
            })
            .collect();
        let mut used = Vec::new();
        classes(&scene.items, &mut used);
        for class in used {
            let parts: Vec<&str> = class.split(' ').collect();
            let whole = format!(".{}", parts.join("."));
            let first = format!(".{}", parts[0]);
            assert!(
                selectors.contains(&whole) || selectors.contains(&first),
                "{name}: no rule for `{class}`"
            );
        }
    }
}

/// Two logos for the brand tests: a blue one, and a near-black one that
/// would not show on a dark card.
struct TwoLogos;

impl archgram_core::logos::Logos for TwoLogos {
    fn path(&self, slug: &str) -> Option<&str> {
        matches!(slug, "blue" | "ink").then_some("M0 0h24v24H0z")
    }
    fn slugs(&self) -> Vec<&str> {
        vec!["blue", "ink"]
    }
    fn colour(&self, slug: &str) -> Option<&str> {
        match slug {
            "blue" => Some("2563EB"),
            "ink" => Some("181717"),
            _ => None,
        }
    }
}

/// A logo is always in its brand's colour, with or without flows; a brand
/// colour that would not show on the card in a theme is the text colour
/// there (DESIGN.md, Components: Technology logo).
#[test]
fn logos_are_in_their_brands_colours() {
    let spec = archgram_core::parse_spec(
        r#"{ "archgram": 1, "title": "t", "description": "d",
        "nodes": [{ "id": "a", "kind": "service", "label": "A", "tech": "blue" },
                  { "id": "b", "kind": "database", "label": "B", "tech": "ink" }],
        "edges": [{ "from": "a", "to": "b" }] }"#,
    )
    .unwrap();
    let svg = archgram_core::draw_with(&spec, Options::default(), &TwoLogos).unwrap();
    assert!(svg.contains(r#"class="logo brand-blue""#), "{svg}");
    assert!(svg.contains(".logo.brand-blue, .logo-icon.brand-blue { fill: #2563eb; }"));
    assert!(svg.contains(".logo.brand-ink, .logo-icon.brand-ink { fill: #181717; }"));
    assert!(svg.contains(
        "@media (prefers-color-scheme: dark) { .logo.brand-ink, .logo-icon.brand-ink { fill: var(--text); } }"
    ));
    assert!(
        !svg.contains(r#"class="brand "#),
        "no copy only motion shows"
    );
}

/// The credit is drawn in the legend's type, muted, and `credit: false`
/// leaves out both the text and its rule.
#[test]
fn the_credit_is_drawn_unless_turned_off() {
    let spec = |credit: &str| {
        format!(
            r#"{{ "archgram": 1, "title": "t", "description": "d"{credit},
            "nodes": [{{ "id": "a", "kind": "service", "label": "A" }}] }}"#
        )
    };
    let on = archgram_core::build(&spec(""), Options::default()).unwrap();
    assert!(on.contains(r#"<text class="credit""#), "{on}");
    assert!(on.contains(">by archgram</text>"));
    assert!(on.contains(".credit {"));
    let off = archgram_core::build(&spec(r#", "credit": false"#), Options::default()).unwrap();
    assert!(!off.contains("credit"), "{off}");
    assert!(!off.contains("by archgram"));
}
