#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::{auth, round_lifecycle},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("41000000-0000-0000-0000-000000000001");
const INDIVIDUAL: Uuid = uuid!("41000000-0000-0000-0000-000000000006");
const SCRAMBLE: Uuid = uuid!("41000000-0000-0000-0000-000000000007");
const DIRECT_ROUND: Uuid = uuid!("41000000-0000-0000-0000-000000000008");
const TEAM_A: Uuid = uuid!("41000000-0000-0000-0000-000000000021");
const TEAM_B: Uuid = uuid!("41000000-0000-0000-0000-000000000022");
const TEAM_C: Uuid = uuid!("41000000-0000-0000-0000-000000000023");
const DIRECT_TEAM: Uuid = uuid!("41000000-0000-0000-0000-000000000024");
const HOLE_1: Uuid = uuid!("41000000-0000-0000-0000-000000000031");
const HOLE_2: Uuid = uuid!("41000000-0000-0000-0000-000000000032");
const P1: Uuid = uuid!("41000000-0000-0000-0000-000000000041");
const P2: Uuid = uuid!("41000000-0000-0000-0000-000000000042");
const ADMIN: Uuid = uuid!("41000000-0000-0000-0000-000000000051");
const SCORER: Uuid = uuid!("41000000-0000-0000-0000-000000000052");
const USER_1: Uuid = uuid!("41000000-0000-0000-0000-000000000053");
const USER_5: Uuid = uuid!("41000000-0000-0000-0000-000000000054");
const VIEWER: Uuid = uuid!("41000000-0000-0000-0000-000000000055");
const UNLINKED: Uuid = uuid!("41000000-0000-0000-0000-000000000056");
const USER_3: Uuid = uuid!("41000000-0000-0000-0000-000000000057");

const FIXTURE: &str = r#"
INSERT INTO users (id, username, display_name, role) VALUES
('41000000-0000-0000-0000-000000000051', 'flight_admin', 'Admin', 'admin'),
('41000000-0000-0000-0000-000000000052', 'flight_scorer', 'Scorer', 'scorer'),
('41000000-0000-0000-0000-000000000055', 'flight_viewer', 'Viewer', 'viewer'),
('41000000-0000-0000-0000-000000000056', 'flight_unlinked', 'Unlinked', 'player');
INSERT INTO players (id, display_name, current_handicap_index) VALUES
('41000000-0000-0000-0000-000000000041', 'Zulu Player', 1.0),
('41000000-0000-0000-0000-000000000042', 'Alpha Player', 2.0),
('41000000-0000-0000-0000-000000000043', 'Bravo Player', 3.0),
('41000000-0000-0000-0000-000000000044', 'Charlie Player', 4.0),
('41000000-0000-0000-0000-000000000045', 'Delta Player', 5.0),
('41000000-0000-0000-0000-000000000046', 'Echo Player', 6.0);
INSERT INTO users (id, username, display_name, role, player_id) VALUES
('41000000-0000-0000-0000-000000000053', 'flight_player_1', 'Player 1', 'player', '41000000-0000-0000-0000-000000000041'),
('41000000-0000-0000-0000-000000000054', 'flight_player_5', 'Player 5', 'player', '41000000-0000-0000-0000-000000000045'),
('41000000-0000-0000-0000-000000000057', 'flight_player_3', 'Player 3', 'player', '41000000-0000-0000-0000-000000000043');
INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds, status)
VALUES ('41000000-0000-0000-0000-000000000001', 'Flight Score Cup', '2026-08-01', '2026-08-03', 3, 'active');
INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
SELECT '41000000-0000-0000-0000-000000000001', id, current_handicap_index FROM players;
INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
('41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000051', 'admin'),
('41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000052', 'scorer'),
('41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000053', 'player'),
('41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000054', 'player'),
('41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000057', 'player'),
('41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000055', 'viewer'),
('41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000056', 'player');
INSERT INTO courses (id, name) VALUES ('41000000-0000-0000-0000-000000000002', 'Flight Course');
INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
VALUES ('41000000-0000-0000-0000-000000000003', '41000000-0000-0000-0000-000000000002', 'Main', 113, 8.0);
INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES
('41000000-0000-0000-0000-000000000031', '41000000-0000-0000-0000-000000000003', 1, 4, 1),
('41000000-0000-0000-0000-000000000032', '41000000-0000-0000-0000-000000000003', 2, 4, 2);
INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format) VALUES
('41000000-0000-0000-0000-000000000006', '41000000-0000-0000-0000-000000000001', 1, 'Individual', '2026-08-01', '41000000-0000-0000-0000-000000000002', 'Flight Course', '41000000-0000-0000-0000-000000000003', 'Main', 2, 'individual_stroke_play'),
('41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', 2, 'Scramble', '2026-08-02', '41000000-0000-0000-0000-000000000002', 'Flight Course', '41000000-0000-0000-0000-000000000003', 'Main', 2, 'team_scramble'),
('41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', 3, 'Legacy direct', '2026-08-03', '41000000-0000-0000-0000-000000000002', 'Flight Course', '41000000-0000-0000-0000-000000000003', 'Main', 2, 'team_scramble');
INSERT INTO teams (id, round_id, tournament_id, name) VALUES
('41000000-0000-0000-0000-000000000021', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', 'Zulu Team'),
('41000000-0000-0000-0000-000000000022', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', 'Alpha Team'),
('41000000-0000-0000-0000-000000000023', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', 'Other Flight Team'),
('41000000-0000-0000-0000-000000000024', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', 'Direct Team'),
('41000000-0000-0000-0000-000000000025', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', 'Split Team');
INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id, display_order) VALUES
('41000000-0000-0000-0000-000000000021', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000041', 1),
('41000000-0000-0000-0000-000000000021', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000042', 2),
('41000000-0000-0000-0000-000000000022', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000043', 1),
('41000000-0000-0000-0000-000000000022', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000044', 2),
('41000000-0000-0000-0000-000000000023', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000045', 1),
('41000000-0000-0000-0000-000000000023', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000046', 2),
('41000000-0000-0000-0000-000000000024', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000041', 1),
('41000000-0000-0000-0000-000000000024', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000042', 2),
('41000000-0000-0000-0000-000000000025', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000043', 1),
('41000000-0000-0000-0000-000000000025', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000044', 2);
INSERT INTO flights (id, round_id, tournament_id, name) VALUES
('41000000-0000-0000-0000-000000000061', '41000000-0000-0000-0000-000000000006', '41000000-0000-0000-0000-000000000001', 'Individual One'),
('41000000-0000-0000-0000-000000000062', '41000000-0000-0000-0000-000000000006', '41000000-0000-0000-0000-000000000001', 'Individual Two'),
('41000000-0000-0000-0000-000000000063', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', 'Scramble One'),
('41000000-0000-0000-0000-000000000064', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', 'Scramble Two'),
('41000000-0000-0000-0000-000000000065', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', 'Legacy One'),
('41000000-0000-0000-0000-000000000066', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', 'Legacy Two');
INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id, display_order) VALUES
('41000000-0000-0000-0000-000000000061', '41000000-0000-0000-0000-000000000006', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000041', 1),
('41000000-0000-0000-0000-000000000061', '41000000-0000-0000-0000-000000000006', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000042', 2),
('41000000-0000-0000-0000-000000000062', '41000000-0000-0000-0000-000000000006', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000043', 1),
('41000000-0000-0000-0000-000000000062', '41000000-0000-0000-0000-000000000006', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000044', 2),
('41000000-0000-0000-0000-000000000062', '41000000-0000-0000-0000-000000000006', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000045', 3),
('41000000-0000-0000-0000-000000000062', '41000000-0000-0000-0000-000000000006', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000046', 4),
('41000000-0000-0000-0000-000000000063', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000041', 1),
('41000000-0000-0000-0000-000000000063', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000042', 2),
('41000000-0000-0000-0000-000000000063', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000043', 3),
('41000000-0000-0000-0000-000000000063', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000044', 4),
('41000000-0000-0000-0000-000000000064', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000045', 1),
('41000000-0000-0000-0000-000000000064', '41000000-0000-0000-0000-000000000007', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000046', 2),
('41000000-0000-0000-0000-000000000065', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000041', 1),
('41000000-0000-0000-0000-000000000065', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000042', 2),
('41000000-0000-0000-0000-000000000065', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000043', 3),
('41000000-0000-0000-0000-000000000066', '41000000-0000-0000-0000-000000000008', '41000000-0000-0000-0000-000000000001', '41000000-0000-0000-0000-000000000044', 1);
"#;

async fn seed(pool: &PgPool) {
    seed_with_team_format(pool, false).await;
}

async fn seed_with_team_format(pool: &PgPool, foursomes: bool) {
    sqlx::raw_sql(FIXTURE).execute(pool).await.unwrap();
    if foursomes {
        sqlx::query(
            "UPDATE rounds SET scoring_format = 'two_player_foursomes', handicap_allowance_percent = 50 WHERE id = $1",
        )
        .bind(SCRAMBLE)
        .execute(pool)
        .await
        .unwrap();
    }
    round_lifecycle::open(pool, INDIVIDUAL).await.unwrap();
    round_lifecycle::open(pool, SCRAMBLE).await.unwrap();

    let mut legacy = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.round_opening_id', $1, true)")
        .bind(DIRECT_ROUND.to_string())
        .execute(&mut *legacy)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO round_handicap_snapshots
           (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap)
         VALUES ($1, $2, $3, 1.0, 1, 1), ($1, $2, $4, 2.0, 2, 2),
                ($1, $2, $5, 3.0, 3, 3), ($1, $2, $6, 4.0, 4, 4)",
    )
    .bind(DIRECT_ROUND)
    .bind(TOURNAMENT)
    .bind(P1)
    .bind(P2)
    .bind(uuid!("41000000-0000-0000-0000-000000000043"))
    .bind(uuid!("41000000-0000-0000-0000-000000000044"))
    .execute(&mut *legacy)
    .await
    .unwrap();
    legacy.commit().await.unwrap();

    for user in [ADMIN, SCORER, USER_1, USER_3, USER_5, VIEWER, UNLINKED] {
        auth::create_session(
            pool,
            user,
            &hash_session_token(token(user)),
            Utc::now() + ChronoDuration::hours(1),
        )
        .await
        .unwrap();
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn foursomes_preserves_scramble_flight_wide_authority_parity(pool: PgPool) {
    seed_with_team_format(&pool, true).await;
    let app = api::router(AppState::new(pool));

    let access_response = app.clone().oneshot(access(SCRAMBLE, USER_1)).await.unwrap();
    assert_eq!(access_response.status(), StatusCode::OK);
    assert_eq!(
        body(access_response).await["writable_owners"],
        json!([
            {"type": "team", "id": TEAM_B},
            {"type": "team", "id": TEAM_A}
        ])
    );
    for hole in [HOLE_1, HOLE_2] {
        let response = app
            .clone()
            .oneshot(save(
                SCRAMBLE,
                hole,
                json!({"type": "team", "id": TEAM_B}),
                USER_1,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(
        app.oneshot(confirm(SCRAMBLE, "team", TEAM_B, USER_1))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
}

fn token(user: Uuid) -> &'static str {
    match user {
        ADMIN => "flight-score-admin-token",
        SCORER => "flight-score-scorer-token",
        USER_1 => "flight-score-player-one-token",
        USER_3 => "flight-score-player-three-token",
        USER_5 => "flight-score-player-five-token",
        VIEWER => "flight-score-viewer-token",
        UNLINKED => "flight-score-unlinked-token",
        _ => "flight-score-missing-token",
    }
}

fn access(round: Uuid, user: Uuid) -> Request<Body> {
    Request::get(format!("/api/rounds/{round}/score-access"))
        .header("cookie", format!("golf_session={}", token(user)))
        .body(Body::empty())
        .unwrap()
}

fn save(round: Uuid, hole: Uuid, owner: Value, user: Uuid) -> Request<Body> {
    Request::put(format!("/api/rounds/{round}/scores"))
        .header("content-type", "application/json")
        .header("cookie", format!("golf_session={}", token(user)))
        .header("x-csrf-token", derive_csrf_token(token(user)))
        .body(Body::from(
            json!({"hole_id": hole, "owner": owner, "gross_strokes": 4}).to_string(),
        ))
        .unwrap()
}

fn confirm(round: Uuid, kind: &str, owner: Uuid, user: Uuid) -> Request<Body> {
    Request::post(format!(
        "/api/rounds/{round}/scorecards/{kind}/{owner}/confirm"
    ))
    .header("content-type", "application/json")
    .header("cookie", format!("golf_session={}", token(user)))
    .header("x-csrf-token", derive_csrf_token(token(user)))
    .body(Body::from("{}"))
    .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn flight_members_list_save_and_confirm_every_eligible_card_without_crossing_flights(
    pool: PgPool,
) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool.clone()));

    let individual_access = app
        .clone()
        .oneshot(access(INDIVIDUAL, USER_1))
        .await
        .unwrap();
    assert_eq!(individual_access.status(), StatusCode::OK);
    assert_eq!(
        individual_access.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(
        body(individual_access).await["writable_owners"],
        json!([
            {"type": "player", "id": P2},
            {"type": "player", "id": P1}
        ])
    );
    for hole in [HOLE_1, HOLE_2] {
        let response = app
            .clone()
            .oneshot(save(
                INDIVIDUAL,
                hole,
                json!({"type": "player", "id": P2}),
                USER_1,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert_eq!(
        app.clone()
            .oneshot(confirm(INDIVIDUAL, "player", P2, USER_1))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let scramble_access = app.clone().oneshot(access(SCRAMBLE, USER_1)).await.unwrap();
    assert_eq!(
        body(scramble_access).await["writable_owners"],
        json!([
            {"type": "team", "id": TEAM_B},
            {"type": "team", "id": TEAM_A}
        ])
    );
    for hole in [HOLE_1, HOLE_2] {
        assert_eq!(
            app.clone()
                .oneshot(save(
                    SCRAMBLE,
                    hole,
                    json!({"type": "team", "id": TEAM_B}),
                    USER_1,
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        app.clone()
            .oneshot(confirm(SCRAMBLE, "team", TEAM_B, USER_1))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let second_team_member_access = app.clone().oneshot(access(SCRAMBLE, USER_3)).await.unwrap();
    assert_eq!(
        body(second_team_member_access).await["writable_owners"],
        json!([
            {"type": "team", "id": TEAM_B},
            {"type": "team", "id": TEAM_A}
        ])
    );
    for hole in [HOLE_1, HOLE_2] {
        assert_eq!(
            app.clone()
                .oneshot(save(
                    SCRAMBLE,
                    hole,
                    json!({"type": "team", "id": TEAM_A}),
                    USER_3,
                ))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        app.clone()
            .oneshot(confirm(SCRAMBLE, "team", TEAM_A, USER_3))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let denied = app
        .clone()
        .oneshot(save(
            SCRAMBLE,
            HOLE_1,
            json!({"type": "team", "id": TEAM_C}),
            USER_1,
        ))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM scores WHERE round_id = $1 AND team_id = $2",
        )
        .bind(SCRAMBLE)
        .bind(TEAM_C)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        app.clone()
            .oneshot(confirm(SCRAMBLE, "team", TEAM_A, USER_5))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    let cross_round = app
        .clone()
        .oneshot(save(
            SCRAMBLE,
            HOLE_1,
            json!({"type": "team", "id": DIRECT_TEAM}),
            USER_1,
        ))
        .await
        .unwrap();
    assert_eq!(cross_round.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(cross_round).await["error"]["code"],
        "score_owner_not_eligible"
    );

    let fallback = app.oneshot(access(DIRECT_ROUND, USER_1)).await.unwrap();
    assert_eq!(
        body(fallback).await["writable_owners"],
        json!([{"type": "team", "id": DIRECT_TEAM}])
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn exact_role_link_session_and_membership_remain_authoritative(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool.clone()));

    for privileged in [ADMIN, SCORER] {
        let response = app
            .clone()
            .oneshot(access(SCRAMBLE, privileged))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body(response).await["writable_owners"],
            json!([
                {"type": "team", "id": TEAM_B},
                {"type": "team", "id": TEAM_C},
                {"type": "team", "id": TEAM_A}
            ])
        );
    }
    for denied in [VIEWER, UNLINKED] {
        let response = app.clone().oneshot(access(SCRAMBLE, denied)).await.unwrap();
        assert_eq!(body(response).await["writable_owners"], json!([]));
    }

    sqlx::query("UPDATE users SET player_id = NULL WHERE id = $1")
        .bind(USER_1)
        .execute(&pool)
        .await
        .unwrap();
    let unlinked = app.clone().oneshot(access(SCRAMBLE, USER_1)).await.unwrap();
    assert_eq!(body(unlinked).await["writable_owners"], json!([]));
    sqlx::query("UPDATE users SET player_id = $1 WHERE id = $2")
        .bind(P1)
        .bind(USER_1)
        .execute(&pool)
        .await
        .unwrap();

    sqlx::query("UPDATE tournament_memberships SET role = 'viewer' WHERE tournament_id = $1 AND user_id = $2")
        .bind(TOURNAMENT)
        .bind(USER_1)
        .execute(&pool)
        .await
        .unwrap();
    let viewer = app.clone().oneshot(access(SCRAMBLE, USER_1)).await.unwrap();
    assert_eq!(body(viewer).await["writable_owners"], json!([]));
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id = $1 AND user_id = $2")
        .bind(TOURNAMENT)
        .bind(USER_1)
        .execute(&pool)
        .await
        .unwrap();
    let revoked_membership = app.clone().oneshot(access(SCRAMBLE, USER_1)).await.unwrap();
    assert_eq!(body(revoked_membership).await["writable_owners"], json!([]));

    sqlx::query("UPDATE user_sessions SET revoked_at = now() WHERE user_id = $1")
        .bind(USER_5)
        .execute(&pool)
        .await
        .unwrap();
    let revoked_session = app.oneshot(access(SCRAMBLE, USER_5)).await.unwrap();
    assert_eq!(revoked_session.status(), StatusCode::UNAUTHORIZED);
}
