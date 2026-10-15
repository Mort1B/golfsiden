#[path = "../invitation_read_sessions/support.rs"]
#[allow(dead_code)]
mod common;
use axum::http::header;
pub use common::{
    TRIP, USER, app, blocker, body, expire_at_wait, fixture, get, lock_membership, session,
    wait_blocked, write,
};
use serde_json::Value;
use sqlx::PgPool;
pub fn path() -> String {
    format!("/api/tournaments/{TRIP}/final-round-visibility")
}
pub fn assert_private(response: &axum::response::Response) {
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
}
pub async fn read_state(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object(
      'tournaments',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tournaments t),
      'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY tournament_id,user_id) FROM tournament_memberships m),
      'scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM scores s)
    )").fetch_one(pool).await.unwrap()
}
