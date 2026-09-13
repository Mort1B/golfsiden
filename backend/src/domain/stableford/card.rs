//! Typed player card transport; point values never occupy stroke fields.
use super::{HoleLayout, PlayerHoleInput, project_card};
pub use crate::domain::four_ball_card::{Entry, HoleSource, Input, Partner, ReadEntry};
use crate::domain::{
    score_visibility::{VisibilityMetadata, VisibilityMode},
    scorecards::ScoreOwner,
    scoring::{ScoringError, handicap_strokes_for_hole},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Values {
    pub gross_points: i32,
    pub net_points: i32,
    pub gross_equivalent: i32,
    pub net_equivalent: i32,
    pub actual_gross_total: Option<i32>,
    pub actual_net_total: Option<i32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Hole<E = Entry> {
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
    pub handicap_strokes: i32,
    pub score: Option<E>,
    pub net_strokes: Option<i32>,
    pub gross_points: Option<i32>,
    pub net_points: Option<i32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Card<E = Entry> {
    pub format: crate::domain::models::ScoringFormat,
    pub round_id: Uuid,
    pub owner: ScoreOwner,
    pub owner_name: String,
    pub playing_handicap: i16,
    pub holes: Vec<Hole<E>>,
    pub values: Option<Values>,
    pub holes_scored: usize,
    pub number_of_holes: usize,
    pub visible_hole_count: usize,
    pub complete: Option<bool>,
    pub confirmed: Option<bool>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub visibility: VisibilityMetadata,
}
pub fn project(
    mut card: Card,
    sources: Vec<HoleSource>,
    entries: Vec<Entry>,
) -> Result<Card, ScoringError> {
    let invalid = || ScoringError::InvalidHole;
    let sources: [HoleSource; 18] = sources.try_into().map_err(|_| invalid())?;
    if sources.iter().map(|s| s.hole_number).ne(1..=18) {
        return Err(invalid());
    }
    let mut layout = [HoleLayout {
        par: 4,
        stroke_index: 1,
    }; 18];
    let mut inputs = [PlayerHoleInput::Unentered; 18];
    let mut visible = [true; 18];
    let restricted = card.visibility.mode == VisibilityMode::FrontNine;
    for (((source, layout), input), visible) in sources
        .iter()
        .zip(&mut layout)
        .zip(&mut inputs)
        .zip(&mut visible)
    {
        *layout = HoleLayout {
            par: u8::try_from(source.par).map_err(|_| invalid())?,
            stroke_index: u8::try_from(source.stroke_index).map_err(|_| invalid())?,
        };
        *visible = !restricted || source.hole_number <= 9;
        if *visible {
            *input = entries
                .iter()
                .find(|e| e.hole_id == source.hole_id)
                .map(|e| e.input.domain())
                .transpose()
                .map_err(|_| invalid())?
                .unwrap_or(PlayerHoleInput::Unentered);
        }
    }
    let result =
        project_card(inputs, layout, card.playing_handicap, visible).map_err(|_| invalid())?;
    for (source, result) in sources
        .into_iter()
        .zip(result.holes)
        .filter(|(s, _)| !restricted || s.hole_number <= 9)
    {
        let score = entries
            .iter()
            .find(|e| e.hole_id == source.hole_id)
            .cloned();
        card.holes.push(Hole {
            hole_id: source.hole_id,
            hole_number: source.hole_number,
            par: source.par,
            stroke_index: source.stroke_index,
            handicap_strokes: handicap_strokes_for_hole(
                i32::from(card.playing_handicap),
                i32::from(source.stroke_index),
                18,
            )?,
            score,
            net_strokes: result.and_then(|r| r.strokes.map(|s| s.net.value())),
            gross_points: result.map(|r| r.points.gross.value()),
            net_points: result.map(|r| r.points.net.value()),
        });
    }
    card.holes_scored = usize::from(result.resolved_holes);
    card.visible_hole_count = usize::from(result.visible_hole_count);
    card.complete = (!restricted).then_some(result.is_complete());
    card.values = result.totals.map(|t| Values {
        gross_points: t.points.gross.value(),
        net_points: t.points.net.value(),
        gross_equivalent: t.overall.gross.value(),
        net_equivalent: t.overall.net.value(),
        actual_gross_total: result.full_stroke_totals.map(|s| s.gross.value()),
        actual_net_total: result.full_stroke_totals.map(|s| s.net.value()),
    });
    if restricted {
        card.confirmed = None;
        card.confirmed_at = None;
    }
    Ok(card)
}
pub fn read_projection(card: Card) -> Card<ReadEntry> {
    Card {
        format: card.format,
        round_id: card.round_id,
        owner: card.owner,
        owner_name: card.owner_name,
        playing_handicap: card.playing_handicap,
        holes: card
            .holes
            .into_iter()
            .map(|h| Hole {
                hole_id: h.hole_id,
                hole_number: h.hole_number,
                par: h.par,
                stroke_index: h.stroke_index,
                handicap_strokes: h.handicap_strokes,
                score: h.score.map(|e| ReadEntry {
                    id: e.id,
                    input: e.input,
                }),
                net_strokes: h.net_strokes,
                gross_points: h.gross_points,
                net_points: h.net_points,
            })
            .collect(),
        values: card.values,
        holes_scored: card.holes_scored,
        number_of_holes: card.number_of_holes,
        visible_hole_count: card.visible_hole_count,
        complete: card.complete,
        confirmed: card.confirmed,
        confirmed_at: card.confirmed_at,
        visibility: card.visibility,
    }
}
