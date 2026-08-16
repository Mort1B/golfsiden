#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header, request::Builder},
};
use chrono::{Duration, Utc};
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

const TOURNAMENT_B: Uuid = uuid!("82000000-0000-0000-0000-000000000002");
const ROUND_B: Uuid = uuid!("82000000-0000-0000-0000-000000000003");
const TEAM_B: Uuid = uuid!("82000000-0000-0000-0000-000000000004");
const PLAYER_LINKED: Uuid = uuid!("82000000-0000-0000-0000-000000000005");
const PLAYER_NEW: Uuid = uuid!("82000000-0000-0000-0000-000000000006");
const ADMIN_A: Uuid = uuid!("82000000-0000-0000-0000-000000000011");
const ADMIN_B: Uuid = uuid!("82000000-0000-0000-0000-000000000012");
const SCORER_B: Uuid = uuid!("82000000-0000-0000-0000-000000000013");
const PLAYER_B: Uuid = uuid!("82000000-0000-0000-0000-000000000014");
const VIEWER_B: Uuid = uuid!("82000000-0000-0000-0000-000000000015");

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index) VALUES
        ('82000000-0000-0000-0000-000000000005', 'Linked', 10.0),
        ('82000000-0000-0000-0000-000000000006', 'New entrant', 14.0);
        INSERT INTO users (id, email, display_name, role, player_id) VALUES
        ('82000000-0000-0000-0000-000000000011', 'admin-a@test', 'Admin A', 'admin', NULL),
        ('82000000-0000-0000-0000-000000000012', 'admin-b@test', 'Admin B', 'viewer', NULL),
        ('82000000-0000-0000-0000-000000000013', 'scorer-b@test', 'Scorer B', 'admin', NULL),
        ('82000000-0000-0000-0000-000000000014', 'player-b@test', 'Player B', 'admin', '82000000-0000-0000-0000-000000000005'),
        ('82000000-0000-0000-0000-000000000015', 'viewer-b@test', 'Viewer B', 'admin', NULL);
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds) VALUES
        ('82000000-0000-0000-0000-000000000001', 'Alpha', '2026-01-01', '2026-01-02', 2),
        ('82000000-0000-0000-0000-000000000002', 'Beta', '2026-02-01', '2026-02-02', 2);
        INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
        VALUES ('82000000-0000-0000-0000-000000000002', '82000000-0000-0000-0000-000000000005', 7.0);
        INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_name, tee_name, scoring_format)
        VALUES ('82000000-0000-0000-0000-000000000003', '82000000-0000-0000-0000-000000000002', 1, 'Beta round', '2026-02-01', 'TBD', 'TBD', 'team_scramble');
        INSERT INTO teams (id, round_id, tournament_id, name)
        VALUES ('82000000-0000-0000-0000-000000000004', '82000000-0000-0000-0000-000000000003', '82000000-0000-0000-0000-000000000002', 'Beta team');
        INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id)
        VALUES ('82000000-0000-0000-0000-000000000004', '82000000-0000-0000-0000-000000000003', '82000000-0000-0000-0000-000000000002', '82000000-0000-0000-0000-000000000005');
        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
        ('82000000-0000-0000-0000-000000000001', '82000000-0000-0000-0000-000000000011', 'admin'),
        ('82000000-0000-0000-0000-000000000002', '82000000-0000-0000-0000-000000000012', 'admin'),
        ('82000000-0000-0000-0000-000000000002', '82000000-0000-0000-0000-000000000013', 'scorer'),
        ('82000000-0000-0000-0000-000000000002', '82000000-0000-0000-0000-000000000014', 'player'),
        ('82000000-0000-0000-0000-000000000002', '82000000-0000-0000-0000-000000000015', 'viewer');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    for (user_id, token) in [
        (ADMIN_A, "admin-a-token"),
        (ADMIN_B, "admin-b-token"),
        (SCORER_B, "scorer-b-token"),
        (PLAYER_B, "player-b-token"),
        (VIEWER_B, "viewer-b-token"),
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
}

fn authorized(builder: Builder, token: &str) -> Builder {
    builder
        .header(header::COOKIE, format!("golf_session={token}"))
        .header("x-csrf-token", derive_csrf_token(token))
}

fn json_request(builder: Builder, token: &str, value: Value) -> Request<Body> {
    authorized(builder, token)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn cross_tournament_admin_is_denied_through_every_resource_shape(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool));
    let requests = vec![
        json_request(
            Request::post(format!("/api/tournaments/{TOURNAMENT_B}/players")),
            "admin-a-token",
            json!({"player_id": PLAYER_NEW}),
        ),
        json_request(
            Request::post(format!(
                "/api/tournaments/{TOURNAMENT_B}/players/{PLAYER_LINKED}/handicaps"
            )),
            "admin-a-token",
            json!({"handicap_index": 5.0}),
        ),
        json_request(
            Request::post(format!("/api/tournaments/{TOURNAMENT_B}/rounds")),
            "admin-a-token",
            json!({"round_number":2,"name":"Second","round_date":"2026-02-02","course_name":"TBD","tee_name":"TBD","scoring_format":"individual_stroke_play"}),
        ),
        json_request(
            Request::post(format!("/api/rounds/{ROUND_B}/teams")),
            "admin-a-token",
            json!({"name":"Other"}),
        ),
        json_request(
            Request::post(format!("/api/teams/{TEAM_B}/members")),
            "admin-a-token",
            json!({"player_id": PLAYER_NEW}),
        ),
        authorized(
            Request::delete(format!("/api/teams/{TEAM_B}/members/{PLAYER_LINKED}")),
            "admin-a-token",
        )
        .body(Body::empty())
        .unwrap(),
        authorized(
            Request::post(format!("/api/rounds/{ROUND_B}/open")),
            "admin-a-token",
        )
        .body(Body::empty())
        .unwrap(),
        authorized(
            Request::post(format!("/api/rounds/{ROUND_B}/complete")),
            "admin-a-token",
        )
        .body(Body::empty())
        .unwrap(),
        authorized(
            Request::post(format!("/api/rounds/{ROUND_B}/lock")),
            "admin-a-token",
        )
        .body(Body::empty())
        .unwrap(),
    ];
    for request in requests {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_eq!(body(response).await["error"]["code"], "forbidden");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn tournament_roles_sessions_csrf_and_me_read_model_are_authoritative(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    for token in ["scorer-b-token", "player-b-token", "viewer-b-token"] {
        let response = app
            .clone()
            .oneshot(json_request(
                Request::post(format!("/api/tournaments/{TOURNAMENT_B}/players")),
                token,
                json!({"player_id": PLAYER_NEW}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let handicap_response = app
            .clone()
            .oneshot(json_request(
                Request::post(format!(
                    "/api/tournaments/{TOURNAMENT_B}/players/{PLAYER_LINKED}/handicaps"
                )),
                token,
                json!({"handicap_index": 5.0}),
            ))
            .await
            .unwrap();
        assert_eq!(handicap_response.status(), StatusCode::FORBIDDEN);
    }
    let no_session = app
        .clone()
        .oneshot(
            Request::post(format!("/api/tournaments/{TOURNAMENT_B}/players"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_session.status(), StatusCode::UNAUTHORIZED);
    let invalid_session = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/tournaments/{TOURNAMENT_B}/players")),
            "unknown-session-token",
            json!({"player_id": PLAYER_NEW}),
        ))
        .await
        .unwrap();
    assert_eq!(invalid_session.status(), StatusCode::UNAUTHORIZED);
    let no_csrf = app
        .clone()
        .oneshot(
            Request::post(format!("/api/tournaments/{TOURNAMENT_B}/players"))
                .header(header::COOKIE, "golf_session=admin-b-token")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"player_id": PLAYER_NEW}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(no_csrf.status(), StatusCode::FORBIDDEN);

    let expired = auth::create_session(
        &pool,
        ADMIN_B,
        &hash_session_token("expired-admin-token"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    sqlx::query(
        "UPDATE user_sessions SET created_at = now() - interval '2 hours',
         expires_at = now() - interval '1 hour' WHERE id = $1",
    )
    .bind(expired.session_id)
    .execute(&pool)
    .await
    .unwrap();
    let expired_response = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/tournaments/{TOURNAMENT_B}/players")),
            "expired-admin-token",
            json!({"player_id": PLAYER_NEW}),
        ))
        .await
        .unwrap();
    assert_eq!(expired_response.status(), StatusCode::UNAUTHORIZED);

    let revoked = auth::create_session(
        &pool,
        ADMIN_B,
        &hash_session_token("revoked-admin-token"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    auth::revoke_session(&pool, revoked.session_id)
        .await
        .unwrap();
    let revoked_response = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/tournaments/{TOURNAMENT_B}/players")),
            "revoked-admin-token",
            json!({"player_id": PLAYER_NEW}),
        ))
        .await
        .unwrap();
    assert_eq!(revoked_response.status(), StatusCode::UNAUTHORIZED);

    let added = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/tournaments/{TOURNAMENT_B}/players")),
            "admin-b-token",
            json!({"player_id": PLAYER_NEW, "tournament_handicap": 12.3}),
        ))
        .await
        .unwrap();
    assert_eq!(added.status(), StatusCode::CREATED);
    let audit_actor = sqlx::query_scalar::<_, Uuid>(
        "SELECT changed_by FROM tournament_handicap_history WHERE tournament_id = $1 AND player_id = $2",
    ).bind(TOURNAMENT_B).bind(PLAYER_NEW).fetch_one(&pool).await.unwrap();
    assert_eq!(audit_actor, ADMIN_B);

    let mine = app
        .oneshot(
            Request::get("/api/me/tournaments")
                .header(header::COOKIE, "golf_session=player-b-token")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(mine.status(), StatusCode::OK);
    let mine = body(mine).await;
    assert_eq!(mine.as_array().unwrap().len(), 1);
    assert_eq!(mine[0]["tournament"]["id"], TOURNAMENT_B.to_string());
    assert_eq!(mine[0]["role"], "player");
    assert_eq!(mine[0]["player_id"], PLAYER_LINKED.to_string());
}

#[sqlx::test(migrations = "../migrations")]
async fn tournament_handicap_change_is_audited_and_only_affects_future_rounds(pool: PgPool) {
    seed(&pool).await;
    let future_round = uuid!("82000000-0000-0000-0000-000000000031");
    sqlx::raw_sql(
        r#"
        UPDATE tournaments
        SET status = 'active'
        WHERE id = '82000000-0000-0000-0000-000000000002';
        INSERT INTO courses (id, name)
        VALUES ('82000000-0000-0000-0000-000000000021', 'Snapshot Course');
        INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
        VALUES ('82000000-0000-0000-0000-000000000022',
                '82000000-0000-0000-0000-000000000021', 'Test', 113, 4.0);
        INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
        VALUES ('82000000-0000-0000-0000-000000000023',
                '82000000-0000-0000-0000-000000000022', 1, 4, 1);
        UPDATE rounds
        SET course_id = '82000000-0000-0000-0000-000000000021',
            course_name = 'Snapshot Course',
            tee_id = '82000000-0000-0000-0000-000000000022',
            tee_name = 'Test', number_of_holes = 1,
            scoring_format = 'individual_stroke_play'
        WHERE id = '82000000-0000-0000-0000-000000000003';
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_id,
           course_name, tee_id, tee_name, number_of_holes, scoring_format)
        VALUES
          ('82000000-0000-0000-0000-000000000031',
           '82000000-0000-0000-0000-000000000002', 2, 'Future round',
           '2026-02-02', '82000000-0000-0000-0000-000000000021',
           'Snapshot Course', '82000000-0000-0000-0000-000000000022',
           'Test', 1, 'individual_stroke_play');
        INSERT INTO teams (id, round_id, tournament_id, name)
        VALUES ('82000000-0000-0000-0000-000000000032',
                '82000000-0000-0000-0000-000000000031',
                '82000000-0000-0000-0000-000000000002', 'Future group');
        INSERT INTO team_memberships
          (team_id, round_id, tournament_id, player_id)
        VALUES ('82000000-0000-0000-0000-000000000032',
                '82000000-0000-0000-0000-000000000031',
                '82000000-0000-0000-0000-000000000002',
                '82000000-0000-0000-0000-000000000005');
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    round_lifecycle::open(&pool, ROUND_B).await.unwrap();

    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let spoof = app
        .clone()
        .oneshot(json_request(
            Request::post(format!(
                "/api/tournaments/{TOURNAMENT_B}/players/{PLAYER_LINKED}/handicaps"
            )),
            "admin-b-token",
            json!({"handicap_index": 3.5, "changed_by": ADMIN_A}),
        ))
        .await
        .unwrap();
    assert_eq!(spoof.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body(spoof).await["error"]["code"], "validation_error");

    let changed = app
        .oneshot(json_request(
            Request::post(format!(
                "/api/tournaments/{TOURNAMENT_B}/players/{PLAYER_LINKED}/handicaps"
            )),
            "admin-b-token",
            json!({"handicap_index": 3.5, "reason": "trip review"}),
        ))
        .await
        .unwrap();
    assert_eq!(changed.status(), StatusCode::CREATED);
    let changed = body(changed).await;
    assert_eq!(changed["handicap_index"], 3.5);
    assert_eq!(changed["changed_by"], ADMIN_B.to_string());
    assert_eq!(events.try_recv().unwrap().id, TOURNAMENT_B);

    round_lifecycle::open(&pool, future_round).await.unwrap();
    let snapshots = sqlx::query_as::<_, (Uuid, f64)>(
        "SELECT round_id, handicap_index::float8
         FROM round_handicap_snapshots
         WHERE player_id = $1 ORDER BY round_id",
    )
    .bind(PLAYER_LINKED)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(snapshots, vec![(ROUND_B, 7.0), (future_round, 3.5)]);
    let profile_handicap = sqlx::query_scalar::<_, f64>(
        "SELECT current_handicap_index::float8 FROM players WHERE id = $1",
    )
    .bind(PLAYER_LINKED)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(profile_handicap, 10.0);
    let history = sqlx::query_as::<_, (f64, Option<Uuid>, Option<String>)>(
        "SELECT handicap_index::float8, changed_by, reason
         FROM tournament_handicap_history
         WHERE tournament_id = $1 AND player_id = $2",
    )
    .bind(TOURNAMENT_B)
    .bind(PLAYER_LINKED)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        history,
        vec![(3.5, Some(ADMIN_B), Some("trip review".to_owned()))]
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn platform_admin_actor_is_derived_and_spoof_fields_use_json_error_envelope(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    let spoof = app
        .clone()
        .oneshot(json_request(
            Request::post(format!("/api/players/{PLAYER_LINKED}/handicaps")),
            "admin-a-token",
            json!({"handicap_index": 4.2, "changed_by": ADMIN_B}),
        ))
        .await
        .unwrap();
    assert_eq!(spoof.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body(spoof).await["error"]["code"], "validation_error");

    let changed = app
        .oneshot(json_request(
            Request::post(format!("/api/players/{PLAYER_LINKED}/handicaps")),
            "admin-a-token",
            json!({"handicap_index": 4.2, "reason": "review"}),
        ))
        .await
        .unwrap();
    assert_eq!(changed.status(), StatusCode::CREATED);
    let actor = sqlx::query_scalar::<_, Uuid>(
        "SELECT changed_by FROM handicap_history WHERE player_id = $1 ORDER BY created_at DESC LIMIT 1",
    ).bind(PLAYER_LINKED).fetch_one(&pool).await.unwrap();
    assert_eq!(actor, ADMIN_A);
}
