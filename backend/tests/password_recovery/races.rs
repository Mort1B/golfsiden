use super::support::*;
use golf_api::{
    domain::password_recovery::RecoveryToken,
    repositories::{
        auth,
        password_recovery::{self, RecoveryError},
    },
};
use sqlx::PgPool;
use std::time::Duration;

#[sqlx::test(migrations = "../migrations")]
async fn parallel_redemption_has_one_winner_and_stale_login_cannot_create_session(pool: PgPool) {
    let r = fixture(&pool).await;
    let verified = auth::find_login_user(&pool, "anders")
        .await
        .unwrap()
        .unwrap();
    let (grant, token) = issue(&pool, &r).await;
    let hash = golf_api::auth::hash_password(NEW_PASSWORD.as_bytes()).unwrap();
    let (a, b) = tokio::join!(
        password_recovery::redeem(&pool, grant.id, &token, &hash),
        password_recovery::redeem(&pool, grant.id, &token, &hash)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(matches!(a, Err(RecoveryError::Invalid)) || matches!(b, Err(RecoveryError::Invalid)));
    assert!(
        auth::create_verified_session(
            &pool,
            USER,
            &golf_api::auth::hash_session_token("stale-login"),
            chrono::Utc::now() + chrono::Duration::hours(1),
            (
                &verified.password_hash.unwrap(),
                "anders",
                verified.credential_generation
            )
        )
        .await
        .unwrap()
        .is_none()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn queued_redemption_rechecks_password_change_and_session_expiry_after_wait(pool: PgPool) {
    let r = fixture(&pool).await;
    let (grant, token) = issue(&pool, &r).await;
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(USER)
        .execute(&mut *lock)
        .await
        .unwrap();
    let future = password_recovery::redeem(&pool, grant.id, &token, "new-password-hash");
    tokio::pin!(future);
    assert!(
        tokio::time::timeout(Duration::from_millis(80), &mut future)
            .await
            .is_err()
    );
    sqlx::query("UPDATE users SET password_hash='intervening-password-hash' WHERE id=$1")
        .bind(USER)
        .execute(&mut *lock)
        .await
        .unwrap();
    lock.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), future)
            .await
            .unwrap(),
        Err(RecoveryError::Invalid)
    ));
    // Session lock is held before account waits, so expiry must use clock_timestamp.
    sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '200 milliseconds' WHERE id=$1").bind(r.session_id).execute(&pool).await.unwrap();
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(USER)
        .execute(&mut *lock)
        .await
        .unwrap();
    let token = RecoveryToken::generate().unwrap();
    let token_hash = token.hash();
    let future = password_recovery::admin_issue(&pool, &r, &token_hash);
    tokio::pin!(future);
    assert!(
        tokio::time::timeout(Duration::from_millis(300), &mut future)
            .await
            .is_err()
    );
    lock.commit().await.unwrap();
    assert!(matches!(future.await, Err(RecoveryError::Unauthenticated)));
}
#[sqlx::test(migrations = "../migrations")]
async fn membership_updates_and_new_admin_insert_serialize_with_account_locks(pool: PgPool) {
    let r = fixture(&pool).await;
    let (grant, token) = issue(&pool, &r).await;
    let mut membership = pool.begin().await.unwrap();
    sqlx::query("UPDATE tournament_memberships SET role='admin' WHERE user_id=$1")
        .bind(USER)
        .execute(&mut *membership)
        .await
        .unwrap();
    let future = password_recovery::preview(&pool, grant.id, &token);
    tokio::pin!(future);
    assert!(
        tokio::time::timeout(Duration::from_millis(80), &mut future)
            .await
            .is_err()
    );
    membership.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), future)
            .await
            .unwrap(),
        Err(RecoveryError::Invalid)
    ));
    sqlx::query("UPDATE tournament_memberships SET role='player' WHERE user_id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    let (grant, token) = issue(&pool, &r).await;
    let trip:uuid::Uuid=sqlx::query_scalar("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES(gen_random_uuid(),'Other','2026-09-10','2026-09-10',1,1) RETURNING id").fetch_one(&pool).await.unwrap();
    let mut target = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(USER)
        .execute(&mut *target)
        .await
        .unwrap();
    let insert = sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(trip)
    .bind(USER)
    .execute(&pool);
    tokio::pin!(insert);
    assert!(
        tokio::time::timeout(Duration::from_millis(80), &mut insert)
            .await
            .is_err()
    );
    target.commit().await.unwrap();
    insert.await.unwrap();
    assert!(matches!(
        password_recovery::preview(&pool, grant.id, &token).await,
        Err(RecoveryError::Invalid)
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn committed_revoke_wins_over_waiting_redemption(pool: PgPool) {
    let r = fixture(&pool).await;
    let (grant, token) = issue(&pool, &r).await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(USER)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE password_recovery_grants SET outcome='revoked',ended_at=clock_timestamp() WHERE id=$1").bind(grant.id).execute(&mut *tx).await.unwrap();
    let future = password_recovery::redeem(&pool, grant.id, &token, "new-hash");
    tokio::pin!(future);
    assert!(
        tokio::time::timeout(Duration::from_millis(80), &mut future)
            .await
            .is_err()
    );
    tx.commit().await.unwrap();
    assert!(matches!(future.await, Err(RecoveryError::Invalid)));
}
