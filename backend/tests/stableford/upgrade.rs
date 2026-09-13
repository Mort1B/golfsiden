#[allow(dead_code)]
#[path = "../four_ball/support.rs"]
mod legacy;
use golf_api::{
    domain::{
        four_ball_card::Input,
        scorecards::{ExpectedScore, ScoreOwner},
    },
    repositories::{four_ball, round_lifecycle, scorecards},
    schema::MIGRATOR,
};
use legacy::*;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;
async fn snapshot(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM scores s),'score_audits',(SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM score_audits a),'score_receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY r.request_id) FROM score_mutation_receipts r),'inputs',(SELECT jsonb_agg(to_jsonb(i) ORDER BY i.id) FROM four_ball_inputs i),'input_audits',(SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM four_ball_input_audits a),'input_receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY r.request_id) FROM four_ball_mutation_receipts r),'confirmations',(SELECT jsonb_agg(to_jsonb(c) ORDER BY c.id) FROM scorecard_confirmations c),'snapshots',(SELECT jsonb_agg(to_jsonb(s) ORDER BY s.player_id) FROM round_handicap_snapshots s))").fetch_one(pool).await.unwrap()
}
async fn v28(pool: &PgPool) {
    for migration in MIGRATOR.iter().filter(|m| m.version <= 28) {
        sqlx::raw_sql(&migration.sql).execute(pool).await.unwrap();
    }
}
async fn upgrade(pool: &PgPool) {
    sqlx::raw_sql(include_str!("../../../migrations/0029_stableford.sql"))
        .execute(pool)
        .await
        .unwrap();
}
#[sqlx::test(migrations = false)]
async fn populated_v28_four_ball_data_and_immutable_receipt_survive_upgrade_and_replay(
    pool: PgPool,
) {
    v28(&pool).await;
    let session = ready(&pool).await;
    let request = Uuid::new_v4();
    let mut op = operation(
        session,
        11,
        1,
        Input::Numeric { gross_strokes: 4 },
        ExpectedScore::Absent {},
    );
    op.request_id = request;
    let receipt = four_ball::save_conditional(&pool, op).await.unwrap().value;
    for hole in 2..=18 {
        numeric(&pool, session, 11, hole, 4).await;
    }
    four_ball::confirm(&pool, session, id(5), id(21))
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    upgrade(&pool).await;
    assert_eq!(before, snapshot(&pool).await);
    let mut op = operation(
        session,
        11,
        1,
        Input::Numeric { gross_strokes: 4 },
        ExpectedScore::Absent {},
    );
    op.request_id = request;
    let replay = four_ball::save_conditional(&pool, op).await.unwrap();
    assert_eq!(replay.value, receipt);
    assert!(!replay.changed);
    assert_eq!(before, snapshot(&pool).await);
}
#[sqlx::test(migrations = false)]
async fn populated_v28_legacy_numeric_data_and_immutable_receipt_survive_upgrade(pool: PgPool) {
    v28(&pool).await;
    let session = draft(&pool, "team_scramble").await;
    round_lifecycle::open(&pool, id(5)).await.unwrap();
    let request = Uuid::new_v4();
    let op = || scorecards::ConditionalSaveScore {
        request_id: request,
        round_id: id(5),
        hole_id: id(101),
        owner: ScoreOwner::Team { id: id(21) },
        gross_strokes: 4,
        expected_score: ExpectedScore::Absent {},
        session_id: session,
    };
    let receipt = scorecards::save_conditional(&pool, op())
        .await
        .unwrap()
        .value;
    for hole in 2..=18 {
        scorecards::save_conditional(
            &pool,
            scorecards::ConditionalSaveScore {
                request_id: Uuid::new_v4(),
                round_id: id(5),
                hole_id: id(100 + hole),
                owner: ScoreOwner::Team { id: id(21) },
                gross_strokes: 4,
                expected_score: ExpectedScore::Absent {},
                session_id: session,
            },
        )
        .await
        .unwrap();
    }
    scorecards::confirm_authenticated(&pool, id(5), ScoreOwner::Team { id: id(21) }, session)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    upgrade(&pool).await;
    assert_eq!(before, snapshot(&pool).await);
    let replay = scorecards::save_conditional(&pool, op()).await.unwrap();
    assert_eq!(replay.value, receipt);
    assert!(!replay.changed);
    assert_eq!(before, snapshot(&pool).await);
}
