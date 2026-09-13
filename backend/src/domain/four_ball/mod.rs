//! Pure foundation for future 18-hole four-ball; not a selectable round format.

mod handicap;

pub use handicap::{DEFAULT_ALLOWANCE_PERCENT, calculate_handicap};

pub use super::player_score_input::{GrossScore, PlayerHoleInput};
use super::scoring::handicap_strokes_for_hole;
use uuid::Uuid;

pub const HOLE_COUNT: usize = 18;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FourBallError {
    #[error("allowance must be between 0 and 100 percent")]
    InvalidAllowance,
    #[error("handicap exceeds the supported snapshot range")]
    HandicapOutOfRange,
    #[error("four-ball requires two distinct players")]
    DuplicatePlayer,
    #[error("stroke indexes must be a permutation of 1 through 18")]
    InvalidStrokeIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerHole {
    pub player_id: Uuid,
    /// Preserved per-player Playing Handicap, never the current profile value.
    pub playing_handicap: i16,
    pub input: PlayerHoleInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartnerCard {
    pub player_id: Uuid,
    pub playing_handicap: i16,
    pub holes: [PlayerHoleInput; HOLE_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectedScore {
    pub score: i32,
    /// All equal winners, ordered by stable player identity for display only.
    pub player_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideHole {
    pub gross: SelectedScore,
    pub net: SelectedScore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideTotals {
    pub gross: i32,
    pub net: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideCard {
    pub holes: [Option<SideHole>; HOLE_COUNT],
    pub scored_holes: u8,
    /// None means no side-hole result, not a round total of zero.
    pub totals: Option<SideTotals>,
}

impl SideCard {
    /// Arithmetic completeness only, not authorization or persisted confirmation.
    pub fn is_complete(&self) -> bool {
        usize::from(self.scored_holes) == HOLE_COUNT
    }
}

pub fn aggregate_hole(
    partners: [PlayerHole; 2],
    stroke_index: u8,
) -> Result<Option<SideHole>, FourBallError> {
    let [first, second] = partners;
    if first.player_id == second.player_id {
        return Err(FourBallError::DuplicatePlayer);
    }
    if !(1..=18).contains(&stroke_index) {
        return Err(FourBallError::InvalidStrokeIndex);
    }
    let mut gross = None;
    let mut net = None;
    for partner in partners {
        if let PlayerHoleInput::Numeric(score) = partner.input {
            // The i16 snapshot bound keeps signed allocation and net arithmetic safe.
            let strokes = handicap_strokes_for_hole(
                i32::from(partner.playing_handicap),
                i32::from(stroke_index),
                18,
            )
            .map_err(|_| FourBallError::InvalidStrokeIndex)?;
            select(&mut gross, score.value(), partner.player_id);
            select(&mut net, score.value() - strokes, partner.player_id);
        }
    }
    Ok(gross.zip(net).map(|(gross, net)| SideHole { gross, net }))
}

fn select(selected: &mut Option<SelectedScore>, score: i32, player_id: Uuid) {
    match selected {
        Some(current) if score == current.score => {
            current.player_ids.push(player_id);
            current.player_ids.sort_unstable();
        }
        Some(current) if score > current.score => {}
        _ => {
            *selected = Some(SelectedScore {
                score,
                player_ids: vec![player_id],
            });
        }
    }
}

/// Both the inputs and allocation describe a full 18-hole card. Apply visibility
/// filtering before any future public projection; do not publish this full result.
pub fn aggregate_card(
    partners: [PartnerCard; 2],
    stroke_indexes: [u8; HOLE_COUNT],
) -> Result<SideCard, FourBallError> {
    let [first, second] = partners;
    if first.player_id == second.player_id {
        return Err(FourBallError::DuplicatePlayer);
    }
    let mut sorted_indexes = stroke_indexes;
    sorted_indexes.sort_unstable();
    if sorted_indexes.into_iter().ne(1..=18) {
        return Err(FourBallError::InvalidStrokeIndex);
    }
    let mut card = SideCard {
        holes: std::array::from_fn(|_| None),
        scored_holes: 0,
        totals: None,
    };
    for (((hole, stroke_index), first_input), second_input) in card
        .holes
        .iter_mut()
        .zip(stroke_indexes)
        .zip(first.holes)
        .zip(second.holes)
    {
        *hole = aggregate_hole(
            [
                PlayerHole {
                    player_id: first.player_id,
                    playing_handicap: first.playing_handicap,
                    input: first_input,
                },
                PlayerHole {
                    player_id: second.player_id,
                    playing_handicap: second.playing_handicap,
                    input: second_input,
                },
            ],
            stroke_index,
        )?;
        if let Some(result) = hole {
            card.scored_holes += 1;
            let totals = card.totals.get_or_insert(SideTotals { gross: 0, net: 0 });
            // At most 18 holes, gross 1..20 and signed i16 handicaps bound these sums.
            totals.gross += result.gross.score;
            totals.net += result.net.score;
        }
    }
    Ok(card)
}

#[cfg(test)]
mod tests;
