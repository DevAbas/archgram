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
    let svg = build(&example(name), Options { mode }).unwrap_or_else(|e| panic!("{name}: {e:?}"));
    let suffix = match mode {
        Mode::Auto => "",
        Mode::Light => ".light",
        Mode::Dark => ".dark",
    };
    let path = format!("{}/tests/golden/{name}{suffix}.svg", env!("CARGO_MANIFEST_DIR"));
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
    for name in ["linkshort", "rag", "kinds", "kinds-vertical"] {
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
    let light = build(&example("kinds"), Options { mode: Mode::Light }).unwrap();
    assert!(!light.contains("prefers-color-scheme"));
    let auto = build(&example("kinds"), Options { mode: Mode::Auto }).unwrap();
    assert!(auto.contains("@media (prefers-color-scheme: dark)"));
}
