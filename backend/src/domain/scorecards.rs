use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::scoring::{ScoringError, hole_net_score};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScoreOwner {
    Player { id: Uuid },
    Team { id: Uuid },
}

impl ScoreOwner {
    pub fn player_id(self) -> Option<Uuid> {
        match self {
            Self::Player { id } => Some(id),
            Self::Team { .. } => None,
        }
    }

    pub fn team_id(self) -> Option<Uuid> {
        match self {
            Self::Player { .. } => None,
            Self::Team { id } => Some(id),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreEntry {
    pub id: Uuid,
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub owner: ScoreOwner,
    pub gross_strokes: i16,
    pub submitted_by: Uuid,
    pub submitted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ScorecardHoleSource {
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
    pub score: Option<ScoreEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScorecardHole {
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
    pub score: Option<ScoreEntry>,
    pub net_strokes: Option<i32>,
}

#[derive(Debug, Clone, Copy)]
pub struct ConfirmationState {
    pub confirmed_by: Uuid,
    pub confirmed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScorecardSummary {
    pub round_id: Uuid,
    pub owner: ScoreOwner,
    pub holes: Vec<ScorecardHole>,
    pub gross_total: i32,
    pub net_total: i32,
    pub playing_handicap: i32,
    pub holes_scored: usize,
    pub number_of_holes: usize,
    pub complete: bool,
    pub confirmed: bool,
    pub confirmed_by: Option<Uuid>,
    pub confirmed_at: Option<DateTime<Utc>>,
}

pub fn summarize(
    round_id: Uuid,
    owner: ScoreOwner,
    playing_handicap: i32,
    sources: Vec<ScorecardHoleSource>,
    confirmation: Option<ConfirmationState>,
) -> Result<ScorecardSummary, ScoringError> {
    let number_of_holes = sources.len();
    let mut gross_total = 0;
    let mut net_total = 0;
    let mut holes_scored = 0;
    let mut holes = Vec::with_capacity(number_of_holes);
    for source in sources {
        let net_strokes = source
            .score
            .as_ref()
            .map(|score| {
                hole_net_score(
                    i32::from(score.gross_strokes),
                    playing_handicap,
                    i32::from(source.stroke_index),
                    number_of_holes as i32,
                )
            })
            .transpose()?;
        if let Some(score) = &source.score {
            holes_scored += 1;
            gross_total += i32::from(score.gross_strokes);
        }
        if let Some(net) = net_strokes {
            net_total += net;
        }
        holes.push(ScorecardHole {
            hole_id: source.hole_id,
            hole_number: source.hole_number,
            par: source.par,
            stroke_index: source.stroke_index,
            score: source.score,
            net_strokes,
        });
    }
    let complete = number_of_holes > 0 && holes_scored == number_of_holes;
    let confirmation = confirmation.filter(|_| complete);
    Ok(ScorecardSummary {
        round_id,
        owner,
        holes,
        gross_total,
        net_total,
        playing_handicap,
        holes_scored,
        number_of_holes,
        complete,
        confirmed: confirmation.is_some(),
        confirmed_by: confirmation.map(|value| value.confirmed_by),
        confirmed_at: confirmation.map(|value| value.confirmed_at),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(hole_number: i16, stroke_index: i16, gross: Option<i16>) -> ScorecardHoleSource {
        let round_id = Uuid::from_u128(1);
        let player_id = Uuid::from_u128(2);
        ScorecardHoleSource {
            hole_id: Uuid::from_u128(100 + hole_number as u128),
            hole_number,
            par: 4,
            stroke_index,
            score: gross.map(|gross_strokes| ScoreEntry {
                id: Uuid::from_u128(200 + hole_number as u128),
                round_id,
                hole_id: Uuid::from_u128(100 + hole_number as u128),
                owner: ScoreOwner::Player { id: player_id },
                gross_strokes,
                submitted_by: Uuid::from_u128(3),
                submitted_at: Utc::now(),
                updated_at: Utc::now(),
            }),
        }
    }

    #[test]
    fn partial_and_complete_totals_allocate_handicap_by_hole() {
        let owner = ScoreOwner::Player {
            id: Uuid::from_u128(2),
        };
        let partial = summarize(
            Uuid::from_u128(1),
            owner,
            1,
            vec![source(1, 1, Some(5)), source(2, 2, None)],
            None,
        )
        .unwrap();
        assert_eq!((partial.gross_total, partial.net_total), (5, 4));
        assert!(!partial.complete);

        let confirmation = ConfirmationState {
            confirmed_by: Uuid::from_u128(3),
            confirmed_at: Utc::now(),
        };
        let complete = summarize(
            Uuid::from_u128(1),
            owner,
            1,
            vec![source(1, 1, Some(5)), source(2, 2, Some(4))],
            Some(confirmation),
        )
        .unwrap();
        assert_eq!((complete.gross_total, complete.net_total), (9, 8));
        assert!(complete.complete && complete.confirmed);
    }

    #[test]
    fn plus_handicap_adds_strokes_to_net_total() {
        let summary = summarize(
            Uuid::from_u128(1),
            ScoreOwner::Player {
                id: Uuid::from_u128(2),
            },
            -1,
            vec![source(1, 1, Some(4)), source(2, 2, Some(4))],
            None,
        )
        .unwrap();
        assert_eq!((summary.gross_total, summary.net_total), (8, 9));
    }
}
