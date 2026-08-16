#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header, request::Builder},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{
        derive_csrf_token, generate_invitation_token, hash_invitation_token, hash_session_token,
    },
    repositories::auth,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT_ID: Uuid = uuid!("94000000-0000-0000-0000-000000000001");
const INVITATION_ID: Uuid = uuid!("94000000-0000-0000-0000-000000000002");
const ADMIN_ID: Uuid = uuid!("94000000-0000-0000-0000-000000000010");
const USER_A: Uuid = uuid!("94000000-0000-0000-0000-000000000011");
const USER_B: Uuid = uuid!("94000000-0000-0000-0000-000000000012");

async fn seed(pool: &PgPool, max_uses: Option<i32>) -> String {
    seed_with_expiry(pool, max_uses, Utc::now() + Duration::days(1)).await
}

async fn seed_with_expiry(
    pool: &PgPool,
    max_uses: Option<i32>,
    expires_at: chrono::DateTime<Utc>,
) -> String {
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index) VALUES
        ('94000000-0000-0000-0000-000000000021', 'Player A', 8.0),
        ('94000000-0000-0000-0000-000000000022', 'Player B', 9.0);
        INSERT INTO users (id, email, display_name, role, player_id) VALUES
        ('94000000-0000-0000-0000-000000000010', 'admin@race.test', 'Admin', 'viewer', NULL),
        ('94000000-0000-0000-0000-000000000011', 'a@race.test', 'A', 'player',
         '94000000-0000-0000-0000-000000000021'),
        ('94000000-0000-0000-0000-000000000012', 'b@race.test', 'B', 'player',
         '94000000-0000-0000-0000-000000000022');
        INSERT INTO tournaments
          (id, name, start_date, end_date, number_of_rounds, status)
        VALUES ('94000000-0000-0000-0000-000000000001', 'Race',
                '2026-09-01', '2026-09-02', 1, 'draft');
        INSERT INTO tournament_memberships (tournament_id, user_id, role)
        VALUES ('94000000-0000-0000-0000-000000000001',
                '94000000-0000-0000-0000-000000000010', 'admin');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    let token = generate_invitation_token().unwrap();
    sqlx::query(
        "INSERT INTO tournament_invitations
         (id, tournament_id, token_hash, created_by_user_id, expires_at,
            max_uses, series_id)
         VALUES ($1, $2, $3, $4, $5, $6, $1)",
    )
    .bind(INVITATION_ID)
    .bind(TOURNAMENT_ID)
    .bind(hash_invitation_token(&token).as_slice())
    .bind(ADMIN_ID)
    .bind(expires_at)
    .bind(max_uses)
    .execute(pool)
    .await
    .unwrap();
    for (user_id, session_token) in [
        (ADMIN_ID, "admin-race-session"),
        (USER_A, "a-race-session"),
        (USER_B, "b-race-session"),
    ] {
        auth::create_session(
            pool,
            user_id,
            &hash_session_token(session_token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
    }
    token
}

async fn lock_invitation(pool: &PgPool) -> sqlx::Transaction<'_, sqlx::Postgres> {
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM tournament_invitations WHERE id = $1 FOR UPDATE")
        .bind(INVITATION_ID)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction
}

fn authorized(builder: Builder, session_token: &str) -> Builder {
    builder
        .header(header::COOKIE, format!("golf_session={session_token}"))
        .header("x-csrf-token", derive_csrf_token(session_token))
}

fn json_request(builder: Builder, value: Value) -> Request<Body> {
    builder
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}

fn accept_request(session_token: &str, invitation_token: &str) -> Request<Body> {
    json_request(
        authorized(
            Request::post(format!("/api/invitations/{INVITATION_ID}/accept")),
            session_token,
        ),
        json!({"token": invitation_token}),
    )
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_final_slot_and_duplicate_accepts_do_not_over_redeem(pool: PgPool) {
    let token = seed(&pool, Some(1)).await;
    let app = api::router(AppState::new(pool.clone()));
    let (first, second) = tokio::join!(
        app.clone()
            .oneshot(accept_request("a-race-session", &token)),
        app.clone()
            .oneshot(accept_request("b-race-session", &token)),
    );
    let mut statuses = [first.unwrap().status(), second.unwrap().status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM invitation_redemptions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );

    let joined_user = sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM invitation_redemptions WHERE tournament_id = $1",
    )
    .bind(TOURNAMENT_ID)
    .fetch_one(&pool)
    .await
    .unwrap();
    let session = if joined_user == USER_A {
        "a-race-session"
    } else {
        "b-race-session"
    };
    let (duplicate_a, duplicate_b) = tokio::join!(
        app.clone().oneshot(accept_request(session, &token)),
        app.clone().oneshot(accept_request(session, &token)),
    );
    assert_eq!(duplicate_a.unwrap().status(), StatusCode::OK);
    assert_eq!(duplicate_b.unwrap().status(), StatusCode::OK);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM invitation_redemptions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_rotations_create_exactly_one_successor(pool: PgPool) {
    seed(&pool, None).await;
    let app = api::router(AppState::new(pool.clone()));
    let request = || {
        json_request(
            authorized(
                Request::post(format!(
                    "/api/tournaments/{TOURNAMENT_ID}/invitations/{INVITATION_ID}/rotate"
                )),
                "admin-race-session",
            ),
            json!({}),
        )
    };
    let (first, second) = tokio::join!(
        app.clone().oneshot(request()),
        app.clone().oneshot(request()),
    );
    let mut statuses = [first.unwrap().status(), second.unwrap().status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::CREATED, StatusCode::CONFLICT]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tournament_invitations WHERE predecessor_id = $1",
        )
        .bind(INVITATION_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn accept_racing_rotation_commits_one_consistent_order(pool: PgPool) {
    let token = seed(&pool, None).await;
    let app = api::router(AppState::new(pool.clone()));
    let rotate = json_request(
        authorized(
            Request::post(format!(
                "/api/tournaments/{TOURNAMENT_ID}/invitations/{INVITATION_ID}/rotate"
            )),
            "admin-race-session",
        ),
        json!({}),
    );
    let (accepted, rotated) = tokio::join!(
        app.clone()
            .oneshot(accept_request("a-race-session", &token)),
        app.clone().oneshot(rotate),
    );
    assert_eq!(rotated.unwrap().status(), StatusCode::CREATED);
    assert!(matches!(
        accepted.unwrap().status(),
        StatusCode::CREATED | StatusCode::GONE
    ));
    let (successors, redemptions) = sqlx::query_as::<_, (i64, i64)>(
        "SELECT
           (SELECT count(*) FROM tournament_invitations WHERE predecessor_id = $1),
           (SELECT count(*) FROM invitation_redemptions WHERE tournament_id = $2)",
    )
    .bind(INVITATION_ID)
    .bind(TOURNAMENT_ID)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(successors, 1);
    assert!(redemptions == 0 || redemptions == 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn accept_racing_revoke_never_redeems_after_revocation(pool: PgPool) {
    let token = seed(&pool, None).await;
    let app = api::router(AppState::new(pool.clone()));
    let revoke = authorized(
        Request::delete(format!(
            "/api/tournaments/{TOURNAMENT_ID}/invitations/{INVITATION_ID}"
        )),
        "admin-race-session",
    )
    .body(Body::empty())
    .unwrap();
    let (accepted, revoked) = tokio::join!(
        app.clone()
            .oneshot(accept_request("a-race-session", &token)),
        app.clone().oneshot(revoke),
    );
    assert_eq!(revoked.unwrap().status(), StatusCode::NO_CONTENT);
    assert!(matches!(
        accepted.unwrap().status(),
        StatusCode::CREATED | StatusCode::GONE
    ));
    let (revoked_at, redemptions) = sqlx::query_as::<_, (bool, i64)>(
        "SELECT revoked_at IS NOT NULL,
                (SELECT count(*) FROM invitation_redemptions WHERE tournament_id = $2)
         FROM tournament_invitations WHERE id = $1",
    )
    .bind(INVITATION_ID)
    .bind(TOURNAMENT_ID)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(revoked_at);
    assert!(redemptions == 0 || redemptions == 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn direct_redemption_racing_acceptance_uses_one_lock_order(pool: PgPool) {
    let token = seed(&pool, None).await;
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'player')",
    )
    .bind(TOURNAMENT_ID)
    .bind(USER_A)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournament_players
           (tournament_id, player_id, tournament_handicap)
         VALUES ($1, '94000000-0000-0000-0000-000000000021', 8.0)",
    )
    .bind(TOURNAMENT_ID)
    .execute(&pool)
    .await
    .unwrap();

    let blocker = lock_invitation(&pool).await;
    let direct_pool = pool.clone();
    let direct = tokio::spawn(async move {
        sqlx::query(
            "INSERT INTO invitation_redemptions
               (id, invitation_id, series_id, tournament_id, user_id, player_id, mode)
             VALUES ($1, $2, $2, $3, $4,
                     '94000000-0000-0000-0000-000000000021', 'acceptance')",
        )
        .bind(Uuid::new_v4())
        .bind(INVITATION_ID)
        .bind(TOURNAMENT_ID)
        .bind(USER_A)
        .execute(&direct_pool)
        .await
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    let app = api::router(AppState::new(pool.clone()));
    let acceptance = tokio::spawn(app.oneshot(accept_request("a-race-session", &token)));
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(!direct.is_finished());
    assert!(!acceptance.is_finished());
    blocker.commit().await.unwrap();

    let (direct_result, acceptance_result) =
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            tokio::join!(direct, acceptance)
        })
        .await
        .expect("direct redemption and API acceptance deadlocked");
    direct_result.unwrap().unwrap();
    let acceptance_response = acceptance_result.unwrap().unwrap();
    assert_eq!(acceptance_response.status(), StatusCode::OK);
    let acceptance_body = acceptance_response
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(
        serde_json::from_slice::<Value>(&acceptance_body).unwrap()["status"],
        "already_joined"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM invitation_redemptions
             WHERE tournament_id = $1 AND user_id = $2",
        )
        .bind(TOURNAMENT_ID)
        .bind(USER_A)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn registration_rechecks_database_time_after_waiting_past_expiry(pool: PgPool) {
    let token = seed_with_expiry(&pool, None, Utc::now() + Duration::milliseconds(1_500)).await;
    let blocker = lock_invitation(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    let request = json_request(
        Request::post(format!("/api/invitations/{INVITATION_ID}/register")),
        json!({
            "token": token,
            "account": {"email": "wait-register@test", "password": "a secure test password"},
            "player": {"display_name": "Wait register", "handicap_index": 10.0}
        }),
    );
    let task = tokio::spawn(app.oneshot(request));
    tokio::time::sleep(std::time::Duration::from_millis(1_700)).await;
    assert!(!task.is_finished());
    blocker.commit().await.unwrap();
    let response = task.await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::GONE);
    assert_eq!(
        sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT
               (SELECT count(*) FROM users WHERE email = 'wait-register@test'),
               (SELECT count(*) FROM players WHERE display_name = 'Wait register'),
               (SELECT count(*) FROM invitation_redemptions)",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        (0, 0, 0)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn acceptance_rechecks_database_time_after_waiting_past_expiry(pool: PgPool) {
    let token = seed_with_expiry(&pool, None, Utc::now() + Duration::milliseconds(1_500)).await;
    let blocker = lock_invitation(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    let task = tokio::spawn(app.oneshot(accept_request("a-race-session", &token)));
    tokio::time::sleep(std::time::Duration::from_millis(1_700)).await;
    assert!(!task.is_finished());
    blocker.commit().await.unwrap();
    let response = task.await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::GONE);
    assert_eq!(
        sqlx::query_as::<_, (i64, i64, i64)>(
            "SELECT
               (SELECT count(*) FROM tournament_memberships WHERE user_id = $1),
               (SELECT count(*) FROM tournament_players WHERE player_id =
                    '94000000-0000-0000-0000-000000000021'),
               (SELECT count(*) FROM invitation_redemptions)",
        )
        .bind(USER_A)
        .fetch_one(&pool)
        .await
        .unwrap(),
        (0, 0, 0)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn rotation_rechecks_database_time_after_waiting_past_expiry(pool: PgPool) {
    seed_with_expiry(&pool, None, Utc::now() + Duration::milliseconds(1_500)).await;
    let blocker = lock_invitation(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    let request = json_request(
        authorized(
            Request::post(format!(
                "/api/tournaments/{TOURNAMENT_ID}/invitations/{INVITATION_ID}/rotate"
            )),
            "admin-race-session",
        ),
        json!({}),
    );
    let task = tokio::spawn(app.oneshot(request));
    tokio::time::sleep(std::time::Duration::from_millis(1_700)).await;
    assert!(!task.is_finished());
    blocker.commit().await.unwrap();
    let response = task.await.unwrap().unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        sqlx::query_as::<_, (bool, i64)>(
            "SELECT revoked_at IS NOT NULL,
                    (SELECT count(*) FROM tournament_invitations
                     WHERE predecessor_id = $1)
             FROM tournament_invitations WHERE id = $1",
        )
        .bind(INVITATION_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        (false, 0)
    );
}
