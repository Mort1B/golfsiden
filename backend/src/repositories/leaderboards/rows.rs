use sqlx::FromRow;
use uuid::Uuid;

use crate::domain::models::{ParticipantStatus, RoundStatus, ScoringFormat};

#[derive(FromRow)]
pub(super) struct RoundRow {
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub round_number: i16,
    pub status: RoundStatus,
    pub scoring_format: ScoringFormat,
    pub number_of_holes: i16,
    pub handicap_enabled: bool,
    pub handicap_allowance_percent: i16,
}

#[derive(FromRow)]
pub(super) struct HoleRow {
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
}

#[derive(FromRow)]
pub(super) struct SnapshotRow {
    pub round_id: Uuid,
    pub player_id: Uuid,
    pub display_name: String,
    pub course_handicap: i16,
    pub playing_handicap: i16,
}

#[derive(FromRow)]
pub(super) struct TeamSnapshotRow {
    pub round_id: Uuid,
    pub team_id: Uuid,
    pub playing_handicap: i16,
}

#[derive(FromRow)]
pub(super) struct TeamRow {
    pub round_id: Uuid,
    pub team_id: Uuid,
    pub team_name: String,
}

#[derive(FromRow)]
pub(super) struct MembershipRow {
    pub round_id: Uuid,
    pub team_id: Uuid,
    pub player_id: Uuid,
    pub display_name: String,
    pub display_order: Option<i16>,
}

#[derive(FromRow)]
pub(super) struct ScoreRow {
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub player_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub gross_strokes: i16,
}

#[derive(FromRow)]
pub(super) struct ConfirmationRow {
    pub round_id: Uuid,
    pub player_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
}

#[derive(FromRow)]
pub(super) struct ParticipantRow {
    pub player_id: Uuid,
    pub display_name: String,
    pub status: ParticipantStatus,
}
