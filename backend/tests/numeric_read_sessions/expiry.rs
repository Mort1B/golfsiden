use super::support::*;
use axum::http::{StatusCode, header};
use sqlx::PgPool;
use tokio::time::{Duration, sleep, timeout};

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_membership_wait_denies_all_three_numeric_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let mut events = state.live_events.subscribe();
    let mut observed = Vec::new();
    for path in paths() {
        let (session, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '8 seconds' WHERE id=$1").bind(session).execute(&pool).await.unwrap();
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        wait_blocked(&pool, pid).await;
        let active: bool = sqlx::query_scalar(
            "SELECT expires_at>clock_timestamp() FROM user_sessions WHERE id=$1",
        )
        .bind(session)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(active);
        timeout(Duration::from_secs(15), async {
            loop {
                let expired: bool = sqlx::query_scalar(
                    "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1",
                )
                .bind(session)
                .fetch_one(&pool)
                .await
                .unwrap();
                if expired {
                    break;
                }
                sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        let status = response.status();
        let cache = response.headers().get(header::CACHE_CONTROL).cloned();
        let value = body(response).await;
        eprintln!("{path}: status={status}, code={}", value["error"]["code"]);
        observed.push((path, status, cache, value));
    }
    assert!(events.try_recv().is_err());
    for (path, status, cache, value) in observed {
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{path}");
        assert!(cache.unwrap().to_str().unwrap().contains("no-store"));
        assert_eq!(value["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_final_card_or_owner_loading_denies_all_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    for path in paths() {
        let (session, token) = session(&pool).await;
        let baseline = get(&app, &path, &token).await;
        assert_eq!(baseline.status(), StatusCode::OK);
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_loading(&mut gate, &path).await;
        sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '8 seconds' WHERE id=$1").bind(session).execute(&pool).await.unwrap();
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        expire_at_wait(&pool, session, pid).await;
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED, "{path}");
        assert_private(&r);
        assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    }
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn viewer_unlinked_and_singles_empty_access_recheck_expiry(pool: PgPool) {
    use golf_api::domain::models::TournamentRole;
    let state = fixture(&pool).await;
    let app = app(&state);
    let mut events = state.live_events.subscribe();
    for (role, round) in [
        (TournamentRole::Viewer, ROUND),
        (TournamentRole::Player, ROUND),
        (TournamentRole::Admin, DRAFT),
    ] {
        sqlx::query(
            "UPDATE tournament_memberships SET role=$1 WHERE tournament_id=$2 AND user_id=$3",
        )
        .bind(role)
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        if round == DRAFT {
            sqlx::query("UPDATE rounds SET scoring_format='singles_match_play' WHERE id=$1")
                .bind(round)
                .execute(&pool)
                .await
                .unwrap();
        }
        let path = format!("/api/rounds/{round}/score-access");
        let (session, token) = session(&pool).await;
        let r = get(&app, &path, &token).await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(body(r).await["writable_owners"], serde_json::json!([]));
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '8 seconds' WHERE id=$1").bind(session).execute(&pool).await.unwrap();
        let router = app.clone();
        let read = tokio::spawn(async move { get(&router, &path, &token).await });
        expire_at_wait(&pool, session, pid).await;
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_private(&r);
        assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    }
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_four_ball_access_final_mapping_denies_success(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
    sqlx::query("UPDATE rounds SET scoring_format='four_ball_stroke_play' WHERE id=$1")
        .bind(DRAFT)
        .execute(&pool)
        .await
        .unwrap();
    golf_api::repositories::round_lifecycle::open_authorized(&pool, session, DRAFT)
        .await
        .unwrap();
    let path = format!("/api/rounds/{DRAFT}/score-access");
    let r = get(&app, &path, &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        body(r).await["writable_owners"].as_array().unwrap().len(),
        4
    );
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut gate = pool.begin().await.unwrap();
    let pid = blocker(&mut gate).await;
    lock_loading(&mut gate, &path).await;
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '8 seconds' WHERE id=$1",
    )
    .bind(session)
    .execute(&pool)
    .await
    .unwrap();
    let router = app.clone();
    let read = tokio::spawn(async move { get(&router, &path, &token).await });
    expire_at_wait(&pool, session, pid).await;
    gate.commit().await.unwrap();
    let r = timeout(Duration::from_secs(10), read)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_private(&r);
    assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}
