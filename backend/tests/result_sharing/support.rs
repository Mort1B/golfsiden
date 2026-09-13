use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use golf_api::{
    auth::hash_session_token,
    domain::result_sharing::ResultShareToken,
    repositories::{auth, result_sharing},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};
pub const TRIP: Uuid = uuid!("00000000-0000-0000-0000-000000002001");
pub const ADMIN: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
pub const USER: Uuid = uuid!("00000000-0000-0000-0000-000000001001");
pub const ADMIN_TOKEN: &str = "public-results-admin-test";
pub const USER_TOKEN: &str = "public-results-user-test";
pub async fn fixture(pool: &PgPool) -> Uuid {
    sqlx::raw_sql(include_str!("../../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
    let mut admin = Uuid::nil();
    for (id, token) in [(ADMIN, ADMIN_TOKEN), (USER, USER_TOKEN)] {
        let session = auth::create_session(
            pool,
            id,
            &hash_session_token(token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
        if id == ADMIN {
            admin = session.session_id;
        }
    }
    admin
}
pub async fn issue(
    pool: &PgPool,
    session: Uuid,
    expected: Option<Uuid>,
) -> (result_sharing::GrantMetadata, ResultShareToken) {
    let token = ResultShareToken::generate().unwrap();
    let grant = result_sharing::issue(pool, session, TRIP, expected, &token.hash())
        .await
        .unwrap();
    (grant, token)
}
pub fn request(
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
    csrf: bool,
) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("cookie", format!("golf_session={token}"));
        if csrf {
            request = request.header("x-csrf-token", golf_api::auth::derive_csrf_token(token));
        }
    }
    request
        .body(body.map_or_else(Body::empty, |body| Body::from(body.to_string())))
        .unwrap()
}
pub async fn response(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    assert_eq!(response.headers()["x-robots-tag"], "noindex, nofollow");
    let status = response.status();
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}
pub fn public_request(
    id: Uuid,
    token: &ResultShareToken,
    metric: &str,
    cookie: Option<&str>,
) -> Request<Body> {
    request(
        "POST",
        &format!("/api/public/results/{id}"),
        cookie,
        Some(json!({"token":token.expose(),"metric":metric})),
        false,
    )
}
pub async fn context(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, session: Uuid) {
    sqlx::query("SELECT set_config('app.result_share_session_id',$1::text,true)")
        .bind(session)
        .execute(&mut **tx)
        .await
        .unwrap();
}
pub async fn expiring(pool: &PgPool, session: Uuid, milliseconds: i64) -> (Uuid, ResultShareToken) {
    let mut tx = pool.begin().await.unwrap();
    context(&mut tx, session).await;
    let token = ResultShareToken::generate().unwrap();
    let id = Uuid::new_v4();
    sqlx::query("WITH expiry AS(SELECT clock_timestamp()+($5::bigint*interval '1 millisecond') AS expires) INSERT INTO tournament_result_shares(id,tournament_id,token_hash,created_by,created_at,expires_at) SELECT $1,$2,$3,$4,expires-interval '30 days',expires FROM expiry")
        .bind(id).bind(TRIP).bind(token.hash().as_bytes()).bind(ADMIN).bind(milliseconds).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    (id, token)
}
pub async fn wait_locks(pool: &PgPool, count: i64) {
    tokio::time::timeout(std::time::Duration::from_secs(3),async{
        loop {let n:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock'").fetch_one(pool).await.unwrap();
        if n>=count {break;}tokio::time::sleep(std::time::Duration::from_millis(10)).await;}
    }).await.expect("missing observed contention");
}
