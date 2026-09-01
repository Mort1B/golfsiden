use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::domain::score_visibility::{VisibilityMetadata, VisibilityMode};

use super::{ScoreOwner, ScorecardSummary};

#[derive(Debug, Clone, Serialize)]
pub struct ScorecardReadScore {
    pub id: Uuid,
    pub gross_strokes: i16,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScorecardReadHole {
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
    pub score: Option<ScorecardReadScore>,
    pub net_strokes: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScorecardReadProjection {
    pub round_id: Uuid,
    pub owner: ScoreOwner,
    pub holes: Vec<ScorecardReadHole>,
    pub gross_total: i32,
    pub net_total: i32,
    pub playing_handicap: i32,
    pub holes_scored: usize,
    pub number_of_holes: usize,
    pub visible_hole_count: usize,
    pub complete: Option<bool>,
    pub confirmed: Option<bool>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub visibility: VisibilityMetadata,
}

pub fn read_projection(
    summary: ScorecardSummary,
    visibility: VisibilityMetadata,
) -> ScorecardReadProjection {
    let restricted = visibility.mode == VisibilityMode::FrontNine;
    let holes = summary
        .holes
        .iter()
        .filter(|hole| !restricted || hole.hole_number <= 9)
        .map(|hole| ScorecardReadHole {
            hole_id: hole.hole_id,
            hole_number: hole.hole_number,
            par: hole.par,
            stroke_index: hole.stroke_index,
            score: hole.score.as_ref().map(|score| ScorecardReadScore {
                id: score.id,
                gross_strokes: score.gross_strokes,
            }),
            net_strokes: hole.net_strokes,
        })
        .collect::<Vec<_>>();
    let gross_total = holes
        .iter()
        .filter_map(|hole| hole.score.as_ref())
        .map(|score| i32::from(score.gross_strokes))
        .sum();
    let net_total = holes.iter().filter_map(|hole| hole.net_strokes).sum();
    let holes_scored = holes.iter().filter(|hole| hole.score.is_some()).count();
    ScorecardReadProjection {
        round_id: summary.round_id,
        owner: summary.owner,
        visible_hole_count: holes.len(),
        holes,
        gross_total,
        net_total,
        playing_handicap: summary.playing_handicap,
        holes_scored,
        number_of_holes: summary.number_of_holes,
        complete: (!restricted).then_some(summary.complete),
        confirmed: (!restricted).then_some(summary.confirmed),
        confirmed_at: (!restricted).then_some(summary.confirmed_at).flatten(),
        visibility,
    }
}
