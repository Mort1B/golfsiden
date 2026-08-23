#![cfg(feature = "database-tests")]

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use axum::{
    Json, Router,
    body::Body,
    extract::Path,
    http::{Request, StatusCode, header},
    routing::get,
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api, auth::hash_session_token, course_provider::CourseProviderClient,
    repositories::auth,
};
use http_body_util::BodyExt;
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
        VALUES ('8c000000-0000-0000-0000-000000000001', 'Catalog trip', '2026-08-01', '2026-08-02', 2);
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

async fn mock_provider(calls: Arc<AtomicUsize>) -> String {
    let app = Router::new().route(
        "/v1/courses/{id}",
        get(move |Path(id): Path<String>| {
            let calls = calls.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Json(json!({"course": {
                    "id": id,
                    "club_name": "Unexpected upstream call",
                    "course_name": "Unexpected upstream call",
                    "tees": {"female": [], "male": []}
                }}))
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    format!("http://{address}/")
}

#[sqlx::test(migrations = "../migrations")]
async fn local_catalog_is_admin_scoped_searchable_and_gates_provider_detail(pool: PgPool) {
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
    let catalog_path = format!("/api/tournaments/{TOURNAMENT_ID}/course-catalog");

    let unauthenticated = app
        .clone()
        .oneshot(request(&catalog_path, None))
        .await
        .unwrap();
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        body(unauthenticated).await["error"]["code"],
        "unauthenticated"
    );

    let forbidden = app
        .clone()
        .oneshot(request(&catalog_path, Some("course-outsider")))
        .await
        .unwrap();
    assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    assert_eq!(body(forbidden).await["error"]["code"], "forbidden");

    let all = app
        .clone()
        .oneshot(request(&catalog_path, Some("course-admin")))
        .await
        .unwrap();
    assert_eq!(all.status(), StatusCode::OK);
    assert_eq!(all.headers()[header::CACHE_CONTROL], "private, no-store");
    let all_body = body(all).await;
    assert_eq!(all_body["courses"].as_array().unwrap().len(), 8);
    assert_eq!(all_body["courses"][0]["display_name"], "Hacienda del Álamo");
    assert_eq!(all_body["courses"][0]["provider_course_id"], Value::Null);
    assert_eq!(all_body["courses"][0]["provider_status"], "missing");
    assert_eq!(all_body["courses"][5]["display_name"], "Miklagard GK");
    assert_eq!(all_body["courses"][5]["provider_course_id"], "0zm1pe1a");
    assert_eq!(all_body["courses"][5]["provider_status"], "incomplete");
    assert_eq!(all_body["courses"][7]["display_name"], "Haga GK");
    assert_eq!(all_body["courses"][0]["provider"], "golf_course_api");
    assert!(all_body["courses"][0].get("aliases").is_none());

    for (query, expected) in [
        ("Hacienda%20del%20Alamos", "Hacienda del Álamo"),
        ("OPPEGARD", "Oppegård GK"),
        ("dr%C3%B8bak", "Drøbak GK"),
    ] {
        let response = app
            .clone()
            .oneshot(request(
                &format!("{catalog_path}?q={query}"),
                Some("course-admin"),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response_body = body(response).await;
        assert_eq!(response_body["courses"].as_array().unwrap().len(), 1);
        assert_eq!(response_body["courses"][0]["display_name"], expected);
    }

    for suffix in ["?q=%20%20", "?q=x", "?q=two%0Alines", "?fuzzy_match=true"] {
        let response = app
            .clone()
            .oneshot(request(
                &format!("{catalog_path}{suffix}"),
                Some("course-admin"),
            ))
            .await
            .unwrap();
        if suffix == "?q=%20%20" {
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(body(response).await["courses"].as_array().unwrap().len(), 8);
        } else {
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert_eq!(body(response).await["error"]["code"], "validation_error");
        }
    }
    let oversized = format!("{catalog_path}?q={}", "x".repeat(81));
    let response = app
        .clone()
        .oneshot(request(&oversized, Some("course-admin")))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(body(response).await["error"]["code"], "validation_error");

    let removed_search = app
        .clone()
        .oneshot(request(
            &format!("/api/tournaments/{TOURNAMENT_ID}/course-provider/search?q=Oslo"),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(removed_search.status(), StatusCode::NOT_FOUND);

    let incomplete = app
        .clone()
        .oneshot(request(
            &format!("/api/tournaments/{TOURNAMENT_ID}/course-provider/courses/DCM3CN0G"),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(incomplete.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(incomplete).await["error"]["code"],
        "course_catalog_incomplete"
    );

    let unknown = app
        .clone()
        .oneshot(request(
            &format!("/api/tournaments/{TOURNAMENT_ID}/course-provider/courses/abcde123"),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
    assert_eq!(
        body(unknown).await["error"]["code"],
        "course_catalog_not_found"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let app_without_key = api::router(AppState::new(pool));
    let local_without_key = app_without_key
        .oneshot(request(
            &format!("{catalog_path}?q=Oslo"),
            Some("course-admin"),
        ))
        .await
        .unwrap();
    assert_eq!(local_without_key.status(), StatusCode::OK);
    assert_eq!(
        body(local_without_key).await["courses"][0]["display_name"],
        "Oslo GK"
    );
}
