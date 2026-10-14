use super::support::*;
use crate::match_fixture as seed;
use axum::http::StatusCode;
use golf_api::{
    domain::{
        match_play::commands::{Command, Event},
        scorecards::ScoreRevision,
    },
    repositories::{match_play, round_completion},
};
use sqlx::PgPool;
use uuid::Uuid;
#[sqlx::test(migrations = "../migrations")]
async fn draft_format_completed_and_locked_error_order_stays_match_specific(pool: PgPool) {
    let admin = seed::draft(&pool, "singles_match_play").await;
    let app = golf_api::api::router(golf_api::AppState::new(pool.clone()));
    let updated = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
        .bind(ROUND)
        .fetch_one(&pool)
        .await
        .unwrap();
    match_play::setup::replace(
        &pool,
        admin,
        ROUND,
        match_play::setup::Assignments {
            expected_round_updated_at: updated,
            matches: vec![
                match_play::setup::Pair {
                    first_player_id: PLAYER,
                    second_player_id: seed::id(12),
                },
                match_play::setup::Pair {
                    first_player_id: seed::id(13),
                    second_player_id: seed::id(14),
                },
            ],
        },
    )
    .await
    .unwrap();
    for (index, path) in paths(&pool).await.into_iter().enumerate() {
        let r = get(&app, &path, seed::TOKEN).await;
        assert_private(&r);
        assert_eq!(
            r.status(),
            if index == 0 {
                StatusCode::OK
            } else {
                StatusCode::FORBIDDEN
            }
        );
        if index == 0 {
            let value = body(r).await;
            assert_eq!(value["round_status"], "draft");
            assert!(value["opponents"][0]["playing_handicap"].is_null());
        }
    }
    golf_api::repositories::round_lifecycle::open_authorized(&pool, admin, ROUND)
        .await
        .unwrap();
    let matches: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM singles_matches WHERE round_id=$1 ORDER BY id")
            .bind(ROUND)
            .fetch_all(&pool)
            .await
            .unwrap();
    for id in matches {
        let conceder: Uuid =
            sqlx::query_scalar("SELECT second_player_id FROM singles_matches WHERE id=$1")
                .bind(id)
                .fetch_one(&pool)
                .await
                .unwrap();
        for (revision, command) in [
            (
                1,
                Command::Report {
                    event: Event::Concession {
                        conceding_player_id: conceder,
                        communicated: true,
                        after_hole: 0,
                    },
                },
            ),
            (
                2,
                Command::Confirm {
                    result_agreed_or_awarded: true,
                },
            ),
        ] {
            match_play::execute(
                &pool,
                admin,
                ROUND,
                id,
                match_play_request(revision, command),
            )
            .await
            .unwrap();
        }
    }
    seed::session(&pool, 250, Some(11), "scorer", "state-scorer").await;
    round_completion::complete_authorized(&pool, admin, ROUND)
        .await
        .unwrap();
    let scoring = &paths(&pool).await[1];
    assert_eq!(
        get(&app, scoring, "state-scorer").await.status(),
        StatusCode::OK
    );
    round_completion::lock_authorized(&pool, admin, ROUND)
        .await
        .unwrap();
    let r = get(&app, scoring, "state-scorer").await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
    assert_private(&r);
    for path in paths(&pool).await {
        let r = get(&app, &path, seed::TOKEN).await;
        assert_eq!(r.status(), StatusCode::OK);
        assert_private(&r);
        assert_eq!(body(r).await["round_status"], "locked");
    }
}
fn match_play_request(
    revision: i64,
    command: Command,
) -> golf_api::domain::match_play::commands::Request {
    golf_api::domain::match_play::commands::Request {
        request_id: Uuid::new_v4(),
        expected_revision: ScoreRevision::from_database(revision).unwrap(),
        command,
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn wrong_format_stays_not_found_before_membership_resolution(pool: PgPool) {
    seed::draft(&pool, "individual_stroke_play").await;
    let app = golf_api::api::router(golf_api::AppState::new(pool.clone()));
    sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    for suffix in ["", "/scoring"] {
        let path = format!(
            "/api/rounds/{ROUND}/match-play/matches/{}{suffix}",
            Uuid::new_v4()
        );
        let r = get(&app, &path, seed::TOKEN).await;
        assert_eq!(r.status(), StatusCode::NOT_FOUND);
        assert_private(&r);
    }
}
