#![cfg(feature = "database-tests")]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query},
    http::{Request, StatusCode, header},
    response::IntoResponse,
    routing::get,
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api, auth::hash_session_token, course_provider::CourseProviderClient,
    repositories::auth,
};
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT_ID: Uuid = uuid!("8c000000-0000-0000-0000-000000000001");
const ADMIN_ID: Uuid = uuid!("8c000000-0000-0000-0000-000000000002");
const OUTSIDER_ID: Uuid = uuid!("8c000000-0000-0000-0000-000000000003");

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO users (id, username, display_name, role) VALUES
        ('8c000000-0000-0000-0000-000000000002', 'course_admin', 'Course admin', 'viewer'),
        ('8c000000-0000-0000-0000-000000000003', 'course_outsider', 'Course outsider', 'admin');
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
        VALUES ('8c000000-0000-0000-0000-000000000001', 'Provider trip', '2026-08-01', '2026-08-02', 2);
        INSERT INTO tournament_memberships (tournament_id, user_id, role)
        VALUES ('8c000000-0000-0000-0000-000000000001', '8c000000-0000-0000-0000-000000000002', 'admin');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    for (user_id, token) in [(ADMIN_ID, "course-admin"), (OUTSIDER_ID, "course-outsider")] {
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

fn request(path: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::get(path);
    if let Some(token) = token {
        builder = builder.header(header::COOKIE, format!("golf_session={token}"));
    }
    builder.body(Body::empty()).unwrap()
}

async fn body(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

#[derive(Deserialize)]
struct ProviderSearchQuery {
    search_query: String,
}

async fn mock_provider(calls: Arc<AtomicUsize>) -> String {
    let app = Router::new()
        .route(
            "/v1/search",
            get({
                let calls = calls.clone();
                move |Query(query): Query<ProviderSearchQuery>| {
                    let calls = calls.clone();
                    async move {
                        calls.fetch_add(1, Ordering::SeqCst);
                        if query.search_query == "limited" {
                            return (
                                StatusCode::TOO_MANY_REQUESTS,
                                Json(json!({"error": "secret upstream limit detail"})),
                            )
                                .into_response();
                        }
                        Json(json!({"courses": [{
                            "id": "7k2m9qb4",
                            "club_name": "Murray Golf Club",
                            "course_name": "Course No. 1",
                            "location": {"country": "United States"},
                            "tees": {"female": 1, "male": 1}
                        }]}))
                        .into_response()
                    }
                }
            }),
        )
        .route(
            "/v1/courses/{id}",
            get(move |Path(id): Path<String>| {
                let calls = calls.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    if id != "7k2m9qb4" {
                        return (
                            StatusCode::NOT_FOUND,
                            Json(json!({"error": "secret missing detail"})),
                        )
                            .into_response();
                    }
                    Json(json!({
                        "id": "7k2m9qb4",
                        "club_name": "Murray Golf Club",
                        "course_name": "Course No. 1",
                        "location": {"country": "United States"},
                        "tees": {"female": [], "male": [{
                            "tee_name": "Blue", "course_rating": 72.0,
                            "slope_rating": 120, "total_yards": 400,
                            "total_meters": 366, "number_of_holes": 1,
                            "par_total": 4,
                            "holes": [{"par": 4, "yardage": 400, "handicap": 1}]
                        }]}
                    }))
                    .into_response()
                }
            }),
        );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{address}/")
}

#[sqlx::test(migrations = "../migrations")]
async fn admin_authorization_finishes_before_bounded_provider_reads(pool: PgPool) {
    seed(&pool).await;
    let calls = Arc::new(AtomicUsize::new(0));
    let base_url = mock_provider(calls.clone()).await;
    let provider =
        CourseProviderClient::configured_with_base_url("server-secret".into(), &base_url).unwrap();
    let app = api::router(AppState::with_auth_and_course_provider(
        pool.clone(),
        golf_api::auth::AuthConfig::local(),
        provider,
    ));
    let search_path = format!(
        "/api/tournaments/{TOURNAMENT_ID}/course-provider/search?q=Murray&fuzzy_match=false"
    );

    let unauthenticated = app
        .clone()
        .oneshot(request(&search_path, None))
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let forbidden = app
        .clone()
        .oneshot(request(&search_path, Some("course-outsider")))
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    let invalid = app
        .clone()
        .oneshot(request(
            &format!("/api/tournaments/{TOURNAMENT_ID}/course-provider/search?q=x"),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body(invalid).await["error"]["code"], "validation_error");
    let missing_q = app
        .clone()
        .oneshot(request(
            &format!("/api/tournaments/{TOURNAMENT_ID}/course-provider/search"),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(missing_q.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body(missing_q).await["error"]["code"], "validation_error");
    let malformed_fuzzy = app
        .clone()
        .oneshot(request(
            &format!(
                "/api/tournaments/{TOURNAMENT_ID}/course-provider/search?q=Murray&fuzzy_match=maybe"
            ),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(malformed_fuzzy.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        body(malformed_fuzzy).await["error"]["code"],
        "validation_error"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let search = app
        .clone()
        .oneshot(request(&search_path, Some("course-admin")))
        .await
        .unwrap();
    assert_eq!(search.status(), StatusCode::OK);
    assert_eq!(search.headers()[header::CACHE_CONTROL], "private, no-store");
    let search_body = body(search).await;
    assert_eq!(search_body["courses"][0]["provider"], "golf_course_api");
    assert_eq!(search_body["courses"][0]["provider_course_id"], "7k2m9qb4");

    let detail = app
        .clone()
        .oneshot(request(
            &format!("/api/tournaments/{TOURNAMENT_ID}/course-provider/courses/7K2M9QB4"),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = body(detail).await;
    assert_eq!(detail_body["tees"][0]["holes"][0]["number"], 1);
    assert_eq!(detail_body["tees"][0]["holes"][0]["stroke_index"], 1);
    assert!(detail_body["tees"][0].get("id").is_none());
    assert_eq!(calls.load(Ordering::SeqCst), 2);

    let missing_detail = app
        .clone()
        .oneshot(request(
            &format!("/api/tournaments/{TOURNAMENT_ID}/course-provider/courses/abcde123"),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(missing_detail.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        missing_detail.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let missing_body = body(missing_detail).await;
    assert_eq!(missing_body["error"]["code"], "course_provider_not_found");
    assert!(!missing_body.to_string().contains("secret"));

    let limited_path = format!(
        "/api/tournaments/{TOURNAMENT_ID}/course-provider/search?q=limited&fuzzy_match=true"
    );
    let limited = app
        .clone()
        .oneshot(request(&limited_path, Some("course-admin")))
        .await
        .unwrap();
    assert_eq!(limited.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        limited.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let limited_body = body(limited).await;
    assert_eq!(limited_body["error"]["code"], "course_provider_exhausted");
    assert!(!limited_body.to_string().contains("secret"));
    assert_eq!(calls.load(Ordering::SeqCst), 4);

    let after_limit = app
        .clone()
        .oneshot(request(
            &format!("/api/tournaments/{TOURNAMENT_ID}/course-provider/search?q=after-limit"),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(after_limit.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body(after_limit).await["error"]["code"],
        "course_provider_exhausted"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 4);

    let unavailable = api::router(AppState::new(pool))
        .oneshot(request(&search_path, Some("course-admin")))
        .await
        .unwrap();
    assert_eq!(unavailable.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        body(unavailable).await["error"]["code"],
        "course_provider_unavailable"
    );
}
