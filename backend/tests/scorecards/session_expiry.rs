use super::*;

#[derive(Clone, Copy)]
enum Operation {
    NewSave,
    Correction,
    UnchangedSave,
    FirstConfirmation,
    RepeatConfirmation,
}

async fn prepare(pool: &PgPool, operation: Operation) -> Request<Body> {
    seed_open(pool).await;
    let owner = ScoreOwner::Player { id: PLAYER_A };
    if !matches!(operation, Operation::NewSave) {
        for hole_id in [HOLE_1, HOLE_2] {
            scorecards::save(
                pool,
                scorecards::SaveScore {
                    round_id: INDIVIDUAL_ROUND_ID,
                    hole_id,
                    owner,
                    gross_strokes: 4,
                    submitted_by: USER_B,
                },
            )
            .await
            .unwrap();
        }
        if !matches!(operation, Operation::FirstConfirmation) {
            scorecards::confirm(pool, INDIVIDUAL_ROUND_ID, owner, USER_B)
                .await
                .unwrap();
        }
    }
    match operation {
        Operation::FirstConfirmation | Operation::RepeatConfirmation => {
            confirm_request(INDIVIDUAL_ROUND_ID, "player", PLAYER_A, USER_A)
        }
        _ => save_request(
            INDIVIDUAL_ROUND_ID,
            HOLE_1,
            json!({"type": "player", "id": PLAYER_A}),
            if matches!(operation, Operation::UnchangedSave) {
                4
            } else {
                5
            },
            USER_A,
        ),
    }
}

// Compare complete rows, including actor, timestamps, revisions and confirmation
// identity: counting rows alone would miss a correction or a no-op rewrite.
async fn stored_state(pool: &PgPool) -> Value {
    sqlx::query_scalar(
        "SELECT jsonb_build_object(
          'scores', (SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY s.id), '[]') FROM scores s),
          'audits', (SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY a.id), '[]') FROM score_audits a),
          'confirmations', (SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY c.id), '[]') FROM scorecard_confirmations c))",
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn after_membership_wait(pool: PgPool, operation: Operation, expire: bool) {
    let request = prepare(&pool, operation).await;
    let before = stored_state(&pool).await;
    let state = AppState::new(pool.clone());
    let app = api::router(Arc::clone(&state));
    let mut events = state.live_events.subscribe();
    let mut gate = pool.begin().await.unwrap();
    let gate_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *gate)
        .await
        .unwrap();
    sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR UPDATE")
        .bind(TOURNAMENT_ID).bind(USER_A).execute(&mut *gate).await.unwrap();
    if expire {
        sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '2 seconds' WHERE user_id=$1")
            .bind(USER_A).execute(&pool).await.unwrap();
    }
    let job = tokio::spawn(async move { app.oneshot(request).await.unwrap() });
    // Observe this gate's actual waiter, rather than sleeping and hoping the
    // request has reached authorization. No other query waits on this gate.
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND $1=ANY(pg_blocking_pids(pid)))")
                .bind(gate_pid).fetch_one(&pool).await.unwrap();
            if blocked { break; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.expect("request must reach membership lock before expiry");
    let live: bool = sqlx::query_scalar(
        "SELECT expires_at>clock_timestamp() FROM user_sessions WHERE user_id=$1",
    )
    .bind(USER_A)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(
        live,
        "session must still be valid when membership wait is observed"
    );
    if expire {
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let expired: bool = sqlx::query_scalar(
                    "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE user_id=$1",
                )
                .bind(USER_A)
                .fetch_one(&pool)
                .await
                .unwrap();
                if expired {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
    gate.commit().await.unwrap();
    let response = tokio::time::timeout(Duration::from_secs(3), job)
        .await
        .unwrap()
        .unwrap();
    if expire {
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response_json(response).await["error"]["code"],
            "unauthenticated"
        );
        assert_eq!(stored_state(&pool).await, before);
        assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
    } else {
        assert_eq!(response.status(), StatusCode::OK);
        if matches!(
            operation,
            Operation::UnchangedSave | Operation::RepeatConfirmation
        ) {
            assert_eq!(stored_state(&pool).await, before);
        } else {
            assert_ne!(stored_state(&pool).await, before);
            assert_eq!(events.try_recv().unwrap().id, INDIVIDUAL_ROUND_ID);
        }
        assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
    }
}

macro_rules! case {
    ($name:ident, $operation:ident, $expire:expr) => {
        #[sqlx::test(migrations = "../migrations")]
        async fn $name(pool: PgPool) {
            after_membership_wait(pool, Operation::$operation, $expire).await;
        }
    };
}
case!(expired_new_save, NewSave, true);
case!(expired_correction, Correction, true);
case!(expired_unchanged_save, UnchangedSave, true);
case!(expired_first_confirmation, FirstConfirmation, true);
case!(expired_repeat_confirmation, RepeatConfirmation, true);
case!(valid_new_save, NewSave, false);
case!(valid_correction, Correction, false);
case!(valid_unchanged_save, UnchangedSave, false);
case!(valid_first_confirmation, FirstConfirmation, false);
case!(valid_repeat_confirmation, RepeatConfirmation, false);
