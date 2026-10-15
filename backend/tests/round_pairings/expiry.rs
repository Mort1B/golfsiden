use super::{
    ADMIN, CONVERTED_FLIGHT, LEGACY_TEAM, OTHER_FLIGHT, P3, ROUND, assert_no_event, assert_private,
    body, put, seed, updated_at, valid,
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{AppState, api, auth::hash_session_token, repositories::auth};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("b1000000-0000-0000-0000-000000000001");
const OLD_FLIGHT: Uuid = uuid!("b1000000-0000-0000-0000-000000000033");

async fn stored(pool: &PgPool) -> Value {
    sqlx::query_scalar(include_str!("state.sql"))
        .bind(ROUND)
        .bind(TOURNAMENT)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn wait_blocked(pool: &PgPool, blocker: i32, query_pattern: &str) {
    timeout(Duration::from_secs(6), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                 WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))
                   AND query LIKE $2)",
            )
            .bind(blocker)
            .bind(query_pattern)
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
    .expect("pairing request must reach the selected final-transaction wait");
}

async fn wait_expired(pool: &PgPool, session: Uuid) {
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

async fn cycle(pool: PgPool, late: bool) {
    seed(&pool).await;
    sqlx::query("INSERT INTO flights(id,round_id,tournament_id,name,starting_hole,tee_time) VALUES($1,$2,$3,'Old solo',2,'09:10')")
        .bind(OLD_FLIGHT).bind(ROUND).bind(TOURNAMENT).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id,display_order) VALUES($1,$2,$3,$4,4)")
        .bind(OLD_FLIGHT).bind(ROUND).bind(TOURNAMENT).bind(P3).execute(&pool).await.unwrap();
    sqlx::raw_sql(include_str!("late_gate.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let before = stored(&pool).await;
    let expected = updated_at(&pool).await;

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
            .bind(ROUND)
            .execute(&mut *round_gate)
            .await
            .unwrap();
        let mut request = tokio::spawn({
            let app = app.clone();
            async move { app.oneshot(put(&token, valid(expected))).await.unwrap() }
        });
        // The preflight reads the round without locking it. Only final
        // authorization takes this lock, before acquiring session authority.
        tokio::select! {
            _ = wait_blocked(&pool, round_pid, "SELECT tournament_id FROM rounds%FOR UPDATE") => {},
            response = &mut request => panic!("pairing returned before final round wait: {}", response.unwrap().status()),
        }
        let mut gate = pool.begin().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *gate)
            .await
            .unwrap();
        if late {
            sqlx::query("SELECT pg_advisory_xact_lock(260015,2)")
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
            _ = wait_blocked(&pool, pid, if late { "UPDATE rounds SET updated_at%" } else { "SELECT role FROM tournament_memberships%FOR SHARE" }) => {},
            response = &mut request => panic!("pairing returned before late={late} wait: {}", response.unwrap().status()),
        }
        assert_no_event(&mut events);
        assert_eq!(
            stored(&pool).await,
            before,
            "pending roster stays uncommitted"
        );
        if expire {
            let active: bool = sqlx::query_scalar(
                "SELECT expires_at>clock_timestamp() FROM user_sessions WHERE id=$1",
            )
            .bind(session)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert!(active, "must observe the selected wait before expiry");
            wait_expired(&pool, session).await;
        }
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(6), request)
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
        assert_private(&response);
        if expire {
            assert_eq!(
                body(response).await,
                json!({"error":{"code":"unauthenticated","message":"authentication required"}})
            );
            assert_eq!(
                stored(&pool).await,
                before,
                "complete roster and round facts roll back"
            );
        } else {
            let result = body(response).await;
            assert_eq!(result["round_id"], ROUND.to_string());
            assert_eq!(result["tournament_id"], TOURNAMENT.to_string());
            assert_eq!(result["teams"], json!([]));
            assert_eq!(result["legacy_individual_groups"], json!([]));
            assert_eq!(result["flights"].as_array().unwrap().len(), 2);
            let after = stored(&pool).await;
            assert_eq!(after["teams"], json!([]));
            assert_eq!(after["team_members"], json!([]));
            assert_eq!(after["flights"].as_array().unwrap().len(), 2);
            assert_eq!(after["flight_members"].as_array().unwrap().len(), 3);
            assert_ne!(after["round"]["updated_at"], before["round"]["updated_at"]);
            for id in [CONVERTED_FLIGHT, OTHER_FLIGHT] {
                assert!(
                    after["flights"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|f| f["id"] == id.to_string())
                );
            }
            assert!(
                !after["flights"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|f| f["id"] == OLD_FLIGHT.to_string())
            );
            assert_eq!(before["teams"][0]["id"], LEGACY_TEAM.to_string());
            for key in [
                "tournament",
                "entrants",
                "players",
                "snapshots",
                "team_snapshots",
                "scores",
                "confirmations",
            ] {
                assert_eq!(after[key], before[key], "valid replacement preserves {key}");
            }
            let event = events.try_recv().unwrap();
            assert_eq!(
                (event.resource, event.tournament_id, event.id),
                ("round", TOURNAMENT, ROUND)
            );
        }
        assert_no_event(&mut events);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_final_membership_wait_rolls_back_pairings(pool: PgPool) {
    cycle(pool, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_after_roster_writes_rolls_back_pairings(pool: PgPool) {
    cycle(pool, true).await;
}
