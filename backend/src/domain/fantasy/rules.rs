use super::FantasyError;

/// One immutable version governs every round in a Fantasy game.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(into = "i16")]
pub enum RulesVersion {
    Legacy,
    ExceptionalAce,
}
impl TryFrom<i16> for RulesVersion {
    type Error = FantasyError;
    fn try_from(value: i16) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Legacy),
            2 => Ok(Self::ExceptionalAce),
            _ => Err(FantasyError::InvalidFacts),
        }
    }
}
impl From<RulesVersion> for i16 {
    fn from(value: RulesVersion) -> Self {
        match value {
            RulesVersion::Legacy => 1,
            RulesVersion::ExceptionalAce => 2,
        }
    }
}
