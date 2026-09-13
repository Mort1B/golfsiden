use super::*;
use axum::{
    body::Body,
    http::{Request as HttpRequest, StatusCode},
};
use golf_api::{
    AppState, api,
    auth::hash_session_token,
    domain::{
        leaderboards::LeaderboardMetric,
        result_sharing::ResultShareToken,
        scorecards::{ExpectedScore, ScoreOwner},
    },
    repositories::{auth, leaderboards, result_sharing, round_completion, scorecards},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;
async fn get(app: axum::Router, path: String, token: &str) -> (StatusCode, Value) {
    let r = app
        .oneshot(
            HttpRequest::get(path)
                .header("cookie", format!("golf_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    let json = serde_json::from_slice(&r.into_body().collect().await.unwrap().to_bytes()).unwrap();
    (status, json)
}
#[sqlx::test(migrations = "../migrations")]
async fn all_match_overall_is_typed_unavailable_and_legacy_routes_authorize_then_exclude(
    pool: PgPool,
) {
    let (session, _, _) = ready(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    for metric in ["gross", "net"] {
        let (status, value) = get(
            app.clone(),
            format!("/api/tournaments/{}/leaderboards/{metric}", id(2)),
            TOKEN,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            value,
            json!({"type":"not_applicable","reason":"match_only","tournament_id":id(2),"metric":metric})
        );
    }
    let token = ResultShareToken::generate().unwrap();
    assert!(matches!(
        result_sharing::issue(&pool, session, id(2), None, &token.hash()).await,
        Err(result_sharing::ShareError::OverallUnavailable)
    ));
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES($1,'match_outsider','Outsider','player')").bind(id(220)).execute(&pool).await.unwrap();
    auth::create_session(
        &pool,
        id(220),
        &hash_session_token("outsider-match"),
        chrono::Utc::now() + chrono::Duration::hours(1),
    )
    .await
    .unwrap();
    let (status, _) = get(
        app.clone(),
        format!("/api/tournaments/{}/leaderboards/gross", id(2)),
        "outsider-match",
    )
    .await;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let paths = [
        format!("/api/rounds/{}/leaderboards/gross", id(5)),
        format!("/api/rounds/{}/scorecards/player/{}", id(5), id(11)),
        format!("/api/rounds/{}/scorecards/player/{}/scoring", id(5), id(11)),
        format!("/api/rounds/{}/completion-validation", id(5)),
    ];
    for path in paths {
        let (status, _) = get(app.clone(), path.clone(), TOKEN).await;
        assert!(
            status == StatusCode::CONFLICT || status == StatusCode::NOT_FOUND,
            "{path} {status}"
        );
        let (status, _) = get(app.clone(), path.clone(), "outsider-match").await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{path}");
    }
    let (status, value) = get(app, format!("/api/rounds/{}/score-access", id(5)), TOKEN).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(value["writable_owners"], json!([]));
}
#[sqlx::test(migrations = "../migrations")]
async fn mixed_stroke_draft_stableford_and_open_final_match_keep_units_current_round_and_ties(
    pool: PgPool,
) {
    let session = fixture::draft_rounds(&pool, "individual_stroke_play", 3).await;
    for (flight, players) in [(231, [11, 12]), (232, [13, 14])] {
        sqlx::query("INSERT INTO flights(id,round_id,tournament_id,name) VALUES($1,$2,$3,$4)")
            .bind(id(flight))
            .bind(id(7))
            .bind(id(2))
            .bind(format!("Flight {flight}"))
            .execute(&pool)
            .await
            .unwrap();
        for player in players {
            sqlx::query("INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(id(flight)).bind(id(7)).bind(id(2)).bind(id(player)).execute(&pool).await.unwrap();
        }
    }
    let updated = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
        .bind(id(7))
        .fetch_one(&pool)
        .await
        .unwrap();
    match_play::setup::replace(
        &pool,
        session,
        id(7),
        Assignments {
            expected_round_updated_at: updated,
            matches: vec![
                Pair {
                    first_player_id: id(11),
                    second_player_id: id(12),
                },
                Pair {
                    first_player_id: id(13),
                    second_player_id: id(14),
                },
            ],
        },
    )
    .await
    .unwrap();
    round_lifecycle::open(&pool, id(5)).await.unwrap();
    round_lifecycle::open(&pool, id(7)).await.unwrap();
    for player in [11, 12, 13, 14] {
        for hole in 1..=18 {
            scorecards::save_conditional(
                &pool,
                scorecards::ConditionalSaveScore {
                    request_id: Uuid::new_v4(),
                    round_id: id(5),
                    hole_id: id(100 + hole),
                    owner: ScoreOwner::Player { id: id(player) },
                    gross_strokes: 4,
                    expected_score: ExpectedScore::Absent {},
                    session_id: session,
                },
            )
            .await
            .unwrap();
        }
    }
    sqlx::query("UPDATE tournaments SET final_round_back_nine_hidden=TRUE WHERE id=$1")
        .bind(id(2))
        .execute(&pool)
        .await
        .unwrap();
    let viewer = fixture::session(&pool, 221, None, "viewer", "mixed-viewer").await;
    let board =
        leaderboards::tournament_for_member(&pool, id(221), id(2), LeaderboardMetric::Gross)
            .await
            .unwrap();
    assert_eq!(board.current_round_id, Some(id(5)));
    assert_eq!(board.entries[0].contributions[0].holes_scored, 18);
    let json = serde_json::to_value(&board).unwrap();
    assert_eq!(json["entries"][0]["value"]["type"], "overall_equivalent");
    assert!(json["entries"][0].get("gross_total").is_none());
    for player in [11, 12, 13, 14] {
        scorecards::confirm_authenticated(
            &pool,
            id(5),
            ScoreOwner::Player { id: id(player) },
            session,
        )
        .await
        .unwrap();
    }
    round_completion::complete_authorized(&pool, session, id(5))
        .await
        .unwrap();
    // Final scheduled match provides no comparable gross tie-break.
    let board =
        leaderboards::tournament_for_member(&pool, id(221), id(2), LeaderboardMetric::Gross)
            .await
            .unwrap();
    assert_eq!(board.current_round_id, None);
    assert_eq!(board.included_round_ids, vec![id(5)]);
    assert!(
        board
            .entries
            .iter()
            .all(|e| e.position == Some(1) && e.tie_break_score_to_par.is_none())
    );
    let token = ResultShareToken::generate().unwrap();
    let grant = result_sharing::issue(&pool, session, id(2), None, &token.hash())
        .await
        .unwrap();
    let public = serde_json::to_value(
        result_sharing::read(&pool, grant.id, &token, LeaderboardMetric::Gross)
            .await
            .unwrap()
            .results,
    )
    .unwrap();
    assert!(
        public["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["value"].as_object().unwrap().len() == 4)
    );
    assert!(
        match_play::reads::table(&pool, viewer, id(2))
            .await
            .unwrap()
            .entries
            .iter()
            .all(|e| e.played == 0)
    );
}
