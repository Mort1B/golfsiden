#![cfg(feature = "database-tests")]
mod fantasy_results_support;
use fantasy_results_support::*;

async fn old_schema(pool: &PgPool) {
    sqlx::migrate::Migrator {
        migrations: std::borrow::Cow::Owned(
            golf_api::schema::MIGRATOR
                .iter()
                .filter(|m| m.version <= 38)
                .cloned()
                .collect(),
        ),
        ..sqlx::migrate::Migrator::DEFAULT
    }
    .run(pool)
    .await
    .unwrap();
}
async fn version(pool: &PgPool, t: Uuid) -> i16 {
    sqlx::query_scalar("SELECT rules_version FROM fantasy_games WHERE tournament_id=$1")
        .bind(t)
        .fetch_one(pool)
        .await
        .unwrap()
}
async fn fresh_fixture(pool: &PgPool) -> Fantasy {
    sqlx::query("UPDATE users SET username=replace(id::text,'-','')")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query("UPDATE user_sessions SET token_hash=decode(replace(id::text,'-','')||replace(id::text,'-',''),'hex')").execute(pool).await.unwrap();
    fixture(pool).await
}

#[sqlx::test(migrations = false)]
async fn upgrade_only_effectively_open_games_and_preserve_receipts(pool: PgPool) {
    old_schema(&pool).await;
    let open = fresh_fixture(&pool).await;
    let input = lineup(&open);
    let receipt = fantasy::save(
        &pool,
        open.base.session,
        open.base.tournament,
        open.rounds[0],
        &input,
    )
    .await
    .unwrap();
    let before = all_surfaces(&pool, &open).await;
    let disabled = fresh_fixture(&pool).await;
    fantasy::configure(
        &pool,
        disabled.base.session,
        disabled.base.tournament,
        false,
    )
    .await
    .unwrap();
    let expired = fresh_fixture(&pool).await;
    close(&pool, &expired, expired.rounds[0]).await;
    let materialized = fresh_fixture(&pool).await;
    close(&pool, &materialized, materialized.rounds[0]).await;
    read(&pool, &materialized, materialized.rounds[0]).await;
    let opened = fresh_fixture(&pool).await;
    ready_round(&pool, &opened, opened.rounds[0]).await;
    golf_api::repositories::round_lifecycle::open_authorized(
        &pool,
        opened.base.session,
        opened.rounds[0],
    )
    .await
    .unwrap();
    let empty = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES($1,'Empty','2026-10-01','2026-10-01',1,1)").bind(empty).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO fantasy_games(tournament_id) VALUES($1)")
        .bind(empty)
        .execute(&pool)
        .await
        .unwrap();
    golf_api::schema::MIGRATOR.run(&pool).await.unwrap();
    let after = all_surfaces(&pool, &open).await;
    for (before, after) in before.iter().zip(&after) {
        assert_eq!(before["rules_version"], 1);
        assert_eq!(after["rules_version"], 2);
        assert_ne!(before["revision"], after["revision"]);
        let mut before = before.clone();
        let mut after = after.clone();
        for value in [&mut before, &mut after] {
            value.as_object_mut().unwrap().remove("rules_version");
            value.as_object_mut().unwrap().remove("revision");
        }
        assert_eq!(before, after);
    }

    for t in [open.base.tournament, disabled.base.tournament, empty] {
        assert_eq!(version(&pool, t).await, 2);
    }
    for t in [
        expired.base.tournament,
        materialized.base.tournament,
        opened.base.tournament,
    ] {
        assert_eq!(version(&pool, t).await, 1);
    }
    let replay = fantasy::save(
        &pool,
        open.base.session,
        open.base.tournament,
        open.rounds[0],
        &input,
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::to_value(receipt).unwrap(),
        serde_json::to_value(replay).unwrap()
    );
    let rejected = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES($1,'Rejected legacy','2026-10-01','2026-10-01',1,1)").bind(rejected).execute(&pool).await.unwrap();
    assert!(
        sqlx::query("INSERT INTO fantasy_games(tournament_id,rules_version) VALUES($1,1)")
            .bind(rejected)
            .execute(&pool)
            .await
            .is_err()
    );
    let new = fresh_fixture(&pool).await;
    assert_eq!(version(&pool, new.base.tournament).await, 2);
    for (t, target) in [
        (open.base.tournament, 1_i16),
        (expired.base.tournament, 2),
        (new.base.tournament, 3),
    ] {
        assert!(
            sqlx::query("UPDATE fantasy_games SET rules_version=$2 WHERE tournament_id=$1")
                .bind(t)
                .bind(target)
                .execute(&pool)
                .await
                .is_err()
        );
    }
}

#[sqlx::test(migrations = false)]
async fn migration_uses_clock_after_waiting_for_expiry(pool: PgPool) {
    old_schema(&pool).await;
    let f = fixture(&pool).await;
    let deadline: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT clock_timestamp()+interval '1 second'")
            .fetch_one(&pool)
            .await
            .unwrap();
    fantasy::set_deadline(
        &pool,
        f.base.session,
        f.base.tournament,
        f.rounds[0],
        Some(deadline),
    )
    .await
    .unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(f.rounds[0])
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let clone = pool.clone();
    let migration = tokio::spawn(async move { golf_api::schema::MIGRATOR.run(&clone).await });
    let timeout = std::time::Instant::now();
    loop {
        assert!(timeout.elapsed() < std::time::Duration::from_secs(10));
        let (now, waiting):(chrono::DateTime<Utc>,bool)=sqlx::query_as("SELECT clock_timestamp(),EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%LOCK TABLE rounds, fantasy_games%')").fetch_one(&pool).await.unwrap();
        if now >= deadline && waiting {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    blocker.commit().await.unwrap();
    migration.await.unwrap().unwrap();
    assert_eq!(version(&pool, f.base.tournament).await, 1);
    assert!(
        read(&pool, &f, f.rounds[0])
            .await
            .window
            .locked_at
            .is_some()
    );
}

#[sqlx::test(migrations = false)]
async fn frozen_legacy_json_and_source_tokens_survive_upgrade(pool: PgPool) {
    old_schema(&pool).await;
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let input = lineup(&f);
    fantasy::save(&pool, f.base.session, f.base.tournament, r, &input)
        .await
        .unwrap();
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    let h = holes(&pool, r).await;
    for (p, gross) in [(f.players[0], 1), (f.players[1], 2)] {
        numeric(&pool, &f, r, ScoreOwner::Player { id: p }, h[0], gross).await;
        disposition(&pool, &f, r, OwnerKind::Player, p, false).await;
    }
    let before = scores(&pool, &f, r, f.base.session).await;
    let source = fantasy::source(
        &pool,
        f.base.session,
        f.base.tournament,
        r,
        OwnerKind::Player,
        f.players[0],
    )
    .await
    .unwrap();
    let overall = serde_json::to_value(
        results::overall(&pool, f.base.session, f.base.tournament)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        golfer(&before, f.players[0])["holes"][0]["points"]["total"],
        10
    );
    assert_eq!(
        golfer(&before, f.players[1])["holes"][0]["points"]["total"],
        10
    );
    golf_api::schema::MIGRATOR.run(&pool).await.unwrap();
    assert_eq!(before, scores(&pool, &f, r, f.base.session).await);
    assert_eq!(
        overall,
        serde_json::to_value(
            results::overall(&pool, f.base.session, f.base.tournament)
                .await
                .unwrap()
        )
        .unwrap()
    );
    assert_eq!(
        source.source_token,
        fantasy::source(
            &pool,
            f.base.session,
            f.base.tournament,
            r,
            OwnerKind::Player,
            f.players[0]
        )
        .await
        .unwrap()
        .source_token
    );
    assert_eq!(before["rules_version"], 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn version_two_stroke_stableford_captains_and_breakdowns_agree(pool: PgPool) {
    let f = fixture(&pool).await;
    assert_eq!(version(&pool, f.base.tournament).await, 2);
    for (index, r) in f.rounds[..2].iter().copied().enumerate() {
        if index == 1 {
            sqlx::query("UPDATE rounds SET scoring_format='individual_stableford' WHERE id=$1")
                .bind(r)
                .execute(&pool)
                .await
                .unwrap();
        }
        fantasy::save(&pool, f.base.session, f.base.tournament, r, &lineup(&f))
            .await
            .unwrap();
        close(&pool, &f, r).await;
        source_fixture(&pool, &f, r).await;
        let h = holes(&pool, r).await;
        for (p, gross) in [(f.players[0], 1), (f.players[1], 2)] {
            snapshot(&pool, r, p, 18, 18).await;
            if index == 0 {
                numeric(&pool, &f, r, ScoreOwner::Player { id: p }, h[0], gross).await;
            } else {
                stableford::save_conditional(
                    &pool,
                    stableford::SaveInput {
                        request_id: Uuid::new_v4(),
                        round_id: r,
                        hole_id: h[0],
                        owner: ScoreOwner::Player { id: p },
                        input: Input::Numeric {
                            gross_strokes: gross,
                        },
                        expected_score: ExpectedScore::Absent {},
                        session_id: f.base.session,
                    },
                )
                .await
                .unwrap();
            }
        }
        for p in &f.players {
            disposition(&pool, &f, r, OwnerKind::Player, *p, false).await;
        }
        let value = scores(&pool, &f, r, f.base.session).await;
        assert_eq!(value["rules_version"], 2);
        for (p, category, total) in [
            (f.players[0], "ace", 20),
            (f.players[1], "albatross_or_better", 5),
        ] {
            let row = golfer(&value, p);
            assert_eq!(row["holes"][0]["category"], category);
            assert_eq!(row["holes"][0]["points"]["total"], total);
            assert_eq!(row["points"]["total"], total);
            assert_eq!(row["recorded_hole_points"], total);
        }
        assert_eq!(manager(&value, f.base.admin)["points"]["total"], 30);
    }
    let overall = serde_json::to_value(
        results::overall(&pool, f.base.session, f.base.tournament)
            .await
            .unwrap(),
    )
    .unwrap();
    let gb = serde_json::to_value(
        results::golfer_breakdown(&pool, f.base.session, f.base.tournament, f.players[0])
            .await
            .unwrap(),
    )
    .unwrap();
    let mb = serde_json::to_value(
        results::manager_breakdown(&pool, f.base.session, f.base.tournament, f.base.admin)
            .await
            .unwrap(),
    )
    .unwrap();
    for v in [&overall, &gb, &mb] {
        assert_eq!(v["rules_version"], 2);
        assert!(!v["revision"].as_str().unwrap().is_empty());
    }
    assert_eq!(gb["standing"]["points"]["total"], 40);
    assert_eq!(mb["standing"]["points"]["total"], 60);
    assert_eq!(gb["standing"]["rank"], 1);
}

#[sqlx::test(migrations = false)]
async fn migration_waits_for_real_round_opening_and_preserves_legacy(pool: PgPool) {
    old_schema(&pool).await;
    let f = fixture(&pool).await;
    ready_round(&pool, &f, f.rounds[0]).await;
    sqlx::raw_sql("CREATE FUNCTION scoring_open_gate() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN PERFORM pg_advisory_xact_lock(393939); RETURN NEW; END $$; CREATE TRIGGER scoring_open_gate BEFORE UPDATE OF status ON rounds FOR EACH ROW EXECUTE FUNCTION scoring_open_gate();").execute(&pool).await.unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT pg_advisory_xact_lock(393939)")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let clone = pool.clone();
    let session = f.base.session;
    let r = f.rounds[0];
    let opening = tokio::spawn(async move {
        golf_api::repositories::round_lifecycle::open_authorized(&clone, session, r).await
    });
    let timeout = std::time::Instant::now();
    loop {
        assert!(timeout.elapsed() < std::time::Duration::from_secs(10));
        let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event='advisory' AND query LIKE 'UPDATE rounds SET status%')").fetch_one(&pool).await.unwrap();
        if waiting {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    let clone = pool.clone();
    let migration = tokio::spawn(async move { golf_api::schema::MIGRATOR.run(&clone).await });
    loop {
        assert!(timeout.elapsed() < std::time::Duration::from_secs(10));
        let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%LOCK TABLE rounds, fantasy_games%')").fetch_one(&pool).await.unwrap();
        if waiting {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    blocker.commit().await.unwrap();
    opening.await.unwrap().unwrap();
    migration.await.unwrap().unwrap();
    assert_eq!(version(&pool, f.base.tournament).await, 1);
    let opened: bool =
        sqlx::query_scalar("SELECT opened_at IS NOT NULL FROM fantasy_rounds WHERE round_id=$1")
            .bind(r)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(opened);
}

async fn all_surfaces(pool: &PgPool, f: &Fantasy) -> Vec<Value> {
    let t = f.base.tournament;
    let s = f.base.session;
    vec![
        serde_json::to_value(results::overall(pool, s, t).await.unwrap()).unwrap(),
        serde_json::to_value(
            results::round_results(pool, s, t, f.rounds[0])
                .await
                .unwrap(),
        )
        .unwrap(),
        serde_json::to_value(
            results::golfer_breakdown(pool, s, t, f.players[0])
                .await
                .unwrap(),
        )
        .unwrap(),
        serde_json::to_value(
            results::manager_breakdown(pool, s, t, f.base.admin)
                .await
                .unwrap(),
        )
        .unwrap(),
    ]
}
