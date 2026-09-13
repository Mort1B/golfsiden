use super::*;
#[sqlx::test(migrations = "../migrations")]
async fn concurrent_notes_have_one_winner_and_independent_matches_progress(pool: PgPool) {
    let (session, m, other) = ready(&pool).await;
    let a = request(
        1,
        Command::Note {
            player_id: id(11),
            hole_number: 1,
            gross_strokes: 4,
        },
    );
    let b = request(
        1,
        Command::Note {
            player_id: id(12),
            hole_number: 1,
            gross_strokes: 5,
        },
    );
    let (a, b) = tokio::join!(
        match_play::execute(&pool, session, id(5), m, a),
        match_play::execute(&pool, session, id(5), m, b)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    submit(
        &pool,
        session,
        other,
        1,
        Command::Note {
            player_id: id(13),
            hole_number: 1,
            gross_strokes: 4,
        },
    )
    .await;
    let count = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM singles_match_notes")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}
async fn wait_lock(pool: &PgPool) {
    tokio::time::timeout(std::time::Duration::from_secs(4),async{loop{let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock')").fetch_one(pool).await.unwrap();if waiting{break;}tokio::time::sleep(std::time::Duration::from_millis(10)).await;}}).await.unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn waiting_note_rechecks_current_session_before_any_effect(pool: PgPool) {
    let (_, m, _) = ready(&pool).await;
    let player = fixture::session(&pool, 208, Some(11), "player", "waiting-note").await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(id(5))
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    let p = pool.clone();
    let write = tokio::spawn(async move {
        match_play::execute(
            &p,
            player,
            id(5),
            m,
            request(
                1,
                Command::Note {
                    player_id: id(11),
                    hole_number: 1,
                    gross_strokes: 4,
                },
            ),
        )
        .await
    });
    wait_lock(&pool).await;
    sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(player)
        .execute(&pool)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        write.await.unwrap(),
        Err(match_play::Error::Unauthenticated)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM singles_match_receipts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn terminal_report_and_note_serialize_and_only_one_applies(pool: PgPool) {
    let (session, m, _) = ready(&pool).await;
    let a = request(
        1,
        Command::Report {
            event: golf_api::domain::match_play::commands::Event::Concession {
                conceding_player_id: id(12),
                communicated: true,
                after_hole: 0,
            },
        },
    );
    let b = request(
        1,
        Command::Note {
            player_id: id(11),
            hole_number: 1,
            gross_strokes: 4,
        },
    );
    let (a, b) = tokio::join!(
        match_play::execute(&pool, session, id(5), m, a),
        match_play::execute(&pool, session, id(5), m, b)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM singles_match_audits WHERE match_id=$1")
            .bind(m)
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn account_request_identity_cannot_be_reused_across_matches(pool: PgPool) {
    let (session, m, other) = ready(&pool).await;
    let a = request(
        1,
        Command::Note {
            player_id: id(11),
            hole_number: 1,
            gross_strokes: 4,
        },
    );
    let mut b = request(
        1,
        Command::Note {
            player_id: id(13),
            hole_number: 1,
            gross_strokes: 4,
        },
    );
    b.request_id = a.request_id;
    let (a, b) = tokio::join!(
        match_play::execute(&pool, session, id(5), m, a),
        match_play::execute(&pool, session, id(5), other, b)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    assert!(matches!(
        a.err().or_else(|| b.err()),
        Some(match_play::Error::Conflict("match_request_mismatch"))
    ));
    for table in [
        "singles_match_receipts",
        "singles_match_audits",
        "singles_match_notes",
    ] {
        let query = format!("SELECT count(*) FROM {table}");
        assert_eq!(
            sqlx::query_scalar::<_, i64>(&query)
                .fetch_one(&pool)
                .await
                .unwrap(),
            1
        );
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn waiting_command_rechecks_expired_session_and_removed_membership(pool: PgPool) {
    let (_, m, _) = ready(&pool).await;
    for (user, token, expire) in [(210, "expire-match", true), (211, "remove-match", false)] {
        let player = fixture::session(
            &pool,
            user,
            Some(if expire { 11 } else { 12 }),
            "player",
            token,
        )
        .await;
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
            .bind(id(5))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
        let p = pool.clone();
        let write = tokio::spawn(async move {
            match_play::execute(
                &p,
                player,
                id(5),
                m,
                request(
                    1,
                    Command::Note {
                        player_id: id(11),
                        hole_number: 1,
                        gross_strokes: 4,
                    },
                ),
            )
            .await
        });
        wait_lock(&pool).await;
        if expire {
            sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp() WHERE id=$1")
                .bind(player)
                .execute(&pool)
                .await
                .unwrap();
        } else {
            sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
                .bind(id(2))
                .bind(id(user))
                .execute(&pool)
                .await
                .unwrap();
        }
        tx.commit().await.unwrap();
        assert!(matches!(
            write.await.unwrap(),
            Err(match_play::Error::Unauthenticated | match_play::Error::Forbidden)
        ));
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM singles_match_receipts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
