//! Pure foundation for future individual Stableford; not a selectable format.

mod values;

pub use super::player_score_input::{GrossScore, PlayerHoleInput};
use super::scoring::handicap_strokes_for_hole;
pub use values::{GrossNet, OverallEquivalent, StablefordPoints, StrokeTotal};

pub const HOLE_COUNT: usize = 18;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum StablefordError {
    #[error("hole par must be between 2 and 7")]
    InvalidPar,
    #[error("stroke indexes must be a permutation of 1 through 18")]
    InvalidStrokeIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoleLayout {
    pub par: u8,
    pub stroke_index: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoleResult {
    pub points: GrossNet<StablefordPoints>,
    /// A pickup has no numeric gross or net strokes.
    pub strokes: Option<GrossNet<StrokeTotal>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardTotals {
    pub points: GrossNet<StablefordPoints>,
    pub overall: GrossNet<OverallEquivalent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardResult {
    /// Hidden or unentered holes have no result. The caller owns the visibility mask.
    pub holes: [Option<HoleResult>; HOLE_COUNT],
    pub visible_hole_count: u8,
    pub resolved_holes: u8,
    /// No resolved visible holes means no totals, not a zero-point entry.
    pub totals: Option<CardTotals>,
    /// Present only for 18 visible numeric holes; never a partial subtotal.
    pub full_stroke_totals: Option<GrossNet<StrokeTotal>>,
}

impl CardResult {
    /// Arithmetic completeness, not authorization or persisted confirmation.
    pub fn is_complete(&self) -> bool {
        usize::from(self.resolved_holes) == HOLE_COUNT
    }
}

pub fn calculate_hole(
    input: PlayerHoleInput,
    layout: HoleLayout,
    playing_handicap: i16,
) -> Result<Option<HoleResult>, StablefordError> {
    validate_hole(layout)?;
    let strokes = match input {
        PlayerHoleInput::Unentered => return Ok(None),
        PlayerHoleInput::NoScore => None,
        PlayerHoleInput::Numeric(gross) => {
            // Signed i16 snapshots keep allocation and all arithmetic in i32 range.
            let received = handicap_strokes_for_hole(
                i32::from(playing_handicap),
                i32::from(layout.stroke_index),
                18,
            )
            .map_err(|_| StablefordError::InvalidStrokeIndex)?;
            Some(GrossNet {
                gross: StrokeTotal(gross.value()),
                net: StrokeTotal(gross.value() - received),
            })
        }
    };
    let points = match strokes {
        Some(strokes) => GrossNet {
            gross: points_for_strokes(layout.par, strokes.gross),
            net: points_for_strokes(layout.par, strokes.net),
        },
        None => GrossNet {
            gross: StablefordPoints(0),
            net: StablefordPoints(0),
        },
    };
    Ok(Some(HoleResult { points, strokes }))
}

fn validate_hole(layout: HoleLayout) -> Result<(), StablefordError> {
    if !(2..=7).contains(&layout.par) {
        return Err(StablefordError::InvalidPar);
    }
    if !(1..=18).contains(&layout.stroke_index) {
        return Err(StablefordError::InvalidStrokeIndex);
    }
    Ok(())
}

fn points_for_strokes(par: u8, strokes: StrokeTotal) -> StablefordPoints {
    StablefordPoints((2 + i32::from(par) - strokes.0).max(0))
}

pub fn calculate_card(
    inputs: [PlayerHoleInput; HOLE_COUNT],
    layout: [HoleLayout; HOLE_COUNT],
    playing_handicap: i16,
) -> Result<CardResult, StablefordError> {
    project_card(inputs, layout, playing_handicap, [true; HOLE_COUNT])
}

/// The caller must supply an authorized mask. This is only arithmetic projection,
/// not membership authorization or the policy excluding a hidden completed final.
/// Keep the full layout/snapshot; hidden inputs are skipped before calculation.
pub fn project_card(
    inputs: [PlayerHoleInput; HOLE_COUNT],
    layout: [HoleLayout; HOLE_COUNT],
    playing_handicap: i16,
    visible: [bool; HOLE_COUNT],
) -> Result<CardResult, StablefordError> {
    for hole in layout {
        validate_hole(hole)?;
    }
    let mut indexes = layout.map(|hole| hole.stroke_index);
    indexes.sort_unstable();
    if indexes.into_iter().ne(1..=18) {
        return Err(StablefordError::InvalidStrokeIndex);
    }
    let mut card = CardResult {
        holes: [None; HOLE_COUNT],
        visible_hole_count: 0,
        resolved_holes: 0,
        totals: None,
        full_stroke_totals: None,
    };
    let mut numeric_holes = 0;
    let mut strokes = GrossNet { gross: 0, net: 0 };
    let mut points = GrossNet { gross: 0, net: 0 };
    for (((result, input), hole), is_visible) in
        card.holes.iter_mut().zip(inputs).zip(layout).zip(visible)
    {
        if !is_visible {
            continue;
        }
        card.visible_hole_count += 1;
        *result = calculate_hole(input, hole, playing_handicap)?;
        if let Some(hole_result) = result {
            card.resolved_holes += 1;
            points.gross += hole_result.points.gross.0;
            points.net += hole_result.points.net.0;
            if let Some(numeric) = hole_result.strokes {
                numeric_holes += 1;
                strokes.gross += numeric.gross.0;
                strokes.net += numeric.net.0;
            }
        }
    }
    // Exactly 18 holes and i16 snapshots bound sums well inside i32 range.
    if card.resolved_holes > 0 {
        let baseline = 2 * i32::from(card.resolved_holes);
        card.totals = Some(CardTotals {
            points: GrossNet {
                gross: StablefordPoints(points.gross),
                net: StablefordPoints(points.net),
            },
            overall: GrossNet {
                gross: OverallEquivalent(baseline - points.gross),
                net: OverallEquivalent(baseline - points.net),
            },
        });
    }
    if numeric_holes == HOLE_COUNT {
        card.full_stroke_totals = Some(GrossNet {
            gross: StrokeTotal(strokes.gross),
            net: StrokeTotal(strokes.net),
        });
    }
    Ok(card)
}

#[cfg(test)]
mod tests;
