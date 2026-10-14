#![cfg(feature = "database-tests")]
#[path = "stableford/support.rs"]
#[allow(dead_code)]
mod match_fixture;
mod match_card_read_sessions {
    mod authorization;
    mod contracts;
    mod expiry;
    mod retry;
    mod states;
    mod support;
}
