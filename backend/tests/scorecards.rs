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
    domain::scorecards::ScoreOwner,
    repositories::{auth, round_completion, round_lifecycle, scorecards},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::{PgPool, Row};
use tokio::sync::broadcast::error::TryRecvError;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT_ID: Uuid = uuid!("30000000-0000-0000-0000-000000000001");
const INDIVIDUAL_ROUND_ID: Uuid = uuid!("30000000-0000-0000-0000-000000000006");
const SCRAMBLE_ROUND_ID: Uuid = uuid!("30000000-0000-0000-0000-000000000007");
const DRAFT_ROUND_ID: Uuid = uuid!("30000000-0000-0000-0000-000000000008");
const OTHER_ROUND_TEAM_ID: Uuid = uuid!("30000000-0000-0000-0000-000000000009");
const SCRAMBLE_TEAM_ID: Uuid = uuid!("30000000-0000-0000-0000-000000000010");
const PLAYER_A: Uuid = uuid!("30000000-0000-0000-0000-000000000011");
const PLAYER_B: Uuid = uuid!("30000000-0000-0000-0000-000000000012");
const USER_A: Uuid = uuid!("30000000-0000-0000-0000-000000000013");
const USER_B: Uuid = uuid!("30000000-0000-0000-0000-000000000014");
const PLAYER_USER_A: Uuid = uuid!("30000000-0000-0000-0000-000000000015");
const PLAYER_USER_B: Uuid = uuid!("30000000-0000-0000-0000-000000000016");
const PLAYER_USER_C: Uuid = uuid!("30000000-0000-0000-0000-000000000018");
const VIEWER_USER: Uuid = uuid!("30000000-0000-0000-0000-000000000019");
const UNLINKED_PLAYER_USER: Uuid = uuid!("30000000-0000-0000-0000-000000000020");
const HOLE_1: Uuid = uuid!("30000000-0000-0000-0000-000000000021");
const HOLE_2: Uuid = uuid!("30000000-0000-0000-0000-000000000022");

const FIXTURE: &str = r#"
INSERT INTO users (id, username, display_name, role) VALUES
('30000000-0000-0000-0000-000000000013', 'admin_a', 'Admin A', 'admin'),
('30000000-0000-0000-0000-000000000014', 'scorer_b', 'Scorer B', 'scorer'),
('30000000-0000-0000-0000-000000000019', 'viewer', 'Viewer', 'viewer'),
('30000000-0000-0000-0000-000000000020', 'unlinked', 'Unlinked', 'player');
INSERT INTO players (id, display_name, current_handicap_index) VALUES
('30000000-0000-0000-0000-000000000011', 'Ada', 2.0),
('30000000-0000-0000-0000-000000000012', 'Bjorn', 6.0),
('30000000-0000-0000-0000-000000000017', 'Carla', 12.0);
INSERT INTO users (id, username, display_name, role, player_id) VALUES
('30000000-0000-0000-0000-000000000015', 'ada_player', 'Ada', 'player', '30000000-0000-0000-0000-000000000011'),
('30000000-0000-0000-0000-000000000016', 'bjorn_player', 'Bjorn', 'player', '30000000-0000-0000-0000-000000000012'),
('30000000-0000-0000-0000-000000000018', 'carla_player', 'Carla', 'player', '30000000-0000-0000-0000-000000000017');
INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds, status)
VALUES ('30000000-0000-0000-0000-000000000001', 'Score Cup', '2026-08-01', '2026-08-03', 3, 'active');
INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap) VALUES
('30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000011', 2.0),
('30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000012', 6.0);
INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
('30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000013', 'admin'),
('30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000014', 'scorer'),
('30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000015', 'player'),
('30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000016', 'player'),
('30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000018', 'player'),
('30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000019', 'viewer'),
('30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000020', 'player');
INSERT INTO courses (id, name) VALUES
('30000000-0000-0000-0000-000000000002', 'Score Course');
INSERT INTO tees (id, course_id, name, slope_rating, course_rating) VALUES
('30000000-0000-0000-0000-000000000003', '30000000-0000-0000-0000-000000000002', 'Short', 113, 8.0);
INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES
('30000000-0000-0000-0000-000000000021', '30000000-0000-0000-0000-000000000003', 1, 4, 1),
('30000000-0000-0000-0000-000000000022', '30000000-0000-0000-0000-000000000003', 2, 4, 2);
INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format) VALUES
('30000000-0000-0000-0000-000000000006', '30000000-0000-0000-0000-000000000001', 1, 'Individual', '2026-08-01', '30000000-0000-0000-0000-000000000002', 'Score Course', '30000000-0000-0000-0000-000000000003', 'Short', 2, 'individual_stroke_play'),
('30000000-0000-0000-0000-000000000007', '30000000-0000-0000-0000-000000000001', 2, 'Scramble', '2026-08-02', '30000000-0000-0000-0000-000000000002', 'Score Course', '30000000-0000-0000-0000-000000000003', 'Short', 2, 'team_scramble'),
('30000000-0000-0000-0000-000000000008', '30000000-0000-0000-0000-000000000001', 3, 'Draft', '2026-08-03', '30000000-0000-0000-0000-000000000002', 'Score Course', '30000000-0000-0000-0000-000000000003', 'Short', 2, 'individual_stroke_play');
INSERT INTO teams (id, round_id, tournament_id, name) VALUES
('30000000-0000-0000-0000-000000000009', '30000000-0000-0000-0000-000000000008', '30000000-0000-0000-0000-000000000001', 'Other Round Team'),
('30000000-0000-0000-0000-000000000010', '30000000-0000-0000-0000-000000000007', '30000000-0000-0000-0000-000000000001', 'Scramble Team');
INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id, display_order) VALUES
('30000000-0000-0000-0000-000000000009', '30000000-0000-0000-0000-000000000008', '30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000011', 1),
('30000000-0000-0000-0000-000000000010', '30000000-0000-0000-0000-000000000007', '30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000011', 1),
('30000000-0000-0000-0000-000000000010', '30000000-0000-0000-0000-000000000007', '30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000012', 2);
INSERT INTO flights (id, round_id, tournament_id, name) VALUES
('30000000-0000-0000-0000-000000000031', '30000000-0000-0000-0000-000000000006', '30000000-0000-0000-0000-000000000001', 'Individual Flight'),
('30000000-0000-0000-0000-000000000032', '30000000-0000-0000-0000-000000000007', '30000000-0000-0000-0000-000000000001', 'Scramble Flight');
INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id, display_order) VALUES
('30000000-0000-0000-0000-000000000031', '30000000-0000-0000-0000-000000000006', '30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000011', 1),
('30000000-0000-0000-0000-000000000031', '30000000-0000-0000-0000-000000000006', '30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000012', 2),
('30000000-0000-0000-0000-000000000032', '30000000-0000-0000-0000-000000000007', '30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000011', 1),
('30000000-0000-0000-0000-000000000032', '30000000-0000-0000-0000-000000000007', '30000000-0000-0000-0000-000000000001', '30000000-0000-0000-0000-000000000012', 2);
"#;

async fn seed_open(pool: &PgPool) {
    sqlx::raw_sql(FIXTURE).execute(pool).await.unwrap();
    round_lifecycle::open(pool, INDIVIDUAL_ROUND_ID)
        .await
        .unwrap();
    round_lifecycle::open(pool, SCRAMBLE_ROUND_ID)
        .await
        .unwrap();
    for user_id in [
        USER_A,
        USER_B,
        PLAYER_USER_A,
        PLAYER_USER_B,
        PLAYER_USER_C,
        VIEWER_USER,
        UNLINKED_PLAYER_USER,
    ] {
        let token = token_for_user(user_id);
        auth::create_session(
            pool,
            user_id,
            &hash_session_token(token),
            Utc::now() + ChronoDuration::hours(1),
        )
        .await
        .unwrap();
    }
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn save_request(
    round_id: Uuid,
    hole_id: Uuid,
    owner: Value,
    gross: i16,
    user: Uuid,
) -> Request<Body> {
    Request::put(format!("/api/rounds/{round_id}/scores"))
        .header("content-type", "application/json")
        .header("cookie", format!("golf_session={}", token_for_user(user)))
        .header("x-csrf-token", derive_csrf_token(token_for_user(user)))
        .body(Body::from(
            json!({
                "hole_id": hole_id,
                "owner": owner,
                "gross_strokes": gross
            })
            .to_string(),
        ))
        .unwrap()
}

fn confirm_request(round_id: Uuid, owner_type: &str, owner_id: Uuid, user: Uuid) -> Request<Body> {
    Request::post(format!(
        "/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}/confirm"
    ))
    .header("content-type", "application/json")
    .header("cookie", format!("golf_session={}", token_for_user(user)))
    .header("x-csrf-token", derive_csrf_token(token_for_user(user)))
    .body(Body::from("{}"))
    .unwrap()
}

fn access_request(round_id: Uuid, user: Uuid) -> Request<Body> {
    Request::get(format!("/api/rounds/{round_id}/score-access"))
        .header("cookie", format!("golf_session={}", token_for_user(user)))
        .body(Body::empty())
        .unwrap()
}

fn token_for_user(user_id: Uuid) -> &'static str {
    match user_id {
        USER_A => "score-test-user-a-token",
        USER_B => "score-test-user-b-token",
        PLAYER_USER_A => "score-test-player-a-token",
        PLAYER_USER_B => "score-test-player-b-token",
        PLAYER_USER_C => "score-test-player-c-token",
        VIEWER_USER => "score-test-viewer-token",
        UNLINKED_PLAYER_USER => "score-test-unlinked-player-token",
        _ => "score-test-missing-user-token",
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn individual_api_saves_corrects_confirms_and_preserves_true_noops(pool: PgPool) {
    seed_open(&pool).await;
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));
    let owner = json!({"type": "player", "id": PLAYER_A});

    let mut events = state.live_events.subscribe();
    let first = app
        .clone()
        .oneshot(save_request(
            INDIVIDUAL_ROUND_ID,
            HOLE_1,
            owner.clone(),
            5,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = response_json(first).await;
    assert_eq!(first_body["gross_strokes"], 5);
    assert_eq!(events.try_recv().unwrap().id, INDIVIDUAL_ROUND_ID);

    let unchanged = app
        .clone()
        .oneshot(save_request(
            INDIVIDUAL_ROUND_ID,
            HOLE_1,
            owner.clone(),
            5,
            USER_B,
        ))
        .await
        .unwrap();
    let unchanged_body = response_json(unchanged).await;
    assert_eq!(unchanged_body["submitted_by"], USER_A.to_string());
    assert_eq!(unchanged_body["updated_at"], first_body["updated_at"]);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    let corrected = app
        .clone()
        .oneshot(save_request(
            INDIVIDUAL_ROUND_ID,
            HOLE_1,
            owner.clone(),
            6,
            USER_B,
        ))
        .await
        .unwrap();
    assert_eq!(corrected.status(), StatusCode::OK);
    assert_eq!(events.try_recv().unwrap().id, INDIVIDUAL_ROUND_ID);
    let audits = sqlx::query("SELECT old_gross_strokes, new_gross_strokes, changed_by FROM score_audits ORDER BY changed_at")
        .fetch_all(&pool).await.unwrap();
    assert_eq!(audits.len(), 2);
    assert_eq!(
        audits[1].get::<Option<i16>, _>("old_gross_strokes"),
        Some(5)
    );
    assert_eq!(audits[1].get::<i16, _>("new_gross_strokes"), 6);
    assert_eq!(audits[1].get::<Uuid, _>("changed_by"), USER_B);

    let incomplete = app
        .clone()
        .oneshot(confirm_request(
            INDIVIDUAL_ROUND_ID,
            "player",
            PLAYER_A,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(incomplete.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(incomplete).await["error"]["code"],
        "scorecard_incomplete"
    );

    app.clone()
        .oneshot(save_request(
            INDIVIDUAL_ROUND_ID,
            HOLE_2,
            owner.clone(),
            4,
            USER_A,
        ))
        .await
        .unwrap();
    let confirmed = app
        .clone()
        .oneshot(confirm_request(
            INDIVIDUAL_ROUND_ID,
            "player",
            PLAYER_A,
            USER_B,
        ))
        .await
        .unwrap();
    let confirmed_body = response_json(confirmed).await;
    assert_eq!(confirmed_body["gross_total"], 10);
    assert_eq!(confirmed_body["net_total"], 8);
    assert_eq!(confirmed_body["confirmed"], true);
    assert_eq!(confirmed_body["confirmed_by"], USER_B.to_string());

    sqlx::query("UPDATE players SET current_handicap_index = 20.0 WHERE id = $1")
        .bind(PLAYER_A)
        .execute(&pool)
        .await
        .unwrap();
    let preserved = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/rounds/{INDIVIDUAL_ROUND_ID}/scorecards/player/{PLAYER_A}"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_json(preserved).await["net_total"], 8);

    let mut noop_events = state.live_events.subscribe();
    let reconfirmed = app
        .clone()
        .oneshot(confirm_request(
            INDIVIDUAL_ROUND_ID,
            "player",
            PLAYER_A,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(
        response_json(reconfirmed).await["confirmed_by"],
        USER_B.to_string()
    );
    assert!(matches!(noop_events.try_recv(), Err(TryRecvError::Empty)));

    app.clone()
        .oneshot(save_request(INDIVIDUAL_ROUND_ID, HOLE_1, owner, 5, USER_A))
        .await
        .unwrap();
    let summary = app
        .oneshot(
            Request::get(format!(
                "/api/rounds/{INDIVIDUAL_ROUND_ID}/scorecards/player/{PLAYER_A}"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response_json(summary).await["confirmed"], false);
}

#[sqlx::test(migrations = "../migrations")]
async fn team_summary_and_api_conflicts_are_format_and_round_specific(pool: PgPool) {
    seed_open(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    let team_owner = json!({"type": "team", "id": SCRAMBLE_TEAM_ID});
    let saved = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_1,
            team_owner,
            4,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(saved.status(), StatusCode::OK);
    let summary = app
        .clone()
        .oneshot(
            Request::get(format!(
                "/api/rounds/{SCRAMBLE_ROUND_ID}/scorecards/team/{SCRAMBLE_TEAM_ID}"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    let body = response_json(summary).await;
    assert_eq!(body["playing_handicap"], 2);
    assert_eq!(body["net_total"], 3);

    let wrong_format = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_1,
            json!({"type": "player", "id": PLAYER_A}),
            4,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(
        response_json(wrong_format).await["error"]["code"],
        "score_owner_format_mismatch"
    );

    let cross_round = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_2,
            json!({"type": "team", "id": OTHER_ROUND_TEAM_ID}),
            4,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(
        response_json(cross_round).await["error"]["code"],
        "score_owner_not_eligible"
    );

    let other_course = Uuid::new_v4();
    let other_tee = Uuid::new_v4();
    let other_hole = Uuid::new_v4();
    sqlx::query("INSERT INTO courses (id, name) VALUES ($1, 'Other')")
        .bind(other_course)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tees (id, course_id, name) VALUES ($1, $2, 'Other')")
        .bind(other_tee)
        .bind(other_course)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES ($1, $2, 1, 4, 1)",
    )
    .bind(other_hole)
    .bind(other_tee)
    .execute(&pool)
    .await
    .unwrap();
    let wrong_hole = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            other_hole,
            json!({"type": "team", "id": SCRAMBLE_TEAM_ID}),
            4,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(
        response_json(wrong_hole).await["error"]["code"],
        "score_hole_mismatch"
    );

    let draft = app
        .clone()
        .oneshot(save_request(
            DRAFT_ROUND_ID,
            HOLE_1,
            json!({"type": "player", "id": PLAYER_A}),
            4,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(
        response_json(draft).await["error"]["code"],
        "round_not_editable"
    );

    let unsnapshotted = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO players (id, display_name, current_handicap_index) VALUES ($1, 'Late player', 12.0)",
    )
    .bind(unsnapshotted)
    .execute(&pool)
    .await
    .unwrap();
    let ineligible = app
        .clone()
        .oneshot(save_request(
            INDIVIDUAL_ROUND_ID,
            HOLE_1,
            json!({"type": "player", "id": unsnapshotted}),
            4,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(
        response_json(ineligible).await["error"]["code"],
        "score_owner_not_eligible"
    );
    let missing_actor = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_2,
            json!({"type": "team", "id": SCRAMBLE_TEAM_ID}),
            4,
            Uuid::new_v4(),
        ))
        .await
        .unwrap();
    assert_eq!(missing_actor.status(), StatusCode::UNAUTHORIZED);

    let malformed = app
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_2,
            json!({"type": "other", "id": SCRAMBLE_TEAM_ID}),
            4,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(malformed.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        response_json(malformed).await["error"]["code"],
        "validation_error"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn both_teammates_can_write_the_team_card_but_other_players_cannot(pool: PgPool) {
    seed_open(&pool).await;
    let app = api::router(AppState::new(pool.clone()));

    let access = app
        .clone()
        .oneshot(
            Request::get(format!("/api/rounds/{SCRAMBLE_ROUND_ID}/score-access"))
                .header(
                    "cookie",
                    format!("golf_session={}", token_for_user(PLAYER_USER_A)),
                )
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(access.status(), StatusCode::OK);
    assert_eq!(
        response_json(access).await["writable_owners"],
        json!([{"type": "team", "id": SCRAMBLE_TEAM_ID}])
    );

    let owner = json!({"type": "team", "id": SCRAMBLE_TEAM_ID});
    let first = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_1,
            owner.clone(),
            4,
            PLAYER_USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(
        response_json(first).await["submitted_by"],
        PLAYER_USER_A.to_string()
    );

    let second_hole = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_2,
            owner.clone(),
            4,
            PLAYER_USER_B,
        ))
        .await
        .unwrap();
    assert_eq!(second_hole.status(), StatusCode::OK);
    let confirmed_by_first_teammate = app
        .clone()
        .oneshot(confirm_request(
            SCRAMBLE_ROUND_ID,
            "team",
            SCRAMBLE_TEAM_ID,
            PLAYER_USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(confirmed_by_first_teammate.status(), StatusCode::OK);
    assert_eq!(
        response_json(confirmed_by_first_teammate).await["confirmed_by"],
        PLAYER_USER_A.to_string()
    );

    let teammate_correction = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_1,
            owner.clone(),
            5,
            PLAYER_USER_B,
        ))
        .await
        .unwrap();
    assert_eq!(teammate_correction.status(), StatusCode::OK);
    assert_eq!(
        response_json(teammate_correction).await["submitted_by"],
        PLAYER_USER_B.to_string()
    );
    let reconfirmed = app
        .clone()
        .oneshot(confirm_request(
            SCRAMBLE_ROUND_ID,
            "team",
            SCRAMBLE_TEAM_ID,
            PLAYER_USER_B,
        ))
        .await
        .unwrap();
    assert_eq!(reconfirmed.status(), StatusCode::OK);
    assert_eq!(
        response_json(reconfirmed).await["confirmed_by"],
        PLAYER_USER_B.to_string()
    );

    let unrelated = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_1,
            owner.clone(),
            6,
            PLAYER_USER_C,
        ))
        .await
        .unwrap();
    assert_eq!(unrelated.status(), StatusCode::FORBIDDEN);
    assert_eq!(response_json(unrelated).await["error"]["code"], "forbidden");
    assert_eq!(
        sqlx::query_scalar::<_, i16>(
            "SELECT gross_strokes FROM scores WHERE round_id = $1 AND hole_id = $2 AND team_id = $3",
        )
        .bind(SCRAMBLE_ROUND_ID)
        .bind(HOLE_1)
        .bind(SCRAMBLE_TEAM_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        5
    );

    let other_individual = app
        .clone()
        .oneshot(save_request(
            INDIVIDUAL_ROUND_ID,
            HOLE_1,
            json!({"type": "player", "id": PLAYER_A}),
            5,
            PLAYER_USER_B,
        ))
        .await
        .unwrap();
    assert_eq!(other_individual.status(), StatusCode::FORBIDDEN);

    for denied_user in [VIEWER_USER, UNLINKED_PLAYER_USER] {
        let denied = app
            .clone()
            .oneshot(save_request(
                SCRAMBLE_ROUND_ID,
                HOLE_1,
                owner.clone(),
                6,
                denied_user,
            ))
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    }

    sqlx::query("UPDATE users SET player_id = NULL WHERE id = $1")
        .bind(PLAYER_USER_A)
        .execute(&pool)
        .await
        .unwrap();
    let after_unlink = app
        .clone()
        .oneshot(save_request(
            SCRAMBLE_ROUND_ID,
            HOLE_1,
            owner.clone(),
            6,
            PLAYER_USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(after_unlink.status(), StatusCode::FORBIDDEN);

    let spoofed = Request::put(format!("/api/rounds/{SCRAMBLE_ROUND_ID}/scores"))
        .header("content-type", "application/json")
        .header(
            "cookie",
            format!("golf_session={}", token_for_user(PLAYER_USER_B)),
        )
        .header(
            "x-csrf-token",
            derive_csrf_token(token_for_user(PLAYER_USER_B)),
        )
        .body(Body::from(
            json!({
                "hole_id": HOLE_1,
                "owner": owner,
                "gross_strokes": 6,
                "submitted_by": USER_A,
            })
            .to_string(),
        ))
        .unwrap();
    let spoofed = app.oneshot(spoofed).await.unwrap();
    assert_eq!(spoofed.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "../migrations")]
async fn tournament_membership_is_authoritative_for_privileged_score_access(pool: PgPool) {
    seed_open(&pool).await;
    let other_tournament = uuid!("30000000-0000-0000-0000-000000000099");
    sqlx::query(
        "DELETE FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id IN ($2, $3)",
    )
    .bind(TOURNAMENT_ID)
    .bind(USER_A)
    .bind(USER_B)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds)
         VALUES ($1, 'Other score trip', '2026-09-01', '2026-09-01', 1)",
    )
    .bind(other_tournament)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin'), ($1, $3, 'scorer')",
    )
    .bind(other_tournament)
    .bind(USER_A)
    .bind(USER_B)
    .execute(&pool)
    .await
    .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    for user in [USER_A, USER_B] {
        let access = app
            .clone()
            .oneshot(access_request(INDIVIDUAL_ROUND_ID, user))
            .await
            .unwrap();
        assert_eq!(access.status(), StatusCode::OK);
        assert_eq!(response_json(access).await["writable_owners"], json!([]));
        let save = app
            .clone()
            .oneshot(save_request(
                INDIVIDUAL_ROUND_ID,
                HOLE_1,
                json!({"type": "player", "id": PLAYER_A}),
                4,
                user,
            ))
            .await
            .unwrap();
        assert_eq!(save.status(), StatusCode::FORBIDDEN);
        let confirm = app
            .clone()
            .oneshot(confirm_request(
                INDIVIDUAL_ROUND_ID,
                "player",
                PLAYER_A,
                user,
            ))
            .await
            .unwrap();
        assert_eq!(confirm.status(), StatusCode::FORBIDDEN);
    }

    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin'), ($1, $3, 'scorer')",
    )
    .bind(TOURNAMENT_ID)
    .bind(USER_A)
    .bind(USER_B)
    .execute(&pool)
    .await
    .unwrap();
    for user in [USER_A, USER_B] {
        let access = app
            .clone()
            .oneshot(access_request(INDIVIDUAL_ROUND_ID, user))
            .await
            .unwrap();
        assert_eq!(
            response_json(access).await["writable_owners"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }
    for (hole_id, user) in [(HOLE_1, USER_A), (HOLE_2, USER_B)] {
        let saved = app
            .clone()
            .oneshot(save_request(
                INDIVIDUAL_ROUND_ID,
                hole_id,
                json!({"type": "player", "id": PLAYER_A}),
                4,
                user,
            ))
            .await
            .unwrap();
        assert_eq!(saved.status(), StatusCode::OK);
    }
    let confirmed = app
        .clone()
        .oneshot(confirm_request(
            INDIVIDUAL_ROUND_ID,
            "player",
            PLAYER_A,
            USER_B,
        ))
        .await
        .unwrap();
    assert_eq!(confirmed.status(), StatusCode::OK);

    sqlx::query(
        "UPDATE tournament_memberships SET role = 'viewer'
         WHERE tournament_id = $1 AND user_id = $2",
    )
    .bind(TOURNAMENT_ID)
    .bind(USER_A)
    .execute(&pool)
    .await
    .unwrap();
    let role_changed = app
        .clone()
        .oneshot(save_request(
            INDIVIDUAL_ROUND_ID,
            HOLE_1,
            json!({"type": "player", "id": PLAYER_B}),
            5,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(role_changed.status(), StatusCode::FORBIDDEN);

    sqlx::query(
        "DELETE FROM tournament_memberships
         WHERE tournament_id = $1 AND user_id = $2",
    )
    .bind(TOURNAMENT_ID)
    .bind(USER_B)
    .execute(&pool)
    .await
    .unwrap();
    let revoked = app
        .oneshot(confirm_request(
            INDIVIDUAL_ROUND_ID,
            "player",
            PLAYER_A,
            USER_B,
        ))
        .await
        .unwrap();
    assert_eq!(revoked.status(), StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../migrations")]
async fn database_guards_preserve_audits_locking_and_parent_cascades(pool: PgPool) {
    seed_open(&pool).await;
    let owner = ScoreOwner::Player { id: PLAYER_A };

    let mut wrong_format = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(SCRAMBLE_ROUND_ID)
        .execute(&mut *wrong_format)
        .await
        .unwrap();
    let format_error = sqlx::query("INSERT INTO scores (id, round_id, tournament_id, hole_id, player_id, gross_strokes, submitted_by) VALUES ($1, $2, $3, $4, $5, 4, $6)")
        .bind(Uuid::new_v4()).bind(SCRAMBLE_ROUND_ID).bind(TOURNAMENT_ID)
        .bind(HOLE_1).bind(PLAYER_A).bind(USER_A).execute(&mut *wrong_format).await.unwrap_err();
    assert_eq!(
        format_error
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("score_owner_format_mismatch")
    );
    wrong_format.rollback().await.unwrap();

    let other_course = Uuid::new_v4();
    let other_tee = Uuid::new_v4();
    let other_hole = Uuid::new_v4();
    sqlx::query("INSERT INTO courses (id, name) VALUES ($1, 'Guard Course')")
        .bind(other_course)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tees (id, course_id, name) VALUES ($1, $2, 'Guard Tee')")
        .bind(other_tee)
        .bind(other_course)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES ($1, $2, 1, 4, 1)",
    )
    .bind(other_hole)
    .bind(other_tee)
    .execute(&pool)
    .await
    .unwrap();
    let mut wrong_hole = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(INDIVIDUAL_ROUND_ID)
        .execute(&mut *wrong_hole)
        .await
        .unwrap();
    let hole_error = sqlx::query("INSERT INTO scores (id, round_id, tournament_id, hole_id, player_id, gross_strokes, submitted_by) VALUES ($1, $2, $3, $4, $5, 4, $6)")
        .bind(Uuid::new_v4()).bind(INDIVIDUAL_ROUND_ID).bind(TOURNAMENT_ID)
        .bind(other_hole).bind(PLAYER_A).bind(USER_A).execute(&mut *wrong_hole).await.unwrap_err();
    assert_eq!(
        hole_error
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("score_hole_not_in_round")
    );
    wrong_hole.rollback().await.unwrap();

    let saved = scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: INDIVIDUAL_ROUND_ID,
            hole_id: HOLE_1,
            owner,
            gross_strokes: 5,
            submitted_by: USER_A,
        },
    )
    .await
    .unwrap()
    .value;

    let direct_delete = sqlx::query("DELETE FROM scores WHERE id = $1")
        .bind(saved.id)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        direct_delete
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("score_mutation_context_required")
    );
    let mut contextual_delete = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(INDIVIDUAL_ROUND_ID)
        .fetch_one(&mut *contextual_delete)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(INDIVIDUAL_ROUND_ID)
        .execute(&mut *contextual_delete)
        .await
        .unwrap();
    let delete_error = sqlx::query("DELETE FROM scores WHERE id = $1")
        .bind(saved.id)
        .execute(&mut *contextual_delete)
        .await
        .unwrap_err();
    assert_eq!(
        delete_error
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("score_delete_forbidden")
    );
    contextual_delete.rollback().await.unwrap();
    let audit_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM score_audits WHERE score_id = $1")
        .bind(saved.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let audit_update = sqlx::query("UPDATE score_audits SET new_gross_strokes = 9 WHERE id = $1")
        .bind(audit_id)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        audit_update
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("score_audit_immutable")
    );

    scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: INDIVIDUAL_ROUND_ID,
            hole_id: HOLE_2,
            owner,
            gross_strokes: 4,
            submitted_by: USER_A,
        },
    )
    .await
    .unwrap();
    scorecards::confirm(&pool, INDIVIDUAL_ROUND_ID, owner, USER_A)
        .await
        .unwrap();
    let other_owner = ScoreOwner::Player { id: PLAYER_B };
    for hole_id in [HOLE_1, HOLE_2] {
        scorecards::save(
            &pool,
            scorecards::SaveScore {
                round_id: INDIVIDUAL_ROUND_ID,
                hole_id,
                owner: other_owner,
                gross_strokes: 5,
                submitted_by: USER_A,
            },
        )
        .await
        .unwrap();
    }
    scorecards::confirm(&pool, INDIVIDUAL_ROUND_ID, other_owner, USER_A)
        .await
        .unwrap();
    round_completion::complete(&pool, INDIVIDUAL_ROUND_ID)
        .await
        .unwrap();
    let completed_correction = scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: INDIVIDUAL_ROUND_ID,
            hole_id: HOLE_1,
            owner,
            gross_strokes: 6,
            submitted_by: USER_B,
        },
    )
    .await
    .unwrap();
    assert!(completed_correction.changed);
    scorecards::confirm(&pool, INDIVIDUAL_ROUND_ID, owner, USER_B)
        .await
        .unwrap();
    round_completion::lock(&pool, INDIVIDUAL_ROUND_ID)
        .await
        .unwrap();
    let locked = scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: INDIVIDUAL_ROUND_ID,
            hole_id: HOLE_1,
            owner,
            gross_strokes: 7,
            submitted_by: USER_B,
        },
    )
    .await;
    assert!(matches!(
        locked,
        Err(scorecards::ScorecardError::Conflict(_))
    ));

    let mut correction = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(INDIVIDUAL_ROUND_ID)
        .fetch_one(&mut *correction)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true), set_config('app.admin_correction', 'true', true)")
        .bind(INDIVIDUAL_ROUND_ID).execute(&mut *correction).await.unwrap();
    sqlx::query("UPDATE scores SET gross_strokes = 7, submitted_by = $2 WHERE id = $1")
        .bind(saved.id)
        .bind(USER_B)
        .execute(&mut *correction)
        .await
        .unwrap();
    correction.commit().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM score_audits WHERE score_id = $1")
            .bind(saved.id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        3
    );

    sqlx::query("DELETE FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT_ID)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM scores")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM score_audits")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn final_hole_and_correction_serialize_before_confirmation(pool: PgPool) {
    seed_open(&pool).await;
    let owner = ScoreOwner::Player { id: PLAYER_A };
    scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: INDIVIDUAL_ROUND_ID,
            hole_id: HOLE_1,
            owner,
            gross_strokes: 5,
            submitted_by: USER_A,
        },
    )
    .await
    .unwrap();

    let mut final_hole = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(INDIVIDUAL_ROUND_ID)
        .execute(&mut *final_hole)
        .await
        .unwrap();
    sqlx::query("INSERT INTO scores (id, round_id, tournament_id, hole_id, player_id, gross_strokes, submitted_by) VALUES ($1, $2, $3, $4, $5, 4, $6)")
        .bind(Uuid::new_v4()).bind(INDIVIDUAL_ROUND_ID).bind(TOURNAMENT_ID)
        .bind(HOLE_2).bind(PLAYER_A).bind(USER_A).execute(&mut *final_hole).await.unwrap();
    let confirm_pool = pool.clone();
    let mut confirmation = tokio::spawn(async move {
        scorecards::confirm(&confirm_pool, INDIVIDUAL_ROUND_ID, owner, USER_A).await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut confirmation)
            .await
            .is_err()
    );
    final_hole.commit().await.unwrap();
    assert!(confirmation.await.unwrap().unwrap().value.confirmed);

    let mut correction = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(INDIVIDUAL_ROUND_ID)
        .execute(&mut *correction)
        .await
        .unwrap();
    sqlx::query("UPDATE scores SET gross_strokes = 6, submitted_by = $2 WHERE round_id = $1 AND hole_id = $3 AND player_id = $4")
        .bind(INDIVIDUAL_ROUND_ID).bind(USER_B).bind(HOLE_1).bind(PLAYER_A)
        .execute(&mut *correction).await.unwrap();
    let confirm_pool = pool.clone();
    let mut reconfirmation = tokio::spawn(async move {
        scorecards::confirm(&confirm_pool, INDIVIDUAL_ROUND_ID, owner, USER_B).await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut reconfirmation)
            .await
            .is_err()
    );
    correction.commit().await.unwrap();
    let result = reconfirmation.await.unwrap().unwrap();
    assert!(result.changed && result.value.confirmed);
    assert_eq!(result.value.gross_total, 10);
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_score_writes_apply_in_round_lock_order(pool: PgPool) {
    seed_open(&pool).await;
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));
    let mut events = state.live_events.subscribe();
    let owner = json!({"type": "player", "id": PLAYER_A});

    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(INDIVIDUAL_ROUND_ID)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();

    let first_app = app.clone();
    let first_owner = owner.clone();
    let first = tokio::spawn(async move {
        first_app
            .oneshot(save_request(
                INDIVIDUAL_ROUND_ID,
                HOLE_1,
                first_owner,
                5,
                USER_A,
            ))
            .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    let second = tokio::spawn(async move {
        app.oneshot(save_request(INDIVIDUAL_ROUND_ID, HOLE_1, owner, 6, USER_B))
            .await
    });
    tokio::time::sleep(Duration::from_millis(50)).await;
    blocker.commit().await.unwrap();

    let first_response = first.await.unwrap().unwrap();
    let second_response = second.await.unwrap().unwrap();
    assert_eq!(first_response.status(), StatusCode::OK);
    assert_eq!(second_response.status(), StatusCode::OK);
    let first_body = response_json(first_response).await;
    let second_body = response_json(second_response).await;
    assert_eq!(first_body["gross_strokes"], 5);
    assert_eq!(second_body["gross_strokes"], 6);
    assert_eq!(second_body["submitted_by"], USER_B.to_string());
    assert_ne!(first_body["updated_at"], second_body["updated_at"]);

    let audits = sqlx::query(
        "SELECT old_gross_strokes, new_gross_strokes FROM score_audits ORDER BY changed_at, id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(audits.len(), 2);
    assert_eq!(audits[0].get::<Option<i16>, _>("old_gross_strokes"), None);
    assert_eq!(audits[0].get::<i16, _>("new_gross_strokes"), 5);
    assert_eq!(
        audits[1].get::<Option<i16>, _>("old_gross_strokes"),
        Some(5)
    );
    assert_eq!(audits[1].get::<i16, _>("new_gross_strokes"), 6);
    assert_eq!(events.recv().await.unwrap().id, INDIVIDUAL_ROUND_ID);
    assert_eq!(events.recv().await.unwrap().id, INDIVIDUAL_ROUND_ID);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}
