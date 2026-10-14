#![cfg(feature = "database-tests")]
mod lifecycle_expiry {
    #[path = "../stableford/support.rs"]
    #[allow(dead_code)]
    mod base;
    mod scenarios;
    mod support;
}
