use super::{
    models::ScoringFormat,
    scoring::{ScoringError, scramble_playing_handicap},
};

const SCRAMBLE_MAX_INDEX_TENTHS: i32 = 360;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoreOwnerKind {
    Player,
    Team,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotHandicapPolicy {
    UncappedIndividualRoundAllowance,
    UncappedCourseHandicap,
    IndexCappedCourseHandicap { maximum_index_tenths: i32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamPlayingHandicap {
    Scramble35And15,
    FoursomesCombinedUnrounded50Percent,
}

impl TeamPlayingHandicap {
    pub const fn uses_preserved_team_snapshot(self) -> bool {
        matches!(self, Self::FoursomesCombinedUnrounded50Percent)
    }

    pub fn calculate(
        self,
        course_handicaps: &[i32],
        allowance_percent: i16,
    ) -> Result<Option<i32>, ScoringError> {
        match self {
            Self::Scramble35And15 => {
                scramble_playing_handicap(course_handicaps, allowance_percent).map(Some)
            }
            Self::FoursomesCombinedUnrounded50Percent => Ok(None),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundFormatPolicy {
    PlayerOwned {
        snapshot_handicap: SnapshotHandicapPolicy,
    },
    TeamOwned {
        exact_team_size: u16,
        snapshot_handicap: SnapshotHandicapPolicy,
        team_playing_handicap: TeamPlayingHandicap,
    },
}

impl RoundFormatPolicy {
    pub const fn for_format(format: ScoringFormat) -> Self {
        match format {
            ScoringFormat::IndividualStrokePlay => Self::PlayerOwned {
                snapshot_handicap: SnapshotHandicapPolicy::UncappedIndividualRoundAllowance,
            },
            ScoringFormat::TeamScramble => Self::TeamOwned {
                exact_team_size: 2,
                snapshot_handicap: SnapshotHandicapPolicy::IndexCappedCourseHandicap {
                    maximum_index_tenths: SCRAMBLE_MAX_INDEX_TENTHS,
                },
                team_playing_handicap: TeamPlayingHandicap::Scramble35And15,
            },
            ScoringFormat::TwoPlayerFoursomes => Self::TeamOwned {
                exact_team_size: 2,
                snapshot_handicap: SnapshotHandicapPolicy::UncappedCourseHandicap,
                team_playing_handicap: TeamPlayingHandicap::FoursomesCombinedUnrounded50Percent,
            },
        }
    }

    pub const fn owner_kind(self) -> ScoreOwnerKind {
        match self {
            Self::PlayerOwned { .. } => ScoreOwnerKind::Player,
            Self::TeamOwned { .. } => ScoreOwnerKind::Team,
        }
    }

    pub fn exact_team_size(self) -> Option<usize> {
        match self {
            Self::PlayerOwned { .. } => None,
            Self::TeamOwned {
                exact_team_size, ..
            } => Some(usize::from(exact_team_size)),
        }
    }

    pub const fn snapshot_handicap(self) -> SnapshotHandicapPolicy {
        match self {
            Self::PlayerOwned { snapshot_handicap }
            | Self::TeamOwned {
                snapshot_handicap, ..
            } => snapshot_handicap,
        }
    }

    pub fn effective_index_tenths(self, registered_tenths: i32) -> i32 {
        match self.snapshot_handicap() {
            SnapshotHandicapPolicy::UncappedIndividualRoundAllowance
            | SnapshotHandicapPolicy::UncappedCourseHandicap => registered_tenths,
            SnapshotHandicapPolicy::IndexCappedCourseHandicap {
                maximum_index_tenths,
            } => registered_tenths.min(maximum_index_tenths),
        }
    }

    pub fn team_playing_handicap(
        self,
        course_handicaps: &[i32],
        allowance_percent: i16,
    ) -> Result<Option<i32>, ScoringError> {
        match self {
            Self::PlayerOwned { .. } => Ok(None),
            Self::TeamOwned {
                team_playing_handicap,
                ..
            } => team_playing_handicap.calculate(course_handicaps, allowance_percent),
        }
    }

    pub const fn required_allowance_percent(self) -> Option<i16> {
        match self {
            Self::TeamOwned {
                team_playing_handicap: TeamPlayingHandicap::FoursomesCombinedUnrounded50Percent,
                ..
            } => Some(50),
            _ => None,
        }
    }

    pub const fn requires_preserved_team_handicap_snapshot(self) -> bool {
        match self {
            Self::TeamOwned {
                team_playing_handicap,
                ..
            } => team_playing_handicap.uses_preserved_team_snapshot(),
            Self::PlayerOwned { .. } => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn individual_policy_is_player_owned_and_uncapped() {
        let policy = RoundFormatPolicy::for_format(ScoringFormat::IndividualStrokePlay);

        assert_eq!(policy.owner_kind(), ScoreOwnerKind::Player);
        assert_eq!(policy.exact_team_size(), None);
        assert_eq!(policy.effective_index_tenths(540), 540);
        assert_eq!(
            policy.snapshot_handicap(),
            SnapshotHandicapPolicy::UncappedIndividualRoundAllowance
        );
        assert_eq!(policy.team_playing_handicap(&[12, 20], 95), Ok(None));
    }

    #[test]
    fn scramble_policy_is_exact_two_player_team_with_existing_formula() {
        let policy = RoundFormatPolicy::for_format(ScoringFormat::TeamScramble);

        assert_eq!(policy.owner_kind(), ScoreOwnerKind::Team);
        assert_eq!(policy.exact_team_size(), Some(2));
        assert_eq!(policy.effective_index_tenths(359), 359);
        assert_eq!(policy.effective_index_tenths(540), 360);
        assert_eq!(
            policy.snapshot_handicap(),
            SnapshotHandicapPolicy::IndexCappedCourseHandicap {
                maximum_index_tenths: 360
            }
        );
        assert_eq!(policy.team_playing_handicap(&[10, 20], 100), Ok(Some(7)));
    }

    #[test]
    fn foursomes_policy_is_exact_two_player_team_with_fixed_allowance() {
        let policy = RoundFormatPolicy::for_format(ScoringFormat::TwoPlayerFoursomes);
        assert_eq!(policy.owner_kind(), ScoreOwnerKind::Team);
        assert_eq!(policy.exact_team_size(), Some(2));
        assert_eq!(policy.effective_index_tenths(540), 540);
        assert_eq!(policy.required_allowance_percent(), Some(50));
        assert!(policy.requires_preserved_team_handicap_snapshot());
    }
}
