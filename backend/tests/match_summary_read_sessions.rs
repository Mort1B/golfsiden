#![cfg(feature = "database-tests")]
#[path = "match_card_read_sessions/support.rs"]
#[allow(dead_code)]
mod card_support;
#[path = "stableford/support.rs"]
#[allow(dead_code)]
mod match_fixture;
mod match_summary_read_sessions {
    mod authorization;
    mod contracts;
    mod expiry;
    mod retry;
    mod support;
}
