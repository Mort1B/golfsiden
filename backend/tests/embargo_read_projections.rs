#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::hash_session_token,
    domain::scorecards::ScoreOwner,
    repositories::{auth, round_completion, round_lifecycle, scorecards, tournaments},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("71000000-0000-0000-0000-000000000001");
const ROUND: Uuid = uuid!("71000000-0000-0000-0000-000000000002");
const ADMIN: Uuid = uuid!("71000000-0000-0000-0000-000000000003");
const SCORER: Uuid = uuid!("71000000-0000-0000-0000-000000000004");
const PLAYER_USER: Uuid = uuid!("71000000-0000-0000-0000-000000000005");
const VIEWER: Uuid = uuid!("71000000-0000-0000-0000-000000000006");
const PLAYER_A: Uuid = uuid!("71000000-0000-0000-0000-000000000007");
const PLAYER_B: Uuid = uuid!("71000000-0000-0000-0000-000000000008");
const OTHER_TOURNAMENT: Uuid = uuid!("71000000-0000-0000-0000-000000000020");
const OTHER_ROUND: Uuid = uuid!("71000000-0000-0000-0000-000000000021");
const TEAM: Uuid = uuid!("71000000-0000-0000-0000-000000000022");

async fn seed(pool: &PgPool) -> Vec<Uuid> {
    sqlx::raw_sql(
        r#"
        INSERT INTO users (id, username, display_name, role) VALUES
        ('71000000-0000-0000-0000-000000000003', 'embargo_admin', 'Admin', 'viewer'),
        ('71000000-0000-0000-0000-000000000004', 'embargo_scorer', 'Scorer', 'viewer'),
        ('71000000-0000-0000-0000-000000000006', 'embargo_viewer', 'Viewer', 'viewer');
        INSERT INTO players (id, display_name, current_handicap_index) VALUES
        ('71000000-0000-0000-0000-000000000007', 'Ada', 0),
        ('71000000-0000-0000-0000-000000000008', 'Bob', 0);
        INSERT INTO users (id, username, display_name, role, player_id) VALUES
        ('71000000-0000-0000-0000-000000000005', 'embargo_player', 'Ada', 'player', '71000000-0000-0000-0000-000000000007');
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds, counted_rounds)
        VALUES ('71000000-0000-0000-0000-000000000001', 'Embargo Cup', '2026-09-01', '2026-09-01', 1, 1);
        INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap) VALUES
        ('71000000-0000-0000-0000-000000000001', '71000000-0000-0000-0000-000000000007', 18),
        ('71000000-0000-0000-0000-000000000001', '71000000-0000-0000-0000-000000000008', 0);
        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
        ('71000000-0000-0000-0000-000000000001', '71000000-0000-0000-0000-000000000003', 'admin'),
        ('71000000-0000-0000-0000-000000000001', '71000000-0000-0000-0000-000000000004', 'scorer'),
        ('71000000-0000-0000-0000-000000000001', '71000000-0000-0000-0000-000000000005', 'player'),
        ('71000000-0000-0000-0000-000000000001', '71000000-0000-0000-0000-000000000006', 'viewer');
        INSERT INTO courses (id, name) VALUES ('71000000-0000-0000-0000-000000000009', 'Embargo Course');
        INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
        VALUES ('71000000-0000-0000-0000-000000000010', '71000000-0000-0000-0000-000000000009', 'Test', 113, 72);
        INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
        SELECT gen_random_uuid(), '71000000-0000-0000-0000-000000000010', hole, 4, hole
        FROM generate_series(1, 18) AS hole;
        INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format)
        VALUES ('71000000-0000-0000-0000-000000000002', '71000000-0000-0000-0000-000000000001', 1, 'Final', '2026-09-01', '71000000-0000-0000-0000-000000000009', 'Embargo Course', '71000000-0000-0000-0000-000000000010', 'Test', 18, 'individual_stroke_play');
        INSERT INTO flights (id, round_id, tournament_id, name) VALUES
        ('71000000-0000-0000-0000-000000000011', '71000000-0000-0000-0000-000000000002', '71000000-0000-0000-0000-000000000001', 'A'),
        ('71000000-0000-0000-0000-000000000012', '71000000-0000-0000-0000-000000000002', '71000000-0000-0000-0000-000000000001', 'B');
        INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id, display_order) VALUES
        ('71000000-0000-0000-0000-000000000011', '71000000-0000-0000-0000-000000000002', '71000000-0000-0000-0000-000000000001', '71000000-0000-0000-0000-000000000007', 1),
        ('71000000-0000-0000-0000-000000000012', '71000000-0000-0000-0000-000000000002', '71000000-0000-0000-0000-000000000001', '71000000-0000-0000-0000-000000000008', 1);
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    let mut admin_session = None;
    for user in [ADMIN, SCORER, PLAYER_USER, VIEWER] {
        let session = auth::create_session(
            pool,
            user,
            &hash_session_token(token(user)),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
        if user == ADMIN {
            admin_session = Some(session.session_id);
        }
    }
    let updated_at = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT)
        .fetch_one(pool)
        .await
        .unwrap();
    tournaments::start_authorized(
        pool,
        admin_session.expect("admin fixture session exists"),
        TOURNAMENT,
        updated_at,
    )
    .await
    .unwrap();
    round_lifecycle::open(pool, ROUND).await.unwrap();
    sqlx::query_scalar("SELECT id FROM holes WHERE tee_id = '71000000-0000-0000-0000-000000000010' ORDER BY hole_number")
        .fetch_all(pool)
        .await
        .unwrap()
}

fn token(user: Uuid) -> &'static str {
    match user {
        ADMIN => "embargo-admin-token",
        SCORER => "embargo-scorer-token",
        PLAYER_USER => "embargo-player-token",
        VIEWER => "embargo-viewer-token",
        _ => "missing",
    }
}

async fn get(app: &axum::Router, path: &str, user: Uuid) -> (StatusCode, Value, String) {
    let response = app
        .clone()
        .oneshot(
            Request::get(path)
                .header("cookie", format!("golf_session={}", token(user)))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let cache = response
        .headers()
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();
    let body =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    (status, body, cache)
}

async fn save_all(pool: &PgPool, holes: &[Uuid], player: Uuid, gross: i16) {
    for hole_id in holes {
        scorecards::save(
            pool,
            scorecards::SaveScore {
                round_id: ROUND,
                hole_id: *hole_id,
                owner: ScoreOwner::Player { id: player },
                gross_strokes: gross,
                submitted_by: ADMIN,
            },
        )
        .await
        .unwrap();
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn open_final_round_redacts_reads_but_exact_scoring_projection_stays_full(pool: PgPool) {
    let holes = seed(&pool).await;
    scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: ROUND,
            hole_id: holes[0],
            owner: ScoreOwner::Player { id: PLAYER_A },
            gross_strokes: 5,
            submitted_by: ADMIN,
        },
    )
    .await
    .unwrap();
    scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: ROUND,
            hole_id: holes[9],
            owner: ScoreOwner::Player { id: PLAYER_A },
            gross_strokes: 2,
            submitted_by: ADMIN,
        },
    )
    .await
    .unwrap();
    let app = api::router(AppState::new(pool));
    let completion_path = format!("/api/rounds/{ROUND}/completion-validation");
    let (_, completion, completion_cache) = get(&app, &completion_path, PLAYER_USER).await;
    assert_eq!(completion_cache, "private, no-store");
    assert_eq!(completion["visibility"]["mode"], "front_nine");
    let completion_ada = completion["owners"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["owner"]["id"] == PLAYER_A.to_string())
        .unwrap();
    assert_eq!(completion_ada["holes_scored"], 1);
    assert_eq!(completion_ada["required_holes"], 9);
    assert!(completion_ada["complete"].is_null());
    assert!(completion_ada["confirmed"].is_null());
    assert!(completion["ready_to_complete"].is_null());
    assert!(completion["ready_to_lock"].is_null());
    assert_eq!(completion["issues"], serde_json::json!([]));
    let (_, admin_completion, _) = get(&app, &completion_path, ADMIN).await;
    let admin_completion_ada = admin_completion["owners"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["owner"]["id"] == PLAYER_A.to_string())
        .unwrap();
    assert_eq!(admin_completion["visibility"]["mode"], "full");
    assert_eq!(admin_completion_ada["holes_scored"], 2);
    assert_eq!(admin_completion_ada["required_holes"], 18);
    assert_eq!(admin_completion_ada["complete"], false);
    let card_path = format!("/api/rounds/{ROUND}/scorecards/player/{PLAYER_A}");
    for user in [SCORER, PLAYER_USER, VIEWER] {
        let (status, body, cache) = get(&app, &card_path, user).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(cache, "private, no-store");
        assert_eq!(body["visibility"]["mode"], "front_nine");
        assert_eq!(body["visible_hole_count"], 9);
        assert_eq!(body["gross_total"], 5);
        assert!(body["complete"].is_null() && body["confirmed"].is_null());
        assert!(!body.to_string().contains("submitted_by"));
        assert!(
            body["holes"]
                .as_array()
                .unwrap()
                .iter()
                .all(|hole| hole["hole_number"].as_i64().unwrap() <= 9)
        );
    }
    let (_, admin_card, _) = get(&app, &card_path, ADMIN).await;
    assert_eq!(admin_card["visibility"]["mode"], "full");
    assert_eq!(admin_card["gross_total"], 7);
    assert!(!admin_card.to_string().contains("submitted_by"));

    let leaderboard_path = format!("/api/rounds/{ROUND}/leaderboards/gross");
    let (_, restricted, _) = get(&app, &leaderboard_path, SCORER).await;
    let ada = restricted["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["owner"]["id"] == PLAYER_A.to_string())
        .unwrap();
    assert_eq!(restricted["visible_hole_count"], 9);
    assert_eq!(ada["gross_total"], 5);
    assert!(ada["complete"].is_null() && ada["confirmed"].is_null());
    let (_, admin_board, _) = get(&app, &leaderboard_path, ADMIN).await;
    let admin_ada = admin_board["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["owner"]["id"] == PLAYER_A.to_string())
        .unwrap();
    assert_eq!(admin_ada["gross_total"], 7);
    let (_, restricted_net, _) = get(
        &app,
        &format!("/api/rounds/{ROUND}/leaderboards/net"),
        PLAYER_USER,
    )
    .await;
    let net_ada = restricted_net["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["owner"]["id"] == PLAYER_A.to_string())
        .unwrap();
    assert_eq!(net_ada["net_total"], 4);

    let scoring_path = format!("{card_path}/scoring");
    let (status, scoring, cache) = get(&app, &scoring_path, PLAYER_USER).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cache, "private, no-store");
    assert_eq!(scoring["gross_total"], 7);
    assert!(scoring.to_string().contains("submitted_by"));
    let (status, _, _) = get(
        &app,
        &format!("/api/rounds/{ROUND}/scorecards/player/{PLAYER_B}/scoring"),
        PLAYER_USER,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = get(&app, &scoring_path, VIEWER).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../migrations")]
async fn scoring_and_read_errors_preserve_session_and_cross_target_boundaries(pool: PgPool) {
    seed(&pool).await;
    sqlx::query("INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds) VALUES ($1, 'Other', '2026-09-02', '2026-09-02', 1)")
        .bind(OTHER_TOURNAMENT)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format) VALUES ($1, $2, 1, 'Other', '2026-09-02', '71000000-0000-0000-0000-000000000009', 'Embargo Course', '71000000-0000-0000-0000-000000000010', 'Test', 18, 'individual_stroke_play')")
        .bind(OTHER_ROUND)
        .bind(OTHER_TOURNAMENT)
        .execute(&pool)
        .await
        .unwrap();
    let app = api::router(AppState::new(pool));
    for suffix in ["", "/scoring"] {
        let path = format!("/api/rounds/{OTHER_ROUND}/scorecards/player/{PLAYER_A}{suffix}");
        let (status, body, cache) = get(&app, &path, ADMIN).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body["error"]["code"], "forbidden");
        assert_eq!(cache, "no-store");
    }
    let missing = Uuid::new_v4();
    let (status, body, cache) = get(
        &app,
        &format!("/api/rounds/{missing}/scorecards/player/{PLAYER_A}"),
        ADMIN,
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["error"]["code"], "not_found");
    assert!(cache.is_empty());
    let unauthenticated = app
        .oneshot(
            Request::get(format!(
                "/api/rounds/{ROUND}/scorecards/player/{PLAYER_A}/scoring"
            ))
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "../migrations")]
async fn team_final_blackout_redacts_reads_but_exact_team_scorers_keep_full_projection(
    pool: PgPool,
) {
    let holes = seed(&pool).await;
    sqlx::raw_sql(
        r#"
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
        VALUES ('71000000-0000-0000-0000-000000000020', 'Team Final', '2026-09-02', '2026-09-02', 1);
        INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap) VALUES
        ('71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000007', 18),
        ('71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000008', 0);
        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
        ('71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000003', 'admin'),
        ('71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000004', 'scorer'),
        ('71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000005', 'player'),
        ('71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000006', 'viewer');
        INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format)
        VALUES ('71000000-0000-0000-0000-000000000021', '71000000-0000-0000-0000-000000000020', 1, 'Team Final', '2026-09-02', '71000000-0000-0000-0000-000000000009', 'Embargo Course', '71000000-0000-0000-0000-000000000010', 'Test', 18, 'team_scramble');
        INSERT INTO teams (id, round_id, tournament_id, name)
        VALUES ('71000000-0000-0000-0000-000000000022', '71000000-0000-0000-0000-000000000021', '71000000-0000-0000-0000-000000000020', 'Exact Team');
        INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id, display_order) VALUES
        ('71000000-0000-0000-0000-000000000022', '71000000-0000-0000-0000-000000000021', '71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000007', 1),
        ('71000000-0000-0000-0000-000000000022', '71000000-0000-0000-0000-000000000021', '71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000008', 2);
        INSERT INTO flights (id, round_id, tournament_id, name)
        VALUES ('71000000-0000-0000-0000-000000000023', '71000000-0000-0000-0000-000000000021', '71000000-0000-0000-0000-000000000020', 'Team Flight');
        INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id, display_order) VALUES
        ('71000000-0000-0000-0000-000000000023', '71000000-0000-0000-0000-000000000021', '71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000007', 1),
        ('71000000-0000-0000-0000-000000000023', '71000000-0000-0000-0000-000000000021', '71000000-0000-0000-0000-000000000020', '71000000-0000-0000-0000-000000000008', 2);
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let session_id = sqlx::query_scalar(
        "SELECT id FROM user_sessions WHERE user_id = $1 AND revoked_at IS NULL ORDER BY created_at DESC LIMIT 1",
    )
    .bind(ADMIN)
    .fetch_one(&pool)
    .await
    .unwrap();
    let updated_at = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id = $1")
        .bind(OTHER_TOURNAMENT)
        .fetch_one(&pool)
        .await
        .unwrap();
    tournaments::start_authorized(&pool, session_id, OTHER_TOURNAMENT, updated_at)
        .await
        .unwrap();
    round_lifecycle::open(&pool, OTHER_ROUND).await.unwrap();
    for (hole_id, gross) in [(holes[0], 5), (holes[9], 2)] {
        scorecards::save(
            &pool,
            scorecards::SaveScore {
                round_id: OTHER_ROUND,
                hole_id,
                owner: ScoreOwner::Team { id: TEAM },
                gross_strokes: gross,
                submitted_by: ADMIN,
            },
        )
        .await
        .unwrap();
    }
    let app = api::router(AppState::new(pool));
    let card = format!("/api/rounds/{OTHER_ROUND}/scorecards/team/{TEAM}");
    let (_, read, _) = get(&app, &card, VIEWER).await;
    assert_eq!(read["visibility"]["mode"], "front_nine");
    assert_eq!(read["gross_total"], 5);
    assert_eq!(read["holes"].as_array().unwrap().len(), 9);
    let (_, completion, _) = get(
        &app,
        &format!("/api/rounds/{OTHER_ROUND}/completion-validation"),
        VIEWER,
    )
    .await;
    assert_eq!(completion["visibility"]["mode"], "front_nine");
    assert_eq!(completion["owners"][0]["owner"]["id"], TEAM.to_string());
    assert_eq!(completion["owners"][0]["holes_scored"], 1);
    assert_eq!(completion["owners"][0]["required_holes"], 9);
    assert!(completion["owners"][0]["complete"].is_null());
    assert!(completion["owners"][0]["confirmed"].is_null());
    assert_eq!(completion["issues"], serde_json::json!([]));

    for user in [SCORER, PLAYER_USER] {
        let (status, scoring, cache) = get(&app, &format!("{card}/scoring"), user).await;
        assert_eq!(status, StatusCode::OK, "user {user}");
        assert_eq!(cache, "private, no-store");
        assert_eq!(scoring["gross_total"], 7);
        assert_eq!(scoring["holes"].as_array().unwrap().len(), 18);
    }
    let other_owner = Uuid::new_v4();
    let (status, _, _) = get(
        &app,
        &format!("/api/rounds/{OTHER_ROUND}/scorecards/team/{other_owner}/scoring"),
        PLAYER_USER,
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _, _) = get(&app, &format!("{card}/scoring"), VIEWER).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[sqlx::test(migrations = "../migrations")]
async fn hidden_completed_final_round_is_omitted_from_non_admin_tournament_totals(pool: PgPool) {
    let holes = seed(&pool).await;
    save_all(&pool, &holes, PLAYER_A, 4).await;
    save_all(&pool, &holes, PLAYER_B, 5).await;
    for player in [PLAYER_A, PLAYER_B] {
        scorecards::confirm(&pool, ROUND, ScoreOwner::Player { id: player }, ADMIN)
            .await
            .unwrap();
    }
    round_completion::complete(&pool, ROUND).await.unwrap();
    let app = api::router(AppState::new(pool));
    let path = format!("/api/tournaments/{TOURNAMENT}/leaderboards/gross");
    let (_, restricted, cache) = get(&app, &path, SCORER).await;
    assert_eq!(cache, "private, no-store");
    assert_eq!(restricted["visibility"]["mode"], "front_nine");
    assert!(restricted["visibility"]["hidden_until"].is_string());
    assert_eq!(restricted["included_round_ids"], serde_json::json!([]));
    assert!(
        restricted["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| {
                entry["completed_rounds"] == 0
                    && entry["counted_contributions"] == 0
                    && entry["contributions"].as_array().unwrap().is_empty()
                    && entry["position"].is_null()
            })
    );
    let (_, admin, _) = get(&app, &path, ADMIN).await;
    assert_eq!(admin["visibility"]["mode"], "full");
    assert_eq!(admin["included_round_ids"], serde_json::json!([ROUND]));
    assert!(
        admin["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["completed_rounds"] == 1)
    );
    let completion_path = format!("/api/rounds/{ROUND}/completion-validation");
    let (_, restricted_completion, _) = get(&app, &completion_path, SCORER).await;
    assert_eq!(restricted_completion["visibility"]["mode"], "front_nine");
    assert!(restricted_completion["ready_to_lock"].is_null());
    assert_eq!(restricted_completion["issues"], serde_json::json!([]));
    assert!(
        restricted_completion["owners"]
            .as_array()
            .unwrap()
            .iter()
            .all(|owner| {
                owner["holes_scored"] == 9
                    && owner["required_holes"] == 9
                    && owner["complete"].is_null()
                    && owner["confirmed"].is_null()
            })
    );
    let (_, admin_completion, _) = get(&app, &completion_path, ADMIN).await;
    assert_eq!(admin_completion["ready_to_lock"], true);
    assert!(
        admin_completion["owners"]
            .as_array()
            .unwrap()
            .iter()
            .all(|owner| {
                owner["holes_scored"] == 18
                    && owner["complete"] == true
                    && owner["confirmed"] == true
            })
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn locked_future_null_and_expired_deadlines_fail_closed_then_reveal(pool: PgPool) {
    let holes = seed(&pool).await;
    save_all(&pool, &holes, PLAYER_A, 4).await;
    save_all(&pool, &holes, PLAYER_B, 5).await;
    for player in [PLAYER_A, PLAYER_B] {
        scorecards::confirm(&pool, ROUND, ScoreOwner::Player { id: player }, ADMIN)
            .await
            .unwrap();
    }
    round_completion::complete(&pool, ROUND).await.unwrap();
    round_completion::lock(&pool, ROUND).await.unwrap();
    let app = api::router(AppState::new(pool.clone()));
    let card = format!("/api/rounds/{ROUND}/scorecards/player/{PLAYER_A}");
    let (_, future, _) = get(&app, &card, PLAYER_USER).await;
    assert_eq!(future["visibility"]["mode"], "front_nine");
    assert!(future["complete"].is_null() && future["confirmed"].is_null());
    let (status, _, _) = get(&app, &format!("{card}/scoring"), PLAYER_USER).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let completion_path = format!("/api/rounds/{ROUND}/completion-validation");
    let (_, locked_completion, _) = get(&app, &completion_path, PLAYER_USER).await;
    assert_eq!(locked_completion["visibility"]["mode"], "front_nine");
    assert!(locked_completion["ready_to_complete"].is_null());
    assert!(locked_completion["ready_to_lock"].is_null());
    assert_eq!(locked_completion["issues"], serde_json::json!([]));

    sqlx::query("ALTER TABLE rounds DISABLE TRIGGER rounds_protect_final_score_embargo")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET final_scores_hidden_until = NULL WHERE id = $1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    let (_, null_deadline, _) = get(&app, &card, PLAYER_USER).await;
    assert_eq!(null_deadline["visibility"]["mode"], "front_nine");
    assert!(null_deadline["visibility"]["hidden_until"].is_null());
    let (_, null_completion, _) = get(&app, &completion_path, PLAYER_USER).await;
    assert_eq!(null_completion["visibility"]["mode"], "front_nine");
    assert!(null_completion["ready_to_lock"].is_null());
    assert_eq!(null_completion["issues"], serde_json::json!([]));

    sqlx::query("UPDATE rounds SET final_scores_hidden_until = clock_timestamp() - interval '1 second' WHERE id = $1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    let (_, expired, _) = get(&app, &card, PLAYER_USER).await;
    assert_eq!(expired["visibility"]["mode"], "full");
    assert_eq!(expired["holes"].as_array().unwrap().len(), 18);
    assert_eq!(expired["complete"], true);
    assert_eq!(expired["confirmed"], true);
    let (_, expired_completion, _) = get(&app, &completion_path, PLAYER_USER).await;
    assert_eq!(expired_completion["visibility"]["mode"], "full");
    assert_eq!(expired_completion["owners"][0]["complete"], true);
    assert_eq!(expired_completion["owners"][0]["confirmed"], true);
    assert!(expired_completion["ready_to_complete"].is_boolean());
    assert!(expired_completion["ready_to_lock"].is_boolean());
    let (_, tournament, _) = get(
        &app,
        &format!("/api/tournaments/{TOURNAMENT}/leaderboards/gross"),
        PLAYER_USER,
    )
    .await;
    assert_eq!(tournament["visibility"]["mode"], "full");
    assert_eq!(tournament["included_round_ids"], serde_json::json!([ROUND]));
}

#[sqlx::test(migrations = "../migrations")]
async fn corrupt_hidden_hole_facts_fail_before_every_projection(pool: PgPool) {
    let holes = seed(&pool).await;
    sqlx::query("ALTER TABLE holes DISABLE TRIGGER USER")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM holes WHERE id = $1")
        .bind(holes[17])
        .execute(&pool)
        .await
        .unwrap();
    let app = api::router(AppState::new(pool));
    let card = format!("/api/rounds/{ROUND}/scorecards/player/{PLAYER_A}");
    for user in [PLAYER_USER, ADMIN] {
        let (status, body, _) = get(&app, &card, user).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body["error"]["code"], "internal_error");
    }
}
