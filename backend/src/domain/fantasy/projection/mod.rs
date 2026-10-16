//! Pure adapters over preserved typed facts. Visibility is supplied before any
//! hidden source influences arithmetic, dispositions, match outcomes or metadata.
mod facts;
mod hole;
mod matches;
mod non_match;
mod standings;
use super::{
    GolferId, Points,
    holes::{Category, HolePoints},
};
pub use facts::*;
pub use matches::project_match;
pub use non_match::project_non_match;
pub use standings::{Contribution, rank_contributions};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct HoleDetail {
    pub id: Uuid,
    pub number: i16,
    pub par: i16,
    pub points: HolePoints,
    pub net_strokes: Option<i32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Settlement {
    Playing,
    Unconfirmed,
    Confirmed,
    NonFinish,
    StaleNonFinish,
    Withheld,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchOutcome {
    Win,
    Draw,
    Loss,
}
#[derive(Debug, Clone)]
pub struct GolferRound {
    pub golfer: GolferId,
    pub points: Points,
    pub team_id: Option<Uuid>,
    pub holes: Vec<HoleDetail>,
    pub placement_points: Option<i64>,
    pub settlement: Settlement,
    pub match_outcome: Option<MatchOutcome>,
}
impl HoleDetail {
    pub fn category(&self) -> Option<Category> {
        match self.points {
            HolePoints::Recorded(c) => Some(c),
            _ => None,
        }
    }
}

/// Identify missing partner inputs only on holes the canonical Fantasy evaluator
/// still marks pending. Explicit no-score is resolved input, never an absent row.
pub fn unresolved_four_ball_inputs(
    facts: &Facts,
) -> Result<Vec<(Uuid, Vec<i16>)>, super::FantasyError> {
    if facts.round.scoring_format != crate::domain::models::ScoringFormat::FourBallStrokePlay {
        return Ok(Vec::new());
    }
    let mut missing = std::collections::BTreeMap::<Uuid, Vec<i16>>::new();
    for h in &facts.holes {
        if matches!(
            hole::evaluate(facts, h, 0, false)?.detail.points,
            super::holes::HolePoints::Pending
        ) {
            for player in &facts.players {
                if !facts
                    .four_ball
                    .iter()
                    .any(|input| input.player_id == *player && input.hole_id == h.id)
                {
                    missing.entry(*player).or_default().push(h.hole_number);
                }
            }
        }
    }
    Ok(missing.into_iter().collect())
}
