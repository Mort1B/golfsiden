use super::*;
#[sqlx::test(migrations = "../migrations")]
async fn sql_identity_terminal_receipt_and_settings_guards(pool: PgPool) {
    let (session, m, _) = ready(&pool).await;
    for sql in [
        "UPDATE singles_matches SET revision=revision+1 WHERE id=$1",
        "UPDATE singles_matches SET confirmed=TRUE,terminal=TRUE,revision=revision+1 WHERE id=$1",
        "DELETE FROM singles_match_opponents WHERE match_id=$1",
        "UPDATE singles_matches SET second_player_id=first_player_id WHERE id=$1",
    ] {
        assert!(
            sqlx::query(sql).bind(m).execute(&pool).await.is_err(),
            "{sql}"
        );
    }
    assert!(sqlx::query("INSERT INTO singles_match_notes(match_id,player_id,hole_number,gross_strokes) VALUES($1,$2,1,4)").bind(m).bind(id(11)).execute(&pool).await.is_err());
    assert!(
        sqlx::query("UPDATE rounds SET handicap_allowance_percent=90 WHERE id=$1")
            .bind(id(5))
            .execute(&pool)
            .await
            .is_err()
    );
    let updated = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
        .bind(id(5))
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        match_play::setup::replace(
            &pool,
            session,
            id(5),
            Assignments {
                expected_round_updated_at: updated,
                matches: vec![]
            }
        )
        .await
        .is_err()
    );
}

async fn ctx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: Uuid,
    m: Uuid,
    command: serde_json::Value,
) -> Uuid {
    let request = Uuid::new_v4();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(id(5))
        .fetch_one(&mut **tx)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM singles_matches WHERE id=$1 FOR UPDATE")
        .bind(m)
        .fetch_one(&mut **tx)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.match_actor',$1::text,true),set_config('app.match_session',$2::text,true),set_config('app.match_note_id',$3::text,true),set_config('app.match_command',$4,true),set_config('app.match_request',$5::text,true),set_config('app.score_mutation_round_id',$6::text,true)").bind(id(1)).bind(session).bind(m).bind(command.to_string()).bind(request).bind(id(5)).execute(&mut **tx).await.unwrap();
    request
}
#[sqlx::test(migrations = "../migrations")]
async fn direct_context_cannot_bypass_revision_audit_or_invent_terminal_points(pool: PgPool) {
    let (session, m, _) = ready(&pool).await;
    let mut tx = pool.begin().await.unwrap();
    ctx(
        &mut tx,
        session,
        m,
        serde_json::json!({"type":"note","player_id":id(11),"hole_number":1,"gross_strokes":4}),
    )
    .await;
    sqlx::query("INSERT INTO singles_match_notes(match_id,player_id,hole_number,gross_strokes) VALUES($1,$2,1,4)").bind(m).bind(id(11)).execute(&mut *tx).await.unwrap();
    assert!(tx.commit().await.is_err());
    let mut tx = pool.begin().await.unwrap();
    ctx(
        &mut tx,
        session,
        m,
        serde_json::json!({"type":"confirm","result_agreed_or_awarded":true}),
    )
    .await;
    assert!(sqlx::query("UPDATE singles_matches SET revision=revision+1,terminal=TRUE,confirmed=TRUE,first_half_points=2,second_half_points=0 WHERE id=$1").bind(m).execute(&mut *tx).await.is_err());
    tx.rollback().await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    ctx(
        &mut tx,
        session,
        m,
        serde_json::json!({"type":"note","player_id":id(11),"hole_number":1,"gross_strokes":4}),
    )
    .await;
    sqlx::query("UPDATE singles_matches SET revision=revision+1 WHERE id=$1")
        .bind(m)
        .execute(&mut *tx)
        .await
        .unwrap();
    assert!(tx.commit().await.is_err());
    let count = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM singles_match_notes")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
#[sqlx::test(migrations = "../migrations")]
async fn late_receipt_failure_rolls_back_notes_ledger_confirmation_and_points(pool: PgPool) {
    use golf_api::domain::match_play::commands::{CorrectionKind, Event};
    let (session, m, _) = ready(&pool).await;
    submit(
        &pool,
        session,
        m,
        1,
        Command::Report {
            event: Event::Concession {
                conceding_player_id: id(12),
                communicated: true,
                after_hole: 0,
            },
        },
    )
    .await;
    submit(
        &pool,
        session,
        m,
        2,
        Command::Confirm {
            result_agreed_or_awarded: true,
        },
    )
    .await;
    let before = serde_json::to_value(
        match_play::reads::get(&pool, session, id(5), m, true)
            .await
            .unwrap(),
    )
    .unwrap();
    let events = before["accepted_events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| serde_json::from_value(e["id"].clone()).unwrap())
        .collect();
    sqlx::raw_sql("CREATE FUNCTION fail_match_receipt() RETURNS trigger LANGUAGE plpgsql AS $$BEGIN RAISE EXCEPTION 'simulated receipt failure';END$$;CREATE TRIGGER fail_receipt BEFORE INSERT ON singles_match_receipts FOR EACH ROW EXECUTE FUNCTION fail_match_receipt();").execute(&pool).await.unwrap();
    assert!(
        match_play::execute(
            &pool,
            session,
            id(5),
            m,
            request(
                3,
                Command::Correct {
                    kind: CorrectionKind::RecordingError,
                    reason: "Would invalidate confirmed result".into(),
                    superseded_event_ids: events,
                    replacement: vec![]
                }
            )
        )
        .await
        .is_err()
    );
    assert_eq!(
        before,
        serde_json::to_value(
            match_play::reads::get(&pool, session, id(5), m, true)
                .await
                .unwrap()
        )
        .unwrap()
    );
    let other = sqlx::query_scalar::<_, Uuid>("SELECT id FROM singles_matches WHERE id<>$1")
        .bind(m)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        match_play::execute(
            &pool,
            session,
            id(5),
            other,
            request(
                1,
                Command::Note {
                    player_id: id(13),
                    hole_number: 1,
                    gross_strokes: 4
                }
            )
        )
        .await
        .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM singles_match_notes")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM singles_match_audits")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn scorer_sql_context_cannot_append_organizer_award(pool: PgPool) {
    let (_, m, _) = ready(&pool).await;
    let scorer = fixture::session(&pool, 215, Some(11), "scorer", "sql-scorer").await;
    let event = serde_json::json!({"type":"award","winner_player_id":id(11),"reason":"Unauthorized ruling","after_hole":0});
    let mut tx = pool.begin().await.unwrap();
    ctx(
        &mut tx,
        scorer,
        m,
        serde_json::json!({"type":"report","event":event.clone()}),
    )
    .await;
    sqlx::query("SELECT set_config('app.match_actor',$1::text,true)")
        .bind(id(215))
        .execute(&mut *tx)
        .await
        .unwrap();
    let ledger = serde_json::json!([{"id":Uuid::new_v4(),"event":event}]);
    assert!(
        sqlx::query(
            "UPDATE singles_matches SET revision=revision+1,ledger=$2,terminal=TRUE WHERE id=$1"
        )
        .bind(m)
        .bind(ledger)
        .execute(&mut *tx)
        .await
        .is_err()
    );
    tx.rollback().await.unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn missing_historical_handicap_snapshot_fails_closed(pool: PgPool) {
    let (session, m, _) = ready(&pool).await;
    sqlx::raw_sql("ALTER TABLE round_handicap_snapshots DISABLE TRIGGER USER")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM round_handicap_snapshots WHERE round_id=$1 AND player_id=$2")
        .bind(id(5))
        .bind(id(11))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::raw_sql("ALTER TABLE round_handicap_snapshots ENABLE TRIGGER USER")
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        match_play::reads::get(&pool, session, id(5), m, false)
            .await
            .is_err()
    );
    assert!(
        match_play::execute(
            &pool,
            session,
            id(5),
            m,
            request(
                1,
                Command::Note {
                    player_id: id(11),
                    hole_number: 1,
                    gross_strokes: 4
                }
            )
        )
        .await
        .is_err()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM singles_match_receipts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
