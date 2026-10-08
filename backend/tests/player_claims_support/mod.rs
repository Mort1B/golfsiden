#![allow(unused_imports, dead_code)]
pub use axum::{
    body::Body,
    http::{Request, StatusCode},
};
pub use chrono::{Duration, Utc};
pub use golf_api::{
    AppState, api,
    auth::{
        derive_csrf_token, generate_invitation_token, hash_invitation_token, hash_session_token,
    },
    repositories::{
        auth,
        player_claims::{self, ClaimError, Grant, RegisterParams},
    },
};
pub use http_body_util::BodyExt;
pub use serde_json::{Value, json};
pub use sqlx::PgPool;
pub use tower::ServiceExt;
pub use uuid::Uuid;
pub const ADMIN_TOKEN: &str = "claim-admin-session";
pub struct Fixture {
    pub tournament: Uuid,
    pub admin: Uuid,
    pub other: Uuid,
    pub session: Uuid,
}
pub async fn seed(pool: &PgPool) -> Fixture {
    let f = Fixture {
        tournament: Uuid::new_v4(),
        admin: Uuid::new_v4(),
        other: Uuid::new_v4(),
        session: Uuid::nil(),
    };
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES($1,'claims_admin','Admin','player'),($2,'claims_other','Other','admin')").bind(f.admin).bind(f.other).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES($1,'Claim Trip','2026-10-01','2026-10-03',1,1)").bind(f.tournament).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(f.tournament)
    .bind(f.admin)
    .execute(pool)
    .await
    .unwrap();
    let session = auth::create_session(
        pool,
        f.admin,
        &hash_session_token(ADMIN_TOKEN),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id;
    Fixture { session, ..f }
}
pub async fn prepared(pool: &PgPool, f: &Fixture) -> (Grant, String) {
    let token = generate_invitation_token().unwrap();
    let grant = player_claims::create(
        pool,
        f.session,
        f.tournament,
        "Prepared Player",
        12.3,
        &hash_invitation_token(&token),
    )
    .await
    .unwrap();
    (grant, token)
}
pub async fn claim(
    pool: &PgPool,
    id: Uuid,
    token: &str,
    username: &str,
) -> Result<golf_api::repositories::invitations::RegisteredPlayer, ClaimError> {
    player_claims::register(
        pool,
        RegisterParams {
            id,
            token,
            username,
            password_hash: "test-hash",
            session_token_hash: &hash_session_token(&Uuid::new_v4().to_string()),
            session_expires_at: Utc::now() + Duration::hours(1),
        },
    )
    .await
}
pub async fn history(pool: &PgPool, player: Uuid) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('global',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM handicap_history h WHERE player_id=$1),'tournament',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM tournament_handicap_history h WHERE player_id=$1),'snapshots',(SELECT jsonb_agg(to_jsonb(h) ORDER BY round_id) FROM round_handicap_snapshots h WHERE player_id=$1),'audits',(SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM score_audits a JOIN scores s ON s.id=a.score_id WHERE s.player_id=$1),'scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM scores s WHERE player_id=$1))").bind(player).fetch_one(pool).await.unwrap()
}
pub fn request(method: &str, path: &str, token: Option<&str>, value: Value) -> Request<Body> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        request = request
            .header("cookie", format!("golf_session={token}"))
            .header("x-csrf-token", derive_csrf_token(token));
    }
    request.body(Body::from(value.to_string())).unwrap()
}
pub async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
pub async fn round(pool: &PgPool, f: &Fixture) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES($1,$2,1,'Round','2026-10-01','','','individual_stroke_play')").bind(id).bind(f.tournament).execute(pool).await.unwrap();
    id
}
// Owner-only fixture construction isolates already valid historical states;
// production guards remain enabled for every operation under test.
pub async fn historical_status(pool: &PgPool, round: Uuid, player: Uuid, status: &str) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::raw_sql("ALTER TABLE rounds DISABLE TRIGGER USER; ALTER TABLE round_handicap_snapshots DISABLE TRIGGER USER").execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO round_handicap_snapshots(round_id,tournament_id,player_id,handicap_index,course_handicap,playing_handicap) SELECT id,tournament_id,$2,12.3,12,12 FROM rounds WHERE id=$1 ON CONFLICT DO NOTHING").bind(round).bind(player).execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE rounds SET status=$2::round_status WHERE id=$1")
        .bind(round)
        .bind(status)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::raw_sql("SET CONSTRAINTS ALL IMMEDIATE; ALTER TABLE rounds ENABLE TRIGGER USER; ALTER TABLE round_handicap_snapshots ENABLE TRIGGER USER").execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}
pub async fn closed_status(pool: &PgPool, tournament: Uuid, status: &str) {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("ALTER TABLE tournaments DISABLE TRIGGER USER")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE tournaments SET status=$2::tournament_status WHERE id=$1")
        .bind(tournament)
        .bind(status)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE tournaments ENABLE TRIGGER USER")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
}

pub async fn wait_for_lock(pool: &PgPool, pattern: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(5),async{
        loop{
            let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND wait_event_type='Lock' AND query LIKE $1)").bind(pattern).fetch_one(pool).await.unwrap();
            if blocked{break;} tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
}

pub async fn historical_score(pool: &PgPool, f: &Fixture, round: Uuid, player: Uuid) {
    let course = Uuid::new_v4();
    let tee = Uuid::new_v4();
    let hole = Uuid::new_v4();
    let score = Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO courses(id,name) VALUES($1,'Historical course')")
        .bind(course)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tees(id,course_id,name,slope_rating,course_rating) VALUES($1,$2,'Historical tee',113,4)").bind(tee).bind(course).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO holes(id,tee_id,hole_number,par,stroke_index) VALUES($1,$2,1,4,1)")
        .bind(hole)
        .bind(tee)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::raw_sql("ALTER TABLE rounds DISABLE TRIGGER USER; ALTER TABLE scores DISABLE TRIGGER USER; ALTER TABLE score_audits DISABLE TRIGGER USER").execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE rounds SET course_id=$2,tee_id=$3,number_of_holes=1 WHERE id=$1")
        .bind(round)
        .bind(course)
        .bind(tee)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("INSERT INTO scores(id,round_id,tournament_id,hole_id,player_id,gross_strokes,submitted_by) VALUES($1,$2,$3,$4,$5,4,$6)").bind(score).bind(round).bind(f.tournament).bind(hole).bind(player).bind(f.admin).execute(&mut *tx).await.unwrap();
    sqlx::query(
        "INSERT INTO score_audits(id,score_id,changed_by,new_gross_strokes) VALUES($1,$2,$3,4)",
    )
    .bind(Uuid::new_v4())
    .bind(score)
    .bind(f.admin)
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::raw_sql("SET CONSTRAINTS ALL IMMEDIATE; ALTER TABLE rounds ENABLE TRIGGER USER; ALTER TABLE scores ENABLE TRIGGER USER; ALTER TABLE score_audits ENABLE TRIGGER USER").execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
}
