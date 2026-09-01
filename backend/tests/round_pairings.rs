#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{DateTime, Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::{auth, tournaments},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::broadcast::error::TryRecvError;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const ROUND: Uuid = uuid!("b1000000-0000-0000-0000-000000000002");
const LEGACY_TEAM: Uuid = uuid!("b1000000-0000-0000-0000-000000000003");
const ADMIN: Uuid = uuid!("b1000000-0000-0000-0000-000000000010");
const VIEWER: Uuid = uuid!("b1000000-0000-0000-0000-000000000011");
const OUTSIDER: Uuid = uuid!("b1000000-0000-0000-0000-000000000012");
const P1: Uuid = uuid!("b1000000-0000-0000-0000-000000000021");
const P2: Uuid = uuid!("b1000000-0000-0000-0000-000000000022");
const P3: Uuid = uuid!("b1000000-0000-0000-0000-000000000023");
const P4: Uuid = uuid!("b1000000-0000-0000-0000-000000000024");
const CONVERTED_FLIGHT: Uuid = uuid!("b1000000-0000-0000-0000-000000000031");
const OTHER_FLIGHT: Uuid = uuid!("b1000000-0000-0000-0000-000000000032");
const ADMIN_TOKEN: &str = "pairings-admin-token";
const VIEWER_TOKEN: &str = "pairings-viewer-token";
const OUTSIDER_TOKEN: &str = "pairings-outsider-token";

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO users (id, username, display_name, role) VALUES
          ('b1000000-0000-0000-0000-000000000010', 'pairing_admin', 'Admin', 'viewer'),
          ('b1000000-0000-0000-0000-000000000011', 'pairing_viewer', 'Viewer', 'admin'),
          ('b1000000-0000-0000-0000-000000000012', 'pairing_outsider', 'Outsider', 'admin');
        INSERT INTO players (id, display_name, current_handicap_index) VALUES
          ('b1000000-0000-0000-0000-000000000021', 'Ada', 10.0),
          ('b1000000-0000-0000-0000-000000000022', 'Bob', 11.0),
          ('b1000000-0000-0000-0000-000000000023', 'Cid', 12.0),
          ('b1000000-0000-0000-0000-000000000024', 'Dee', 13.0);
        UPDATE players SET active=FALSE WHERE id='b1000000-0000-0000-0000-000000000024';
        INSERT INTO tournaments
          (id, name, start_date, end_date, number_of_rounds)
          VALUES ('b1000000-0000-0000-0000-000000000001', 'Pairings',
                  '2026-08-01', '2026-08-02', 1);
        INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap, status) VALUES
          ('b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000021', 10.0, 'active'),
          ('b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000022', 11.0, 'active'),
          ('b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000023', 12.0, 'active'),
          ('b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000024', 13.0, 'withdrawn');
        INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_name, tee_name, scoring_format)
          VALUES ('b1000000-0000-0000-0000-000000000002', 'b1000000-0000-0000-0000-000000000001', 1, 'Round', '2026-08-01', 'TBD', 'TBD', 'individual_stroke_play');
        INSERT INTO teams (id, round_id, tournament_id, name, starting_hole, tee_time, created_at, updated_at)
          VALUES ('b1000000-0000-0000-0000-000000000003', 'b1000000-0000-0000-0000-000000000002', 'b1000000-0000-0000-0000-000000000001', 'Legacy A', 4, '08:30', '2020-01-01 00:00:00.123456+00', '2020-01-02 00:00:00.654321+00');
        INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id, display_order, created_at) VALUES
          ('b1000000-0000-0000-0000-000000000003', 'b1000000-0000-0000-0000-000000000002', 'b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000022', 7, '2020-01-03 00:00:00.111222+00'),
          ('b1000000-0000-0000-0000-000000000003', 'b1000000-0000-0000-0000-000000000002', 'b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000021', NULL, '2020-01-04 00:00:00.333444+00');
        INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
          ('b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000010', 'admin'),
          ('b1000000-0000-0000-0000-000000000001', 'b1000000-0000-0000-0000-000000000011', 'viewer');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    let mut admin_session_id = None;
    for (user, token) in [
        (ADMIN, ADMIN_TOKEN),
        (VIEWER, VIEWER_TOKEN),
        (OUTSIDER, OUTSIDER_TOKEN),
    ] {
        let session = auth::create_session(
            pool,
            user,
            &hash_session_token(token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
        if user == ADMIN {
            admin_session_id = Some(session.session_id);
        }
    }
    let tournament_id = uuid!("b1000000-0000-0000-0000-000000000001");
    let tournament_updated_at =
        sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id = $1")
            .bind(tournament_id)
            .fetch_one(pool)
            .await
            .unwrap();
    tournaments::start_authorized(
        pool,
        admin_session_id.unwrap(),
        tournament_id,
        tournament_updated_at,
    )
    .await
    .unwrap();
}

async fn updated_at(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id = $1")
        .bind(ROUND)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn get(token: Option<&str>) -> Request<Body> {
    let mut builder = Request::get(format!("/api/rounds/{ROUND}/pairings"));
    if let Some(token) = token {
        builder = builder.header(header::COOKIE, format!("golf_session={token}"));
    }
    builder.body(Body::empty()).unwrap()
}

fn put(token: &str, value: Value) -> Request<Body> {
    Request::put(format!("/api/rounds/{ROUND}/pairings"))
        .header(header::COOKIE, format!("golf_session={token}"))
        .header("x-csrf-token", derive_csrf_token(token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(value.to_string()))
        .unwrap()
}

fn raw_put(
    round_id: Uuid,
    token: Option<&str>,
    csrf: bool,
    content_type: Option<&str>,
    body: Body,
) -> Request<Body> {
    let mut builder = Request::put(format!("/api/rounds/{round_id}/pairings"));
    if let Some(token) = token {
        builder = builder.header(header::COOKIE, format!("golf_session={token}"));
        if csrf {
            builder = builder.header("x-csrf-token", derive_csrf_token(token));
        }
    }
    if let Some(content_type) = content_type {
        builder = builder.header(header::CONTENT_TYPE, content_type);
    }
    builder.body(body).unwrap()
}

fn open_request() -> Request<Body> {
    Request::post(format!("/api/rounds/{ROUND}/open"))
        .header(header::COOKIE, format!("golf_session={ADMIN_TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(ADMIN_TOKEN))
        .body(Body::empty())
        .unwrap()
}

fn valid(timestamp: DateTime<Utc>) -> Value {
    json!({
        "expected_round_updated_at": timestamp,
        "teams": [],
        "flights": [
          {"id": CONVERTED_FLIGHT, "name":"Legacy A", "starting_hole":4, "tee_time":"08:30:00", "members":[{"player_id":P2},{"player_id":P1}]},
          {"id": OTHER_FLIGHT, "name":"Solo", "starting_hole":null, "tee_time":null, "members":[{"player_id":P3}]}
        ],
        "legacy_conversions":[{"team_id":LEGACY_TEAM,"flight_id":CONVERTED_FLIGHT}]
    })
}

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

fn assert_private(response: &axum::response::Response) {
    assert_eq!(
        response.headers().get(header::CACHE_CONTROL).unwrap(),
        "private, no-store"
    );
}

async fn wait_until_blocked(pool: &PgPool, locker_pid: i32) {
    for _ in 0..100 {
        let blocked: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1 = ANY(pg_blocking_pids(pid)))",
        )
        .bind(locker_pid)
        .fetch_one(pool)
        .await
        .unwrap();
        if blocked {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("pairing replacement did not reach the round lock");
}

async fn wait_for_blocked_request_count(pool: &PgPool, minimum: i64) {
    for _ in 0..100 {
        let blocked: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM pg_stat_activity
             WHERE datname=current_database() AND cardinality(pg_blocking_pids(pid)) > 0",
        )
        .fetch_one(pool)
        .await
        .unwrap();
        if blocked >= minimum {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("expected blocked pairing requests did not reach their locks");
}

fn assert_no_event(receiver: &mut tokio::sync::broadcast::Receiver<golf_api::LiveEvent>) {
    assert!(matches!(
        receiver.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn auth_decode_missing_and_cache_contract_is_uniform(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let missing = uuid!("b1000000-0000-0000-0000-000000000099");

    for request in [
        get(None),
        raw_put(ROUND, None, false, Some("text/plain"), Body::from("{")),
    ] {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_private(&response);
    }
    let outsider = raw_put(ROUND, Some(OUTSIDER_TOKEN), true, None, Body::from("{"));
    let response = app.clone().oneshot(outsider).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_private(&response);

    let missing_get = Request::get(format!("/api/rounds/{missing}/pairings"))
        .header(header::COOKIE, format!("golf_session={ADMIN_TOKEN}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(missing_get).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_private(&response);
    let missing_put = raw_put(missing, Some(ADMIN_TOKEN), true, None, Body::from("{"));
    let response = app.clone().oneshot(missing_put).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_private(&response);

    for request in [
        raw_put(ROUND, Some(ADMIN_TOKEN), true, Some("text/plain"), Body::from("{}")),
        raw_put(ROUND, Some(ADMIN_TOKEN), true, Some("application/json"), Body::from("{")),
        raw_put(
            ROUND,
            Some(ADMIN_TOKEN),
            true,
            Some("application/json"),
            Body::from(json!({"expected_round_updated_at":updated_at(&pool).await,"teams":[],"flights":[],"legacy_conversions":[],"unknown":true}).to_string()),
        ),
    ] {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_private(&response);
    }
    assert_no_event(&mut events);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flights WHERE round_id=$1")
            .bind(ROUND)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn member_read_and_exact_legacy_conversion_are_private_atomic_and_auditable(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let app = api::router(state.clone());
    for token in [ADMIN_TOKEN, VIEWER_TOKEN] {
        let response = app.clone().oneshot(get(Some(token))).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        let payload = body(response).await;
        assert_eq!(payload["teams"], json!([]));
        assert_eq!(payload["legacy_individual_groups"][0]["name"], "Legacy A");
        assert_eq!(payload["active_entrants"][0]["display_name"], "Ada");
    }
    assert_eq!(
        app.clone().oneshot(get(None)).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.clone()
            .oneshot(get(Some(OUTSIDER_TOKEN)))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );

    let previous = updated_at(&pool).await;
    let mut events = state.live_events.subscribe();
    let response = app
        .oneshot(put(ADMIN_TOKEN, valid(previous)))
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::OK,
        "{}",
        body(response).await
    );
    let event = events.try_recv().unwrap();
    assert_eq!((event.resource, event.id), ("round", ROUND));
    let facts: (String, i16, DateTime<Utc>, DateTime<Utc>) = sqlx::query_as(
        "SELECT name, starting_hole, created_at, updated_at FROM flights WHERE id = $1",
    )
    .bind(CONVERTED_FLIGHT)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(facts.0, "Legacy A");
    assert_eq!(facts.1, 4);
    assert_eq!(
        facts.2,
        "2020-01-01T00:00:00.123456Z"
            .parse::<DateTime<Utc>>()
            .unwrap()
    );
    assert_eq!(
        facts.3,
        "2020-01-02T00:00:00.654321Z"
            .parse::<DateTime<Utc>>()
            .unwrap()
    );
    let member_facts: Vec<(Uuid, Option<i16>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT player_id, display_order, created_at FROM flight_memberships WHERE flight_id=$1 ORDER BY display_order NULLS LAST, player_id",
    )
    .bind(CONVERTED_FLIGHT)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        member_facts,
        vec![
            (P2, Some(7), "2020-01-03T00:00:00.111222Z".parse().unwrap()),
            (P1, None, "2020-01-04T00:00:00.333444Z".parse().unwrap()),
        ]
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM teams WHERE round_id=$1")
            .bind(ROUND)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(updated_at(&pool).await > previous);
}

#[sqlx::test(migrations = "../migrations")]
async fn stale_invalid_and_non_admin_replacements_roll_back_without_events(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let timestamp = updated_at(&pool).await;
    let missing_csrf = Request::put(format!("/api/rounds/{ROUND}/pairings"))
        .header(header::COOKIE, format!("golf_session={ADMIN_TOKEN}"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(valid(timestamp).to_string()))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(missing_csrf).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let oversized = Request::put(format!("/api/rounds/{ROUND}/pairings"))
        .header(header::COOKIE, format!("golf_session={ADMIN_TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(ADMIN_TOKEN))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(vec![b' '; 256 * 1024 + 1]))
        .unwrap();
    let oversized = app.clone().oneshot(oversized).await.unwrap();
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_private(&oversized);
    assert_eq!(body(oversized).await["error"]["code"], "payload_too_large");
    assert_no_event(&mut events);
    let denied = app
        .clone()
        .oneshot(put(VIEWER_TOKEN, valid(timestamp)))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_private(&denied);
    assert_no_event(&mut events);
    let mut invalid = valid(timestamp);
    invalid["flights"][0]["members"] = json!([{"player_id":P1}]);
    let rejected = app
        .clone()
        .oneshot(put(ADMIN_TOKEN, invalid))
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::CONFLICT);
    assert_private(&rejected);
    assert_eq!(
        body(rejected).await["error"]["code"],
        "invalid_legacy_conversion"
    );
    assert_no_event(&mut events);
    let stale = app
        .oneshot(put(
            ADMIN_TOKEN,
            valid("2000-01-01T00:00:00Z".parse().unwrap()),
        ))
        .await
        .unwrap();
    assert_eq!(stale.status(), StatusCode::CONFLICT);
    assert_private(&stale);
    assert_eq!(body(stale).await["error"]["code"], "round_pairings_stale");
    assert_no_event(&mut events);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM teams WHERE id=$1")
            .bind(LEGACY_TEAM)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flights WHERE round_id=$1")
            .bind(ROUND)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn granular_team_writes_are_retired(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool));
    let create = Request::post(format!("/api/rounds/{ROUND}/teams"))
        .body(Body::empty())
        .unwrap();
    let assign = Request::post(format!("/api/teams/{LEGACY_TEAM}/members"))
        .body(Body::empty())
        .unwrap();
    let remove = Request::delete(format!("/api/teams/{LEGACY_TEAM}/members/{P1}"))
        .body(Body::empty())
        .unwrap();
    assert_eq!(
        app.clone().oneshot(create).await.unwrap().status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(
        app.clone().oneshot(assign).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.oneshot(remove).await.unwrap().status(),
        StatusCode::NOT_FOUND
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn empty_and_partial_draft_rosters_are_valid_for_both_formats(pool: PgPool) {
    seed(&pool).await;
    sqlx::query("DELETE FROM team_memberships WHERE round_id=$1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM teams WHERE round_id=$1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    let empty = json!({"expected_round_updated_at":updated_at(&pool).await,"teams":[],"flights":[],"legacy_conversions":[]});
    let response = app.clone().oneshot(put(ADMIN_TOKEN, empty)).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_private(&response);
    assert_eq!(events.try_recv().unwrap().resource, "round");

    let partial = json!({
        "expected_round_updated_at":updated_at(&pool).await,
        "teams":[],
        "flights":[{"id":OTHER_FLIGHT,"name":"Partial","starting_hole":null,"tee_time":null,"members":[{"player_id":P1}]}],
        "legacy_conversions":[]
    });
    let response = app
        .clone()
        .oneshot(put(ADMIN_TOKEN, partial))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(events.try_recv().unwrap().resource, "round");

    sqlx::query("UPDATE rounds SET scoring_format='team_scramble' WHERE id=$1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    let empty_scramble = json!({"expected_round_updated_at":updated_at(&pool).await,"teams":[],"flights":[],"legacy_conversions":[]});
    assert_eq!(
        app.clone()
            .oneshot(put(ADMIN_TOKEN, empty_scramble))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(events.try_recv().unwrap().resource, "round");
    let partial_scramble = json!({
        "expected_round_updated_at":updated_at(&pool).await,
        "teams":[{"id":uuid!("b1000000-0000-0000-0000-000000000050"),"name":"Partial team","members":[{"player_id":P2}],"schedule_flight_id":null}],
        "flights":[{"id":uuid!("b1000000-0000-0000-0000-000000000051"),"name":"Partial flight","starting_hole":null,"tee_time":null,"members":[{"player_id":P1}]}],
        "legacy_conversions":[]
    });
    assert_eq!(
        app.oneshot(put(ADMIN_TOKEN, partial_scramble))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(events.try_recv().unwrap().resource, "round");
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn entrant_activity_is_effective_and_ineligible_submissions_write_nothing(pool: PgPool) {
    seed(&pool).await;
    sqlx::query(
        "UPDATE tournament_players SET status='withdrawn' WHERE tournament_id=$1 AND player_id=$2",
    )
    .bind(uuid!("b1000000-0000-0000-0000-000000000001"))
    .bind(P2)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE players SET active=FALSE WHERE id=$1")
        .bind(P3)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM team_memberships WHERE round_id=$1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM teams WHERE round_id=$1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let response = app.clone().oneshot(get(Some(ADMIN_TOKEN))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let payload = body(response).await;
    assert_eq!(payload["active_entrants"].as_array().unwrap().len(), 1);
    assert_eq!(payload["active_entrants"][0]["player_id"], P1.to_string());
    let inactive = payload["inactive_entrants"].as_array().unwrap();
    assert_eq!(inactive.len(), 3);
    assert_eq!(
        (
            inactive[0]["status"].as_str(),
            inactive[0]["player_active"].as_bool()
        ),
        (Some("withdrawn"), Some(true))
    );
    assert_eq!(
        (
            inactive[1]["status"].as_str(),
            inactive[1]["player_active"].as_bool()
        ),
        (Some("active"), Some(false))
    );
    assert_eq!(
        (
            inactive[2]["status"].as_str(),
            inactive[2]["player_active"].as_bool()
        ),
        (Some("withdrawn"), Some(false))
    );

    for player_id in [P2, P3, P4] {
        let request = json!({
            "expected_round_updated_at":updated_at(&pool).await,
            "teams":[],
            "flights":[{"id":Uuid::new_v4(),"name":"Ineligible","starting_hole":null,"tee_time":null,"members":[{"player_id":player_id}]}],
            "legacy_conversions":[]
        });
        let response = app
            .clone()
            .oneshot(put(ADMIN_TOKEN, request))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_private(&response);
        assert_eq!(
            body(response).await["error"]["code"],
            "invalid_pairing_roster"
        );
    }
    assert_no_event(&mut events);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flights WHERE round_id=$1")
            .bind(ROUND)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn identity_legacy_and_referenced_deletion_conflicts_are_stable_and_atomic(pool: PgPool) {
    seed(&pool).await;
    let other_round = uuid!("b1000000-0000-0000-0000-000000000060");
    let other_team = uuid!("b1000000-0000-0000-0000-000000000061");
    let other_flight = uuid!("b1000000-0000-0000-0000-000000000062");
    sqlx::query("INSERT INTO rounds (id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES ($1,$2,2,'Other','2026-08-02','TBD','TBD','individual_stroke_play')")
        .bind(other_round).bind(uuid!("b1000000-0000-0000-0000-000000000001")).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO teams (id,round_id,tournament_id,name) VALUES ($1,$2,$3,'Other team')",
    )
    .bind(other_team)
    .bind(other_round)
    .bind(uuid!("b1000000-0000-0000-0000-000000000001"))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO flights (id,round_id,tournament_id,name) VALUES ($1,$2,$3,'Other flight')",
    )
    .bind(other_flight)
    .bind(other_round)
    .bind(uuid!("b1000000-0000-0000-0000-000000000001"))
    .execute(&pool)
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let timestamp = updated_at(&pool).await;

    let cases = [
        (
            json!({"expected_round_updated_at":timestamp,"teams":[],"flights":[{"id":LEGACY_TEAM,"name":"Legacy A","starting_hole":4,"tee_time":"08:30:00","members":[{"player_id":P2},{"player_id":P1}]}],"legacy_conversions":[{"team_id":LEGACY_TEAM,"flight_id":LEGACY_TEAM}]}),
            StatusCode::CONFLICT,
            "pairing_identity_conflict",
        ),
        (
            json!({"expected_round_updated_at":timestamp,"teams":[{"id":other_team,"name":"X","members":[],"schedule_flight_id":null}],"flights":[],"legacy_conversions":[]}),
            StatusCode::CONFLICT,
            "pairing_identity_conflict",
        ),
        (
            json!({"expected_round_updated_at":timestamp,"teams":[],"flights":[{"id":other_flight,"name":"X","starting_hole":null,"tee_time":null,"members":[]}],"legacy_conversions":[]}),
            StatusCode::CONFLICT,
            "pairing_identity_conflict",
        ),
        (
            json!({"expected_round_updated_at":timestamp,"teams":[],"flights":[],"legacy_conversions":[]}),
            StatusCode::CONFLICT,
            "legacy_mapping_required",
        ),
        (
            json!({"expected_round_updated_at":timestamp,"teams":[],"flights":[{"id":CONVERTED_FLIGHT,"name":"Legacy A","starting_hole":4,"tee_time":"08:30:00","members":[{"player_id":P2},{"player_id":P1}]}],"legacy_conversions":[{"team_id":P4,"flight_id":CONVERTED_FLIGHT}]}),
            StatusCode::CONFLICT,
            "legacy_mapping_required",
        ),
        (
            json!({"expected_round_updated_at":timestamp,"teams":[],"flights":[],"legacy_conversions":[{"team_id":LEGACY_TEAM,"flight_id":CONVERTED_FLIGHT}]}),
            StatusCode::CONFLICT,
            "invalid_legacy_conversion",
        ),
        (
            json!({"expected_round_updated_at":timestamp,"teams":[],"flights":[{"id":CONVERTED_FLIGHT,"name":"Legacy A","starting_hole":4,"tee_time":"08:30:00","members":[{"player_id":P2},{"player_id":P1}]}],"legacy_conversions":[{"team_id":LEGACY_TEAM,"flight_id":CONVERTED_FLIGHT},{"team_id":LEGACY_TEAM,"flight_id":OTHER_FLIGHT}]}),
            StatusCode::BAD_REQUEST,
            "validation_error",
        ),
    ];
    for (request, status, code) in cases {
        let response = app
            .clone()
            .oneshot(put(ADMIN_TOKEN, request))
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_private(&response);
        assert_eq!(body(response).await["error"]["code"], code);
        assert_no_event(&mut events);
    }

    let mut connection = pool.acquire().await.unwrap();
    sqlx::query("SET session_replication_role = replica")
        .execute(&mut *connection)
        .await
        .unwrap();
    sqlx::query("INSERT INTO scorecard_confirmations (id,round_id,tournament_id,team_id,confirmed_by) VALUES ($1,$2,$3,$4,$5)")
        .bind(uuid!("b1000000-0000-0000-0000-000000000063")).bind(ROUND)
        .bind(uuid!("b1000000-0000-0000-0000-000000000001")).bind(LEGACY_TEAM).bind(ADMIN)
        .execute(&mut *connection).await.unwrap();
    sqlx::query("SET session_replication_role = origin")
        .execute(&mut *connection)
        .await
        .unwrap();
    drop(connection);
    let response = app
        .oneshot(put(ADMIN_TOKEN, valid(timestamp)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_private(&response);
    assert_eq!(body(response).await["error"]["code"], "team_is_referenced");
    assert_no_event(&mut events);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM teams WHERE id=$1")
            .bind(LEGACY_TEAM)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flights WHERE round_id=$1")
            .bind(ROUND)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_opening_wins_round_lock_and_suppresses_pairing_event(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;
    let mut opening = pool.begin().await.unwrap();
    let locker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *opening)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(ROUND)
        .execute(&mut *opening)
        .await
        .unwrap();
    let request = tokio::spawn(async move {
        app.oneshot(put(ADMIN_TOKEN, valid(expected)))
            .await
            .unwrap()
    });
    wait_until_blocked(&pool, locker_pid).await;
    sqlx::query("SELECT set_config('app.round_opening_id', $1, true)")
        .bind(ROUND.to_string())
        .execute(&mut *opening)
        .await
        .unwrap();
    for player_id in [P1, P2, P3] {
        sqlx::query("INSERT INTO round_handicap_snapshots (round_id,tournament_id,player_id,handicap_index,course_handicap,playing_handicap) VALUES ($1,$2,$3,10.0,10,10)")
            .bind(ROUND).bind(uuid!("b1000000-0000-0000-0000-000000000001")).bind(player_id)
            .execute(&mut *opening).await.unwrap();
    }
    sqlx::query("UPDATE rounds SET status='open' WHERE id=$1")
        .bind(ROUND)
        .execute(&mut *opening)
        .await
        .unwrap();
    opening.commit().await.unwrap();
    let response = request.await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(body(response).await["error"]["code"], "round_not_draft");
    assert_no_event(&mut events);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flights WHERE round_id=$1")
            .bind(ROUND)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_membership_revocation_is_reauthorized_after_round_lock(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;
    let mut revocation = pool.begin().await.unwrap();
    let locker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *revocation)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(ROUND)
        .execute(&mut *revocation)
        .await
        .unwrap();
    let request = tokio::spawn(async move {
        app.oneshot(put(ADMIN_TOKEN, valid(expected)))
            .await
            .unwrap()
    });
    wait_until_blocked(&pool, locker_pid).await;
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
        .bind(uuid!("b1000000-0000-0000-0000-000000000001"))
        .bind(ADMIN)
        .execute(&mut *revocation)
        .await
        .unwrap();
    revocation.commit().await.unwrap();
    let response = request.await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_no_event(&mut events);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM teams WHERE id=$1")
            .bind(LEGACY_TEAM)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_round_change_becomes_stable_stale_conflict(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;
    let mut change = pool.begin().await.unwrap();
    let locker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *change)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(ROUND)
        .execute(&mut *change)
        .await
        .unwrap();
    let request = tokio::spawn(async move {
        app.oneshot(put(ADMIN_TOKEN, valid(expected)))
            .await
            .unwrap()
    });
    wait_until_blocked(&pool, locker_pid).await;
    sqlx::query("UPDATE rounds SET name=name WHERE id=$1")
        .bind(ROUND)
        .execute(&mut *change)
        .await
        .unwrap();
    change.commit().await.unwrap();
    let response = request.await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(response).await["error"]["code"],
        "round_pairings_stale"
    );
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn simultaneous_same_token_replacements_commit_once_and_emit_once(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;
    let first = tokio::spawn({
        let app = app.clone();
        async move {
            app.oneshot(put(ADMIN_TOKEN, valid(expected)))
                .await
                .unwrap()
        }
    });
    let second = tokio::spawn(async move {
        app.oneshot(put(ADMIN_TOKEN, valid(expected)))
            .await
            .unwrap()
    });
    let mut responses = vec![first.await.unwrap(), second.await.unwrap()];
    let statuses: Vec<_> = responses.iter().map(|response| response.status()).collect();
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::OK)
            .count(),
        1
    );
    assert_eq!(
        statuses
            .iter()
            .filter(|status| **status == StatusCode::CONFLICT)
            .count(),
        1
    );
    for response in &responses {
        assert_private(response);
    }
    let loser_index = responses
        .iter()
        .position(|response| response.status() == StatusCode::CONFLICT)
        .unwrap();
    let loser = responses.swap_remove(loser_index);
    assert_eq!(body(loser).await["error"]["code"], "round_pairings_stale");
    assert_eq!(events.try_recv().unwrap().resource, "round");
    assert_no_event(&mut events);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flights WHERE round_id=$1")
            .bind(ROUND)
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn pairing_first_holds_round_lock_until_commit_before_opening_rechecks(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;
    let mut session_blocker = pool.begin().await.unwrap();
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *session_blocker)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM user_sessions WHERE user_id=$1 AND revoked_at IS NULL FOR UPDATE")
        .bind(ADMIN)
        .execute(&mut *session_blocker)
        .await
        .unwrap();
    let pairing = tokio::spawn({
        let app = app.clone();
        async move {
            app.oneshot(put(ADMIN_TOKEN, valid(expected)))
                .await
                .unwrap()
        }
    });
    wait_until_blocked(&pool, blocker_pid).await;
    let opening = tokio::spawn(async move { app.oneshot(open_request()).await.unwrap() });
    wait_for_blocked_request_count(&pool, 2).await;
    assert!(
        !opening.is_finished(),
        "opening completed before pairings released the round lock"
    );
    session_blocker.commit().await.unwrap();
    let pairing = pairing.await.unwrap();
    assert_eq!(pairing.status(), StatusCode::OK);
    assert_private(&pairing);
    let opening = opening.await.unwrap();
    assert_eq!(opening.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(opening).await,
        json!({"error":{"code":"conflict","message":"round is not ready to open"}})
    );
    assert_eq!(events.try_recv().unwrap().resource, "round");
    assert_no_event(&mut events);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flights WHERE round_id=$1")
            .bind(ROUND)
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn scramble_identity_is_retained_and_schedule_transfer_must_be_explicit(pool: PgPool) {
    seed(&pool).await;
    sqlx::query("UPDATE rounds SET scoring_format='team_scramble' WHERE id=$1")
        .bind(ROUND)
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;
    let team_two = uuid!("b1000000-0000-0000-0000-000000000040");
    let scheduled_flight = uuid!("b1000000-0000-0000-0000-000000000041");
    let solo_flight = uuid!("b1000000-0000-0000-0000-000000000042");
    let mut request = json!({
        "expected_round_updated_at": expected,
        "teams":[
          {"id":LEGACY_TEAM,"name":"Shared result","members":[{"player_id":P2},{"player_id":P1}],"schedule_flight_id":null},
          {"id":team_two,"name":"Second result","members":[{"player_id":P3}],"schedule_flight_id":null}
        ],
        "flights":[
          {"id":scheduled_flight,"name":"Flight A","starting_hole":4,"tee_time":"08:30:00","members":[{"player_id":P2},{"player_id":P1}]},
          {"id":solo_flight,"name":"Flight B","starting_hole":null,"tee_time":null,"members":[{"player_id":P3}]}
        ],
        "legacy_conversions":[]
    });
    let inferred = app
        .clone()
        .oneshot(put(ADMIN_TOKEN, request.clone()))
        .await
        .unwrap();
    assert_eq!(inferred.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(inferred).await["error"]["code"],
        "invalid_schedule_transfer"
    );
    assert_no_event(&mut events);
    request["teams"][0]["schedule_flight_id"] = json!(scheduled_flight);
    let transferred = app.oneshot(put(ADMIN_TOKEN, request)).await.unwrap();
    assert_eq!(transferred.status(), StatusCode::OK);
    let payload = body(transferred).await;
    assert!(
        payload["teams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|team| team["id"] == LEGACY_TEAM.to_string())
    );
    assert_eq!(payload["legacy_individual_groups"], json!([]));
    let schedule: (Option<i16>, Option<chrono::NaiveTime>) =
        sqlx::query_as("SELECT starting_hole, tee_time FROM teams WHERE id=$1")
            .bind(LEGACY_TEAM)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(schedule, (None, None));
    let event = events.try_recv().unwrap();
    assert_eq!((event.resource, event.id), ("round", ROUND));
    assert_no_event(&mut events);
}
