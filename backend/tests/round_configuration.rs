#![cfg(feature = "database-tests")]

use std::sync::Arc;

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    course_provider::{
        CourseDetail, CourseLocation, Hole, Tee, TeeCategory, revision_adapter::select_and_validate,
    },
    repositories::{
        auth, course_revisions as course_revision_repository, round_configuration, round_lifecycle,
        tournaments,
    },
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::broadcast::error::TryRecvError;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const ADMIN: Uuid = uuid!("84000000-0000-0000-0000-000000000001");
const VIEWER: Uuid = uuid!("84000000-0000-0000-0000-000000000002");
const DRAFT: Uuid = uuid!("84000000-0000-0000-0000-000000000004");
const OPEN: Uuid = uuid!("84000000-0000-0000-0000-000000000005");
const ADMIN_TOKEN: &str = "round-configuration-admin-token";
const VIEWER_TOKEN: &str = "round-configuration-viewer-token";

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        "INSERT INTO users (id, username, display_name, role) VALUES
         ('84000000-0000-0000-0000-000000000001', 'configuration_admin', 'Admin', 'player'),
         ('84000000-0000-0000-0000-000000000002', 'configuration_viewer', 'Viewer', 'admin');
         INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds)
         VALUES ('84000000-0000-0000-0000-000000000003', 'Configuration Cup',
                 '2026-10-01', '2026-10-02', 2);
         INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
         ('84000000-0000-0000-0000-000000000003',
          '84000000-0000-0000-0000-000000000001', 'admin'),
         ('84000000-0000-0000-0000-000000000003',
          '84000000-0000-0000-0000-000000000002', 'viewer');
         INSERT INTO players (id, display_name, current_handicap_index)
         VALUES ('84000000-0000-0000-0000-000000000020', 'Open Player', 8.0);
         INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
         VALUES ('84000000-0000-0000-0000-000000000003',
                 '84000000-0000-0000-0000-000000000020', 8.0);
         INSERT INTO courses (id, name)
         VALUES ('84000000-0000-0000-0000-000000000021', 'Open Course');
         INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
         VALUES ('84000000-0000-0000-0000-000000000022',
                 '84000000-0000-0000-0000-000000000021', 'Open Tee', 113, 4.0);
         INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
         VALUES ('84000000-0000-0000-0000-000000000023',
                 '84000000-0000-0000-0000-000000000022', 1, 4, 1);
         INSERT INTO rounds
           (id, tournament_id, round_number, name, round_date, course_id,
            course_name, tee_id, tee_name, number_of_holes, scoring_format)
         VALUES
         ('84000000-0000-0000-0000-000000000004',
          '84000000-0000-0000-0000-000000000003', 1, 'Draft', '2026-10-01',
          NULL, '', NULL, '', 18, 'individual_stroke_play'),
         ('84000000-0000-0000-0000-000000000005',
          '84000000-0000-0000-0000-000000000003', 2, 'Open', '2026-10-02',
          '84000000-0000-0000-0000-000000000021', 'Open Course',
          '84000000-0000-0000-0000-000000000022', 'Open Tee', 1,
          'individual_stroke_play');
         INSERT INTO flights (id, round_id, tournament_id, name)
         VALUES ('84000000-0000-0000-0000-000000000024',
                 '84000000-0000-0000-0000-000000000005',
                 '84000000-0000-0000-0000-000000000003', 'Open Flight');
         INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id)
         VALUES ('84000000-0000-0000-0000-000000000024',
                 '84000000-0000-0000-0000-000000000005',
                 '84000000-0000-0000-0000-000000000003',
                 '84000000-0000-0000-0000-000000000020');",
    )
    .execute(pool)
    .await
    .unwrap();
    let mut admin_session_id = None;
    for (user, token) in [(ADMIN, ADMIN_TOKEN), (VIEWER, VIEWER_TOKEN)] {
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
    let admin_session_id = admin_session_id.unwrap();
    let updated_at = sqlx::query_scalar(
        "SELECT updated_at FROM tournaments
         WHERE id = '84000000-0000-0000-0000-000000000003'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    tournaments::start_authorized(
        pool,
        admin_session_id,
        uuid!("84000000-0000-0000-0000-000000000003"),
        updated_at,
    )
    .await
    .unwrap();
    round_lifecycle::open_authorized(pool, admin_session_id, OPEN)
        .await
        .unwrap();
}

async fn make_draft_round_openable(pool: &PgPool) {
    sqlx::raw_sql(
        "INSERT INTO courses (id, name)
         VALUES ('84000000-0000-0000-0000-000000000012', 'Existing Course');
         INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
         VALUES ('84000000-0000-0000-0000-000000000013',
                 '84000000-0000-0000-0000-000000000012', 'Existing Tee', 113, 4.0);
         INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
         VALUES ('84000000-0000-0000-0000-000000000014',
                 '84000000-0000-0000-0000-000000000013', 1, 4, 1);
         UPDATE rounds SET course_id = '84000000-0000-0000-0000-000000000012',
             course_name = 'Existing Course',
             tee_id = '84000000-0000-0000-0000-000000000013',
             tee_name = 'Existing Tee', number_of_holes = 1
         WHERE id = '84000000-0000-0000-0000-000000000004';
         INSERT INTO flights (id, round_id, tournament_id, name)
         VALUES ('84000000-0000-0000-0000-000000000015',
                 '84000000-0000-0000-0000-000000000004',
                 '84000000-0000-0000-0000-000000000003', 'Race Flight');
         INSERT INTO flight_memberships (flight_id, round_id, tournament_id, player_id)
         VALUES ('84000000-0000-0000-0000-000000000015',
                 '84000000-0000-0000-0000-000000000004',
                 '84000000-0000-0000-0000-000000000003',
                 '84000000-0000-0000-0000-000000000020');",
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn timestamp(pool: &PgPool, round: Uuid) -> String {
    sqlx::query_scalar::<_, chrono::DateTime<Utc>>("SELECT updated_at FROM rounds WHERE id = $1")
        .bind(round)
        .fetch_one(pool)
        .await
        .unwrap()
        .to_rfc3339()
}

fn manual(expected: &str) -> Value {
    json!({
        "expected_round_updated_at": expected,
        "selection": {
            "source": "manual",
            "course_name": " Exact Course ",
            "location": " Oslo, Norway ",
            "tee": {
                "category": "male",
                "name": " White ",
                "course_rating": 71.2,
                "slope_rating": 125,
                "holes": [
                    {"par": 4, "stroke_index": 2, "distance": 401},
                    {"par": 3, "stroke_index": 1, "distance": null}
                ]
            }
        }
    })
}

fn request(round: Uuid, token: Option<&str>, csrf: bool, body: Value) -> Request<Body> {
    raw_request(
        round,
        token,
        csrf,
        Some("application/json"),
        Body::from(body.to_string()),
    )
}

fn raw_request(
    round: Uuid,
    token: Option<&str>,
    csrf: bool,
    content_type: Option<&str>,
    body: Body,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method("PUT")
        .uri(format!("/api/rounds/{round}/course-configuration"));
    if let Some(content_type) = content_type {
        builder = builder.header(header::CONTENT_TYPE, content_type);
    }
    if let Some(token) = token {
        builder = builder.header("cookie", format!("golf_session={token}"));
        if csrf {
            builder = builder.header("x-csrf-token", derive_csrf_token(token));
        }
    }
    builder.body(body).unwrap()
}

fn open_request() -> Request<Body> {
    Request::post(format!("/api/rounds/{DRAFT}/open"))
        .header("cookie", format!("golf_session={ADMIN_TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(ADMIN_TOKEN))
        .body(Body::empty())
        .unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

async fn revision_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM courses WHERE source IS NOT NULL")
        .fetch_one(pool)
        .await
        .unwrap()
}

fn assert_no_event(events: &mut tokio::sync::broadcast::Receiver<golf_api::LiveEvent>) {
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn manual_configuration_is_exact_atomic_private_and_emits_once(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let response = api::router(state)
        .oneshot(request(
            DRAFT,
            Some(ADMIN_TOKEN),
            true,
            manual(&timestamp(&pool, DRAFT).await),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let body = response_json(response).await;
    assert_eq!(body["course_name"], "Exact Course");
    assert_eq!(body["tee_name"], "White");
    assert_eq!(body["number_of_holes"], 2);

    let facts = sqlx::query_as::<_, (String, String, String, i16, Option<i16>)>(
        "SELECT c.name, c.location, t.name, h1.yardage, h2.yardage
         FROM rounds r
         JOIN courses c ON c.id = r.course_id
         JOIN tees t ON t.id = r.tee_id
         JOIN holes h1 ON h1.tee_id = t.id AND h1.hole_number = 1
         JOIN holes h2 ON h2.tee_id = t.id AND h2.hole_number = 2
         WHERE r.id = $1",
    )
    .bind(DRAFT)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        facts,
        (
            "Exact Course".into(),
            "Oslo, Norway".into(),
            "White".into(),
            401,
            None
        )
    );
    let metadata = sqlx::query_as::<_, (String, Option<String>, bool, String, i16, i16)>(
        "SELECT c.source::text, c.provider_course_id, c.imported_at IS NOT NULL,
                t.category::text, (t.course_rating * 10)::int2, t.slope_rating
         FROM rounds r JOIN courses c ON c.id = r.course_id
         JOIN tees t ON t.id = r.tee_id WHERE r.id = $1",
    )
    .bind(DRAFT)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        metadata,
        ("manual".into(), None, true, "male".into(), 712, 125)
    );
    let holes = sqlx::query_as::<_, (i16, i16, Option<i16>)>(
        "SELECT h.par, h.stroke_index, h.yardage FROM rounds r
         JOIN holes h ON h.tee_id = r.tee_id WHERE r.id = $1 ORDER BY h.hole_number",
    )
    .bind(DRAFT)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(holes, vec![(4, 2, Some(401)), (3, 1, None)]);
    let event = events.try_recv().unwrap();
    assert_eq!(event.resource, "round");
    assert_eq!(event.id, DRAFT);
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn auth_csrf_scope_validation_and_preflight_fail_without_effects(pool: PgPool) {
    seed(&pool).await;
    let expected = timestamp(&pool, DRAFT).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);

    let cases = [
        (
            request(DRAFT, None, false, manual(&expected)),
            StatusCode::UNAUTHORIZED,
            "unauthenticated",
        ),
        (
            request(DRAFT, Some(ADMIN_TOKEN), false, manual(&expected)),
            StatusCode::FORBIDDEN,
            "forbidden",
        ),
        (
            request(DRAFT, Some(VIEWER_TOKEN), true, manual(&expected)),
            StatusCode::FORBIDDEN,
            "forbidden",
        ),
        (
            request(DRAFT, Some(VIEWER_TOKEN), true, json!({"malformed": true})),
            StatusCode::FORBIDDEN,
            "forbidden",
        ),
        (
            request(
                Uuid::new_v4(),
                Some(ADMIN_TOKEN),
                true,
                json!({"malformed": true}),
            ),
            StatusCode::NOT_FOUND,
            "not_found",
        ),
        (
            request(
                DRAFT,
                Some(ADMIN_TOKEN),
                true,
                json!({"expected_round_updated_at": expected, "selection": {"source": "manual"}, "extra": true}),
            ),
            StatusCode::BAD_REQUEST,
            "validation_error",
        ),
        (
            request(
                DRAFT,
                Some(ADMIN_TOKEN),
                true,
                json!({"expected_round_updated_at": "2000-01-01T00:00:00Z", "selection": {"nonsense": true}}),
            ),
            StatusCode::CONFLICT,
            "round_configuration_stale",
        ),
        (
            request(
                OPEN,
                Some(ADMIN_TOKEN),
                true,
                json!({"expected_round_updated_at": timestamp(&pool, OPEN).await, "selection": {"nonsense": true}}),
            ),
            StatusCode::CONFLICT,
            "round_not_draft",
        ),
    ];
    for (request, status, code) in cases {
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(response_json(response).await["error"]["code"], code);
    }
    assert_eq!(revision_count(&pool).await, 0);
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn invalid_manual_and_incomplete_catalog_leave_no_write_or_event(pool: PgPool) {
    seed(&pool).await;
    let expected = timestamp(&pool, DRAFT).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let mut invalid = manual(&expected);
    invalid["selection"]["tee"]["holes"][1]["stroke_index"] = json!(2);
    let invalid_response = app
        .clone()
        .oneshot(request(DRAFT, Some(ADMIN_TOKEN), true, invalid))
        .await
        .unwrap();
    assert_eq!(invalid_response.status(), StatusCode::BAD_REQUEST);

    let provider = json!({
        "expected_round_updated_at": expected,
        "selection": {
            "source": "golf_course_api",
            "provider_course_id": "0zm1pe1a",
            "tee": {"category": "male", "name": "White"}
        }
    });
    let provider_response = app
        .oneshot(request(DRAFT, Some(ADMIN_TOKEN), true, provider))
        .await
        .unwrap();
    assert_eq!(provider_response.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(provider_response).await["error"]["code"],
        "course_catalog_incomplete"
    );
    assert_eq!(revision_count(&pool).await, 0);
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn content_type_body_limit_and_provider_id_errors_are_private_and_write_nothing(
    pool: PgPool,
) {
    seed(&pool).await;
    let expected = timestamp(&pool, DRAFT).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(Arc::clone(&state));
    for content_type in [None, Some("text/plain")] {
        let response = app
            .clone()
            .oneshot(raw_request(
                DRAFT,
                Some(ADMIN_TOKEN),
                true,
                content_type,
                Body::from(manual(&expected).to_string()),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        assert_eq!(
            response_json(response).await["error"]["code"],
            "validation_error"
        );
    }

    let oversized = app
        .clone()
        .oneshot(raw_request(
            DRAFT,
            Some(ADMIN_TOKEN),
            true,
            Some("application/json; charset=utf-8"),
            Body::from(" ".repeat(33 * 1024)),
        ))
        .await
        .unwrap();
    assert_eq!(oversized.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        oversized.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(
        response_json(oversized).await["error"]["code"],
        "payload_too_large"
    );

    let malformed_provider_id = app
        .oneshot(request(
            DRAFT,
            Some(ADMIN_TOKEN),
            true,
            json!({
                "expected_round_updated_at": expected,
                "selection": {
                    "source": "golf_course_api",
                    "provider_course_id": "***",
                    "tee": {"category": "male", "name": "White"}
                }
            }),
        ))
        .await
        .unwrap();
    assert_eq!(malformed_provider_id.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        malformed_provider_id.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(revision_count(&pool).await, 0);
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn validated_provider_command_uses_final_repository_and_same_token_serializes(pool: PgPool) {
    seed(&pool).await;
    let expected = sqlx::query_scalar::<_, chrono::DateTime<Utc>>(
        "SELECT updated_at FROM rounds WHERE id = $1",
    )
    .bind(DRAFT)
    .fetch_one(&pool)
    .await
    .unwrap();
    let session_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM user_sessions WHERE user_id = $1")
            .bind(ADMIN)
            .fetch_one(&pool)
            .await
            .unwrap();
    let provider = select_and_validate(
        CourseDetail {
            provider: "golf_course_api",
            provider_course_id: "provider-fact-id".into(),
            club_name: "Provider Club".into(),
            course_name: "Provider Course".into(),
            scorecard_url: None,
            location: CourseLocation {
                country: Some("Norway".into()),
                ..CourseLocation::default()
            },
            tees: vec![Tee {
                category: TeeCategory::Female,
                name: "Red".into(),
                course_rating: 70.1,
                slope_rating: 121,
                total_yards: 700,
                total_meters: 640,
                number_of_holes: 2,
                par_total: 7,
                holes: vec![
                    Hole {
                        number: 1,
                        par: 4,
                        yardage: 400,
                        stroke_index: 2,
                    },
                    Hole {
                        number: 2,
                        par: 3,
                        yardage: 300,
                        stroke_index: 1,
                    },
                ],
            }],
        },
        TeeCategory::Female,
        " Red ",
    )
    .unwrap();

    let first = round_configuration::configure(&pool, session_id, DRAFT, expected, &provider)
        .await
        .unwrap();
    assert_eq!(first.course_name, "Provider Course");
    assert_eq!(first.tee_name, "Red");
    let stored = course_revision_repository::find_by_course_id(
        &pool,
        first.course_id.expect("configured round has a course id"),
    )
    .await
    .unwrap();
    assert_eq!(
        stored.source,
        golf_api::domain::course_revisions::CourseRevisionSource::GolfCourseApi
    );
    assert_eq!(
        stored.provider_course_id.as_deref(),
        Some("provider-fact-id")
    );
    assert_eq!(stored.location.as_deref(), Some("Norway"));
    assert_eq!(
        stored.tee.category,
        golf_api::domain::course_revisions::TeeCategory::Female
    );
    assert_eq!(stored.tee.course_rating_tenths, 701);
    assert_eq!(stored.tee.slope_rating, 121);
    assert_eq!(
        stored
            .tee
            .holes
            .iter()
            .map(|hole| (hole.par, hole.stroke_index, hole.distance))
            .collect::<Vec<_>>(),
        vec![(4, 2, Some(400)), (3, 1, Some(300))]
    );
    let stale = round_configuration::configure(&pool, session_id, DRAFT, expected, &provider)
        .await
        .unwrap_err();
    assert!(matches!(
        stale,
        round_configuration::RoundConfigurationError::Stale
    ));
    assert_eq!(revision_count(&pool).await, 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn failed_round_attachment_rolls_back_revision_and_event(pool: PgPool) {
    seed(&pool).await;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_round_configuration() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'forced attachment failure'; END; $$;
         CREATE TRIGGER reject_round_configuration BEFORE UPDATE OF course_id ON rounds
         FOR EACH ROW EXECUTE FUNCTION reject_round_configuration();",
    )
    .execute(&pool)
    .await
    .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let response = api::router(state)
        .oneshot(request(
            DRAFT,
            Some(ADMIN_TOKEN),
            true,
            manual(&timestamp(&pool, DRAFT).await),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(revision_count(&pool).await, 0);
    let attached =
        sqlx::query_scalar::<_, Option<Uuid>>("SELECT course_id FROM rounds WHERE id = $1")
            .bind(DRAFT)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(attached, None);
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_same_token_requests_create_one_revision_and_one_event(pool: PgPool) {
    seed(&pool).await;
    let expected = timestamp(&pool, DRAFT).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let first = app
        .clone()
        .oneshot(request(DRAFT, Some(ADMIN_TOKEN), true, manual(&expected)));
    let second = app.oneshot(request(DRAFT, Some(ADMIN_TOKEN), true, manual(&expected)));
    let (first, second) = tokio::join!(first, second);
    let statuses = [first.unwrap().status(), second.unwrap().status()];
    assert!(statuses.contains(&StatusCode::OK));
    assert!(statuses.contains(&StatusCode::CONFLICT));
    assert_eq!(revision_count(&pool).await, 1);
    let event = events.try_recv().unwrap();
    assert_eq!(event.id, DRAFT);
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn final_repository_reauthorizes_after_membership_or_session_loss(pool: PgPool) {
    seed(&pool).await;
    let expected = sqlx::query_scalar::<_, chrono::DateTime<Utc>>(
        "SELECT updated_at FROM rounds WHERE id = $1",
    )
    .bind(DRAFT)
    .fetch_one(&pool)
    .await
    .unwrap();
    let session_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM user_sessions WHERE user_id = $1")
            .bind(ADMIN)
            .fetch_one(&pool)
            .await
            .unwrap();
    round_configuration::preflight(&pool, ADMIN, DRAFT)
        .await
        .unwrap();
    let validated = golf_api::domain::course_revisions::validate(
        golf_api::domain::course_revisions::CourseRevisionCommand {
            source: golf_api::domain::course_revisions::CourseRevisionSource::Manual,
            provider_course_id: None,
            course_name: "Reauthorization Course".into(),
            location: None,
            tee: golf_api::domain::course_revisions::TeeRevisionCommand {
                category: golf_api::domain::course_revisions::TeeCategory::Male,
                name: "White".into(),
                course_rating: 70.0,
                slope_rating: 120,
                holes: vec![golf_api::domain::course_revisions::HoleRevisionCommand {
                    par: 4,
                    stroke_index: 1,
                    distance: None,
                }],
            },
        },
    )
    .unwrap();

    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id = $1 AND user_id = $2")
        .bind(uuid!("84000000-0000-0000-0000-000000000003"))
        .bind(ADMIN)
        .execute(&pool)
        .await
        .unwrap();
    let membership_loss =
        round_configuration::configure(&pool, session_id, DRAFT, expected, &validated)
            .await
            .unwrap_err();
    assert!(matches!(
        membership_loss,
        round_configuration::RoundConfigurationError::Authorization(
            golf_api::repositories::tournament_authorization::AuthorizationError::Forbidden
        )
    ));

    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin')",
    )
    .bind(uuid!("84000000-0000-0000-0000-000000000003"))
    .bind(ADMIN)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE user_sessions SET revoked_at = now() WHERE id = $1")
        .bind(session_id)
        .execute(&pool)
        .await
        .unwrap();
    let session_loss =
        round_configuration::configure(&pool, session_id, DRAFT, expected, &validated)
            .await
            .unwrap_err();
    assert!(matches!(
        session_loss,
        round_configuration::RoundConfigurationError::Authorization(
            golf_api::repositories::tournament_authorization::AuthorizationError::Unauthenticated
        )
    ));
    assert_eq!(revision_count(&pool).await, 0);
}

#[sqlx::test(migrations = "../migrations")]
async fn opening_first_forces_waiting_configuration_to_recheck_without_orphan_or_event(
    pool: PgPool,
) {
    seed(&pool).await;
    make_draft_round_openable(&pool).await;
    let expected = timestamp(&pool, DRAFT).await;
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));
    let mut events = state.live_events.subscribe();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(DRAFT)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();

    let opening_app = app.clone();
    let mut opening = tokio::spawn(async move { opening_app.oneshot(open_request()).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut opening)
            .await
            .is_err()
    );
    let mut configuration = tokio::spawn(async move {
        app.oneshot(request(DRAFT, Some(ADMIN_TOKEN), true, manual(&expected)))
            .await
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut configuration,)
            .await
            .is_err()
    );
    blocker.commit().await.unwrap();

    let opened = opening.await.unwrap().unwrap();
    assert_eq!(opened.status(), StatusCode::OK);
    let rejected = configuration.await.unwrap().unwrap();
    assert_eq!(rejected.status(), StatusCode::CONFLICT);
    assert_eq!(
        response_json(rejected).await["error"]["code"],
        "round_not_draft"
    );
    assert_eq!(revision_count(&pool).await, 0);
    let state = sqlx::query_as::<_, (String, i64)>(
        "SELECT r.status::text, count(s.player_id)
         FROM rounds r LEFT JOIN round_handicap_snapshots s ON s.round_id = r.id
         WHERE r.id = $1 GROUP BY r.status",
    )
    .bind(DRAFT)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(state, ("open".into(), 1));
    assert_eq!(events.try_recv().unwrap().id, DRAFT);
    assert_no_event(&mut events);
}

#[sqlx::test(migrations = "../migrations")]
async fn configuration_first_commits_before_waiting_open_rechecks_readiness(pool: PgPool) {
    seed(&pool).await;
    make_draft_round_openable(&pool).await;
    let expected = timestamp(&pool, DRAFT).await;
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));
    let mut events = state.live_events.subscribe();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(DRAFT)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();

    let configuration_app = app.clone();
    let mut configuration = tokio::spawn(async move {
        configuration_app
            .oneshot(request(DRAFT, Some(ADMIN_TOKEN), true, manual(&expected)))
            .await
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut configuration,)
            .await
            .is_err()
    );
    let mut opening = tokio::spawn(async move { app.oneshot(open_request()).await });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut opening)
            .await
            .is_err()
    );
    blocker.commit().await.unwrap();

    let configured = configuration.await.unwrap().unwrap();
    assert_eq!(configured.status(), StatusCode::OK);
    let opened = opening.await.unwrap().unwrap();
    assert_eq!(opened.status(), StatusCode::OK);
    assert_eq!(revision_count(&pool).await, 1);
    let round = sqlx::query_as::<_, (String, String, i16, i64)>(
        "SELECT r.status::text, r.tee_name, r.number_of_holes, count(s.player_id)
         FROM rounds r LEFT JOIN round_handicap_snapshots s ON s.round_id = r.id
         WHERE r.id = $1 GROUP BY r.status, r.tee_name, r.number_of_holes",
    )
    .bind(DRAFT)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(round, ("open".into(), "White".into(), 2, 1));
    assert_eq!(events.try_recv().unwrap().id, DRAFT);
    assert_eq!(events.try_recv().unwrap().id, DRAFT);
    assert_no_event(&mut events);
}
