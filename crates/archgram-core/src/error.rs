//! Problems with a spec, each located so the author can find it.

use std::fmt;

/// Where in the spec a problem is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Location {
    /// A JSON pointer (RFC 6901) into the spec, such as `/nodes/2/frame`.
    Pointer(String),
    /// A line and column, counted from 1, for problems found while reading
    /// the text itself.
    LineColumn { line: usize, column: usize },
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Location::Pointer(p) if p.is_empty() => f.write_str("/"),
            Location::Pointer(p) => f.write_str(p),
            Location::LineColumn { line, column } => write!(f, "{line}:{column}"),
        }
    }
}

/// One problem with a spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecError {
    pub location: Location,
    pub message: String,
}

impl SpecError {
    pub(crate) fn at(pointer: impl Into<String>, message: impl Into<String>) -> Self {
        SpecError {
            location: Location::Pointer(pointer.into()),
            message: message.into(),
        }
    }
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}

impl std::error::Error for SpecError {}
