use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use axum::{
    Json, Router,
    extract::Path,
    http::{HeaderMap, StatusCode},
    routing::get,
};
use serde_json::{Value, json};
use tokio::{
    net::TcpListener,
    sync::{Notify, mpsc},
};

use super::{CourseProviderClient, CourseProviderError, normalize_course_id};

async fn serve(router: Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    format!("http://{address}/")
}

fn valid_course(id: &str) -> Value {
    json!({"course": {
        "id": id,
        "club_name": "Murray Golf Club",
        "course_name": "Course No. 1",
        "location": {"city": "Murray", "country": "United States"},
        "tees": {"female": [], "male": [{
            "tee_name": "Blue",
            "course_rating": 75.7,
            "slope_rating": 132,
            "total_yards": 484,
            "total_meters": 443,
            "number_of_holes": 1,
            "par_total": 4,
            "holes": [{"par": 4, "yardage": 484, "handicap": 1}]
        }]}
    }})
}

#[tokio::test]
async fn detail_uses_bearer_auth_parses_envelope_and_caches() {
    let calls = Arc::new(AtomicUsize::new(0));
    let handler_calls = calls.clone();
    let base_url = serve(Router::new().route(
        "/v1/courses/{id}",
        get(move |headers: HeaderMap, Path(id): Path<String>| {
            let calls = handler_calls.clone();
            async move {
                assert_eq!(headers.get("authorization").unwrap(), "Bearer secret-key");
                calls.fetch_add(1, Ordering::SeqCst);
                Json(valid_course(&id))
            }
        }),
    ))
    .await;
    let client =
        CourseProviderClient::configured_with_base_url("secret-key".into(), &base_url).unwrap();

    let first = client.course("7k2m9qb4").await.unwrap();
    let second = client.course("7k2m9qb4").await.unwrap();

    assert_eq!(first.tees.len(), 1);
    assert_eq!(first.tees[0].name, "Blue");
    assert_eq!(first.tees[0].holes[0].number, 1);
    assert_eq!(first.tees[0].holes[0].stroke_index, 1);
    assert_eq!(second.provider_course_id, "7k2m9qb4");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    let serialized = serde_json::to_value(first).unwrap();
    assert!(serialized["tees"][0].get("id").is_none());
}

#[tokio::test]
async fn detail_rejects_missing_stroke_index_and_wrong_envelope() {
    let missing_handicap = serve(Router::new().route(
        "/v1/courses/{id}",
        get(|Path(id): Path<String>| async move {
            let mut value = valid_course(&id);
            value["course"]["tees"]["male"][0]["holes"][0]
                .as_object_mut()
                .unwrap()
                .remove("handicap");
            Json(value)
        }),
    ))
    .await;
    let client =
        CourseProviderClient::configured_with_base_url("key".into(), &missing_handicap).unwrap();
    assert_eq!(
        client.course("7k2m9qb4").await.unwrap_err(),
        CourseProviderError::InvalidResponse
    );

    let wrong_envelope = serve(Router::new().route(
        "/v1/courses/{id}",
        get(|Path(id): Path<String>| async move { Json(valid_course(&id)["course"].clone()) }),
    ))
    .await;
    let client =
        CourseProviderClient::configured_with_base_url("key".into(), &wrong_envelope).unwrap();
    assert_eq!(
        client.course("7k2m9qb4").await.unwrap_err(),
        CourseProviderError::InvalidResponse
    );
}

#[tokio::test]
async fn detail_rejects_courses_without_tees() {
    let base_url = serve(Router::new().route(
        "/v1/courses/{id}",
        get(|Path(id): Path<String>| async move {
            let mut value = valid_course(&id);
            value["course"]["tees"] = json!({"female": [], "male": []});
            Json(value)
        }),
    ))
    .await;
    let client = CourseProviderClient::configured_with_base_url("key".into(), &base_url).unwrap();
    assert_eq!(
        client.course("7k2m9qb4").await.unwrap_err(),
        CourseProviderError::InvalidResponse
    );
}

#[tokio::test]
async fn detail_rejects_stroke_indexes_outside_the_hole_count_permutation() {
    let base_url = serve(Router::new().route(
        "/v1/courses/{id}",
        get(|Path(id): Path<String>| async move {
            let mut value = valid_course(&id);
            value["course"]["tees"]["male"][0]["holes"][0]["handicap"] = json!(2);
            Json(value)
        }),
    ))
    .await;
    let client = CourseProviderClient::configured_with_base_url("key".into(), &base_url).unwrap();
    assert_eq!(
        client.course("7k2m9qb4").await.unwrap_err(),
        CourseProviderError::InvalidResponse
    );
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
        "/v1/courses/{id}",
        get(move || {
            let oversized = oversized.clone();
            async move { oversized }
        }),
    ))
    .await;
    let client = CourseProviderClient::configured_with_base_url("key".into(), &base_url).unwrap();
    assert_eq!(
        client.course("abcde123").await.unwrap_err(),
        CourseProviderError::InvalidResponse
    );

    let invalid_base = serve(Router::new().route(
        "/v1/courses/{id}",
        get(|| async { Json(Value::String("wrong shape".to_owned())) }),
    ))
    .await;
    let invalid =
        CourseProviderClient::configured_with_base_url("key".into(), &invalid_base).unwrap();
    assert_eq!(
        invalid.course("abcde123").await.unwrap_err(),
        CourseProviderError::InvalidResponse
    );
}

#[tokio::test]
async fn timeout_and_saturation_are_deliberate_errors() {
    let timeout_base = serve(Router::new().route(
        "/v1/courses/{id}",
        get(|Path(id): Path<String>| async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            Json(valid_course(&id))
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
        timeout_client.course("abcde123").await.unwrap_err(),
        CourseProviderError::Timeout
    );

    let (started_tx, mut started_rx) = mpsc::channel(1);
    let release = Arc::new(Notify::new());
    let handler_release = release.clone();
    let saturation_base = serve(Router::new().route(
        "/v1/courses/{id}",
        get(move |Path(id): Path<String>| {
            let started_tx = started_tx.clone();
            let release = handler_release.clone();
            async move {
                started_tx.send(()).await.unwrap();
                release.notified().await;
                Json(valid_course(&id))
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
    let first = tokio::spawn(async move { first_client.course("abcde123").await });
    started_rx.recv().await.unwrap();
    assert_eq!(
        saturation_client.course("abcde124").await.unwrap_err(),
        CourseProviderError::Saturated
    );
    release.notify_one();
    assert!(first.await.unwrap().is_ok());
}

#[tokio::test]
async fn local_daily_quota_is_checked_after_cache_and_before_upstream() {
    let base_url = serve(Router::new().route(
        "/v1/courses/{id}",
        get(|Path(id): Path<String>| async move { Json(valid_course(&id)) }),
    ))
    .await;
    let client =
        CourseProviderClient::build("key".into(), &base_url, 1, Duration::from_secs(1), 1).unwrap();

    assert!(client.course("abcde123").await.is_ok());
    assert!(client.course("abcde123").await.is_ok());
    assert_eq!(
        client.course("abcde124").await.unwrap_err(),
        CourseProviderError::Exhausted
    );
}

#[test]
fn course_ids_remain_opaque_and_bounded() {
    assert_eq!(normalize_course_id("7K2M9QB4").unwrap(), "7k2m9qb4");
    assert!(normalize_course_id("12345678").is_err());
    assert!(normalize_course_id("with_i00").is_err());
}
