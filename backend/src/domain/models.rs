use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "tournament_status", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TournamentStatus {
    Draft,
    Active,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "scoring_mode", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ScoringMode {
    Individual,
    Team,
    Combined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "scoring_format", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum ScoringFormat {
    IndividualStrokePlay,
    TeamScramble,
    TwoPlayerFoursomes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "tournament_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum TournamentRole {
    Admin,
    Scorer,
    Player,
    Viewer,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Tournament {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub number_of_rounds: i16,
    pub counted_rounds: i16,
    pub mandatory_round_id: Option<Uuid>,
    pub status: TournamentStatus,
    pub scoring_mode: ScoringMode,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MyTournament {
    pub tournament: Tournament,
    pub role: TournamentRole,
    pub player_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TournamentPlayer {
    pub tournament_id: Uuid,
    pub player_id: Uuid,
    pub display_name: String,
    pub player_active: bool,
    pub tournament_handicap: f64,
    pub seed: Option<i16>,
    pub status: ParticipantStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct TournamentHandicapHistoryEntry {
    pub id: Uuid,
    pub tournament_id: Uuid,
    pub player_id: Uuid,
    pub handicap_index: f64,
    pub effective_from: DateTime<Utc>,
    pub changed_by: Option<Uuid>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, sqlx::Type)]
#[sqlx(
    type_name = "tournament_handicap_lock_reason",
    rename_all = "snake_case"
)]
#[serde(rename_all = "snake_case")]
pub enum TournamentHandicapLockReason {
    RoundOpened,
    SnapshotCaptured,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TournamentHandicapCorrectionState {
    Editable,
    Locked {
        reason: TournamentHandicapLockReason,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct TournamentPlayerRoster {
    pub handicap_correction: TournamentHandicapCorrectionState,
    pub players: Vec<TournamentPlayer>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TournamentHandicapCorrection {
    pub player: TournamentPlayer,
    pub audit: TournamentHandicapHistoryEntry,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessIssueCode {
    RoundNotDraft,
    TournamentNotOpenable,
    NoActiveEntrants,
    MissingTeamAssignment,
    IneligibleTeamAssignment,
    EmptyTeam,
    InvalidScrambleTeamSize,
    InvalidFoursomesTeamSize,
    MissingFlightAssignment,
    IneligibleFlightAssignment,
    EmptyFlight,
    LegacyIndividualGroupsPresent,
    TeamSplitAcrossFlights,
    MissingCourse,
    MissingTee,
    MismatchedCourseTee,
    MissingHandicapRatings,
    InvalidHoleCount,
    InvalidHoleNumbers,
    InvalidStrokeIndexes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessIssue {
    pub code: ReadinessIssueCode,
    pub message: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessPlayer {
    pub player_id: Uuid,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessTeamSize {
    pub team_id: Uuid,
    pub team_name: String,
    pub player_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReadinessFlightSize {
    pub flight_id: Uuid,
    pub flight_name: String,
    pub player_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PairingValidation {
    pub round_id: Uuid,
    pub ready: bool,
    pub issues: Vec<ReadinessIssue>,
    pub missing_players: Vec<ReadinessPlayer>,
    pub ineligible_players: Vec<ReadinessPlayer>,
    pub team_sizes: Vec<ReadinessTeamSize>,
    pub missing_flight_players: Vec<ReadinessPlayer>,
    pub ineligible_flight_players: Vec<ReadinessPlayer>,
    pub flight_sizes: Vec<ReadinessFlightSize>,
    pub legacy_individual_groups: Vec<ReadinessTeamSize>,
    pub split_teams: Vec<ReadinessTeamSize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, FromRow)]
pub struct RoundHandicapSnapshot {
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub player_id: Uuid,
    pub handicap_index: f64,
    pub course_handicap: i16,
    pub playing_handicap: i16,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, FromRow)]
pub struct RoundTeamHandicapSnapshot {
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub team_id: Uuid,
    pub playing_handicap: i16,
    pub captured_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenRoundResult {
    pub round: Round,
    pub handicap_snapshots: Vec<RoundHandicapSnapshot>,
    pub team_handicap_snapshots: Vec<RoundTeamHandicapSnapshot>,
}
