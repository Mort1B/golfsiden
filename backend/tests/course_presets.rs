#![cfg(feature = "database-tests")]
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::{auth, course_presets},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};
const ADMIN: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
const TRIP: Uuid = uuid!("00000000-0000-0000-0000-000000002001");
const TOKEN: &str = "preset-test-session";
async fn seed(pool: &PgPool) {
    sqlx::raw_sql(include_str!("../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
    auth::create_session(
        pool,
        ADMIN,
        &hash_session_token(TOKEN),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
fn get(id: Uuid, token: Option<&str>) -> Request<Body> {
    let mut r = Request::get(format!("/api/tournaments/{id}/course-presets"));
    if let Some(t) = token {
        r = r.header("cookie", format!("golf_session={t}"));
    }
    r.body(Body::empty()).unwrap()
}
const PARS: [[i16; 18]; 3] = [
    [4, 4, 4, 5, 3, 4, 4, 3, 5, 4, 4, 3, 4, 5, 3, 4, 4, 5],
    [5, 3, 4, 5, 4, 3, 4, 4, 5, 3, 4, 4, 5, 3, 4, 4, 4, 4],
    [4, 3, 4, 5, 4, 5, 5, 3, 4, 5, 4, 4, 4, 3, 4, 4, 3, 4],
];
const INDEXES: [[i16; 18]; 3] = [
    [
        5, 7, 11, 15, 17, 1, 9, 13, 3, 14, 4, 16, 12, 8, 18, 6, 2, 10,
    ],
    [
        8, 4, 16, 10, 18, 12, 14, 2, 6, 5, 1, 17, 9, 13, 15, 7, 11, 3,
    ],
    [
        13, 17, 1, 7, 11, 9, 15, 3, 5, 6, 10, 16, 8, 12, 2, 4, 18, 14,
    ],
];
#[sqlx::test(migrations = "../migrations")]
async fn exact_supplied_presets_are_finalized_and_preserve_all_hole_facts(pool: PgPool) {
    seed(&pool).await;
    let presets = course_presets::list_for_admin(&pool, ADMIN, TRIP)
        .await
        .unwrap();
    let values = serde_json::to_value(&presets).unwrap();
    assert_eq!(presets.len(), 3);
    for (i, (name, rating, slope)) in [
        ("Hacienda del Alamo Golf Club", 72.5, 125),
        ("Saurines Golf Course", 66.2, 116),
        ("Mar Menor Golf Course", 66.9, 118),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(values[i]["course_name"], name);
        assert_eq!(values[i]["tee"]["course_rating"], rating);
        assert_eq!(values[i]["tee"]["slope_rating"], slope);
        assert_eq!(values[i]["tee"]["category"], "male");
        assert_eq!(values[i]["tee"]["name"], "Red Tees");
        assert_eq!(presets[i].tee.holes.len(), 18);
        assert_eq!(
            presets[i]
                .tee
                .holes
                .iter()
                .map(|h| h.par)
                .collect::<Vec<_>>(),
            PARS[i]
        );
        assert_eq!(
            presets[i]
                .tee
                .holes
                .iter()
                .map(|h| h.stroke_index)
                .collect::<Vec<_>>(),
            INDEXES[i]
        );
        assert_eq!(presets[i].tee.holes.iter().map(|h| h.par).sum::<i16>(), 72);
        assert!(
            presets[i]
                .tee
                .holes
                .iter()
                .enumerate()
                .all(|(n, h)| h.number as usize == n + 1 && h.distance.is_none())
        );
        assert!(
            sqlx::query("UPDATE courses SET name='Modified' WHERE id=$1")
                .bind(presets[i].id)
                .execute(&pool)
                .await
                .is_err()
        );
        assert!(
            sqlx::query(
                "UPDATE holes SET par=3 WHERE tee_id IN (SELECT id FROM tees WHERE course_id=$1)"
            )
            .bind(presets[i].id)
            .execute(&pool)
            .await
            .is_err()
        );
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn preset_api_is_exact_admin_private_and_excludes_arbitrary_course_rows(pool: PgPool) {
    seed(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    assert_eq!(
        app.clone().oneshot(get(TRIP, None)).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        app.clone()
            .oneshot(get(Uuid::new_v4(), Some(TOKEN)))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    let r = app.clone().oneshot(get(TRIP, Some(TOKEN))).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["cache-control"], "private, no-store");
    assert_eq!(body(r).await.as_array().unwrap().len(), 3);
    for role in ["player", "scorer", "viewer"] {
        sqlx::query("UPDATE tournament_memberships SET role=$1::tournament_role WHERE tournament_id=$2 AND user_id=$3").bind(role).bind(TRIP).bind(ADMIN).execute(&pool).await.unwrap();
        let r = app.clone().oneshot(get(TRIP, Some(TOKEN))).await.unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_eq!(r.headers()["cache-control"], "private, no-store");
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn preset_save_creates_independent_revisions_and_stale_write_has_no_orphans(pool: PgPool) {
    seed(&pool).await;
    let presets = course_presets::list_for_admin(&pool, ADMIN, TRIP)
        .await
        .unwrap();
    let value = serde_json::to_value(&presets[0]).unwrap();
    let mut selection = json!({"source":"manual","course_name":value["course_name"],"location":value["location"],"tee":value["tee"]});
    for hole in selection["tee"]["holes"].as_array_mut().unwrap() {
        hole.as_object_mut().unwrap().remove("number");
    }
    let rounds: Vec<(Uuid, chrono::DateTime<Utc>)> = sqlx::query_as(
        "SELECT id,updated_at FROM rounds WHERE tournament_id=$1 ORDER BY round_number LIMIT 2",
    )
    .bind(TRIP)
    .fetch_all(&pool)
    .await
    .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    let mut saved_ids = Vec::new();
    for (id, version) in &rounds {
        let req = || {
            Request::put(format!("/api/rounds/{id}/course-configuration"))
                .header("cookie", format!("golf_session={TOKEN}"))
                .header("x-csrf-token", derive_csrf_token(TOKEN))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({"expected_round_updated_at":version,"selection":selection}).to_string(),
                ))
                .unwrap()
        };
        let r = app.clone().oneshot(req()).await.unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        let r = body(r).await;
        assert_eq!(r["course_name"], value["course_name"]);
        assert_eq!(r["number_of_holes"], 18);
        saved_ids.push(r["course_id"].as_str().unwrap().to_owned());
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM courses")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(
            app.clone().oneshot(req()).await.unwrap().status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM courses")
                .fetch_one(&pool)
                .await
                .unwrap(),
            count
        );
    }
    assert_ne!(saved_ids[0], saved_ids[1]);
    assert_ne!(saved_ids[0], presets[0].id.to_string());
    assert_eq!(
        course_presets::list_for_admin(&pool, ADMIN, TRIP)
            .await
            .unwrap()
            .len(),
        3
    );
}
#[sqlx::test(migrations = false)]
async fn schema20_upgrade_retains_existing_rounds_and_seed_stays_idempotent(pool: PgPool) {
    for migration in golf_api::schema::MIGRATOR.iter().filter(|m| m.version < 21) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    seed(&pool).await;
    let before: Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::raw_sql(include_str!(
        "../../migrations/0021_supplied_course_presets.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let after: Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(before, after);
    sqlx::raw_sql(include_str!("../seed.sql"))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        course_presets::list_for_admin(&pool, ADMIN, TRIP)
            .await
            .unwrap()
            .len(),
        3
    );
}

#[sqlx::test(migrations = false)]
async fn schema20_upgrade_preserves_opened_handicap_snapshots_and_team_ownership(pool: PgPool) {
    for migration in golf_api::schema::MIGRATOR.iter().filter(|m| m.version < 21) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    seed(&pool).await;
    let session_id: Uuid = sqlx::query_scalar("SELECT id FROM user_sessions WHERE user_id=$1")
        .bind(ADMIN)
        .fetch_one(&pool)
        .await
        .unwrap();
    let version = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap();
    golf_api::repositories::tournaments::start_authorized(&pool, session_id, TRIP, version)
        .await
        .unwrap();
    golf_api::repositories::round_lifecycle::open(
        &pool,
        uuid!("00000000-0000-0000-0000-000000004001"),
    )
    .await
    .unwrap();
    let snapshot_sql = "SELECT jsonb_build_object(
        'rounds', (SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r),
        'snapshots', (SELECT jsonb_agg(to_jsonb(s) ORDER BY round_id, player_id) FROM round_handicap_snapshots s),
        'teams', (SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM teams t),
        'members', (SELECT jsonb_agg(to_jsonb(m) ORDER BY team_id, player_id) FROM team_memberships m))";
    let before: Value = sqlx::query_scalar(snapshot_sql)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before["snapshots"].as_array().unwrap().len(), 8);
    sqlx::raw_sql(include_str!(
        "../../migrations/0021_supplied_course_presets.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let after: Value = sqlx::query_scalar(snapshot_sql)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
}
