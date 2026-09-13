use crate::domain::{
    four_ball_card::{Entry, Input},
    models::{RoundStatus, ScoringFormat},
    scorecards::{ScoreOwner, ScoreRevision},
};
use crate::repositories::scorecards::ScorecardError;
use chrono::{DateTime, Utc};
use uuid::Uuid;
#[derive(sqlx::FromRow)]
pub(super) struct Context {
    pub id: Uuid,
    pub tournament_id: Uuid,
    pub tee_id: Option<Uuid>,
    pub status: RoundStatus,
    pub scoring_format: ScoringFormat,
    pub round_number: i16,
    pub tournament_round_count: i16,
    pub final_round_back_nine_hidden: bool,
}
#[derive(sqlx::FromRow)]
pub(super) struct InputRow {
    pub id: Uuid,
    pub revision: i64,
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub player_id: Uuid,
    pub gross_strokes: Option<i16>,
    pub submitted_by: Uuid,
    pub submitted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
impl InputRow {
    pub fn entry(self) -> Result<Entry, ScorecardError> {
        Ok(Entry {
            id: self.id,
            revision: ScoreRevision::from_database(self.revision)
                .ok_or(ScorecardError::InvalidStoredData)?,
            round_id: self.round_id,
            hole_id: self.hole_id,
            owner: ScoreOwner::Player { id: self.player_id },
            input: self
                .gross_strokes
                .map(|gross_strokes| Input::Numeric { gross_strokes })
                .unwrap_or(Input::NoScore {}),
            submitted_by: self.submitted_by,
            submitted_at: self.submitted_at,
            updated_at: self.updated_at,
        })
    }
}
