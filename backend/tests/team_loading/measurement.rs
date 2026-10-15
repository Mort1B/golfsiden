use super::support::*;
use golf_api::repositories::teams;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use std::{path::Path, time::Instant};
use uuid::Uuid;

const REPEATS: i64 = 30;

async fn read(
    pool: &PgPool,
    app: &axum::Router,
    mode: &str,
    session: Uuid,
    round: Uuid,
    token: &str,
) -> Value {
    match mode {
        "internal" => serde_json::to_value(teams::list(pool, round).await.unwrap()).unwrap(),
        "session" => {
            serde_json::to_value(teams::list_for_session(pool, session, round).await.unwrap())
                .unwrap()
        }
        "http" => http(app, round, token).await,
        _ => unreachable!(),
    }
}

#[sqlx::test(migrations = "../migrations")]
#[ignore = "exclusive disposable PostgreSQL with pg_stat_statements required"]
async fn counts_and_complete_local_read_cost(pool: PgPool) {
    assert_eq!(std::env::var("GOLF_TEAM_MEASURE").as_deref(), Ok("1"));
    let output = std::env::var("GOLF_TEAM_MEASURE_OUT").expect("measurement output path required");
    sqlx::query("CREATE EXTENSION IF NOT EXISTS pg_stat_statements")
        .execute(&pool)
        .await
        .unwrap();
    let measured = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(1)
        .connect_with((*pool.connect_options()).clone())
        .await
        .unwrap();
    let (app, session, token) = setup(&measured).await;
    let mut targets = Vec::new();
    for (index, count) in [0, 1, 12, 100].into_iter().enumerate() {
        let target = round(&measured, index as i16 + 1, count, false).await;
        targets.push((count, target));
    }
    let mut evidence = Vec::new();
    let mut regressions = Vec::new();
    for (count, round) in targets {
        let expected = original(&measured, round).await;
        let payload_sha256 = format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&expected).unwrap())
        );
        for mode in ["internal", "session", "http"] {
            for _ in 0..3 {
                assert_eq!(
                    read(&measured, &app, mode, session, round, &token).await,
                    expected
                );
            }
            sqlx::query("SELECT pg_stat_statements_reset()")
                .execute(&pool)
                .await
                .unwrap();
            let mut samples = Vec::new();
            for _ in 0..REPEATS {
                let start = Instant::now();
                let result = read(&measured, &app, mode, session, round, &token).await;
                samples.push(start.elapsed().as_secs_f64() * 1000.0);
                assert_eq!(result, expected);
            }
            let queries: Vec<Value> = sqlx::query("SELECT query,calls,rows,total_exec_time FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND toplevel AND query NOT LIKE '%pg_stat_statements%' ORDER BY query")
                .fetch_all(&pool).await.unwrap().into_iter().map(|r| json!({"query":r.get::<String,_>("query"),"calls":r.get::<i64,_>("calls"),"rows":r.get::<i64,_>("rows"),"execution_ms":r.get::<f64,_>("total_exec_time")})).collect();
            let mut team_calls = 0;
            let mut member_calls = 0;
            let mut authority_calls = 0;
            let mut transaction_calls = 0;
            for query in &queries {
                let sql = query["query"].as_str().unwrap();
                let calls = query["calls"].as_i64().unwrap();
                if sql.contains("FROM team_memberships tm JOIN players p") {
                    member_calls += calls;
                } else if sql.contains("FROM teams WHERE round_id") {
                    team_calls += calls;
                } else if sql.starts_with("SELECT") {
                    authority_calls += calls;
                } else {
                    transaction_calls += calls;
                }
            }
            let bounded =
                team_calls == REPEATS && member_calls == if count == 0 { 0 } else { REPEATS };
            if !bounded {
                regressions.push(format!("{count} teams / {mode}: {team_calls} team + {member_calls} member calls over {REPEATS} reads"));
            }
            assert_eq!(
                authority_calls,
                REPEATS
                    * match mode {
                        "internal" => 0,
                        "session" => 4,
                        "http" => 5,
                        _ => unreachable!(),
                    }
            );
            // PostgreSQL records BEGIN/COMMIT; utility visibility is not a
            // protocol round-trip counter (SET TRANSACTION is not recorded).
            assert_eq!(transaction_calls, REPEATS * 2);
            evidence.push(json!({"teams":count,"mode":mode,"repeats":REPEATS,"warm_ms":samples,"team_calls":team_calls,"member_calls":member_calls,"authority_calls":authority_calls,"transaction_calls":transaction_calls,"bounded":bounded,"payload_sha256":payload_sha256,"queries":queries}));
        }
    }
    let version: String = sqlx::query_scalar("SELECT version()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let settings: Vec<Value> = sqlx::query("SELECT name,setting FROM pg_settings WHERE name IN ('lc_collate','shared_buffers','work_mem','jit','pg_stat_statements.track','pg_stat_statements.track_planning') ORDER BY name").fetch_all(&pool).await.unwrap().into_iter().map(|r|json!({"name":r.get::<String,_>("name"),"setting":r.get::<String,_>("setting")})).collect();
    // Retain only synthetic fixture payloads, timings and normalized statements.
    std::fs::write(Path::new(&output), serde_json::to_string_pretty(&json!({"postgres":version,"settings":settings,"samples":evidence,"regressions":regressions})).unwrap()).unwrap();
    measured.close().await;
    assert!(
        regressions.is_empty(),
        "team loading exceeds the bounded query budget: {regressions:?}"
    );
}
