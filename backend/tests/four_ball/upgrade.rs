use super::support::*;
use golf_api::{
    domain::scorecards::{ExpectedScore, ScoreOwner},
    repositories::{round_lifecycle, scorecards},
    schema::MIGRATOR,
};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;
async fn snapshot(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('scores',(SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM scores s),'audits',(SELECT jsonb_agg(to_jsonb(a) ORDER BY a.id) FROM score_audits a),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY r.request_id) FROM score_mutation_receipts r),'confirmations',(SELECT jsonb_agg(to_jsonb(c) ORDER BY c.id) FROM scorecard_confirmations c),'snapshots',(SELECT jsonb_agg(to_jsonb(h) ORDER BY h.player_id) FROM round_handicap_snapshots h))").fetch_one(pool).await.unwrap()
}
#[sqlx::test(migrations = false)]
async fn populated_v27_upgrade_preserves_legacy_scores_audits_receipts_confirmations_and_replay(
    pool: PgPool,
) {
    for migration in MIGRATOR.iter().filter(|m| m.version <= 27) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let session = draft(&pool, "team_scramble").await;
    round_lifecycle::open(&pool, id(5)).await.unwrap();
    let request = Uuid::new_v4();
    let mut first = None;
    for hole in 1..=18 {
        let result = scorecards::save_conditional(
            &pool,
            scorecards::ConditionalSaveScore {
                request_id: if hole == 1 { request } else { Uuid::new_v4() },
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
        if hole == 1 {
            first = Some(result.value);
        }
    }
    scorecards::confirm_authenticated(&pool, id(5), ScoreOwner::Team { id: id(21) }, session)
        .await
        .unwrap();
    let before = snapshot(&pool).await;
    sqlx::raw_sql(include_str!("../../../migrations/0028_four_ball.sql"))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(before, snapshot(&pool).await);
    let replay = scorecards::save_conditional(
        &pool,
        scorecards::ConditionalSaveScore {
            request_id: request,
            round_id: id(5),
            hole_id: id(101),
            owner: ScoreOwner::Team { id: id(21) },
            gross_strokes: 4,
            expected_score: ExpectedScore::Absent {},
            session_id: session,
        },
    )
    .await
    .unwrap();
    assert_eq!(Some(replay.value), first);
    assert!(!replay.changed);
    assert_eq!(before, snapshot(&pool).await);
}
