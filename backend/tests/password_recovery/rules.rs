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
use uuid::Uuid;

#[sqlx::test(migrations = "../migrations")]
async fn issuance_denies_wrong_tournament_self_privileged_unlinked_inactive_and_missing_membership(
    pool: PgPool,
) {
    let mut r = fixture(&pool).await;
    let token = RecoveryToken::generate().unwrap();
    r.tournament_id = Uuid::new_v4();
    assert!(matches!(
        password_recovery::admin_issue(&pool, &r, &token.hash()).await,
        Err(RecoveryError::Forbidden)
    ));
    r.tournament_id = TRIP;
    for (change, restore) in [
        (
            "UPDATE users SET role='admin' WHERE id=$1",
            "UPDATE users SET role='player' WHERE id=$1",
        ),
        (
            "UPDATE tournament_memberships SET role='admin' WHERE user_id=$1",
            "UPDATE tournament_memberships SET role='player' WHERE user_id=$1",
        ),
        (
            "UPDATE users SET player_id=NULL WHERE id=$1",
            "UPDATE users SET player_id=id WHERE id=$1",
        ),
        (
            "UPDATE players SET active=false WHERE id=$1",
            "UPDATE players SET active=true WHERE id=$1",
        ),
        (
            "UPDATE tournament_players SET status='withdrawn' WHERE player_id=$1",
            "UPDATE tournament_players SET status='active' WHERE player_id=$1",
        ),
        (
            "DELETE FROM tournament_memberships WHERE user_id=$1",
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) SELECT tournament_id,$1,'player' FROM tournament_players WHERE player_id=$1",
        ),
    ] {
        sqlx::query(change).bind(USER).execute(&pool).await.unwrap();
        assert!(
            matches!(
                password_recovery::admin_issue(&pool, &r, &token.hash()).await,
                Err(RecoveryError::Forbidden)
            ),
            "{change}"
        );
        sqlx::query(restore)
            .bind(USER)
            .execute(&pool)
            .await
            .unwrap();
    }
    let other_trip:Uuid=sqlx::query_scalar("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES(gen_random_uuid(),'Other','2026-09-10','2026-09-10',1,1) RETURNING id").fetch_one(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(other_trip)
    .bind(USER)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        password_recovery::admin_issue(&pool, &r, &token.hash()).await,
        Err(RecoveryError::Forbidden)
    ));
    // Self even when the user is a player and exact tournament administrator.
    r.user_id = USER;
    r.player_id = USER;
    r.session_id = auth::create_session(
        &pool,
        USER,
        &golf_api::auth::hash_session_token("self-session"),
        chrono::Utc::now() + chrono::Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id;
    r.verified_password = profile::credential(&pool, USER).await.unwrap().unwrap();
    assert!(matches!(
        password_recovery::admin_issue(&pool, &r, &token.hash()).await,
        Err(RecoveryError::Forbidden)
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn authority_changes_away_and_back_never_revive_grants(pool: PgPool) {
    let r = fixture(&pool).await;
    for (subject, change, restore) in [
        (
            ADMIN,
            "UPDATE tournament_memberships SET role='viewer' WHERE user_id=$1",
            "UPDATE tournament_memberships SET role='admin' WHERE user_id=$1",
        ),
        (
            ADMIN,
            "DELETE FROM tournament_memberships WHERE user_id=$1",
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) SELECT id,$1,'admin' FROM tournaments",
        ),
        (
            USER,
            "UPDATE users SET role='admin' WHERE id=$1",
            "UPDATE users SET role='player' WHERE id=$1",
        ),
        (
            USER,
            "UPDATE tournament_memberships SET role='admin' WHERE user_id=$1",
            "UPDATE tournament_memberships SET role='player' WHERE user_id=$1",
        ),
        (
            USER,
            "UPDATE users SET player_id=NULL WHERE id=$1",
            "UPDATE users SET player_id=id WHERE id=$1",
        ),
        (
            USER,
            "UPDATE players SET active=false WHERE id=$1",
            "UPDATE players SET active=true WHERE id=$1",
        ),
        (
            USER,
            "UPDATE tournament_players SET status='withdrawn' WHERE player_id=$1",
            "UPDATE tournament_players SET status='active' WHERE player_id=$1",
        ),
        (
            USER,
            "DELETE FROM tournament_memberships WHERE user_id=$1",
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) SELECT tournament_id,$1,'player' FROM tournament_players WHERE player_id=$1",
        ),
    ] {
        let (grant, token) = issue(&pool, &r).await;
        sqlx::query(change)
            .bind(subject)
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(restore)
            .bind(subject)
            .execute(&pool)
            .await
            .unwrap();
        assert!(
            matches!(
                password_recovery::preview(&pool, grant.id, &token).await,
                Err(RecoveryError::Invalid)
            ),
            "{change}"
        );
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn replacement_revocation_and_password_change_invalidate_but_username_change_preserves(
    pool: PgPool,
) {
    let r = fixture(&pool).await;
    let (first, a) = issue(&pool, &r).await;
    let (second, b) = issue(&pool, &r).await;
    assert!(matches!(
        password_recovery::preview(&pool, first.id, &a).await,
        Err(RecoveryError::Invalid)
    ));
    password_recovery::admin_revoke(&pool, &r).await.unwrap();
    assert!(matches!(
        password_recovery::preview(&pool, second.id, &b).await,
        Err(RecoveryError::Invalid)
    ));
    let (third, c) = issue(&pool, &r).await;
    let sid = auth::create_session(
        &pool,
        USER,
        &golf_api::auth::hash_session_token("target"),
        chrono::Utc::now() + chrono::Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id;
    let verified = profile::credential(&pool, USER).await.unwrap().unwrap();
    let p = profile::get(&pool, sid).await.unwrap();
    profile::update_credential(
        &pool,
        sid,
        p.version,
        &verified,
        CredentialChange::Username("anders_new".into()),
    )
    .await
    .unwrap();
    password_recovery::preview(&pool, third.id, &c)
        .await
        .unwrap();
    let p = profile::get(&pool, sid).await.unwrap();
    profile::update_credential(
        &pool,
        sid,
        p.version,
        &verified,
        CredentialChange::Password(golf_api::auth::hash_password(NEW_PASSWORD.as_bytes()).unwrap()),
    )
    .await
    .unwrap();
    assert!(matches!(
        password_recovery::preview(&pool, third.id, &c).await,
        Err(RecoveryError::Invalid)
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn operator_recovery_supports_admin_and_unlinked_accounts_with_explicit_provenance(
    pool: PgPool,
) {
    fixture(&pool).await;
    let token = RecoveryToken::generate().unwrap();
    let grant = password_recovery::operator_issue(
        &pool,
        ADMIN,
        "Known organizer called back",
        &token.hash(),
    )
    .await
    .unwrap();
    let kind: (String, Option<Uuid>, Option<Uuid>) = sqlx::query_as(
        "SELECT issuer_kind,issuer_user_id,tournament_id FROM password_recovery_grants WHERE id=$1",
    )
    .bind(grant.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(kind, ("operator".into(), None, None));
    password_recovery::preview(&pool, grant.id, &token)
        .await
        .unwrap();
    password_recovery::operator_revoke(&pool, ADMIN, "Organizer cancelled")
        .await
        .unwrap();
    assert!(
        password_recovery::preview(&pool, grant.id, &token)
            .await
            .is_err()
    );
    let token = RecoveryToken::generate().unwrap();
    let grant = password_recovery::operator_issue(
        &pool,
        ADMIN,
        "Known organizer called back again",
        &token.hash(),
    )
    .await
    .unwrap();
    password_recovery::redeem(
        &pool,
        grant.id,
        &token,
        &golf_api::auth::hash_password(NEW_PASSWORD.as_bytes()).unwrap(),
    )
    .await
    .unwrap();
}
