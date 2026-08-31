#![cfg(feature = "database-tests")]

use std::time::Duration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{
    AppState, api,
    auth::hash_session_token,
    domain::scorecards::ScoreOwner,
    repositories::{auth, round_completion, round_lifecycle, scorecards},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("50000000-0000-0000-0000-000000000001");
const ROUND_ONE: Uuid = uuid!("50000000-0000-0000-0000-000000000011");
const ROUND_TWO: Uuid = uuid!("50000000-0000-0000-0000-000000000012");
const ROUND_THREE: Uuid = uuid!("50000000-0000-0000-0000-000000000013");
const ROUND_DRAFT: Uuid = uuid!("50000000-0000-0000-0000-000000000014");
const PLAYER_A: Uuid = uuid!("50000000-0000-0000-0000-000000000021");
const PLAYER_B: Uuid = uuid!("50000000-0000-0000-0000-000000000022");
const PLAYER_PLUS: Uuid = uuid!("50000000-0000-0000-0000-000000000023");
const PLAYER_D: Uuid = uuid!("50000000-0000-0000-0000-000000000024");
const PLAYER_ZERO: Uuid = uuid!("50000000-0000-0000-0000-000000000025");
const TEAM_ONE: Uuid = uuid!("50000000-0000-0000-0000-000000000031");
const TEAM_TWO_A: Uuid = uuid!("50000000-0000-0000-0000-000000000032");
const TEAM_TWO_B: Uuid = uuid!("50000000-0000-0000-0000-000000000033");
const CURRENT_A: Uuid = uuid!("50000000-0000-0000-0000-000000000034");
const LATEST_TEAM: Uuid = uuid!("50000000-0000-0000-0000-000000000036");
const LATEST_TEAM_TWO: Uuid = uuid!("50000000-0000-0000-0000-000000000037");
const HOLE_ONE: Uuid = uuid!("50000000-0000-0000-0000-000000000041");
const HOLE_TWO: Uuid = uuid!("50000000-0000-0000-0000-000000000042");
const USER: Uuid = uuid!("50000000-0000-0000-0000-000000000051");
const SESSION_TOKEN: &str = "leaderboard-private-token";

const FIXTURE: &str = r#"
INSERT INTO users (id, username, display_name, role) VALUES
('50000000-0000-0000-0000-000000000051', 'leaderboard_scorer', 'Scorer', 'scorer');
INSERT INTO players (id, display_name, current_handicap_index) VALUES
('50000000-0000-0000-0000-000000000021', 'Ada', 8.0),
('50000000-0000-0000-0000-000000000022', 'Bob', 20.0),
('50000000-0000-0000-0000-000000000023', 'Plus', -1.0),
('50000000-0000-0000-0000-000000000024', 'Zed', 5.0),
('50000000-0000-0000-0000-000000000025', 'Withdrawn zero', 12.0);
INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds, counted_rounds, status)
VALUES ('50000000-0000-0000-0000-000000000001', 'Leaderboard Cup', '2026-08-01', '2026-08-04', 4, 2, 'active');
INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap, status) VALUES
('50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000021', 8.0, 'active'),
('50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000022', 20.0, 'active'),
('50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000023', -1.0, 'active'),
('50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000024', 5.0, 'active'),
('50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000025', 12.0, 'withdrawn');
INSERT INTO courses (id, name) VALUES
('50000000-0000-0000-0000-000000000002', 'Leaderboard Course');
INSERT INTO tees (id, course_id, name, slope_rating, course_rating) VALUES
('50000000-0000-0000-0000-000000000003', '50000000-0000-0000-0000-000000000002', 'Test', 113, 8.0);
INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES
('50000000-0000-0000-0000-000000000042', '50000000-0000-0000-0000-000000000003', 2, 4, 2),
('50000000-0000-0000-0000-000000000041', '50000000-0000-0000-0000-000000000003', 1, 4, 1);
INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format) VALUES
('50000000-0000-0000-0000-000000000011', '50000000-0000-0000-0000-000000000001', 1, 'Individual', '2026-08-01', '50000000-0000-0000-0000-000000000002', 'Leaderboard Course', '50000000-0000-0000-0000-000000000003', 'Test', 2, 'individual_stroke_play'),
('50000000-0000-0000-0000-000000000012', '50000000-0000-0000-0000-000000000001', 2, 'Scramble', '2026-08-02', '50000000-0000-0000-0000-000000000002', 'Leaderboard Course', '50000000-0000-0000-0000-000000000003', 'Test', 2, 'team_scramble'),
('50000000-0000-0000-0000-000000000013', '50000000-0000-0000-0000-000000000001', 3, 'Current', '2026-08-03', '50000000-0000-0000-0000-000000000002', 'Leaderboard Course', '50000000-0000-0000-0000-000000000003', 'Test', 2, 'team_scramble'),
('50000000-0000-0000-0000-000000000014', '50000000-0000-0000-0000-000000000001', 4, 'Draft', '2026-08-04', '50000000-0000-0000-0000-000000000002', 'Leaderboard Course', '50000000-0000-0000-0000-000000000003', 'Test', 2, 'team_scramble');
INSERT INTO teams (id, round_id, tournament_id, name) VALUES
('50000000-0000-0000-0000-000000000032', '50000000-0000-0000-0000-000000000012', '50000000-0000-0000-0000-000000000001', 'Low pair'),
('50000000-0000-0000-0000-000000000033', '50000000-0000-0000-0000-000000000012', '50000000-0000-0000-0000-000000000001', 'Mixed pair'),
('50000000-0000-0000-0000-000000000034', '50000000-0000-0000-0000-000000000013', '50000000-0000-0000-0000-000000000001', 'Current A'),
('50000000-0000-0000-0000-000000000035', '50000000-0000-0000-0000-000000000013', '50000000-0000-0000-0000-000000000001', 'Current B');
INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id, display_order) VALUES
('50000000-0000-0000-0000-000000000032', '50000000-0000-0000-0000-000000000012', '50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000021', 2),
('50000000-0000-0000-0000-000000000032', '50000000-0000-0000-0000-000000000012', '50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000022', 1),
('50000000-0000-0000-0000-000000000033', '50000000-0000-0000-0000-000000000012', '50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000023', NULL),
('50000000-0000-0000-0000-000000000033', '50000000-0000-0000-0000-000000000012', '50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000024', 1),
('50000000-0000-0000-0000-000000000034', '50000000-0000-0000-0000-000000000013', '50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000021', 1),
('50000000-0000-0000-0000-000000000034', '50000000-0000-0000-0000-000000000013', '50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000023', 2),
('50000000-0000-0000-0000-000000000035', '50000000-0000-0000-0000-000000000013', '50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000022', 1),
('50000000-0000-0000-0000-000000000035', '50000000-0000-0000-0000-000000000013', '50000000-0000-0000-0000-000000000001', '50000000-0000-0000-0000-000000000024', 2);
INSERT INTO flights (id, round_id, tournament_id, name) VALUES
('50000000-0000-0000-0000-000000000061', '50000000-0000-0000-0000-000000000011', '50000000-0000-0000-0000-000000000001', 'Round 1 flight'),
('50000000-0000-0000-0000-000000000062', '50000000-0000-0000-0000-000000000012', '50000000-0000-0000-0000-000000000001', 'Round 2 flight'),
('50000000-0000-0000-0000-000000000063', '50000000-0000-0000-0000-000000000013', '50000000-0000-0000-0000-000000000001', 'Round 3 flight'),
('50000000-0000-0000-0000-000000000064', '50000000-0000-0000-0000-000000000014', '50000000-0000-0000-0000-000000000001', 'Round 4 flight');
INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id, display_order)
SELECT f.id, f.round_id, f.tournament_id, p.player_id, p.display_order
FROM (VALUES
    ('50000000-0000-0000-0000-000000000061'::uuid, '50000000-0000-0000-0000-000000000011'::uuid, '50000000-0000-0000-0000-000000000001'::uuid),
    ('50000000-0000-0000-0000-000000000062'::uuid, '50000000-0000-0000-0000-000000000012'::uuid, '50000000-0000-0000-0000-000000000001'::uuid),
    ('50000000-0000-0000-0000-000000000063'::uuid, '50000000-0000-0000-0000-000000000013'::uuid, '50000000-0000-0000-0000-000000000001'::uuid),
    ('50000000-0000-0000-0000-000000000064'::uuid, '50000000-0000-0000-0000-000000000014'::uuid, '50000000-0000-0000-0000-000000000001'::uuid)
) AS f(id, round_id, tournament_id)
CROSS JOIN (VALUES
    ('50000000-0000-0000-0000-000000000021'::uuid, 1),
    ('50000000-0000-0000-0000-000000000022'::uuid, 2),
    ('50000000-0000-0000-0000-000000000023'::uuid, 3),
    ('50000000-0000-0000-0000-000000000024'::uuid, 4)
) AS p(player_id, display_order);
"#;

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(FIXTURE).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'scorer') ON CONFLICT DO NOTHING",
    )
    .bind(TOURNAMENT)
    .bind(USER)
    .execute(pool)
    .await
    .unwrap();
    auth::create_session(
        pool,
        USER,
        &hash_session_token(SESSION_TOKEN),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap();
}

async fn open(pool: &PgPool, round_id: Uuid) {
    round_lifecycle::open(pool, round_id).await.unwrap();
}

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

async fn get(app: &axum::Router, path: String) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::get(path)
                .header("cookie", format!("golf_session={SESSION_TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    (response.status(), body(response).await)
}

async fn save(pool: &PgPool, round_id: Uuid, hole_id: Uuid, owner: ScoreOwner, gross: i16) {
    scorecards::save(
        pool,
        scorecards::SaveScore {
            round_id,
            hole_id,
            owner,
            gross_strokes: gross,
            submitted_by: USER,
        },
    )
    .await
    .unwrap();
}

async fn open_latest_round(pool: &PgPool) {
    for (team_id, name, members) in [
        (LATEST_TEAM, "Latest current", [PLAYER_A, PLAYER_B]),
        (
            LATEST_TEAM_TWO,
            "Latest current two",
            [PLAYER_PLUS, PLAYER_D],
        ),
    ] {
        sqlx::query(
            "INSERT INTO teams (id, round_id, tournament_id, name) VALUES ($1, $2, $3, $4)",
        )
        .bind(team_id)
        .bind(ROUND_DRAFT)
        .bind(TOURNAMENT)
        .bind(name)
        .execute(pool)
        .await
        .unwrap();
        for (order, player_id) in members.into_iter().enumerate() {
            sqlx::query("INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id, display_order) VALUES ($1, $2, $3, $4, $5)")
                .bind(team_id)
                .bind(ROUND_DRAFT)
                .bind(TOURNAMENT)
                .bind(player_id)
                .bind((order + 1) as i16)
                .execute(pool)
                .await
                .unwrap();
        }
    }
    open(pool, ROUND_DRAFT).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn foursomes_round_and_tournament_leaderboards_use_preserved_team_snapshots(pool: PgPool) {
    seed(&pool).await;
    sqlx::query(
        "UPDATE rounds SET scoring_format = 'two_player_foursomes', handicap_allowance_percent = 50 WHERE id = $1",
    )
    .bind(ROUND_TWO)
    .execute(&pool)
    .await
    .unwrap();
    open(&pool, ROUND_TWO).await;
    for team_id in [TEAM_TWO_A, TEAM_TWO_B] {
        for hole_id in [HOLE_ONE, HOLE_TWO] {
            save(
                &pool,
                ROUND_TWO,
                hole_id,
                ScoreOwner::Team { id: team_id },
                5,
            )
            .await;
        }
        scorecards::confirm(&pool, ROUND_TWO, ScoreOwner::Team { id: team_id }, USER)
            .await
            .unwrap();
    }
    round_completion::complete(&pool, ROUND_TWO).await.unwrap();

    let app = api::router(AppState::new(pool));
    let (status, response) = get(&app, format!("/api/rounds/{ROUND_TWO}/leaderboards/net")).await;
    assert_eq!(status, StatusCode::OK);
    let entries = response["entries"].as_array().unwrap();
    let low_pair = entries
        .iter()
        .find(|entry| entry["owner"]["id"] == TEAM_TWO_A.to_string())
        .unwrap();
    let mixed_pair = entries
        .iter()
        .find(|entry| entry["owner"]["id"] == TEAM_TWO_B.to_string())
        .unwrap();
    assert_eq!(low_pair["playing_handicap"], 14);
    assert_eq!(mixed_pair["playing_handicap"], 2);

    let (status, tournament) = get(
        &app,
        format!("/api/tournaments/{TOURNAMENT}/leaderboards/net"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let ada = tournament["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["player_id"] == PLAYER_A.to_string())
        .unwrap();
    assert_eq!(ada["completed_rounds"], 1);
    assert_eq!(ada["net_total"], -4);
}

#[sqlx::test(migrations = "../migrations")]
async fn round_api_handles_draft_missing_partial_plus_handicap_and_exact_contract(pool: PgPool) {
    seed(&pool).await;
    open(&pool, ROUND_ONE).await;
    save(
        &pool,
        ROUND_ONE,
        HOLE_TWO,
        ScoreOwner::Player { id: PLAYER_PLUS },
        4,
    )
    .await;
    sqlx::query("UPDATE players SET current_handicap_index = 30 WHERE id = $1")
        .bind(PLAYER_PLUS)
        .execute(&pool)
        .await
        .unwrap();
    let app = api::router(AppState::new(pool));

    let (status, response) = get(&app, format!("/api/rounds/{ROUND_ONE}/leaderboards/gross")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response,
        json!({
            "round_id": ROUND_ONE,
            "tournament_id": TOURNAMENT,
            "status": "open",
            "scoring_format": "individual_stroke_play",
            "metric": "gross",
            "number_of_holes": 2,
            "entries": [
                {"position": 1, "tied": false, "owner": {"type": "player", "id": PLAYER_PLUS}, "owner_name": "Plus", "members": [], "holes_scored": 1, "number_of_holes": 2, "complete": false, "confirmed": false, "playing_handicap": -1, "gross_total": 4, "net_total": 5, "par_played": 4, "score_to_par": 0},
                {"position": null, "tied": false, "owner": {"type": "player", "id": PLAYER_A}, "owner_name": "Ada", "members": [], "holes_scored": 0, "number_of_holes": 2, "complete": false, "confirmed": false, "playing_handicap": 8, "gross_total": 0, "net_total": 0, "par_played": 0, "score_to_par": 0},
                {"position": null, "tied": false, "owner": {"type": "player", "id": PLAYER_B}, "owner_name": "Bob", "members": [], "holes_scored": 0, "number_of_holes": 2, "complete": false, "confirmed": false, "playing_handicap": 20, "gross_total": 0, "net_total": 0, "par_played": 0, "score_to_par": 0},
                {"position": null, "tied": false, "owner": {"type": "player", "id": PLAYER_D}, "owner_name": "Zed", "members": [], "holes_scored": 0, "number_of_holes": 2, "complete": false, "confirmed": false, "playing_handicap": 5, "gross_total": 0, "net_total": 0, "par_played": 0, "score_to_par": 0}
            ]
        })
    );
    let (_, net) = get(&app, format!("/api/rounds/{ROUND_ONE}/leaderboards/net")).await;
    let mut expected_net = response.clone();
    expected_net["metric"] = json!("net");
    expected_net["entries"][0]["score_to_par"] = json!(1);
    assert_eq!(net, expected_net);

    let (_, draft) = get(&app, format!("/api/rounds/{ROUND_DRAFT}/leaderboards/net")).await;
    assert_eq!(draft["entries"], json!([]));
    let missing = Uuid::new_v4();
    let (status, missing_body) = get(&app, format!("/api/rounds/{missing}/leaderboards/net")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        missing_body,
        json!({"error":{"code":"not_found","message":"resource not found"}})
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn scramble_formula_members_and_disabled_scorecard_parity(pool: PgPool) {
    seed(&pool).await;
    open(&pool, ROUND_TWO).await;
    save(
        &pool,
        ROUND_TWO,
        HOLE_ONE,
        ScoreOwner::Team { id: TEAM_TWO_A },
        5,
    )
    .await;
    let app = api::router(AppState::new(pool.clone()));
    let (_, enabled) = get(&app, format!("/api/rounds/{ROUND_TWO}/leaderboards/net")).await;
    let entry = enabled["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["owner"]["id"] == TEAM_TWO_A.to_string())
        .unwrap();
    assert_eq!(entry["playing_handicap"], 6);
    assert_eq!(entry["net_total"], 2);
    assert_eq!(entry["members"][0]["player_id"], PLAYER_B.to_string());

    let disabled_pool = pool;
    let other_round = ROUND_THREE;
    sqlx::query("UPDATE rounds SET handicap_enabled = false, scoring_format = 'team_scramble' WHERE id = $1")
        .bind(other_round).execute(&disabled_pool).await.unwrap();
    open(&disabled_pool, other_round).await;
    save(
        &disabled_pool,
        other_round,
        HOLE_ONE,
        ScoreOwner::Team { id: CURRENT_A },
        5,
    )
    .await;
    let disabled_app = api::router(AppState::new(disabled_pool));
    let (_, leaderboard) = get(
        &disabled_app,
        format!("/api/rounds/{other_round}/leaderboards/net"),
    )
    .await;
    let selected = leaderboard["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["owner"]["id"] == CURRENT_A.to_string())
        .unwrap();
    let (_, scorecard) = get(
        &disabled_app,
        format!("/api/rounds/{other_round}/scorecards/team/{CURRENT_A}"),
    )
    .await;
    assert_eq!(selected["playing_handicap"], 0);
    assert_eq!(selected["gross_total"], selected["net_total"]);
    assert_eq!(scorecard["playing_handicap"], 0);
    assert_eq!(scorecard["gross_total"], scorecard["net_total"]);
}

#[sqlx::test(migrations = "../migrations")]
async fn tournament_api_aggregates_completed_rounds_and_keeps_current_teams(pool: PgPool) {
    seed(&pool).await;
    for round in [ROUND_ONE, ROUND_TWO, ROUND_THREE] {
        open(&pool, round).await;
    }
    open_latest_round(&pool).await;
    for (player, gross) in [
        (PLAYER_A, 4),
        (PLAYER_B, 5),
        (PLAYER_PLUS, 4),
        (PLAYER_D, 5),
    ] {
        let owner = ScoreOwner::Player { id: player };
        for hole in [HOLE_ONE, HOLE_TWO] {
            save(&pool, ROUND_ONE, hole, owner, gross).await;
        }
        scorecards::confirm(&pool, ROUND_ONE, owner, USER)
            .await
            .unwrap();
    }
    for (team, gross) in [(TEAM_TWO_A, 4), (TEAM_TWO_B, 5)] {
        let owner = ScoreOwner::Team { id: team };
        for hole in [HOLE_ONE, HOLE_TWO] {
            save(&pool, ROUND_TWO, hole, owner, gross).await;
        }
        scorecards::confirm(&pool, ROUND_TWO, owner, USER)
            .await
            .unwrap();
    }
    round_completion::complete(&pool, ROUND_ONE).await.unwrap();
    round_completion::complete(&pool, ROUND_TWO).await.unwrap();
    round_completion::lock(&pool, ROUND_TWO).await.unwrap();
    sqlx::query("UPDATE tournament_players SET status = 'withdrawn' WHERE tournament_id = $1 AND player_id = $2")
        .bind(TOURNAMENT).bind(PLAYER_D).execute(&pool).await.unwrap();
    save(
        &pool,
        ROUND_ONE,
        HOLE_ONE,
        ScoreOwner::Player { id: PLAYER_A },
        6,
    )
    .await;

    let app = api::router(AppState::new(pool));
    let (status, response) = get(
        &app,
        format!("/api/tournaments/{TOURNAMENT}/leaderboards/net"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response,
        json!({
            "tournament_id": TOURNAMENT,
            "metric": "net",
            "required_counted_rounds": 2,
            "current_round_id": ROUND_DRAFT,
            "included_round_ids": [ROUND_ONE, ROUND_TWO],
            "entries": [
                {"position": 1, "tied": false, "player_id": PLAYER_B, "display_name": "Bob", "status": "active", "completed_rounds": 2, "counted_contributions": 2, "eligible": true, "gross_total": 18, "net_total": -8, "par_total": 16, "score_to_par": -24, "contributions": [
                    {"round_id": ROUND_ONE, "owner": {"type": "player", "id": PLAYER_B}, "owner_name": "Bob", "gross_total": 10, "net_total": -10, "par_total": 8, "score_to_par": -18, "counted": true},
                    {"round_id": ROUND_TWO, "owner": {"type": "team", "id": TEAM_TWO_A}, "owner_name": "Low pair", "gross_total": 8, "net_total": 2, "par_total": 8, "score_to_par": -6, "counted": true}
                ], "current_team": {"round_id": ROUND_DRAFT, "team_id": LATEST_TEAM, "team_name": "Latest current"}},
                {"position": 2, "tied": false, "player_id": PLAYER_A, "display_name": "Ada", "status": "active", "completed_rounds": 2, "counted_contributions": 2, "eligible": true, "gross_total": 18, "net_total": 4, "par_total": 16, "score_to_par": -12, "contributions": [
                    {"round_id": ROUND_ONE, "owner": {"type": "player", "id": PLAYER_A}, "owner_name": "Ada", "gross_total": 10, "net_total": 2, "par_total": 8, "score_to_par": -6, "counted": true},
                    {"round_id": ROUND_TWO, "owner": {"type": "team", "id": TEAM_TWO_A}, "owner_name": "Low pair", "gross_total": 8, "net_total": 2, "par_total": 8, "score_to_par": -6, "counted": true}
                ], "current_team": {"round_id": ROUND_DRAFT, "team_id": LATEST_TEAM, "team_name": "Latest current"}},
                {"position": 3, "tied": false, "player_id": PLAYER_D, "display_name": "Zed", "status": "withdrawn", "completed_rounds": 2, "counted_contributions": 2, "eligible": true, "gross_total": 20, "net_total": 15, "par_total": 16, "score_to_par": -1, "contributions": [
                    {"round_id": ROUND_ONE, "owner": {"type": "player", "id": PLAYER_D}, "owner_name": "Zed", "gross_total": 10, "net_total": 5, "par_total": 8, "score_to_par": -3, "counted": true},
                    {"round_id": ROUND_TWO, "owner": {"type": "team", "id": TEAM_TWO_B}, "owner_name": "Mixed pair", "gross_total": 10, "net_total": 10, "par_total": 8, "score_to_par": 2, "counted": true}
                ], "current_team": {"round_id": ROUND_DRAFT, "team_id": LATEST_TEAM_TWO, "team_name": "Latest current two"}},
                {"position": 4, "tied": false, "player_id": PLAYER_PLUS, "display_name": "Plus", "status": "active", "completed_rounds": 2, "counted_contributions": 2, "eligible": true, "gross_total": 18, "net_total": 19, "par_total": 16, "score_to_par": 3, "contributions": [
                    {"round_id": ROUND_ONE, "owner": {"type": "player", "id": PLAYER_PLUS}, "owner_name": "Plus", "gross_total": 8, "net_total": 9, "par_total": 8, "score_to_par": 1, "counted": true},
                    {"round_id": ROUND_TWO, "owner": {"type": "team", "id": TEAM_TWO_B}, "owner_name": "Mixed pair", "gross_total": 10, "net_total": 10, "par_total": 8, "score_to_par": 2, "counted": true}
                ], "current_team": {"round_id": ROUND_DRAFT, "team_id": LATEST_TEAM_TWO, "team_name": "Latest current two"}},
                {"position": null, "tied": false, "player_id": PLAYER_ZERO, "display_name": "Withdrawn zero", "status": "withdrawn", "completed_rounds": 0, "counted_contributions": 0, "eligible": false, "gross_total": 0, "net_total": 0, "par_total": 0, "score_to_par": 0, "contributions": [], "current_team": null}
            ]
        })
    );

    let missing = Uuid::new_v4();
    let (status, response) = get(
        &app,
        format!("/api/tournaments/{missing}/leaderboards/gross"),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        response,
        json!({"error":{"code":"not_found","message":"resource not found"}})
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn current_team_lookup_supports_already_open_legacy_individual_rounds(pool: PgPool) {
    seed(&pool).await;
    open_latest_round(&pool).await;
    sqlx::query("ALTER TABLE rounds DISABLE TRIGGER USER")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET scoring_format = 'individual_stroke_play' WHERE id = $1")
        .bind(ROUND_DRAFT)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE rounds ENABLE TRIGGER USER")
        .execute(&pool)
        .await
        .unwrap();

    let app = api::router(AppState::new(pool));
    let (status, response) = get(
        &app,
        format!("/api/tournaments/{TOURNAMENT}/leaderboards/net"),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(response["current_round_id"], ROUND_DRAFT.to_string());
    let entries = response["entries"].as_array().unwrap();
    let ada = entries
        .iter()
        .find(|entry| entry["player_id"] == PLAYER_A.to_string())
        .unwrap();
    assert_eq!(ada["current_team"]["team_id"], LATEST_TEAM.to_string());
    let plus = entries
        .iter()
        .find(|entry| entry["player_id"] == PLAYER_PLUS.to_string())
        .unwrap();
    assert_eq!(plus["current_team"]["team_id"], LATEST_TEAM_TWO.to_string());
}

#[sqlx::test(migrations = "../migrations")]
async fn invalid_stored_owner_returns_internal_error(pool: PgPool) {
    seed(&pool).await;
    open(&pool, ROUND_ONE).await;
    sqlx::query("ALTER TABLE teams DISABLE TRIGGER USER")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO teams (id, round_id, tournament_id, name) VALUES ($1, $2, $3, 'Invalid owner')",
    )
    .bind(TEAM_ONE)
    .bind(ROUND_ONE)
    .bind(TOURNAMENT)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("ALTER TABLE scores DISABLE TRIGGER USER")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO scores (id, round_id, tournament_id, hole_id, team_id, gross_strokes, submitted_by) VALUES ($1, $2, $3, $4, $5, 4, $6)")
        .bind(Uuid::new_v4()).bind(ROUND_ONE).bind(TOURNAMENT).bind(HOLE_ONE).bind(TEAM_ONE).bind(USER)
        .execute(&pool).await.unwrap();
    let app = api::router(AppState::new(pool));
    let (status, response) = get(&app, format!("/api/rounds/{ROUND_ONE}/leaderboards/gross")).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response,
        json!({"error":{"code":"internal_error","message":"internal server error"}})
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn incomplete_completed_round_returns_exact_internal_error(pool: PgPool) {
    seed(&pool).await;
    open(&pool, ROUND_ONE).await;
    for (player, gross) in [
        (PLAYER_A, 4),
        (PLAYER_B, 5),
        (PLAYER_PLUS, 4),
        (PLAYER_D, 5),
    ] {
        let owner = ScoreOwner::Player { id: player };
        for hole in [HOLE_ONE, HOLE_TWO] {
            save(&pool, ROUND_ONE, hole, owner, gross).await;
        }
        scorecards::confirm(&pool, ROUND_ONE, owner, USER)
            .await
            .unwrap();
    }
    round_completion::complete(&pool, ROUND_ONE).await.unwrap();
    sqlx::query("ALTER TABLE scores DISABLE TRIGGER USER")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scores WHERE round_id = $1 AND player_id = $2 AND hole_id = $3")
        .bind(ROUND_ONE)
        .bind(PLAYER_A)
        .bind(HOLE_ONE)
        .execute(&pool)
        .await
        .unwrap();

    let app = api::router(AppState::new(pool));
    let expected = json!({"error":{"code":"internal_error","message":"internal server error"}});
    for path in [
        format!("/api/rounds/{ROUND_ONE}/leaderboards/net"),
        format!("/api/tournaments/{TOURNAMENT}/leaderboards/net"),
    ] {
        let (status, response) = get(&app, path).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response, expected);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn repeatable_read_does_not_mix_player_names_during_a_request(pool: PgPool) {
    seed(&pool).await;
    open(&pool, ROUND_ONE).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE round_handicap_snapshots IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    let request = tokio::spawn(async move {
        get(&app, format!("/api/rounds/{ROUND_ONE}/leaderboards/gross")).await
    });
    let mut observed_wait = false;
    for _ in 0..50 {
        observed_wait = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname = current_database() AND pid <> pg_backend_pid() AND wait_event_type = 'Lock' AND query LIKE 'SELECT rhs.round_id, rhs.player_id%')",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        if observed_wait {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(
        observed_wait,
        "leaderboard read did not reach the blocked snapshot query"
    );
    sqlx::query("UPDATE players SET display_name = 'Changed' WHERE id = $1")
        .bind(PLAYER_A)
        .execute(&pool)
        .await
        .unwrap();
    blocker.commit().await.unwrap();
    let (status, response) = request.await.unwrap();
    assert_eq!(status, StatusCode::OK);
    assert!(
        response["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["owner_name"] == "Ada")
    );
}
