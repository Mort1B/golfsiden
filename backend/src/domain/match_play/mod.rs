//! Pure 18-hole singles foundation, not a selectable format or reporting API.

mod handicap;
mod points;

pub use handicap::{HandicapAllocation, MatchMode};
pub use points::{ConfirmationStatus, MatchPoints, PointAward};

pub const HOLE_COUNT: u8 = 18;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MatchPlayError {
    #[error("hole number and stroke index must be between 1 and 18")]
    InvalidHole,
    #[error("hole reports must resolve a contiguous sequence starting at hole 1")]
    OutOfOrderHole,
    #[error("no further report is permitted after the match has ended")]
    MatchAlreadyFinished,
    #[error("an unfinished match cannot have a confirmed point award")]
    UnfinishedConfirmation,
    #[error("match point total exceeds its supported range")]
    PointsOverflow,
}

/// Slots refer to the two preserved opponents; they do not create player/team IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opponent {
    First,
    Second,
}

impl Opponent {
    pub const fn other(self) -> Self {
        match self {
            Self::First => Self::Second,
            Self::Second => Self::First,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HoleOutcome {
    WonBy(Opponent),
    Halved,
}

/// Already accepted sporting facts, distinct from numeric proposals. A future
/// reporting boundary must validate actor, agreement/concession/ruling provenance,
/// effective point for visibility, revisions and authority before constructing these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchReport {
    Hole { number: u8, outcome: HoleOutcome },
    Conceded { by: Opponent },
    Awarded { winner: Opponent },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchFinish {
    OnHoles {
        winner: Opponent,
        margin: u8,
        holes_remaining: u8,
    },
    Draw,
    Conceded {
        winner: Opponent,
    },
    Awarded {
        winner: Opponent,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchState {
    // Positive means First leads; private fields preserve derivation invariants.
    lead: i8,
    resolved_holes: u8,
    finish: Option<MatchFinish>,
}

impl MatchState {
    pub fn lead(self) -> i8 {
        self.lead
    }

    /// Resolved reports, not a claim that every hole was physically played.
    pub fn resolved_holes(self) -> u8 {
        self.resolved_holes
    }

    pub fn holes_remaining(self) -> u8 {
        HOLE_COUNT - self.resolved_holes
    }

    pub fn finish(self) -> Option<MatchFinish> {
        self.finish
    }
}

/// Derives one full accepted sequence, failing rather than ignoring invalid or
/// post-finish reports. This does not authorize corrections or withdraw concessions.
/// For a restricted read, a future caller must supply only the permitted prefix
/// and permitted terminal events; full-state metadata cannot be copied then hidden.
pub fn derive_match(reports: &[MatchReport]) -> Result<MatchState, MatchPlayError> {
    let mut state = MatchState {
        lead: 0,
        resolved_holes: 0,
        finish: None,
    };
    for report in reports {
        if state.finish.is_some() {
            return Err(MatchPlayError::MatchAlreadyFinished);
        }
        match *report {
            MatchReport::Hole { number, outcome } => {
                if !(1..=HOLE_COUNT).contains(&number) {
                    return Err(MatchPlayError::InvalidHole);
                }
                if number != state.resolved_holes + 1 {
                    return Err(MatchPlayError::OutOfOrderHole);
                }
                state.resolved_holes = number;
                state.lead += match outcome {
                    HoleOutcome::WonBy(Opponent::First) => 1,
                    HoleOutcome::WonBy(Opponent::Second) => -1,
                    HoleOutcome::Halved => 0,
                };
                let margin = state.lead.unsigned_abs();
                if margin > state.holes_remaining() {
                    state.finish = Some(MatchFinish::OnHoles {
                        winner: if state.lead > 0 {
                            Opponent::First
                        } else {
                            Opponent::Second
                        },
                        margin,
                        holes_remaining: state.holes_remaining(),
                    });
                } else if number == HOLE_COUNT {
                    state.finish = Some(MatchFinish::Draw);
                }
            }
            MatchReport::Conceded { by } => {
                state.finish = Some(MatchFinish::Conceded { winner: by.other() });
            }
            MatchReport::Awarded { winner } => {
                state.finish = Some(MatchFinish::Awarded { winner });
            }
        }
    }
    Ok(state)
}

#[cfg(test)]
mod tests;
