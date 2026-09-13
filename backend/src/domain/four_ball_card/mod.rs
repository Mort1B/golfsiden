//! Explicit player inputs and the derived, team-owned four-ball card.
use super::{
    four_ball::{self, GrossScore, PlayerHole, PlayerHoleInput},
    score_visibility::{VisibilityMetadata, VisibilityMode},
    scorecards::{ScoreOwner, ScoreRevision},
    scoring::handicap_strokes_for_hole,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Input {
    Numeric { gross_strokes: i16 },
    NoScore {},
}
impl Input {
    pub fn domain(self) -> Result<PlayerHoleInput, super::player_score_input::InvalidGrossScore> {
        match self {
            Self::Numeric { gross_strokes } => {
                GrossScore::new(i32::from(gross_strokes)).map(PlayerHoleInput::Numeric)
            }
            Self::NoScore {} => Ok(PlayerHoleInput::NoScore),
        }
    }
    pub fn gross(self) -> Option<i16> {
        match self {
            Self::Numeric { gross_strokes } => Some(gross_strokes),
            Self::NoScore {} => None,
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub id: Uuid,
    pub revision: ScoreRevision,
    pub round_id: Uuid,
    pub hole_id: Uuid,
    pub owner: ScoreOwner,
    pub input: Input,
    pub submitted_by: Uuid,
    pub submitted_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Partner {
    pub player_id: Uuid,
    pub display_name: String,
    pub playing_handicap: i16,
}
#[derive(Debug, Clone, Serialize)]
pub struct PlayerInput<E = Entry> {
    pub player_id: Uuid,
    pub handicap_strokes: i32,
    pub score: Option<E>,
    pub net_strokes: Option<i32>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Selected {
    pub strokes: i32,
    pub player_ids: Vec<Uuid>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Hole<E = Entry> {
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
    pub players: [PlayerInput<E>; 2],
    pub gross: Option<Selected>,
    pub net: Option<Selected>,
}
#[derive(Debug, Clone, Serialize)]
pub struct Card<E = Entry> {
    pub round_id: Uuid,
    pub owner: ScoreOwner,
    pub owner_name: String,
    pub partners: [Partner; 2],
    pub holes: Vec<Hole<E>>,
    pub gross_total: Option<i32>,
    pub net_total: Option<i32>,
    pub par_played: i32,
    pub holes_scored: usize,
    pub number_of_holes: usize,
    pub visible_hole_count: usize,
    pub complete: Option<bool>,
    pub confirmed: Option<bool>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub visibility: VisibilityMetadata,
}
#[derive(sqlx::FromRow)]
pub struct HoleSource {
    pub hole_id: Uuid,
    pub hole_number: i16,
    pub par: i16,
    pub stroke_index: i16,
}

pub fn project(
    mut card: Card,
    sources: Vec<HoleSource>,
    entries: Vec<Entry>,
) -> Result<Card, super::scoring::ScoringError> {
    use super::scoring::ScoringError::InvalidHole;
    if sources.len() != 18 || card.partners[0].player_id == card.partners[1].player_id {
        return Err(InvalidHole);
    }
    let restricted = card.visibility.mode == VisibilityMode::FrontNine;
    card.visible_hole_count = if restricted { 9 } else { 18 };
    for source in sources
        .into_iter()
        .filter(|h| usize::try_from(h.hole_number).is_ok_and(|n| n <= card.visible_hole_count))
    {
        let players = card.partners.each_ref().map(|partner| {
            let score = entries
                .iter()
                .find(|entry| {
                    entry.hole_id == source.hole_id
                        && entry.owner.player_id() == Some(partner.player_id)
                })
                .cloned();
            let strokes = handicap_strokes_for_hole(
                i32::from(partner.playing_handicap),
                i32::from(source.stroke_index),
                18,
            )?;
            let net_strokes = score
                .as_ref()
                .and_then(|entry| entry.input.gross())
                .map(|gross| i32::from(gross) - strokes);
            Ok(PlayerInput {
                player_id: partner.player_id,
                handicap_strokes: strokes,
                score,
                net_strokes,
            })
        });
        let [first, second] = players;
        let players: [PlayerInput; 2] = [first?, second?];
        let inputs = players.each_ref().map(|player| {
            player
                .score
                .as_ref()
                .map(|score| score.input.domain())
                .transpose()
                .map(|input| input.unwrap_or(PlayerHoleInput::Unentered))
                .map_err(|_| InvalidHole)
        });
        let [first, second] = inputs;
        let result = four_ball::aggregate_hole(
            [
                PlayerHole {
                    player_id: card.partners[0].player_id,
                    playing_handicap: card.partners[0].playing_handicap,
                    input: first?,
                },
                PlayerHole {
                    player_id: card.partners[1].player_id,
                    playing_handicap: card.partners[1].playing_handicap,
                    input: second?,
                },
            ],
            u8::try_from(source.stroke_index).map_err(|_| InvalidHole)?,
        )
        .map_err(|_| InvalidHole)?;
        let (gross, net) = if let Some(result) = result {
            card.holes_scored += 1;
            *card.gross_total.get_or_insert(0) += result.gross.score;
            *card.net_total.get_or_insert(0) += result.net.score;
            card.par_played += i32::from(source.par);
            (
                Some(Selected {
                    strokes: result.gross.score,
                    player_ids: result.gross.player_ids,
                }),
                Some(Selected {
                    strokes: result.net.score,
                    player_ids: result.net.player_ids,
                }),
            )
        } else {
            (None, None)
        };
        card.holes.push(Hole {
            hole_id: source.hole_id,
            hole_number: source.hole_number,
            par: source.par,
            stroke_index: source.stroke_index,
            players,
            gross,
            net,
        });
    }
    card.complete = (!restricted).then_some(card.holes_scored == 18);
    if restricted {
        card.confirmed = None;
        card.confirmed_at = None;
    }
    Ok(card)
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadEntry {
    pub id: Uuid,
    pub input: Input,
}
/// Member/history reads never expose revision, actor or delivery metadata.
pub fn read_projection(card: Card) -> Card<ReadEntry> {
    Card {
        round_id: card.round_id,
        owner: card.owner,
        owner_name: card.owner_name,
        partners: card.partners,
        holes: card
            .holes
            .into_iter()
            .map(|hole| Hole {
                hole_id: hole.hole_id,
                hole_number: hole.hole_number,
                par: hole.par,
                stroke_index: hole.stroke_index,
                players: hole.players.map(|player| PlayerInput {
                    player_id: player.player_id,
                    handicap_strokes: player.handicap_strokes,
                    net_strokes: player.net_strokes,
                    score: player.score.map(|entry| ReadEntry {
                        id: entry.id,
                        input: entry.input,
                    }),
                }),
                gross: hole.gross,
                net: hole.net,
            })
            .collect(),
        gross_total: card.gross_total,
        net_total: card.net_total,
        par_played: card.par_played,
        holes_scored: card.holes_scored,
        number_of_holes: card.number_of_holes,
        visible_hole_count: card.visible_hole_count,
        complete: card.complete,
        confirmed: card.confirmed,
        confirmed_at: card.confirmed_at,
        visibility: card.visibility,
    }
}
