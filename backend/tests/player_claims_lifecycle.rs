#![cfg(feature = "database-tests")]
mod player_claims_support;
use player_claims_support::*;

#[sqlx::test(migrations = "../migrations")]
async fn withdrawal_account_contention_returns_conflict_without_partial_writes(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, token) = prepared(&pool, &f).await;
    let claimed = claim(&pool, g.claim_id, &token, "contended_player")
        .await
        .unwrap();
    for statement in [
        "SELECT id FROM users WHERE id=$1 FOR UPDATE",
        "SELECT user_id FROM tournament_memberships WHERE user_id=$1 FOR UPDATE",
    ] {
        let before: Value =
            sqlx::query_scalar("SELECT to_jsonb(g) FROM player_claim_grants g WHERE id=$1")
                .bind(g.claim_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        let mut held = pool.begin().await.unwrap();
        sqlx::query(statement)
            .bind(claimed.session.user_id)
            .fetch_all(&mut *held)
            .await
            .unwrap();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            player_claims::withdraw(&pool, f.session, f.tournament, g.player_id),
        )
        .await
        .unwrap();
        assert!(matches!(result, Err(ClaimError::Busy)));
        held.rollback().await.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, String>(
                "SELECT status::text FROM tournament_players WHERE player_id=$1"
            )
            .bind(g.player_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            "active"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM tournament_player_withdrawals WHERE player_id=$1"
            )
            .bind(g.player_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            0
        );
        assert_eq!(
            sqlx::query_scalar::<_, Value>(
                "SELECT to_jsonb(g) FROM player_claim_grants g WHERE id=$1"
            )
            .bind(g.claim_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
            before
        );
    }
    player_claims::withdraw(&pool, f.session, f.tournament, g.player_id)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn completed_and_archived_claims_work_without_reopening_joining(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, t) = prepared(&pool, &f).await;
    let r = round(&pool, &f).await;
    historical_status(&pool, r, g.player_id, "locked").await;
    closed_status(&pool, f.tournament, "completed").await;
    historical_score(&pool, &f, r, g.player_id).await;
    let before = history(&pool, g.player_id).await;
    assert_eq!(before["scores"].as_array().unwrap().len(), 1);
    assert_eq!(before["audits"].as_array().unwrap().len(), 1);
    assert!(
        sqlx::query(
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'player')"
        )
        .bind(f.tournament)
        .bind(f.other)
        .execute(&pool)
        .await
        .is_err()
    );
    assert!(matches!(
        player_claims::create(&pool, f.session, f.tournament, "No", 10.0, &[1; 32]).await,
        Err(ClaimError::Closed)
    ));
    assert!(matches!(
        player_claims::withdraw(&pool, f.session, f.tournament, g.player_id).await,
        Err(ClaimError::Closed)
    ));
    player_claims::revoke(&pool, f.session, f.tournament, g.player_id)
        .await
        .unwrap();
    let new = player_claims::reissue(
        &pool,
        f.session,
        f.tournament,
        g.player_id,
        &hash_invitation_token(&t),
    )
    .await
    .unwrap();
    player_claims::preview(&pool, new.claim_id, &t)
        .await
        .unwrap();
    claim(&pool, new.claim_id, &t, "completed_claim")
        .await
        .unwrap();
    assert_eq!(history(&pool, g.player_id).await, before);
    // A second already prepared entrant proves the archived path too.
    closed_status(&pool, f.tournament, "draft").await;
    let (second, token) = prepared(&pool, &f).await;
    closed_status(&pool, f.tournament, "archived").await;
    claim(&pool, second.claim_id, &token, "archived_claim")
        .await
        .unwrap();
    assert!(
        sqlx::query(
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'player')"
        )
        .bind(f.tournament)
        .bind(f.other)
        .execute(&pool)
        .await
        .is_err()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn withdrawal_guards_draft_flight_and_open_completed_snapshots(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, _) = prepared(&pool, &f).await;
    let r = round(&pool, &f).await;
    let flight = Uuid::new_v4();
    sqlx::query("INSERT INTO flights(id,round_id,tournament_id,name) VALUES($1,$2,$3,'First')")
        .bind(flight)
        .bind(r)
        .bind(f.tournament)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(flight).bind(r).bind(f.tournament).bind(g.player_id).execute(&pool).await.unwrap();
    assert!(matches!(
        player_claims::withdraw(&pool, f.session, f.tournament, g.player_id).await,
        Err(ClaimError::DraftAssignment)
    ));
    sqlx::query("DELETE FROM flight_memberships WHERE player_id=$1")
        .bind(g.player_id)
        .execute(&pool)
        .await
        .unwrap();
    for status in ["open", "completed"] {
        historical_status(&pool, r, g.player_id, status).await;
        assert!(matches!(
            player_claims::withdraw(&pool, f.session, f.tournament, g.player_id).await,
            Err(ClaimError::LiveRound)
        ));
    }
    historical_status(&pool, r, g.player_id, "locked").await;
    historical_score(&pool, &f, r, g.player_id).await;
    let before = history(&pool, g.player_id).await;
    assert_eq!(before["scores"].as_array().unwrap().len(), 1);
    assert_eq!(before["audits"].as_array().unwrap().len(), 1);
    player_claims::withdraw(&pool, f.session, f.tournament, g.player_id)
        .await
        .unwrap();
    assert_eq!(history(&pool, g.player_id).await, before);
}
#[sqlx::test(migrations = "../migrations")]
async fn concurrent_claim_and_withdrawal_never_resurrect_entrant(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, t) = prepared(&pool, &f).await;
    let (claimed, withdrawn) = tokio::join!(
        claim(&pool, g.claim_id, &t, "race_claim"),
        player_claims::withdraw(&pool, f.session, f.tournament, g.player_id)
    );
    withdrawn.unwrap();
    assert!(claimed.is_ok() || matches!(claimed, Err(ClaimError::Unavailable)));
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM tournament_players WHERE player_id=$1"
        )
        .bind(g.player_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "withdrawn"
    );
    assert!(matches!(
        player_claims::preview(&pool, g.claim_id, &t).await,
        Err(ClaimError::Unavailable)
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_and_session_revocation_after_lock_waits_are_rechecked(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, t) = prepared(&pool, &f).await;
    let mut held = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM players WHERE id=$1 FOR UPDATE")
        .bind(g.player_id)
        .execute(&mut *held)
        .await
        .unwrap();
    let work_pool = pool.clone();
    let work_token = t.clone();
    let work =
        tokio::spawn(
            async move { claim(&work_pool, g.claim_id, &work_token, "expired_wait").await },
        );
    wait_for_lock(&pool, "SELECT active FROM players%").await;
    // Editing expiry under the held player lock deterministically precedes the
    // claimant's final eligibility check, regardless of task scheduling.
    sqlx::query("UPDATE player_claim_grants SET expires_at=clock_timestamp()-interval '1 second' WHERE id=$1").bind(g.claim_id).execute(&mut *held).await.unwrap();
    held.commit().await.unwrap();
    assert!(matches!(work.await.unwrap(), Err(ClaimError::Unavailable)));
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '1 second' WHERE id=$1",
    )
    .bind(f.session)
    .execute(&pool)
    .await
    .unwrap();
    let mut held = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR UPDATE")
        .bind(f.tournament)
        .execute(&mut *held)
        .await
        .unwrap();
    let work_pool = pool.clone();
    let session = f.session;
    let tournament = f.tournament;
    let work = tokio::spawn(async move {
        player_claims::create(
            &work_pool,
            session,
            tournament,
            "Expired Admin",
            10.0,
            &[1; 32],
        )
        .await
    });
    wait_for_lock(&pool, "SELECT status IN%FOR SHARE").await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    held.commit().await.unwrap();
    assert!(matches!(
        work.await.unwrap(),
        Err(ClaimError::Authorization(_))
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn username_wait_past_expiry_rolls_back_entire_claim(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, t) = prepared(&pool, &f).await;
    sqlx::query("UPDATE player_claim_grants SET expires_at=clock_timestamp()+interval '1 second' WHERE id=$1").bind(g.claim_id).execute(&pool).await.unwrap();
    let mut held = pool.begin().await.unwrap();
    sqlx::query(
        "INSERT INTO users(id,username,display_name,role) VALUES($1,'held_name','Held','player')",
    )
    .bind(Uuid::new_v4())
    .execute(&mut *held)
    .await
    .unwrap();
    let work_pool = pool.clone();
    let work = tokio::spawn(async move { claim(&work_pool, g.claim_id, &t, "held_name").await });
    wait_for_lock(&pool, "INSERT INTO users%").await;
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    held.rollback().await.unwrap();
    assert!(matches!(work.await.unwrap(), Err(ClaimError::Unavailable)));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users WHERE player_id=$1")
            .bind(g.player_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT claimed_at IS NULL FROM player_claim_grants WHERE id=$1"
        )
        .bind(g.claim_id)
        .fetch_one(&pool)
        .await
        .unwrap()
    );
}
