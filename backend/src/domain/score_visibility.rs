use chrono::{DateTime, Utc};
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
    pub observed_at: DateTime<Utc>,
    pub hidden_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy)]
pub struct VisibilityFacts {
    pub role: TournamentRole,
    pub is_final_round: bool,
    pub status: RoundStatus,
    pub number_of_holes: i16,
    pub hidden_until: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
}

pub fn visibility(facts: VisibilityFacts) -> VisibilityMetadata {
    let non_admin = facts.role != TournamentRole::Admin;
    let applicable = non_admin && facts.is_final_round && facts.number_of_holes == 18;
    let hidden = applicable
        && match facts.status {
            RoundStatus::Open => true,
            RoundStatus::Completed | RoundStatus::Locked => facts
                .hidden_until
                .is_none_or(|deadline| facts.observed_at < deadline),
            RoundStatus::Draft => false,
        };
    VisibilityMetadata {
        mode: if hidden {
            VisibilityMode::FrontNine
        } else {
            VisibilityMode::Full
        },
        observed_at: facts.observed_at,
        hidden_until: facts.hidden_until,
    }
}

pub fn unrestricted(observed_at: DateTime<Utc>) -> VisibilityMetadata {
    VisibilityMetadata {
        mode: VisibilityMode::Full,
        observed_at,
        hidden_until: None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    #[test]
    fn non_open_deadline_equality_reveals_but_open_remains_hidden() {
        let now = Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0).unwrap();
        let base = VisibilityFacts {
            role: TournamentRole::Player,
            is_final_round: true,
            status: RoundStatus::Completed,
            number_of_holes: 18,
            hidden_until: Some(now),
            observed_at: now,
        };
        assert_eq!(visibility(base).mode, VisibilityMode::Full);
        assert_eq!(
            visibility(VisibilityFacts {
                hidden_until: Some(now + chrono::Duration::seconds(1)),
                ..base
            })
            .mode,
            VisibilityMode::FrontNine
        );
        assert_eq!(
            visibility(VisibilityFacts {
                hidden_until: Some(now - chrono::Duration::seconds(1)),
                ..base
            })
            .mode,
            VisibilityMode::Full
        );
        assert_eq!(
            visibility(VisibilityFacts {
                status: RoundStatus::Open,
                ..base
            })
            .mode,
            VisibilityMode::FrontNine
        );
    }

    #[test]
    fn only_exact_admin_bypasses_applicable_blackout() {
        let now = Utc.with_ymd_and_hms(2026, 9, 1, 12, 0, 0).unwrap();
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
                    hidden_until: None,
                    observed_at: now,
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
                hidden_until: None,
                observed_at: now,
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
                    hidden_until: None,
                    observed_at: now,
                })
                .mode,
                VisibilityMode::Full
            );
        }
    }
}
