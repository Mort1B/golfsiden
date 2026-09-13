use super::support::*;
use golf_api::repositories::{result_sharing, round_lifecycle, tournaments};
use serde_json::Value;
use sqlx::PgPool;
use uuid::{Uuid, uuid};
#[sqlx::test(migrations = "../migrations")]
async fn database_guards_hash_lifetime_identity_terminal_and_derived_audits(pool: PgPool) {
    let session = fixture(&pool).await;
    let (grant, _) = issue(&pool, session, None).await;
    for sql in [
        "UPDATE tournament_result_shares SET token_hash=decode(repeat('00',32),'hex') WHERE id=$1",
        "UPDATE tournament_result_shares SET expires_at=expires_at+interval '1 day' WHERE id=$1",
        "DELETE FROM tournament_result_shares WHERE id=$1",
        "UPDATE tournament_result_share_audits SET action='revoked' WHERE grant_id=$1",
        "DELETE FROM tournament_result_share_audits WHERE grant_id=$1",
        "INSERT INTO tournament_result_share_audits(tournament_id,grant_id,actor_user_id,action) SELECT tournament_id,id,created_by,'issued' FROM tournament_result_shares WHERE id=$1",
    ] {
        assert!(
            sqlx::query(sql)
                .bind(grant.id)
                .execute(&pool)
                .await
                .is_err()
        );
    }
    result_sharing::revoke(&pool, session, TRIP, grant.id)
        .await
        .unwrap();
    assert!(
        sqlx::query(
            "UPDATE tournament_result_shares SET revoked_at=NULL,revoked_by=NULL WHERE id=$1"
        )
        .bind(grant.id)
        .execute(&pool)
        .await
        .is_err()
    );
    // A stale credential generation cannot forge a DB workflow context.
    sqlx::query("UPDATE users SET password_hash='changed-test-hash' WHERE id=$1")
        .bind(ADMIN)
        .execute(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    context(&mut tx, session).await;
    assert!(sqlx::query("INSERT INTO tournament_result_shares(id,tournament_id,token_hash,created_by,created_at,expires_at) VALUES($1,$2,decode(repeat('00',32),'hex'),$3,now(),now()+interval '30 days')").bind(Uuid::new_v4()).bind(TRIP).bind(ADMIN).execute(&mut *tx).await.is_err());
}
#[sqlx::test(migrations = "../migrations")]
async fn deleting_parent_tournament_intentionally_cascades_capability_and_its_audits(pool: PgPool) {
    let session = fixture(&pool).await;
    let trip = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Temporary','2026-09-01','2026-09-01',1)").bind(trip).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(trip)
    .bind(ADMIN)
    .execute(&pool)
    .await
    .unwrap();
    let token = golf_api::domain::result_sharing::ResultShareToken::generate().unwrap();
    let grant = result_sharing::issue(&pool, session, trip, None, &token.hash())
        .await
        .unwrap();
    sqlx::query("DELETE FROM tournaments WHERE id=$1")
        .bind(trip)
        .execute(&pool)
        .await
        .unwrap();
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM tournament_result_share_audits WHERE grant_id=$1")
            .bind(grant.id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0);
    assert!(matches!(
        result_sharing::read(
            &pool,
            grant.id,
            &token,
            golf_api::domain::leaderboards::LeaderboardMetric::Gross
        )
        .await,
        Err(result_sharing::ShareError::Unavailable)
    ));
}
#[sqlx::test(migrations = false)]
async fn schema25_upgrade_retains_actual_history_and_does_not_create_links(pool: PgPool) {
    for migration in golf_api::schema::MIGRATOR.iter().filter(|m| m.version < 26) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let session = fixture(&pool).await;
    let version = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap();
    tournaments::start_authorized(&pool, session, TRIP, version)
        .await
        .unwrap();
    let round = uuid!("00000000-0000-0000-0000-000000004001");
    round_lifecycle::open_authorized(&pool, session, round)
        .await
        .unwrap();
    // Insert using the historical workflow without decoding a current-schema
    // ScoreEntry, which now requires the revision column introduced in 0027.
    sqlx::raw_sql("BEGIN; SELECT set_config('app.score_mutation_round_id','00000000-0000-0000-0000-000000004001',true); INSERT INTO scores(id,round_id,tournament_id,hole_id,team_id,gross_strokes,submitted_by) VALUES(gen_random_uuid(),'00000000-0000-0000-0000-000000004001','00000000-0000-0000-0000-000000002001','00000000-0000-0000-0000-000000003201','00000000-0000-0000-0000-000000005001',5,'00000000-0000-0000-0000-000000000001'); COMMIT;").execute(&pool).await.unwrap();
    let query = "SELECT jsonb_build_object('tournaments',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tournaments t),'scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM scores s),'snapshots',(SELECT jsonb_agg(to_jsonb(h) ORDER BY round_id,player_id) FROM round_handicap_snapshots h))";
    let before: Value = sqlx::query_scalar(query).fetch_one(&pool).await.unwrap();
    assert!(!before["scores"].as_array().unwrap().is_empty());
    assert!(!before["snapshots"].as_array().unwrap().is_empty());
    sqlx::raw_sql(include_str!(
        "../../../migrations/0026_public_result_sharing.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        before,
        sqlx::query_scalar::<_, Value>(query)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_result_shares")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let (grant, _) = issue(&pool, session, None).await;
    assert!(grant.revoked_at.is_none());
}
