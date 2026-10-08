use super::{FantasyError, GolferId, RoundId};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lineup {
    picks: [GolferId; 4],
    captain: GolferId,
}
impl Lineup {
    pub fn new(picks: &[GolferId], captain: GolferId) -> Result<Self, FantasyError> {
        let picks: [GolferId; 4] = picks.try_into().map_err(|_| FantasyError::InvalidLineup)?;
        if picks.iter().collect::<BTreeSet<_>>().len() != 4 || !picks.contains(&captain) {
            return Err(FantasyError::InvalidLineup);
        }
        Ok(Self { picks, captain })
    }
    pub fn picks(&self) -> &[GolferId; 4] {
        &self.picks
    }
    pub fn captain(&self) -> GolferId {
        self.captain
    }
    fn eligible(&self, eligibility: &BTreeSet<GolferId>) -> bool {
        self.picks.iter().all(|id| eligibility.contains(id))
    }
}

/// The adapter must supply only complete lineups already locked at the target
/// deadline, scoped to this manager/tournament. This is not a wall-clock check.
#[derive(Debug, Clone)]
pub struct EarlierLockedLineup {
    pub round: RoundId,
    pub round_order: u32,
    pub lineup: Lineup,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Submitted,
    CarriedForward { source_round: RoundId },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Locked {
        round: RoundId,
        lineup: Lineup,
        origin: Origin,
    },
    Missed,
    Invalid,
    /// No current Fantasy entry/membership; never enroll implicitly.
    NotParticipating,
}

pub struct DeadlineSelection<'a> {
    pub round: RoundId,
    pub round_order: u32,
    pub participating_at_deadline: bool,
    pub eligible_at_deadline: &'a BTreeSet<GolferId>,
    /// Last accepted submission only, never unsaved or rejected edits.
    pub accepted_current: Option<&'a Lineup>,
    pub earlier_locked_at_deadline: &'a [EarlierLockedLineup],
}

pub fn resolve_selection(input: DeadlineSelection<'_>) -> Result<Selection, FantasyError> {
    if !input.participating_at_deadline {
        return Ok(Selection::NotParticipating);
    }
    let mut orders = BTreeSet::new();
    let mut rounds = BTreeSet::new();
    for previous in input.earlier_locked_at_deadline {
        if previous.round_order >= input.round_order || previous.round == input.round {
            return Err(FantasyError::InvalidFacts);
        }
        if !orders.insert(previous.round_order) || !rounds.insert(previous.round) {
            return Err(FantasyError::DuplicateIdentity);
        }
    }
    if let Some(lineup) = input
        .accepted_current
        .filter(|lineup| lineup.eligible(input.eligible_at_deadline))
    {
        return Ok(Selection::Locked {
            round: input.round,
            lineup: lineup.clone(),
            origin: Origin::Submitted,
        });
    }
    let previous = input
        .earlier_locked_at_deadline
        .iter()
        .max_by_key(|prior| prior.round_order);
    if let Some(previous) = previous {
        if previous.lineup.eligible(input.eligible_at_deadline) {
            return Ok(Selection::Locked {
                round: input.round,
                lineup: previous.lineup.clone(),
                origin: Origin::CarriedForward {
                    source_round: previous.round,
                },
            });
        }
        return Ok(Selection::Invalid);
    }
    Ok(if input.accepted_current.is_some() {
        Selection::Invalid
    } else {
        Selection::Missed
    })
}
