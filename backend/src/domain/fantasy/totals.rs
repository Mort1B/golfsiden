use super::{
    FantasyError, GolferId, ManagerId, Points, RoundId, rounds::GolferPoints, selections::Selection,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn sum_points(values: &[Points]) -> Result<Points, FantasyError> {
    if values.contains(&Points::Withheld) {
        return Ok(Points::Withheld);
    }
    let mut total = 0_i64;
    let mut pending = false;
    let mut provisional = false;
    for value in values {
        let points = match *value {
            Points::NotStarted => {
                provisional = true;
                0
            }
            Points::Pending { recorded } => {
                pending = true;
                recorded
            }
            Points::Provisional(points) => {
                provisional = true;
                points
            }
            Points::Settled(points) => points,
            Points::Withheld => return Ok(Points::Withheld),
        };
        total = total.checked_add(points).ok_or(FantasyError::Overflow)?;
    }
    Ok(if pending {
        Points::Pending { recorded: total }
    } else if provisional {
        Points::Provisional(total)
    } else {
        Points::Settled(total)
    })
}

/// A missed/invalid entry is an explicit zero; an absent result for a selected
/// golfer remains pending. No manager multiplier modifies the underlying golfer.
pub fn manager_round(
    round: RoundId,
    selection: &Selection,
    golfers: &[GolferPoints],
) -> Result<Option<Points>, FantasyError> {
    let Selection::Locked {
        round: selected_round,
        lineup,
        ..
    } = selection
    else {
        return Ok(match selection {
            Selection::NotParticipating => None,
            Selection::Missed | Selection::Invalid => Some(Points::Settled(0)),
            Selection::Locked { .. } => return Err(FantasyError::InvalidFacts),
        });
    };
    if round != *selected_round {
        return Err(FantasyError::InvalidFacts);
    }
    let mut by_golfer = BTreeMap::new();
    for golfer in golfers {
        if by_golfer.insert(golfer.golfer, golfer.points).is_some() {
            return Err(FantasyError::DuplicateIdentity);
        }
    }
    // Check privacy first, so even an overflow in another contribution cannot
    // make the observable result depend on otherwise concealed values.
    if lineup
        .picks()
        .iter()
        .any(|id| by_golfer.get(id) == Some(&Points::Withheld))
    {
        return Ok(Some(Points::Withheld));
    }
    let mut contributions = Vec::new();
    for id in lineup.picks() {
        let points = by_golfer
            .get(id)
            .copied()
            .unwrap_or(Points::Pending { recorded: 0 });
        contributions.push(points.multiply(if *id == lineup.captain() { 2 } else { 1 })?);
    }
    Ok(Some(sum_points(&contributions)?))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoundContribution {
    pub round: RoundId,
    pub points: Points,
}

/// Expected round IDs are the complete tournament inventory, not best-N rounds.
/// Missing contributions explicitly make the result pending, never settled zero.
pub fn all_round_total(
    expected: &[RoundId],
    contributions: &[RoundContribution],
) -> Result<Points, FantasyError> {
    let expected_set: BTreeSet<_> = expected.iter().copied().collect();
    if expected_set.len() != expected.len() {
        return Err(FantasyError::DuplicateIdentity);
    }
    let mut actual = BTreeMap::new();
    for contribution in contributions {
        if !expected_set.contains(&contribution.round) {
            return Err(FantasyError::InvalidFacts);
        }
        if actual
            .insert(contribution.round, contribution.points)
            .is_some()
        {
            return Err(FantasyError::DuplicateIdentity);
        }
    }
    let values: Vec<_> = expected
        .iter()
        .map(|id| {
            actual
                .get(id)
                .copied()
                .unwrap_or(Points::Pending { recorded: 0 })
        })
        .collect();
    sum_points(&values)
}

pub struct SeasonEntry<'a, Id> {
    pub id: Id,
    pub rounds: &'a [RoundContribution],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Standing<Id> {
    pub id: Id,
    pub points: Points,
    pub rank: Option<usize>,
}

pub(crate) fn standings<Id: Copy + Ord>(
    expected_rounds: &[RoundId],
    roster: &[Id],
    entries: &[SeasonEntry<'_, Id>],
) -> Result<Vec<Standing<Id>>, FantasyError> {
    let roster_set: BTreeSet<_> = roster.iter().copied().collect();
    if roster_set.len() != roster.len() {
        return Err(FantasyError::DuplicateIdentity);
    }
    let mut results = BTreeMap::new();
    for entry in entries {
        if !roster_set.contains(&entry.id) {
            return Err(FantasyError::InvalidFacts);
        }
        if results
            .insert(entry.id, all_round_total(expected_rounds, entry.rounds)?)
            .is_some()
        {
            return Err(FantasyError::DuplicateIdentity);
        }
    }
    let mut board = Vec::new();
    for id in roster {
        let points = match results.get(id) {
            Some(points) => *points,
            None => all_round_total(expected_rounds, &[])?,
        };
        board.push(Standing {
            id: *id,
            points,
            rank: None,
        });
    }
    // Hidden entrants cannot change the visible rank/ordering of other entrants.
    // Conservatively retain identity order and suppress all ranks for this board.
    if board.iter().any(|entry| entry.points == Points::Withheld) {
        board.sort_by_key(|entry| entry.id);
        return Ok(board);
    }
    board.sort_by(|a, b| {
        b.points
            .total()
            .cmp(&a.points.total())
            .then(a.id.cmp(&b.id))
    });
    let mut previous = None;
    let mut position = 0;
    for (index, entry) in board.iter_mut().enumerate() {
        if let Some(total) = entry.points.total() {
            if previous != Some(total) {
                position = index + 1;
            }
            previous = Some(total);
            entry.rank = Some(position);
        }
    }
    Ok(board)
}

pub fn golfer_standings(
    expected_rounds: &[RoundId],
    roster: &[GolferId],
    entries: &[SeasonEntry<'_, GolferId>],
) -> Result<Vec<Standing<GolferId>>, FantasyError> {
    standings(expected_rounds, roster, entries)
}
pub fn manager_standings(
    expected_rounds: &[RoundId],
    roster: &[ManagerId],
    entries: &[SeasonEntry<'_, ManagerId>],
) -> Result<Vec<Standing<ManagerId>>, FantasyError> {
    standings(expected_rounds, roster, entries)
}
