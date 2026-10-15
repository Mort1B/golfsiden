use super::super::CourseProviderApiError;
use crate::{
    auth::hash_session_token,
    course_provider::{CourseDetail, CourseProviderClient},
    repositories::auth,
};
use axum::{
    Json, Router,
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{net::TcpListener, sync::Notify, task::JoinHandle};
use uuid::{Uuid, uuid};

pub const USER: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
pub const TRIP: Uuid = uuid!("00000000-0000-0000-0000-000000002001");
pub const COURSE: &str = "7k2m9qb4";
pub async fn fixture(pool: &PgPool) {
    sqlx::raw_sql(include_str!("../../../../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
}
pub async fn session(pool: &PgPool) -> Uuid {
    auth::create_session(
        pool,
        USER,
        &hash_session_token(&Uuid::new_v4().to_string()),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id
}
pub fn output(result: Result<CourseDetail, CourseProviderApiError>) -> Response {
    match result {
        Ok(course) => Json(course).into_response(),
        Err(error) => error.into_response(),
    }
}
pub async fn body(response: Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
pub struct Mock {
    pub client: CourseProviderClient,
    pub calls: Arc<AtomicUsize>,
    pub requested: Arc<Notify>,
    pub release: Arc<Notify>,
    server: JoinHandle<()>,
}
impl Drop for Mock {
    fn drop(&mut self) {
        self.server.abort();
    }
}
pub async fn provider(gated: bool, error: bool) -> Mock {
    let calls = Arc::new(AtomicUsize::new(0));
    let requested = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let count = Arc::clone(&calls);
    let started = Arc::clone(&requested);
    let gate = Arc::clone(&release);
    let router=Router::new().route("/v1/courses/{id}",get(move |Path(id): Path<String>| {
        let calls=Arc::clone(&count);let started=Arc::clone(&started);let gate=Arc::clone(&gate);
        async move {
            calls.fetch_add(1,Ordering::SeqCst);started.notify_one();if gated {gate.notified().await;}
            if error {return StatusCode::BAD_GATEWAY.into_response();}
            Json(json!({"course":{"id":id,"club_name":"Synthetic club","course_name":"Synthetic course","tees":{"female":[],"male":[{"tee_name":"Blue","course_rating":72.0,"slope_rating":113,"total_yards":400,"total_meters":366,"number_of_holes":1,"par_total":4,"holes":[{"par":4,"yardage":400,"handicap":1}]}]}}})).into_response()
        }
    }));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let client = CourseProviderClient::configured_with_base_url(
        "synthetic-local-key".into(),
        &format!("http://{address}/"),
    )
    .unwrap();
    Mock {
        client,
        calls,
        requested,
        release,
        server,
    }
}
