use super::support::*;
use axum::http::{StatusCode, header};
use golf_api::{
    domain::{leaderboards::LeaderboardMetric, scorecards::ScoreOwner},
    repositories::{auth, leaderboards, round_lifecycle, scorecards},
};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use uuid::uuid;

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_valid_reads_preserve_response_parity_without_writes(pool: PgPool) {
    let f = fixture(&pool).await;
    let (_, token) = session(&pool).await;
    let mut events = f.state.live_events.subscribe();
    let mut tasks = Vec::new();
    for (index, path) in paths().into_iter().enumerate() {
        let expected = if index < 2 {
            serde_json::to_value(
                leaderboards::round(
                    &pool,
                    ROUND,
                    if index % 2 == 0 {
                        LeaderboardMetric::Gross
                    } else {
                        LeaderboardMetric::Net
                    },
                )
                .await
                .unwrap(),
            )
            .unwrap()
        } else {
            serde_json::to_value(
                leaderboards::tournament(
                    &pool,
                    TRIP,
                    if index % 2 == 0 {
                        LeaderboardMetric::Gross
                    } else {
                        LeaderboardMetric::Net
                    },
                )
                .await
                .unwrap(),
            )
            .unwrap()
        };
        for _ in 0..3 {
            let app = f.app.clone();
            let path = path.clone();
            let token = token.clone();
            let expected = expected.clone();
            tasks.push(tokio::spawn(async move {
                let response = get(&app, &path, &token).await;
                assert_eq!(response.status(), StatusCode::OK, "{path}");
                assert_eq!(
                    response.headers()[header::CACHE_CONTROL],
                    "private, no-store"
                );
                assert_eq!(body(response).await, expected, "{path}");
            }));
        }
    }
    for task in tasks {
        timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
    }
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn cancelled_wait_releases_authority_and_leaves_pool_reusable(pool: PgPool) {
    let f = fixture(&pool).await;
    let (session, token) = session(&pool).await;
    let mut gate = pool.begin().await.unwrap();
    let pid = blocker(&mut gate).await;
    lock_membership(&mut gate).await;
    let app = f.app.clone();
    let pending_token = token.clone();
    let task = tokio::spawn(async move { get(&app, &paths()[0], &pending_token).await });
    wait_blocked(&pool, pid).await;
    task.abort();
    gate.rollback().await.unwrap();
    assert!(task.await.unwrap_err().is_cancelled());
    timeout(Duration::from_secs(5), auth::revoke_session(&pool, session))
        .await
        .unwrap()
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert_eq!(
        get(&f.app, &paths()[0], &token).await.status(),
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn retry_after_membership_demotion_recomputes_hidden_final_projection(pool: PgPool) {
    let f = fixture(&pool).await;
    let (session, token) = session(&pool).await;
    let final_round = uuid!("00000000-0000-0000-0000-000000004005");
    let player = uuid!("00000000-0000-0000-0000-000000001001");
    round_lifecycle::open_authorized(&pool, session, final_round)
        .await
        .unwrap();
    for hole in [
        uuid!("00000000-0000-0000-0000-000000003201"),
        uuid!("00000000-0000-0000-0000-000000003218"),
    ] {
        scorecards::save_authenticated(
            &pool,
            scorecards::AuthenticatedSaveScore {
                session_id: session,
                round_id: final_round,
                owner: ScoreOwner::Player { id: player },
                hole_id: hole,
                gross_strokes: 4,
            },
        )
        .await
        .unwrap();
    }
    for path in paths()
        .into_iter()
        .map(|p| p.replace(&ROUND.to_string(), &final_round.to_string()))
    {
        sqlx::query(
            "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        let full = body(get(&f.app, &path, &token).await).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let app = f.app.clone();
        let pending_path = path.clone();
        let pending_token = token.clone();
        let task = tokio::spawn(async move { get(&app, &pending_path, &pending_token).await });
        wait_blocked(&pool, pid).await;
        sqlx::query(
            "UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&mut *gate)
        .await
        .unwrap();
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{path}");
        let after = body(response).await;
        let fresh = body(get(&f.app, &path, &token).await).await;
        assert_eq!(
            after, fresh,
            "must recompute visibility from the fresh role: {path}"
        );
        assert_ne!(
            after, full,
            "hidden back-nine facts must not survive retry: {path}"
        );
        if path.contains("/rounds/") {
            assert!(
                after["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|entry| entry["holes_scored"].as_i64().unwrap() <= 1)
            );
        }
    }
}
