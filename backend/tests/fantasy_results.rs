#![cfg(feature = "database-tests")]
mod fantasy_results_support;
use fantasy_results_support::*;
#[sqlx::test(migrations = "../migrations")]
async fn individual_net_dnf_finality_captain_corrections_and_all_rounds(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let input = lineup(&f);
    fantasy::save(&pool, f.base.session, f.base.tournament, r, &input)
        .await
        .unwrap();
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    let h = holes(&pool, r).await;
    numeric(
        &pool,
        &f,
        r,
        ScoreOwner::Player { id: f.players[0] },
        h[0],
        3,
    )
    .await;
    let value = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&value, f.players[0])["points"],
        json!({"state":"pending","recorded":3})
    );
    assert_eq!(
        golfer(&value, f.players[0])["holes"][0]["category"],
        "eagle"
    );
    fill(&pool, &f, r, ScoreOwner::Player { id: f.players[0] }, 4).await;
    fill(&pool, &f, r, ScoreOwner::Player { id: f.players[1] }, 8).await;
    let value = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&value, f.players[0])["points"],
        json!({"state":"provisional","total":22})
    );
    for p in &f.players[2..] {
        disposition(&pool, &f, r, OwnerKind::Player, *p, false).await;
    }
    let value = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&value, f.players[0])["points"],
        json!({"state":"settled","total":22})
    );
    assert_eq!(
        golfer(&value, f.players[1])["points"],
        json!({"state":"settled","total":-58})
    );
    assert_eq!(
        manager(&value, f.base.admin)["points"],
        json!({"state":"settled","total":-94})
    );
    assert_eq!(
        golfer(&value, f.players[2])["holes"][0]["points"]["state"],
        "omitted_non_finish"
    );
    assert_eq!(value["round"]["sporting_status"], "open");
    let overall = serde_json::to_value(
        results::overall(&pool, f.base.session, f.base.tournament)
            .await
            .unwrap(),
    )
    .unwrap();
    let row = overall["golfers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|g| g["id"] == f.players[0].to_string())
        .unwrap();
    assert_eq!(row["points"], json!({"state":"provisional","total":22}));
    assert_eq!(row["rounds"].as_array().unwrap().len(), 3);
    // Current handicap edits cannot change a preserved result.
    sqlx::query("UPDATE players SET current_handicap_index=54 WHERE id=$1")
        .bind(f.players[0])
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(scores(&pool, &f, r, f.base.session).await, value);
    numeric(
        &pool,
        &f,
        r,
        ScoreOwner::Player { id: f.players[2] },
        h[0],
        4,
    )
    .await;
    let changed = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&changed, f.players[2])["settlement"],
        "stale_non_finish"
    );
    assert_eq!(
        golfer(&changed, f.players[2])["points"],
        json!({"state":"pending","recorded":1})
    );
    assert_eq!(
        golfer(&changed, f.players[2])["holes"][1]["points"]["state"],
        "pending"
    );
    assert_ne!(value["revision"], changed["revision"]);
    disposition(&pool, &f, r, OwnerKind::Player, f.players[2], true).await;
    let settled = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&settled, f.players[2])["points"],
        json!({"state":"settled","total":1})
    );
    assert_scoped_parity(&pool, &f, r, f.base.session).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn future_rounds_prelock_privacy_no_false_rank_and_no_participation(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    fantasy::save(&pool, f.member_session, f.base.tournament, r, &lineup(&f))
        .await
        .unwrap();
    let value = scores(&pool, &f, r, f.base.session).await;
    assert!(manager(&value, f.base.other)["lineup"].is_null());
    assert!(
        value["golfers"]
            .as_array()
            .unwrap()
            .iter()
            .all(|g| g["rank"].is_null() && g["points"]["state"] == "not_started")
    );
    let details = serde_json::to_value(
        results::manager_breakdown(&pool, f.base.session, f.base.tournament, f.base.other)
            .await
            .unwrap(),
    )
    .unwrap();
    assert!(details["rounds"][0]["result"]["lineup"].is_null());
    assert_eq!(details["standing"]["points"]["state"], "not_started");
    assert!(details["standing"]["rank"].is_null());
    close(&pool, &f, r).await;
    assert_eq!(
        scores(&pool, &f, r, f.base.session).await["round"]["points"]["state"],
        "not_started"
    );
    source_fixture(&pool, &f, r).await;
    let new = prepared(&pool, &f.base).await.0.player_id;
    let value = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(golfer(&value, new)["points"]["state"], "not_participating");
    assert!(golfer(&value, new)["rank"].is_null());
}
#[sqlx::test(migrations = "../migrations")]
async fn hidden_final_full_json_invariant_and_release(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[2];
    fantasy::save(&pool, f.base.session, f.base.tournament, r, &lineup(&f))
        .await
        .unwrap();
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    hide(&pool, &f, true).await;
    let h = holes(&pool, r).await;
    numeric(
        &pool,
        &f,
        r,
        ScoreOwner::Player { id: f.players[0] },
        h[0],
        4,
    )
    .await;
    let before = scores(&pool, &f, r, f.member_session).await;
    let total_before = serde_json::to_value(
        results::overall(&pool, f.member_session, f.base.tournament)
            .await
            .unwrap(),
    )
    .unwrap();
    let detail_before = serde_json::to_value(
        results::golfer_breakdown(&pool, f.member_session, f.base.tournament, f.players[0])
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(golfer(&before, f.players[0])["points"]["state"], "withheld");
    assert_eq!(golfer(&before, f.players[0])["recorded_hole_points"], 1);
    assert!(before["round"]["sporting_status"].is_null());
    numeric(
        &pool,
        &f,
        r,
        ScoreOwner::Player { id: f.players[0] },
        h[17],
        1,
    )
    .await;
    disposition(&pool, &f, r, OwnerKind::Player, f.players[0], false).await;
    assert_eq!(scores(&pool, &f, r, f.member_session).await, before);
    assert_eq!(
        serde_json::to_value(
            results::overall(&pool, f.member_session, f.base.tournament)
                .await
                .unwrap()
        )
        .unwrap(),
        total_before
    );
    assert_eq!(
        serde_json::to_value(
            results::golfer_breakdown(&pool, f.member_session, f.base.tournament, f.players[0])
                .await
                .unwrap()
        )
        .unwrap(),
        detail_before
    );
    let admin = scores(&pool, &f, r, f.base.session).await;
    assert_eq!(
        golfer(&admin, f.players[0])["points"],
        json!({"state":"settled","total":21})
    );
    assert_scoped_parity(&pool, &f, r, f.member_session).await;
    hide(&pool, &f, false).await;
    assert_eq!(
        golfer(&scores(&pool, &f, r, f.member_session).await, f.players[0])["points"],
        json!({"state":"settled","total":21})
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn scoped_not_found_rolls_back_lazy_closure_and_member_api_contract(pool: PgPool) {
    let f = fixture(&pool).await;
    let other_tournament = Uuid::new_v4();
    let foreign_round = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES($1,'Other trip','2026-10-01','2026-10-03',1,1)").bind(other_tournament).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES($1,$2,1,'Other round','2026-10-01','','','individual_stroke_play')").bind(foreign_round).bind(other_tournament).execute(&pool).await.unwrap();
    let r = f.rounds[0];
    close(&pool, &f, r).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    for path in [
        format!(
            "/api/tournaments/{}/fantasy/rounds/{foreign_round}/results",
            f.base.tournament
        ),
        format!(
            "/api/tournaments/{}/fantasy/rounds/{}/results",
            f.base.tournament,
            Uuid::new_v4()
        ),
        format!(
            "/api/tournaments/{}/fantasy/results/golfers/{}",
            f.base.tournament,
            Uuid::new_v4()
        ),
        format!(
            "/api/tournaments/{}/fantasy/results/managers/{}",
            f.base.tournament,
            Uuid::new_v4()
        ),
    ] {
        let response = app
            .clone()
            .oneshot(request("GET", &path, Some(ADMIN_TOKEN), json!(null)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_selections")
                .fetch_one(&pool)
                .await
                .unwrap(),
            0
        );
        assert!(events.try_recv().is_err());
    }
    let path = format!("/api/tournaments/{}/fantasy/results", f.base.tournament);
    let response = app
        .clone()
        .oneshot(request("GET", &path, Some(ADMIN_TOKEN), json!(null)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let event = events.try_recv().unwrap();
    assert_eq!(event.tournament_id, f.base.tournament);
    assert_eq!(event.id, f.base.tournament);
    sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(f.member_session)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        app.oneshot(request("GET", &path, Some("fantasy-member"), json!(null)))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn valid_round_read_finalizes_other_due_round_once(pool: PgPool) {
    let f = fixture(&pool).await;
    let due = f.rounds[0];
    let target = f.rounds[1];
    close(&pool, &f, due).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let path = format!(
        "/api/tournaments/{}/fantasy/rounds/{target}/results",
        f.base.tournament
    );
    for first in [true, false] {
        let response = app
            .clone()
            .oneshot(request("GET", &path, Some(ADMIN_TOKEN), json!(null)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        if first {
            let event = events.try_recv().unwrap();
            assert_eq!(event.tournament_id, f.base.tournament);
            assert_eq!(event.id, f.base.tournament);
        }
        assert!(events.try_recv().is_err());
    }
    let locked: Vec<Uuid> = sqlx::query_scalar(
        "SELECT round_id FROM fantasy_rounds WHERE locked_at IS NOT NULL ORDER BY round_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(locked, vec![due]);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM fantasy_selections WHERE round_id=$1 AND state='missed'"
        )
        .bind(due)
        .fetch_one(&pool)
        .await
        .unwrap(),
        2
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn round_result_rejects_revocation_while_waiting_without_lazy_commit(pool: PgPool) {
    let f = fixture(&pool).await;
    close(&pool, &f, f.rounds[0]).await;
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(f.rounds[0])
        .execute(&mut *holder)
        .await
        .unwrap();
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let path = format!(
        "/api/tournaments/{}/fantasy/rounds/{}/results",
        f.base.tournament, f.rounds[1]
    );
    let task = tokio::spawn(async move {
        app.oneshot(request("GET", &path, Some("fantasy-member"), json!(null)))
            .await
            .unwrap()
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'SELECT id FROM rounds WHERE tournament_id=%')")
                .fetch_one(&pool).await.unwrap();
            if waiting { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(f.member_session)
        .execute(&pool)
        .await
        .unwrap();
    holder.commit().await.unwrap();
    let response = task.await.unwrap();
    // Repeatable-read may report a serialization conflict before the recheck can
    // observe revocation; neither that retryable response nor 401 may commit.
    assert!(matches!(
        response.status(),
        StatusCode::UNAUTHORIZED | StatusCode::CONFLICT
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM fantasy_rounds WHERE locked_at IS NOT NULL"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_selections")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(events.try_recv().is_err());
    assert!(matches!(
        results::round_results(&pool, f.member_session, f.base.tournament, f.rounds[1]).await,
        Err(Error::Authorization(_))
    ));
}
