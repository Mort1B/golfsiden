use super::super::SourceToken;
use crate::domain::models::{RoundStatus, ScoringFormat};
use serde::Deserialize;
use uuid::Uuid;
#[derive(Debug, Deserialize)]
pub struct Round {
    pub id: Uuid,
    pub scoring_format: ScoringFormat,
    pub status: RoundStatus,
    pub number_of_holes: i16,
    pub handicap_enabled: bool,
    pub handicap_allowance_percent: i16,
}
#[derive(Debug, Deserialize)]
pub struct Hole {
    pub id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
}
#[derive(Debug, Deserialize)]
pub struct Snapshot {
    pub player_id: Uuid,
    pub course_handicap: i16,
    pub playing_handicap: i16,
}
#[derive(Debug, Deserialize)]
pub struct TeamSnapshot {
    pub playing_handicap: i16,
}
#[derive(Debug, Deserialize)]
pub struct Score {
    pub hole_id: Uuid,
    pub gross_strokes: i16,
}
#[derive(Debug, Deserialize)]
pub struct Input {
    pub player_id: Uuid,
    pub hole_id: Uuid,
    pub gross_strokes: Option<i16>,
}
#[derive(Debug, Deserialize)]
pub struct Match {
    pub first_player_id: Uuid,
    pub second_player_id: Uuid,
    pub ledger: serde_json::Value,
    pub confirmed: bool,
}
#[derive(Debug, Deserialize)]
pub struct Facts {
    pub round: Round,
    pub owner_id: Uuid,
    pub players: Vec<Uuid>,
    pub holes: Vec<Hole>,
    pub snapshots: Vec<Snapshot>,
    pub team_snapshot: Option<TeamSnapshot>,
    pub scores: Vec<Score>,
    pub four_ball: Vec<Input>,
    pub stableford: Vec<Input>,
    pub confirmation: Vec<serde_json::Value>,
    pub matches: Vec<Match>,
}
pub struct OwnerFacts {
    pub facts: Facts,
    pub source: SourceToken,
    pub non_finish: Option<SourceToken>,
}
