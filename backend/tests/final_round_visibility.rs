#![cfg(feature = "database-tests")]

use std::{sync::Arc, time::Duration as StdDuration};

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::{auth, tournament_visibility},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{sync::broadcast::error::TryRecvError, time::timeout};
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("18000000-0000-0000-0000-000000000001");
const OTHER_TOURNAMENT: Uuid = uuid!("18000000-0000-0000-0000-000000000002");
const ADMIN: Uuid = uuid!("18000000-0000-0000-0000-000000000003");
const VIEWER: Uuid = uuid!("18000000-0000-0000-0000-000000000004");
const ADMIN_TOKEN: &str = "visibility-admin-token";
const VIEWER_TOKEN: &str = "visibility-viewer-token";

const MIGRATIONS_THROUGH_17: [&str; 17] = [
    include_str!("../../migrations/0001_initial_schema.sql"),
    include_str!("../../migrations/0002_round_opening.sql"),
    include_str!("../../migrations/0003_scorecards.sql"),
    include_str!("../../migrations/0004_round_completion.sql"),
    include_str!("../../migrations/0005_auth_sessions.sql"),
    include_str!("../../migrations/0006_tournament_memberships.sql"),
    include_str!("../../migrations/0007_tournament_invitations.sql"),
    include_str!("../../migrations/0008_reusable_invitations.sql"),
    include_str!("../../migrations/0009_username_accounts_fixed_handicaps.sql"),
    include_str!("../../migrations/0010_course_revisions.sql"),
    include_str!("../../migrations/0011_round_flights.sql"),
    include_str!("../../migrations/0012_remove_flight_scorekeepers.sql"),
    include_str!("../../migrations/0013_two_player_foursomes.sql"),
    include_str!("../../migrations/0014_tournament_counted_rounds.sql"),
    include_str!("../../migrations/0015_tournament_start.sql"),
    include_str!("../../migrations/0016_tournament_mandatory_round.sql"),
    include_str!("../../migrations/0017_final_score_embargo.sql"),
];
const MIGRATION_18: &str = include_str!("../../migrations/0018_admin_final_round_visibility.sql");

async fn seed(pool: &PgPool) -> Uuid {
    sqlx::raw_sql(
        r#"
        INSERT INTO users (id, username, display_name, role) VALUES
        ('18000000-0000-0000-0000-000000000003', 'visibility_admin', 'Admin', 'viewer'),
        ('18000000-0000-0000-0000-000000000004', 'visibility_viewer', 'Viewer', 'admin');
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds) VALUES
        ('18000000-0000-0000-0000-000000000001', 'Visibility', '2026-09-01', '2026-09-02', 2),
        ('18000000-0000-0000-0000-000000000002', 'Other', '2026-09-01', '2026-09-01', 1);
        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
        ('18000000-0000-0000-0000-000000000001', '18000000-0000-0000-0000-000000000003', 'admin'),
        ('18000000-0000-0000-0000-000000000001', '18000000-0000-0000-0000-000000000004', 'viewer'),
        ('18000000-0000-0000-0000-000000000002', '18000000-0000-0000-0000-000000000004', 'admin');
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_name, tee_name,
           number_of_holes, scoring_format) VALUES
        ('18000000-0000-0000-0000-000000000011', '18000000-0000-0000-0000-000000000001', 1, 'First', '2026-09-01', '', '', 18, 'individual_stroke_play'),
        ('18000000-0000-0000-0000-000000000012', '18000000-0000-0000-0000-000000000001', 2, 'Final', '2026-09-02', '', '', 18, 'individual_stroke_play');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    let admin_session = auth::create_session(
        pool,
        ADMIN,
        &hash_session_token(ADMIN_TOKEN),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    auth::create_session(
        pool,
        VIEWER,
        &hash_session_token(VIEWER_TOKEN),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    admin_session.session_id
}

fn visibility_request(
    method: &str,
    tournament_id: Uuid,
    token: Option<&str>,
    body: Option<Value>,
) -> Request<Body> {
    let uri = format!("/api/tournaments/{tournament_id}/final-round-visibility");
    let mut request = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        request = request.header(header::COOKIE, format!("golf_session={token}"));
        if method == "PATCH" {
            request = request.header("x-csrf-token", derive_csrf_token(token));
        }
    }
    if body.is_some() {
        request = request.header(header::CONTENT_TYPE, "application/json");
    }
    request
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap()
}

async fn response_body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

async fn stored_visibility(pool: &PgPool, tournament_id: Uuid) -> (bool, DateTime<Utc>) {
    sqlx::query_as(
        "SELECT final_round_back_nine_hidden, visibility_updated_at
         FROM tournaments WHERE id = $1",
    )
    .bind(tournament_id)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn private_api_enforces_auth_csrf_shape_stale_tokens_and_post_commit_event(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(Arc::clone(&state));

    let unauthenticated = app
        .clone()
        .oneshot(visibility_request("GET", TOURNAMENT, None, None))
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let forbidden = app
        .clone()
        .oneshot(visibility_request(
            "GET",
            TOURNAMENT,
            Some(VIEWER_TOKEN),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let missing = app
        .clone()
        .oneshot(visibility_request(
            "GET",
            Uuid::new_v4(),
            Some(ADMIN_TOKEN),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    let get = app
        .clone()
        .oneshot(visibility_request(
            "GET",
            TOURNAMENT,
            Some(ADMIN_TOKEN),
            None,
        ))
        .await
        .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    assert_eq!(get.headers()[header::CACHE_CONTROL], "private, no-store");
    let initial = response_body(get).await;
    assert_eq!(initial["tournament_id"], TOURNAMENT.to_string());
    assert_eq!(initial["back_nine_hidden"], true);
    let expected = initial["visibility_updated_at"].clone();

    let no_csrf = Request::patch(format!(
        "/api/tournaments/{TOURNAMENT}/final-round-visibility"
    ))
    .header(header::COOKIE, format!("golf_session={ADMIN_TOKEN}"))
    .header(header::CONTENT_TYPE, "application/json")
    .body(Body::from(
        json!({"back_nine_hidden": false, "expected_visibility_updated_at": expected}).to_string(),
    ))
    .unwrap();
    assert_eq!(
        app.clone().oneshot(no_csrf).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let malformed = visibility_request(
        "PATCH",
        TOURNAMENT,
        Some(ADMIN_TOKEN),
        Some(json!({
            "back_nine_hidden": false,
            "expected_visibility_updated_at": expected,
            "extra": true
        })),
    );
    assert_eq!(
        app.clone().oneshot(malformed).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );
    let denied = visibility_request(
        "PATCH",
        TOURNAMENT,
        Some(VIEWER_TOKEN),
        Some(json!({
            "back_nine_hidden": false,
            "expected_visibility_updated_at": expected
        })),
    );
    assert_eq!(
        app.clone().oneshot(denied).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let cross_target = visibility_request(
        "PATCH",
        OTHER_TOURNAMENT,
        Some(ADMIN_TOKEN),
        Some(json!({
            "back_nine_hidden": false,
            "expected_visibility_updated_at": expected
        })),
    );
    assert_eq!(
        app.clone().oneshot(cross_target).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    let no_op = visibility_request(
        "PATCH",
        TOURNAMENT,
        Some(ADMIN_TOKEN),
        Some(json!({
            "back_nine_hidden": true,
            "expected_visibility_updated_at": expected
        })),
    );
    let no_op = app.clone().oneshot(no_op).await.unwrap();
    assert_eq!(no_op.status(), StatusCode::OK);
    assert_eq!(
        response_body(no_op).await["visibility_updated_at"],
        expected
    );
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    let release = visibility_request(
        "PATCH",
        TOURNAMENT,
        Some(ADMIN_TOKEN),
        Some(json!({
            "back_nine_hidden": false,
            "expected_visibility_updated_at": expected
        })),
    );
    let release = app.clone().oneshot(release).await.unwrap();
    assert_eq!(release.status(), StatusCode::OK);
    assert_eq!(
        release.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let released = response_body(release).await;
    assert_eq!(released["back_nine_hidden"], false);
    assert_ne!(released["visibility_updated_at"], expected);
    let event = events.try_recv().unwrap();
    assert_eq!(event.resource, "visibility");
    assert_eq!(event.tournament_id, TOURNAMENT);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    let stale = visibility_request(
        "PATCH",
        TOURNAMENT,
        Some(ADMIN_TOKEN),
        Some(json!({
            "back_nine_hidden": true,
            "expected_visibility_updated_at": expected
        })),
    );
    let stale = app.oneshot(stale).await.unwrap();
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_body(stale).await["error"]["code"],
        "final_round_visibility_stale"
    );
    assert!(!stored_visibility(&pool, TOURNAMENT).await.0);
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_expected_token_writes_serialize_without_deadlock(pool: PgPool) {
    let admin_session = seed(&pool).await;
    let expected = stored_visibility(&pool, TOURNAMENT).await.1;
    let first_pool = pool.clone();
    let second_pool = pool.clone();
    let first = tokio::spawn(async move {
        tournament_visibility::update_authorized(
            &first_pool,
            admin_session,
            TOURNAMENT,
            false,
            expected,
        )
        .await
    });
    let second = tokio::spawn(async move {
        tournament_visibility::update_authorized(
            &second_pool,
            admin_session,
            TOURNAMENT,
            false,
            expected,
        )
        .await
    });
    let (first, second) = timeout(StdDuration::from_secs(5), async {
        tokio::join!(first, second)
    })
    .await
    .expect("visibility writes deadlocked");
    let results = [first.unwrap(), second.unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|result| matches!(
                result,
                Err(tournament_visibility::FinalRoundVisibilityError::Stale)
            ))
            .count(),
        1
    );
    assert!(!stored_visibility(&pool, TOURNAMENT).await.0);
}

#[sqlx::test(migrations = "../migrations")]
async fn database_guard_requires_exact_active_session_and_manages_timestamp(pool: PgPool) {
    let admin_session = seed(&pool).await;
    let direct =
        sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden = FALSE WHERE id = $1")
            .bind(TOURNAMENT)
            .execute(&pool)
            .await
            .unwrap_err();
    assert_eq!(
        constraint(&direct),
        Some("final_round_visibility_context_required")
    );

    let viewer_session: Uuid = sqlx::query_scalar(
        "SELECT id FROM user_sessions WHERE user_id = $1 AND revoked_at IS NULL",
    )
    .bind(VIEWER)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut viewer = pool.begin().await.unwrap();
    set_context(&mut viewer, TOURNAMENT, viewer_session).await;
    let denied =
        sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden = FALSE WHERE id = $1")
            .bind(TOURNAMENT)
            .execute(&mut *viewer)
            .await
            .unwrap_err();
    assert_eq!(
        constraint(&denied),
        Some("final_round_visibility_admin_required")
    );
    viewer.rollback().await.unwrap();

    let timestamp_only = sqlx::query(
        "UPDATE tournaments SET visibility_updated_at = clock_timestamp() WHERE id = $1",
    )
    .bind(TOURNAMENT)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&timestamp_only),
        Some("final_round_visibility_timestamp_managed")
    );

    let (_, before) = stored_visibility(&pool, TOURNAMENT).await;
    let mut valid = pool.begin().await.unwrap();
    set_context(&mut valid, TOURNAMENT, admin_session).await;
    sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden = FALSE WHERE id = $1")
        .bind(TOURNAMENT)
        .execute(&mut *valid)
        .await
        .unwrap();
    valid.commit().await.unwrap();
    let (hidden, after) = stored_visibility(&pool, TOURNAMENT).await;
    assert!(!hidden);
    assert!(after > before);

    sqlx::query("UPDATE user_sessions SET revoked_at = clock_timestamp() WHERE id = $1")
        .bind(admin_session)
        .execute(&pool)
        .await
        .unwrap();
    let mut revoked = pool.begin().await.unwrap();
    set_context(&mut revoked, TOURNAMENT, admin_session).await;
    let revoked_error =
        sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden = TRUE WHERE id = $1")
            .bind(TOURNAMENT)
            .execute(&mut *revoked)
            .await
            .unwrap_err();
    assert_eq!(
        constraint(&revoked_error),
        Some("final_round_visibility_admin_required")
    );
}

async fn set_context(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    tournament_id: Uuid,
    session_id: Uuid,
) {
    sqlx::query(
        "SELECT
           set_config('app.final_round_visibility_tournament_id', $1::text, true),
           set_config('app.final_round_visibility_session_id', $2::text, true)",
    )
    .bind(tournament_id)
    .bind(session_id)
    .execute(&mut **transaction)
    .await
    .unwrap();
}

fn constraint(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .and_then(|database| database.constraint())
}

#[sqlx::test(migrations = false)]
async fn schema_17_upgrade_preserves_only_already_visible_finals(pool: PgPool) {
    for migration in MIGRATIONS_THROUGH_17 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        r#"
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds) VALUES
        ('18100000-0000-0000-0000-000000000001', 'Expired completed', '2026-01-01', '2026-01-01', 1),
        ('18100000-0000-0000-0000-000000000002', 'Expired locked', '2026-01-01', '2026-01-01', 1),
        ('18100000-0000-0000-0000-000000000003', 'Future completed', '2026-01-01', '2026-01-01', 1),
        ('18100000-0000-0000-0000-000000000004', 'Null locked', '2026-01-01', '2026-01-01', 1),
        ('18100000-0000-0000-0000-000000000005', 'Expired open', '2026-01-01', '2026-01-01', 1),
        ('18100000-0000-0000-0000-000000000006', 'Expired non-final', '2026-01-01', '2026-01-02', 2);
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_name, tee_name,
           status, scoring_format) VALUES
        ('18100000-0000-0000-0000-000000000011', '18100000-0000-0000-0000-000000000001', 1, 'Final', '2026-01-01', '', '', 'completed', 'individual_stroke_play'),
        ('18100000-0000-0000-0000-000000000012', '18100000-0000-0000-0000-000000000002', 1, 'Final', '2026-01-01', '', '', 'locked', 'individual_stroke_play'),
        ('18100000-0000-0000-0000-000000000013', '18100000-0000-0000-0000-000000000003', 1, 'Final', '2026-01-01', '', '', 'completed', 'individual_stroke_play'),
        ('18100000-0000-0000-0000-000000000014', '18100000-0000-0000-0000-000000000004', 1, 'Final', '2026-01-01', '', '', 'locked', 'individual_stroke_play'),
        ('18100000-0000-0000-0000-000000000015', '18100000-0000-0000-0000-000000000005', 1, 'Final', '2026-01-01', '', '', 'open', 'individual_stroke_play'),
        ('18100000-0000-0000-0000-000000000016', '18100000-0000-0000-0000-000000000006', 1, 'Not final', '2026-01-01', '', '', 'completed', 'individual_stroke_play');
        ALTER TABLE rounds DISABLE TRIGGER rounds_protect_final_score_embargo;
        UPDATE rounds SET final_scores_hidden_until = '2020-01-01T00:00:00Z'
        WHERE id IN (
          '18100000-0000-0000-0000-000000000011',
          '18100000-0000-0000-0000-000000000012',
          '18100000-0000-0000-0000-000000000015',
          '18100000-0000-0000-0000-000000000016'
        );
        UPDATE rounds SET final_scores_hidden_until = '2100-01-01T00:00:00Z'
        WHERE id = '18100000-0000-0000-0000-000000000013';
        ALTER TABLE rounds ENABLE TRIGGER rounds_protect_final_score_embargo;
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(MIGRATION_18).execute(&pool).await.unwrap();
    let rows = sqlx::query_as::<_, (Uuid, bool)>(
        "SELECT id, final_round_back_nine_hidden FROM tournaments
         WHERE id::text LIKE '18100000-%' ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows.into_iter().map(|row| row.1).collect::<Vec<_>>(),
        vec![false, false, true, true, true, true]
    );
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS (
               SELECT 1 FROM information_schema.columns
               WHERE table_name = 'rounds' AND column_name = 'final_scores_hidden_until'
             )",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    assert!(
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT to_regprocedure('maintain_final_score_embargo()')::text",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
        .is_none()
    );
}
