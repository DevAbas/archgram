//! archgram's engine: a spec in, a picture out, with no I/O
//! (ARCHITECTURE.md, Bird's eye view).

mod error;
pub mod font;
pub mod geometry;
pub mod layout;
pub mod measure;
pub mod render;
pub mod spec;
pub mod tokens;
mod validate;

pub use error::{Location, SpecError};
pub use spec::Spec;
pub use validate::{FORMAT_VERSION, validate};

/// Reads a JSON spec and checks it. Returns the spec, or every problem found:
/// one problem when the text is not a well-formed spec, all of them when it
/// is well-formed but breaks the rules in docs/SPEC.md.
///
/// # Errors
///
/// The problems, each with its location in the spec.
pub fn parse_spec(json: &str) -> Result<Spec, Vec<SpecError>> {
    let spec: Spec = serde_json::from_str(json).map_err(|e| {
        vec![SpecError {
            location: Location::LineColumn {
                line: e.line(),
                column: e.column(),
            },
            message: e
                .to_string()
                .split(" at line ")
                .next()
                .unwrap_or_default()
                .to_owned(),
        }]
    })?;
    let errors = validate(&spec);
    if errors.is_empty() { Ok(spec) } else { Err(errors) }
}

/// Reads, checks and draws a JSON spec: the whole pipeline.
///
/// # Errors
///
/// The spec's problems, as [`parse_spec`] reports them.
pub fn build(json: &str, options: render::Options) -> Result<String, Vec<SpecError>> {
    draw(&parse_spec(json)?, options)
}

/// Measures, lays out and draws a spec that has passed validation.
///
/// # Errors
///
/// Layout hints that contradict the edges.
pub fn draw(spec: &Spec, options: render::Options) -> Result<String, Vec<SpecError>> {
    let sizes = measure::card_sizes(spec);
    let placement = layout::place(spec, &sizes)?;
    Ok(render::render(spec, &placement, options))
}

/// The characters in the spec's text that the embedded font does not have,
/// in order and without repeats. They are drawn in the reader's font.
#[must_use]
pub fn uncovered_characters(spec: &Spec) -> Vec<char> {
    let mut out: Vec<char> = Vec::new();
    for (weight, text) in measure::text_runs(spec) {
        for c in font::uncovered(&text, weight) {
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out
}
