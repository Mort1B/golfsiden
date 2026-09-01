#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Method, Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::auth,
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("8a000000-0000-0000-0000-000000000001");
const HIDDEN_TOURNAMENT: Uuid = uuid!("8a000000-0000-0000-0000-000000000002");
const PLAYER: Uuid = uuid!("8a000000-0000-0000-0000-000000000003");
const MEMBER: Uuid = uuid!("8a000000-0000-0000-0000-000000000004");
const PLATFORM_ADMIN: Uuid = uuid!("8a000000-0000-0000-0000-000000000005");
const MEMBER_TOKEN: &str = "retired-routes-member-token";
const PLATFORM_ADMIN_TOKEN: &str = "retired-routes-platform-admin-token";

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index)
        VALUES ('8a000000-0000-0000-0000-000000000003', 'Scoped player', 11.2);
        INSERT INTO users (id, username, display_name, role, player_id) VALUES
        ('8a000000-0000-0000-0000-000000000004', 'retired_member', 'Member', 'viewer',
         '8a000000-0000-0000-0000-000000000003'),
        ('8a000000-0000-0000-0000-000000000005', 'retired_platform', 'Platform', 'admin', NULL);
        INSERT INTO tournaments
          (id, name, start_date, end_date, number_of_rounds, counted_rounds) VALUES
        ('8a000000-0000-0000-0000-000000000001', 'Scoped tournament',
         '2026-09-01', '2026-09-01', 1, 1),
        ('8a000000-0000-0000-0000-000000000002', 'Hidden tournament',
         '2026-10-01', '2026-10-01', 1, 1);
        INSERT INTO tournament_memberships (tournament_id, user_id, role)
        VALUES ('8a000000-0000-0000-0000-000000000001',
                '8a000000-0000-0000-0000-000000000004', 'player');
        INSERT INTO tournament_players
          (tournament_id, player_id, tournament_handicap)
        VALUES ('8a000000-0000-0000-0000-000000000001',
                '8a000000-0000-0000-0000-000000000003', 11.2);
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    for (user_id, token) in [
        (MEMBER, MEMBER_TOKEN),
        (PLATFORM_ADMIN, PLATFORM_ADMIN_TOKEN),
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

fn request(method: Method, path: &str, token: Option<&str>) -> Request<Body> {
    let mutation = method != Method::GET;
    let mut builder = Request::builder().method(method).uri(path);
    if let Some(token) = token {
        builder = builder.header(header::COOKIE, format!("golf_session={token}"));
        if mutation {
            builder = builder.header("x-csrf-token", derive_csrf_token(token));
        }
    }
    if mutation {
        builder = builder.header(header::CONTENT_TYPE, "application/json");
    }
    builder
        .body(Body::from(if mutation { "{}" } else { "" }))
        .unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn legacy_player_and_tournament_creation_methods_are_unrouted_for_every_session(
    pool: PgPool,
) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let initial_tournament_count = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournaments")
        .fetch_one(&pool)
        .await
        .unwrap();
    let player_id = PLAYER.to_string();
    let retired = [
        (Method::GET, "/api/players".to_owned()),
        (Method::POST, "/api/players".to_owned()),
        (Method::GET, format!("/api/players/{player_id}")),
        (Method::PATCH, format!("/api/players/{player_id}")),
        (Method::DELETE, format!("/api/players/{player_id}")),
        (Method::GET, format!("/api/players/{player_id}/handicaps")),
        (Method::POST, format!("/api/players/{player_id}/handicaps")),
    ];

    for token in [None, Some(MEMBER_TOKEN), Some(PLATFORM_ADMIN_TOKEN)] {
        for (method, path) in &retired {
            let response = app
                .clone()
                .oneshot(request(method.clone(), path, token))
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{method} {path}");
        }
        let response = app
            .clone()
            .oneshot(request(Method::POST, "/api/tournaments", token))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournaments")
            .fetch_one(&pool)
            .await
            .unwrap(),
        initial_tournament_count
    );
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn tournament_collection_and_private_roster_remain_membership_scoped(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool));

    let anonymous = app
        .clone()
        .oneshot(request(Method::GET, "/api/tournaments", None))
        .await
        .unwrap();
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        json_body(anonymous).await["error"]["code"],
        "unauthenticated"
    );

    let member = app
        .clone()
        .oneshot(request(Method::GET, "/api/tournaments", Some(MEMBER_TOKEN)))
        .await
        .unwrap();
    assert_eq!(member.status(), StatusCode::OK);
    assert_eq!(member.headers()[header::CACHE_CONTROL], "private, no-store");
    let tournaments = json_body(member).await;
    assert_eq!(tournaments.as_array().unwrap().len(), 1);
    assert_eq!(tournaments[0]["id"], TOURNAMENT.to_string());
    assert_ne!(tournaments[0]["id"], HIDDEN_TOURNAMENT.to_string());

    let platform_admin = app
        .clone()
        .oneshot(request(
            Method::GET,
            "/api/tournaments",
            Some(PLATFORM_ADMIN_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(platform_admin.status(), StatusCode::OK);
    assert_eq!(
        platform_admin.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(json_body(platform_admin).await, serde_json::json!([]));

    let roster = app
        .clone()
        .oneshot(request(
            Method::GET,
            &format!("/api/tournaments/{TOURNAMENT}/players"),
            Some(MEMBER_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(roster.status(), StatusCode::OK);
    assert_eq!(roster.headers()[header::CACHE_CONTROL], "private, no-store");
    let roster = json_body(roster).await;
    assert_eq!(roster["players"].as_array().unwrap().len(), 1);
    assert_eq!(roster["players"][0]["player_id"], PLAYER.to_string());

    let global_admin_roster = app
        .oneshot(request(
            Method::GET,
            &format!("/api/tournaments/{TOURNAMENT}/players"),
            Some(PLATFORM_ADMIN_TOKEN),
        ))
        .await
        .unwrap();
    assert_eq!(global_admin_roster.status(), StatusCode::FORBIDDEN);
}
