//! Separate units prevent a points-derived contribution being used as strokes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrossNet<T> {
    pub gross: T,
    pub net: T,
}

/// Native Stableford points. Higher values rank ahead of lower values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StablefordPoints(pub(super) i32);

impl StablefordPoints {
    pub fn value(self) -> i32 {
        self.0
    }
}

/// Points-derived overall comparison value. Lower values rank ahead.
/// This is neither an actual stroke total nor a sum of points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverallEquivalent(pub(super) i32);

impl OverallEquivalent {
    pub fn value(self) -> i32 {
        self.0
    }
}

/// Recorded gross strokes or their handicap-adjusted net value, not capped at
/// the Stableford zero-point threshold. Adjusted net values can be negative.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StrokeTotal(pub(super) i32);

impl StrokeTotal {
    pub fn value(self) -> i32 {
        self.0
    }
}
