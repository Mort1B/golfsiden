use super::support::*;
use axum::{body::Body, http::Request};
use golf_api::{
    AppState, api,
    domain::{leaderboards::LeaderboardMetric, result_sharing::ResultShareToken},
    repositories::{result_sharing, round_lifecycle, scorecards},
};
use http_body_util::BodyExt;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn assert_basis(pool: &PgPool, session: uuid::Uuid, contributions: usize) {
    let app = api::router(AppState::new(pool.clone()));
    let token = ResultShareToken::generate().unwrap();
    let grant = result_sharing::issue(pool, session, id(2), None, &token.hash())
        .await
        .unwrap();
    for (name, metric) in [
        ("gross", LeaderboardMetric::Gross),
        ("net", LeaderboardMetric::Net),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(format!("/api/tournaments/{}/leaderboards/{name}", id(2)))
                    .header("cookie", format!("golf_session={TOKEN}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let board: Value =
            serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(board["entries"].as_array().unwrap().len(), 4);
        assert_eq!(board["included_round_ids"], serde_json::json!([]));
        for entry in board["entries"].as_array().unwrap() {
            assert_eq!(entry["value"]["type"], "overall_equivalent");
            assert_eq!(entry["value"]["version"], 1);
            assert!(entry.get("gross_total").is_none());
            assert_eq!(
                entry["contributions"].as_array().unwrap().len(),
                contributions
            );
            if contributions > 0 {
                assert!(entry["contributions"][0].get("value").is_none());
                assert_eq!(entry["contributions"][0]["round_id"], id(5).to_string());
            }
        }
        let public = serde_json::to_value(
            result_sharing::read(pool, grant.id, &token, metric)
                .await
                .unwrap()
                .results,
        )
        .unwrap();
        for entry in public["entries"].as_array().unwrap() {
            assert_eq!(entry["value"]["type"], "overall_equivalent");
            assert_eq!(entry["value"].as_object().unwrap().len(), 4);
            assert!(entry.get("gross_total").is_none());
        }
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn all_draft_stableford_uses_equivalents_in_private_api_and_public_projection(pool: PgPool) {
    let session = draft(&pool, "individual_stableford").await;
    assert_basis(&pool, session, 0).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn open_stroke_results_with_draft_stableford_keep_typed_overall_and_legacy_contributions(
    pool: PgPool,
) {
    let session = draft_rounds(&pool, "individual_stroke_play", 2).await;
    round_lifecycle::open(&pool, id(5)).await.unwrap();
    for player in [11, 12, 13, 14] {
        scorecards::save_conditional(
            &pool,
            scorecards::ConditionalSaveScore {
                request_id: uuid::Uuid::new_v4(),
                round_id: id(5),
                hole_id: id(101),
                owner: golf_api::domain::scorecards::ScoreOwner::Player { id: id(player) },
                gross_strokes: 4,
                expected_score: golf_api::domain::scorecards::ExpectedScore::Absent {},
                session_id: session,
            },
        )
        .await
        .unwrap();
    }
    assert_basis(&pool, session, 1).await;
}
