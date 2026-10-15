use super::support::*;
use axum::http::StatusCode;
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{
    auth::hash_session_token,
    repositories::{auth, invitations},
};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

#[sqlx::test(migrations = "../migrations")]
async fn metadata_history_series_counts_order_and_secrecy_survive_concurrent_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let secret = invitation(&pool).await;
    let (admin, token) = session(&pool).await;
    let registered=write(&app,"POST",&format!("/api/invitations/{INVITE}/register"),None,json!({"token":secret,"account":{"username":"invitation_read_joined","password":"invitation-read-test-password"},"player":{"display_name":"Synthetic joined player","handicap_index":10}})).await;
    assert_eq!(registered.status(), StatusCode::CREATED);
    let rotated = write(
        &app,
        "POST",
        &format!("{}/{INVITE}/rotate", path()),
        Some(&token),
        json!({}),
    )
    .await;
    assert_eq!(rotated.status(), StatusCode::CREATED);
    let rotated = body(rotated).await;
    let successor = rotated["id"].clone();
    let created = Utc::now() - ChronoDuration::days(3);
    for (id, expired) in [(Uuid::from_u128(52), true), (Uuid::from_u128(53), false)] {
        sqlx::query("INSERT INTO tournament_invitations(id,tournament_id,series_id,created_by_user_id,token_hash,created_at,expires_at,revoked_at,revoked_by_user_id,max_uses) VALUES($1,$2,$1,$3,$4,$5,$6,$7,$8,NULL)")
        .bind(id).bind(TRIP).bind(USER).bind(golf_api::auth::hash_invitation_token(&Uuid::new_v4().to_string()).as_slice()).bind(created)
        .bind(if expired {Utc::now()-ChronoDuration::days(1)}else{Utc::now()+ChronoDuration::days(1)})
        .bind((!expired).then(Utc::now)).bind((!expired).then_some(USER)).execute(&pool).await.unwrap();
    }
    let expected =
        serde_json::to_value(invitations::list(&pool, admin, TRIP).await.unwrap()).unwrap();
    let list = expected.as_array().unwrap();
    assert_eq!(list.len(), 4);
    assert_eq!(
        list.iter().map(|i| i["id"].clone()).collect::<Vec<_>>(),
        vec![
            successor.clone(),
            json!(INVITE),
            json!(Uuid::from_u128(53)),
            json!(Uuid::from_u128(52))
        ]
    );
    assert_eq!(list[0]["series_id"], json!(INVITE));
    assert_eq!(list[0]["predecessor_id"], json!(INVITE));
    assert_eq!(list[0]["redemption_count"], 1);
    assert_eq!(list[1]["redemption_count"], 1);
    assert_eq!(list[1]["revoked_by_user_id"], json!(USER));
    assert_eq!(list[1]["revocation_actor_known"], true);
    assert!(!list[1]["revoked_at"].is_null());
    assert_eq!(list[2]["redemption_count"], 0);
    assert_eq!(list[3]["redemption_count"], 0);
    assert!(list[3]["revoked_at"].is_null());
    assert!(list[3]["max_uses"].is_null());
    let keys = [
        "id",
        "tournament_id",
        "series_id",
        "predecessor_id",
        "created_by_user_id",
        "created_at",
        "expires_at",
        "revoked_at",
        "revoked_by_user_id",
        "revocation_actor_known",
        "max_uses",
        "redemption_count",
    ]
    .into_iter()
    .collect::<std::collections::BTreeSet<_>>();
    for row in list {
        assert_eq!(
            row.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<std::collections::BTreeSet<_>>(),
            keys
        );
    }
    assert!(!expected.to_string().contains(&secret));
    assert!(
        !expected
            .to_string()
            .contains(rotated["token"].as_str().unwrap())
    );
    let before = read_state(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut reads = Vec::new();
    for _ in 0..12 {
        let router = app.clone();
        let t = token.clone();
        let value = expected.clone();
        reads.push(tokio::spawn(async move {
            let r = get(&router, &path(), &t).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_private(&r);
            assert_eq!(body(r).await, value);
        }));
    }
    for read in reads {
        timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(read_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn exact_roles_cross_tournament_global_admin_and_error_contracts_stay_intact(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    for role in ["admin", "scorer", "player", "viewer"] {
        sqlx::query("UPDATE tournament_memberships SET role=$1::text::tournament_role WHERE tournament_id=$2 AND user_id=$3").bind(role).bind(TRIP).bind(USER).execute(&pool).await.unwrap();
        let r = get(&app, &path(), &token).await;
        assert_private(&r);
        assert_eq!(
            r.status(),
            if role == "admin" {
                StatusCode::OK
            } else {
                StatusCode::FORBIDDEN
            }
        );
        if role == "admin" {
            assert_eq!(body(r).await, json!([]));
        } else {
            assert_eq!(body(r).await["error"]["code"], "forbidden");
        }
    }
    let outsider = Uuid::from_u128(9);
    let other = Uuid::from_u128(99);
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES($1,'other_invite_admin','Other admin','admin')").bind(outsider).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Other trip','2026-10-15','2026-10-15',1)").bind(other).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(other)
    .bind(outsider)
    .execute(&pool)
    .await
    .unwrap();
    let outside_token = Uuid::new_v4().to_string();
    auth::create_session(
        &pool,
        outsider,
        &hash_session_token(&outside_token),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap();
    let r = get(&app, &path(), &outside_token).await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert_private(&r);
    let r = get(
        &app,
        &format!("/api/tournaments/{other}/invitations"),
        &outside_token,
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_private(&r);
    assert_eq!(body(r).await, json!([]));
    for target in ["invalid".to_owned(), Uuid::new_v4().to_string()] {
        let r = get(
            &app,
            &format!("/api/tournaments/{target}/invitations"),
            &token,
        )
        .await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert_private(&r);
    }
    let r = get(&app, &path(), "missing-session").await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_private(&r);
    assert!(matches!(
        invitations::list(&pool, Uuid::new_v4(), Uuid::new_v4()).await,
        Err(invitations::InvitationError::Unauthenticated)
    ));
    let r = write(&app, "POST", &path(), None, json!({})).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_private(&r);
}

#[sqlx::test(migrations = "../migrations")]
async fn committed_membership_demotion_or_removal_during_wait_denies_without_retry(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    for remove in [false, true] {
        sqlx::query(
            "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let router = app.clone();
        let t = token.clone();
        let read = tokio::spawn(async move { get(&router, &path(), &t).await });
        wait_blocked(&pool, pid).await;
        sqlx::query(if remove {
            "DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2"
        } else {
            "UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1 AND user_id=$2"
        })
        .bind(TRIP)
        .bind(USER)
        .execute(&mut *gate)
        .await
        .unwrap();
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_private(&r);
        assert_eq!(body(r).await["error"]["code"], "forbidden");
    }
}

async fn failed_once(pool: PgPool, code: &str) {
    let state = fixture(&pool).await;
    invitation(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let before = read_state(&pool).await;
    let mut events = state.live_events.subscribe();
    sqlx::raw_sql("CREATE SEQUENCE read_attempts; CREATE TABLE read_failure(code text NOT NULL); CREATE FUNCTION fail_invitation_read() RETURNS boolean LANGUAGE plpgsql AS $$ DECLARE failure_code text; BEGIN PERFORM nextval('read_attempts'); SELECT code INTO failure_code FROM read_failure; RAISE EXCEPTION 'synthetic invitation read failure' USING ERRCODE=failure_code; END $$; ALTER TABLE tournament_invitations RENAME TO stored_invitations; CREATE VIEW tournament_invitations AS SELECT * FROM stored_invitations WHERE fail_invitation_read();").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO read_failure(code) VALUES($1)")
        .bind(code)
        .execute(&pool)
        .await
        .unwrap();
    let r = get(&app, &path(), &token).await;
    assert_eq!(r.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_private(&r);
    let value = body(r).await;
    assert_eq!(value["error"]["code"], "internal_error");
    assert!(!value.to_string().contains("synthetic"));
    assert!(!value.is_array());
    let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM read_attempts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempts, 1);
    sqlx::raw_sql("DROP VIEW tournament_invitations; ALTER TABLE stored_invitations RENAME TO tournament_invitations;").execute(&pool).await.unwrap();
    assert_eq!(read_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}
#[sqlx::test(migrations = "../migrations")]
async fn synthetic_serialization_failure_keeps_single_attempt_and_generic_500(pool: PgPool) {
    failed_once(pool, "40001").await;
}
#[sqlx::test(migrations = "../migrations")]
async fn unrelated_database_failure_keeps_single_attempt_and_generic_500(pool: PgPool) {
    failed_once(pool, "XX000").await;
}

#[sqlx::test(migrations = "../migrations")]
async fn cancellation_releases_exclusive_session_and_user_authority(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (id, token) = session(&pool).await;
    let before = read_state(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut gate = pool.begin().await.unwrap();
    let pid = blocker(&mut gate).await;
    lock_membership(&mut gate).await;
    let router = app.clone();
    let read = tokio::spawn(async move { get(&router, &path(), &token).await });
    wait_blocked(&pool, pid).await;
    read.abort();
    assert!(read.await.unwrap_err().is_cancelled());
    gate.commit().await.unwrap();
    timeout(Duration::from_secs(10), auth::revoke_session(&pool, id))
        .await
        .unwrap()
        .unwrap();
    let (_, fresh) = session(&pool).await;
    let r = get(&app, &path(), &fresh).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_private(&r);
    assert_eq!(body(r).await, json!([]));
    assert_eq!(read_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}
