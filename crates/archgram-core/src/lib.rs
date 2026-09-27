//! archgram's engine: a spec in, a picture out, with no I/O
//! (ARCHITECTURE.md, Bird's eye view). This version reads and checks specs;
//! layout, routing and rendering follow.

mod error;
pub mod spec;
mod validate;

pub use error::{Location, SpecError};
pub use spec::Spec;
pub use validate::{FORMAT_VERSION, PALETTES, validate};

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
