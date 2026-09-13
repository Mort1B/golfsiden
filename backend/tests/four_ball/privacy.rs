use super::support::*;
use golf_api::{
    domain::{
        four_ball_card::{Input, read_projection},
        leaderboards::LeaderboardMetric,
        result_sharing::ResultShareToken,
        scorecards::ExpectedScore,
    },
    repositories::{four_ball, leaderboards, result_sharing, round_completion},
};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;
async fn outputs(pool: &PgPool, session: Uuid, grant: Uuid, token: &ResultShareToken) -> Value {
    let card = read_projection(
        four_ball::get(pool, session, id(5), id(21), false)
            .await
            .unwrap(),
    );
    let completion = round_completion::validation_for_member(pool, id(41), id(5))
        .await
        .unwrap();
    let mut metrics = Vec::new();
    for metric in [LeaderboardMetric::Gross, LeaderboardMetric::Net] {
        metrics.push(json!({"round":leaderboards::round_for_member(pool,id(41),id(5),metric).await.unwrap(),"overall":leaderboards::tournament_for_member(pool,id(41),id(2),metric).await.unwrap(),"public":result_sharing::read(pool,grant,token,metric).await.unwrap().results}));
    }
    json!({"card":card,"completion":completion,"metrics":metrics})
}
async fn change(pool: &PgPool, session: Uuid, player: u128, hole: u128, input: Input) {
    let side = if player <= 12 { 21 } else { 22 };
    let card = four_ball::get(pool, session, id(5), id(side), true)
        .await
        .unwrap();
    let entry = card
        .holes
        .iter()
        .find(|h| h.hole_id == id(100 + hole))
        .unwrap()
        .players
        .iter()
        .find(|p| p.player_id == id(player))
        .unwrap()
        .score
        .as_ref();
    let expected = entry
        .map(|e| ExpectedScore::Present {
            score_id: e.id,
            revision: e.revision,
        })
        .unwrap_or(ExpectedScore::Absent {});
    four_ball::save_conditional(pool, operation(session, player, hole, input, expected))
        .await
        .unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn hidden_changes_are_invisible_to_round_completion_overall_public_and_history(pool: PgPool) {
    let admin = ready(&pool).await;
    fill(&pool, admin).await;
    let viewer = session(&pool, 41, None, "viewer", "projection-viewer").await;
    let token = ResultShareToken::generate().unwrap();
    let grant = result_sharing::issue(&pool, admin, id(2), None, &token.hash())
        .await
        .unwrap();
    let before = outputs(&pool, viewer, grant.id, &token).await;
    assert_eq!(
        before["metrics"][0]["round"]["entries"][0]["holes_scored"],
        9
    );
    assert_eq!(
        before["metrics"][0]["public"]["entries"][0]["provisional_holes_scored"],
        9
    );
    for (player, input) in [
        (11, Input::Numeric { gross_strokes: 3 }),
        (12, Input::Numeric { gross_strokes: 4 }),
        (12, Input::NoScore {}),
    ] {
        change(&pool, admin, player, 18, input).await;
        assert_eq!(before, outputs(&pool, viewer, grant.id, &token).await);
    }
    for team in [21, 22] {
        four_ball::confirm(&pool, admin, id(5), id(team))
            .await
            .unwrap();
    }
    round_completion::complete(&pool, id(5)).await.unwrap();
    let completed = outputs(&pool, viewer, grant.id, &token).await;
    assert_eq!(
        completed["metrics"][0]["overall"]["included_round_ids"],
        json!([])
    );
    assert!(
        completed["metrics"][0]["public"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["counted_contributions"] == 0)
    );
    change(&pool, admin, 11, 18, Input::NoScore {}).await;
    assert_eq!(completed, outputs(&pool, viewer, grant.id, &token).await);
}
#[sqlx::test(migrations = "../migrations")]
async fn completed_pickup_remains_readable_excludes_incomplete_side_and_restore_qualifies(
    pool: PgPool,
) {
    let admin = ready(&pool).await;
    fill(&pool, admin).await;
    for team in [21, 22] {
        four_ball::confirm(&pool, admin, id(5), id(team))
            .await
            .unwrap();
    }
    round_completion::complete(&pool, id(5)).await.unwrap();
    change(&pool, admin, 11, 18, Input::NoScore {}).await;
    let round = leaderboards::round(&pool, id(5), LeaderboardMetric::Net)
        .await
        .unwrap();
    let side = round
        .entries
        .iter()
        .find(|e| e.owner == golf_api::domain::leaderboards::LeaderboardOwner::Team { id: id(21) })
        .unwrap();
    assert_eq!(side.complete, Some(false));
    assert_eq!(side.holes_scored, 17);
    let overall = leaderboards::tournament(&pool, id(2), LeaderboardMetric::Net)
        .await
        .unwrap();
    for player in [11, 12] {
        let row = overall
            .entries
            .iter()
            .find(|e| e.player_id == id(player))
            .unwrap();
        assert!(!row.eligible);
        assert!(row.contributions.is_empty());
    }
    assert!(round_completion::lock(&pool, id(5)).await.is_err());
    change(&pool, admin, 11, 18, Input::Numeric { gross_strokes: 4 }).await;
    four_ball::confirm(&pool, admin, id(5), id(21))
        .await
        .unwrap();
    let overall = leaderboards::tournament(&pool, id(2), LeaderboardMetric::Net)
        .await
        .unwrap();
    for player in [11, 12] {
        let row = overall
            .entries
            .iter()
            .find(|e| e.player_id == id(player))
            .unwrap();
        assert!(row.eligible);
        assert_eq!(row.contributions.len(), 1);
    }
    round_completion::lock(&pool, id(5)).await.unwrap();
}
