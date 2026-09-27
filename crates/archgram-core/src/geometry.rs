//! Plain geometry shared by the stages.

/// An axis-aligned rectangle in output pixels; `x`, `y` is the top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    #[must_use]
    pub fn right(&self) -> f64 {
        self.x + self.w
    }

    #[must_use]
    pub fn bottom(&self) -> f64 {
        self.y + self.h
    }

    #[must_use]
    pub fn centre_y(&self) -> f64 {
        self.y + self.h / 2.0
    }

    #[must_use]
    pub fn centre_x(&self) -> f64 {
        self.x + self.w / 2.0
    }

    /// Whether the two rectangles share any area (touching edges do not count).
    #[must_use]
    pub fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.right() && other.x < self.right() && self.y < other.bottom() && other.y < self.bottom()
    }
}

/// A point in output pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A width and a height in output pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub w: f64,
    pub h: f64,
}
