use super::super::Error;
use super::{
    load::{Loaded, Round},
    models::*,
};
use crate::domain::{
    fantasy::{
        self, GolferId, ManagerId, Points, RoundId,
        holes::HolePoints,
        projection::{self, Contribution, GolferRound},
        rounds::GolferPoints,
        selections::{self, Selection},
        totals,
    },
    models::{RoundStatus, ScoringFormat},
    score_visibility::VisibilityMode,
};
use std::collections::BTreeMap;
use uuid::Uuid;
pub(super) struct Projected {
    pub result: RoundResult,
    pub golfers: BTreeMap<Uuid, Contribution>,
    pub managers: BTreeMap<Uuid, Contribution>,
}
fn invalid(_: fantasy::FantasyError) -> Error {
    Error::Invalid
}
pub(super) fn project(data: &Loaded, r: &Round, t: Uuid, caller: Uuid) -> Result<Projected, Error> {
    let visibility = *data.visibility.get(&r.id).ok_or(Error::Invalid)?;
    let hidden = visibility.mode == VisibilityMode::FrontNine;
    let started = r.status != RoundStatus::Draft;
    let owners = data.owners.get(&r.id).map(Vec::as_slice).unwrap_or(&[]);
    let projected = if !started {
        vec![]
    } else if r.scoring_format == ScoringFormat::SinglesMatchPlay {
        owners
            .iter()
            .map(|f| projection::project_match(f, hidden))
            .collect::<Result<Vec<_>, _>>()
            .map_err(invalid)?
    } else {
        projection::project_non_match(
            owners,
            usize::try_from(r.number_of_holes).map_err(|_| Error::Invalid)?,
            hidden,
        )
        .map_err(invalid)?
    };
    let projected: BTreeMap<_, _> = projected.into_iter().map(|g| (g.golfer.0, g)).collect();
    let golfers: BTreeMap<_, _> = data
        .golfers
        .iter()
        .map(|g| {
            (
                g.id,
                if !started {
                    Contribution::NotStarted
                } else {
                    projected
                        .get(&g.id)
                        .map_or(Contribution::NotParticipating, |g| {
                            Contribution::Played(g.points)
                        })
                },
            )
        })
        .collect();
    let rank_input: Vec<_> = golfers
        .iter()
        .map(|(id, value)| (GolferId(*id), vec![*value]))
        .collect();
    let ranks = projection::rank_contributions(&[RoundId(r.id)], &rank_input).map_err(invalid)?;
    let mut golfer_rows = vec![];
    for golfer in &data.golfers {
        let contribution = *golfers.get(&golfer.id).ok_or(Error::Invalid)?;
        let details = projected.get(&golfer.id);
        let (holes, recorded) = hole_results(details)?;
        golfer_rows.push(GolferResult {
            player_id: golfer.id,
            display_name: golfer.display_name.clone(),
            points: contribution.into(),
            rank: ranks.get(&GolferId(golfer.id)).and_then(|(_, rank)| *rank),
            team_id: details.and_then(|d| d.team_id),
            holes,
            recorded_hole_points: recorded,
            placement_points: details.and_then(|d| d.placement_points),
            settlement: details.map(|d| d.settlement),
            match_outcome: details.and_then(|d| d.match_outcome),
        });
    }
    let scores: Vec<_> = projected
        .values()
        .map(|g| GolferPoints {
            golfer: g.golfer,
            points: g.points,
        })
        .collect();
    let (mut manager_rows, managers) = managers(data, r, caller, &scores, &golfers)?;
    let rank_input: Vec<_> = managers
        .iter()
        .map(|(id, value)| (ManagerId(*id), vec![*value]))
        .collect();
    let ranks = projection::rank_contributions(&[RoundId(r.id)], &rank_input).map_err(invalid)?;
    for row in &mut manager_rows {
        row.rank = ranks
            .get(&ManagerId(row.user_id))
            .and_then(|(_, rank)| *rank);
    }
    golfer_rows.sort_by_key(|g| (g.rank.unwrap_or(usize::MAX), g.player_id));
    manager_rows.sort_by_key(|m| (m.rank.unwrap_or(usize::MAX), m.user_id));
    let round_points = if !started {
        Points::NotStarted
    } else if hidden {
        Points::Withheld
    } else if scores.is_empty() {
        Points::Pending { recorded: 0 }
    } else {
        totals::sum_points(&scores.iter().map(|s| s.points).collect::<Vec<_>>()).map_err(invalid)?
    };
    let summary = RoundSummary {
        round_id: r.id,
        round_number: r.round_number,
        name: r.name.clone(),
        format: r.scoring_format,
        sporting_status: (!hidden).then_some(r.status),
        visibility,
        points: round_points.into(),
    };
    Ok(Projected {
        result: RoundResult {
            tournament_id: t,
            revision: String::new(),
            round: summary,
            golfers: golfer_rows,
            managers: manager_rows,
            finalized: false,
        },
        golfers,
        managers,
    })
}
fn hole_results(details: Option<&GolferRound>) -> Result<(Vec<HoleResult>, Option<i64>), Error> {
    let Some(details) = details else {
        return Ok((vec![], None));
    };
    let mut recorded = 0_i64;
    let mut holes = vec![];
    for hole in &details.holes {
        let points = match hole.points {
            HolePoints::Withheld => ResultPoints::Withheld,
            HolePoints::Pending if details.settlement == projection::Settlement::NonFinish => {
                ResultPoints::OmittedNonFinish
            }
            HolePoints::Pending => ResultPoints::Pending { recorded: 0 },
            HolePoints::Recorded(c) => {
                recorded = recorded.checked_add(c.points()).ok_or(Error::Invalid)?;
                ResultPoints::Settled { total: c.points() }
            }
        };
        holes.push(HoleResult {
            hole_id: hole.id,
            hole_number: hole.number,
            par: hole.par,
            category: hole.category(),
            net_strokes: hole.net_strokes,
            points,
        });
    }
    // This is the permitted recorded-hole subtotal, never full hidden placement.
    Ok((holes, (!details.holes.is_empty()).then_some(recorded)))
}
fn managers(
    data: &Loaded,
    r: &Round,
    caller: Uuid,
    scores: &[GolferPoints],
    golfers: &BTreeMap<Uuid, Contribution>,
) -> Result<(Vec<ManagerResult>, BTreeMap<Uuid, Contribution>), Error> {
    let mut rows = vec![];
    let mut points = BTreeMap::new();
    for manager in &data.managers {
        let saved = data.selections.get(&(r.id, manager.id));
        let visible = r.locked_at.is_some() || manager.id == caller;
        let lineup = if visible {
            saved
                .filter(|s| s.picks.is_some())
                .map(|s| {
                    Ok::<Lineup, Error>(Lineup {
                        picks: s.picks.clone().ok_or(Error::Invalid)?,
                        captain: s.captain.ok_or(Error::Invalid)?,
                        origin: s.origin.clone().ok_or(Error::Invalid)?,
                        source_round: s.source_round,
                    })
                })
                .transpose()?
        } else {
            None
        };
        let mut contributions = vec![];
        let (selection_state, contribution) = if r.status == RoundStatus::Draft {
            (
                if r.locked_at.is_none() {
                    "unlocked".into()
                } else {
                    saved.map_or("not_participating".into(), |s| s.state.clone())
                },
                Contribution::NotStarted,
            )
        } else {
            match saved.map(|s| s.state.as_str()) {
                Some("locked") => {
                    let lineup = lineup.as_ref().ok_or(Error::Invalid)?;
                    let chosen = selections::Lineup::new(
                        &lineup
                            .picks
                            .iter()
                            .copied()
                            .map(GolferId)
                            .collect::<Vec<_>>(),
                        GolferId(lineup.captain),
                    )
                    .map_err(invalid)?;
                    let origin = match lineup.source_round {
                        Some(source) => selections::Origin::CarriedForward {
                            source_round: RoundId(source),
                        },
                        None => selections::Origin::Submitted,
                    };
                    // A selected golfer absent from the actual frozen round field is known
                    // nonparticipation and contributes zero, not an invented missing scorecard.
                    let complete_scores: Vec<_> = golfers
                        .iter()
                        .map(|(id, c)| GolferPoints {
                            golfer: GolferId(*id),
                            points: c.domain(),
                        })
                        .collect();
                    let value = totals::manager_round(
                        RoundId(r.id),
                        &Selection::Locked {
                            round: RoundId(r.id),
                            lineup: chosen,
                            origin,
                        },
                        &complete_scores,
                    )
                    .map_err(invalid)?
                    .ok_or(Error::Invalid)?;
                    for player in &lineup.picks {
                        let base = *golfers.get(player).ok_or(Error::Invalid)?;
                        let multiplier = if *player == lineup.captain { 2 } else { 1 };
                        contributions.push(PickResult {
                            player_id: *player,
                            captain: *player == lineup.captain,
                            multiplier,
                            base_points: base.into(),
                            points: match base {
                                Contribution::NotParticipating => ResultPoints::NotParticipating,
                                _ => base.domain().multiply(multiplier).map_err(invalid)?.into(),
                            },
                        });
                    }
                    ("locked".into(), Contribution::Played(value))
                }
                Some("missed" | "invalid") => {
                    let selection = if saved.is_some_and(|s| s.state == "missed") {
                        Selection::Missed
                    } else {
                        Selection::Invalid
                    };
                    let value = totals::manager_round(RoundId(r.id), &selection, scores)
                        .map_err(invalid)?
                        .ok_or(Error::Invalid)?;
                    (
                        saved.ok_or(Error::Invalid)?.state.clone(),
                        Contribution::Played(value),
                    )
                }
                Some("not_participating") => {
                    ("not_participating".into(), Contribution::NotParticipating)
                }
                None if r.locked_at.is_some_and(|at| manager.entered_at > at) => {
                    ("not_participating".into(), Contribution::NotParticipating)
                }
                _ => (
                    "pending".into(),
                    Contribution::Played(Points::Pending { recorded: 0 }),
                ),
            }
        };
        points.insert(manager.id, contribution);
        rows.push(ManagerResult {
            user_id: manager.id,
            display_name: manager.display_name.clone(),
            points: contribution.into(),
            rank: None,
            selection_state,
            lineup,
            contributions,
        });
    }
    Ok((rows, points))
}
