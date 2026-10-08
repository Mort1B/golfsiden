#![cfg(feature = "database-tests")]
mod fantasy_support;
use fantasy_support::*;
async fn wait_for_round_waiter(pool: &PgPool) {
    tokio::time::timeout(std::time::Duration::from_secs(5),async{
  loop{let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'SELECT id FROM rounds WHERE tournament_id=%')").fetch_one(pool).await.unwrap();if waiting{break;}tokio::time::sleep(std::time::Duration::from_millis(10)).await;}
 }).await.unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn held_round_save_rechecks_actual_deadline(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let input = lineup(&f);
    fantasy::set_deadline(
        &pool,
        f.base.session,
        f.base.tournament,
        r,
        Some(Utc::now() + Duration::milliseconds(400)),
    )
    .await
    .unwrap();
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(r)
        .execute(&mut *holder)
        .await
        .unwrap();
    let p = pool.clone();
    let session = f.member_session;
    let t = f.base.tournament;
    let task = tokio::spawn(async move { fantasy::save(&p, session, t, r, &input).await });
    wait_for_round_waiter(&pool).await;
    tokio::time::sleep(std::time::Duration::from_millis(420)).await;
    holder.commit().await.unwrap();
    assert!(matches!(task.await.unwrap(), Err(Error::Closed)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_lineups")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(
        fantasy::set_deadline(&pool, f.base.session, t, r, None)
            .await
            .is_err()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn revocation_while_waiting_creates_no_receipt(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let input = lineup(&f);
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(r)
        .execute(&mut *holder)
        .await
        .unwrap();
    let p = pool.clone();
    let session = f.member_session;
    let t = f.base.tournament;
    let task = tokio::spawn(async move { fantasy::save(&p, session, t, r, &input).await });
    wait_for_round_waiter(&pool).await;
    sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(session)
        .execute(&pool)
        .await
        .unwrap();
    holder.commit().await.unwrap();
    assert!(matches!(task.await.unwrap(), Err(Error::Authorization(_))));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_lineups")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn non_fantasy_roster_write_not_blocked_and_fantasy_conflict_is_explicit(pool: PgPool) {
    let f = seed(&pool).await;
    let r = round(&pool, &f).await;
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(r)
        .execute(&mut *holder)
        .await
        .unwrap();
    let (g, _) = tokio::time::timeout(std::time::Duration::from_secs(3), prepared(&pool, &f))
        .await
        .unwrap();
    assert!(!g.player_id.is_nil());
    holder.commit().await.unwrap();
    fantasy::configure(&pool, f.session, f.tournament, true)
        .await
        .unwrap();
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(r)
        .execute(&mut *holder)
        .await
        .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    let response = app
        .oneshot(request(
            "POST",
            &format!("/api/tournaments/{}/players", f.tournament),
            Some(ADMIN_TOKEN),
            json!({"display_name":"New player","handicap_index":12.0}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    holder.rollback().await.unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn direct_tenant_cardinality_history_and_window_constraints(pool: PgPool) {
    let f = fixture(&pool).await;
    let input = lineup(&f);
    let r = f.rounds[0];
    fantasy::save(&pool, f.base.session, f.base.tournament, r, &input)
        .await
        .unwrap();
    let sql = "INSERT INTO fantasy_lineups(id,tournament_id,round_id,user_id,revision,first_player,second_player,third_player,fourth_player,captain,origin,request_id,expected_revision) SELECT gen_random_uuid(),tournament_id,round_id,user_id,revision+1,first_player,second_player,third_player,fourth_player,captain,'submitted',gen_random_uuid(),NULL FROM fantasy_lineups LIMIT 1";
    assert!(sqlx::query(sql).execute(&pool).await.is_err());
    let sql = "INSERT INTO fantasy_lineups(id,tournament_id,round_id,user_id,revision,first_player,second_player,third_player,fourth_player,captain,origin,request_id,expected_revision) SELECT gen_random_uuid(),tournament_id,round_id,user_id,revision+1,first_player,first_player,third_player,fourth_player,captain,'submitted',gen_random_uuid(),revision FROM fantasy_lineups LIMIT 1";
    assert!(sqlx::query(sql).execute(&pool).await.is_err());
    assert!(sqlx::query("INSERT INTO fantasy_eligibility_history(tournament_id,player_id,eligible,effective_at) VALUES($1,$2,false,'2020-01-01')").bind(f.base.tournament).bind(f.players[0]).execute(&pool).await.is_err());
    assert!(sqlx::query("INSERT INTO fantasy_membership_history(tournament_id,user_id,present) VALUES($1,$2,false)").bind(f.base.tournament).bind(f.base.admin).execute(&pool).await.is_err());
    assert!(sqlx::query("UPDATE tournament_memberships SET user_id=gen_random_uuid() WHERE tournament_id=$1 AND user_id=$2").bind(f.base.tournament).bind(f.base.other).execute(&pool).await.is_err());
    close(&pool, &f, r).await;
    read(&pool, &f, r).await;
    assert!(
        sqlx::query("UPDATE fantasy_rounds SET locked_at=NULL,deadline=NULL WHERE round_id=$1")
            .bind(r)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("UPDATE fantasy_games SET enabled=false WHERE tournament_id=$1")
            .bind(f.base.tournament)
            .execute(&pool)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn real_opening_serializes_manual_save_and_preserves_earlier_lock(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    ready_round(&pool, &f, r).await;
    assert!(
        sqlx::query("UPDATE fantasy_rounds SET opened_at=clock_timestamp() WHERE round_id=$1")
            .bind(r)
            .execute(&pool)
            .await
            .is_err()
    );
    let input = lineup(&f);
    let (opened, saved) = tokio::join!(
        golf_api::repositories::round_lifecycle::open_authorized(&pool, f.base.session, r),
        fantasy::save(&pool, f.member_session, f.base.tournament, r, &input)
    );
    opened.unwrap();
    let view = read(&pool, &f, r).await;
    assert!(view.window.locked_at.is_some());
    match saved {
        Ok(receipt) => assert_eq!(
            view.selections
                .iter()
                .find(|s| s.user_id == f.base.other)
                .unwrap()
                .receipt
                .as_ref()
                .unwrap()
                .id,
            receipt.id
        ),
        Err(Error::Closed | Error::Conflict) => assert_eq!(
            view.selections
                .iter()
                .find(|s| s.user_id == f.base.other)
                .unwrap()
                .state,
            "missed"
        ),
        Err(e) => panic!("unexpected save error: {e}"),
    }
    let r = f.rounds[1];
    ready_round(&pool, &f, r).await;
    close(&pool, &f, r).await;
    let earlier = read(&pool, &f, r).await.window.locked_at;
    golf_api::repositories::round_lifecycle::open_authorized(&pool, f.base.session, r)
        .await
        .unwrap();
    let view = read(&pool, &f, r).await;
    assert_eq!(view.window.locked_at, earlier);
    assert!(view.window.opened_at.is_some());
}
#[sqlx::test(migrations = "../migrations")]
async fn new_enrollment_waits_for_global_activity_and_captures_committed_state(pool: PgPool) {
    let f = fixture(&pool).await;
    let player = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,'New golfer',10)",
    )
    .bind(player)
    .execute(&pool)
    .await
    .unwrap();
    let mut holder = pool.begin().await.unwrap();
    sqlx::query("UPDATE players SET active=false WHERE id=$1")
        .bind(player)
        .execute(&mut *holder)
        .await
        .unwrap();
    let p = pool.clone();
    let t = f.base.tournament;
    let task = tokio::spawn(async move {
        sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES($1,$2,10)").bind(t).bind(player).execute(&p).await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5),async{loop{let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'INSERT INTO tournament_players%')").fetch_one(&pool).await.unwrap();if waiting{break;}tokio::time::sleep(std::time::Duration::from_millis(10)).await;}}).await.unwrap();
    holder.commit().await.unwrap();
    task.await.unwrap().unwrap();
    let eligible:bool=sqlx::query_scalar("SELECT eligible FROM fantasy_eligibility_history WHERE tournament_id=$1 AND player_id=$2 ORDER BY effective_at DESC,id DESC LIMIT 1").bind(t).bind(player).fetch_one(&pool).await.unwrap();
    assert!(!eligible);
    assert!(
        !read(&pool, &f, f.rounds[0])
            .await
            .eligible_players
            .contains(&player)
    );
}
