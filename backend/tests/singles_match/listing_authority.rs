use super::*;
use std::time::Duration;

async fn read(
    pool: &PgPool,
    session: Uuid,
    filtered: bool,
    player: Uuid,
) -> Result<match_play::reads::Listing, match_play::Error> {
    if filtered {
        match_play::reads::list_for_player(pool, session, id(5), player).await
    } else {
        match_play::reads::list(pool, session, id(5)).await
    }
}
async fn waiting(pool: &PgPool, pattern: &str) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let found: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND cardinality(pg_blocking_pids(pid))>0 AND query LIKE $1)")
                .bind(pattern).fetch_one(pool).await.unwrap();
            if found { return; }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }).await.unwrap();
}
async fn expired(pool: &PgPool, session: Uuid) {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let expired: bool = sqlx::query_scalar(
                "SELECT expires_at <= clock_timestamp() FROM user_sessions WHERE id=$1",
            )
            .bind(session)
            .fetch_one(pool)
            .await
            .unwrap();
            if expired {
                return;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn filtered_listing_rejects_expiry_during_last_card_materialization(pool: PgPool) {
    let (session, _, _) = ready(&pool).await;
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1",
    )
    .bind(session)
    .execute(&pool)
    .await
    .unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE singles_match_notes IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let p = pool.clone();
    let task = tokio::spawn(async move { read(&p, session, true, id(11)).await });
    waiting(
        &pool,
        "SELECT player_id,hole_number,gross_strokes FROM singles_match_notes%",
    )
    .await;
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT expires_at > clock_timestamp() FROM user_sessions WHERE id=$1"
        )
        .bind(session)
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    expired(&pool, session).await;
    blocker.commit().await.unwrap();
    assert!(matches!(
        task.await.unwrap(),
        Err(match_play::Error::Unauthenticated)
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn empty_listing_rejects_expiry_after_membership_wait(pool: PgPool) {
    let (session, _, _) = ready(&pool).await;
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1",
    )
    .bind(session)
    .execute(&pool)
    .await
    .unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT role FROM tournament_memberships WHERE user_id=$1 FOR UPDATE")
        .bind(id(1))
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let p = pool.clone();
    let task = tokio::spawn(async move { read(&p, session, true, id(999)).await });
    waiting(&pool, "SELECT role FROM tournament_memberships%").await;
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT expires_at > clock_timestamp() FROM user_sessions WHERE id=$1"
        )
        .bind(session)
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    expired(&pool, session).await;
    blocker.commit().await.unwrap();
    assert!(matches!(
        task.await.unwrap(),
        Err(match_play::Error::Unauthenticated)
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn listing_holds_session_and_membership_until_assembly_finishes(pool: PgPool) {
    let (_, _, _) = ready(&pool).await;
    for filtered in [false, true] {
        let user = if filtered { 212 } else { 211 };
        let session = fixture::session(
            &pool,
            user,
            None,
            "scorer",
            if filtered {
                "hold-filtered"
            } else {
                "hold-full"
            },
        )
        .await;
        let mut blocker = pool.begin().await.unwrap();
        sqlx::query("LOCK TABLE singles_match_notes IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *blocker)
            .await
            .unwrap();
        let p = pool.clone();
        let task = tokio::spawn(async move { read(&p, session, filtered, id(11)).await });
        waiting(
            &pool,
            "SELECT player_id,hole_number,gross_strokes FROM singles_match_notes%",
        )
        .await;
        let p = pool.clone();
        let revoke = tokio::spawn(async move {
            sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
                .bind(session)
                .execute(&p)
                .await
                .unwrap()
        });
        let p = pool.clone();
        let remove = tokio::spawn(async move {
            sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
                .bind(id(user))
                .execute(&p)
                .await
                .unwrap()
        });
        waiting(&pool, "UPDATE user_sessions SET revoked_at%").await;
        waiting(&pool, "DELETE FROM tournament_memberships%").await;
        assert!(!revoke.is_finished() && !remove.is_finished());
        blocker.commit().await.unwrap();
        let result = tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            result.writable_match_ids.len(),
            if filtered { 1 } else { 2 }
        );
        revoke.await.unwrap();
        remove.await.unwrap();
        assert!(matches!(
            read(&pool, session, filtered, id(11)).await,
            Err(match_play::Error::Unauthenticated)
        ));
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn listing_write_discovery_matches_unchanged_scoring_authority(pool: PgPool) {
    let (admin, _, _) = ready(&pool).await;
    let scorer = fixture::session(&pool, 221, None, "scorer", "oracle-scorer").await;
    let player = fixture::session(&pool, 222, Some(11), "player", "oracle-player").await;
    let unlinked = fixture::session(&pool, 223, None, "player", "oracle-unlinked").await;
    let viewer = fixture::session(&pool, 224, Some(13), "viewer", "oracle-viewer").await;
    for session in [admin, scorer, player, unlinked, viewer] {
        for filtered in [false, true] {
            for target in [11, 12, 13, 14, 999] {
                let result = read(&pool, session, filtered, id(target)).await.unwrap();
                for card in result.matches {
                    let ordinary =
                        match_play::reads::get(&pool, session, id(5), card.match_id, false)
                            .await
                            .unwrap();
                    assert_eq!(
                        serde_json::to_value(&card).unwrap(),
                        serde_json::to_value(ordinary).unwrap()
                    );
                    let scoring =
                        match_play::reads::get(&pool, session, id(5), card.match_id, true).await;
                    assert!(
                        scoring.is_ok() || matches!(scoring, Err(match_play::Error::Forbidden))
                    );
                    assert_eq!(
                        result.writable_match_ids.contains(&card.match_id),
                        scoring.is_ok()
                    );
                }
            }
        }
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn listing_waiting_behind_committed_revocation_or_removal_fails_closed(pool: PgPool) {
    ready(&pool).await;
    for (index, (filtered, revoke)) in [(false, true), (true, true), (false, false), (true, false)]
        .into_iter()
        .enumerate()
    {
        let user = 240 + index as u128;
        let session = fixture::session(
            &pool,
            user,
            None,
            "scorer",
            &format!("waiting-list-{index}"),
        )
        .await;
        let mut blocker = pool.begin().await.unwrap();
        if revoke {
            sqlx::query("SELECT id FROM user_sessions WHERE id=$1 FOR UPDATE")
                .bind(session)
                .fetch_one(&mut *blocker)
                .await
                .unwrap();
        } else {
            sqlx::query("SELECT role FROM tournament_memberships WHERE user_id=$1 FOR UPDATE")
                .bind(id(user))
                .fetch_one(&mut *blocker)
                .await
                .unwrap();
        }
        let p = pool.clone();
        let task = tokio::spawn(async move { read(&p, session, filtered, id(11)).await });
        waiting(
            &pool,
            if revoke {
                "SELECT s.id AS session_id%"
            } else {
                "SELECT role FROM tournament_memberships%"
            },
        )
        .await;
        if revoke {
            sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
                .bind(session)
                .execute(&mut *blocker)
                .await
                .unwrap();
        } else {
            sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
                .bind(id(user))
                .execute(&mut *blocker)
                .await
                .unwrap();
        }
        blocker.commit().await.unwrap();
        match tokio::time::timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap()
        {
            Err(match_play::Error::Unauthenticated | match_play::Error::Forbidden) => {}
            Err(match_play::Error::Database(sqlx::Error::Database(error))) => {
                assert_eq!(error.code().as_deref(), Some("40001"))
            }
            _ => panic!("waiting listing must fail closed"),
        }
        let fresh = read(&pool, session, filtered, id(11)).await;
        assert!(if revoke {
            matches!(fresh, Err(match_play::Error::Unauthenticated))
        } else {
            matches!(fresh, Err(match_play::Error::Forbidden))
        });
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn incomplete_persisted_flight_never_grants_match_from_one_eligible_opponent(pool: PgPool) {
    let (_, first, _) = ready(&pool).await;
    let a = fixture::session(&pool, 251, Some(11), "player", "one-first").await;
    let b = fixture::session(&pool, 252, Some(12), "player", "one-second").await;
    // Defensive stored-data fixture: ordinary writes cannot remove open pairings.
    // Temporarily disable only that guard in this isolated test transaction.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query(
        "ALTER TABLE flight_memberships DISABLE TRIGGER flight_memberships_protect_round_pairing",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("DELETE FROM flight_memberships WHERE round_id=$1 AND player_id=ANY($2)")
        .bind(id(5))
        .bind(vec![id(11), id(12)])
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "ALTER TABLE flight_memberships ENABLE TRIGGER flight_memberships_protect_round_pairing",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    for session in [a, b] {
        for filtered in [false, true] {
            let result = read(&pool, session, filtered, id(11)).await.unwrap();
            assert!(result.writable_match_ids.is_empty());
            assert!(result.matches.iter().any(|card| card.match_id == first));
        }
        assert!(matches!(
            match_play::reads::get(&pool, session, id(5), first, true).await,
            Err(match_play::Error::Forbidden)
        ));
    }
}
