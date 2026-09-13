use super::{HOLE_COUNT, HoleOutcome, MatchPlayError, Opponent};
use crate::domain::player_score_input::GrossScore;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MatchMode {
    Gross,
    #[default]
    Net,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandicapAllocation {
    relative: [i32; 2],
}

impl HandicapAllocation {
    /// Takes already-preserved Playing Handicaps. This does not calculate or freeze
    /// opening snapshots. Widen before subtracting: opposite i16 extremes differ by 65535.
    pub fn new(mode: MatchMode, playing_handicaps: [i16; 2]) -> Self {
        let [first, second] = playing_handicaps.map(i32::from);
        let lowest = first.min(second);
        Self {
            relative: match mode {
                MatchMode::Gross => [0, 0],
                MatchMode::Net => [first - lowest, second - lowest],
            },
        }
    }

    pub fn relative_handicaps(self) -> [i32; 2] {
        self.relative
    }

    pub fn strokes_for_hole(self, stroke_index: u8) -> Result<[i32; 2], MatchPlayError> {
        if !(1..=HOLE_COUNT).contains(&stroke_index) {
            return Err(MatchPlayError::InvalidHole);
        }
        let holes = i32::from(HOLE_COUNT);
        Ok(self.relative.map(|handicap| {
            handicap / holes + i32::from(i32::from(stroke_index) <= handicap % holes)
        }))
    }

    /// Proposes an outcome only from two completed numeric scores. Missing scores
    /// cannot become losses/halves. A conceded next stroke must already be included
    /// in the supplied score and retain provenance in the future reporting layer.
    /// The caller must explicitly accept a report; this never rewrites one.
    pub fn propose_numeric_hole(
        self,
        gross: [GrossScore; 2],
        stroke_index: u8,
    ) -> Result<HoleOutcome, MatchPlayError> {
        let [first_strokes, second_strokes] = self.strokes_for_hole(stroke_index)?;
        let [first_gross, second_gross] = gross.map(GrossScore::value);
        Ok(
            match (first_gross - first_strokes).cmp(&(second_gross - second_strokes)) {
                std::cmp::Ordering::Less => HoleOutcome::WonBy(Opponent::First),
                std::cmp::Ordering::Equal => HoleOutcome::Halved,
                std::cmp::Ordering::Greater => HoleOutcome::WonBy(Opponent::Second),
            },
        )
    }
}
