use super::{Facts, Hole, HoleDetail};
use crate::domain::{
    fantasy::{
        FantasyError,
        holes::{self, HoleInput, HolePoints, HoleSnapshot},
    },
    four_ball::{self, GrossScore, PlayerHole, PlayerHoleInput},
    models::ScoringFormat,
    scoring, stableford,
};
fn invalid<T>(_: T) -> FantasyError {
    FantasyError::InvalidFacts
}
fn snapshot(f: &Facts, h: &Hole, handicap: i32) -> HoleSnapshot {
    HoleSnapshot {
        par: i32::from(h.par),
        playing_handicap: handicap,
        stroke_index: i32::from(h.stroke_index),
        number_of_holes: i32::from(f.round.number_of_holes),
    }
}
fn input(gross: Option<Option<i16>>) -> Result<PlayerHoleInput, FantasyError> {
    Ok(match gross {
        None => PlayerHoleInput::Unentered,
        Some(None) => PlayerHoleInput::NoScore,
        Some(Some(g)) => PlayerHoleInput::Numeric(GrossScore::new(i32::from(g)).map_err(invalid)?),
    })
}
fn fantasy_input(value: PlayerHoleInput, snapshot: HoleSnapshot) -> HoleInput {
    match value {
        PlayerHoleInput::Unentered => HoleInput::Pending,
        PlayerHoleInput::NoScore => HoleInput::Pickup,
        PlayerHoleInput::Numeric(g) => HoleInput::Strokes {
            gross: g.value(),
            snapshot,
        },
    }
}
pub(super) struct Evaluated {
    pub detail: HoleDetail,
    pub ranking: Option<i64>,
}
pub(super) fn evaluate(
    f: &Facts,
    h: &Hole,
    handicap: i32,
    hidden: bool,
) -> Result<Evaluated, FantasyError> {
    if hidden {
        return Ok(Evaluated {
            detail: HoleDetail {
                id: h.id,
                number: h.hole_number,
                par: h.par,
                points: HolePoints::Withheld,
                net_strokes: None,
            },
            ranking: None,
        });
    }
    let (points, net, ranking) = match f.round.scoring_format {
        ScoringFormat::FourBallStrokePlay => {
            let [a, b] = f.players.as_slice() else {
                return Err(FantasyError::InvalidFacts);
            };
            let partner = |p| -> Result<PlayerHole, FantasyError> {
                let handicap = if f.round.handicap_enabled {
                    f.snapshots
                        .iter()
                        .find(|s| s.player_id == p)
                        .ok_or(FantasyError::InvalidFacts)?
                        .playing_handicap
                } else {
                    0
                };
                Ok(PlayerHole {
                    player_id: p,
                    playing_handicap: handicap,
                    input: input(
                        f.four_ball
                            .iter()
                            .find(|s| s.player_id == p && s.hole_id == h.id)
                            .map(|s| s.gross_strokes),
                    )?,
                })
            };
            let partners = [partner(*a)?, partner(*b)?];
            let points =
                holes::score_four_ball(partners.map(|p| {
                    fantasy_input(p.input, snapshot(f, h, i32::from(p.playing_handicap)))
                }))?;
            // A pending partner may change the counting result; do not expose a numeric
            // Fantasy side score until both inputs resolve.
            let side = if matches!(points, HolePoints::Pending) {
                None
            } else {
                four_ball::aggregate_hole(partners, u8::try_from(h.stroke_index).map_err(invalid)?)
                    .map_err(invalid)?
            };
            let net = side.map(|s| s.net.score);
            (points, net, net.map(i64::from))
        }
        ScoringFormat::IndividualStableford => {
            let value = input(
                f.stableford
                    .iter()
                    .find(|s| s.hole_id == h.id)
                    .map(|s| s.gross_strokes),
            )?;
            let native = stableford::calculate_hole(
                value,
                stableford::HoleLayout {
                    par: u8::try_from(h.par).map_err(invalid)?,
                    stroke_index: u8::try_from(h.stroke_index).map_err(invalid)?,
                },
                i16::try_from(handicap).map_err(invalid)?,
            )
            .map_err(invalid)?;
            let net = native.and_then(|n| n.strokes.map(|s| s.net.value()));
            (
                holes::score_hole(fantasy_input(value, snapshot(f, h, handicap)))?,
                net,
                native.map(|n| i64::from(n.points.net.value())),
            )
        }
        ScoringFormat::IndividualStrokePlay
        | ScoringFormat::TeamScramble
        | ScoringFormat::TwoPlayerFoursomes => {
            let gross = f
                .scores
                .iter()
                .find(|s| s.hole_id == h.id)
                .map(|s| i32::from(s.gross_strokes));
            let points =
                holes::score_hole(
                    gross.map_or(HoleInput::Pending, |gross| HoleInput::Strokes {
                        gross,
                        snapshot: snapshot(f, h, handicap),
                    }),
                )?;
            let net = gross
                .map(|g| {
                    scoring::hole_net_score(
                        g,
                        handicap,
                        i32::from(h.stroke_index),
                        i32::from(f.round.number_of_holes),
                    )
                })
                .transpose()
                .map_err(invalid)?;
            (points, net, net.map(i64::from))
        }
        ScoringFormat::SinglesMatchPlay => return Err(FantasyError::InvalidFacts),
    };
    Ok(Evaluated {
        detail: HoleDetail {
            id: h.id,
            number: h.hole_number,
            par: h.par,
            points,
            net_strokes: net,
        },
        ranking,
    })
}
