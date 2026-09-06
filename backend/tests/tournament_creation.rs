#![cfg(feature = "database-tests")]

#[sqlx::test(migrations = "../migrations")]
async fn successful_yesterday_request_can_replay_but_new_past_plan_cannot_create(pool: PgPool) {
    use sha2::{Digest, Sha256};
    seed(&pool).await;
    session(&pool, USER, TOKEN).await;
    let key = Uuid::new_v4();
    let mut old = plan();
    old.start_date -= Duration::days(3);
    old.end_date -= Duration::days(3);
    old.rounds[0].round_date = old.start_date;
    old.invitation_expires_at -= Duration::days(3);
    let hash = Sha256::digest(serde_json::to_vec(&old).unwrap()).to_vec();
    let mut tx = pool.begin().await.unwrap();
    let (trip, _) =
        golf_api::repositories::tournament_plan::insert(&mut tx, USER, Some((USER, 8.2)), &old)
            .await
            .unwrap();
    sqlx::query("INSERT INTO tournament_creation_requests(user_id,request_id,request_hash,tournament_id) VALUES($1,$2,$3,$4)").bind(USER).bind(key).bind(hash).bind(trip.id).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let mut value = payload(key);
    value["tournament"]["start_date"] = json!(old.start_date);
    value["tournament"]["end_date"] = json!(old.end_date);
    value["tournament"]["mandatory_round_number"] = Value::Null;
    value["rounds"] = json!([{"round_number":1,"name":"Individual","round_date":old.start_date,"scoring_format":"individual_stroke_play"}]);
    let app = api::router(AppState::new(pool.clone()));
    let response = app
        .clone()
        .oneshot(request(&value, Some(TOKEN), true))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await["tournament_id"], trip.id.to_string());
    let before = counts(&pool).await;
    value["request_id"] = json!(Uuid::new_v4());
    assert_eq!(
        app.oneshot(request(&value, Some(TOKEN), true))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(counts(&pool).await, before);
}
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::{auth, tournament_creation},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};
const USER: Uuid = uuid!("00000000-0000-0000-0000-000000001001");
const ADMIN: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
const OLD: Uuid = uuid!("00000000-0000-0000-0000-000000002001");
const TOKEN: &str = "create-existing-user";
async fn seed(pool: &PgPool) {
    sqlx::raw_sql(include_str!("../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
}
async fn session(pool: &PgPool, user: Uuid, token: &str) -> Uuid {
    auth::create_session(
        pool,
        user,
        &hash_session_token(token),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id
}
fn payload(key: Uuid) -> Value {
    let date = (Utc::now() + Duration::days(2)).date_naive();
    json!({"request_id":key,"tournament":{"name":" Another trip ","description":" Test first ","start_date":date,"end_date":date,"counted_rounds":1,"mandatory_round_number":2},
      "rounds":[{"round_number":2,"name":"Foursomes","round_date":date,"scoring_format":"two_player_foursomes"},{"round_number":1,"name":"Individual","round_date":date,"scoring_format":"individual_stroke_play"}]})
}
fn request(value: &Value, token: Option<&str>, csrf: bool) -> Request<Body> {
    let mut r = Request::post("/api/tournaments").header("content-type", "application/json");
    if let Some(token) = token {
        r = r.header("cookie", format!("golf_session={token}"));
        if csrf {
            r = r.header("x-csrf-token", derive_csrf_token(token));
        }
    }
    r.body(Body::from(value.to_string())).unwrap()
}
async fn body(r: axum::response::Response) -> Value {
    serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
async fn counts(pool: &PgPool) -> (i64, i64, i64, i64, i64) {
    sqlx::query_as("SELECT (SELECT count(*) FROM tournaments),(SELECT count(*) FROM rounds),(SELECT count(*) FROM tournament_memberships),(SELECT count(*) FROM tournament_players),(SELECT count(*) FROM tournament_creation_requests)").fetch_one(pool).await.unwrap()
}
fn plan() -> golf_api::domain::tournament_plan::ValidatedTournamentPlan {
    use golf_api::domain::{
        models::ScoringFormat,
        tournament_plan::{RoundInput, TournamentPlanInput, validate},
    };
    let date = (Utc::now() + Duration::days(2)).date_naive();
    validate(
        TournamentPlanInput {
            tournament_name: "Another trip".into(),
            description: "Test first".into(),
            start_date: date,
            end_date: date,
            counted_rounds: 1,
            mandatory_round_number: None,
            rounds: vec![RoundInput {
                round_number: 1,
                name: "Individual".into(),
                round_date: date,
                scoring_format: ScoringFormat::IndividualStrokePlay,
            }],
        },
        Utc::now().date_naive(),
    )
    .unwrap()
}
#[sqlx::test(migrations = "../migrations")]
async fn existing_player_creates_two_trips_without_account_session_or_history_replacement(
    pool: PgPool,
) {
    seed(&pool).await;
    session(&pool, USER, TOKEN).await;
    let identity:Value=sqlx::query_scalar("SELECT jsonb_build_object('users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),'sessions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM user_sessions s),'players',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM players p),'old',(SELECT jsonb_agg(to_jsonb(t) ORDER BY player_id) FROM tournament_players t WHERE tournament_id=$1))").bind(OLD).fetch_one(&pool).await.unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let mut ids = vec![];
    for _ in 0..2 {
        let key = Uuid::new_v4();
        let value = payload(key);
        let response = app
            .clone()
            .oneshot(request(&value, Some(TOKEN), true))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert!(!response.headers().contains_key("set-cookie"));
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let r = body(response).await;
        assert_eq!(r["request_id"], key.to_string());
        assert_eq!(r["created"], true);
        let id: Uuid = r["tournament_id"].as_str().unwrap().parse().unwrap();
        ids.push(id);
        let facts:(String,String,i16,i16,String)=sqlx::query_as("SELECT name,status::text,number_of_rounds,counted_rounds,scoring_mode::text FROM tournaments WHERE id=$1").bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(
            facts,
            (
                "Another trip".into(),
                "draft".into(),
                2,
                1,
                "combined".into()
            )
        );
        let role: String = sqlx::query_scalar(
            "SELECT role::text FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(id)
        .bind(USER)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(role, "admin");
        let h:f64=sqlx::query_scalar("SELECT tournament_handicap::float8 FROM tournament_players WHERE tournament_id=$1 AND player_id=$2").bind(id).bind(USER).fetch_one(&pool).await.unwrap();
        assert_eq!(h, 8.2);
        let history:f64=sqlx::query_scalar("SELECT handicap_index::float8 FROM tournament_handicap_history WHERE tournament_id=$1 AND player_id=$2 AND changed_by=$2").bind(id).bind(USER).fetch_one(&pool).await.unwrap();
        assert_eq!(history, 8.2);
        let rows:Vec<(i16,String,Option<Uuid>,i16)>=sqlx::query_as("SELECT round_number,status::text,course_id,handicap_allowance_percent FROM rounds WHERE tournament_id=$1 ORDER BY round_number").bind(id).fetch_all(&pool).await.unwrap();
        assert_eq!(
            rows,
            vec![
                (1, "draft".into(), None, 100),
                (2, "draft".into(), None, 50)
            ]
        );
        let mandatory:i16=sqlx::query_scalar("SELECT r.round_number FROM rounds r JOIN tournaments t ON t.mandatory_round_id=r.id WHERE t.id=$1").bind(id).fetch_one(&pool).await.unwrap();
        assert_eq!(mandatory, 2);
        assert_eq!(events.try_recv().unwrap().tournament_id, id);
        let replay = app
            .clone()
            .oneshot(request(&value, Some(TOKEN), true))
            .await
            .unwrap();
        assert_eq!(replay.status(), StatusCode::OK);
        assert_eq!(body(replay).await["created"], false);
        assert!(events.try_recv().is_err());
        let mut changed = value;
        changed["tournament"]["name"] = json!("Changed");
        assert_eq!(
            app.clone()
                .oneshot(request(&changed, Some(TOKEN), true))
                .await
                .unwrap()
                .status(),
            StatusCode::CONFLICT
        );
    }
    assert_ne!(ids[0], ids[1]);
    let after:Value=sqlx::query_scalar("SELECT jsonb_build_object('users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),'sessions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM user_sessions s),'players',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM players p),'old',(SELECT jsonb_agg(to_jsonb(t) ORDER BY player_id) FROM tournament_players t WHERE tournament_id=$1))").bind(OLD).fetch_one(&pool).await.unwrap();
    assert_eq!(identity, after);
    for id in ids {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM tournament_invitations WHERE tournament_id=$1"
            )
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn strict_auth_csrf_validation_and_late_failure_leave_no_orphans(pool: PgPool) {
    seed(&pool).await;
    session(&pool, USER, TOKEN).await;
    let app = api::router(AppState::new(pool.clone()));
    let before = counts(&pool).await;
    let input = payload(Uuid::new_v4());
    assert_eq!(
        app.clone()
            .oneshot(request(&json!({}), None, false))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.clone()
            .oneshot(request(&input, Some(TOKEN), false))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    for field in ["user_id", "player_id", "role", "status"] {
        let mut value = input.clone();
        value[field] = json!("forged");
        assert_eq!(
            app.clone()
                .oneshot(request(&value, Some(TOKEN), true))
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let mut invalid = input.clone();
    invalid["rounds"][0]["round_number"] = json!(1);
    assert_eq!(
        app.clone()
            .oneshot(request(&invalid, Some(TOKEN), true))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(before, counts(&pool).await);
    sqlx::raw_sql("CREATE FUNCTION reject_creation_receipt() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test rollback'; END $$; CREATE TRIGGER reject_creation_receipt BEFORE INSERT ON tournament_creation_requests FOR EACH ROW EXECUTE FUNCTION reject_creation_receipt()").execute(&pool).await.unwrap();
    assert_eq!(
        app.oneshot(request(&input, Some(TOKEN), true))
            .await
            .unwrap()
            .status(),
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(before, counts(&pool).await);
}
#[sqlx::test(migrations = "../migrations")]
async fn concurrent_separate_sessions_retry_once_and_other_accounts_have_independent_keys(
    pool: PgPool,
) {
    seed(&pool).await;
    let a = session(&pool, USER, TOKEN).await;
    let b = session(&pool, USER, "second-session").await;
    let c = session(&pool, ADMIN, "admin-session").await;
    let input = plan();
    let key = Uuid::new_v4();
    let (one, two) = tokio::join!(
        tournament_creation::create(&pool, a, key, &input),
        tournament_creation::create(&pool, b, key, &input)
    );
    let (one, two) = (one.unwrap(), two.unwrap());
    assert_eq!(one.tournament_id, two.tournament_id);
    assert_ne!(one.created, two.created);
    let other = tournament_creation::create(&pool, c, key, &input)
        .await
        .unwrap();
    assert_ne!(one.tournament_id, other.tournament_id);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tournament_players WHERE tournament_id=$1"
        )
        .bind(other.tournament_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    sqlx::query("UPDATE players SET active=false WHERE id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    let inactive = tournament_creation::create(&pool, a, Uuid::new_v4(), &input)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tournament_players WHERE tournament_id=$1"
        )
        .bind(inactive.tournament_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn expired_after_player_wait_and_revoked_sessions_cannot_create_or_retry(pool: PgPool) {
    seed(&pool).await;
    let id = session(&pool, USER, TOKEN).await;
    let input = plan();
    let key = Uuid::new_v4();
    tournament_creation::create(&pool, id, key, &input)
        .await
        .unwrap();
    auth::revoke_session(&pool, id).await.unwrap();
    assert!(
        tournament_creation::create(&pool, id, key, &input)
            .await
            .is_err()
    );
    let short = session(&pool, USER, "short-session").await;
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '1 second' WHERE id=$1",
    )
    .bind(short)
    .execute(&pool)
    .await
    .unwrap();
    let before = counts(&pool).await;
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM players WHERE id=$1 FOR UPDATE")
        .bind(USER)
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    let cloned = pool.clone();
    let pending = tokio::spawn(async move {
        tournament_creation::create(&cloned, short, Uuid::new_v4(), &plan()).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    lock.commit().await.unwrap();
    assert!(pending.await.unwrap().is_err());
    assert_eq!(before, counts(&pool).await);
}
#[sqlx::test(migrations = false)]
async fn schema21_upgrade_preserves_seed_and_adds_empty_retry_registry(pool: PgPool) {
    for m in golf_api::schema::MIGRATOR.iter().filter(|m| m.version < 22) {
        sqlx::raw_sql(&m.sql).execute(&pool).await.unwrap();
    }
    seed(&pool).await;
    let before: Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tournaments t")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::raw_sql(include_str!(
        "../../migrations/0022_authenticated_tournament_creation.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let after: Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tournaments t")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, after);
    assert_eq!(counts(&pool).await.4, 0);
    seed(&pool).await;
}
