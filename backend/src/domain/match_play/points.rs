use super::{MatchFinish, MatchPlayError, MatchState, Opponent};

/// Supplied by the future authorized persistence boundary; not a confirmation action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmationStatus {
    Unconfirmed,
    Confirmed,
}

/// Exact half-point units. This type is never an overall stroke contribution.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MatchPoints(u32);

impl MatchPoints {
    pub fn half_units(self) -> u32 {
        self.0
    }

    pub fn checked_add(self, other: Self) -> Result<Self, MatchPlayError> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or(MatchPlayError::PointsOverflow)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointAward {
    pub first: MatchPoints,
    pub second: MatchPoints,
}

impl MatchState {
    pub fn point_award(
        self,
        confirmation: ConfirmationStatus,
    ) -> Result<Option<PointAward>, MatchPlayError> {
        if confirmation == ConfirmationStatus::Unconfirmed {
            return Ok(None);
        }
        let winner = match self.finish() {
            None => return Err(MatchPlayError::UnfinishedConfirmation),
            Some(MatchFinish::Draw) => None,
            Some(MatchFinish::OnHoles { winner, .. })
            | Some(MatchFinish::Conceded { winner })
            | Some(MatchFinish::Awarded { winner }) => Some(winner),
        };
        let (first, second) = match winner {
            Some(Opponent::First) => (2, 0),
            Some(Opponent::Second) => (0, 2),
            None => (1, 1),
        };
        Ok(Some(PointAward {
            first: MatchPoints(first),
            second: MatchPoints(second),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_sum_rejects_overflow() {
        assert_eq!(
            MatchPoints(u32::MAX).checked_add(MatchPoints(1)),
            Err(MatchPlayError::PointsOverflow)
        );
        assert_eq!(
            MatchPoints(u32::MAX - 1).checked_add(MatchPoints(1)),
            Ok(MatchPoints(u32::MAX))
        );
    }
}
