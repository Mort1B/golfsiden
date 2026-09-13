#![cfg(feature = "database-tests")]
#[path = "password_recovery/api.rs"]
mod api_tests;
#[path = "password_recovery/contention.rs"]
mod contention;
#[path = "password_recovery/races.rs"]
mod races;
#[path = "password_recovery/rules.rs"]
mod rules;
#[path = "password_recovery/schema.rs"]
mod schema;
#[path = "password_recovery/support.rs"]
mod support;
