use super::{FantasyError, GolferId, Points, SourceToken, holes::HolePoints};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Individual(GolferId),
    Team {
        team_id: Uuid,
        partners: [GolferId; 2],
    },
}
impl Owner {
    fn golfers(self) -> Vec<GolferId> {
        match self {
            Self::Individual(id) => vec![id],
            Self::Team { partners, .. } => partners.to_vec(),
        }
    }
}

/// Already derived by the preserved format adapter. Stableford stays in native
/// points, never converted to strokes or used to infer hole categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetRanking {
    Strokes(i64),
    Stableford(i64),
}
impl NetRanking {
    fn compare(self, other: Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Strokes(a), Self::Strokes(b)) => a.cmp(&b),
            (Self::Stableford(a), Self::Stableford(b)) => b.cmp(&a),
            // Mixed units are rejected before sorting.
            (Self::Strokes(_), Self::Stableford(_)) => std::cmp::Ordering::Less,
            (Self::Stableford(_), Self::Strokes(_)) => std::cmp::Ordering::Greater,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    Playing,
    Complete {
        confirmed: bool,
    },
    /// Only valid when its attested token matches the complete current source.
    NonFinish {
        attested: SourceToken,
    },
}

pub struct NonMatchOwner<'a> {
    pub owner: Owner,
    /// One entry per configured hole, including explicit pending/withheld entries.
    pub holes: &'a [HolePoints],
    pub source: SourceToken,
    pub finish: Finish,
    pub ranking: Option<NetRanking>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GolferPoints {
    pub golfer: GolferId,
    pub points: Points,
}

pub fn placement_points(position: usize) -> Result<i64, FantasyError> {
    match position {
        0 => Err(FantasyError::InvalidFacts),
        1 => Ok(10),
        2 => Ok(8),
        3 => Ok(6),
        4 => Ok(5),
        5 => Ok(4),
        6 => Ok(3),
        7 => Ok(2),
        8 => Ok(1),
        _ => Ok(0),
    }
}

/// Rank complete finishing owners once, then attribute the identical award to
/// each preserved partner. All owners of the round must be supplied by adapter.
pub fn score_non_match_round(
    expected_owners: usize,
    expected_holes: usize,
    owners: &[NonMatchOwner<'_>],
) -> Result<Vec<GolferPoints>, FantasyError> {
    if expected_holes == 0 || expected_owners != owners.len() {
        return Err(FantasyError::InvalidFacts);
    }
    let mut golfers = BTreeSet::new();
    let mut teams = BTreeSet::new();
    for owner in owners {
        if owner.holes.len() != expected_holes {
            return Err(FantasyError::InvalidFacts);
        }
        if let Owner::Team { team_id, .. } = owner.owner
            && !teams.insert(team_id)
        {
            return Err(FantasyError::DuplicateIdentity);
        }
        for golfer in owner.owner.golfers() {
            if !golfers.insert(golfer) {
                return Err(FantasyError::DuplicateIdentity);
            }
        }
    }
    // Concealed facts cannot influence ranks, totals, finality or error details.
    if owners
        .iter()
        .any(|owner| owner.holes.contains(&HolePoints::Withheld))
    {
        return Ok(golfers
            .into_iter()
            .map(|golfer| GolferPoints {
                golfer,
                points: Points::Withheld,
            })
            .collect());
    }
    let mut field: Vec<(usize, NetRanking)> = Vec::new();
    let mut all_settled = true;
    let mut recorded = Vec::new();
    for (index, owner) in owners.iter().enumerate() {
        let mut points = 0_i64;
        let mut missing = false;
        for hole in owner.holes {
            match hole {
                HolePoints::Recorded(category) => {
                    points = points
                        .checked_add(category.points())
                        .ok_or(FantasyError::Overflow)?
                }
                HolePoints::Pending => missing = true,
                HolePoints::Withheld => return Err(FantasyError::InvalidFacts),
            }
        }
        recorded.push(points);
        match owner.finish {
            Finish::Complete { confirmed } => {
                if missing {
                    return Err(FantasyError::InvalidFacts);
                }
                let rank = owner.ranking.ok_or(FantasyError::InvalidFacts)?;
                if matches!(rank, NetRanking::Stableford(value) if value < 0) {
                    return Err(FantasyError::InvalidFacts);
                }
                if field.first().is_some_and(|(_, prior)| {
                    std::mem::discriminant(prior) != std::mem::discriminant(&rank)
                }) {
                    return Err(FantasyError::InvalidFacts);
                }
                field.push((index, rank));
                all_settled &= confirmed;
            }
            Finish::Playing => all_settled = false,
            Finish::NonFinish { attested } => all_settled &= attested == owner.source,
        }
    }
    field.sort_by(|(a_index, a), (b_index, b)| a.compare(*b).then(a_index.cmp(b_index)));
    let mut placements = vec![0_i64; owners.len()];
    let mut previous = None;
    let mut position = 0;
    for (index, (owner_index, ranking)) in field.into_iter().enumerate() {
        if previous != Some(ranking) {
            position = index + 1;
        }
        previous = Some(ranking);
        let target = placements
            .get_mut(owner_index)
            .ok_or(FantasyError::InvalidFacts)?;
        *target = placement_points(position)?;
    }
    let mut result = Vec::new();
    for ((owner, recorded), placement) in owners.iter().zip(recorded).zip(placements) {
        let points = match owner.finish {
            Finish::NonFinish { attested } if attested == owner.source => Points::Settled(recorded),
            Finish::NonFinish { .. } | Finish::Playing => Points::Pending { recorded },
            Finish::Complete { .. } => {
                let total = recorded
                    .checked_add(placement)
                    .ok_or(FantasyError::Overflow)?;
                if all_settled {
                    Points::Settled(total)
                } else {
                    Points::Provisional(total)
                }
            }
        };
        result.extend(
            owner
                .owner
                .golfers()
                .into_iter()
                .map(|golfer| GolferPoints { golfer, points }),
        );
    }
    result.sort_by_key(|entry| entry.golfer);
    Ok(result)
}
