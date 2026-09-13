use super::*;
use golf_api::{
    domain::{scorecards::ExpectedScore, stableford::card::Input},
    repositories::stableford,
    schema::MIGRATOR,
};
use serde_json::Value;
async fn snapshot(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('inputs',(SELECT jsonb_agg(to_jsonb(i) ORDER BY id) FROM stableford_inputs i),'audits',(SELECT jsonb_agg(to_jsonb(a) ORDER BY id) FROM stableford_input_audits a),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY request_id) FROM stableford_mutation_receipts r),'snapshots',(SELECT jsonb_agg(to_jsonb(s) ORDER BY player_id) FROM round_handicap_snapshots s),'confirmations',(SELECT jsonb_agg(to_jsonb(c) ORDER BY id) FROM scorecard_confirmations c))").fetch_one(pool).await.unwrap()
}
#[sqlx::test(migrations = false)]
async fn populated_schema29_stableford_receipts_scores_and_snapshots_survive_upgrade(pool: PgPool) {
    for migration in MIGRATOR.iter().filter(|m| m.version <= 29) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let session = fixture::ready(&pool).await;
    let request_id = Uuid::new_v4();
    let operation = || {
        let mut op = fixture::operation(
            session,
            11,
            1,
            Input::Numeric { gross_strokes: 4 },
            ExpectedScore::Absent {},
        );
        op.request_id = request_id;
        op
    };
    let acknowledgement = stableford::save_conditional(&pool, operation())
        .await
        .unwrap()
        .value;
    for hole in 2..=18 {
        fixture::numeric(&pool, session, 11, hole, 4).await;
    }
    stableford::confirm(&pool, session, id(5), id(11))
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    for migration in MIGRATOR.iter().filter(|m| m.version > 29) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    assert_eq!(before, snapshot(&pool).await);
    let replay = stableford::save_conditional(&pool, operation())
        .await
        .unwrap();
    assert_eq!(acknowledgement, replay.value);
    assert!(!replay.changed);
    assert_eq!(before, snapshot(&pool).await);
}

#[sqlx::test(migrations = false)]
async fn schema31_match_configuration_normalizes_without_changing_legacy_rows(pool: PgPool) {
    for migration in MIGRATOR.iter().filter(|m| m.version <= 31) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds,mandatory_round_id) VALUES('00000000-0000-0000-0000-000000000302','All match','2026-09-13','2026-09-13',1,1,NULL),('00000000-0000-0000-0000-000000000303','Mixed','2026-09-13','2026-09-13',2,2,'00000000-0000-0000-0000-000000000313'),('00000000-0000-0000-0000-000000000304','Legacy partial','2026-09-13','2026-09-13',2,2,NULL);INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES('00000000-0000-0000-0000-000000000312','00000000-0000-0000-0000-000000000302',1,'Match','2026-09-13','','','singles_match_play'),('00000000-0000-0000-0000-000000000313','00000000-0000-0000-0000-000000000303',1,'Match','2026-09-13','','','singles_match_play'),('00000000-0000-0000-0000-000000000314','00000000-0000-0000-0000-000000000303',2,'Stroke','2026-09-13','','','individual_stroke_play')").execute(&pool).await.unwrap();
    let before: Value = sqlx::query_scalar("SELECT to_jsonb(t) FROM tournaments t WHERE id=$1")
        .bind(id(0x304))
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/0032_match_overall_configuration.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let all: Option<i16> = sqlx::query_scalar("SELECT counted_rounds FROM tournaments WHERE id=$1")
        .bind(id(0x302))
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(all, None);
    let mixed: (Option<i16>, Option<Uuid>) =
        sqlx::query_as("SELECT counted_rounds,mandatory_round_id FROM tournaments WHERE id=$1")
            .bind(id(0x303))
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(mixed, (Some(1), None));
    let after: Value = sqlx::query_scalar("SELECT to_jsonb(t) FROM tournaments t WHERE id=$1")
        .bind(id(0x304))
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
}
