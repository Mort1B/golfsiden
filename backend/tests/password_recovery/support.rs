use axum::{body::Body, http::Request};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{self, derive_csrf_token, hash_password, hash_session_token},
    config::RecoveryOrigin,
    domain::password_recovery::RecoveryToken,
    repositories::{
        auth as sessions,
        password_recovery::{self, AdminRequest, GrantMetadata},
    },
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use uuid::{Uuid, uuid};
pub const ADMIN: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
pub const USER: Uuid = uuid!("00000000-0000-0000-0000-000000001001");
pub const OTHER: Uuid = uuid!("00000000-0000-0000-0000-000000001002");
pub const TRIP: Uuid = uuid!("00000000-0000-0000-0000-000000002001");
pub const TOKEN: &str = "recovery-admin-session";
pub const PASSWORD: &str = "recovery-old-password";
pub const NEW_PASSWORD: &str = "recovery-new-password";
pub async fn fixture(pool: &PgPool) -> AdminRequest {
    sqlx::raw_sql(include_str!("../../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
    let hash = hash_password(PASSWORD.as_bytes()).unwrap();
    sqlx::query("UPDATE users SET password_hash=$1 WHERE id=ANY($2)")
        .bind(&hash)
        .bind([ADMIN, USER, OTHER])
        .execute(pool)
        .await
        .unwrap();
    let principal = sessions::create_session(
        pool,
        ADMIN,
        &hash_session_token(TOKEN),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    let verified = golf_api::repositories::profile::credential(pool, ADMIN)
        .await
        .unwrap()
        .unwrap();
    AdminRequest {
        session_id: principal.session_id,
        user_id: ADMIN,
        tournament_id: TRIP,
        player_id: USER,
        verified_password: verified,
    }
}
pub fn app(pool: PgPool) -> axum::Router {
    let mut config = auth::AuthConfig::local();
    config.recovery_origin = Some(RecoveryOrigin::parse("https://golf.example", false).unwrap());
    api::router(AppState::with_auth(pool, config))
}
pub fn request(path: &str, value: Value, authenticated: bool, csrf: bool) -> Request<Body> {
    let mut request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json");
    if authenticated {
        request = request.header("cookie", format!("golf_session={TOKEN}"));
        if csrf {
            request = request.header("x-csrf-token", derive_csrf_token(TOKEN));
        }
    }
    request.body(Body::from(value.to_string())).unwrap()
}
pub async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
pub async fn issue(pool: &PgPool, request: &AdminRequest) -> (GrantMetadata, RecoveryToken) {
    let token = RecoveryToken::generate().unwrap();
    (
        password_recovery::admin_issue(pool, request, &token.hash())
            .await
            .unwrap(),
        token,
    )
}
pub fn admin_path() -> String {
    format!("/api/tournaments/{TRIP}/players/{USER}/password-recovery")
}
pub fn public_path(id: Uuid, action: &str) -> String {
    format!("/api/auth/password-recovery/{id}/{action}")
}
