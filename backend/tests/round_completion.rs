#![cfg(feature = "database-tests")]

use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    domain::{round_completion::TransitionBlocker, scorecards::ScoreOwner},
    repositories::{auth, round_completion, round_completion::RoundCompletionError, scorecards},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::broadcast::error::TryRecvError;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const USER_ID: Uuid = uuid!("40000000-0000-0000-0000-000000000001");
const TOURNAMENT_ID: Uuid = uuid!("40000000-0000-0000-0000-000000000002");
const INDIVIDUAL_ROUND: Uuid = uuid!("40000000-0000-0000-0000-000000000003");
const SCRAMBLE_ROUND: Uuid = uuid!("40000000-0000-0000-0000-000000000004");
const PLAYER_A: Uuid = uuid!("40000000-0000-0000-0000-000000000011");
const PLAYER_B: Uuid = uuid!("40000000-0000-0000-0000-000000000012");
const SCRAMBLE_TEAM: Uuid = uuid!("40000000-0000-0000-0000-000000000022");
const HOLE_1: Uuid = uuid!("40000000-0000-0000-0000-000000000031");
const HOLE_2: Uuid = uuid!("40000000-0000-0000-0000-000000000032");
const SESSION_TOKEN: &str = "round-completion-admin-token";

const FIXTURE: &str = r#"
INSERT INTO users (id, username, display_name, role)
VALUES ('40000000-0000-0000-0000-000000000001', 'completion_admin', 'Admin', 'admin');
INSERT INTO players (id, display_name, current_handicap_index) VALUES
('40000000-0000-0000-0000-000000000011', 'Ada', 4.0),
('40000000-0000-0000-0000-000000000012', 'Bea', 12.0);
INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds, status)
VALUES ('40000000-0000-0000-0000-000000000002', 'Completion Cup', '2026-09-01', '2026-09-02', 2, 'active');
INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap) VALUES
('40000000-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000011', 4.0),
('40000000-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000012', 12.0);
INSERT INTO courses (id, name) VALUES ('40000000-0000-0000-0000-000000000041', 'Completion Course');
INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
VALUES ('40000000-0000-0000-0000-000000000042', '40000000-0000-0000-0000-000000000041', 'Test', 113, 8.0);
INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES
('40000000-0000-0000-0000-000000000031', '40000000-0000-0000-0000-000000000042', 1, 4, 1),
('40000000-0000-0000-0000-000000000032', '40000000-0000-0000-0000-000000000042', 2, 4, 2);
INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format) VALUES
('40000000-0000-0000-0000-000000000003', '40000000-0000-0000-0000-000000000002', 1, 'Individual', '2026-09-01', '40000000-0000-0000-0000-000000000041', 'Completion Course', '40000000-0000-0000-0000-000000000042', 'Test', 2, 'individual_stroke_play'),
('40000000-0000-0000-0000-000000000004', '40000000-0000-0000-0000-000000000002', 2, 'Scramble', '2026-09-02', '40000000-0000-0000-0000-000000000041', 'Completion Course', '40000000-0000-0000-0000-000000000042', 'Test', 2, 'team_scramble');
INSERT INTO teams (id, round_id, tournament_id, name) VALUES
('40000000-0000-0000-0000-000000000021', '40000000-0000-0000-0000-000000000003', '40000000-0000-0000-0000-000000000002', 'Group 1'),
('40000000-0000-0000-0000-000000000022', '40000000-0000-0000-0000-000000000004', '40000000-0000-0000-0000-000000000002', 'Scramble 1');
INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id) VALUES
('40000000-0000-0000-0000-000000000021', '40000000-0000-0000-0000-000000000003', '40000000-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000011'),
('40000000-0000-0000-0000-000000000021', '40000000-0000-0000-0000-000000000003', '40000000-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000012'),
('40000000-0000-0000-0000-000000000022', '40000000-0000-0000-0000-000000000004', '40000000-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000011'),
('40000000-0000-0000-0000-000000000022', '40000000-0000-0000-0000-000000000004', '40000000-0000-0000-0000-000000000002', '40000000-0000-0000-0000-000000000012');
"#;

async fn seed_open(pool: &PgPool) {
    sqlx::raw_sql(FIXTURE).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin')",
    )
    .bind(TOURNAMENT_ID)
    .bind(USER_ID)
    .execute(pool)
    .await
    .unwrap();
    auth::create_session(
        pool,
        USER_ID,
        &hash_session_token(SESSION_TOKEN),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap();
    for round_id in [INDIVIDUAL_ROUND, SCRAMBLE_ROUND] {
        let mut transaction = pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
            .bind(round_id)
            .fetch_one(&mut *transaction)
            .await
            .unwrap();
        sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
            .bind(round_id)
            .execute(&mut *transaction)
            .await
            .unwrap();
        for (player_id, handicap) in [(PLAYER_A, 4_i16), (PLAYER_B, 12_i16)] {
            sqlx::query("INSERT INTO round_handicap_snapshots (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap) VALUES ($1, $2, $3, $4, $4, $4)")
                .bind(round_id).bind(TOURNAMENT_ID).bind(player_id).bind(handicap)
                .execute(&mut *transaction).await.unwrap();
        }
        sqlx::query("UPDATE rounds SET status = 'open' WHERE id = $1")
            .bind(round_id)
            .execute(&mut *transaction)
            .await
            .unwrap();
        transaction.commit().await.unwrap();
    }
}

fn authorize(builder: axum::http::request::Builder) -> axum::http::request::Builder {
    builder
        .header("cookie", format!("golf_session={SESSION_TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(SESSION_TOKEN))
}

async fn fill_and_confirm(pool: &PgPool, round_id: Uuid, owner: ScoreOwner) {
    for (hole_id, strokes) in [(HOLE_1, 4_i16), (HOLE_2, 5_i16)] {
        scorecards::save(
            pool,
            scorecards::SaveScore {
                round_id,
                hole_id,
                owner,
                gross_strokes: strokes,
                submitted_by: USER_ID,
            },
        )
        .await
        .unwrap();
    }
    scorecards::confirm(pool, round_id, owner, USER_ID)
        .await
        .unwrap();
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn validation_is_deterministic_for_individual_and_scramble_rounds(pool: PgPool) {
    seed_open(&pool).await;
    scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: INDIVIDUAL_ROUND,
            hole_id: HOLE_1,
            owner: ScoreOwner::Player { id: PLAYER_B },
            gross_strokes: 4,
            submitted_by: USER_ID,
        },
    )
    .await
    .unwrap();

    let individual = round_completion::validation(&pool, INDIVIDUAL_ROUND)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(individual.owners.len(), 2);
    assert_eq!(individual.owners[0].owner_name, "Ada");
    assert_eq!(individual.owners[1].owner_name, "Bea");
    assert_eq!(individual.owners[1].holes_scored, 1);
    assert!(!individual.ready_to_complete);

    let scramble = round_completion::validation(&pool, SCRAMBLE_ROUND)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(scramble.owners.len(), 1);
    assert_eq!(
        scramble.owners[0].owner,
        ScoreOwner::Team { id: SCRAMBLE_TEAM }
    );
    assert_eq!(scramble.owners[0].holes_scored, 0);
    assert!(!scramble.ready_to_complete);

    let draft_round = Uuid::new_v4();
    sqlx::query("INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format) VALUES ($1, $2, 3, 'Empty draft', '2026-09-03', '40000000-0000-0000-0000-000000000041', 'Completion Course', '40000000-0000-0000-0000-000000000042', 'Test', 2, 'individual_stroke_play')")
        .bind(draft_round).bind(TOURNAMENT_ID).execute(&pool).await.unwrap();
    let draft = round_completion::validation(&pool, draft_round)
        .await
        .unwrap()
        .unwrap();
    assert!(draft.owners.is_empty());
    assert!(!draft.ready_to_complete && !draft.ready_to_lock);
}

#[sqlx::test(migrations = "../migrations")]
async fn api_completes_and_locks_once_with_sse_after_each_commit(pool: PgPool) {
    seed_open(&pool).await;
    fill_and_confirm(&pool, INDIVIDUAL_ROUND, ScoreOwner::Player { id: PLAYER_A }).await;
    fill_and_confirm(&pool, INDIVIDUAL_ROUND, ScoreOwner::Player { id: PLAYER_B }).await;
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));

    let validation = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/rounds/{INDIVIDUAL_ROUND}/completion-validation"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(validation.status(), StatusCode::OK);
    assert_eq!(response_json(validation).await["ready_to_complete"], true);

    let mut events = state.live_events.subscribe();
    let completed = app
        .clone()
        .oneshot(
            authorize(Request::post(format!(
                "/api/rounds/{INDIVIDUAL_ROUND}/complete"
            )))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(completed.status(), StatusCode::OK);
    assert_eq!(response_json(completed).await["status"], "completed");
    assert_eq!(events.try_recv().unwrap().id, INDIVIDUAL_ROUND);

    let completed_validation = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/rounds/{INDIVIDUAL_ROUND}/completion-validation"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let completed_body = response_json(completed_validation).await;
    assert_eq!(completed_body["status"], "completed");
    assert_eq!(completed_body["ready_to_lock"], true);

    let mut rejected_events = state.live_events.subscribe();
    let repeated = app
        .clone()
        .oneshot(
            authorize(Request::post(format!(
                "/api/rounds/{INDIVIDUAL_ROUND}/complete"
            )))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(repeated.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(repeated).await,
        json!({"error": {"code": "round_not_open", "message": "round must be open to complete"}})
    );
    assert!(matches!(
        rejected_events.try_recv(),
        Err(TryRecvError::Empty)
    ));

    let locked = app
        .clone()
        .oneshot(
            authorize(Request::post(format!(
                "/api/rounds/{INDIVIDUAL_ROUND}/lock"
            )))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(locked.status(), StatusCode::OK);
    assert_eq!(response_json(locked).await["status"], "locked");
    assert_eq!(events.try_recv().unwrap().id, INDIVIDUAL_ROUND);

    let locked_validation = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/rounds/{INDIVIDUAL_ROUND}/completion-validation"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(locked_validation.status(), StatusCode::OK);
    let body = response_json(locked_validation).await;
    assert_eq!(body["status"], "locked");
    assert_eq!(body["ready_to_complete"], false);
    assert_eq!(body["ready_to_lock"], false);

    let repeated_lock = app
        .clone()
        .oneshot(
            authorize(Request::post(format!(
                "/api/rounds/{INDIVIDUAL_ROUND}/lock"
            )))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(repeated_lock.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(repeated_lock).await,
        json!({"error": {"code": "round_not_completed", "message": "round must be completed to lock"}})
    );

    let missing_id = Uuid::new_v4();
    let missing = app
        .clone()
        .oneshot(
            Request::get(format!("/api/rounds/{}/completion-validation", missing_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response_json(missing).await,
        json!({"error": {"code": "not_found", "message": "resource not found"}})
    );
    for path in ["complete", "lock"] {
        let missing_post = app
            .clone()
            .oneshot(
                authorize(Request::post(format!("/api/rounds/{missing_id}/{path}")))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing_post.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response_json(missing_post).await,
            json!({"error": {"code": "not_found", "message": "resource not found"}})
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn scramble_completion_and_source_state_conflicts_are_enforced(pool: PgPool) {
    seed_open(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let wrong_state = app
        .clone()
        .oneshot(
            authorize(Request::post(format!("/api/rounds/{SCRAMBLE_ROUND}/lock")))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(wrong_state.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(wrong_state).await,
        json!({"error": {"code": "round_not_completed", "message": "round must be completed to lock"}})
    );
    let incomplete_api = app
        .oneshot(
            authorize(Request::post(format!(
                "/api/rounds/{SCRAMBLE_ROUND}/complete"
            )))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(incomplete_api.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(incomplete_api).await,
        json!({"error": {"code": "round_scorecards_incomplete", "message": "one or more scorecards are incomplete"}})
    );
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
    let premature_lock = round_completion::lock(&pool, SCRAMBLE_ROUND).await;
    assert!(matches!(
        premature_lock,
        Err(RoundCompletionError::Blocked {
            blocker: TransitionBlocker::InvalidSourceState,
            ..
        })
    ));
    let incomplete = round_completion::complete(&pool, SCRAMBLE_ROUND).await;
    assert!(matches!(
        incomplete,
        Err(RoundCompletionError::Blocked {
            blocker: TransitionBlocker::IncompleteScorecards,
            ..
        })
    ));

    fill_and_confirm(
        &pool,
        SCRAMBLE_ROUND,
        ScoreOwner::Team { id: SCRAMBLE_TEAM },
    )
    .await;
    assert_eq!(
        round_completion::complete(&pool, SCRAMBLE_ROUND)
            .await
            .unwrap()
            .status,
        golf_api::domain::models::RoundStatus::Completed
    );
    assert_eq!(
        round_completion::lock(&pool, SCRAMBLE_ROUND)
            .await
            .unwrap()
            .status,
        golf_api::domain::models::RoundStatus::Locked
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn completed_correction_requires_reconfirmation_before_lock(pool: PgPool) {
    seed_open(&pool).await;
    for player_id in [PLAYER_A, PLAYER_B] {
        fill_and_confirm(
            &pool,
            INDIVIDUAL_ROUND,
            ScoreOwner::Player { id: player_id },
        )
        .await;
    }
    round_completion::complete(&pool, INDIVIDUAL_ROUND)
        .await
        .unwrap();
    scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: INDIVIDUAL_ROUND,
            hole_id: HOLE_1,
            owner: ScoreOwner::Player { id: PLAYER_A },
            gross_strokes: 6,
            submitted_by: USER_ID,
        },
    )
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let blocked = api::router(state)
        .oneshot(
            authorize(Request::post(format!(
                "/api/rounds/{INDIVIDUAL_ROUND}/lock"
            )))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(blocked).await,
        json!({"error": {"code": "round_scorecards_unconfirmed", "message": "one or more scorecards are unconfirmed"}})
    );
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
    scorecards::confirm(
        &pool,
        INDIVIDUAL_ROUND,
        ScoreOwner::Player { id: PLAYER_A },
        USER_ID,
    )
    .await
    .unwrap();
    round_completion::lock(&pool, INDIVIDUAL_ROUND)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn direct_sql_cannot_bypass_completion_or_lock_workflows(pool: PgPool) {
    seed_open(&pool).await;
    let direct = sqlx::query("UPDATE rounds SET status = 'completed' WHERE id = $1")
        .bind(INDIVIDUAL_ROUND)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        direct
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("round_completion_context_required")
    );

    let mut contextual = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(INDIVIDUAL_ROUND)
        .fetch_one(&mut *contextual)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.round_completion_id', $1::text, true)")
        .bind(INDIVIDUAL_ROUND)
        .execute(&mut *contextual)
        .await
        .unwrap();
    let incomplete = sqlx::query("UPDATE rounds SET status = 'completed' WHERE id = $1")
        .bind(INDIVIDUAL_ROUND)
        .execute(&mut *contextual)
        .await
        .unwrap_err();
    assert_eq!(
        incomplete
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("round_scorecards_not_ready")
    );
    contextual.rollback().await.unwrap();

    for player_id in [PLAYER_A, PLAYER_B] {
        fill_and_confirm(
            &pool,
            INDIVIDUAL_ROUND,
            ScoreOwner::Player { id: player_id },
        )
        .await;
    }
    round_completion::complete(&pool, INDIVIDUAL_ROUND)
        .await
        .unwrap();
    let direct_lock = sqlx::query("UPDATE rounds SET status = 'locked' WHERE id = $1")
        .bind(INDIVIDUAL_ROUND)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        direct_lock
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("round_lock_context_required")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn completion_and_lock_serialize_with_each_other_and_score_corrections(pool: PgPool) {
    seed_open(&pool).await;
    for player_id in [PLAYER_A, PLAYER_B] {
        fill_and_confirm(
            &pool,
            INDIVIDUAL_ROUND,
            ScoreOwner::Player { id: player_id },
        )
        .await;
    }
    let first_pool = pool.clone();
    let second_pool = pool.clone();
    let (first, second) = tokio::join!(
        async move { round_completion::complete(&first_pool, INDIVIDUAL_ROUND).await },
        async move { round_completion::complete(&second_pool, INDIVIDUAL_ROUND).await }
    );
    assert_eq!(
        [&first, &second]
            .iter()
            .filter(|result| result.is_ok())
            .count(),
        1
    );

    let mut correction = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(INDIVIDUAL_ROUND)
        .fetch_one(&mut *correction)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(INDIVIDUAL_ROUND)
        .execute(&mut *correction)
        .await
        .unwrap();
    sqlx::query("UPDATE scores SET gross_strokes = 7 WHERE round_id = $1 AND player_id = $2 AND hole_id = $3")
        .bind(INDIVIDUAL_ROUND).bind(PLAYER_A).bind(HOLE_1)
        .execute(&mut *correction).await.unwrap();
    let lock_pool = pool.clone();
    let mut locking =
        tokio::spawn(async move { round_completion::lock(&lock_pool, INDIVIDUAL_ROUND).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut locking)
            .await
            .is_err()
    );
    correction.commit().await.unwrap();
    assert!(matches!(
        locking.await.unwrap(),
        Err(RoundCompletionError::Blocked {
            blocker: TransitionBlocker::UnconfirmedScorecards,
            ..
        })
    ));
}
