#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::auth,
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::broadcast::error::TryRecvError;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT_A: Uuid = uuid!("15000000-0000-0000-0000-000000000001");
const TOURNAMENT_B: Uuid = uuid!("15000000-0000-0000-0000-000000000002");
const ADMIN_A: Uuid = uuid!("15000000-0000-0000-0000-000000000011");
const ADMIN_B: Uuid = uuid!("15000000-0000-0000-0000-000000000012");
const ADMIN_A_TOKEN: &str = "tournament-start-admin-a";
const ADMIN_B_TOKEN: &str = "tournament-start-admin-b";

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index) VALUES
        ('15000000-0000-0000-0000-000000000021', 'Entrant A', 8.0),
        ('15000000-0000-0000-0000-000000000022', 'Entrant B', 9.0);
        INSERT INTO users (id, username, display_name, role) VALUES
        ('15000000-0000-0000-0000-000000000011', 'start_admin_a', 'Admin A', 'player'),
        ('15000000-0000-0000-0000-000000000012', 'start_admin_b', 'Admin B', 'admin');
        INSERT INTO tournaments
          (id, name, start_date, end_date, number_of_rounds, counted_rounds)
        VALUES
        ('15000000-0000-0000-0000-000000000001', 'Alpha',
         '2026-09-01', '2026-09-02', 2, 1),
        ('15000000-0000-0000-0000-000000000002', 'Beta',
         '2026-10-01', '2026-10-02', 2, 1);
        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
        ('15000000-0000-0000-0000-000000000001',
         '15000000-0000-0000-0000-000000000011', 'admin'),
        ('15000000-0000-0000-0000-000000000002',
         '15000000-0000-0000-0000-000000000012', 'admin');
        INSERT INTO tournament_players
          (tournament_id, player_id, tournament_handicap)
        VALUES
        ('15000000-0000-0000-0000-000000000001',
         '15000000-0000-0000-0000-000000000021', 8.0),
        ('15000000-0000-0000-0000-000000000002',
         '15000000-0000-0000-0000-000000000022', 9.0);
        -- UUID order deliberately disagrees with round-number order.
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_name, tee_name,
           scoring_format)
        VALUES
        ('15000000-0000-0000-0000-000000000032',
         '15000000-0000-0000-0000-000000000001', 1, 'A1', '2026-09-01', '', '',
         'individual_stroke_play'),
        ('15000000-0000-0000-0000-000000000031',
         '15000000-0000-0000-0000-000000000001', 2, 'A2', '2026-09-02', '', '',
         'individual_stroke_play'),
        ('15000000-0000-0000-0000-000000000041',
         '15000000-0000-0000-0000-000000000002', 1, 'B1', '2026-10-01', '', '',
         'individual_stroke_play'),
        ('15000000-0000-0000-0000-000000000042',
         '15000000-0000-0000-0000-000000000002', 2, 'B2', '2026-10-02', '', '',
         'individual_stroke_play');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    for (user_id, token) in [(ADMIN_A, ADMIN_A_TOKEN), (ADMIN_B, ADMIN_B_TOKEN)] {
        auth::create_session(
            pool,
            user_id,
            &hash_session_token(token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
    }
}

async fn updated_at(pool: &PgPool, tournament_id: Uuid) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id = $1")
        .bind(tournament_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn start_request(
    tournament_id: Uuid,
    token: Option<&str>,
    csrf: bool,
    expected: DateTime<Utc>,
) -> Request<Body> {
    let mut builder = Request::post(format!("/api/tournaments/{tournament_id}/start"))
        .header(header::CONTENT_TYPE, "application/json");
    if let Some(token) = token {
        builder = builder.header(header::COOKIE, format!("golf_session={token}"));
        if csrf {
            builder = builder.header("x-csrf-token", derive_csrf_token(token));
        }
    }
    builder
        .body(Body::from(
            json!({"expected_tournament_updated_at": expected}).to_string(),
        ))
        .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn status(pool: &PgPool, tournament_id: Uuid) -> String {
    sqlx::query_scalar::<_, String>("SELECT status::text FROM tournaments WHERE id = $1")
        .bind(tournament_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn database_rejects_every_non_draft_tournament_insert(pool: PgPool) {
    for status in ["active", "completed", "archived"] {
        let error = sqlx::query(
            "INSERT INTO tournaments
               (id, name, start_date, end_date, number_of_rounds, counted_rounds, status)
             VALUES ($1, $2, '2026-11-01', '2026-11-01', 1, 1,
                     $3::tournament_status)",
        )
        .bind(Uuid::new_v4())
        .bind(format!("Invalid {status}"))
        .bind(status)
        .execute(&pool)
        .await
        .unwrap_err();
        assert_eq!(
            error
                .as_database_error()
                .and_then(|database| database.constraint()),
            Some("tournament_insert_requires_draft")
        );
    }

    sqlx::query(
        "INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds, counted_rounds)
         VALUES ($1, 'Valid draft', '2026-11-01', '2026-11-01', 1, 1)",
    )
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn exact_admin_starts_ready_plan_once_without_changing_rounds(pool: PgPool) {
    seed(&pool).await;
    let expected = updated_at(&pool, TOURNAMENT_A).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    let response = app
        .clone()
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            true,
            expected,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let started = body(response).await;
    assert_eq!(started["status"], "active");
    assert_ne!(started["updated_at"], json!(expected));
    let round_statuses = sqlx::query_scalar::<_, String>(
        "SELECT status::text FROM rounds WHERE tournament_id = $1 ORDER BY round_number",
    )
    .bind(TOURNAMENT_A)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(round_statuses, ["draft", "draft"]);
    let event = events.try_recv().unwrap();
    assert_eq!(event.resource, "tournament");
    assert_eq!(event.id, TOURNAMENT_A);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    let idempotent = app
        .clone()
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            true,
            expected,
        ))
        .await
        .unwrap();
    assert_eq!(idempotent.status(), StatusCode::OK);
    assert_eq!(body(idempotent).await["updated_at"], started["updated_at"]);
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));

    sqlx::query("UPDATE tournaments SET status = 'completed' WHERE id = $1")
        .bind(TOURNAMENT_A)
        .execute(&pool)
        .await
        .unwrap();
    let completed = app
        .clone()
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            true,
            updated_at(&pool, TOURNAMENT_A).await,
        ))
        .await
        .unwrap();
    assert_eq!(completed.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(completed).await["error"]["code"],
        "tournament_start_invalid_state"
    );
    sqlx::query("UPDATE tournaments SET status = 'archived' WHERE id = $1")
        .bind(TOURNAMENT_A)
        .execute(&pool)
        .await
        .unwrap();
    let archived = app
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            true,
            updated_at(&pool, TOURNAMENT_A).await,
        ))
        .await
        .unwrap();
    assert_eq!(archived.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(archived).await["error"]["code"],
        "tournament_start_invalid_state"
    );
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_start_requests_commit_once_and_emit_once(pool: PgPool) {
    seed(&pool).await;
    let expected = updated_at(&pool, TOURNAMENT_A).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));

    let mut requests = Vec::new();
    for _ in 0..2 {
        let app = app.clone();
        let barrier = barrier.clone();
        requests.push(tokio::spawn(async move {
            barrier.wait().await;
            app.oneshot(start_request(
                TOURNAMENT_A,
                Some(ADMIN_A_TOKEN),
                true,
                expected,
            ))
            .await
            .unwrap()
        }));
    }
    barrier.wait().await;
    let responses = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let first = requests.remove(0).await.unwrap();
        let second = requests.remove(0).await.unwrap();
        [first, second]
    })
    .await
    .expect("concurrent starts deadlocked");

    let mut timestamps = Vec::new();
    for response in responses {
        assert_eq!(response.status(), StatusCode::OK);
        let payload = body(response).await;
        assert_eq!(payload["status"], "active");
        timestamps.push(payload["updated_at"].clone());
    }
    assert_eq!(timestamps[0], timestamps[1]);
    assert_ne!(timestamps[0], json!(expected));
    let event = events.try_recv().unwrap();
    assert_eq!(event.resource, "tournament");
    assert_eq!(event.id, TOURNAMENT_A);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}

#[sqlx::test(migrations = "../migrations")]
async fn start_and_counted_round_update_serialize_without_deadlock(pool: PgPool) {
    seed(&pool).await;
    let expected = updated_at(&pool, TOURNAMENT_A).await;
    let session_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM user_sessions WHERE token_hash = $1")
            .bind(hash_session_token(ADMIN_A_TOKEN).as_slice())
            .fetch_one(&pool)
            .await
            .unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));

    let start_pool = pool.clone();
    let start_barrier = barrier.clone();
    let start = tokio::spawn(async move {
        start_barrier.wait().await;
        golf_api::repositories::tournaments::start_authorized(
            &start_pool,
            session_id,
            TOURNAMENT_A,
            expected,
        )
        .await
    });
    let update_pool = pool.clone();
    let update_barrier = barrier.clone();
    let update = tokio::spawn(async move {
        update_barrier.wait().await;
        golf_api::repositories::tournaments::update_counted_rounds_authorized(
            &update_pool,
            session_id,
            TOURNAMENT_A,
            2,
            Some(uuid!("15000000-0000-0000-0000-000000000032")),
            expected,
        )
        .await
    });
    barrier.wait().await;
    let (start, update) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(start, update)
    })
    .await
    .expect("start and counted-round update deadlocked");
    let start = start.unwrap();
    let update = update.unwrap();

    match (start, update) {
        (
            Ok(started),
            Err(golf_api::repositories::tournaments::TournamentMutationError::ConfigurationLocked),
        ) => {
            assert!(started.changed);
        }
        (
            Err(golf_api::repositories::tournaments::TournamentMutationError::StartStale),
            Ok(updated),
        ) => {
            assert!(updated.changed);
        }
        _ => panic!("unexpected start/configuration race result"),
    }
    let stored = sqlx::query_as::<_, (String, i16, Option<Uuid>)>(
        "SELECT status::text, counted_rounds, mandatory_round_id
         FROM tournaments WHERE id = $1",
    )
    .bind(TOURNAMENT_A)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        stored == ("active".to_owned(), 1, None)
            || stored
                == (
                    "draft".to_owned(),
                    2,
                    Some(uuid!("15000000-0000-0000-0000-000000000032")),
                )
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn cross_tournament_global_admin_stale_and_auth_fail_without_mutation(pool: PgPool) {
    seed(&pool).await;
    let expected = updated_at(&pool, TOURNAMENT_A).await;
    let expected_b = updated_at(&pool, TOURNAMENT_B).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    let cross = app
        .clone()
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_B_TOKEN),
            true,
            expected,
        ))
        .await
        .unwrap();
    assert_eq!(cross.status(), StatusCode::FORBIDDEN);

    let reverse_cross = app
        .clone()
        .oneshot(start_request(
            TOURNAMENT_B,
            Some(ADMIN_A_TOKEN),
            true,
            expected_b,
        ))
        .await
        .unwrap();
    assert_eq!(reverse_cross.status(), StatusCode::FORBIDDEN);
    assert_eq!(updated_at(&pool, TOURNAMENT_A).await, expected);
    assert_eq!(updated_at(&pool, TOURNAMENT_B).await, expected_b);

    let stale = app
        .clone()
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            true,
            expected - Duration::seconds(1),
        ))
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_eq!(body(stale).await["error"]["code"], "tournament_start_stale");

    let no_csrf = app
        .clone()
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            false,
            expected,
        ))
        .await
        .unwrap();
    assert_eq!(no_csrf.status(), StatusCode::FORBIDDEN);
    let missing = app
        .clone()
        .oneshot(start_request(TOURNAMENT_A, None, false, expected))
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    sqlx::query(
        "UPDATE user_sessions
         SET expires_at = created_at + interval '1 microsecond'
         WHERE token_hash = $1",
    )
    .bind(hash_session_token(ADMIN_A_TOKEN).as_slice())
    .execute(&pool)
    .await
    .unwrap();
    let expired = app
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            true,
            expected,
        ))
        .await
        .unwrap();
    assert_eq!(expired.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(status(&pool, TOURNAMENT_A).await, "draft");
    assert_eq!(status(&pool, TOURNAMENT_B).await, "draft");
    assert_eq!(updated_at(&pool, TOURNAMENT_A).await, expected);
    assert_eq!(updated_at(&pool, TOURNAMENT_B).await, expected_b);
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn authenticated_admin_gets_not_found_for_missing_tournament_without_mutation(pool: PgPool) {
    seed(&pool).await;
    let expected_a = updated_at(&pool, TOURNAMENT_A).await;
    let expected_b = updated_at(&pool, TOURNAMENT_B).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    let response = app
        .oneshot(start_request(
            Uuid::new_v4(),
            Some(ADMIN_A_TOKEN),
            true,
            Utc::now(),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(status(&pool, TOURNAMENT_A).await, "draft");
    assert_eq!(status(&pool, TOURNAMENT_B).await, "draft");
    assert_eq!(updated_at(&pool, TOURNAMENT_A).await, expected_a);
    assert_eq!(updated_at(&pool, TOURNAMENT_B).await, expected_b);
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn non_contiguous_round_numbers_are_not_ready_without_mutation(pool: PgPool) {
    seed(&pool).await;
    let expected = updated_at(&pool, TOURNAMENT_A).await;
    sqlx::query("UPDATE rounds SET round_number = 3 WHERE tournament_id = $1 AND round_number = 2")
        .bind(TOURNAMENT_A)
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    let response = app
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            true,
            expected,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(response).await["error"]["code"],
        "tournament_start_not_ready"
    );
    assert_eq!(status(&pool, TOURNAMENT_A).await, "draft");
    assert_eq!(updated_at(&pool, TOURNAMENT_A).await, expected);
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn inactive_registered_player_is_not_a_ready_entrant(pool: PgPool) {
    seed(&pool).await;
    let expected = updated_at(&pool, TOURNAMENT_A).await;
    sqlx::query(
        "UPDATE players SET active = false
         WHERE id = '15000000-0000-0000-0000-000000000021'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    let response = app
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            true,
            expected,
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(response).await["error"]["code"],
        "tournament_start_not_ready"
    );
    assert_eq!(status(&pool, TOURNAMENT_A).await, "draft");
    assert_eq!(updated_at(&pool, TOURNAMENT_A).await, expected);
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn incomplete_non_draft_and_entrantless_plans_are_not_ready(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    sqlx::query("DELETE FROM rounds WHERE tournament_id = $1 AND round_number = 2")
        .bind(TOURNAMENT_A)
        .execute(&pool)
        .await
        .unwrap();
    let incomplete = app
        .clone()
        .oneshot(start_request(
            TOURNAMENT_A,
            Some(ADMIN_A_TOKEN),
            true,
            updated_at(&pool, TOURNAMENT_A).await,
        ))
        .await
        .unwrap();
    assert_eq!(incomplete.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(incomplete).await["error"]["code"],
        "tournament_start_not_ready"
    );

    sqlx::query("UPDATE tournament_players SET status = 'withdrawn' WHERE tournament_id = $1")
        .bind(TOURNAMENT_B)
        .execute(&pool)
        .await
        .unwrap();
    let entrantless = app
        .clone()
        .oneshot(start_request(
            TOURNAMENT_B,
            Some(ADMIN_B_TOKEN),
            true,
            updated_at(&pool, TOURNAMENT_B).await,
        ))
        .await
        .unwrap();
    assert_eq!(entrantless.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(entrantless).await["error"]["code"],
        "tournament_start_not_ready"
    );

    sqlx::query("UPDATE tournament_players SET status = 'active' WHERE tournament_id = $1")
        .bind(TOURNAMENT_B)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM rounds WHERE tournament_id = $1 AND round_number = 2")
        .bind(TOURNAMENT_B)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET status = 'open' WHERE tournament_id = $1 AND round_number = 1")
        .bind(TOURNAMENT_B)
        .execute(&pool)
        .await
        .unwrap_err();
    sqlx::query("DELETE FROM rounds WHERE tournament_id = $1")
        .bind(TOURNAMENT_B)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO rounds
           (id, tournament_id, round_number, name, round_date, course_name, tee_name,
            status, scoring_format)
         VALUES ($2, $1, 1, 'Stored open', '2026-10-01', '', '', 'open',
                 'individual_stroke_play'),
                ($3, $1, 2, 'Stored draft', '2026-10-02', '', '', 'draft',
                 'individual_stroke_play')",
    )
    .bind(TOURNAMENT_B)
    .bind(Uuid::new_v4())
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .unwrap();
    let non_draft = app
        .oneshot(start_request(
            TOURNAMENT_B,
            Some(ADMIN_B_TOKEN),
            true,
            updated_at(&pool, TOURNAMENT_B).await,
        ))
        .await
        .unwrap();
    assert_eq!(non_draft.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(non_draft).await["error"]["code"],
        "tournament_start_not_ready"
    );
    assert_eq!(status(&pool, TOURNAMENT_A).await, "draft");
    assert_eq!(status(&pool, TOURNAMENT_B).await, "draft");
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn database_guards_start_context_readiness_and_transition_order(pool: PgPool) {
    seed(&pool).await;

    let mut draft_opening = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
        .bind(uuid!("15000000-0000-0000-0000-000000000032"))
        .execute(&mut *draft_opening)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO round_handicap_snapshots
           (round_id, tournament_id, player_id, handicap_index, course_handicap,
            playing_handicap)
         VALUES ($1, $2, $3, 8.0, 8, 8)",
    )
    .bind(uuid!("15000000-0000-0000-0000-000000000032"))
    .bind(TOURNAMENT_A)
    .bind(uuid!("15000000-0000-0000-0000-000000000021"))
    .execute(&mut *draft_opening)
    .await
    .unwrap();
    let draft_parent = sqlx::query("UPDATE rounds SET status = 'open' WHERE id = $1")
        .bind(uuid!("15000000-0000-0000-0000-000000000032"))
        .execute(&mut *draft_opening)
        .await
        .unwrap_err();
    assert_eq!(
        draft_parent
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("round_opening_tournament_inactive")
    );
    draft_opening.rollback().await.unwrap();

    sqlx::query(
        "INSERT INTO rounds
           (id, tournament_id, round_number, name, round_date, course_name, tee_name,
            status, scoring_format)
         VALUES ($1, $2, 3, 'Stored open', '2026-09-03', '', '', 'open',
                 'individual_stroke_play'),
                ($3, $2, 4, 'Stored completed', '2026-09-04', '', '', 'completed',
                 'individual_stroke_play')",
    )
    .bind(uuid!("15000000-0000-0000-0000-000000000033"))
    .bind(TOURNAMENT_A)
    .bind(uuid!("15000000-0000-0000-0000-000000000034"))
    .execute(&pool)
    .await
    .unwrap();
    let completion = sqlx::query("UPDATE rounds SET status = 'completed' WHERE id = $1")
        .bind(uuid!("15000000-0000-0000-0000-000000000033"))
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        completion
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("round_completion_context_required")
    );
    let locking = sqlx::query("UPDATE rounds SET status = 'locked' WHERE id = $1")
        .bind(uuid!("15000000-0000-0000-0000-000000000034"))
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        locking
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("round_lock_context_required")
    );
    sqlx::query("DELETE FROM rounds WHERE tournament_id = $1 AND round_number > 2")
        .bind(TOURNAMENT_A)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::raw_sql(
        "INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds, counted_rounds)
         VALUES ('15000000-0000-0000-0000-000000000050', 'Foursomes',
                 '2026-11-01', '2026-11-01', 1, 1);
         INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ('15000000-0000-0000-0000-000000000050',
                 '15000000-0000-0000-0000-000000000011', 'admin');
         INSERT INTO tournament_players
           (tournament_id, player_id, tournament_handicap)
         VALUES
         ('15000000-0000-0000-0000-000000000050',
          '15000000-0000-0000-0000-000000000021', 8.0),
         ('15000000-0000-0000-0000-000000000050',
          '15000000-0000-0000-0000-000000000022', 9.0);
         INSERT INTO rounds
           (id, tournament_id, round_number, name, round_date, course_name, tee_name,
            scoring_format, handicap_allowance_percent)
         VALUES ('15000000-0000-0000-0000-000000000051',
                 '15000000-0000-0000-0000-000000000050', 1, 'Foursomes',
                 '2026-11-01', '', '', 'two_player_foursomes', 50);
         INSERT INTO teams (id, round_id, tournament_id, name)
         VALUES ('15000000-0000-0000-0000-000000000052',
                 '15000000-0000-0000-0000-000000000051',
                 '15000000-0000-0000-0000-000000000050', 'Pair');",
    )
    .execute(&pool)
    .await
    .unwrap();
    golf_api::repositories::tournaments::start_authorized(
        &pool,
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM user_sessions WHERE token_hash = $1")
            .bind(hash_session_token(ADMIN_A_TOKEN).as_slice())
            .fetch_one(&pool)
            .await
            .unwrap(),
        uuid!("15000000-0000-0000-0000-000000000050"),
        updated_at(&pool, uuid!("15000000-0000-0000-0000-000000000050")).await,
    )
    .await
    .unwrap();
    let mut foursomes = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
        .bind(uuid!("15000000-0000-0000-0000-000000000051"))
        .execute(&mut *foursomes)
        .await
        .unwrap();
    sqlx::raw_sql(
        "INSERT INTO round_handicap_snapshots
           (round_id, tournament_id, player_id, handicap_index, course_handicap,
            playing_handicap)
         VALUES
         ('15000000-0000-0000-0000-000000000051',
          '15000000-0000-0000-0000-000000000050',
          '15000000-0000-0000-0000-000000000021', 8.0, 8, 8),
         ('15000000-0000-0000-0000-000000000051',
          '15000000-0000-0000-0000-000000000050',
          '15000000-0000-0000-0000-000000000022', 9.0, 9, 9);",
    )
    .execute(&mut *foursomes)
    .await
    .unwrap();
    let missing_team_snapshot = sqlx::query("UPDATE rounds SET status = 'open' WHERE id = $1")
        .bind(uuid!("15000000-0000-0000-0000-000000000051"))
        .execute(&mut *foursomes)
        .await
        .unwrap_err();
    assert_eq!(
        missing_team_snapshot
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("round_opening_team_snapshots_incomplete")
    );
    foursomes.rollback().await.unwrap();

    let direct = sqlx::query("UPDATE tournaments SET status = 'active' WHERE id = $1")
        .bind(TOURNAMENT_A)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        direct
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("tournament_start_context_required")
    );

    let invalid = sqlx::query("UPDATE tournaments SET status = 'completed' WHERE id = $1")
        .bind(TOURNAMENT_A)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        invalid
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("tournament_status_transition_invalid")
    );

    let mut spoofed = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT
           set_config('app.tournament_start_tournament_id', $1::text, true),
           set_config('app.tournament_start_user_id', $2::text, true)",
    )
    .bind(TOURNAMENT_A)
    .bind(ADMIN_B)
    .execute(&mut *spoofed)
    .await
    .unwrap();
    let denied = sqlx::query("UPDATE tournaments SET status = 'active' WHERE id = $1")
        .bind(TOURNAMENT_A)
        .execute(&mut *spoofed)
        .await
        .unwrap_err();
    assert_eq!(
        denied
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("tournament_start_admin_required")
    );

    sqlx::query("UPDATE players SET active = false WHERE id = $1")
        .bind(uuid!("15000000-0000-0000-0000-000000000021"))
        .execute(&pool)
        .await
        .unwrap();
    let mut admin = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT
           set_config('app.tournament_start_tournament_id', $1::text, true),
           set_config('app.tournament_start_user_id', $2::text, true)",
    )
    .bind(TOURNAMENT_A)
    .bind(ADMIN_A)
    .execute(&mut *admin)
    .await
    .unwrap();
    let not_ready = sqlx::query("UPDATE tournaments SET status = 'active' WHERE id = $1")
        .bind(TOURNAMENT_A)
        .execute(&mut *admin)
        .await
        .unwrap_err();
    assert_eq!(
        not_ready
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("tournament_start_entrant_not_ready")
    );
}
