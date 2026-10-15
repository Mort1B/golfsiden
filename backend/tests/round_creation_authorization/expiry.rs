use super::{ADMIN, TOURNAMENT_ID, gated_json_request, response_json, seed, valid_round};
use axum::{
    Router,
    http::{Request, StatusCode, header::CACHE_CONTROL},
};
use chrono::{Duration as ChronoDuration, Utc};
use golf_api::{AppState, api, auth::hash_session_token, repositories::auth};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use tokio::{
    sync::broadcast::error::TryRecvError,
    time::{Duration, timeout},
};
use tower::ServiceExt;
use uuid::Uuid;

async fn stored(pool: &PgPool) -> Value {
    sqlx::query_scalar(
        "SELECT jsonb_build_object(
          'rounds',(SELECT coalesce(jsonb_agg(to_jsonb(r) ORDER BY id),'[]') FROM rounds r WHERE tournament_id=$1),
          'windows',(SELECT coalesce(jsonb_agg(to_jsonb(f) ORDER BY round_id),'[]') FROM fantasy_rounds f WHERE tournament_id=$1),
          'game',(SELECT to_jsonb(g) FROM fantasy_games g WHERE tournament_id=$1),
          'tournament',(SELECT to_jsonb(t) FROM tournaments t WHERE id=$1),
          'guard',(SELECT to_jsonb(g) FROM tournament_overall_configuration_guards g WHERE tournament_id=$1))",
    )
    .bind(TOURNAMENT_ID)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn install_late_gate(pool: &PgPool) {
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION test_round_creation_wait() RETURNS trigger LANGUAGE plpgsql AS $$
        BEGIN
            IF NOT EXISTS(SELECT 1 FROM rounds WHERE id=NEW.id AND status='draft') OR
               NOT EXISTS(SELECT 1 FROM fantasy_rounds WHERE round_id=NEW.id AND tournament_id=NEW.tournament_id) THEN
                RAISE EXCEPTION 'expected pending round and Fantasy window before late wait';
            END IF;
            PERFORM pg_advisory_xact_lock(260015,1);
            RETURN NEW;
        END $$;
        -- Alphabetical AFTER trigger ordering places this after fantasy_round_created.
        CREATE TRIGGER test_round_creation_wait AFTER INSERT ON rounds
            FOR EACH ROW EXECUTE FUNCTION test_round_creation_wait();
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
}

async fn gate(pool: &PgPool, late: bool) -> (Transaction<'_, Postgres>, i32) {
    let mut tx = pool.begin().await.unwrap();
    let pid = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    if late {
        sqlx::query("SELECT pg_advisory_xact_lock(260015,1)")
            .execute(&mut *tx)
            .await
            .unwrap();
    } else {
        sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR UPDATE")
            .bind(TOURNAMENT_ID).bind(ADMIN).execute(&mut *tx).await.unwrap();
    }
    (tx, pid)
}

async fn wait_blocked(pool: &PgPool, pid: i32, late: bool) {
    timeout(Duration::from_secs(5), async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                 WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid))
                   AND query LIKE $2)",
            )
            .bind(pid)
            .bind(if late {
                "INSERT INTO rounds%"
            } else {
                "%FROM tournament_memberships%"
            })
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
    .expect("creation must reach the selected final-transaction wait");
}

async fn wait_expired(pool: &PgPool, session: Uuid) {
    timeout(Duration::from_secs(10), async {
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
    sqlx::query("INSERT INTO fantasy_games(tournament_id) VALUES($1)")
        .bind(TOURNAMENT_ID)
        .execute(&pool)
        .await
        .unwrap();
    install_late_gate(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app: Router = api::router(state);
    let before = stored(&pool).await;

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
        let (request, preflight_finished, resume) = gated_json_request(
            Request::post(format!("/api/tournaments/{TOURNAMENT_ID}/rounds")),
            &token,
            valid_round(),
        );
        let mut task = tokio::spawn({
            let app = app.clone();
            async move { app.oneshot(request).await.unwrap() }
        });
        // Preflight also locks membership. Gate only the final write transaction.
        timeout(Duration::from_secs(5), preflight_finished)
            .await
            .unwrap()
            .unwrap();
        if expire {
            sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '5 seconds' WHERE id=$1")
                .bind(session).execute(&pool).await.unwrap();
        }
        let (blocker, pid) = gate(&pool, late).await;
        resume.send(()).unwrap();
        tokio::select! {
            _ = wait_blocked(&pool, pid, late) => {},
            response = &mut task => panic!("creation returned before late={late} wait: {}", response.unwrap().status()),
        }
        assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
        assert_eq!(
            stored(&pool).await,
            before,
            "pending writes stay uncommitted"
        );
        if expire {
            wait_expired(&pool, session).await;
        }
        blocker.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            response.status(),
            if expire {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::CREATED
            },
            "expiry={expire}, late={late}"
        );
        if expire {
            assert!(
                response.headers()[CACHE_CONTROL]
                    .to_str()
                    .unwrap()
                    .contains("no-store")
            );
            assert_eq!(
                response_json(response).await["error"]["code"],
                "unauthenticated"
            );
            assert_eq!(
                stored(&pool).await,
                before,
                "round, child and parent facts roll back"
            );
        } else {
            let body = response_json(response).await;
            let id = Uuid::parse_str(body["id"].as_str().unwrap()).unwrap();
            assert_eq!(body["tournament_id"], TOURNAMENT_ID.to_string());
            assert_eq!(body["round_number"], 1);
            assert_eq!(body["status"], "draft");
            let after = stored(&pool).await;
            assert_eq!(after["rounds"].as_array().unwrap().len(), 1);
            assert_eq!(after["windows"].as_array().unwrap().len(), 1);
            assert_eq!(after["rounds"][0]["id"], id.to_string());
            assert_eq!(after["windows"][0]["round_id"], id.to_string());
            assert_eq!(after["tournament"], before["tournament"]);
            assert_eq!(after["game"], before["game"]);
            let event = events.try_recv().unwrap();
            assert_eq!(
                (event.resource, event.tournament_id, event.id),
                ("round", TOURNAMENT_ID, id)
            );
        }
        assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_final_membership_wait_rolls_back_creation(pool: PgPool) {
    cycle(pool, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_after_round_and_child_insert_rolls_back_creation(pool: PgPool) {
    cycle(pool, true).await;
}
