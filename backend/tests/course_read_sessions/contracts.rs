use super::support::*;
use axum::http::{StatusCode, header};
use golf_api::{course_catalog, repositories::course_presets};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

async fn snapshot(pool: &PgPool) -> String {
    sqlx::query_scalar("SELECT md5(jsonb_build_object('users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),'sessions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM user_sessions s),'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY tournament_id,user_id) FROM tournament_memberships m),'courses',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM courses c),'tees',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tees t),'holes',(SELECT jsonb_agg(to_jsonb(h) ORDER BY id) FROM holes h),'presets',(SELECT jsonb_agg(to_jsonb(p) ORDER BY display_order) FROM course_presets p),'rounds',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r))::text)").fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn exact_payloads_roles_query_errors_and_private_extractor_responses(pool: PgPool) {
    let (state, app) = fixture(&pool).await;
    let (_, token) = session(&pool).await;
    let mut events = state.live_events.subscribe();
    let expected = serde_json::to_value(
        course_presets::list_for_admin(&pool, USER, TRIP)
            .await
            .unwrap(),
    )
    .unwrap();
    let r = get(&app, &paths()[1], &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(body(r).await, expected);
    let r = get(&app, &paths()[0], &token).await;
    assert_eq!(
        body(r).await,
        json!({"courses":course_catalog::search(None).unwrap()})
    );
    let r = get(&app, &format!("{}?q=zzzz-unmatched", paths()[0]), &token).await;
    assert_eq!(body(r).await, json!({"courses":[]}));
    let before = snapshot(&pool).await;
    let mut reads = Vec::new();
    for path in paths().into_iter().cycle().take(12) {
        let expected = body(get(&app, &path, &token).await).await;
        let router = app.clone();
        let token = token.clone();
        reads.push(tokio::spawn(async move {
            let r = get(&router, &path, &token).await;
            assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
            assert_eq!(body(r).await, expected);
        }));
    }
    for read in reads {
        timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(snapshot(&pool).await, before);
    let foreign = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Foreign synthetic trip','2026-10-01','2026-10-01',1)").bind(foreign).execute(&pool).await.unwrap();
    for path in paths() {
        let r = get(&app, &path, "unknown").await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        let r = get(
            &app,
            &path.replace(&TRIP.to_string(), &foreign.to_string()),
            &token,
        )
        .await;
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_eq!(body(r).await["error"]["code"], "forbidden");
        for role in ["viewer", "scorer", "player"] {
            sqlx::query("UPDATE tournament_memberships SET role=$1::tournament_role WHERE tournament_id=$2 AND user_id=$3").bind(role).bind(TRIP).bind(USER).execute(&pool).await.unwrap();
            let r = get(&app, &path, &token).await;
            assert_eq!(r.status(), StatusCode::FORBIDDEN);
            assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        }
        sqlx::query(
            "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        let missing = path.replace(&TRIP.to_string(), &Uuid::new_v4().to_string());
        let r = get(&app, &missing, &token).await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        let invalid = path.replace(&TRIP.to_string(), "invalid");
        let r = get(&app, &invalid, &token).await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
    }
    for suffix in ["?q=x", "?other=field", "?q=two%0Alines"] {
        let r = get(&app, &format!("{}{suffix}", paths()[0]), &token).await;
        assert_eq!(r.status(), StatusCode::BAD_REQUEST);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
    }
    // The row/query facts retain the authority snapshot through a final load wait.
    let path = paths()[1].clone();
    let before = body(get(&app, &path, &token).await).await;
    let mut gate = pool.begin().await.unwrap();
    let pid = blocker(&mut gate).await;
    sqlx::query("LOCK TABLE courses IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let router = app.clone();
    let pending = path.clone();
    let read_token = token.clone();
    let read = tokio::spawn(async move { get(&router, &pending, &read_token).await });
    wait_blocked(&pool, pid).await;
    sqlx::query("UPDATE course_presets SET display_order=4 WHERE display_order=1")
        .execute(&mut *gate)
        .await
        .unwrap();
    gate.commit().await.unwrap();
    assert_eq!(
        body(
            timeout(Duration::from_secs(10), read)
                .await
                .unwrap()
                .unwrap()
        )
        .await,
        before
    );
    assert_ne!(body(get(&app, &path, &token).await).await, before);
    sqlx::query("DELETE FROM course_presets")
        .execute(&pool)
        .await
        .unwrap();
    let r = get(&app, &path, &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(body(r).await, json!([]));
    assert!(events.try_recv().is_err());
}
