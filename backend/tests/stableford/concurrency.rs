use super::support::*;
use golf_api::{
    domain::{scorecards::ExpectedScore, stableford::card::Input},
    repositories::{
        round_completion,
        scorecards::{ScorecardConflict, ScorecardError},
        stableford,
    },
};
use sqlx::PgPool;
use std::time::Duration;
async fn wait_lock(pool: &PgPool) {
    tokio::time::timeout(Duration::from_secs(4),async{loop{let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock')").fetch_one(pool).await.unwrap();if waiting{break;}tokio::time::sleep(Duration::from_millis(10)).await;}}).await.unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn pending_input_waits_for_round_lock_then_rejects_locked_round(pool: PgPool) {
    let session = ready(&pool).await;
    fill(&pool, session).await;
    for team in [11, 12, 13, 14] {
        stableford::confirm(&pool, session, id(5), id(team))
            .await
            .unwrap();
    }
    round_completion::complete(&pool, id(5)).await.unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(id(5))
        .execute(&mut *tx)
        .await
        .unwrap();
    let write_pool = pool.clone();
    let write = tokio::spawn(async move {
        stableford::save_conditional(
            &write_pool,
            operation(session, 12, 1, Input::NoScore {}, ExpectedScore::Absent {}),
        )
        .await
    });
    wait_lock(&pool).await;
    sqlx::query("SELECT set_config('app.round_lock_id',$1::text,true)")
        .bind(id(5))
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET status='locked' WHERE id=$1")
        .bind(id(5))
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    assert!(matches!(
        write.await.unwrap(),
        Err(ScorecardError::Conflict(
            ScorecardConflict::RoundNotEditable
        ))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stableford_inputs WHERE player_id=$1 AND round_id='00000000-0000-0000-0000-000000000005'")
            .bind(id(12))
            .fetch_one(&pool)
            .await
            .unwrap(),
        18
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn blocked_delivery_rechecks_revoked_authority_before_input_or_receipt(pool: PgPool) {
    ready(&pool).await;
    let player = session(&pool, 41, Some(11), "player", "revoke-player").await;
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(id(5))
        .execute(&mut *tx)
        .await
        .unwrap();
    let write_pool = pool.clone();
    let write = tokio::spawn(async move {
        stableford::save_conditional(
            &write_pool,
            operation(player, 11, 1, Input::NoScore {}, ExpectedScore::Absent {}),
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
        Err(ScorecardError::Unauthenticated)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stableford_inputs")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn concurrent_same_hole_expected_absence_has_one_winner_and_one_conflict(pool: PgPool) {
    let session = ready(&pool).await;
    let a = operation(
        session,
        11,
        1,
        Input::Numeric { gross_strokes: 4 },
        ExpectedScore::Absent {},
    );
    let b = operation(session, 11, 1, Input::NoScore {}, ExpectedScore::Absent {});
    let (a, b) = tokio::join!(
        stableford::save_conditional(&pool, a),
        stableford::save_conditional(&pool, b)
    );
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    let failure = if a.is_err() { a } else { b };
    assert!(matches!(
        failure,
        Err(ScorecardError::Conflict(ScorecardConflict::VersionConflict))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stableford_input_audits")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stableford_mutation_receipts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}

async fn second_round(pool: &PgPool, session: uuid::Uuid) {
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Other cup','2026-09-13','2026-09-13',1)").bind(id(202)).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(id(202))
    .bind(id(1))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) SELECT $1,player_id,tournament_handicap FROM tournament_players WHERE tournament_id=$2").bind(id(202)).bind(id(2)).execute(pool).await.unwrap();
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_id,course_name,tee_id,tee_name,number_of_holes,scoring_format,handicap_allowance_percent) SELECT $1,$2,1,name,round_date,course_id,course_name,tee_id,tee_name,number_of_holes,scoring_format,handicap_allowance_percent FROM rounds WHERE id=$3").bind(id(205)).bind(id(202)).bind(id(5)).execute(pool).await.unwrap();
    for (_team, flight, players) in [(221, 231, [11, 12]), (222, 232, [13, 14])] {
        sqlx::query("INSERT INTO flights(id,round_id,tournament_id,name) VALUES($1,$2,$3,$4)")
            .bind(id(flight))
            .bind(id(205))
            .bind(id(202))
            .bind(format!("Flight {flight}"))
            .execute(pool)
            .await
            .unwrap();
        for player in players {
            sqlx::query("INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(id(flight)).bind(id(205)).bind(id(202)).bind(id(player)).execute(pool).await.unwrap();
        }
    }
    let updated = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(id(202))
        .fetch_one(pool)
        .await
        .unwrap();
    golf_api::repositories::tournaments::start_authorized(pool, session, id(202), updated)
        .await
        .unwrap();
    golf_api::repositories::round_lifecycle::open(pool, id(205))
        .await
        .unwrap();
}
async fn hold_receipt(
    pool: &PgPool,
    actor: uuid::Uuid,
    request: uuid::Uuid,
) -> sqlx::Transaction<'_, sqlx::Postgres> {
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true),set_config('app.stableford_actor_id',$2::text,true),set_config('app.stableford_request_id',$3::text,true)").bind(id(5)).bind(actor).bind(request).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO stableford_mutation_receipts(user_id,request_id,round_id,request_hash,applied_score_id,applied_revision) SELECT $1,$2,round_id,decode(repeat('00',32),'hex'),id,revision FROM stableford_inputs WHERE round_id=$3 AND player_id=$4 AND hole_id=$5").bind(actor).bind(request).bind(id(5)).bind(id(11)).bind(id(101)).execute(&mut *tx).await.unwrap();
    tx
}
async fn wait_receipt(pool: &PgPool) {
    tokio::time::timeout(Duration::from_secs(4),async{loop{let waiting:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE 'INSERT INTO stableford_mutation_receipts%')").fetch_one(pool).await.unwrap();if waiting{break;}tokio::time::sleep(Duration::from_millis(10)).await;}}).await.unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn cross_round_receipt_wait_rolls_back_input_audit_and_player_confirmation(pool: PgPool) {
    let session = ready(&pool).await;
    fill(&pool, session).await;
    second_round(&pool, session).await;
    for hole in 1..=18 {
        let mut op = operation(
            session,
            11,
            hole,
            Input::Numeric { gross_strokes: 4 },
            ExpectedScore::Absent {},
        );
        op.round_id = id(205);
        stableford::save_conditional(&pool, op).await.unwrap();
    }
    for hole in 1..=18 {
        let mut op = operation(
            session,
            12,
            hole,
            Input::Numeric { gross_strokes: 4 },
            ExpectedScore::Absent {},
        );
        op.round_id = id(205);
        stableford::save_conditional(&pool, op).await.unwrap();
    }
    stableford::confirm(&pool, session, id(205), id(12))
        .await
        .unwrap();
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM stableford_input_audits")
        .fetch_one(&pool)
        .await
        .unwrap();
    let card = stableford::get(&pool, session, id(205), id(12), true)
        .await
        .unwrap();
    let entry = card.holes[0].score.as_ref().unwrap();
    let expected = ExpectedScore::Present {
        score_id: entry.id,
        revision: entry.revision,
    };
    let mut op = operation(session, 12, 1, Input::NoScore {}, expected);
    op.round_id = id(205);
    let winner = hold_receipt(&pool, id(1), op.request_id).await;
    let write_pool = pool.clone();
    let write = tokio::spawn(async move { stableford::save_conditional(&write_pool, op).await });
    wait_receipt(&pool).await;
    winner.commit().await.unwrap();
    assert!(matches!(
        write.await.unwrap(),
        Err(ScorecardError::Conflict(ScorecardConflict::RequestMismatch))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stableford_inputs WHERE player_id=$1 AND round_id='00000000-0000-0000-0000-0000000000cd'")
            .bind(id(12))
            .fetch_one(&pool)
            .await
            .unwrap(),
        18
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stableford_input_audits")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        stableford::get(&pool, session, id(205), id(12), true)
            .await
            .unwrap()
            .confirmed,
        Some(true)
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_after_receipt_wait_rolls_back_and_revoked_receipt_replay_is_denied(pool: PgPool) {
    let admin = ready(&pool).await;
    fill(&pool, admin).await;
    stableford::confirm(&pool, admin, id(5), id(11))
        .await
        .unwrap();
    let player = session(&pool, 41, Some(11), "player", "expires-player").await;
    second_round(&pool, admin).await;
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'player')",
    )
    .bind(id(202))
    .bind(id(41))
    .execute(&pool)
    .await
    .unwrap();
    for hole in 1..=18 {
        let mut op = operation(
            admin,
            11,
            hole,
            Input::Numeric { gross_strokes: 4 },
            ExpectedScore::Absent {},
        );
        op.round_id = id(205);
        stableford::save_conditional(&pool, op).await.unwrap();
    }
    for hole in 1..=18 {
        let mut op = operation(
            admin,
            12,
            hole,
            Input::Numeric { gross_strokes: 4 },
            ExpectedScore::Absent {},
        );
        op.round_id = id(205);
        stableford::save_conditional(&pool, op).await.unwrap();
    }
    stableford::confirm(&pool, admin, id(205), id(12))
        .await
        .unwrap();
    let before: i64 = sqlx::query_scalar("SELECT count(*) FROM stableford_input_audits")
        .fetch_one(&pool)
        .await
        .unwrap();
    let card = stableford::get(&pool, player, id(205), id(12), true)
        .await
        .unwrap();
    let entry = card.holes[0].score.as_ref().unwrap();
    let expected = ExpectedScore::Present {
        score_id: entry.id,
        revision: entry.revision,
    };
    let mut op = operation(player, 12, 1, Input::NoScore {}, expected);
    op.round_id = id(205);
    let winner = hold_receipt(&pool, id(41), op.request_id).await;
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1",
    )
    .bind(player)
    .execute(&pool)
    .await
    .unwrap();
    let write_pool = pool.clone();
    let write = tokio::spawn(async move { stableford::save_conditional(&write_pool, op).await });
    wait_receipt(&pool).await;
    tokio::time::sleep(Duration::from_millis(2100)).await;
    winner.rollback().await.unwrap();
    assert!(matches!(
        write.await.unwrap(),
        Err(ScorecardError::Unauthenticated)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stableford_inputs WHERE player_id=$1 AND round_id='00000000-0000-0000-0000-0000000000cd'")
            .bind(id(12))
            .fetch_one(&pool)
            .await
            .unwrap(),
        18
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM stableford_input_audits")
            .fetch_one(&pool)
            .await
            .unwrap(),
        before
    );
    assert_eq!(
        stableford::get(&pool, admin, id(205), id(12), true)
            .await
            .unwrap()
            .confirmed,
        Some(true)
    );
    let entry = stableford::get(&pool, admin, id(5), id(12), true)
        .await
        .unwrap()
        .holes[1]
        .score
        .clone()
        .unwrap();
    let expected = ExpectedScore::Present {
        score_id: entry.id,
        revision: entry.revision,
    };
    let mut replay = operation(admin, 12, 2, Input::NoScore {}, expected);
    let request = replay.request_id;
    stableford::save_conditional(&pool, replay).await.unwrap();
    sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(admin)
        .execute(&pool)
        .await
        .unwrap();
    replay = operation(admin, 12, 2, Input::NoScore {}, expected);
    replay.request_id = request;
    assert!(matches!(
        stableford::save_conditional(&pool, replay).await,
        Err(ScorecardError::Unauthenticated)
    ));
}
