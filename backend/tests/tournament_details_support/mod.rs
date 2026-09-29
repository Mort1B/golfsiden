#![allow(unused_imports, dead_code)]
pub use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
pub use chrono::{Duration, Utc};
pub use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::auth,
};
pub use http_body_util::BodyExt;
pub use serde_json::{Value, json};
pub use sqlx::PgPool;
pub use std::sync::Arc;
pub use tokio::sync::broadcast::error::TryRecvError;
pub use tower::ServiceExt;
pub use uuid::{Uuid, uuid};
pub const TRIP: Uuid = uuid!("33000000-0000-0000-0000-000000000001");
pub const ADMIN: Uuid = uuid!("33000000-0000-0000-0000-000000000002");
pub const OTHER: Uuid = uuid!("33000000-0000-0000-0000-000000000003");
pub const TOKEN: &str = "details-admin-fixture";
pub const OTHER_TOKEN: &str = "details-other-fixture";
pub async fn seed(pool: &PgPool) {
    sqlx::raw_sql("INSERT INTO users(id,username,display_name,role) VALUES
    ('33000000-0000-0000-0000-000000000002','details_admin','Admin','player'),
    ('33000000-0000-0000-0000-000000000003','details_other','Other','admin');
    INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES
    ('33000000-0000-0000-0000-000000000001','Original','2026-09-01','2026-09-03',1,1);
    INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES
    ('33000000-0000-0000-0000-000000000001','33000000-0000-0000-0000-000000000002','admin'),
    ('33000000-0000-0000-0000-000000000001','33000000-0000-0000-0000-000000000003','viewer');
    INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES
    ('33000000-0000-0000-0000-000000000010','33000000-0000-0000-0000-000000000001',1,'Round','2026-09-02','','','individual_stroke_play');
    INSERT INTO players(id,display_name,current_handicap_index) VALUES ('33000000-0000-0000-0000-000000000020','Player',10);
    INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES
    ('33000000-0000-0000-0000-000000000001','33000000-0000-0000-0000-000000000020',10);")
        .execute(pool).await.unwrap();
    for (id, token) in [(ADMIN, TOKEN), (OTHER, OTHER_TOKEN)] {
        auth::create_session(
            pool,
            id,
            &hash_session_token(token),
            Utc::now() + Duration::hours(1),
        )
        .await
        .unwrap();
    }
}
pub async fn payload(pool: &PgPool) -> Value {
    let version: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
            .bind(TRIP)
            .fetch_one(pool)
            .await
            .unwrap();
    json!({"name":"  Updated  ","description":" New description ","start_date":"2026-09-01","end_date":"2026-09-04","expected_tournament_updated_at":version})
}
pub fn request(token: &str, body: Value) -> Request<Body> {
    Request::patch(format!("/api/tournaments/{TRIP}/details"))
        .header(header::COOKIE, format!("golf_session={token}"))
        .header("x-csrf-token", derive_csrf_token(token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}
pub async fn json_body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
pub async fn preserved(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('rounds',(SELECT jsonb_agg(to_jsonb(r)) FROM rounds r),'players',(SELECT jsonb_agg(to_jsonb(p)) FROM tournament_players p),'other',(SELECT to_jsonb(t)-ARRAY['name','description','start_date','end_date','updated_at'] FROM tournaments t WHERE id=$1))")
        .bind(TRIP).fetch_one(pool).await.unwrap()
}
