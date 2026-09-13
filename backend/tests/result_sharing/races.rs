use super::support::*;
use golf_api::{
    domain::{leaderboards::LeaderboardMetric, result_sharing::ResultShareToken},
    repositories::result_sharing::{self, ShareError},
};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

#[sqlx::test(migrations = "../migrations")]
async fn queued_read_after_committed_revoke_is_uniformly_unavailable(pool: PgPool) {
    let session = fixture(&pool).await;
    let (grant, token) = issue(&pool, session, None).await;
    let mut tx = pool.begin().await.unwrap();
    context(&mut tx, session).await;
    sqlx::query("UPDATE tournament_result_shares SET revoked_at=clock_timestamp(),revoked_by=$2 WHERE id=$1").bind(grant.id).bind(ADMIN).execute(&mut *tx).await.unwrap();
    let reader_pool = pool.clone();
    let reader = tokio::spawn(async move {
        result_sharing::read(&reader_pool, grant.id, &token, LeaderboardMetric::Gross).await
    });
    wait_locks(&pool, 1).await;
    tx.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), reader)
            .await
            .unwrap()
            .unwrap(),
        Err(ShareError::Unavailable)
    ));
}
async fn read_wins(pool: PgPool, rotate: bool) {
    let session = fixture(&pool).await;
    let (grant, token) = issue(&pool, session, None).await;
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE tournament_players IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let reader_pool = pool.clone();
    let reader = tokio::spawn(async move {
        result_sharing::read(&reader_pool, grant.id, &token, LeaderboardMetric::Gross).await
    });
    wait_locks(&pool, 1).await;
    let writer_pool = pool.clone();
    let writer = tokio::spawn(async move {
        if rotate {
            let token = ResultShareToken::generate().unwrap();
            result_sharing::issue(&writer_pool, session, TRIP, Some(grant.id), &token.hash())
                .await
                .map(|_| ())
        } else {
            result_sharing::revoke(&writer_pool, session, TRIP, grant.id)
                .await
                .map(|_| ())
        }
    });
    wait_locks(&pool, 2).await;
    gate.commit().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(3), reader)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.grant_id, grant.id);
    tokio::time::timeout(Duration::from_secs(3), writer)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let revoked: bool = sqlx::query_scalar(
        "SELECT revoked_at IS NOT NULL FROM tournament_result_shares WHERE id=$1",
    )
    .bind(grant.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(revoked);
}
#[sqlx::test(migrations = "../migrations")]
async fn authorized_read_holds_grant_through_assembly_before_revoke(pool: PgPool) {
    read_wins(pool, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn authorized_read_holds_grant_through_assembly_before_rotation(pool: PgPool) {
    read_wins(pool, true).await;
}

async fn expiry_wait(pool: PgPool, grant_lock: bool) {
    let session = fixture(&pool).await;
    let (id, token) = expiring(&pool, session, 500).await;
    let mut gate = pool.begin().await.unwrap();
    if grant_lock {
        sqlx::query("SELECT id FROM tournament_result_shares WHERE id=$1 FOR UPDATE")
            .bind(id)
            .execute(&mut *gate)
            .await
            .unwrap();
    } else {
        sqlx::query("LOCK TABLE tournament_players IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *gate)
            .await
            .unwrap();
    }
    let p = pool.clone();
    let reader = tokio::spawn(async move {
        result_sharing::read(&p, id, &token, LeaderboardMetric::Gross).await
    });
    wait_locks(&pool, 1).await;
    tokio::time::sleep(Duration::from_millis(550)).await;
    gate.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), reader)
            .await
            .unwrap()
            .unwrap(),
        Err(ShareError::Unavailable)
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_is_rechecked_after_grant_lock_wait(pool: PgPool) {
    expiry_wait(pool, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_is_rechecked_after_fact_loading_wait(pool: PgPool) {
    expiry_wait(pool, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn simultaneous_replacements_preserve_one_winner_and_reject_stale_intent(pool: PgPool) {
    let session = fixture(&pool).await;
    let (old, _) = issue(&pool, session, None).await;
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR UPDATE")
        .bind(TRIP)
        .execute(&mut *gate)
        .await
        .unwrap();
    let mut jobs = Vec::new();
    for _ in 0..2 {
        let p = pool.clone();
        jobs.push(tokio::spawn(async move {
            let token = ResultShareToken::generate().unwrap();
            result_sharing::issue(&p, session, TRIP, Some(old.id), &token.hash()).await
        }));
    }
    wait_locks(&pool, 2).await;
    gate.commit().await.unwrap();
    let mut won = 0;
    let mut stale = 0;
    for job in jobs {
        match tokio::time::timeout(Duration::from_secs(3), job)
            .await
            .unwrap()
            .unwrap()
        {
            Ok(_) => won += 1,
            Err(ShareError::Stale) => stale += 1,
            _ => panic!("unexpected replacement outcome"),
        }
    }
    assert_eq!((won, stale), (1, 1));
    let active: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM tournament_result_shares WHERE revoked_at IS NULL")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(active.len(), 1);
    assert_ne!(active[0], old.id);
}
