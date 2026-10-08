use super::*;
use uuid::Uuid;
fn golfer(n: u128) -> GolferId {
    GolferId(Uuid::from_u128(n))
}
fn round(n: u128) -> RoundId {
    RoundId(Uuid::from_u128(n))
}
fn token(n: u8) -> SourceToken {
    SourceToken([n; 32])
}
mod holes_and_rounds;
mod matches_and_totals;
mod selections;
