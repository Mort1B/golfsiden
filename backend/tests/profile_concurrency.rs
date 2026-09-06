#![cfg(feature = "database-tests")]
use chrono::{Duration, Utc};
use golf_api::{
    auth::hash_session_token,
    repositories::{
        auth,
        profile::{self, CredentialChange, DetailsChange, ProfileError},
    },
};
use sqlx::PgPool;
use uuid::Uuid;

async fn user(pool: &PgPool) -> (Uuid, Uuid) {
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,display_name,role,password_hash) VALUES($1,'profile-race','Race','player','original-hash')").bind(user).execute(pool).await.unwrap();
    let s = auth::create_session(
        pool,
        user,
        &hash_session_token("race-session"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    (user, s.session_id)
}
async fn wait_for_lock(pool: &PgPool) {
    tokio::time::timeout(std::time::Duration::from_secs(5),async {
        loop {
            let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND pid<>pg_backend_pid())").fetch_one(pool).await.unwrap();
            if blocked { break; } tokio::task::yield_now().await;
        }
    }).await.unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn verified_login_waiting_for_password_change_cannot_gain_new_generation(pool: PgPool) {
    let (user, sid) = user(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("UPDATE users SET password_hash='replacement-hash' WHERE id=$1")
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
    let task_pool = pool.clone();
    let login = tokio::spawn(async move {
        auth::create_verified_session(
            &task_pool,
            user,
            &hash_session_token("old-password-login"),
            Utc::now() + Duration::hours(1),
            ("original-hash", "profile-race", 0),
        )
        .await
    });
    wait_for_lock(&pool).await;
    tx.commit().await.unwrap();
    assert!(login.await.unwrap().unwrap().is_none());
    assert!(profile::get(&pool, sid).await.is_err());
    // A login that wins the lock is valid only until a later password change.
    let new = auth::create_verified_session(
        &pool,
        user,
        &hash_session_token("new-password-login"),
        Utc::now() + Duration::hours(1),
        ("replacement-hash", "profile-race", 1),
    )
    .await
    .unwrap()
    .unwrap();
    let p = profile::get(&pool, new.session_id).await.unwrap();
    profile::update_credential(
        &pool,
        new.session_id,
        p.version,
        &("replacement-hash".into(), 1),
        CredentialChange::Password("third-hash".into()),
    )
    .await
    .unwrap();
    assert!(
        auth::find_active_session(&pool, &hash_session_token("new-password-login"))
            .await
            .unwrap()
            .is_none()
    );
    // Returning to an earlier hash does not resurrect its old verification.
    sqlx::query("UPDATE users SET password_hash='original-hash' WHERE id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        auth::create_verified_session(
            &pool,
            user,
            &hash_session_token("aba-login"),
            Utc::now() + Duration::hours(1),
            ("original-hash", "profile-race", 0)
        )
        .await
        .unwrap()
        .is_none()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn edits_waiting_on_user_recheck_version_and_session_wall_clock(pool: PgPool) {
    let (user, sid) = user(&pool).await;
    let old = profile::get(&pool, sid).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("UPDATE users SET display_name='Other device' WHERE id=$1")
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
    let task_pool = pool.clone();
    let edit = tokio::spawn(async move {
        profile::update_details(
            &task_pool,
            sid,
            DetailsChange {
                version: old.version,
                player_updated_at: None,
                display_name: "stale".into(),
                handicap: None,
                reason: String::new(),
            },
        )
        .await
    });
    wait_for_lock(&pool).await;
    tx.commit().await.unwrap();
    assert!(matches!(edit.await.unwrap(), Err(ProfileError::Stale)));
    sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '300 milliseconds' WHERE id=$1").bind(sid).execute(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM users WHERE id=$1 FOR UPDATE")
        .bind(user)
        .execute(&mut *tx)
        .await
        .unwrap();
    let task_pool = pool.clone();
    let read = tokio::spawn(async move { profile::get(&task_pool, sid).await });
    wait_for_lock(&pool).await;
    sqlx::query("SELECT pg_sleep(0.35)")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        read.await.unwrap(),
        Err(ProfileError::Unauthenticated)
    ));
}

#[sqlx::test(migrations = false)]
async fn schema22_upgrade_preserves_sessions_until_a_real_password_change(pool: PgPool) {
    for m in golf_api::schema::MIGRATOR.iter().filter(|m| m.version < 23) {
        sqlx::raw_sql(&m.sql).execute(&pool).await.unwrap();
    }
    let user = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,display_name,role,password_hash) VALUES($1,'upgrade-profile','Upgrade','player','initial')").bind(user).execute(&pool).await.unwrap();
    for (token, expired, revoked) in [
        ("upgrade-valid", false, false),
        ("upgrade-expired", true, false),
        ("upgrade-revoked", false, true),
    ] {
        sqlx::query("INSERT INTO user_sessions(id,user_id,token_hash,expires_at,revoked_at,created_at) VALUES($1,$2,$3,now()+CASE WHEN $4 THEN interval '-1 hour' ELSE interval '1 hour' END,CASE WHEN $5 THEN now() ELSE NULL END,now()-interval '2 hours')")
            .bind(Uuid::new_v4()).bind(user).bind(hash_session_token(token)).bind(expired).bind(revoked).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(include_str!(
        "../../migrations/0023_self_service_profile.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        auth::find_active_session(&pool, &hash_session_token("upgrade-valid"))
            .await
            .unwrap()
            .is_some()
    );
    for token in ["upgrade-expired", "upgrade-revoked"] {
        assert!(
            auth::find_active_session(&pool, &hash_session_token(token))
                .await
                .unwrap()
                .is_none()
        );
    }
    sqlx::query("UPDATE users SET display_name='Renamed' WHERE id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    let versions: (i64, i64) =
        sqlx::query_as("SELECT profile_version,credential_generation FROM users WHERE id=$1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(versions, (1, 0));
    sqlx::query("UPDATE users SET password_hash='changed' WHERE id=$1")
        .bind(user)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        auth::find_active_session(&pool, &hash_session_token("upgrade-valid"))
            .await
            .unwrap()
            .is_none()
    );
    let versions: (i64, i64) =
        sqlx::query_as("SELECT profile_version,credential_generation FROM users WHERE id=$1")
            .bind(user)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(versions, (2, 1));
}
