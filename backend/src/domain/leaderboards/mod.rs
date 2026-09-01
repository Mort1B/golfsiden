mod round;
mod tournament;
mod types;

pub use round::{build_round_leaderboard, build_round_leaderboard_projected};
pub use tournament::{build_tournament_leaderboard, build_tournament_leaderboard_projected};
pub use types::*;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LeaderboardError {
    #[error("stored leaderboard data is inconsistent")]
    InvalidStoredData,
    #[error("score calculation failed")]
    Scoring(#[from] super::scoring::ScoringError),
}

#[cfg(test)]
mod tests;
