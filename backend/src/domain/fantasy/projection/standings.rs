use crate::domain::fantasy::{
    FantasyError, Points, RoundId,
    totals::{self, RoundContribution, SeasonEntry},
};
use std::collections::BTreeMap;
/// An absent frozen owner/entry is known nonparticipation, not an incomplete card.
#[derive(Debug, Clone, Copy)]
pub enum Contribution {
    NotStarted,
    NotParticipating,
    Played(Points),
}
impl Contribution {
    pub fn domain(self) -> Points {
        match self {
            Self::NotStarted => Points::NotStarted,
            Self::NotParticipating => Points::Settled(0),
            Self::Played(p) => p,
        }
    }
}
pub fn rank_contributions<Id: Copy + Ord>(
    rounds: &[RoundId],
    entries: &[(Id, Vec<Contribution>)],
) -> Result<BTreeMap<Id, (Contribution, Option<usize>)>, FantasyError> {
    let mut active = vec![];
    let mut prepared = vec![];
    let mut result = BTreeMap::new();
    for (id, values) in entries {
        if values.len() != rounds.len() {
            return Err(FantasyError::InvalidFacts);
        }
        if values.iter().any(|v| matches!(v, Contribution::Played(_))) {
            active.push(*id);
            prepared.push((
                *id,
                rounds
                    .iter()
                    .zip(values)
                    .map(|(round, value)| RoundContribution {
                        round: *round,
                        points: value.domain(),
                    })
                    .collect::<Vec<_>>(),
            ));
        } else {
            let value = if values.iter().any(|v| matches!(v, Contribution::NotStarted)) {
                Contribution::NotStarted
            } else {
                Contribution::NotParticipating
            };
            result.insert(*id, (value, None));
        }
    }
    let seasons: Vec<_> = prepared
        .iter()
        .map(|(id, rounds)| SeasonEntry { id: *id, rounds })
        .collect();
    for row in totals::standings(rounds, &active, &seasons)? {
        result.insert(row.id, (Contribution::Played(row.points), row.rank));
    }
    Ok(result)
}
