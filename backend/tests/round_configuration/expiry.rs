use super::{
    ADMIN, DRAFT, assert_no_event, make_draft_round_openable, manual, request, response_json, seed,
    timestamp,
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{AppState, api, auth::hash_session_token, repositories::auth};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("84000000-0000-0000-0000-000000000003");

async fn stored(pool: &PgPool) -> Value {
    sqlx::query_scalar(include_str!("state.sql"))
        .bind(DRAFT)
        .bind(TOURNAMENT)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn wait_blocked(pool: &PgPool, blocker: i32, pattern: &str) {
    timeout(Duration::from_secs(6), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                 WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))
                   AND query LIKE $2)",
            )
            .bind(blocker)
            .bind(pattern)
            .fetch_one(pool)
            .await
            .unwrap();
            if blocked {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("configuration must reach the selected final-transaction wait");
}

async fn expire_at_gate(pool: &PgPool, session: Uuid) {
    let active: bool =
        sqlx::query_scalar("SELECT expires_at>clock_timestamp() FROM user_sessions WHERE id=$1")
            .bind(session)
            .fetch_one(pool)
            .await
            .unwrap();
    assert!(active, "selected wait must be observed before expiry");
    timeout(Duration::from_secs(12), async {
        loop {
            let expired: bool = sqlx::query_scalar(
                "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1",
            )
            .bind(session)
            .fetch_one(pool)
            .await
            .unwrap();
            if expired {
                return;
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
    })
    .await
    .expect("database clock must pass the session expiry");
}

fn assert_valid_state(before: &Value, after: &Value, result: &Value) {
    assert_eq!(result["id"], DRAFT.to_string());
    assert_eq!(result["tournament_id"], TOURNAMENT.to_string());
    assert_eq!(result["course_name"], "Exact Course");
    assert_eq!(result["tee_name"], "White");
    assert_eq!(result["number_of_holes"], 2);
    assert_eq!(result["status"], "draft");
    let mut preserved = after.clone();
    for (key, added) in [("courses", 1), ("tees", 1), ("holes", 2)] {
        let original = before[key].as_array().unwrap();
        let current = after[key].as_array().unwrap();
        assert_eq!(current.len(), original.len() + added);
        assert!(original.iter().all(|row| current.contains(row)));
        preserved[key] = before[key].clone();
    }
    for key in [
        "course_id",
        "course_name",
        "tee_id",
        "tee_name",
        "number_of_holes",
    ] {
        assert_eq!(after["round"][key], result[key]);
    }
    assert_ne!(after["round"]["course_id"], before["round"]["course_id"]);
    assert_ne!(after["round"]["tee_id"], before["round"]["tee_id"]);
    assert_ne!(after["round"]["updated_at"], before["round"]["updated_at"]);
    let mut original_round = before["round"].as_object().unwrap().clone();
    let mut current_round = after["round"].as_object().unwrap().clone();
    for key in [
        "course_id",
        "course_name",
        "tee_id",
        "tee_name",
        "number_of_holes",
        "updated_at",
    ] {
        original_round.remove(key);
        current_round.remove(key);
    }
    assert_eq!(current_round, original_round);
    preserved["round"] = before["round"].clone();
    assert_eq!(&preserved, before, "unrelated/history facts are preserved");
}

async fn cycle(pool: PgPool, late: bool) {
    seed(&pool).await;
    make_draft_round_openable(&pool).await;
    sqlx::raw_sql(include_str!("late_gate.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let before = stored(&pool).await;
    let expected = timestamp(&pool, DRAFT).await;
    for expire in [true, false] {
        let token = Uuid::new_v4().to_string();
        let session = auth::create_session(
            &pool,
            ADMIN,
            &hash_session_token(&token),
            Utc::now() + ChronoDuration::hours(1),
        )
        .await
        .unwrap()
        .session_id;
        let mut round_gate = pool.begin().await.unwrap();
        let round_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *round_gate)
            .await
            .unwrap();
        sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
            .bind(DRAFT)
            .execute(&mut *round_gate)
            .await
            .unwrap();
        let mut pending = tokio::spawn({
            let app = app.clone();
            let command = request(DRAFT, Some(&token), true, manual(&expected));
            async move { app.oneshot(command).await.unwrap() }
        });
        // Preflight's plain round read completes before this final lock wait.
        tokio::select! {
            _ = wait_blocked(&pool, round_pid, "SELECT tournament_id FROM rounds%FOR UPDATE") => {},
            response = &mut pending => panic!("returned before final round wait: {}", response.unwrap().status()),
        }
        let mut gate = pool.begin().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *gate)
            .await
            .unwrap();
        if late {
            sqlx::query("SELECT pg_advisory_xact_lock(260015,3)")
                .execute(&mut *gate)
                .await
                .unwrap();
        } else {
            sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR UPDATE")
                .bind(TOURNAMENT).bind(ADMIN).execute(&mut *gate).await.unwrap();
        }
        if expire {
            sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '8 seconds' WHERE id=$1")
                .bind(session).execute(&pool).await.unwrap();
        }
        round_gate.commit().await.unwrap();
        tokio::select! {
            _ = wait_blocked(&pool, pid, if late { "UPDATE rounds%SET course_id%" } else { "SELECT role FROM tournament_memberships%FOR SHARE" }) => {},
            response = &mut pending => panic!("returned before late={late} wait: {}", response.unwrap().status()),
        }
        assert_eq!(
            stored(&pool).await,
            before,
            "pending hierarchy stays uncommitted"
        );
        assert_no_event(&mut events);
        if expire {
            expire_at_gate(&pool, session).await;
        }
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(6), pending)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            response.status(),
            if expire {
                axum::http::StatusCode::UNAUTHORIZED
            } else {
                axum::http::StatusCode::OK
            },
            "expiry={expire}, late={late}"
        );
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        if expire {
            assert_eq!(
                response_json(response).await,
                json!({"error":{"code":"unauthenticated","message":"authentication required"}})
            );
            assert_eq!(
                stored(&pool).await,
                before,
                "complete hierarchy/attachment/history rollback"
            );
        } else {
            let result = response_json(response).await;
            let after = stored(&pool).await;
            assert_valid_state(&before, &after, &result);
            let event = events.try_recv().unwrap();
            assert_eq!(
                (event.resource, event.tournament_id, event.id),
                ("round", TOURNAMENT, DRAFT)
            );
        }
        assert_no_event(&mut events);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_final_membership_wait_rolls_back_configuration(pool: PgPool) {
    cycle(pool, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_after_hierarchy_and_attachment_rolls_back_configuration(pool: PgPool) {
    cycle(pool, true).await;
}
