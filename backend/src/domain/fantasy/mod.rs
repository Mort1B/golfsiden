//! Pure Fantasy rules. Callers supply authorized, preserved, complete source facts.
//! No input type here proves database authorization, locking or source-token freshness.
pub mod holes;
pub mod matches;
pub mod rounds;
pub mod selections;
pub mod totals;

use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GolferId(pub Uuid);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ManagerId(pub Uuid);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RoundId(pub Uuid);

/// Opaque owner-wide fingerprint from the future authoritative adapter. It must
/// include owner/round, preserved inputs, confirmations and a retained generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceToken(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Points {
    /// Explicit scheduled future round, not missing source facts.
    NotStarted,
    /// Recorded permitted contribution only; not a complete total or rank.
    Pending {
        recorded: i64,
    },
    Provisional(i64),
    Settled(i64),
    /// Deliberately carries no hidden subtotal, revision or finality metadata.
    Withheld,
}

impl Points {
    pub fn total(self) -> Option<i64> {
        match self {
            Self::Provisional(value) | Self::Settled(value) => Some(value),
            Self::Pending { .. } | Self::Withheld | Self::NotStarted => None,
        }
    }

    pub fn multiply(self, factor: i64) -> Result<Self, FantasyError> {
        let checked = |value: i64| value.checked_mul(factor).ok_or(FantasyError::Overflow);
        Ok(match self {
            Self::Pending { recorded } => Self::Pending {
                recorded: checked(recorded)?,
            },
            Self::Provisional(value) => Self::Provisional(checked(value)?),
            Self::Settled(value) => Self::Settled(checked(value)?),
            Self::Withheld => Self::Withheld,
            Self::NotStarted => Self::NotStarted,
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum FantasyError {
    #[error("invalid preserved Fantasy source facts")]
    InvalidFacts,
    #[error("duplicate owner, golfer, round or entry")]
    DuplicateIdentity,
    #[error("Fantasy points arithmetic overflow")]
    Overflow,
    #[error("a lineup requires four distinct golfers and a captain among them")]
    InvalidLineup,
}

#[cfg(test)]
mod tests;
