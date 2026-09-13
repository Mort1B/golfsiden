use super::*;
use golf_api::domain::scorecards::{ExpectedScore, ScoreRevision};
use golf_api::repositories::scorecards::{ConditionalSaveScore, ScorecardConflict, ScorecardError};

async fn session_id(pool: &PgPool, user: Uuid) -> Uuid {
    sqlx::query_scalar("SELECT id FROM user_sessions WHERE token_hash=$1")
        .bind(hash_session_token(token_for_user(user)).as_slice())
        .fetch_one(pool)
        .await
        .unwrap()
}
fn operation(session: Uuid) -> ConditionalSaveScore {
    ConditionalSaveScore {
        request_id: Uuid::new_v4(),
        round_id: INDIVIDUAL_ROUND_ID,
        hole_id: HOLE_1,
        owner: ScoreOwner::Player { id: PLAYER_A },
        gross_strokes: 5,
        expected_score: ExpectedScore::Absent {},
        session_id: session,
    }
}
fn conditional_request(input: &ConditionalSaveScore, user: Uuid) -> Request<Body> {
    let mut value = serde_json::to_value(input).unwrap();
    value.as_object_mut().unwrap().remove("round_id");
    Request::put(format!("/api/rounds/{}/scores/conditional", input.round_id))
        .header("content-type", "application/json")
        .header("cookie", format!("golf_session={}", token_for_user(user)))
        .header("x-csrf-token", derive_csrf_token(token_for_user(user)))
        .body(Body::from(value.to_string()))
        .unwrap()
}
async fn deliver(app: &axum::Router, input: &ConditionalSaveScore) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(conditional_request(input, USER_A))
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    (response.status(), response_json(response).await)
}
fn expected(value: &Value) -> ExpectedScore {
    ExpectedScore::Present {
        score_id: serde_json::from_value(value["applied_score"]["score_id"].clone()).unwrap(),
        revision: serde_json::from_value(value["applied_score"]["revision"].clone()).unwrap(),
    }
}
async fn legacy(pool: &PgPool, gross: i16) -> golf_api::domain::scorecards::ScoreEntry {
    scorecards::save(
        pool,
        scorecards::SaveScore {
            round_id: INDIVIDUAL_ROUND_ID,
            hole_id: HOLE_1,
            owner: ScoreOwner::Player { id: PLAYER_A },
            gross_strokes: gross,
            submitted_by: USER_A,
        },
    )
    .await
    .unwrap()
    .value
}
async fn wait_locks(pool: &PgPool, n: i64) {
    tokio::time::timeout(Duration::from_secs(3),async{
        loop{let count:i64=sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock'").fetch_one(pool).await.unwrap();
        if count>=n {break;}tokio::time::sleep(Duration::from_millis(10)).await;}
    }).await.expect("expected blocked writers");
}
#[sqlx::test(migrations = "../migrations")]
async fn lost_response_retry_acknowledges_original_revision_after_external_write(pool: PgPool) {
    seed_open(&pool).await;
    let input = operation(session_id(&pool, USER_A).await);
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let (status, first) = deliver(&app, &input).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["request_id"], input.request_id.to_string());
    assert_eq!(first["applied_score"]["revision"], "1");
    assert_eq!(first.as_object().unwrap().len(), 2);
    events.try_recv().unwrap();
    let newer = legacy(&pool, 7).await;
    assert_eq!(newer.revision.as_i64(), 2);
    assert_eq!(deliver(&app, &input).await, (StatusCode::OK, first.clone()));
    assert!(events.try_recv().is_err());
    let card = app
        .clone()
        .oneshot(scoring_request(
            INDIVIDUAL_ROUND_ID,
            "player",
            PLAYER_A,
            USER_A,
        ))
        .await
        .unwrap();
    let card = response_json(card).await;
    assert_eq!(card["holes"][0]["score"]["gross_strokes"], 7);
    assert_eq!(card["holes"][0]["score"]["revision"], "2");
    let read = app
        .clone()
        .oneshot(scorecard_request(
            INDIVIDUAL_ROUND_ID,
            "player",
            PLAYER_A,
            USER_A,
        ))
        .await
        .unwrap();
    assert!(
        response_json(read).await["holes"][0]["score"]
            .get("revision")
            .is_none()
    );
    let mut successor = input.clone();
    successor.request_id = Uuid::new_v4();
    successor.expected_score = expected(&first);
    successor.gross_strokes = 8;
    let denied = deliver(&app, &successor).await;
    assert_eq!(denied.0, StatusCode::CONFLICT);
    assert_eq!(denied.1["error"]["code"], "score_version_conflict");
    successor.request_id = Uuid::new_v4();
    successor.expected_score = ExpectedScore::Present {
        score_id: newer.id,
        revision: newer.revision,
    };
    assert_eq!(
        deliver(&app, &successor).await.1["applied_score"]["revision"],
        "3"
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn stale_absence_equal_value_wrong_identity_and_aba_are_conflicts(pool: PgPool) {
    seed_open(&pool).await;
    let mut input = operation(session_id(&pool, USER_A).await);
    let first = legacy(&pool, 5).await;
    assert!(matches!(
        scorecards::save_conditional(&pool, input.clone()).await,
        Err(ScorecardError::Conflict(ScorecardConflict::VersionConflict))
    ));
    input.expected_score = ExpectedScore::Present {
        score_id: Uuid::new_v4(),
        revision: first.revision,
    };
    assert!(
        scorecards::save_conditional(&pool, input.clone())
            .await
            .is_err()
    );
    input.expected_score = ExpectedScore::Present {
        score_id: first.id,
        revision: first.revision,
    };
    legacy(&pool, 6).await;
    let third = legacy(&pool, 5).await;
    assert_eq!(third.revision.as_i64(), 3);
    assert!(matches!(
        scorecards::save_conditional(&pool, input).await,
        Err(ScorecardError::Conflict(ScorecardConflict::VersionConflict))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM score_mutation_receipts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn same_operation_is_account_scoped_payload_immutable_and_session_independent(pool: PgPool) {
    seed_open(&pool).await;
    let input = operation(session_id(&pool, USER_A).await);
    let first = scorecards::save_conditional(&pool, input.clone())
        .await
        .unwrap();
    for change in 0..3 {
        let mut changed = input.clone();
        match change {
            0 => changed.gross_strokes = 6,
            1 => changed.hole_id = HOLE_2,
            _ => {
                changed.expected_score = ExpectedScore::Present {
                    score_id: first.value.applied_score.score_id,
                    revision: first.value.applied_score.revision,
                }
            }
        };
        assert!(matches!(
            scorecards::save_conditional(&pool, changed).await,
            Err(ScorecardError::Conflict(ScorecardConflict::RequestMismatch))
        ));
    }
    let new_session = auth::create_session(
        &pool,
        USER_A,
        &hash_session_token("new-session-same-user"),
        Utc::now() + ChronoDuration::hours(1),
    )
    .await
    .unwrap();
    let mut again = input.clone();
    again.session_id = new_session.session_id;
    let replay = scorecards::save_conditional(&pool, again).await.unwrap();
    assert_eq!(replay.value, first.value);
    assert!(!replay.changed);
    let mut other = input;
    other.session_id = session_id(&pool, USER_B).await;
    other.expected_score = ExpectedScore::Present {
        score_id: first.value.applied_score.score_id,
        revision: first.value.applied_score.revision,
    };
    assert!(
        !scorecards::save_conditional(&pool, other)
            .await
            .unwrap()
            .changed
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM score_mutation_receipts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn conditional_same_value_records_receipt_without_invalidating_confirmation_audit_or_sse(
    pool: PgPool,
) {
    seed_open(&pool).await;
    let first = legacy(&pool, 5).await;
    scorecards::save(
        &pool,
        scorecards::SaveScore {
            round_id: INDIVIDUAL_ROUND_ID,
            hole_id: HOLE_2,
            owner: ScoreOwner::Player { id: PLAYER_A },
            gross_strokes: 4,
            submitted_by: USER_A,
        },
    )
    .await
    .unwrap();
    scorecards::confirm(
        &pool,
        INDIVIDUAL_ROUND_ID,
        ScoreOwner::Player { id: PLAYER_A },
        USER_A,
    )
    .await
    .unwrap();
    let mut input = operation(session_id(&pool, USER_A).await);
    input.expected_score = ExpectedScore::Present {
        score_id: first.id,
        revision: first.revision,
    };
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let first_ack = deliver(&app, &input).await;
    assert_eq!(first_ack.0, StatusCode::OK);
    assert_eq!(deliver(&app, &input).await, first_ack);
    assert!(events.try_recv().is_err());
    let score = scorecards::get(&pool, INDIVIDUAL_ROUND_ID, input.owner)
        .await
        .unwrap();
    assert!(score.confirmed);
    assert_eq!(
        score.holes[0].score.as_ref().unwrap().updated_at,
        first.updated_at
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM score_audits")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn concurrent_duplicates_acknowledge_once_under_round_lock(pool: PgPool) {
    seed_open(&pool).await;
    let input = operation(session_id(&pool, USER_A).await);
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(INDIVIDUAL_ROUND_ID)
        .execute(&mut *gate)
        .await
        .unwrap();
    let mut jobs = Vec::new();
    for _ in 0..2 {
        let p = pool.clone();
        let i = input.clone();
        jobs.push(tokio::spawn(async move {
            scorecards::save_conditional(&p, i).await
        }));
    }
    wait_locks(&pool, 2).await;
    gate.commit().await.unwrap();
    let mut results = Vec::new();
    for job in jobs {
        results.push(
            tokio::time::timeout(Duration::from_secs(3), job)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
        );
    }
    assert_eq!(results[0].value, results[1].value);
    assert_eq!(results.iter().filter(|r| r.changed).count(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM score_audits")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn same_request_id_racing_across_rounds_rolls_back_losing_score_and_audit(pool: PgPool) {
    seed_open(&pool).await;
    let mut first = operation(session_id(&pool, USER_A).await);
    let mut second = first.clone();
    second.round_id = SCRAMBLE_ROUND_ID;
    second.owner = ScoreOwner::Team {
        id: SCRAMBLE_TEAM_ID,
    };
    for input in [&mut first, &mut second] {
        for hole in [HOLE_1, HOLE_2] {
            let saved = scorecards::save(
                &pool,
                scorecards::SaveScore {
                    round_id: input.round_id,
                    hole_id: hole,
                    owner: input.owner,
                    gross_strokes: 4,
                    submitted_by: USER_A,
                },
            )
            .await
            .unwrap();
            if hole == HOLE_1 {
                input.expected_score = ExpectedScore::Present {
                    score_id: saved.value.id,
                    revision: saved.value.revision,
                };
            }
        }
        scorecards::confirm(&pool, input.round_id, input.owner, USER_A)
            .await
            .unwrap();
    }
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE score_mutation_receipts IN SHARE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let mut jobs = Vec::new();
    for input in [first, second] {
        let p = pool.clone();
        jobs.push(tokio::spawn(async move {
            scorecards::save_conditional(&p, input).await
        }));
    }
    wait_locks(&pool, 2).await;
    gate.commit().await.unwrap();
    let mut won = 0;
    let mut mismatch = 0;
    for job in jobs {
        match tokio::time::timeout(Duration::from_secs(3), job)
            .await
            .unwrap()
            .unwrap()
        {
            Ok(_) => won += 1,
            Err(ScorecardError::Conflict(ScorecardConflict::RequestMismatch)) => mismatch += 1,
            _ => panic!("unexpected cross-round result"),
        }
    }
    assert_eq!((won, mismatch), (1, 1));
    for (table, count) in [
        ("scores", 4),
        ("score_audits", 5),
        ("score_mutation_receipts", 1),
        ("scorecard_confirmations", 1),
    ] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) FROM {table}"))
                .fetch_one(&pool)
                .await
                .unwrap(),
            count
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn cached_receipt_rechecks_membership_session_and_locked_round(pool: PgPool) {
    seed_open(&pool).await;
    let input = operation(session_id(&pool, USER_A).await);
    scorecards::save_conditional(&pool, input.clone())
        .await
        .unwrap();
    sqlx::query(
        "UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1 AND user_id=$2",
    )
    .bind(TOURNAMENT_ID)
    .bind(USER_A)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        scorecards::save_conditional(&pool, input.clone()).await,
        Err(ScorecardError::Forbidden)
    ));
    sqlx::query(
        "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
    )
    .bind(TOURNAMENT_ID)
    .bind(USER_A)
    .execute(&pool)
    .await
    .unwrap();
    for player in [PLAYER_A, PLAYER_B] {
        for hole in [HOLE_1, HOLE_2] {
            scorecards::save(
                &pool,
                scorecards::SaveScore {
                    round_id: INDIVIDUAL_ROUND_ID,
                    hole_id: hole,
                    owner: ScoreOwner::Player { id: player },
                    gross_strokes: 5,
                    submitted_by: USER_A,
                },
            )
            .await
            .unwrap();
        }
        scorecards::confirm(
            &pool,
            INDIVIDUAL_ROUND_ID,
            ScoreOwner::Player { id: player },
            USER_A,
        )
        .await
        .unwrap();
    }
    round_completion::complete(&pool, INDIVIDUAL_ROUND_ID)
        .await
        .unwrap();
    assert!(
        !scorecards::save_conditional(&pool, input.clone())
            .await
            .unwrap()
            .changed
    );
    round_completion::lock(&pool, INDIVIDUAL_ROUND_ID)
        .await
        .unwrap();
    assert!(matches!(
        scorecards::save_conditional(&pool, input.clone()).await,
        Err(ScorecardError::Conflict(
            ScorecardConflict::RoundNotEditable
        ))
    ));
    let mut other = input;
    other.request_id = Uuid::new_v4();
    other.round_id = SCRAMBLE_ROUND_ID;
    other.owner = ScoreOwner::Team {
        id: SCRAMBLE_TEAM_ID,
    };
    scorecards::save_conditional(&pool, other.clone())
        .await
        .unwrap();
    auth::revoke_session(&pool, other.session_id).await.unwrap();
    assert!(matches!(
        scorecards::save_conditional(&pool, other).await,
        Err(ScorecardError::Unauthenticated)
    ));
}

async fn expiry_while_waiting(pool: PgPool, replay: bool) {
    seed_open(&pool).await;
    let input = operation(session_id(&pool, USER_A).await);
    if replay {
        scorecards::save_conditional(&pool, input.clone())
            .await
            .unwrap();
    }
    sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '300 milliseconds' WHERE id=$1").bind(input.session_id).execute(&pool).await.unwrap();
    let mut gate = pool.begin().await.unwrap();
    // The session was checked before this membership wait. The final wall-clock
    // check must reject both fresh writes and previously applied receipt hits.
    sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2 FOR UPDATE").bind(TOURNAMENT_ID).bind(USER_A).execute(&mut *gate).await.unwrap();
    let p = pool.clone();
    let job = tokio::spawn(async move { scorecards::save_conditional(&p, input).await });
    wait_locks(&pool, 1).await;
    tokio::time::sleep(Duration::from_millis(350)).await;
    gate.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), job)
            .await
            .unwrap()
            .unwrap(),
        Err(ScorecardError::Unauthenticated)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM score_mutation_receipts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        i64::from(replay)
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn fresh_delivery_expiring_after_authorization_wait_writes_nothing(pool: PgPool) {
    expiry_while_waiting(pool, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn receipt_replay_expiring_after_authorization_wait_is_rejected(pool: PgPool) {
    expiry_while_waiting(pool, true).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn direct_sql_and_corrections_advance_revisions_but_cannot_forge_them_or_receipts(
    pool: PgPool,
) {
    seed_open(&pool).await;
    let score = legacy(&pool, 5).await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true)")
        .bind(INDIVIDUAL_ROUND_ID)
        .execute(&mut *tx)
        .await
        .unwrap();
    let revision: i64 =
        sqlx::query_scalar("UPDATE scores SET gross_strokes=6 WHERE id=$1 RETURNING revision")
            .bind(score.id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(revision, 2);
    let noop: i64 =
        sqlx::query_scalar("UPDATE scores SET gross_strokes=6 WHERE id=$1 RETURNING revision")
            .bind(score.id)
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(noop, 2);
    tx.commit().await.unwrap();
    for revision in [0, 1, 3, i64::MAX] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true)")
            .bind(INDIVIDUAL_ROUND_ID)
            .execute(&mut *tx)
            .await
            .unwrap();
        let error = sqlx::query("UPDATE scores SET revision=$2 WHERE id=$1")
            .bind(score.id)
            .bind(revision)
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().constraint(),
            Some("score_revision_managed")
        );
    }
    let mut input = operation(session_id(&pool, USER_A).await);
    input.gross_strokes = 6;
    input.expected_score = ExpectedScore::Present {
        score_id: score.id,
        revision: ScoreRevision::from_database(2).unwrap(),
    };
    scorecards::save_conditional(&pool, input.clone())
        .await
        .unwrap();
    for sql in [
        "UPDATE score_mutation_receipts SET applied_revision=99 WHERE request_id=$1",
        "DELETE FROM score_mutation_receipts WHERE request_id=$1",
    ] {
        assert!(
            sqlx::query(sql)
                .bind(input.request_id)
                .execute(&pool)
                .await
                .is_err()
        );
    }
    assert!(sqlx::query("INSERT INTO score_mutation_receipts(user_id,request_id,round_id,request_hash,applied_score_id,applied_revision) VALUES($1,$2,$3,decode(repeat('00',32),'hex'),$4,2)").bind(USER_A).bind(Uuid::new_v4()).bind(INDIVIDUAL_ROUND_ID).bind(score.id).execute(&pool).await.is_err());
    // Complete and lock using the normal workflow, then exercise the existing
    // explicit administrator SQL correction context on the same preserved score.
    for player in [PLAYER_A, PLAYER_B] {
        for hole in [HOLE_1, HOLE_2] {
            if player == PLAYER_A && hole == HOLE_1 {
                continue;
            }
            scorecards::save(
                &pool,
                scorecards::SaveScore {
                    round_id: INDIVIDUAL_ROUND_ID,
                    hole_id: hole,
                    owner: ScoreOwner::Player { id: player },
                    gross_strokes: 4,
                    submitted_by: USER_A,
                },
            )
            .await
            .unwrap();
        }
        scorecards::confirm(
            &pool,
            INDIVIDUAL_ROUND_ID,
            ScoreOwner::Player { id: player },
            USER_A,
        )
        .await
        .unwrap();
    }
    round_completion::complete(&pool, INDIVIDUAL_ROUND_ID)
        .await
        .unwrap();
    round_completion::lock(&pool, INDIVIDUAL_ROUND_ID)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true),set_config('app.admin_correction','true',true)").bind(INDIVIDUAL_ROUND_ID).execute(&mut *tx).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "UPDATE scores SET gross_strokes=7 WHERE id=$1 RETURNING revision"
        )
        .bind(score.id)
        .fetch_one(&mut *tx)
        .await
        .unwrap(),
        3
    );
    tx.commit().await.unwrap();
    sqlx::query("DELETE FROM tournaments WHERE id=$1")
        .bind(TOURNAMENT_ID)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM score_mutation_receipts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn conditional_api_rejects_missing_expected_numeric_revision_csrf_and_unauthorized_owner(
    pool: PgPool,
) {
    seed_open(&pool).await;
    let input = operation(session_id(&pool, USER_A).await);
    let app = api::router(AppState::new(pool.clone()));
    let mut csrf = conditional_request(&input, USER_A);
    csrf.headers_mut().remove("x-csrf-token");
    assert_eq!(
        app.clone().oneshot(csrf).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.clone()
            .oneshot(conditional_request(&input, VIEWER_USER))
            .await
            .unwrap()
            .status(),
        StatusCode::FORBIDDEN
    );
    for expected in [
        Value::Null,
        json!({"type":"present","score_id":Uuid::new_v4(),"revision":1}),
        json!({"type":"absent","extra":true}),
    ] {
        let value = json!({"request_id":input.request_id,"hole_id":HOLE_1,"owner":input.owner,"gross_strokes":5,"expected_score":expected});
        let mut request = conditional_request(&input, USER_A);
        *request.body_mut() = Body::from(value.to_string());
        assert_eq!(
            app.clone().oneshot(request).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM scores")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_after_receipt_insert_wait_rolls_back_score_and_audit(pool: PgPool) {
    seed_open(&pool).await;
    let input = operation(session_id(&pool, USER_A).await);
    sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '300 milliseconds' WHERE id=$1").bind(input.session_id).execute(&pool).await.unwrap();
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE score_mutation_receipts IN SHARE MODE")
        .execute(&mut *gate)
        .await
        .unwrap();
    let p = pool.clone();
    let job = tokio::spawn(async move { scorecards::save_conditional(&p, input).await });
    wait_locks(&pool, 1).await;
    tokio::time::sleep(Duration::from_millis(350)).await;
    gate.commit().await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(3), job)
            .await
            .unwrap()
            .unwrap(),
        Err(ScorecardError::Unauthenticated)
    ));
    for table in ["scores", "score_audits", "score_mutation_receipts"] {
        assert_eq!(
            sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) FROM {table}"))
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
    }
}
