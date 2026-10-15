use super::support::*;
use axum::http::{StatusCode, header};
use golf_api::repositories::{auth, tournament_visibility};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use uuid::Uuid;

#[sqlx::test(migrations = "../migrations")]
async fn full_dto_version_and_both_flags_survive_concurrent_reads_without_effects(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
    for hidden in [true, false, true] {
        let current = tournament_visibility::get_for_admin(&pool, USER, TRIP)
            .await
            .unwrap();
        let expected = tournament_visibility::update_authorized(
            &pool,
            session,
            TRIP,
            hidden,
            current.visibility_updated_at,
        )
        .await
        .unwrap()
        .visibility;
        let expected = serde_json::to_value(expected).unwrap();
        assert_eq!(expected.as_object().unwrap().len(), 3);
        assert_eq!(expected["back_nine_hidden"], hidden);
        assert_eq!(expected["tournament_id"], json!(TRIP));
        let before = read_state(&pool).await;
        let mut events = state.live_events.subscribe();
        let mut readers = Vec::new();
        for _ in 0..12 {
            let router = app.clone();
            let token = token.clone();
            let expected = expected.clone();
            readers.push(tokio::spawn(async move {
                let r = get(&router, &path(), &token).await;
                assert_eq!(r.status(), StatusCode::OK);
                assert_private(&r);
                assert_eq!(body(r).await, expected);
            }));
        }
        for read in readers {
            timeout(Duration::from_secs(10), read)
                .await
                .unwrap()
                .unwrap();
        }
        assert_eq!(read_state(&pool).await, before);
        assert!(events.try_recv().is_err());
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn exact_roles_missing_identity_and_lookup_precedence_stay_intact(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
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
        if role != "admin" {
            assert_eq!(body(r).await["error"]["code"], "forbidden");
        }
    }
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Other trip','2026-10-15','2026-10-15',1)").bind(other).execute(&pool).await.unwrap();
    let r = get(
        &app,
        &format!("/api/tournaments/{other}/final-round-visibility"),
        &token,
    )
    .await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert_private(&r);
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(other)
    .bind(USER)
    .execute(&pool)
    .await
    .unwrap();
    let r = get(
        &app,
        &format!("/api/tournaments/{other}/final-round-visibility"),
        &token,
    )
    .await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_private(&r);
    let r = get(&app, &path(), &token).await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert_private(&r);
    let missing = Uuid::new_v4();
    let r = get(
        &app,
        &format!("/api/tournaments/{missing}/final-round-visibility"),
        &token,
    )
    .await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert_private(&r);
    assert_eq!(body(r).await["error"]["code"], "not_found");
    let r = get(
        &app,
        "/api/tournaments/invalid/final-round-visibility",
        &token,
    )
    .await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert_private(&r);
    for token in ["", "unknown-session"] {
        let r = get(&app, &path(), token).await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_private(&r);
        assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    }
    assert!(matches!(
        tournament_visibility::get_for_session(&pool, Uuid::new_v4(), missing).await,
        Err(
            golf_api::repositories::session_read::SessionReadError::Authorization(
                golf_api::repositories::tournament_authorization::AuthorizationError::NotFound
            )
        )
    ));
    auth::revoke_session(&pool, session).await.unwrap();
    let r = get(&app, &path(), &token).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_private(&r);
}

#[sqlx::test(migrations = "../migrations")]
async fn retry_reloads_complete_visibility_and_version_after_membership_conflict(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
    let before = body(get(&app, &path(), &token).await).await;
    let mut gate = pool.begin().await.unwrap();
    let pid = blocker(&mut gate).await;
    lock_membership(&mut gate).await;
    let router = app.clone();
    let t = token.clone();
    let read = tokio::spawn(async move { get(&router, &path(), &t).await });
    wait_blocked(&pool, pid).await;
    sqlx::query("SELECT set_config('app.final_round_visibility_tournament_id',$1::text,true),set_config('app.final_round_visibility_session_id',$2::text,true)").bind(TRIP).bind(session).execute(&mut *gate).await.unwrap();
    sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden=false WHERE id=$1")
        .bind(TRIP)
        .execute(&mut *gate)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
    )
    .bind(TRIP)
    .bind(USER)
    .execute(&mut *gate)
    .await
    .unwrap();
    gate.commit().await.unwrap();
    let after = read_state(&pool).await;
    let mut events = state.live_events.subscribe();
    let r = timeout(Duration::from_secs(10), read)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_private(&r);
    let value = body(r).await;
    assert_eq!(value["back_nine_hidden"], false);
    assert_ne!(
        value["visibility_updated_at"],
        before["visibility_updated_at"]
    );
    let expected = serde_json::to_value(
        tournament_visibility::get_for_admin(&pool, USER, TRIP)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(value, expected);
    assert_eq!(read_state(&pool).await, after);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn retry_observes_membership_demotion_or_removal(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    for remove in [false, true] {
        sqlx::query(
            "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        let (_, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let router = app.clone();
        let read = tokio::spawn(async move { get(&router, &path(), &token).await });
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

async fn fail_read(pool: PgPool, code: &str) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let before = read_state(&pool).await;
    let mut events = state.live_events.subscribe();
    sqlx::raw_sql("CREATE SEQUENCE read_attempts; CREATE TABLE read_failure(code text NOT NULL); CREATE FUNCTION fail_visibility_read() RETURNS boolean LANGUAGE plpgsql AS $$ DECLARE failure_code text; BEGIN PERFORM nextval('read_attempts'); SELECT code INTO failure_code FROM read_failure; RAISE EXCEPTION 'synthetic private read failure' USING ERRCODE=failure_code; END $$; ALTER TABLE tournaments RENAME TO stored_tournaments; CREATE VIEW tournaments AS SELECT * FROM stored_tournaments WHERE fail_visibility_read();").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO read_failure(code) VALUES($1)")
        .bind(code)
        .execute(&pool)
        .await
        .unwrap();
    let r = get(&app, &path(), &token).await;
    assert_private(&r);
    assert_eq!(
        r.status(),
        if code == "40001" {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        }
    );
    let value = body(r).await;
    assert_eq!(
        value["error"]["code"],
        if code == "40001" {
            "service_unavailable"
        } else {
            "internal_error"
        }
    );
    assert!(!value.to_string().contains("synthetic"));
    assert!(value.get("back_nine_hidden").is_none());
    let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM read_attempts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(attempts, if code == "40001" { 2 } else { 1 });
    sqlx::raw_sql("DROP VIEW tournaments; ALTER TABLE stored_tournaments RENAME TO tournaments;")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(read_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}
#[sqlx::test(migrations = "../migrations")]
async fn exhausted_snapshot_retry_is_private_generic_503(pool: PgPool) {
    fail_read(pool, "40001").await;
}
#[sqlx::test(migrations = "../migrations")]
async fn unrelated_database_error_is_private_generic_500_without_retry(pool: PgPool) {
    fail_read(pool, "XX000").await;
}

#[sqlx::test(migrations = "../migrations")]
async fn cancellation_releases_authority_and_allows_later_reads(pool: PgPool) {
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
    assert_eq!(read_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn patch_headers_stale_noop_and_notification_contracts_are_unchanged(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let initial = body(get(&app, &path(), &token).await).await;
    let mut events = state.live_events.subscribe();
    let r=write(&app,"PATCH",&path(),Some(&token),json!({"back_nine_hidden":true,"expected_visibility_updated_at":initial["visibility_updated_at"]})).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_private(&r);
    assert_eq!(body(r).await, initial);
    assert!(events.try_recv().is_err());
    let r=write(&app,"PATCH",&path(),Some(&token),json!({"back_nine_hidden":false,"expected_visibility_updated_at":initial["visibility_updated_at"]})).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_private(&r);
    assert_eq!(body(r).await["back_nine_hidden"], false);
    assert!(events.try_recv().is_ok());
    assert!(events.try_recv().is_err());
    let r=write(&app,"PATCH",&path(),Some(&token),json!({"back_nine_hidden":true,"expected_visibility_updated_at":initial["visibility_updated_at"]})).await;
    assert_eq!(r.status(), StatusCode::CONFLICT);
    assert!(r.headers().get(header::CACHE_CONTROL).is_none());
    assert_eq!(
        body(r).await["error"]["code"],
        "final_round_visibility_stale"
    );
    assert!(events.try_recv().is_err());
    let r = write(&app, "PATCH", &path(), None, json!({})).await;
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(r.headers()[header::CACHE_CONTROL], "no-store");
}
