#![cfg(feature = "database-tests")]
#[path = "stableford/support.rs"]
mod stableford_fixture;
mod stableford_read_sessions {
    mod authorization;
    mod contracts;
    mod expiry;
    mod retry;
    mod support;
}
