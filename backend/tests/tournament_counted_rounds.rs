#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::{auth, round_lifecycle, tournaments},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::broadcast::error::TryRecvError;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("14000000-0000-0000-0000-000000000001");
const ADMIN: Uuid = uuid!("14000000-0000-0000-0000-000000000002");
const VIEWER: Uuid = uuid!("14000000-0000-0000-0000-000000000003");
const ADMIN_TOKEN: &str = "counted-rounds-admin-token";
const VIEWER_TOKEN: &str = "counted-rounds-viewer-token";

const MIGRATIONS_1_TO_13: [&str; 13] = [
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
];
const MIGRATION_14: &str = include_str!("../../migrations/0014_tournament_counted_rounds.sql");

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        "INSERT INTO users (id, username, display_name, role) VALUES
         ('14000000-0000-0000-0000-000000000002', 'count_admin', 'Admin', 'player'),
         ('14000000-0000-0000-0000-000000000003', 'count_viewer', 'Viewer', 'admin');
         INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds, counted_rounds)
         VALUES ('14000000-0000-0000-0000-000000000001', 'Best of',
                 '2026-09-01', '2026-09-03', 3, 3);
         INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
         ('14000000-0000-0000-0000-000000000001',
          '14000000-0000-0000-0000-000000000002', 'admin'),
         ('14000000-0000-0000-0000-000000000001',
          '14000000-0000-0000-0000-000000000003', 'viewer');
         INSERT INTO rounds
           (id, tournament_id, round_number, name, round_date, course_name, tee_name,
            scoring_format)
         VALUES
         ('14000000-0000-0000-0000-000000000011',
          '14000000-0000-0000-0000-000000000001', 1, 'One', '2026-09-01', '', '',
          'individual_stroke_play'),
         ('14000000-0000-0000-0000-000000000012',
          '14000000-0000-0000-0000-000000000001', 2, 'Two', '2026-09-02', '', '',
          'individual_stroke_play'),
         ('14000000-0000-0000-0000-000000000013',
          '14000000-0000-0000-0000-000000000001', 3, 'Three', '2026-09-03', '', '',
          'individual_stroke_play');",
    )
    .execute(pool)
    .await
    .unwrap();
    for (user, token) in [(ADMIN, ADMIN_TOKEN), (VIEWER, VIEWER_TOKEN)] {
        auth::create_session(
            pool,
            user,
            &hash_session_token(token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
    }
}

async fn make_first_round_openable(pool: &PgPool) {
    sqlx::raw_sql(
        "INSERT INTO players (id, display_name, current_handicap_index)
         VALUES ('14000000-0000-0000-0000-000000000020', 'Race player', 8.0);
         INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
         VALUES ('14000000-0000-0000-0000-000000000001',
                 '14000000-0000-0000-0000-000000000020', 8.0);
         INSERT INTO courses (id, name)
         VALUES ('14000000-0000-0000-0000-000000000021', 'Race course');
         INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
         VALUES ('14000000-0000-0000-0000-000000000022',
                 '14000000-0000-0000-0000-000000000021', 'Race tee', 113, 4.0);
         INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
         VALUES ('14000000-0000-0000-0000-000000000023',
                 '14000000-0000-0000-0000-000000000022', 1, 4, 1);
         UPDATE rounds SET
             course_id = '14000000-0000-0000-0000-000000000021',
             course_name = 'Race course',
             tee_id = '14000000-0000-0000-0000-000000000022',
             tee_name = 'Race tee', number_of_holes = 1
         WHERE id = '14000000-0000-0000-0000-000000000011';
         INSERT INTO flights (id, round_id, tournament_id, name)
         VALUES ('14000000-0000-0000-0000-000000000024',
                 '14000000-0000-0000-0000-000000000011',
                 '14000000-0000-0000-0000-000000000001', 'Race flight');
         INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id)
         VALUES ('14000000-0000-0000-0000-000000000024',
                 '14000000-0000-0000-0000-000000000011',
                 '14000000-0000-0000-0000-000000000001',
                 '14000000-0000-0000-0000-000000000020');",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn updated_at(pool: &PgPool) -> chrono::DateTime<Utc> {
    sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn stored_counted_rounds(pool: &PgPool) -> i16 {
    sqlx::query_scalar("SELECT counted_rounds FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn patch(token: &str, counted_rounds: i16, expected: chrono::DateTime<Utc>) -> Request<Body> {
    Request::patch(format!("/api/tournaments/{TOURNAMENT}/counted-rounds"))
        .header(header::COOKIE, format!("golf_session={token}"))
        .header("x-csrf-token", derive_csrf_token(token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "counted_rounds": counted_rounds,
                "expected_tournament_updated_at": expected,
            })
            .to_string(),
        ))
        .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = false)]
async fn migration_backfills_existing_tournaments_to_all_rounds(pool: PgPool) {
    for migration in MIGRATIONS_1_TO_13 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::query(
        "INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
         VALUES ($1, 'Existing', '2026-01-01', '2026-01-04', 4)",
    )
    .bind(TOURNAMENT)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(MIGRATION_14).execute(&pool).await.unwrap();
    let counted: i16 = sqlx::query_scalar("SELECT counted_rounds FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(counted, 4);
    for (id, invalid) in [(Uuid::new_v4(), 0_i16), (Uuid::new_v4(), 5)] {
        let result = sqlx::query(
            "INSERT INTO tournaments
               (id, name, start_date, end_date, number_of_rounds, counted_rounds)
             VALUES ($1, 'Invalid', '2026-01-01', '2026-01-04', 4, $2)",
        )
        .bind(id)
        .bind(invalid)
        .execute(&pool)
        .await;
        assert!(result.is_err());
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn exact_admin_patch_is_authoritative_optimistic_and_emits_once(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;

    let denied = app
        .clone()
        .oneshot(patch(VIEWER_TOKEN, 2, expected))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
    assert_eq!(stored_counted_rounds(&pool).await, 3);

    let no_op = app
        .clone()
        .oneshot(patch(ADMIN_TOKEN, 3, expected))
        .await
        .unwrap();
    assert_eq!(no_op.status(), StatusCode::OK);
    assert_eq!(body(no_op).await["updated_at"], json!(expected));
    assert_eq!(updated_at(&pool).await, expected);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    let success = app
        .clone()
        .oneshot(patch(ADMIN_TOKEN, 2, expected))
        .await
        .unwrap();
    assert_eq!(success.status(), StatusCode::OK);
    assert_eq!(
        success.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let value = body(success).await;
    assert_eq!(value["counted_rounds"], 2);
    let event = events.try_recv().unwrap();
    assert_eq!(event.resource, "tournament");
    assert_eq!(event.id, TOURNAMENT);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    let stale = app.oneshot(patch(ADMIN_TOKEN, 1, expected)).await.unwrap();
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(stale).await["error"]["code"],
        "tournament_configuration_stale"
    );
    assert_eq!(stored_counted_rounds(&pool).await, 2);
}

#[sqlx::test(migrations = "../migrations")]
async fn patch_rejects_range_unknown_fields_and_missing_csrf_without_events(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;

    let invalid = app
        .clone()
        .oneshot(patch(ADMIN_TOKEN, 4, expected))
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);

    let unknown = Request::patch(format!("/api/tournaments/{TOURNAMENT}/counted-rounds"))
        .header(header::COOKIE, format!("golf_session={ADMIN_TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(ADMIN_TOKEN))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "counted_rounds": 2,
                "expected_tournament_updated_at": expected,
                "extra": true,
            })
            .to_string(),
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(unknown).await.unwrap().status(),
        StatusCode::BAD_REQUEST
    );

    let no_csrf = Request::patch(format!("/api/tournaments/{TOURNAMENT}/counted-rounds"))
        .header(header::COOKIE, format!("golf_session={ADMIN_TOKEN}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({
                "counted_rounds": 2,
                "expected_tournament_updated_at": expected,
            })
            .to_string(),
        ))
        .unwrap();
    assert_eq!(
        app.oneshot(no_csrf).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(stored_counted_rounds(&pool).await, 3);
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn database_requires_admin_context_and_permanently_honors_opening_marker(pool: PgPool) {
    seed(&pool).await;
    let direct = sqlx::query("UPDATE tournaments SET counted_rounds = 2 WHERE id = $1")
        .bind(TOURNAMENT)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        direct
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("tournament_configuration_context_required")
    );

    let mut viewer = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT
           set_config('app.tournament_configuration_tournament_id', $1::text, true),
           set_config('app.tournament_configuration_user_id', $2::text, true)",
    )
    .bind(TOURNAMENT)
    .bind(VIEWER)
    .execute(&mut *viewer)
    .await
    .unwrap();
    let denied = sqlx::query("UPDATE tournaments SET counted_rounds = 2 WHERE id = $1")
        .bind(TOURNAMENT)
        .execute(&mut *viewer)
        .await
        .unwrap_err();
    assert_eq!(
        denied
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("tournament_configuration_admin_required")
    );

    let mut admin = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT
           set_config('app.tournament_configuration_tournament_id', $1::text, true),
           set_config('app.tournament_configuration_user_id', $2::text, true)",
    )
    .bind(TOURNAMENT)
    .bind(ADMIN)
    .execute(&mut *admin)
    .await
    .unwrap();
    sqlx::query("UPDATE tournaments SET counted_rounds = 2 WHERE id = $1")
        .bind(TOURNAMENT)
        .execute(&mut *admin)
        .await
        .unwrap();
    admin.commit().await.unwrap();

    sqlx::query(
        "INSERT INTO tournament_handicap_locks (tournament_id, reason)
         VALUES ($1, 'round_opened')",
    )
    .bind(TOURNAMENT)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("DELETE FROM rounds WHERE tournament_id = $1")
        .bind(TOURNAMENT)
        .execute(&pool)
        .await
        .unwrap();

    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let locked = api::router(state)
        .oneshot(patch(ADMIN_TOKEN, 1, updated_at(&pool).await))
        .await
        .unwrap();
    assert_eq!(locked.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(locked).await["error"]["code"],
        "tournament_configuration_locked"
    );
    assert_eq!(stored_counted_rounds(&pool).await, 2);
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn real_opening_and_configuration_race_serialize_before_the_permanent_lock(pool: PgPool) {
    seed(&pool).await;
    make_first_round_openable(&pool).await;
    let expected = updated_at(&pool).await;
    let session_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM user_sessions WHERE token_hash = $1")
            .bind(hash_session_token(ADMIN_TOKEN).as_slice())
            .fetch_one(&pool)
            .await
            .unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));

    let opening_pool = pool.clone();
    let opening_barrier = barrier.clone();
    let opening = tokio::spawn(async move {
        opening_barrier.wait().await;
        round_lifecycle::open_authorized(
            &opening_pool,
            session_id,
            uuid!("14000000-0000-0000-0000-000000000011"),
        )
        .await
    });
    let update_pool = pool.clone();
    let update_barrier = barrier.clone();
    let update = tokio::spawn(async move {
        update_barrier.wait().await;
        tournaments::update_counted_rounds_authorized(
            &update_pool,
            session_id,
            TOURNAMENT,
            2,
            expected,
        )
        .await
    });
    barrier.wait().await;
    opening.await.unwrap().unwrap();
    let update = update.await.unwrap();
    assert!(matches!(
        update,
        Ok(tournaments::UpdateCountedRoundsResult { changed: true, .. })
            | Err(tournaments::TournamentMutationError::ConfigurationLocked)
    ));
    let counted = stored_counted_rounds(&pool).await;
    assert!(counted == 2 || counted == 3);
    let permanently_locked = tournaments::update_counted_rounds_authorized(
        &pool,
        session_id,
        TOURNAMENT,
        if counted == 2 { 1 } else { 2 },
        updated_at(&pool).await,
    )
    .await;
    assert!(matches!(
        permanently_locked,
        Err(tournaments::TournamentMutationError::ConfigurationLocked)
    ));
}
