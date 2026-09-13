use super::support::*;
use golf_api::{
    domain::password_recovery::RecoveryToken,
    repositories::{
        auth,
        password_recovery::{self, RecoveryError},
        profile::{self, CredentialChange},
    },
};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;

// Observe PostgreSQL's actual waiter before starting a competing request; an
// elapsed timeout alone could mean connection acquisition or scheduler delay.
async fn wait_for_lock(pool: &PgPool, query_fragment: &str) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND pid<>pg_backend_pid() AND wait_event_type='Lock' AND strpos(query,$1)>0)",
            )
            .bind(query_fragment)
            .fetch_one(pool)
            .await
            .unwrap();
            if waiting { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("first contender must reach the database lock wait");
}

#[sqlx::test(migrations = "../migrations")]
async fn grant_expiring_during_account_wait_cannot_change_password(pool: PgPool) {
    fixture(&pool).await;
    let before = profile::credential(&pool, USER).await.unwrap().unwrap();
    let id = Uuid::new_v4();
    let token = RecoveryToken::generate().unwrap();
    sqlx::query("INSERT INTO password_recovery_grants(id,token_hash,target_user_id,credential_generation,authority_version,issuer_kind,reason,issued_at,expires_at) SELECT $1,$2,id,credential_generation,0,'operator','Expiry during lock wait',clock_timestamp()-interval '1 minute',clock_timestamp()+interval '1 second' FROM users WHERE id=$3")
        .bind(id).bind(token.hash().as_bytes()).bind(USER).execute(&pool).await.unwrap();
    password_recovery::preview(&pool, id, &token).await.unwrap();
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(USER)
        .execute(&mut *lock)
        .await
        .unwrap();
    let redeem = password_recovery::redeem(&pool, id, &token, "must-not-be-written");
    tokio::pin!(redeem);
    assert!(
        tokio::time::timeout(Duration::from_millis(1100), &mut redeem)
            .await
            .is_err()
    );
    lock.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), redeem)
            .await
            .unwrap(),
        Err(RecoveryError::Invalid)
    ));
    assert_eq!(
        profile::credential(&pool, USER).await.unwrap().unwrap(),
        before
    );
    let outcome: Option<String> =
        sqlx::query_scalar("SELECT outcome FROM password_recovery_grants WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(outcome, None);
}

#[sqlx::test(migrations = "../migrations")]
async fn queued_profile_password_change_wins_without_deadlocking_redemption(pool: PgPool) {
    let r = fixture(&pool).await;
    let (grant, token) = issue(&pool, &r).await;
    let session = auth::create_session(
        &pool,
        USER,
        &golf_api::auth::hash_session_token("profile-race"),
        chrono::Utc::now() + chrono::Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id;
    let verified = profile::credential(&pool, USER).await.unwrap().unwrap();
    let version = profile::get(&pool, session).await.unwrap().version;
    let hash = golf_api::auth::hash_password(NEW_PASSWORD.as_bytes()).unwrap();
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(USER)
        .execute(&mut *lock)
        .await
        .unwrap();
    let change = profile::update_credential(
        &pool,
        session,
        version,
        &verified,
        CredentialChange::Password(hash.clone()),
    );
    tokio::pin!(change);
    tokio::select! {
        _ = &mut change => panic!("contender completed while its account was locked"),
        () = wait_for_lock(&pool, "FOR UPDATE OF s, u") => {}
    }
    let redeem = password_recovery::redeem(&pool, grant.id, &token, "must-not-be-written");
    tokio::pin!(redeem);
    assert!(
        tokio::time::timeout(Duration::from_millis(80), &mut redeem)
            .await
            .is_err()
    );
    lock.commit().await.unwrap();
    let (changed, redeemed) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(3), change),
        tokio::time::timeout(Duration::from_secs(3), redeem)
    );
    changed.unwrap().unwrap();
    assert!(matches!(redeemed.unwrap(), Err(RecoveryError::Invalid)));
    assert_eq!(
        profile::credential(&pool, USER).await.unwrap().unwrap(),
        (hash, verified.1 + 1)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn queued_unlink_and_relink_cannot_revive_preexisting_grant(pool: PgPool) {
    let r = fixture(&pool).await;
    let (grant, token) = issue(&pool, &r).await;
    let mut relink = pool.begin().await.unwrap();
    sqlx::query("UPDATE users SET player_id=NULL WHERE id=$1")
        .bind(USER)
        .execute(&mut *relink)
        .await
        .unwrap();
    sqlx::query("UPDATE users SET player_id=id WHERE id=$1")
        .bind(USER)
        .execute(&mut *relink)
        .await
        .unwrap();
    let redeem = password_recovery::redeem(&pool, grant.id, &token, "must-not-be-written");
    tokio::pin!(redeem);
    assert!(
        tokio::time::timeout(Duration::from_millis(80), &mut redeem)
            .await
            .is_err()
    );
    relink.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), redeem)
            .await
            .unwrap(),
        Err(RecoveryError::Invalid)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn moving_admin_membership_into_target_serializes_and_invalidates_waiting_grant(
    pool: PgPool,
) {
    let r = fixture(&pool).await;
    let trip: Uuid = sqlx::query_scalar("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES(gen_random_uuid(),'Membership move','2026-09-10','2026-09-10',1,1) RETURNING id")
        .fetch_one(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(trip)
    .bind(OTHER)
    .execute(&pool)
    .await
    .unwrap();
    let (grant, token) = issue(&pool, &r).await;
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(USER)
        .execute(&mut *lock)
        .await
        .unwrap();
    let moving = sqlx::query(
        "UPDATE tournament_memberships SET user_id=$1 WHERE tournament_id=$2 AND user_id=$3",
    )
    .bind(USER)
    .bind(trip)
    .bind(OTHER)
    .execute(&pool);
    tokio::pin!(moving);
    tokio::select! {
        _ = &mut moving => panic!("contender completed while its account was locked"),
        () = wait_for_lock(&pool, "UPDATE tournament_memberships SET user_id=") => {}
    }
    let preview = password_recovery::preview(&pool, grant.id, &token);
    tokio::pin!(preview);
    assert!(
        tokio::time::timeout(Duration::from_millis(80), &mut preview)
            .await
            .is_err()
    );
    lock.commit().await.unwrap();
    let (moved, checked) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(3), moving),
        tokio::time::timeout(Duration::from_secs(3), preview)
    );
    assert_eq!(moved.unwrap().unwrap().rows_affected(), 1);
    assert!(matches!(checked.unwrap(), Err(RecoveryError::Invalid)));
}
