use super::*;
use golf_api::domain::models::TournamentTieBreakPolicy;

fn policy_patch(token: &str, expected: chrono::DateTime<Utc>, policy: Value) -> Request<Body> {
    Request::patch(format!("/api/tournaments/{TOURNAMENT}/counted-rounds"))
        .header(header::COOKIE, format!("golf_session={token}"))
        .header("x-csrf-token", derive_csrf_token(token))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"counted_rounds":3,"mandatory_round_id":null,
            "tie_break_policy":policy,"expected_tournament_updated_at":expected})
            .to_string(),
        ))
        .unwrap()
}

async fn session(pool: &PgPool) -> Uuid {
    sqlx::query_scalar("SELECT id FROM user_sessions WHERE token_hash=$1")
        .bind(hash_session_token(ADMIN_TOKEN).as_slice())
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn policy_context(tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, actor: Uuid) {
    sqlx::query("SELECT set_config('app.tournament_configuration_tournament_id',$1::text,true),set_config('app.tournament_configuration_user_id',$2::text,true)")
        .bind(TOURNAMENT).bind(actor).execute(&mut **tx).await.unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn tie_policy_api_defaults_preserves_old_clients_and_emits_only_committed_changes(
    pool: PgPool,
) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;
    let get = Request::get(format!("/api/tournaments/{TOURNAMENT}"))
        .header(header::COOKIE, format!("golf_session={ADMIN_TOKEN}"))
        .body(Body::empty())
        .unwrap();
    let response = app.clone().oneshot(get).await.unwrap();
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    assert_eq!(body(response).await["tie_break_policy"], "shared_positions");
    let denied = app
        .clone()
        .oneshot(policy_patch(
            VIEWER_TOKEN,
            expected,
            json!("final_round_score"),
        ))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    for invalid in [Value::Null, json!("countback"), json!(true)] {
        assert_eq!(
            app.clone()
                .oneshot(policy_patch(ADMIN_TOKEN, expected, invalid))
                .await
                .unwrap()
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let mut no_csrf = policy_patch(ADMIN_TOKEN, expected, json!("final_round_score"));
    no_csrf.headers_mut().remove("x-csrf-token");
    assert_eq!(
        app.clone().oneshot(no_csrf).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
    let response = app
        .clone()
        .oneshot(policy_patch(
            ADMIN_TOKEN,
            expected,
            json!("final_round_score"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        body(response).await["tie_break_policy"],
        "final_round_score"
    );
    assert_eq!(events.try_recv().unwrap().resource, "tournament");
    let stale = app
        .clone()
        .oneshot(policy_patch(
            ADMIN_TOKEN,
            expected,
            json!("shared_positions"),
        ))
        .await
        .unwrap();
    assert_eq!(
        body(stale).await["error"]["code"],
        "tournament_configuration_stale"
    );
    let current = updated_at(&pool).await;
    let no_op = app
        .clone()
        .oneshot(patch(ADMIN_TOKEN, 3, current))
        .await
        .unwrap();
    assert_eq!(body(no_op).await["tie_break_policy"], "final_round_score");
    assert_eq!(updated_at(&pool).await, current);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));
    let changed = app
        .clone()
        .oneshot(patch(ADMIN_TOKEN, 2, current))
        .await
        .unwrap();
    assert_eq!(body(changed).await["tie_break_policy"], "final_round_score");
    assert_eq!(events.try_recv().unwrap().resource, "tournament");
    let current = updated_at(&pool).await;
    let cleared = app
        .clone()
        .oneshot(policy_patch(
            ADMIN_TOKEN,
            current,
            json!("shared_positions"),
        ))
        .await
        .unwrap();
    assert_eq!(body(cleared).await["tie_break_policy"], "shared_positions");
    let board = Request::get(format!("/api/tournaments/{TOURNAMENT}/leaderboards/gross"))
        .header(header::COOKIE, format!("golf_session={ADMIN_TOKEN}"))
        .body(Body::empty())
        .unwrap();
    let response = app.oneshot(board).await.unwrap();
    assert_eq!(
        response.headers()[header::CACHE_CONTROL],
        "private, no-store"
    );
    let value = body(response).await;
    assert_eq!(value["tie_break_policy"], "shared_positions");
    assert_eq!(value["final_round_number"], 3);
}

#[sqlx::test(migrations = "../migrations")]
async fn tie_policy_database_guard_requires_context_exact_admin_and_permanent_marker(pool: PgPool) {
    seed(&pool).await;
    let update = "UPDATE tournaments SET tie_break_policy='final_round_score' WHERE id=$1";
    let direct = sqlx::query(update)
        .bind(TOURNAMENT)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        direct.as_database_error().unwrap().constraint(),
        Some("tournament_configuration_context_required")
    );
    let mut tx = pool.begin().await.unwrap();
    policy_context(&mut tx, VIEWER).await;
    let denied = sqlx::query(update)
        .bind(TOURNAMENT)
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(
        denied.as_database_error().unwrap().constraint(),
        Some("tournament_configuration_admin_required")
    );
    tx.rollback().await.unwrap();
    sqlx::query("INSERT INTO tournament_handicap_locks(tournament_id,reason) VALUES($1,'snapshot_captured')")
        .bind(TOURNAMENT).execute(&pool).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    policy_context(&mut tx, ADMIN).await;
    let frozen = sqlx::query(update)
        .bind(TOURNAMENT)
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(
        frozen.as_database_error().unwrap().constraint(),
        Some("tournament_configuration_locked")
    );
    tx.rollback().await.unwrap();
    let result = tournaments::update_counted_rounds_authorized(
        &pool,
        session(&pool).await,
        TOURNAMENT,
        3,
        None,
        Some(TournamentTieBreakPolicy::FinalRoundScore),
        updated_at(&pool).await,
    )
    .await;
    assert!(matches!(
        result,
        Err(tournaments::TournamentMutationError::ConfigurationLocked)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn tie_policy_is_frozen_by_start_and_open_at_api_and_database_boundaries(pool: PgPool) {
    seed(&pool).await;
    make_first_round_openable(&pool).await;
    let session = session(&pool).await;
    tournaments::start_authorized(&pool, session, TOURNAMENT, updated_at(&pool).await)
        .await
        .unwrap();
    for open in [false, true] {
        if open {
            round_lifecycle::open_authorized(
                &pool,
                session,
                uuid!("14000000-0000-0000-0000-000000000011"),
            )
            .await
            .unwrap();
        }
        let app = api::router(AppState::new(pool.clone()));
        let denied = app
            .oneshot(policy_patch(
                ADMIN_TOKEN,
                updated_at(&pool).await,
                json!("final_round_score"),
            ))
            .await
            .unwrap();
        assert_eq!(denied.status(), StatusCode::CONFLICT);
        assert_eq!(
            body(denied).await["error"]["code"],
            "tournament_configuration_locked"
        );
        let mut tx = pool.begin().await.unwrap();
        policy_context(&mut tx, ADMIN).await;
        let denied =
            sqlx::query("UPDATE tournaments SET tie_break_policy='final_round_score' WHERE id=$1")
                .bind(TOURNAMENT)
                .execute(&mut *tx)
                .await
                .unwrap_err();
        assert_eq!(
            denied.as_database_error().unwrap().constraint(),
            Some("tournament_configuration_locked")
        );
        tx.rollback().await.unwrap();
    }
}

async fn wait_for_blocked_writers(pool: &PgPool, count: i64) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock'")
                .fetch_one(pool).await.unwrap();
            if waiting >= count { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.expect("writers never reached expected row locks");
}

async fn contended_start_policy(pool: PgPool, start_first: bool) {
    seed(&pool).await;
    make_first_round_openable(&pool).await;
    let session = session(&pool).await;
    let version = updated_at(&pool).await;
    let mut gate = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR UPDATE")
        .bind(TOURNAMENT)
        .execute(&mut *gate)
        .await
        .unwrap();
    let start_signal = std::sync::Arc::new(tokio::sync::Notify::new());
    let change_signal = std::sync::Arc::new(tokio::sync::Notify::new());
    let start_pool = pool.clone();
    let ready = start_signal.clone();
    let start = tokio::spawn(async move {
        ready.notified().await;
        tournaments::start_authorized(&start_pool, session, TOURNAMENT, version).await
    });
    let change_pool = pool.clone();
    let ready = change_signal.clone();
    let change = tokio::spawn(async move {
        ready.notified().await;
        tournaments::update_counted_rounds_authorized(
            &change_pool,
            session,
            TOURNAMENT,
            3,
            None,
            Some(TournamentTieBreakPolicy::FinalRoundScore),
            version,
        )
        .await
    });
    if start_first {
        start_signal.notify_one();
    } else {
        change_signal.notify_one();
    }
    // First writer owns every round lock and waits on the parent. The second
    // must then wait on a round lock, making the commit order deterministic.
    wait_for_blocked_writers(&pool, 1).await;
    if start_first {
        change_signal.notify_one();
    } else {
        start_signal.notify_one();
    }
    wait_for_blocked_writers(&pool, 2).await;
    gate.commit().await.unwrap();
    let (start, change) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(start, change)
    })
    .await
    .expect("start/policy deadlock");
    if start_first {
        assert!(start.unwrap().unwrap().changed);
        assert!(matches!(
            change.unwrap(),
            Err(tournaments::TournamentMutationError::ConfigurationLocked)
        ));
        assert_eq!(
            tournaments::get(&pool, TOURNAMENT)
                .await
                .unwrap()
                .unwrap()
                .tie_break_policy,
            TournamentTieBreakPolicy::SharedPositions
        );
    } else {
        assert!(matches!(
            start.unwrap(),
            Err(tournaments::TournamentMutationError::StartStale)
        ));
        let changed = change.unwrap().unwrap();
        assert!(changed.changed);
        assert_eq!(
            changed.tournament.tie_break_policy,
            TournamentTieBreakPolicy::FinalRoundScore
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn contended_start_wins_and_freezes_waiting_policy(pool: PgPool) {
    contended_start_policy(pool, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn contended_policy_wins_and_invalidates_waiting_start_version(pool: PgPool) {
    contended_start_policy(pool, false).await;
}

#[sqlx::test(migrations = false)]
async fn schema24_upgrade_defaults_shared_without_rewriting_historical_rows_or_plan_hash(
    pool: PgPool,
) {
    for migration in golf_api::schema::MIGRATOR.iter().filter(|m| m.version < 25) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(include_str!("../../seed.sql"))
        .execute(&pool)
        .await
        .unwrap();
    // Use the schema-24 authorized start context, then the unchanged round and
    // score repositories to retain actual sporting history across this upgrade.
    sqlx::raw_sql("BEGIN; SELECT set_config('app.tournament_start_tournament_id','00000000-0000-0000-0000-000000002001',true),set_config('app.tournament_start_user_id','00000000-0000-0000-0000-000000000001',true); UPDATE tournaments SET status='active' WHERE id='00000000-0000-0000-0000-000000002001'; COMMIT;").execute(&pool).await.unwrap();
    let opened = round_lifecycle::open(&pool, uuid!("00000000-0000-0000-0000-000000004001"))
        .await
        .unwrap();
    assert!(!opened.handicap_snapshots.is_empty());
    // Insert using the historical workflow without decoding a current-schema
    // ScoreEntry, which now requires the revision column introduced in 0027.
    sqlx::raw_sql("BEGIN; SELECT set_config('app.score_mutation_round_id','00000000-0000-0000-0000-000000004001',true); INSERT INTO scores(id,round_id,tournament_id,hole_id,team_id,gross_strokes,submitted_by) VALUES(gen_random_uuid(),'00000000-0000-0000-0000-000000004001','00000000-0000-0000-0000-000000002001','00000000-0000-0000-0000-000000003201','00000000-0000-0000-0000-000000005001',5,'00000000-0000-0000-0000-000000000001'); COMMIT;").execute(&pool).await.unwrap();
    // A retained pre-upgrade retry receipt must survive byte-for-byte too.
    sqlx::query("INSERT INTO tournament_creation_requests(user_id,request_id,request_hash,tournament_id) VALUES('00000000-0000-0000-0000-000000001001',$1,decode(repeat('ab',32),'hex'),'00000000-0000-0000-0000-000000002001')")
        .bind(Uuid::new_v4()).execute(&pool).await.unwrap();
    let query = "SELECT jsonb_build_object('tournaments',(SELECT jsonb_agg(to_jsonb(t) - 'tie_break_policy' ORDER BY id) FROM tournaments t),'scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM scores s),'snapshots',(SELECT jsonb_agg(to_jsonb(h) ORDER BY round_id,player_id) FROM round_handicap_snapshots h),'creations',(SELECT jsonb_agg(to_jsonb(c) ORDER BY request_id) FROM tournament_creation_requests c))";
    let before: Value = sqlx::query_scalar(query).fetch_one(&pool).await.unwrap();
    assert!(!before["scores"].as_array().unwrap().is_empty());
    assert!(!before["snapshots"].as_array().unwrap().is_empty());
    assert!(!before["creations"].as_array().unwrap().is_empty());
    sqlx::raw_sql(include_str!(
        "../../../migrations/0025_tournament_tie_breaks.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let after: Value = sqlx::query_scalar(query).fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT bool_and(tie_break_policy='shared_positions') FROM tournaments"
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    assert!(sqlx::query("INSERT INTO tournaments(name,start_date,end_date,number_of_rounds,tie_break_policy) VALUES('Invalid','2026-09-01','2026-09-01',1,'countback')").execute(&pool).await.is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn visible_attributed_final_only_controls_member_rank_and_explanation(pool: PgPool) {
    use golf_api::{
        domain::scorecards::ScoreOwner,
        repositories::{round_completion, scorecards, tournament_visibility},
    };
    seed(&pool).await;
    make_first_round_openable(&pool).await;
    let player_a = uuid!("14000000-0000-0000-0000-000000000020");
    let player_b = Uuid::new_v4();
    let first = uuid!("14000000-0000-0000-0000-000000000011");
    let final_round = uuid!("14000000-0000-0000-0000-000000000013");
    let tee = uuid!("14000000-0000-0000-0000-000000000022");
    sqlx::query(
        "INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,'Other player',8)",
    )
    .bind(player_b)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES($1,$2,8)").bind(TOURNAMENT).bind(player_b).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id) VALUES('14000000-0000-0000-0000-000000000024',$1,$2,$3)").bind(first).bind(TOURNAMENT).bind(player_b).execute(&pool).await.unwrap();
    for hole in 2..=18_i16 {
        sqlx::query(
            "INSERT INTO holes(id,tee_id,hole_number,par,stroke_index) VALUES($1,$2,$3,4,$3)",
        )
        .bind(Uuid::new_v4())
        .bind(tee)
        .bind(hole)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE tees SET course_rating=72 WHERE id=$1")
        .bind(tee)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET number_of_holes=18,course_id='14000000-0000-0000-0000-000000000021',tee_id=$1,course_name='Course',tee_name='Tee' WHERE id=ANY($2)").bind(tee).bind(vec![first,final_round]).execute(&pool).await.unwrap();
    let flight = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO flights(id,round_id,tournament_id,name) VALUES($1,$2,$3,'Final group')",
    )
    .bind(flight)
    .bind(final_round)
    .bind(TOURNAMENT)
    .execute(&pool)
    .await
    .unwrap();
    for player in [player_a, player_b] {
        sqlx::query("INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(flight).bind(final_round).bind(TOURNAMENT).bind(player).execute(&pool).await.unwrap();
    }
    let session = session(&pool).await;
    tournaments::update_counted_rounds_authorized(
        &pool,
        session,
        TOURNAMENT,
        1,
        None,
        Some(TournamentTieBreakPolicy::FinalRoundScore),
        updated_at(&pool).await,
    )
    .await
    .unwrap();
    tournaments::start_authorized(&pool, session, TOURNAMENT, updated_at(&pool).await)
        .await
        .unwrap();
    let holes =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM holes WHERE tee_id=$1 ORDER BY hole_number")
            .bind(tee)
            .fetch_all(&pool)
            .await
            .unwrap();
    for round in [first, final_round] {
        round_lifecycle::open_authorized(&pool, session, round)
            .await
            .unwrap();
        for player in [player_a, player_b] {
            let owner = ScoreOwner::Player { id: player };
            let strokes = if round == first {
                4
            } else if player == player_a {
                5
            } else {
                6
            };
            for hole in &holes {
                scorecards::save(
                    &pool,
                    scorecards::SaveScore {
                        round_id: round,
                        hole_id: *hole,
                        owner,
                        gross_strokes: strokes,
                        submitted_by: ADMIN,
                    },
                )
                .await
                .unwrap();
            }
            scorecards::confirm(&pool, round, owner, ADMIN)
                .await
                .unwrap();
        }
        round_completion::complete(&pool, round).await.unwrap();
    }
    let app = api::router(AppState::new(pool.clone()));
    for (token, comparable) in [(ADMIN_TOKEN, true), (VIEWER_TOKEN, false)] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/api/tournaments/{TOURNAMENT}/leaderboards/gross"))
                    .header(header::COOKIE, format!("golf_session={token}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        let value = body(response).await;
        assert_eq!(value["tie_break_policy"], "final_round_score");
        assert_eq!(value["final_round_number"], 3);
        let entries = value["entries"].as_array().unwrap();
        assert_eq!(entries.len(), 2);
        if comparable {
            assert_eq!(entries[0]["tie_break_score_to_par"], 18);
            assert_eq!(entries[1]["tie_break_score_to_par"], 36);
            assert_eq!(entries[1]["position"], 2);
        } else {
            assert!(entries.iter().all(|entry| entry["position"] == 1
                && entry["tied"] == true
                && entry["tie_break_score_to_par"].is_null()
                && entry["contributions"].as_array().unwrap().len() == 1));
        }
    }
    let version = tournament_visibility::get_for_admin(&pool, ADMIN, TOURNAMENT)
        .await
        .unwrap()
        .visibility_updated_at;
    tournament_visibility::update_authorized(&pool, session, TOURNAMENT, false, version)
        .await
        .unwrap();
    let response = app
        .oneshot(
            Request::get(format!("/api/tournaments/{TOURNAMENT}/leaderboards/gross"))
                .header(header::COOKIE, format!("golf_session={VIEWER_TOKEN}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        body(response).await["entries"][1]["tie_break_score_to_par"],
        36
    );
}
