use serde::Serialize;

use super::models::{RoundStatus, TournamentRole};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibilityMode {
    Full,
    FrontNine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct VisibilityMetadata {
    pub mode: VisibilityMode,
}

#[derive(Debug, Clone, Copy)]
pub struct VisibilityFacts {
    pub role: TournamentRole,
    pub is_final_round: bool,
    pub status: RoundStatus,
    pub number_of_holes: i16,
    pub back_nine_hidden: bool,
}

pub fn visibility(facts: VisibilityFacts) -> VisibilityMetadata {
    let non_admin = facts.role != TournamentRole::Admin;
    let applicable = non_admin && facts.is_final_round && facts.number_of_holes == 18;
    let hidden = applicable
        && facts.back_nine_hidden
        && matches!(
            facts.status,
            RoundStatus::Open | RoundStatus::Completed | RoundStatus::Locked
        );
    VisibilityMetadata {
        mode: if hidden {
            VisibilityMode::FrontNine
        } else {
            VisibilityMode::Full
        },
    }
}

pub fn unrestricted() -> VisibilityMetadata {
    VisibilityMetadata {
        mode: VisibilityMode::Full,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_toggle_controls_open_completed_and_locked_finals() {
        let base = VisibilityFacts {
            role: TournamentRole::Player,
            is_final_round: true,
            status: RoundStatus::Completed,
            number_of_holes: 18,
            back_nine_hidden: false,
        };
        assert_eq!(visibility(base).mode, VisibilityMode::Full);
        assert_eq!(
            visibility(VisibilityFacts {
                back_nine_hidden: true,
                ..base
            })
            .mode,
            VisibilityMode::FrontNine
        );
        for status in [
            RoundStatus::Open,
            RoundStatus::Completed,
            RoundStatus::Locked,
        ] {
            assert_eq!(
                visibility(VisibilityFacts {
                    status,
                    back_nine_hidden: true,
                    ..base
                })
                .mode,
                VisibilityMode::FrontNine
            );
        }
        assert_eq!(
            visibility(VisibilityFacts {
                status: RoundStatus::Draft,
                back_nine_hidden: true,
                ..base
            })
            .mode,
            VisibilityMode::Full
        );
    }

    #[test]
    fn only_exact_admin_bypasses_applicable_blackout() {
        for role in [
            TournamentRole::Scorer,
            TournamentRole::Player,
            TournamentRole::Viewer,
        ] {
            assert_eq!(
                visibility(VisibilityFacts {
                    role,
                    is_final_round: true,
                    status: RoundStatus::Locked,
                    number_of_holes: 18,
                    back_nine_hidden: true,
                })
                .mode,
                VisibilityMode::FrontNine
            );
        }
        assert_eq!(
            visibility(VisibilityFacts {
                role: TournamentRole::Admin,
                is_final_round: true,
                status: RoundStatus::Open,
                number_of_holes: 18,
                back_nine_hidden: true,
            })
            .mode,
            VisibilityMode::Full
        );
        for (is_final_round, number_of_holes) in [(false, 18), (true, 9)] {
            assert_eq!(
                visibility(VisibilityFacts {
                    role: TournamentRole::Player,
                    is_final_round,
                    status: RoundStatus::Open,
                    number_of_holes,
                    back_nine_hidden: true,
                })
                .mode,
                VisibilityMode::Full
            );
        }
    }
}
