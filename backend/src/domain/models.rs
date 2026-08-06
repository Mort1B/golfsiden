use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "tournament_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TournamentStatus {
    Draft,
    Active,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "scoring_mode", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ScoringMode {
    Individual,
    Team,
    Combined,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "participant_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ParticipantStatus {
    Active,
    Withdrawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "round_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum RoundStatus {
    Draft,
    Open,
    Completed,
    Locked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "scoring_format", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ScoringFormat {
    IndividualStrokePlay,
    TeamScramble,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Player {
    pub id: Uuid,
    pub display_name: String,
    pub current_handicap_index: f64,
    pub email: Option<String>,
    pub profile_image_ref: Option<String>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct HandicapHistoryEntry {
    pub id: Uuid,
    pub player_id: Uuid,
    pub handicap_index: f64,
    pub effective_from: DateTime<Utc>,
    pub changed_by: Option<Uuid>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Tournament {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub number_of_rounds: i16,
    pub status: TournamentStatus,
    pub scoring_mode: ScoringMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TournamentPlayer {
    pub tournament_id: Uuid,
    pub player_id: Uuid,
    pub display_name: String,
    pub tournament_handicap: f64,
    pub seed: Option<i16>,
    pub status: ParticipantStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Round {
    pub id: Uuid,
    pub tournament_id: Uuid,
    pub round_number: i16,
    pub name: String,
    pub round_date: NaiveDate,
    pub course_id: Option<Uuid>,
    pub course_name: String,
    pub tee_id: Option<Uuid>,
    pub tee_name: String,
    pub number_of_holes: i16,
    pub status: RoundStatus,
    pub handicap_enabled: bool,
    pub handicap_allowance_percent: i16,
    pub scoring_format: ScoringFormat,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Team {
    pub id: Uuid,
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub name: String,
    pub starting_hole: Option<i16>,
    pub tee_time: Option<NaiveTime>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TeamMember {
    pub player_id: Uuid,
    pub display_name: String,
    pub display_order: Option<i16>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamWithMembers {
    #[serde(flatten)]
    pub team: Team,
    pub members: Vec<TeamMember>,
}
