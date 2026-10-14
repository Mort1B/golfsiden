use super::support::*;
use axum::http::StatusCode;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};

#[sqlx::test(migrations = "../migrations")]
async fn membership_demotion_retries_authority_and_changed_hidden_final_context(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
    let targets = vec![
        format!("/api/rounds/{ROUND}/stableford/scorecards/{PLAYER}"),
        format!("/api/rounds/{ROUND}/stableford/scorecards/{PLAYER}/scoring"),
    ];
    for (index, path) in targets.into_iter().enumerate() {
        sqlx::query(
            "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        let expected =
            sqlx::query_scalar("SELECT visibility_updated_at FROM tournaments WHERE id=$1")
                .bind(TRIP)
                .fetch_one(&pool)
                .await
                .unwrap();
        let released = golf_api::repositories::tournament_visibility::update_authorized(
            &pool, session, TRIP, false, expected,
        )
        .await
        .unwrap();
        assert!(!released.visibility.back_nine_hidden);
        let baseline = get(&app, &path, &token).await;
        assert_eq!(baseline.status(), StatusCode::OK);
        let baseline = body(baseline).await;
        if index == 0 {
            assert_eq!(baseline["visibility"]["mode"], "full");
            assert_eq!(baseline["holes"].as_array().unwrap().len(), 18);
        }
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let router = app.clone();
        let p = path.clone();
        let t = token.clone();
        let read = tokio::spawn(async move { get(&router, &p, &t).await });
        wait_blocked(&pool, pid).await;
        sqlx::query("SELECT set_config('app.final_round_visibility_tournament_id',$1::text,true),set_config('app.final_round_visibility_session_id',$2::text,true)").bind(TRIP).bind(session).execute(&mut *gate).await.unwrap();
        sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden=true WHERE id=$1")
            .bind(TRIP)
            .execute(&mut *gate)
            .await
            .unwrap();
        let hidden: bool =
            sqlx::query_scalar("SELECT final_round_back_nine_hidden FROM tournaments WHERE id=$1")
                .bind(TRIP)
                .fetch_one(&mut *gate)
                .await
                .unwrap();
        assert!(hidden);
        sqlx::query(
            "UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1 AND user_id=$2",
        )
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
        assert_private(&r);
        assert_eq!(
            r.status(),
            if index == 1 {
                StatusCode::FORBIDDEN
            } else {
                StatusCode::OK
            }
        );
        let value = body(r).await;
        if index == 0 {
            assert_eq!(value["visibility"]["mode"], "front_nine");
            assert_eq!(value["holes"].as_array().unwrap().len(), 9);
            assert!(value["complete"].is_null());
            assert!(value["confirmed"].is_null());
            assert!(value["confirmed_at"].is_null());
        } else {
            assert_eq!(value["error"]["code"], "forbidden");
        }
        assert_eq!(value, body(get(&app, &path, &token).await).await);
    }
}

async fn injected_failure(pool: PgPool, code: &str, status: StatusCode, attempts: i64) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
    sqlx::raw_sql("CREATE SEQUENCE read_attempts; CREATE TABLE read_failure(code text NOT NULL);
      CREATE FUNCTION fail_stableford_read() RETURNS boolean LANGUAGE plpgsql AS $$ DECLARE failure_code text; BEGIN PERFORM nextval('read_attempts'); SELECT code INTO failure_code FROM read_failure; RAISE EXCEPTION 'synthetic stableford read failure' USING ERRCODE=failure_code; END $$;
      ALTER TABLE rounds RENAME TO stored_rounds;
      CREATE VIEW rounds AS SELECT * FROM stored_rounds WHERE fail_stableford_read();").execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO read_failure(code) VALUES($1)")
        .bind(code)
        .execute(&pool)
        .await
        .unwrap();
    for path in paths() {
        sqlx::query("ALTER SEQUENCE read_attempts RESTART WITH 1")
            .execute(&pool)
            .await
            .unwrap();
        let r = get(&app, &path, &token).await;
        assert_eq!(r.status(), status, "{path}");
        assert_private(&r);
        let v = body(r).await;
        assert_eq!(
            v["error"]["code"],
            if status == StatusCode::SERVICE_UNAVAILABLE {
                "service_unavailable"
            } else {
                "internal_error"
            }
        );
        assert!(v.get("holes").is_none());
        assert!(v.get("writable_owners").is_none());
        assert!(!v.to_string().contains("synthetic"));
        let actual: i64 = sqlx::query_scalar("SELECT last_value FROM read_attempts")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(actual, attempts, "{path}");
    }
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn both_reads_stop_after_one_serialization_retry_with_private_503(pool: PgPool) {
    injected_failure(pool, "40001", StatusCode::SERVICE_UNAVAILABLE, 2).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn unrelated_database_failures_are_not_retried_on_either_selected_read(pool: PgPool) {
    injected_failure(pool, "XX000", StatusCode::INTERNAL_SERVER_ERROR, 1).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn retry_reloads_changed_owner_name_and_preserves_frozen_handicap_on_both_reads(
    pool: PgPool,
) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    for path in paths() {
        sqlx::query("UPDATE players SET display_name='Player 11' WHERE id=$1")
            .bind(PLAYER)
            .execute(&pool)
            .await
            .unwrap();
        let before = body(get(&app, &path, &token).await).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let router = app.clone();
        let p = path.clone();
        let t = token.clone();
        let read = tokio::spawn(async move { get(&router, &p, &t).await });
        wait_blocked(&pool, pid).await;
        sqlx::query(
            "UPDATE tournament_memberships SET role=role WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&mut *gate)
        .await
        .unwrap();
        sqlx::query("UPDATE players SET display_name='Fresh Stableford owner',current_handicap_index=1 WHERE id=$1").bind(PLAYER).execute(&mut *gate).await.unwrap();
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        assert_private(&r);
        let value = body(r).await;
        assert_ne!(value, before);
        assert_eq!(value["owner_name"], "Fresh Stableford owner");
        assert_eq!(value["playing_handicap"], before["playing_handicap"]);
        assert_eq!(value, body(get(&app, &path, &token).await).await);
    }
}
