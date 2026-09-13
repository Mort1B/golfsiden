//! Shared input states for new scoring foundations; no persistence/transport contract.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("a numeric gross score must be between 1 and 20")]
pub struct InvalidGrossScore;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GrossScore(u8);

impl GrossScore {
    pub fn new(value: i32) -> Result<Self, InvalidGrossScore> {
        if !(1..=20).contains(&value) {
            return Err(InvalidGrossScore);
        }
        Ok(Self(value as u8))
    }

    pub fn value(self) -> i32 {
        i32::from(self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerHoleInput {
    Unentered,
    Numeric(GrossScore),
    NoScore,
}
