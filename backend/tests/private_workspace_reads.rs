#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use golf_api::{AppState, api, auth::hash_session_token, repositories::auth};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("91000000-0000-0000-0000-000000000001");
const OTHER_TOURNAMENT: Uuid = uuid!("91000000-0000-0000-0000-000000000002");
const ROUND: Uuid = uuid!("91000000-0000-0000-0000-000000000011");
const OTHER_ROUND: Uuid = uuid!("91000000-0000-0000-0000-000000000012");

const MEMBERS: [(Uuid, &str, &str); 4] = [
    (
        uuid!("91000000-0000-0000-0000-000000000101"),
        "admin",
        "private-admin-token",
    ),
    (
        uuid!("91000000-0000-0000-0000-000000000102"),
        "scorer",
        "private-scorer-token",
    ),
    (
        uuid!("91000000-0000-0000-0000-000000000103"),
        "player",
        "private-player-token",
    ),
    (
        uuid!("91000000-0000-0000-0000-000000000104"),
        "viewer",
        "private-viewer-token",
    ),
];
const OUTSIDER: Uuid = uuid!("91000000-0000-0000-0000-000000000105");
const OUTSIDER_TOKEN: &str = "private-outsider-token";

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO users (id, username, display_name, role) VALUES
        ('91000000-0000-0000-0000-000000000101', 'private_admin', 'Admin', 'viewer'),
        ('91000000-0000-0000-0000-000000000102', 'private_scorer', 'Scorer', 'admin'),
        ('91000000-0000-0000-0000-000000000103', 'private_player', 'Player', 'admin'),
        ('91000000-0000-0000-0000-000000000104', 'private_viewer', 'Viewer', 'admin'),
        ('91000000-0000-0000-0000-000000000105', 'private_outsider', 'Outsider', 'admin');
        INSERT INTO players (id, display_name, current_handicap_index)
        VALUES ('91000000-0000-0000-0000-000000000201', 'Entrant', 12.0);
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds) VALUES
        ('91000000-0000-0000-0000-000000000001', 'Private Cup', '2026-08-01', '2026-08-02', 1),
        ('91000000-0000-0000-0000-000000000002', 'Private Cup', '2026-08-01', '2026-08-02', 1),
        ('91000000-0000-0000-0000-000000000003', 'Hidden Cup', '2027-09-01', '2027-09-02', 1);
        INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
        VALUES ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000201', 12.0);
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_name, tee_name,
           scoring_format)
        VALUES
          ('91000000-0000-0000-0000-000000000011',
           '91000000-0000-0000-0000-000000000001', 1, 'Private round',
           '2026-08-01', 'TBD', 'TBD', 'individual_stroke_play'),
          ('91000000-0000-0000-0000-000000000012',
           '91000000-0000-0000-0000-000000000002', 1, 'Other round',
           '2026-08-01', 'TBD', 'TBD', 'individual_stroke_play');
        INSERT INTO teams (id, round_id, tournament_id, name)
        VALUES ('91000000-0000-0000-0000-000000000021',
                '91000000-0000-0000-0000-000000000011',
                '91000000-0000-0000-0000-000000000001', 'Group'),
               ('91000000-0000-0000-0000-000000000022',
                '91000000-0000-0000-0000-000000000012',
                '91000000-0000-0000-0000-000000000002', 'Other group');
        INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id)
        VALUES ('91000000-0000-0000-0000-000000000021',
                '91000000-0000-0000-0000-000000000011',
                '91000000-0000-0000-0000-000000000001',
                '91000000-0000-0000-0000-000000000201');
        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
        ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000101', 'admin'),
        ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000102', 'scorer'),
        ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000103', 'player'),
        ('91000000-0000-0000-0000-000000000001', '91000000-0000-0000-0000-000000000104', 'viewer'),
        ('91000000-0000-0000-0000-000000000002', '91000000-0000-0000-0000-000000000104', 'viewer');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    for (user_id, _, token) in MEMBERS
        .into_iter()
        .chain([(OUTSIDER, "outsider", OUTSIDER_TOKEN)])
    {
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

fn scoped_paths(tournament_id: Uuid, round_id: Uuid) -> Vec<String> {
    vec![
        format!("/api/tournaments/{tournament_id}"),
        format!("/api/tournaments/{tournament_id}/players"),
        format!("/api/tournaments/{tournament_id}/rounds"),
        format!("/api/rounds/{round_id}"),
        format!("/api/rounds/{round_id}/pairing-validation"),
        format!("/api/rounds/{round_id}/completion-validation"),
        format!("/api/rounds/{round_id}/teams"),
        format!("/api/rounds/{round_id}/leaderboards/gross"),
        format!("/api/rounds/{round_id}/leaderboards/net"),
        format!("/api/tournaments/{tournament_id}/leaderboards/gross"),
        format!("/api/tournaments/{tournament_id}/leaderboards/net"),
    ]
}

fn get(path: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::get(path);
    if let Some(token) = token {
        builder = builder.header(header::COOKIE, format!("golf_session={token}"));
    }
    builder.body(Body::empty()).unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn every_membership_role_can_read_private_workspace_without_shared_caching(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool));
    for (_, role, token) in MEMBERS {
        for path in scoped_paths(TOURNAMENT, ROUND) {
            let response = app.clone().oneshot(get(&path, Some(token))).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK, "{role}: {path}");
            assert_eq!(
                response.headers().get(header::CACHE_CONTROL).unwrap(),
                "private, no-store",
                "{role}: {path}"
            );
        }
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn collection_is_membership_filtered_and_private(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool));
    let response = app
        .clone()
        .oneshot(get("/api/tournaments", Some("private-viewer-token")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let body = json_body(response).await;
    assert_eq!(body.as_array().unwrap().len(), 2);
    assert_eq!(body[0]["id"], TOURNAMENT.to_string());
    assert_eq!(body[1]["id"], OTHER_TOURNAMENT.to_string());

    let outsider = app
        .oneshot(get("/api/tournaments", Some(OUTSIDER_TOKEN)))
        .await
        .unwrap();
    assert_eq!(outsider.status(), StatusCode::OK);
    assert_eq!(
        outsider.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(json_body(outsider).await, serde_json::json!([]));
}

#[sqlx::test(migrations = "../migrations")]
async fn membership_in_one_tournament_does_not_authorize_other_resource_shapes(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool));
    for path in scoped_paths(OTHER_TOURNAMENT, OTHER_ROUND) {
        let response = app
            .clone()
            .oneshot(get(&path, Some("private-admin-token")))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
        assert_eq!(json_body(response).await["error"]["code"], "forbidden");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn private_reads_distinguish_authentication_membership_and_missing_resources(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool));
    let missing = Uuid::new_v4();
    let mut paths = scoped_paths(TOURNAMENT, ROUND);
    paths.push("/api/tournaments".to_owned());
    for path in &paths {
        for token in [None, Some("unknown-private-token")] {
            let response = app.clone().oneshot(get(path, token)).await.unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
            assert_eq!(
                json_body(response).await["error"]["code"],
                "unauthenticated"
            );
        }
    }
    for path in scoped_paths(TOURNAMENT, ROUND) {
        let response = app
            .clone()
            .oneshot(get(&path, Some(OUTSIDER_TOKEN)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}");
        assert_eq!(json_body(response).await["error"]["code"], "forbidden");
    }
    for path in scoped_paths(missing, missing) {
        let response = app
            .clone()
            .oneshot(get(&path, Some("private-admin-token")))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        assert_eq!(json_body(response).await["error"]["code"], "not_found");
    }
}
