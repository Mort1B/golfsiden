use crate::domain::{
    fantasy::{
        Points,
        holes::Category,
        projection::{MatchOutcome, Settlement},
    },
    models::{RoundStatus, ScoringFormat},
    score_visibility::VisibilityMetadata,
};
use serde::Serialize;
use uuid::Uuid;
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ResultPoints {
    OmittedNonFinish,
    NotStarted,
    NotParticipating,
    Pending { recorded: i64 },
    Provisional { total: i64 },
    Settled { total: i64 },
    Withheld,
}
impl From<Points> for ResultPoints {
    fn from(p: Points) -> Self {
        match p {
            Points::NotStarted => Self::NotStarted,
            Points::Pending { recorded } => Self::Pending { recorded },
            Points::Provisional(total) => Self::Provisional { total },
            Points::Settled(total) => Self::Settled { total },
            Points::Withheld => Self::Withheld,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct RoundSummary {
    pub round_id: Uuid,
    pub round_number: i16,
    pub name: String,
    pub format: ScoringFormat,
    pub sporting_status: Option<RoundStatus>,
    pub visibility: VisibilityMetadata,
    pub points: ResultPoints,
}
#[derive(Debug, Clone, Serialize)]
pub struct RoundPoints {
    pub round_id: Uuid,
    pub points: ResultPoints,
}
#[derive(Debug, Clone, Serialize)]
pub struct Standing {
    pub id: Uuid,
    pub display_name: String,
    pub points: ResultPoints,
    pub rank: Option<usize>,
    pub rounds: Vec<RoundPoints>,
}
#[derive(Debug, Serialize)]
pub struct Results {
    pub tournament_id: Uuid,
    pub rules_version: i16,
    pub revision: String,
    pub rounds: Vec<RoundSummary>,
    pub golfers: Vec<Standing>,
    pub managers: Vec<Standing>,
    #[serde(skip)]
    pub finalized: bool,
}
#[derive(Debug, Clone, Serialize)]
pub struct HoleResult {
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub category: Option<Category>,
    pub net_strokes: Option<i32>,
    pub points: ResultPoints,
}
#[derive(Debug, Clone, Serialize)]
pub struct GolferResult {
    pub player_id: Uuid,
    pub display_name: String,
    pub points: ResultPoints,
    pub rank: Option<usize>,
    pub team_id: Option<Uuid>,
    pub holes: Vec<HoleResult>,
    pub recorded_hole_points: Option<i64>,
    pub placement_points: Option<i64>,
    pub settlement: Option<Settlement>,
    pub match_outcome: Option<MatchOutcome>,
}
#[derive(Debug, Clone, Serialize)]
pub struct PickResult {
    pub player_id: Uuid,
    pub captain: bool,
    pub multiplier: i64,
    pub base_points: ResultPoints,
    pub points: ResultPoints,
}
#[derive(Debug, Clone, Serialize)]
pub struct Lineup {
    pub picks: Vec<Uuid>,
    pub captain: Uuid,
    pub origin: String,
    pub source_round: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ManagerResult {
    pub user_id: Uuid,
    pub display_name: String,
    pub points: ResultPoints,
    pub rank: Option<usize>,
    pub selection_state: String,
    pub lineup: Option<Lineup>,
    pub contributions: Vec<PickResult>,
}
#[derive(Debug, Clone, Serialize)]
pub struct RoundResult {
    pub tournament_id: Uuid,
    pub revision: String,
    pub round: RoundSummary,
    pub golfers: Vec<GolferResult>,
    pub managers: Vec<ManagerResult>,
    #[serde(skip)]
    pub finalized: bool,
}
#[derive(Debug, Serialize)]
pub struct GolferBreakdown {
    pub tournament_id: Uuid,
    pub revision: String,
    pub standing: Standing,
    pub rounds: Vec<GolferRoundBreakdown>,
    #[serde(skip)]
    pub finalized: bool,
}
#[derive(Debug, Serialize)]
pub struct GolferRoundBreakdown {
    pub round: RoundSummary,
    pub result: GolferResult,
}
#[derive(Debug, Serialize)]
pub struct ManagerBreakdown {
    pub tournament_id: Uuid,
    pub revision: String,
    pub standing: Standing,
    pub rounds: Vec<ManagerRoundBreakdown>,
    #[serde(skip)]
    pub finalized: bool,
}
#[derive(Debug, Serialize)]
pub struct ManagerRoundBreakdown {
    pub round: RoundSummary,
    pub result: ManagerResult,
}

impl From<crate::domain::fantasy::projection::Contribution> for ResultPoints {
    fn from(value: crate::domain::fantasy::projection::Contribution) -> Self {
        use crate::domain::fantasy::projection::Contribution;
        match value {
            Contribution::NotStarted => Self::NotStarted,
            Contribution::NotParticipating => Self::NotParticipating,
            Contribution::Played(p) => p.into(),
        }
    }
}
