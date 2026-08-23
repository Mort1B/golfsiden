use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    extract::{Path, Query},
    http::{HeaderMap, StatusCode},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::{Notify, mpsc};

use super::{
    CourseProviderClient, CourseProviderError, normalize_course_id, validate_search_query,
};

async fn serve(router: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{address}/")
}

#[derive(Deserialize)]
struct SearchInput {
    search_query: String,
    fuzzy_match: bool,
}

#[tokio::test]
async fn search_uses_bearer_auth_bounds_results_and_caches() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = calls.clone();
    let base_url = serve(Router::new().route(
        "/v1/search",
        get(
            move |headers: HeaderMap, Query(input): Query<SearchInput>| {
                let calls = handler_calls.clone();
                async move {
                    assert_eq!(headers.get("authorization").unwrap(), "Bearer secret-key");
                    assert_eq!(input.search_query, "Oslo golf");
                    let call_index = calls.fetch_add(1, Ordering::SeqCst);
                    assert_eq!(input.fuzzy_match, call_index == 0);
                    let courses = (0..25)
                        .map(|index| {
                            json!({
                                "id": format!("abcde{index:03}"),
                                "club_name": format!("Club {index}"),
                                "course_name": format!("Course {index}"),
                                "location": {"country": "Norway"},
                                "tees": {"female": 2, "male": 3}
                            })
                        })
                        .collect::<Vec<_>>();
                    Json(json!({"courses": courses}))
                }
            },
        ),
    ))
    .await;
    let client =
        CourseProviderClient::configured_with_base_url("secret-key".into(), &base_url).unwrap();

    let first = client.search("Oslo golf", true).await.unwrap();
    let second = client.search("Oslo golf", true).await.unwrap();
    let exact = client.search("Oslo golf", false).await.unwrap();

    assert_eq!(first.len(), 20);
    assert_eq!(second.len(), 20);
    assert_eq!(exact.len(), 20);
    assert_eq!(first[0].tee_counts.female, 2);
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn detail_normalizes_groups_without_inventing_a_tee_id() {
    let base_url = serve(Router::new().route(
        "/v1/courses/{id}",
        get(|Path(id): Path<String>| async move {
            assert_eq!(id, "7k2m9qb4");
            Json(json!({
                "id": id,
                "club_name": "Murray Golf Club",
                "course_name": "Course No. 1",
                "location": {"city": "Murray", "country": "United States"},
                "tees": {
                    "female": [],
                    "male": [{
                        "tee_name": "Blue",
                        "course_rating": 75.7,
                        "slope_rating": 132,
                        "total_yards": 6348,
                        "total_meters": 5805,
                        "number_of_holes": 1,
                        "par_total": 4,
                        "holes": [{"par": 4, "yardage": 484, "handicap": 9}]
                    }]
                }
            }))
        }),
    ))
    .await;
    let client = CourseProviderClient::configured_with_base_url("key".into(), &base_url).unwrap();

    let course = client.course("7k2m9qb4").await.unwrap();

    assert_eq!(course.tees.len(), 1);
    assert_eq!(course.tees[0].name, "Blue");
    assert_eq!(course.tees[0].holes[0].number, 1);
    assert_eq!(course.tees[0].holes[0].stroke_index, 9);
    let serialized = serde_json::to_value(course).unwrap();
    assert!(serialized["tees"][0].get("id").is_none());
}

#[tokio::test]
async fn upstream_statuses_are_mapped_without_reading_error_bodies() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = calls.clone();
    let base_url = serve(Router::new().route(
        "/v1/courses/{id}",
        get(move |Path(id): Path<String>| {
            let calls = handler_calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                match id.as_str() {
                    "abcde123" => (StatusCode::NOT_FOUND, "missing"),
                    "abcde124" => (StatusCode::TOO_MANY_REQUESTS, "limited"),
                    _ => (StatusCode::UNAUTHORIZED, "do not expose this"),
                }
            }
        }),
    ))
    .await;
    let client = CourseProviderClient::configured_with_base_url("key".into(), &base_url).unwrap();

    assert_eq!(
        client.course("abcde123").await.unwrap_err(),
        CourseProviderError::NotFound
    );
    assert_eq!(
        client.course("abcde124").await.unwrap_err(),
        CourseProviderError::Exhausted
    );
    assert_eq!(
        client.course("abcde125").await.unwrap_err(),
        CourseProviderError::Exhausted
    );
    assert_eq!(calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn oversized_and_invalid_json_responses_fail_closed() {
    let oversized = "x".repeat(super::MAX_RESPONSE_BYTES + 1);
    let base_url = serve(Router::new().route(
        "/v1/search",
        get(move || {
            let oversized = oversized.clone();
            async move { oversized }
        }),
    ))
    .await;
    let client = CourseProviderClient::configured_with_base_url("key".into(), &base_url).unwrap();
    assert_eq!(
        client.search("valid", true).await.unwrap_err(),
        CourseProviderError::InvalidResponse
    );

    let invalid_base = serve(Router::new().route(
        "/v1/search",
        get(|| async { Json(Value::String("wrong shape".to_owned())) }),
    ))
    .await;
    let invalid =
        CourseProviderClient::configured_with_base_url("key".into(), &invalid_base).unwrap();
    assert_eq!(
        invalid.search("valid", true).await.unwrap_err(),
        CourseProviderError::InvalidResponse
    );
}

#[tokio::test]
async fn timeout_and_saturation_are_deliberate_errors() {
    let timeout_base = serve(Router::new().route(
        "/v1/search",
        get(|| async {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Json(json!({"courses": []}))
        }),
    ))
    .await;
    let timeout_client = CourseProviderClient::configured_for_test(
        "key".into(),
        &timeout_base,
        Duration::from_millis(20),
        1,
    )
    .unwrap();
    assert_eq!(
        timeout_client.search("timeout", true).await.unwrap_err(),
        CourseProviderError::Timeout
    );

    let (started_tx, mut started_rx) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let handler_release = release.clone();
    let saturation_base = serve(Router::new().route(
        "/v1/search",
        get(move || {
            let started_tx = started_tx.clone();
            let release = handler_release.clone();
            async move {
                started_tx.send(()).await.unwrap();
                release.notified().await;
                Json(json!({"courses": []}))
            }
        }),
    ))
    .await;
    let saturation_client = CourseProviderClient::configured_for_test(
        "key".into(),
        &saturation_base,
        Duration::from_secs(1),
        1,
    )
    .unwrap();
    let first_client = saturation_client.clone();
    let first = tokio::spawn(async move { first_client.search("first", true).await });
    started_rx.recv().await.unwrap();
    assert_eq!(
        saturation_client.search("second", true).await.unwrap_err(),
        CourseProviderError::Saturated
    );
    release.notify_one();
    assert!(first.await.unwrap().is_ok());
}

#[tokio::test]
async fn local_daily_quota_is_checked_after_cache_and_before_upstream() {
    let base_url =
        serve(Router::new().route("/v1/search", get(|| async { Json(json!({"courses": []})) })))
            .await;
    let client =
        CourseProviderClient::build("key".into(), &base_url, 1, Duration::from_secs(1), 1).unwrap();

    assert!(client.search("cached", true).await.is_ok());
    assert!(client.search("cached", true).await.is_ok());
    assert_eq!(
        client.search("uncached", true).await.unwrap_err(),
        CourseProviderError::Exhausted
    );
}

#[test]
fn boundary_validation_is_bounded_and_course_ids_remain_opaque() {
    assert_eq!(validate_search_query("  Oslo  ").unwrap(), "Oslo");
    assert!(validate_search_query("x").is_err());
    assert!(validate_search_query(&"x".repeat(81)).is_err());
    assert!(validate_search_query("two\nlines").is_err());
    assert_eq!(normalize_course_id("7K2M9QB4").unwrap(), "7k2m9qb4");
    assert!(normalize_course_id("12345678").is_err());
    assert!(normalize_course_id("with_i00").is_err());
}
