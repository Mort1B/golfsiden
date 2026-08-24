use serde::Serialize;
use uuid::Uuid;

use crate::domain::models::{ParticipantStatus, RoundStatus, ScoringFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaderboardMetric {
    Gross,
    Net,
}

#[derive(Debug, Clone)]
pub struct RoundFact {
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub round_number: i16,
    pub status: RoundStatus,
    pub scoring_format: ScoringFormat,
    pub number_of_holes: i16,
    pub handicap_enabled: bool,
    pub handicap_allowance_percent: i16,
}

#[derive(Debug, Clone)]
pub struct HoleFact {
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
}

#[derive(Debug, Clone)]
pub struct SnapshotFact {
    pub round_id: Uuid,
    pub player_id: Uuid,
    pub display_name: String,
    pub course_handicap: i16,
    pub playing_handicap: i16,
}

#[derive(Debug, Clone)]
pub struct TeamSnapshotFact {
    pub round_id: Uuid,
    pub team_id: Uuid,
    pub playing_handicap: i16,
}

#[derive(Debug, Clone)]
pub struct TeamFact {
    pub round_id: Uuid,
    pub team_id: Uuid,
    pub team_name: String,
}

#[derive(Debug, Clone)]
pub struct MembershipFact {
    pub round_id: Uuid,
    pub team_id: Uuid,
    pub player_id: Uuid,
    pub display_name: String,
    pub display_order: Option<i16>,
}

#[derive(Debug, Clone)]
pub struct ScoreFact {
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub player_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
    pub gross_strokes: i16,
}

#[derive(Debug, Clone)]
pub struct ConfirmationFact {
    pub round_id: Uuid,
    pub player_id: Option<Uuid>,
    pub team_id: Option<Uuid>,
}

#[derive(Debug, Clone)]
pub struct RoundLeaderboardFacts {
    pub round: RoundFact,
    pub holes: Vec<HoleFact>,
    pub snapshots: Vec<SnapshotFact>,
    pub team_snapshots: Vec<TeamSnapshotFact>,
    pub teams: Vec<TeamFact>,
    pub memberships: Vec<MembershipFact>,
    pub scores: Vec<ScoreFact>,
    pub confirmations: Vec<ConfirmationFact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LeaderboardOwner {
    Player { id: Uuid },
    Team { id: Uuid },
}

impl LeaderboardOwner {
    pub(crate) fn id(self) -> Uuid {
        match self {
            Self::Player { id } | Self::Team { id } => id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeaderboardMember {
    pub player_id: Uuid,
    pub display_name: String,
    pub display_order: Option<i16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoundLeaderboardEntry {
    pub position: Option<usize>,
    pub tied: bool,
    pub owner: LeaderboardOwner,
    pub owner_name: String,
    pub members: Vec<LeaderboardMember>,
    pub holes_scored: usize,
    pub number_of_holes: usize,
    pub complete: bool,
    pub confirmed: bool,
    pub playing_handicap: i32,
    pub gross_total: i32,
    pub net_total: i32,
    pub par_played: i32,
    pub score_to_par: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoundLeaderboard {
    pub round_id: Uuid,
    pub tournament_id: Uuid,
    pub status: RoundStatus,
    pub scoring_format: ScoringFormat,
    pub metric: LeaderboardMetric,
    pub number_of_holes: usize,
    pub entries: Vec<RoundLeaderboardEntry>,
}

#[derive(Debug, Clone)]
pub struct ParticipantFact {
    pub player_id: Uuid,
    pub display_name: String,
    pub status: ParticipantStatus,
}

#[derive(Debug, Clone)]
pub struct TournamentLeaderboardFacts {
    pub tournament_id: Uuid,
    pub participants: Vec<ParticipantFact>,
    pub rounds: Vec<RoundLeaderboardFacts>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CurrentTeam {
    pub round_id: Uuid,
    pub team_id: Uuid,
    pub team_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TournamentLeaderboardEntry {
    pub position: Option<usize>,
    pub tied: bool,
    pub player_id: Uuid,
    pub display_name: String,
    pub status: ParticipantStatus,
    pub completed_rounds: usize,
    pub gross_total: i32,
    pub net_total: i32,
    pub current_team: Option<CurrentTeam>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TournamentLeaderboard {
    pub tournament_id: Uuid,
    pub metric: LeaderboardMetric,
    pub current_round_id: Option<Uuid>,
    pub included_round_ids: Vec<Uuid>,
    pub entries: Vec<TournamentLeaderboardEntry>,
}
