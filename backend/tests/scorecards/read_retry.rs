use super::*;

#[derive(Clone, Copy)]
enum Change {
    Logout,
    RemoveMembership,
    KeepMembership,
}

async fn stored_state(pool: &PgPool) -> Value {
    sqlx::query_scalar(
        "SELECT jsonb_build_object(
          'scores', (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY s.id), '[]') FROM scores s),
          'audits', (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY a.id), '[]') FROM score_audits a),
          'confirmations', (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY c.id), '[]') FROM scorecard_confirmations c))",
    ).fetch_one(pool).await.unwrap()
}

async fn prepare(pool: &PgPool) {
    seed_open(pool).await;
    for (round_id, owner) in [
        (INDIVIDUAL_ROUND_ID, ScoreOwner::Player { id: PLAYER_A }),
        (
            SCRAMBLE_ROUND_ID,
            ScoreOwner::Team {
                id: SCRAMBLE_TEAM_ID,
            },
        ),
    ] {
        scorecards::save(
            pool,
            scorecards::SaveScore {
                round_id,
                owner,
                hole_id: HOLE_1,
                gross_strokes: 5,
                submitted_by: USER_A,
            },
        )
        .await
        .unwrap();
    }
}

async fn wait_for_lock(pool: &PgPool, blocker: i32) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                 WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(blocker)
            .fetch_one(pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("read must reach the held authority lock");
}

async fn conflicting_authority(pool: PgPool, change: Change, team: bool) {
    prepare(&pool).await;
    let before = stored_state(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(Arc::clone(&state));
    let (round, kind, owner) = if team {
        (SCRAMBLE_ROUND_ID, "team", SCRAMBLE_TEAM_ID)
    } else {
        (INDIVIDUAL_ROUND_ID, "player", PLAYER_A)
    };
    let baseline = response_json(
        app.clone()
            .oneshot(scoring_request(round, kind, owner, USER_A))
            .await
            .unwrap(),
    )
    .await;
    let mut gate = pool.begin().await.unwrap();
    let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    if matches!(change, Change::Logout) {
        sqlx::query("SELECT id FROM user_sessions WHERE user_id=$1 FOR UPDATE")
            .bind(USER_A)
            .execute(&mut *gate)
            .await
            .unwrap();
    } else {
        sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR UPDATE")
            .bind(TOURNAMENT_ID).bind(USER_A).execute(&mut *gate).await.unwrap();
    }
    let job = tokio::spawn(async move {
        app.oneshot(scoring_request(round, kind, owner, USER_A))
            .await
            .unwrap()
    });
    wait_for_lock(&pool, gate_pid).await;
    match change {
        Change::Logout => {
            sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE user_id=$1")
                .bind(USER_A)
                .execute(&mut *gate)
                .await
                .unwrap();
        }
        Change::RemoveMembership => {
            sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
                .bind(TOURNAMENT_ID)
                .bind(USER_A)
                .execute(&mut *gate)
                .await
                .unwrap();
        }
        Change::KeepMembership => {
            sqlx::query(
                "UPDATE tournament_memberships SET role=role WHERE tournament_id=$1 AND user_id=$2",
            )
            .bind(TOURNAMENT_ID)
            .bind(USER_A)
            .execute(&mut *gate)
            .await
            .unwrap();
        }
    }
    gate.commit().await.unwrap();
    let response = tokio::time::timeout(Duration::from_secs(5), job)
        .await
        .unwrap()
        .unwrap();
    let expected = match change {
        Change::Logout => StatusCode::UNAUTHORIZED,
        Change::RemoveMembership => StatusCode::FORBIDDEN,
        Change::KeepMembership => StatusCode::OK,
    };
    assert_eq!(response.status(), expected);
    assert_eq!(
        response.headers()["cache-control"],
        if expected == StatusCode::OK {
            "private, no-store"
        } else {
            "no-store"
        }
    );
    let body = response_json(response).await;
    match change {
        Change::KeepMembership => assert_eq!(body, baseline),
        Change::Logout => assert_eq!(
            body,
            json!({"error":{"code":"unauthenticated","message":"authentication required"}})
        ),
        Change::RemoveMembership => assert_eq!(
            body,
            json!({"error":{"code":"forbidden","message":"request is not permitted"}})
        ),
    }
    assert_eq!(stored_state(&pool).await, before);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_logout_returns_401_without_score_data(pool: PgPool) {
    conflicting_authority(pool, Change::Logout, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_membership_removal_returns_403_without_score_data(pool: PgPool) {
    conflicting_authority(pool, Change::RemoveMembership, true).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_valid_individual_read_recovers_identical_card(pool: PgPool) {
    conflicting_authority(pool, Change::KeepMembership, false).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_valid_team_read_recovers_identical_card(pool: PgPool) {
    conflicting_authority(pool, Change::KeepMembership, true).await;
}

// The view exists only in this disposable test schema. A nontransactional
// sequence proves the exact number of whole-read attempts even after rollback.
async fn injected_read_failure(pool: PgPool, code: &str, status: StatusCode, attempts: i64) {
    prepare(&pool).await;
    let before = stored_state(&pool).await;
    sqlx::raw_sql(
        "CREATE SEQUENCE read_attempts;
         CREATE TABLE read_failure (code text NOT NULL);
         CREATE FUNCTION fail_scoring_read() RETURNS boolean LANGUAGE plpgsql AS $$
         DECLARE failure_code text;
         BEGIN
           PERFORM nextval('read_attempts');
           SELECT code INTO failure_code FROM read_failure;
           RAISE EXCEPTION 'synthetic read failure' USING ERRCODE=failure_code;
         END $$;
         ALTER TABLE rounds RENAME TO stored_rounds;
         CREATE VIEW rounds AS SELECT * FROM stored_rounds WHERE fail_scoring_read();",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO read_failure(code) VALUES ($1)")
        .bind(code)
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let response = api::router(Arc::clone(&state))
        .oneshot(scoring_request(
            INDIVIDUAL_ROUND_ID,
            "player",
            PLAYER_A,
            USER_A,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), status);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let body = response_json(response).await;
    assert_eq!(
        body["error"]["code"],
        if status == StatusCode::SERVICE_UNAVAILABLE {
            "service_unavailable"
        } else {
            "internal_error"
        }
    );
    assert!(body.get("holes").is_none());
    assert!(!body.to_string().contains("synthetic"));
    let actual: i64 = sqlx::query_scalar("SELECT last_value FROM read_attempts")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(actual, attempts);
    assert_eq!(stored_state(&pool).await, before);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
}

#[sqlx::test(migrations = "../migrations")]
async fn repeated_serialization_failure_stops_after_one_retry_with_private_503(pool: PgPool) {
    injected_read_failure(pool, "40001", StatusCode::SERVICE_UNAVAILABLE, 2).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn unrelated_database_error_is_not_retried(pool: PgPool) {
    injected_read_failure(pool, "XX000", StatusCode::INTERNAL_SERVER_ERROR, 1).await;
}
