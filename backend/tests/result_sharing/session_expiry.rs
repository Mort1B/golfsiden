use super::support::*;
use axum::http::StatusCode;
use golf_api::{AppState, api};
use serde_json::{Value, json};
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Clone, Copy, Debug)]
enum Mutation {
    Issue,
    ReplaceBeforeInsert,
    ReplaceAfterInsert,
    Revoke,
}

async fn expired(pool: &PgPool, session: Uuid) -> bool {
    sqlx::query_scalar("SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1")
        .bind(session)
        .fetch_one(pool)
        .await
        .unwrap()
}

// Compare complete relevant grant/audit state without exposing capability hashes.
async fn stored(pool: &PgPool) -> (Value, Value) {
    let grants = sqlx::query_scalar("SELECT COALESCE(jsonb_agg(jsonb_build_object('id',id,'created_at',created_at,'expires_at',expires_at,'revoked_at',revoked_at,'revoked_by',revoked_by,'created_by',created_by) ORDER BY created_at,id),'[]') FROM tournament_result_shares WHERE tournament_id=$1")
        .bind(TRIP).fetch_one(pool).await.unwrap();
    let audits = sqlx::query_scalar("SELECT COALESCE(jsonb_agg(to_jsonb(a) ORDER BY id),'[]') FROM tournament_result_share_audits a WHERE tournament_id=$1")
        .bind(TRIP).fetch_one(pool).await.unwrap();
    (grants, audits)
}

async fn mutation_after_wait(pool: PgPool, mutation: Mutation, expire: bool) {
    let session = fixture(&pool).await;
    let previous = if matches!(mutation, Mutation::Issue) {
        None
    } else {
        Some(issue(&pool, session, None).await)
    };
    let before = stored(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    // Replacement writes two audits. Gate only the new issuance audit to prove
    // rollback of both the old revocation and the new grant at the final write.
    let final_insert = matches!(mutation, Mutation::ReplaceAfterInsert);
    if final_insert {
        sqlx::raw_sql("CREATE FUNCTION test_pause_issued_audit() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(314159,1); RETURN NEW; END $$; CREATE TRIGGER test_pause_issued_audit BEFORE INSERT ON tournament_result_share_audits FOR EACH ROW WHEN (NEW.action='issued') EXECUTE FUNCTION test_pause_issued_audit();")
            .execute(&pool).await.unwrap();
    }
    let mut gate = pool.begin().await.unwrap();
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query(if final_insert {
        "SELECT pg_advisory_xact_lock(314159,1)"
    } else {
        "LOCK TABLE tournament_result_share_audits IN SHARE MODE"
    })
    .execute(&mut *gate)
    .await
    .unwrap();
    if expire {
        sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '3 seconds' WHERE id=$1")
            .bind(session).execute(&pool).await.unwrap();
    }
    let previous_id = previous.as_ref().map(|(grant, _)| grant.id);
    let req = if matches!(mutation, Mutation::Revoke) {
        request(
            "DELETE",
            &format!(
                "/api/tournaments/{TRIP}/result-share/{}",
                previous_id.unwrap()
            ),
            Some(ADMIN_TOKEN),
            None,
            true,
        )
    } else {
        request(
            "POST",
            &format!("/api/tournaments/{TRIP}/result-share"),
            Some(ADMIN_TOKEN),
            Some(json!({"expected_grant_id":previous_id})),
            true,
        )
    };
    let task = tokio::spawn(app.clone().oneshot(req));
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_locks l JOIN pg_stat_activity a ON a.pid=l.pid
                 WHERE a.datname=current_database() AND NOT l.granted
                 AND $1=ANY(pg_blocking_pids(a.pid)) AND
                 (($2 AND l.locktype='advisory' AND l.classid=314159 AND l.objid=1)
                 OR (NOT $2 AND l.relation='tournament_result_share_audits'::regclass
                     AND l.mode='RowExclusiveLock')))",
            )
            .bind(blocker)
            .bind(final_insert)
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("must observe the exact late audit wait before expiry");
    assert!(
        !expired(&pool, session).await,
        "expired before {mutation:?} wait"
    );
    if expire {
        tokio::time::timeout(Duration::from_secs(5), async {
            while !expired(&pool, session).await {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
    gate.commit().await.unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert_eq!(result.headers()["cache-control"], "private, no-store");
    let status = result.status();
    use http_body_util::BodyExt;
    let bytes = result.into_body().collect().await.unwrap().to_bytes();
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    if expire {
        assert_eq!(status, StatusCode::UNAUTHORIZED, "expired {mutation:?}");
        assert_eq!(body["error"]["code"], "unauthenticated");
        assert_eq!(
            stored(&pool).await,
            before,
            "must roll back all grant/audit changes"
        );
    } else {
        assert_eq!(
            status,
            if matches!(mutation, Mutation::Revoke) {
                StatusCode::NO_CONTENT
            } else {
                StatusCode::CREATED
            }
        );
        let after = stored(&pool).await;
        let expected = match mutation {
            Mutation::Issue => (1, 1),
            Mutation::ReplaceBeforeInsert | Mutation::ReplaceAfterInsert => (2, 3),
            Mutation::Revoke => (1, 2),
        };
        assert_eq!(
            (
                after.0.as_array().unwrap().len(),
                after.1.as_array().unwrap().len()
            ),
            expected
        );
        let event = events.try_recv().unwrap();
        assert_eq!(event.resource, "tournament");
        assert_eq!((event.tournament_id, event.id), (TRIP, TRIP));
        if !matches!(mutation, Mutation::Revoke) {
            let id: Uuid = body["grant"]["id"].as_str().unwrap().parse().unwrap();
            let (status, _) = response(
                &app,
                request(
                    "POST",
                    &format!("/api/public/results/{id}"),
                    None,
                    Some(json!({"token":body["token"],"metric":"gross"})),
                    false,
                ),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
        }
    }
    assert!(matches!(
        events.try_recv(),
        Err(tokio::sync::broadcast::error::TryRecvError::Empty)
    ));
    if let Some((grant, token)) = previous {
        let (status, _) = response(&app, public_request(grant.id, &token, "gross", None)).await;
        assert_eq!(
            status,
            if expire {
                StatusCode::OK
            } else {
                StatusCode::NOT_FOUND
            }
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn issue_expiry_at_audit_rolls_back_without_event(pool: PgPool) {
    mutation_after_wait(pool, Mutation::Issue, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn issue_valid_after_audit_wait_commits_once(pool: PgPool) {
    mutation_after_wait(pool, Mutation::Issue, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn replacement_expiry_at_old_audit_preserves_old_link(pool: PgPool) {
    mutation_after_wait(pool, Mutation::ReplaceBeforeInsert, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn replacement_valid_after_old_audit_wait_commits_once(pool: PgPool) {
    mutation_after_wait(pool, Mutation::ReplaceBeforeInsert, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn replacement_expiry_at_new_audit_preserves_old_link(pool: PgPool) {
    mutation_after_wait(pool, Mutation::ReplaceAfterInsert, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn replacement_valid_after_new_audit_wait_commits_once(pool: PgPool) {
    mutation_after_wait(pool, Mutation::ReplaceAfterInsert, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn revoke_expiry_at_audit_preserves_link_without_event(pool: PgPool) {
    mutation_after_wait(pool, Mutation::Revoke, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn revoke_valid_after_audit_wait_commits_once(pool: PgPool) {
    mutation_after_wait(pool, Mutation::Revoke, false).await;
}
