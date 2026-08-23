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
    domain::models::ReadinessIssueCode,
    repositories::{auth, round_lifecycle, round_lifecycle::OpenRoundError},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::{PgPool, Row};
use tokio::sync::broadcast::error::TryRecvError;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT_ID: Uuid = uuid!("20000000-0000-0000-0000-000000000001");
const TEE_ID: Uuid = uuid!("20000000-0000-0000-0000-000000000003");
const ROUND_ID: Uuid = uuid!("20000000-0000-0000-0000-000000000004");
const TEAM_ID: Uuid = uuid!("20000000-0000-0000-0000-000000000005");
const FLIGHT_ID: Uuid = uuid!("20000000-0000-0000-0000-000000000006");
const PLAYER_A: Uuid = uuid!("20000000-0000-0000-0000-000000000011");
const PLAYER_B: Uuid = uuid!("20000000-0000-0000-0000-000000000012");
const PLAYER_C: Uuid = uuid!("20000000-0000-0000-0000-000000000013");
const PLAYER_D: Uuid = uuid!("20000000-0000-0000-0000-000000000014");
const ADMIN_ID: Uuid = uuid!("20000000-0000-0000-0000-000000000099");
const SESSION_TOKEN: &str = "round-lifecycle-admin-token";

const READY_FIXTURE: &str = r#"
INSERT INTO players (id, display_name, current_handicap_index) VALUES
('20000000-0000-0000-0000-000000000011', 'Ada', 0.5),
('20000000-0000-0000-0000-000000000012', 'Bjorn', -0.5),
('20000000-0000-0000-0000-000000000013', 'Clara', 9.6);
INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds, status)
VALUES ('20000000-0000-0000-0000-000000000001', 'Lifecycle Cup', '2026-08-01', '2026-08-02', 2, 'active');
INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap) VALUES
('20000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000011', 0.5),
('20000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000012', -0.5),
('20000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000013', 9.6);
INSERT INTO courses (id, name) VALUES
('20000000-0000-0000-0000-000000000002', 'Lifecycle Links');
INSERT INTO tees (id, course_id, name, slope_rating, course_rating) VALUES
('20000000-0000-0000-0000-000000000003', '20000000-0000-0000-0000-000000000002', 'Test', 113, 8.0);
INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES
('20000000-0000-0000-0000-000000000021', '20000000-0000-0000-0000-000000000003', 1, 4, 1),
('20000000-0000-0000-0000-000000000022', '20000000-0000-0000-0000-000000000003', 2, 4, 2);
INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, handicap_allowance_percent, scoring_format)
VALUES ('20000000-0000-0000-0000-000000000004', '20000000-0000-0000-0000-000000000001', 1, 'Round 1', '2026-08-01', '20000000-0000-0000-0000-000000000002', 'Lifecycle Links', '20000000-0000-0000-0000-000000000003', 'Test', 2, 95, 'individual_stroke_play');
INSERT INTO flights (id, round_id, tournament_id, name)
VALUES ('20000000-0000-0000-0000-000000000006', '20000000-0000-0000-0000-000000000004', '20000000-0000-0000-0000-000000000001', 'Flight 1');
INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id, display_order) VALUES
('20000000-0000-0000-0000-000000000006', '20000000-0000-0000-0000-000000000004', '20000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000011', 1),
('20000000-0000-0000-0000-000000000006', '20000000-0000-0000-0000-000000000004', '20000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000012', 2),
('20000000-0000-0000-0000-000000000006', '20000000-0000-0000-0000-000000000004', '20000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000013', 3);
"#;

async fn seed_ready(pool: &PgPool) {
    sqlx::raw_sql(READY_FIXTURE).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, display_name, role)
         VALUES ($1, 'lifecycle_admin', 'Lifecycle Admin', 'admin')
         ON CONFLICT (id) DO NOTHING",
    )
    .bind(ADMIN_ID)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin') ON CONFLICT DO NOTHING",
    )
    .bind(TOURNAMENT_ID)
    .bind(ADMIN_ID)
    .execute(pool)
    .await
    .unwrap();
    let token_hash = hash_session_token(SESSION_TOKEN);
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM user_sessions WHERE token_hash = $1)",
    )
    .bind(token_hash.as_slice())
    .fetch_one(pool)
    .await
    .unwrap();
    if !exists {
        auth::create_session(
            pool,
            ADMIN_ID,
            &token_hash,
            Utc::now() + ChronoDuration::hours(1),
        )
        .await
        .unwrap();
    }
}

fn authorize(builder: axum::http::request::Builder) -> axum::http::request::Builder {
    builder
        .header("cookie", format!("golf_session={SESSION_TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(SESSION_TOKEN))
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn issue_codes(
    validation: &golf_api::domain::models::PairingValidation,
) -> Vec<ReadinessIssueCode> {
    validation.issues.iter().map(|issue| issue.code).collect()
}

#[sqlx::test(migrations = "../migrations")]
async fn api_opens_once_with_exact_snapshots_and_emits_only_after_success(pool: PgPool) {
    seed_ready(&pool).await;
    sqlx::query("UPDATE players SET current_handicap_index = 2.0 WHERE id = $1")
        .bind(PLAYER_C)
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));

    let missing = app
        .clone()
        .oneshot(
            authorize(Request::post(format!(
                "/api/rounds/{}/open",
                Uuid::new_v4()
            )))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        response_json(missing).await,
        serde_json::json!({"error": {"code": "not_found", "message": "resource not found"}})
    );

    let readiness = app
        .clone()
        .oneshot(
            authorize(Request::get(format!(
                "/api/rounds/{ROUND_ID}/pairing-validation"
            )))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(readiness.status(), StatusCode::OK);
    assert_eq!(response_json(readiness).await["ready"], true);

    let mut success_events = state.live_events.subscribe();
    let opened = app
        .clone()
        .oneshot(
            authorize(Request::post(format!("/api/rounds/{ROUND_ID}/open")))
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(opened.status(), StatusCode::OK);
    let body = response_json(opened).await;
    assert_eq!(body["round"]["status"], "open");
    assert_eq!(body["handicap_snapshots"].as_array().unwrap().len(), 3);
    let event = success_events.try_recv().unwrap();
    assert_eq!(event.resource, "round");
    assert_eq!(event.id, ROUND_ID);

    let frozen_pairing = app
        .clone()
        .oneshot(
            authorize(Request::put(format!("/api/rounds/{ROUND_ID}/pairings")))
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "expected_round_updated_at":"2000-01-01T00:00:00Z",
                        "teams":[], "flights":[], "legacy_conversions":[]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(frozen_pairing.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(frozen_pairing).await["error"]["code"],
        "round_not_draft"
    );

    let snapshots = sqlx::query(
        "SELECT player_id, handicap_index::float8 AS handicap_index, course_handicap, playing_handicap FROM round_handicap_snapshots WHERE round_id = $1 ORDER BY player_id",
    )
    .bind(ROUND_ID)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(snapshots[0].get::<i16, _>("course_handicap"), 1);
    assert_eq!(snapshots[0].get::<i16, _>("playing_handicap"), 0);
    assert_eq!(snapshots[1].get::<i16, _>("course_handicap"), -1);
    assert_eq!(snapshots[1].get::<i16, _>("playing_handicap"), 0);
    assert_eq!(snapshots[2].get::<i16, _>("course_handicap"), 10);
    assert_eq!(snapshots[2].get::<i16, _>("playing_handicap"), 9);

    let mut rollback_events = state.live_events.subscribe();
    let repeated = app
        .oneshot(
            authorize(Request::post(format!("/api/rounds/{ROUND_ID}/open")))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(repeated.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(repeated).await,
        serde_json::json!({"error": {"code": "conflict", "message": "round must be draft"}})
    );
    assert!(matches!(
        rollback_events.try_recv(),
        Err(TryRecvError::Empty)
    ));

    let preserved = sqlx::query_scalar::<_, f64>(
        "SELECT handicap_index::float8 FROM round_handicap_snapshots WHERE round_id = $1 AND player_id = $2",
    )
    .bind(ROUND_ID)
    .bind(PLAYER_C)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(preserved, 9.6);
}

#[sqlx::test(migrations = "../migrations")]
async fn readiness_reports_pairing_eligibility_and_format_rules(pool: PgPool) {
    seed_ready(&pool).await;
    sqlx::query("DELETE FROM flight_memberships WHERE player_id = $1")
        .bind(PLAYER_A)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE tournament_players SET status = 'withdrawn' WHERE player_id = $1")
        .bind(PLAYER_B)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE players SET active = false WHERE id = $1")
        .bind(PLAYER_C)
        .execute(&pool)
        .await
        .unwrap();
    let validation = round_lifecycle::pairing_validation(&pool, ROUND_ID)
        .await
        .unwrap()
        .unwrap();
    let codes = issue_codes(&validation);
    assert!(codes.contains(&ReadinessIssueCode::MissingFlightAssignment));
    assert!(codes.contains(&ReadinessIssueCode::IneligibleFlightAssignment));
    assert_eq!(validation.missing_flight_players[0].player_id, PLAYER_A);
    assert_eq!(validation.ineligible_flight_players.len(), 2);
    assert!(validation.missing_players.is_empty());
    assert!(validation.ineligible_players.is_empty());
    assert!(validation.team_sizes.is_empty());

    let state = AppState::new(pool.clone());
    let mut rollback_events = state.live_events.subscribe();
    let response = api::router(state)
        .oneshot(
            authorize(Request::post(format!("/api/rounds/{ROUND_ID}/open")))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(response).await,
        serde_json::json!({"error": {"code": "conflict", "message": "round is not ready to open"}})
    );
    assert!(matches!(
        rollback_events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));

    sqlx::query("UPDATE players SET active = true WHERE id = $1")
        .bind(PLAYER_C)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE tournament_players SET status = 'active' WHERE player_id = $1")
        .bind(PLAYER_B)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id) VALUES ($1, $2, $3, $4)")
        .bind(FLIGHT_ID)
        .bind(ROUND_ID)
        .bind(TOURNAMENT_ID)
        .bind(PLAYER_A)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET scoring_format = 'team_scramble'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO teams (id, round_id, tournament_id, name) VALUES ($1, $2, $3, 'Team 1')",
    )
    .bind(TEAM_ID)
    .bind(ROUND_ID)
    .bind(TOURNAMENT_ID)
    .execute(&pool)
    .await
    .unwrap();
    for player_id in [PLAYER_A, PLAYER_B, PLAYER_C] {
        sqlx::query("INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id) VALUES ($1, $2, $3, $4)")
            .bind(TEAM_ID)
            .bind(ROUND_ID)
            .bind(TOURNAMENT_ID)
            .bind(player_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    let scramble = round_lifecycle::pairing_validation(&pool, ROUND_ID)
        .await
        .unwrap()
        .unwrap();
    assert!(issue_codes(&scramble).contains(&ReadinessIssueCode::InvalidScrambleTeamSize));

    sqlx::query("UPDATE rounds SET scoring_format = 'individual_stroke_play'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM teams WHERE id = $1")
        .bind(TEAM_ID)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        round_lifecycle::pairing_validation(&pool, ROUND_ID)
            .await
            .unwrap()
            .unwrap()
            .ready
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn individual_readiness_returns_deterministic_flight_and_legacy_details(pool: PgPool) {
    seed_ready(&pool).await;
    let empty_flight_id = uuid!("20000000-0000-0000-0000-000000000007");
    let alpha_team_id = uuid!("20000000-0000-0000-0000-000000000008");
    let zulu_team_id = uuid!("20000000-0000-0000-0000-000000000009");
    sqlx::query(
        "INSERT INTO flights (id, round_id, tournament_id, name) VALUES ($1, $2, $3, 'Alpha empty')",
    )
    .bind(empty_flight_id)
    .bind(ROUND_ID)
    .bind(TOURNAMENT_ID)
    .execute(&pool)
    .await
    .unwrap();
    for (team_id, name, player_id) in [
        (zulu_team_id, "Zulu legacy", PLAYER_A),
        (alpha_team_id, "Alpha legacy", PLAYER_B),
    ] {
        sqlx::query(
            "INSERT INTO teams (id, round_id, tournament_id, name) VALUES ($1, $2, $3, $4)",
        )
        .bind(team_id)
        .bind(ROUND_ID)
        .bind(TOURNAMENT_ID)
        .bind(name)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id) VALUES ($1, $2, $3, $4)")
            .bind(team_id)
            .bind(ROUND_ID)
            .bind(TOURNAMENT_ID)
            .bind(player_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("DELETE FROM flight_memberships WHERE player_id = $1")
        .bind(PLAYER_C)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE players SET active = false WHERE id = $1")
        .bind(PLAYER_B)
        .execute(&pool)
        .await
        .unwrap();

    let validation = round_lifecycle::pairing_validation(&pool, ROUND_ID)
        .await
        .unwrap()
        .unwrap();
    let codes = issue_codes(&validation);
    assert!(codes.contains(&ReadinessIssueCode::MissingFlightAssignment));
    assert!(codes.contains(&ReadinessIssueCode::IneligibleFlightAssignment));
    assert!(codes.contains(&ReadinessIssueCode::EmptyFlight));
    assert!(codes.contains(&ReadinessIssueCode::LegacyIndividualGroupsPresent));
    assert_eq!(validation.missing_flight_players[0].player_id, PLAYER_C);
    assert_eq!(validation.ineligible_flight_players[0].player_id, PLAYER_B);
    assert_eq!(
        validation
            .flight_sizes
            .iter()
            .map(|flight| (flight.flight_name.as_str(), flight.player_count))
            .collect::<Vec<_>>(),
        vec![("Alpha empty", 0), ("Flight 1", 2)]
    );
    assert_eq!(
        validation
            .legacy_individual_groups
            .iter()
            .map(|team| (team.team_name.as_str(), team.player_count))
            .collect::<Vec<_>>(),
        vec![("Alpha legacy", 1), ("Zulu legacy", 1)]
    );
    assert!(validation.missing_players.is_empty());
    assert!(validation.ineligible_players.is_empty());
    assert_eq!(validation.team_sizes, validation.legacy_individual_groups);
    assert!(validation.split_teams.is_empty());

    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let response = api::router(state)
        .oneshot(
            authorize(Request::post(format!("/api/rounds/{ROUND_ID}/open")))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM round_handicap_snapshots WHERE round_id = $1",
        )
        .bind(ROUND_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn scramble_allows_multiple_teams_per_flight_but_reports_split_teams(pool: PgPool) {
    seed_ready(&pool).await;
    sqlx::query(
        "INSERT INTO players (id, display_name, current_handicap_index) VALUES ($1, 'Dag', 4.2)",
    )
    .bind(PLAYER_D)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap) VALUES ($1, $2, 4.2)")
        .bind(TOURNAMENT_ID)
        .bind(PLAYER_D)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id) VALUES ($1, $2, $3, $4)")
        .bind(FLIGHT_ID)
        .bind(ROUND_ID)
        .bind(TOURNAMENT_ID)
        .bind(PLAYER_D)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET scoring_format = 'team_scramble' WHERE id = $1")
        .bind(ROUND_ID)
        .execute(&pool)
        .await
        .unwrap();
    let team_two_id = uuid!("20000000-0000-0000-0000-000000000008");
    for (team_id, name, members) in [
        (TEAM_ID, "Alpha", [PLAYER_A, PLAYER_B]),
        (team_two_id, "Bravo", [PLAYER_C, PLAYER_D]),
    ] {
        sqlx::query(
            "INSERT INTO teams (id, round_id, tournament_id, name) VALUES ($1, $2, $3, $4)",
        )
        .bind(team_id)
        .bind(ROUND_ID)
        .bind(TOURNAMENT_ID)
        .bind(name)
        .execute(&pool)
        .await
        .unwrap();
        for player_id in members {
            sqlx::query("INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id) VALUES ($1, $2, $3, $4)")
                .bind(team_id)
                .bind(ROUND_ID)
                .bind(TOURNAMENT_ID)
                .bind(player_id)
                .execute(&pool)
                .await
                .unwrap();
        }
    }
    assert!(
        round_lifecycle::pairing_validation(&pool, ROUND_ID)
            .await
            .unwrap()
            .unwrap()
            .ready
    );

    let second_flight_id = uuid!("20000000-0000-0000-0000-000000000007");
    sqlx::query(
        "INSERT INTO flights (id, round_id, tournament_id, name) VALUES ($1, $2, $3, 'Flight 2')",
    )
    .bind(second_flight_id)
    .bind(ROUND_ID)
    .bind(TOURNAMENT_ID)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE flight_memberships SET flight_id = $1 WHERE round_id = $2 AND player_id = $3",
    )
    .bind(second_flight_id)
    .bind(ROUND_ID)
    .bind(PLAYER_B)
    .execute(&pool)
    .await
    .unwrap();
    let split = round_lifecycle::pairing_validation(&pool, ROUND_ID)
        .await
        .unwrap()
        .unwrap();
    assert!(issue_codes(&split).contains(&ReadinessIssueCode::TeamSplitAcrossFlights));
    assert_eq!(split.split_teams.len(), 1);
    assert_eq!(split.split_teams[0].team_id, TEAM_ID);
    assert_eq!(split.split_teams[0].player_count, 2);
}

#[sqlx::test(migrations = "../migrations")]
async fn readiness_rejects_tournament_and_course_configuration_errors(pool: PgPool) {
    seed_ready(&pool).await;
    sqlx::query("UPDATE tournaments SET status = 'completed' WHERE id = $1")
        .bind(TOURNAMENT_ID)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE tees SET slope_rating = NULL, course_rating = NULL WHERE id = $1")
        .bind(TEE_ID)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE holes SET hole_number = hole_number + 10, stroke_index = stroke_index + 10",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE holes SET hole_number = hole_number - 9, stroke_index = stroke_index - 9")
        .execute(&pool)
        .await
        .unwrap();
    let validation = round_lifecycle::pairing_validation(&pool, ROUND_ID)
        .await
        .unwrap()
        .unwrap();
    let codes = issue_codes(&validation);
    assert!(codes.contains(&ReadinessIssueCode::TournamentNotOpenable));
    assert!(codes.contains(&ReadinessIssueCode::MissingHandicapRatings));
    assert!(codes.contains(&ReadinessIssueCode::InvalidHoleNumbers));
    assert!(codes.contains(&ReadinessIssueCode::InvalidStrokeIndexes));

    sqlx::query("DELETE FROM holes WHERE hole_number = 3")
        .execute(&pool)
        .await
        .unwrap();
    let gapped = round_lifecycle::pairing_validation(&pool, ROUND_ID)
        .await
        .unwrap()
        .unwrap();
    assert!(issue_codes(&gapped).contains(&ReadinessIssueCode::InvalidHoleCount));
}

#[sqlx::test(migrations = "../migrations")]
async fn course_and_tee_pair_must_match_during_round_creation(pool: PgPool) {
    seed_ready(&pool).await;
    let other_course = Uuid::new_v4();
    sqlx::query("INSERT INTO courses (id, name) VALUES ($1, 'Other')")
        .bind(other_course)
        .execute(&pool)
        .await
        .unwrap();
    let error = sqlx::query("UPDATE rounds SET course_id = $1 WHERE id = $2")
        .bind(other_course)
        .bind(ROUND_ID)
        .execute(&pool)
        .await
        .unwrap_err();
    assert!(
        error
            .as_database_error()
            .is_some_and(|database| database.is_foreign_key_violation())
    );

    let response = api::router(AppState::new(pool.clone()))
        .oneshot(
            authorize(Request::post(format!(
                "/api/tournaments/{TOURNAMENT_ID}/rounds"
            )))
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::json!({
                    "round_number": 2,
                    "name": "Second round",
                    "round_date": "2026-08-02",
                    "course_id": "20000000-0000-0000-0000-000000000002",
                    "course_name": "Wrong course",
                    "tee_id": TEE_ID,
                    "tee_name": "White",
                    "number_of_holes": 3,
                    "handicap_enabled": true,
                    "handicap_allowance_percent": 95,
                    "scoring_format": "individual_stroke_play"
                })
                .to_string(),
            ))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(response).await,
        serde_json::json!({"error": {"code": "validation_error", "message": "course and tee identifiers must match their names"}})
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn opening_and_snapshot_capture_require_the_lifecycle_transaction(pool: PgPool) {
    seed_ready(&pool).await;

    let snapshot_error = sqlx::query(
        "INSERT INTO round_handicap_snapshots (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap) VALUES ($1, $2, $3, 0.5, 1, 1)",
    )
    .bind(ROUND_ID)
    .bind(TOURNAMENT_ID)
    .bind(PLAYER_A)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        snapshot_error
            .as_database_error()
            .and_then(|database| database.constraint()),
        Some("round_snapshot_capture_frozen")
    );

    let status_error = sqlx::query("UPDATE rounds SET status = 'open' WHERE id = $1")
        .bind(ROUND_ID)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        status_error
            .as_database_error()
            .and_then(|database| database.constraint()),
        Some("round_opening_context_required")
    );
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status::text FROM rounds WHERE id = $1")
            .bind(ROUND_ID)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "draft"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn opened_round_freezes_configuration_pairings_and_snapshots_but_can_cascade(pool: PgPool) {
    seed_ready(&pool).await;
    round_lifecycle::open(&pool, ROUND_ID).await.unwrap();

    let guarded_commands = [
        "UPDATE rounds SET status = 'draft' WHERE id = '20000000-0000-0000-0000-000000000004'",
        "UPDATE rounds SET handicap_allowance_percent = 90 WHERE id = '20000000-0000-0000-0000-000000000004'",
        "UPDATE tees SET slope_rating = 120 WHERE id = '20000000-0000-0000-0000-000000000003'",
        "UPDATE holes SET par = 5 WHERE id = '20000000-0000-0000-0000-000000000021'",
        "DELETE FROM flight_memberships WHERE flight_id = '20000000-0000-0000-0000-000000000006' AND player_id = '20000000-0000-0000-0000-000000000011'",
        "DELETE FROM flights WHERE id = '20000000-0000-0000-0000-000000000006'",
        "UPDATE round_handicap_snapshots SET playing_handicap = 4 WHERE round_id = '20000000-0000-0000-0000-000000000004' AND player_id = '20000000-0000-0000-0000-000000000011'",
        "DELETE FROM round_handicap_snapshots WHERE round_id = '20000000-0000-0000-0000-000000000004' AND player_id = '20000000-0000-0000-0000-000000000011'",
        "INSERT INTO round_handicap_snapshots (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap) VALUES ('20000000-0000-0000-0000-000000000004', '20000000-0000-0000-0000-000000000001', '20000000-0000-0000-0000-000000000011', 0.5, 1, 0)",
    ];
    for command in guarded_commands {
        let error = sqlx::query(command).execute(&pool).await.unwrap_err();
        assert!(
            error
                .as_database_error()
                .is_some_and(|database| database.is_check_violation())
        );
    }

    let participant_delete =
        sqlx::query("DELETE FROM tournament_players WHERE tournament_id = $1 AND player_id = $2")
            .bind(TOURNAMENT_ID)
            .bind(PLAYER_A)
            .execute(&pool)
            .await
            .unwrap_err();
    assert!(
        participant_delete
            .as_database_error()
            .is_some_and(|database| database.is_foreign_key_violation())
    );

    sqlx::query("DELETE FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT_ID)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM round_handicap_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn direct_round_deletion_cascades_opening_snapshots(pool: PgPool) {
    seed_ready(&pool).await;
    round_lifecycle::open(&pool, ROUND_ID).await.unwrap();

    sqlx::query("DELETE FROM rounds WHERE id = $1")
        .bind(ROUND_ID)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM round_handicap_snapshots WHERE round_id = $1",
        )
        .bind(ROUND_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn tee_and_hole_mutations_serialize_through_the_round_lock(pool: PgPool) {
    seed_ready(&pool).await;
    let mut tee_change = pool.begin().await.unwrap();
    sqlx::query("UPDATE tees SET slope_rating = 120 WHERE id = $1")
        .bind(TEE_ID)
        .execute(&mut *tee_change)
        .await
        .unwrap();
    let opening_pool = pool.clone();
    let mut opening =
        tokio::spawn(async move { round_lifecycle::open(&opening_pool, ROUND_ID).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut opening)
            .await
            .is_err()
    );
    tee_change.commit().await.unwrap();
    assert!(opening.await.unwrap().is_ok());

    sqlx::query("DELETE FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT_ID)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM players WHERE id = ANY($1)")
        .bind(&[PLAYER_A, PLAYER_B, PLAYER_C][..])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM courses WHERE id = '20000000-0000-0000-0000-000000000002'")
        .execute(&pool)
        .await
        .unwrap();
    seed_ready(&pool).await;

    let mut hole_change = pool.begin().await.unwrap();
    sqlx::query("DELETE FROM holes WHERE tee_id = $1 AND hole_number = 1")
        .bind(TEE_ID)
        .execute(&mut *hole_change)
        .await
        .unwrap();
    let opening_pool = pool.clone();
    let mut opening =
        tokio::spawn(async move { round_lifecycle::open(&opening_pool, ROUND_ID).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut opening)
            .await
            .is_err()
    );
    hole_change.commit().await.unwrap();
    assert!(matches!(
        opening.await.unwrap(),
        Err(OpenRoundError::NotReady(_))
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_open_attempts_create_one_snapshot_set(pool: PgPool) {
    seed_ready(&pool).await;
    let first_pool = pool.clone();
    let second_pool = pool.clone();
    let (first, second) = tokio::join!(
        async move { round_lifecycle::open(&first_pool, ROUND_ID).await },
        async move { round_lifecycle::open(&second_pool, ROUND_ID).await }
    );
    let outcomes = [first, second];
    assert_eq!(outcomes.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(
        outcomes
            .iter()
            .filter(|result| matches!(result, Err(OpenRoundError::NotReady(_))))
            .count(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM round_handicap_snapshots WHERE round_id = $1",
        )
        .bind(ROUND_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        3
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn flight_assignment_and_removal_are_serialized_against_opening(pool: PgPool) {
    seed_ready(&pool).await;
    sqlx::query("DELETE FROM flight_memberships WHERE player_id = $1")
        .bind(PLAYER_A)
        .execute(&pool)
        .await
        .unwrap();

    let mut assignment = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id) VALUES ($1, $2, $3, $4)")
        .bind(FLIGHT_ID)
        .bind(ROUND_ID)
        .bind(TOURNAMENT_ID)
        .bind(PLAYER_A)
        .execute(&mut *assignment)
        .await
        .unwrap();
    let opening_pool = pool.clone();
    let mut opening =
        tokio::spawn(async move { round_lifecycle::open(&opening_pool, ROUND_ID).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut opening)
            .await
            .is_err()
    );
    assignment.commit().await.unwrap();
    assert!(opening.await.unwrap().is_ok());

    sqlx::query("DELETE FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT_ID)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM players WHERE id = ANY($1)")
        .bind(&[PLAYER_A, PLAYER_B, PLAYER_C][..])
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM courses WHERE id = '20000000-0000-0000-0000-000000000002'")
        .execute(&pool)
        .await
        .unwrap();
    seed_ready(&pool).await;
    let mut removal = pool.begin().await.unwrap();
    sqlx::query("DELETE FROM flight_memberships WHERE player_id = $1")
        .bind(PLAYER_A)
        .execute(&mut *removal)
        .await
        .unwrap();
    let opening_pool = pool.clone();
    let mut opening =
        tokio::spawn(async move { round_lifecycle::open(&opening_pool, ROUND_ID).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut opening)
            .await
            .is_err()
    );
    removal.commit().await.unwrap();
    assert!(matches!(
        opening.await.unwrap(),
        Err(OpenRoundError::NotReady(_))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM round_handicap_snapshots")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
