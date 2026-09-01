#![cfg(feature = "database-tests")]

use std::{sync::Arc, time::Duration as StdDuration};

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::{auth, round_completion, round_lifecycle, tournaments},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::{sync::broadcast::error::TryRecvError, time::timeout};
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT_A: Uuid = uuid!("96000000-0000-0000-0000-000000000001");
const TOURNAMENT_B: Uuid = uuid!("96000000-0000-0000-0000-000000000002");
const ROUND_A: Uuid = uuid!("96000000-0000-0000-0000-000000000011");
const ROUND_B: Uuid = uuid!("96000000-0000-0000-0000-000000000012");
const HOLE_A: Uuid = uuid!("96000000-0000-0000-0000-000000000031");
const HOLE_B: Uuid = uuid!("96000000-0000-0000-0000-000000000032");
const FLIGHT_A: Uuid = uuid!("96000000-0000-0000-0000-000000000041");
const FLIGHT_B: Uuid = uuid!("96000000-0000-0000-0000-000000000042");
const TEAM_B: Uuid = uuid!("96000000-0000-0000-0000-000000000051");
const SHARED_PLAYER: Uuid = uuid!("96000000-0000-0000-0000-000000000101");
const PLAYER_B: Uuid = uuid!("96000000-0000-0000-0000-000000000102");
const ADMIN_A: Uuid = uuid!("96000000-0000-0000-0000-000000000201");
const ADMIN_B: Uuid = uuid!("96000000-0000-0000-0000-000000000202");
const SHARED_USER: Uuid = uuid!("96000000-0000-0000-0000-000000000203");
const PLAYER_B_USER: Uuid = uuid!("96000000-0000-0000-0000-000000000204");

const ADMIN_A_TOKEN: &str = "isolation-admin-a-token";
const ADMIN_B_TOKEN: &str = "isolation-admin-b-token";
const SHARED_TOKEN: &str = "isolation-shared-player-token";
const PLAYER_B_TOKEN: &str = "isolation-player-b-token";

const FIXTURE: &str = r#"
INSERT INTO players (id, display_name, current_handicap_index) VALUES
('96000000-0000-0000-0000-000000000101', 'Shared Player', 1.0),
('96000000-0000-0000-0000-000000000102', 'Tournament B Player', 2.0);
INSERT INTO users (id, username, display_name, role, player_id) VALUES
('96000000-0000-0000-0000-000000000201', 'isolation_admin_a', 'Admin A', 'admin', NULL),
('96000000-0000-0000-0000-000000000202', 'isolation_admin_b', 'Admin B', 'viewer', NULL),
('96000000-0000-0000-0000-000000000203', 'isolation_shared', 'Shared account', 'admin', '96000000-0000-0000-0000-000000000101'),
('96000000-0000-0000-0000-000000000204', 'isolation_player_b', 'Player B account', 'player', '96000000-0000-0000-0000-000000000102');
INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds) VALUES
('96000000-0000-0000-0000-000000000001', 'Isolated A', '2026-09-01', '2026-09-01', 1),
('96000000-0000-0000-0000-000000000002', 'Isolated B', '2026-09-02', '2026-09-02', 1);
INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap) VALUES
('96000000-0000-0000-0000-000000000001', '96000000-0000-0000-0000-000000000101', 8.0),
('96000000-0000-0000-0000-000000000002', '96000000-0000-0000-0000-000000000101', 18.0),
('96000000-0000-0000-0000-000000000002', '96000000-0000-0000-0000-000000000102', 22.0);
INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
('96000000-0000-0000-0000-000000000001', '96000000-0000-0000-0000-000000000201', 'admin'),
('96000000-0000-0000-0000-000000000001', '96000000-0000-0000-0000-000000000203', 'player'),
('96000000-0000-0000-0000-000000000002', '96000000-0000-0000-0000-000000000202', 'admin'),
('96000000-0000-0000-0000-000000000002', '96000000-0000-0000-0000-000000000203', 'player'),
('96000000-0000-0000-0000-000000000002', '96000000-0000-0000-0000-000000000204', 'player');
INSERT INTO courses (id, name) VALUES
('96000000-0000-0000-0000-000000000021', 'Course A'),
('96000000-0000-0000-0000-000000000022', 'Course B');
INSERT INTO tees (id, course_id, name, slope_rating, course_rating) VALUES
('96000000-0000-0000-0000-000000000025', '96000000-0000-0000-0000-000000000021', 'A tee', 113, 4.0),
('96000000-0000-0000-0000-000000000026', '96000000-0000-0000-0000-000000000022', 'B tee', 113, 4.0);
INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES
('96000000-0000-0000-0000-000000000031', '96000000-0000-0000-0000-000000000025', 1, 4, 1),
('96000000-0000-0000-0000-000000000032', '96000000-0000-0000-0000-000000000026', 1, 4, 1);
INSERT INTO rounds
  (id, tournament_id, round_number, name, round_date, course_id, course_name,
   tee_id, tee_name, number_of_holes, scoring_format, handicap_allowance_percent)
VALUES
('96000000-0000-0000-0000-000000000011', '96000000-0000-0000-0000-000000000001', 1,
 'Round A', '2026-09-01', '96000000-0000-0000-0000-000000000021', 'Course A',
 '96000000-0000-0000-0000-000000000025', 'A tee', 1, 'individual_stroke_play', 100),
('96000000-0000-0000-0000-000000000012', '96000000-0000-0000-0000-000000000002', 1,
 'Round B', '2026-09-02', '96000000-0000-0000-0000-000000000022', 'Course B',
 '96000000-0000-0000-0000-000000000026', 'B tee', 1, 'two_player_foursomes', 50);
INSERT INTO teams (id, round_id, tournament_id, name) VALUES
('96000000-0000-0000-0000-000000000051', '96000000-0000-0000-0000-000000000012', '96000000-0000-0000-0000-000000000002', 'B-only team');
INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id, display_order) VALUES
('96000000-0000-0000-0000-000000000051', '96000000-0000-0000-0000-000000000012', '96000000-0000-0000-0000-000000000002', '96000000-0000-0000-0000-000000000101', 1),
('96000000-0000-0000-0000-000000000051', '96000000-0000-0000-0000-000000000012', '96000000-0000-0000-0000-000000000002', '96000000-0000-0000-0000-000000000102', 2);
INSERT INTO flights (id, round_id, tournament_id, name) VALUES
('96000000-0000-0000-0000-000000000041', '96000000-0000-0000-0000-000000000011', '96000000-0000-0000-0000-000000000001', 'Flight A'),
('96000000-0000-0000-0000-000000000042', '96000000-0000-0000-0000-000000000012', '96000000-0000-0000-0000-000000000002', 'Flight B');
INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id, display_order) VALUES
('96000000-0000-0000-0000-000000000041', '96000000-0000-0000-0000-000000000011', '96000000-0000-0000-0000-000000000001', '96000000-0000-0000-0000-000000000101', 1),
('96000000-0000-0000-0000-000000000042', '96000000-0000-0000-0000-000000000012', '96000000-0000-0000-0000-000000000002', '96000000-0000-0000-0000-000000000101', 1),
('96000000-0000-0000-0000-000000000042', '96000000-0000-0000-0000-000000000012', '96000000-0000-0000-0000-000000000002', '96000000-0000-0000-0000-000000000102', 2);
"#;

async fn seed_open(pool: &PgPool) {
    sqlx::raw_sql(FIXTURE).execute(pool).await.unwrap();
    for (user_id, token) in [
        (ADMIN_A, ADMIN_A_TOKEN),
        (ADMIN_B, ADMIN_B_TOKEN),
        (SHARED_USER, SHARED_TOKEN),
        (PLAYER_B_USER, PLAYER_B_TOKEN),
    ] {
        auth::create_session(
            pool,
            user_id,
            &hash_session_token(token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
    }
    for (tournament_id, admin, round_id) in [
        (TOURNAMENT_A, ADMIN_A, ROUND_A),
        (TOURNAMENT_B, ADMIN_B, ROUND_B),
    ] {
        let updated_at = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id = $1")
            .bind(tournament_id)
            .fetch_one(pool)
            .await
            .unwrap();
        let session_id = sqlx::query_scalar(
            "SELECT id FROM user_sessions WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(admin)
        .fetch_one(pool)
        .await
        .unwrap();
        tournaments::start_authorized(pool, session_id, tournament_id, updated_at)
            .await
            .unwrap();
        round_lifecycle::open(pool, round_id).await.unwrap();
    }
}

fn get(path: String, token: Option<&str>) -> Request<Body> {
    let mut request = Request::get(path);
    if let Some(token) = token {
        request = request.header(header::COOKIE, format!("golf_session={token}"));
    }
    request.body(Body::empty()).unwrap()
}

fn save(round_id: Uuid, hole_id: Uuid, owner: Value, gross: i16, token: &str) -> Request<Body> {
    Request::put(format!("/api/rounds/{round_id}/scores"))
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::COOKIE, format!("golf_session={token}"))
        .header("x-csrf-token", derive_csrf_token(token))
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

fn confirm(round_id: Uuid, owner_type: &str, owner_id: Uuid, token: &str) -> Request<Body> {
    Request::post(format!(
        "/api/rounds/{round_id}/scorecards/{owner_type}/{owner_id}/confirm"
    ))
    .header(header::CONTENT_TYPE, "application/json")
    .header(header::COOKIE, format!("golf_session={token}"))
    .header("x-csrf-token", derive_csrf_token(token))
    .body(Body::from("{}"))
    .unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn reused_identity_keeps_target_participation_authority_and_results_isolated(pool: PgPool) {
    seed_open(&pool).await;
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));

    let shared_snapshots = sqlx::query_as::<_, (Uuid, Uuid, String)>(
        "SELECT round_id, tournament_id, handicap_index::text
         FROM round_handicap_snapshots
         WHERE player_id = $1
         ORDER BY round_id",
    )
    .bind(SHARED_PLAYER)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(shared_snapshots.len(), 2);
    assert_eq!(shared_snapshots[0].0, ROUND_A);
    assert_eq!(shared_snapshots[0].1, TOURNAMENT_A);
    assert_eq!(shared_snapshots[0].2, "8.0");
    assert_eq!(shared_snapshots[1].0, ROUND_B);
    assert_eq!(shared_snapshots[1].1, TOURNAMENT_B);
    assert_eq!(shared_snapshots[1].2, "18.0");
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM round_team_handicap_snapshots
             WHERE round_id = $1 AND tournament_id = $2 AND team_id = $3"
        )
        .bind(ROUND_B)
        .bind(TOURNAMENT_B)
        .bind(TEAM_B)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );

    let roster_a = app
        .clone()
        .oneshot(get(
            format!("/api/tournaments/{TOURNAMENT_A}/players"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(roster_a.status(), StatusCode::OK);
    let roster_a = json_body(roster_a).await;
    assert!(roster_a.to_string().contains(&SHARED_PLAYER.to_string()));
    assert!(roster_a.to_string().contains("8.0"));
    assert!(!roster_a.to_string().contains(&PLAYER_B.to_string()));

    let roster_b = app
        .clone()
        .oneshot(get(
            format!("/api/tournaments/{TOURNAMENT_B}/players"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(roster_b.status(), StatusCode::OK);
    let roster_b = json_body(roster_b).await;
    assert!(roster_b.to_string().contains(&PLAYER_B.to_string()));
    assert!(roster_b.to_string().contains(&SHARED_PLAYER.to_string()));
    assert!(roster_b.to_string().contains("18.0"));
    assert!(roster_b.to_string().contains("22.0"));

    let pairings_a = app
        .clone()
        .oneshot(get(
            format!("/api/rounds/{ROUND_A}/pairings"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(pairings_a.status(), StatusCode::OK);
    let pairings_a = json_body(pairings_a).await.to_string();
    assert!(pairings_a.contains(&FLIGHT_A.to_string()));
    assert!(pairings_a.contains(&SHARED_PLAYER.to_string()));
    assert!(!pairings_a.contains(&FLIGHT_B.to_string()));
    assert!(!pairings_a.contains(&TEAM_B.to_string()));
    assert!(!pairings_a.contains(&PLAYER_B.to_string()));

    let pairings_b = app
        .clone()
        .oneshot(get(
            format!("/api/rounds/{ROUND_B}/pairings"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(pairings_b.status(), StatusCode::OK);
    let pairings_b = json_body(pairings_b).await.to_string();
    assert!(pairings_b.contains(&FLIGHT_B.to_string()));
    assert!(pairings_b.contains(&TEAM_B.to_string()));
    assert!(pairings_b.contains(&SHARED_PLAYER.to_string()));
    assert!(pairings_b.contains(&PLAYER_B.to_string()));
    assert!(!pairings_b.contains(&FLIGHT_A.to_string()));

    let teams_a = app
        .clone()
        .oneshot(get(
            format!("/api/rounds/{ROUND_A}/teams"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(teams_a.status(), StatusCode::OK);
    assert_eq!(json_body(teams_a).await, json!([]));

    let teams_b = app
        .clone()
        .oneshot(get(
            format!("/api/rounds/{ROUND_B}/teams"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(teams_b.status(), StatusCode::OK);
    let teams_b = json_body(teams_b).await.to_string();
    assert!(teams_b.contains(&TEAM_B.to_string()));
    assert!(teams_b.contains(&SHARED_PLAYER.to_string()));
    assert!(teams_b.contains(&PLAYER_B.to_string()));
    assert!(!teams_b.contains(&ROUND_A.to_string()));

    let access_a = app
        .clone()
        .oneshot(get(
            format!("/api/rounds/{ROUND_A}/score-access"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(access_a.status(), StatusCode::OK);
    assert_eq!(
        json_body(access_a).await["writable_owners"],
        json!([{"type": "player", "id": SHARED_PLAYER}])
    );
    let access_b = app
        .clone()
        .oneshot(get(
            format!("/api/rounds/{ROUND_B}/score-access"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(access_b.status(), StatusCode::OK);
    assert_eq!(
        json_body(access_b).await["writable_owners"],
        json!([{"type": "team", "id": TEAM_B}])
    );

    for path in [
        format!("/api/tournaments/{TOURNAMENT_B}/players"),
        format!("/api/rounds/{ROUND_B}/pairings"),
        format!("/api/rounds/{ROUND_B}/teams"),
        format!("/api/rounds/{ROUND_B}/score-access"),
        format!("/api/rounds/{ROUND_B}/leaderboards/gross"),
        format!("/api/rounds/{ROUND_B}/leaderboards/net"),
        format!("/api/rounds/{ROUND_B}/scorecards/team/{TEAM_B}"),
        format!("/api/tournaments/{TOURNAMENT_B}/leaderboards/gross"),
        format!("/api/tournaments/{TOURNAMENT_B}/leaderboards/net"),
        format!("/api/tournaments/{TOURNAMENT_B}/live"),
    ] {
        let response = app
            .clone()
            .oneshot(get(path.clone(), Some(ADMIN_A_TOKEN)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
    }

    let mut rejected_events = state.live_events.subscribe();
    let rejected_save = app
        .clone()
        .oneshot(save(
            ROUND_B,
            HOLE_B,
            json!({"type": "team", "id": TEAM_B}),
            5,
            ADMIN_A_TOKEN,
        ))
        .await
        .unwrap();
    assert_eq!(rejected_save.status(), StatusCode::FORBIDDEN);
    let rejected_confirmation = app
        .clone()
        .oneshot(confirm(ROUND_B, "team", TEAM_B, ADMIN_A_TOKEN))
        .await
        .unwrap();
    assert_eq!(rejected_confirmation.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM scores WHERE round_id = $1")
            .bind(ROUND_B)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM scorecard_confirmations WHERE round_id = $1"
        )
        .bind(ROUND_B)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert!(matches!(
        rejected_events.try_recv(),
        Err(TryRecvError::Empty)
    ));

    let accepted_b = app
        .clone()
        .oneshot(save(
            ROUND_B,
            HOLE_B,
            json!({"type": "team", "id": TEAM_B}),
            5,
            ADMIN_B_TOKEN,
        ))
        .await
        .unwrap();
    assert_eq!(accepted_b.status(), StatusCode::OK);
    let event = rejected_events.try_recv().unwrap();
    assert_eq!(event.tournament_id, TOURNAMENT_B);
    assert_eq!(event.id, ROUND_B);
    assert_eq!(
        app.clone()
            .oneshot(save(
                ROUND_A,
                HOLE_A,
                json!({"type": "player", "id": SHARED_PLAYER}),
                4,
                SHARED_TOKEN,
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        sqlx::query_as::<_, (Uuid, Option<Uuid>, Option<Uuid>, i16)>(
            "SELECT round_id, player_id, team_id, gross_strokes
             FROM scores ORDER BY round_id"
        )
        .fetch_all(&pool)
        .await
        .unwrap(),
        vec![
            (ROUND_A, Some(SHARED_PLAYER), None, 4),
            (ROUND_B, None, Some(TEAM_B), 5)
        ]
    );

    for (round_id, owner_type, owner_id, token) in [
        (ROUND_A, "player", SHARED_PLAYER, SHARED_TOKEN),
        (ROUND_B, "team", TEAM_B, SHARED_TOKEN),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(confirm(round_id, owner_type, owner_id, token))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
    round_completion::complete(&pool, ROUND_A).await.unwrap();
    round_completion::complete(&pool, ROUND_B).await.unwrap();

    let scorecard_a = app
        .clone()
        .oneshot(get(
            format!("/api/rounds/{ROUND_A}/scorecards/player/{SHARED_PLAYER}"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(scorecard_a.status(), StatusCode::OK);
    let scorecard_a = json_body(scorecard_a).await;
    assert_eq!(scorecard_a["gross_total"], 4);
    assert_eq!(
        scorecard_a["owner"],
        json!({"type": "player", "id": SHARED_PLAYER})
    );
    assert!(!scorecard_a.to_string().contains(&TEAM_B.to_string()));

    let scorecard_b = app
        .clone()
        .oneshot(get(
            format!("/api/rounds/{ROUND_B}/scorecards/team/{TEAM_B}"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(scorecard_b.status(), StatusCode::OK);
    let scorecard_b = json_body(scorecard_b).await;
    assert_eq!(scorecard_b["gross_total"], 5);
    assert_eq!(scorecard_b["owner"], json!({"type": "team", "id": TEAM_B}));
    let scorecard_b = scorecard_b.to_string();
    assert!(scorecard_b.contains(&TEAM_B.to_string()));
    assert!(scorecard_b.contains(&ROUND_B.to_string()));
    assert!(!scorecard_b.contains(&ROUND_A.to_string()));

    for path in [
        format!("/api/rounds/{ROUND_B}/leaderboards/gross"),
        format!("/api/rounds/{ROUND_B}/leaderboards/net"),
        format!("/api/tournaments/{TOURNAMENT_B}/leaderboards/gross"),
        format!("/api/tournaments/{TOURNAMENT_B}/leaderboards/net"),
    ] {
        let response = app
            .clone()
            .oneshot(get(path.clone(), Some(SHARED_TOKEN)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let body = json_body(response).await.to_string();
        assert!(body.contains(&TEAM_B.to_string()), "{path}");
        assert!(body.contains(&SHARED_PLAYER.to_string()), "{path}");
        assert!(body.contains(&PLAYER_B.to_string()), "{path}");
        assert!(!body.contains(&ROUND_A.to_string()), "{path}");
    }

    for metric in ["gross", "net"] {
        for path in [
            format!("/api/rounds/{ROUND_A}/leaderboards/{metric}"),
            format!("/api/tournaments/{TOURNAMENT_A}/leaderboards/{metric}"),
        ] {
            let response = app
                .clone()
                .oneshot(get(path.clone(), Some(SHARED_TOKEN)))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            let body = json_body(response).await.to_string();
            assert!(body.contains(&SHARED_PLAYER.to_string()), "{path}");
            assert!(!body.contains(&TEAM_B.to_string()), "{path}");
            assert!(!body.contains(&PLAYER_B.to_string()), "{path}");
            assert!(!body.contains(&ROUND_B.to_string()), "{path}");
        }
    }

    let stream = app
        .oneshot(get(
            format!("/api/tournaments/{TOURNAMENT_A}/live"),
            Some(SHARED_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(stream.status(), StatusCode::OK);
    let mut stream_body = stream.into_body();
    state.notify("foreign_scope", TOURNAMENT_B, ROUND_B);
    assert!(
        timeout(StdDuration::from_millis(150), stream_body.frame())
            .await
            .is_err(),
        "a B-only event must not produce an A stream frame"
    );
    state.notify("target_scope", TOURNAMENT_A, ROUND_A);
    let frame = timeout(StdDuration::from_secs(2), stream_body.frame())
        .await
        .expect("target tournament event should arrive")
        .expect("stream should remain open")
        .expect("event frame should be valid");
    let event = std::str::from_utf8(&frame.into_data().unwrap())
        .unwrap()
        .to_owned();
    assert_eq!(event, "event: target_scope\ndata: invalidate\n\n");
    assert!(!event.contains("foreign_scope"));
    assert!(!event.contains(&TOURNAMENT_A.to_string()));
    assert!(!event.contains(&TOURNAMENT_B.to_string()));
    assert!(!event.contains(&ROUND_A.to_string()));
    assert!(!event.contains(&ROUND_B.to_string()));
}
