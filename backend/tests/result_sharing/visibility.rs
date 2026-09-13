use super::support::*;
use axum::http::StatusCode;
use golf_api::{
    AppState, api,
    domain::{models::TournamentTieBreakPolicy, scorecards::ScoreOwner},
    repositories::{
        round_completion, round_lifecycle, scorecards, tournament_visibility, tournaments,
    },
};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};
const EARLIER: Uuid = uuid!("00000000-0000-0000-0000-000000004003");
const FINAL: Uuid = uuid!("00000000-0000-0000-0000-000000004005");
async fn save_round(pool: &PgPool, round: Uuid, extra: i16, back_nine_extra: i16) {
    let holes:Vec<(Uuid,i16,i16)>=sqlx::query_as("SELECT h.id,h.hole_number,h.par FROM holes h JOIN rounds r ON r.tee_id=h.tee_id WHERE r.id=$1 ORDER BY h.hole_number").bind(round).fetch_all(pool).await.unwrap();
    let players: Vec<Uuid> = sqlx::query_scalar(
        "SELECT player_id FROM round_handicap_snapshots WHERE round_id=$1 ORDER BY player_id",
    )
    .bind(round)
    .fetch_all(pool)
    .await
    .unwrap();
    for (index, player) in players.iter().enumerate() {
        let owner = ScoreOwner::Player { id: *player };
        for (hole, number, par) in &holes {
            scorecards::save(
                pool,
                scorecards::SaveScore {
                    round_id: round,
                    hole_id: *hole,
                    owner,
                    gross_strokes: *par
                        + extra
                        + if round == FINAL { index as i16 } else { 0 }
                        + if *number > 9 { back_nine_extra } else { 0 },
                    submitted_by: ADMIN,
                },
            )
            .await
            .unwrap();
        }
    }
}
async fn complete(pool: &PgPool, round: Uuid) {
    let players: Vec<Uuid> = sqlx::query_scalar(
        "SELECT player_id FROM round_handicap_snapshots WHERE round_id=$1 ORDER BY player_id",
    )
    .bind(round)
    .fetch_all(pool)
    .await
    .unwrap();
    for player in players {
        scorecards::confirm(pool, round, ScoreOwner::Player { id: player }, ADMIN)
            .await
            .unwrap();
    }
    round_completion::complete(pool, round).await.unwrap();
}
fn allowlist(value: &Value) {
    let mut top = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    top.sort_unstable();
    let mut expected = vec![
        "grant_id",
        "expires_at",
        "tournament_name",
        "metric",
        "required_counted_rounds",
        "final_round_number",
        "tie_break_policy",
        "visibility",
        "entries",
    ];
    expected.sort_unstable();
    assert_eq!(top, expected);
    for entry in value["entries"].as_array().unwrap() {
        let mut fields = entry
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>();
        fields.sort_unstable();
        let mut expected = vec![
            "position",
            "tied",
            "display_name",
            "completed_rounds",
            "counted_contributions",
            "eligible",
            "total",
            "par_total",
            "score_to_par",
            "provisional",
            "provisional_holes_scored",
            "tie_break_score_to_par",
        ];
        expected.sort_unstable();
        assert_eq!(fields, expected);
    }
}
#[sqlx::test(migrations = "../migrations")]
async fn public_projection_ignores_admin_cookies_and_hidden_back_nine_changes_and_rehides_ties(
    pool: PgPool,
) {
    let session = fixture(&pool).await;
    let version = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap();
    let settings = tournaments::update_counted_rounds_authorized(
        &pool,
        session,
        TRIP,
        1,
        None,
        Some(TournamentTieBreakPolicy::FinalRoundScore),
        version,
    )
    .await
    .unwrap();
    tournaments::start_authorized(&pool, session, TRIP, settings.tournament.updated_at)
        .await
        .unwrap();
    let (grant, token) = issue(&pool, session, None).await;
    round_lifecycle::open_authorized(&pool, session, EARLIER)
        .await
        .unwrap();
    save_round(&pool, EARLIER, 0, 0).await;
    complete(&pool, EARLIER).await;
    round_lifecycle::open_authorized(&pool, session, FINAL)
        .await
        .unwrap();
    save_round(&pool, FINAL, 1, 0).await;
    let app = api::router(AppState::new(pool.clone()));
    for metric in ["gross", "net"] {
        save_round(&pool, FINAL, 1, 0).await;
        let (status, anonymous) =
            response(&app, public_request(grant.id, &token, metric, None)).await;
        assert_eq!(status, StatusCode::OK);
        allowlist(&anonymous);
        assert_eq!(
            anonymous,
            response(
                &app,
                public_request(grant.id, &token, metric, Some(ADMIN_TOKEN))
            )
            .await
            .1
        );
        save_round(&pool, FINAL, 1, 2).await;
        assert_eq!(
            anonymous,
            response(&app, public_request(grant.id, &token, metric, None))
                .await
                .1
        );
    }
    complete(&pool, FINAL).await;
    let hidden = response(&app, public_request(grant.id, &token, "gross", None))
        .await
        .1;
    assert!(
        hidden["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["position"] == 1
                && e["tie_break_score_to_par"].is_null()
                && e["completed_rounds"] == 1)
    );
    let before = tournament_visibility::get_for_admin(&pool, ADMIN, TRIP)
        .await
        .unwrap();
    tournament_visibility::update_authorized(
        &pool,
        session,
        TRIP,
        false,
        before.visibility_updated_at,
    )
    .await
    .unwrap();
    let revealed = response(&app, public_request(grant.id, &token, "gross", None))
        .await
        .1;
    assert!(
        revealed["entries"]
            .as_array()
            .unwrap()
            .iter()
            .all(|e| e["tie_break_score_to_par"].is_number())
    );
    assert_eq!(revealed["entries"][0]["tie_break_score_to_par"], 36);
    for metric in ["gross", "net"] {
        let public = response(&app, public_request(grant.id, &token, metric, None))
            .await
            .1;
        allowlist(&public);
        let private = app
            .clone()
            .oneshot(request(
                "GET",
                &format!("/api/tournaments/{TRIP}/leaderboards/{metric}"),
                Some(USER_TOKEN),
                None,
                false,
            ))
            .await
            .unwrap();
        use http_body_util::BodyExt;
        let private: Value =
            serde_json::from_slice(&private.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        for (p, member) in public["entries"]
            .as_array()
            .unwrap()
            .iter()
            .zip(private["entries"].as_array().unwrap())
        {
            assert_eq!(p["position"], member["position"]);
            assert_eq!(p["score_to_par"], member["score_to_par"]);
            assert_eq!(
                p["tie_break_score_to_par"],
                member["tie_break_score_to_par"]
            );
            assert_eq!(
                p["total"],
                member[if metric == "gross" {
                    "gross_total"
                } else {
                    "net_total"
                }]
            );
        }
    }
    let current = tournament_visibility::get_for_admin(&pool, ADMIN, TRIP)
        .await
        .unwrap();
    tournament_visibility::update_authorized(
        &pool,
        session,
        TRIP,
        true,
        current.visibility_updated_at,
    )
    .await
    .unwrap();
    assert_eq!(
        hidden,
        response(
            &app,
            public_request(grant.id, &token, "gross", Some(ADMIN_TOKEN))
        )
        .await
        .1
    );
    // Possessing a capability confers no private workspace access.
    assert_eq!(
        app.oneshot(request(
            "GET",
            &format!("/api/tournaments/{TRIP}/leaderboards/gross"),
            None,
            None,
            false
        ))
        .await
        .unwrap()
        .status(),
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn selected_open_final_hides_changed_back_nine_in_each_metric(pool: PgPool) {
    let session = fixture(&pool).await;
    let version = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap();
    tournaments::start_authorized(&pool, session, TRIP, version)
        .await
        .unwrap();
    let (grant, token) = issue(&pool, session, None).await;
    round_lifecycle::open_authorized(&pool, session, FINAL)
        .await
        .unwrap();
    save_round(&pool, FINAL, 1, 0).await;
    let app = api::router(AppState::new(pool.clone()));
    for (metric, hidden_extra) in [("gross", 2), ("net", 3)] {
        let (status, before) = response(&app, public_request(grant.id, &token, metric, None)).await;
        assert_eq!(status, StatusCode::OK);
        allowlist(&before);
        let entries = before["entries"].as_array().unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().all(|entry| entry["provisional"] == true
            && entry["provisional_holes_scored"] == 9
            && !entry["position"].is_null()
            && entry["tie_break_score_to_par"].is_null()));
        save_round(&pool, FINAL, 1, hidden_extra).await;
        for cookie in [None, Some(ADMIN_TOKEN)] {
            let (status, after) =
                response(&app, public_request(grant.id, &token, metric, cookie)).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(before, after);
        }
    }
}
