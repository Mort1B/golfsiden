use super::{GolferRound, OwnerFacts, Settlement, hole};
use crate::domain::{
    fantasy::{
        FantasyError, GolferId, Points,
        holes::HolePoints,
        rounds::{self, Finish, NetRanking, NonMatchOwner, Owner},
    },
    models::ScoringFormat,
    scoring::scramble_playing_handicap,
};
fn invalid<T>(_: T) -> FantasyError {
    FantasyError::InvalidFacts
}
struct Prepared {
    owner: Owner,
    holes: Vec<super::HoleDetail>,
    hole_points: Vec<HolePoints>,
    ranking: Option<NetRanking>,
    finish: Finish,
    settlement: Settlement,
}
fn prepare(input: &OwnerFacts, hidden: bool) -> Result<Prepared, FantasyError> {
    let f = &input.facts;
    let team = matches!(
        f.round.scoring_format,
        ScoringFormat::TeamScramble
            | ScoringFormat::TwoPlayerFoursomes
            | ScoringFormat::FourBallStrokePlay
    );
    let owner = if team {
        let [a, b] = f.players.as_slice() else {
            return Err(FantasyError::InvalidFacts);
        };
        Owner::Team {
            team_id: f.owner_id,
            partners: [GolferId(*a), GolferId(*b)],
        }
    } else {
        Owner::Individual(GolferId(f.owner_id))
    };
    let handicap = if !f.round.handicap_enabled {
        0
    } else {
        match f.round.scoring_format {
            ScoringFormat::TeamScramble => scramble_playing_handicap(
                &f.snapshots
                    .iter()
                    .map(|s| i32::from(s.course_handicap))
                    .collect::<Vec<_>>(),
                f.round.handicap_allowance_percent,
            )
            .map_err(invalid)?,
            ScoringFormat::TwoPlayerFoursomes => i32::from(
                f.team_snapshot
                    .as_ref()
                    .ok_or(FantasyError::InvalidFacts)?
                    .playing_handicap,
            ),
            ScoringFormat::FourBallStrokePlay => 0,
            _ => i32::from(
                f.snapshots
                    .iter()
                    .find(|s| s.player_id == f.owner_id)
                    .ok_or(FantasyError::InvalidFacts)?
                    .playing_handicap,
            ),
        }
    };
    if f.holes.len() != usize::try_from(f.round.number_of_holes).map_err(invalid)? {
        return Err(FantasyError::InvalidFacts);
    }
    let evaluated = f
        .holes
        .iter()
        .map(|h| hole::evaluate(f, h, handicap, hidden && h.hole_number > 9))
        .collect::<Result<Vec<_>, _>>()?;
    let complete = evaluated
        .iter()
        .all(|h| matches!(h.detail.points, HolePoints::Recorded(_)) && h.ranking.is_some());
    let mut total = 0_i64;
    for h in &evaluated {
        if let Some(rank) = h.ranking {
            total = total.checked_add(rank).ok_or(FantasyError::Overflow)?;
        }
    }
    let ranking = complete.then_some(
        if f.round.scoring_format == ScoringFormat::IndividualStableford {
            NetRanking::Stableford(total)
        } else {
            NetRanking::Strokes(total)
        },
    );
    let (finish, settlement) = if hidden {
        (Finish::Playing, Settlement::Withheld)
    } else if complete && !f.confirmation.is_empty() {
        (Finish::Complete { confirmed: true }, Settlement::Confirmed)
    } else if let Some(attested) = input.non_finish {
        (
            Finish::NonFinish { attested },
            if attested == input.source {
                Settlement::NonFinish
            } else {
                Settlement::StaleNonFinish
            },
        )
    } else if complete {
        (
            Finish::Complete { confirmed: false },
            Settlement::Unconfirmed,
        )
    } else {
        (Finish::Playing, Settlement::Playing)
    };
    let holes: Vec<_> = evaluated.into_iter().map(|h| h.detail).collect();
    let hole_points = holes.iter().map(|h| h.points).collect();
    Ok(Prepared {
        owner,
        holes,
        hole_points,
        ranking,
        finish,
        settlement,
    })
}
pub fn project_non_match(
    inputs: &[OwnerFacts],
    expected_holes: usize,
    hidden: bool,
) -> Result<Vec<GolferRound>, FantasyError> {
    let prepared = inputs
        .iter()
        .map(|f| prepare(f, hidden))
        .collect::<Result<Vec<_>, _>>()?;
    let owners: Vec<_> = prepared
        .iter()
        .zip(inputs)
        .map(|(p, f)| NonMatchOwner {
            owner: p.owner,
            holes: &p.hole_points,
            source: f.source,
            finish: p.finish,
            ranking: p.ranking,
        })
        .collect();
    let scores = rounds::score_non_match_round(owners.len(), expected_holes, &owners)?;
    let mut results = vec![];
    for (p, input) in prepared.iter().zip(inputs) {
        let mut recorded = 0_i64;
        for hole in &p.hole_points {
            if let HolePoints::Recorded(c) = hole {
                recorded = recorded
                    .checked_add(c.points())
                    .ok_or(FantasyError::Overflow)?;
            }
        }
        for player in &input.facts.players {
            let points = scores
                .iter()
                .find(|r| r.golfer == GolferId(*player))
                .ok_or(FantasyError::InvalidFacts)?
                .points;
            let placement_points = if points == Points::Withheld {
                None
            } else {
                points.total().map(|total| total - recorded)
            };
            results.push(GolferRound {
                golfer: GolferId(*player),
                points,
                team_id: match p.owner {
                    Owner::Team { team_id, .. } => Some(team_id),
                    _ => None,
                },
                holes: p.holes.clone(),
                placement_points,
                settlement: p.settlement,
                match_outcome: None,
            });
        }
    }
    Ok(results)
}
