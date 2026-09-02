use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::domain::{
    models::{RoundStatus, ScoringFormat},
    scorecards::{ScoreEntry, ScoreOwner, ScorecardHoleSource},
};

use super::ScorecardError;

#[derive(FromRow)]
pub(super) struct RoundContext {
    pub id: Uuid,
    pub tournament_id: Uuid,
    pub tee_id: Option<Uuid>,
    pub number_of_holes: i16,
    pub status: RoundStatus,
    pub handicap_enabled: bool,
    pub handicap_allowance_percent: i16,
    pub scoring_format: ScoringFormat,
}

#[derive(FromRow)]
pub(super) struct ReadVisibilityContext {
    pub round_number: i16,
    pub tournament_round_count: i16,
    pub final_round_back_nine_hidden: bool,
}

#[derive(FromRow)]
pub(super) struct ScoreRow {
    pub id: Uuid,
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub player_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub gross_strokes: i16,
    pub submitted_by: Uuid,
    pub submitted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
pub(super) struct HoleScoreRow {
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
    pub score_id: Option<Uuid>,
    pub player_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub gross_strokes: Option<i16>,
    pub submitted_by: Option<Uuid>,
    pub submitted_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(FromRow)]
pub(super) struct ConfirmationRow {
    pub confirmed_by: Uuid,
    pub confirmed_at: DateTime<Utc>,
}

pub(super) fn hole_source_from_row(
    row: HoleScoreRow,
) -> Result<ScorecardHoleSource, ScorecardError> {
    let score = match (
        row.score_id,
        row.gross_strokes,
        row.submitted_by,
        row.submitted_at,
        row.updated_at,
    ) {
        (
            Some(id),
            Some(gross_strokes),
            Some(submitted_by),
            Some(submitted_at),
            Some(updated_at),
        ) => Some(score_from_row(ScoreRow {
            id,
            round_id: row.round_id,
            hole_id: row.hole_id,
            player_id: row.player_id,
            team_id: row.team_id,
            gross_strokes,
            submitted_by,
            submitted_at,
            updated_at,
        })?),
        (None, None, None, None, None) => None,
        _ => return Err(ScorecardError::InvalidStoredData),
    };
    Ok(ScorecardHoleSource {
        hole_id: row.hole_id,
        hole_number: row.hole_number,
        par: row.par,
        stroke_index: row.stroke_index,
        score,
    })
}

pub(super) fn score_from_row(row: ScoreRow) -> Result<ScoreEntry, ScorecardError> {
    let owner = match (row.player_id, row.team_id) {
        (Some(id), None) => ScoreOwner::Player { id },
        (None, Some(id)) => ScoreOwner::Team { id },
        _ => return Err(ScorecardError::InvalidStoredData),
    };
    Ok(ScoreEntry {
        id: row.id,
        round_id: row.round_id,
        hole_id: row.hole_id,
        owner,
        gross_strokes: row.gross_strokes,
        submitted_by: row.submitted_by,
        submitted_at: row.submitted_at,
        updated_at: row.updated_at,
    })
}
