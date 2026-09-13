use super::support::*;
use golf_api::{
    domain::{
        four_ball_card::{Input, read_projection},
        leaderboards::LeaderboardMetric,
        scorecards::ExpectedScore,
    },
    repositories::{four_ball, leaderboards, round_completion, round_lifecycle},
};
use sqlx::PgPool;
#[sqlx::test(migrations = "../migrations")]
async fn player_snapshots_independent_winners_and_side_confirmation_completion(pool: PgPool) {
    let session = ready(&pool).await;
    let hcp: Vec<(uuid::Uuid, i16)> = sqlx::query_as(
        "SELECT player_id,playing_handicap FROM round_handicap_snapshots ORDER BY player_id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        hcp,
        vec![(id(11), 0), (id(12), 34), (id(13), -8), (id(14), 9)]
    );
    numeric(&pool, session, 11, 1, 4).await;
    numeric(&pool, session, 12, 1, 5).await;
    let card = four_ball::get(&pool, session, id(5), id(21), true)
        .await
        .unwrap();
    assert_eq!(
        card.holes[0].gross.as_ref().unwrap().player_ids,
        vec![id(11)]
    );
    assert_eq!(card.holes[0].net.as_ref().unwrap().player_ids, vec![id(12)]);
    assert_eq!(
        (card.gross_total, card.net_total, card.holes_scored),
        (Some(4), Some(3), 1)
    );
    assert!(
        four_ball::confirm(&pool, session, id(5), id(21))
            .await
            .is_err()
    );
    for hole in 2..=18 {
        numeric(&pool, session, 11, hole, 4).await;
    }
    for hole in 1..=18 {
        numeric(&pool, session, 13, hole, 5).await;
    }
    four_ball::confirm(&pool, session, id(5), id(21))
        .await
        .unwrap();
    four_ball::confirm(&pool, session, id(5), id(22))
        .await
        .unwrap();
    assert!(
        round_completion::validation(&pool, id(5))
            .await
            .unwrap()
            .unwrap()
            .ready_to_complete
    );
    round_completion::complete(&pool, id(5)).await.unwrap();
    let entry = card.holes[0].players[1].score.as_ref().unwrap();
    let expected = ExpectedScore::Present {
        score_id: entry.id,
        revision: entry.revision,
    };
    let noop = four_ball::save_conditional(
        &pool,
        operation(
            session,
            12,
            1,
            Input::Numeric { gross_strokes: 5 },
            expected,
        ),
    )
    .await
    .unwrap();
    assert!(!noop.changed);
    assert_eq!(
        four_ball::get(&pool, session, id(5), id(21), true)
            .await
            .unwrap()
            .confirmed,
        Some(true)
    );
    four_ball::save_conditional(
        &pool,
        operation(
            session,
            12,
            1,
            Input::Numeric { gross_strokes: 8 },
            expected,
        ),
    )
    .await
    .unwrap();
    assert_eq!(
        four_ball::get(&pool, session, id(5), id(21), true)
            .await
            .unwrap()
            .confirmed,
        Some(false)
    );
    assert!(round_completion::lock(&pool, id(5)).await.is_err());
    four_ball::confirm(&pool, session, id(5), id(21))
        .await
        .unwrap();
    let before = four_ball::get(&pool, session, id(5), id(21), true)
        .await
        .unwrap();
    let entry = before.holes[0].players[1].score.as_ref().unwrap();
    let changed = four_ball::save_conditional(
        &pool,
        operation(
            session,
            12,
            1,
            Input::Numeric { gross_strokes: 9 },
            ExpectedScore::Present {
                score_id: entry.id,
                revision: entry.revision,
            },
        ),
    )
    .await
    .unwrap();
    let after = four_ball::get(&pool, session, id(5), id(21), true)
        .await
        .unwrap();
    assert_eq!(
        (before.gross_total, before.net_total),
        (after.gross_total, after.net_total)
    );
    assert_eq!(after.confirmed, Some(false));
    four_ball::confirm(&pool, session, id(5), id(21))
        .await
        .unwrap();
    four_ball::save_conditional(
        &pool,
        operation(
            session,
            12,
            1,
            Input::NoScore {},
            ExpectedScore::Present {
                score_id: changed.value.applied_score.score_id,
                revision: changed.value.applied_score.revision,
            },
        ),
    )
    .await
    .unwrap();
    let picked_up = four_ball::get(&pool, session, id(5), id(21), true)
        .await
        .unwrap();
    assert_eq!(
        (picked_up.gross_total, picked_up.net_total),
        (before.gross_total, before.net_total)
    );
    assert_eq!(picked_up.confirmed, Some(false));
    four_ball::confirm(&pool, session, id(5), id(21))
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
        assert_eq!(row.contributions.len(), 1);
        assert_eq!(
            row.contributions[0].owner,
            golf_api::domain::leaderboards::LeaderboardOwner::Team { id: id(21) }
        );
        assert_eq!((row.gross_total, row.net_total), (72, 72));
    }
    round_completion::lock(&pool, id(5)).await.unwrap();
    assert!(
        four_ball::save_conditional(
            &pool,
            operation(session, 12, 2, Input::NoScore {}, ExpectedScore::Absent {})
        )
        .await
        .is_err()
    );
    let board = leaderboards::round(&pool, id(5), LeaderboardMetric::Net)
        .await
        .unwrap();
    assert!(
        board
            .entries
            .iter()
            .all(|entry| entry.playing_handicap.is_none())
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn missing_side_numeric_results_and_hidden_inputs_do_not_leak(pool: PgPool) {
    let admin = ready(&pool).await;
    let viewer = session(&pool, 41, None, "viewer", "hidden-viewer").await;
    four_ball::save_conditional(
        &pool,
        operation(admin, 11, 1, Input::NoScore {}, ExpectedScore::Absent {}),
    )
    .await
    .unwrap();
    four_ball::save_conditional(
        &pool,
        operation(admin, 12, 1, Input::NoScore {}, ExpectedScore::Absent {}),
    )
    .await
    .unwrap();
    let empty = four_ball::get(&pool, admin, id(5), id(21), true)
        .await
        .unwrap();
    assert_eq!((empty.holes_scored, empty.gross_total), (0, None));
    numeric(&pool, admin, 11, 2, 4).await;
    let before = serde_json::to_value(read_projection(
        four_ball::get(&pool, viewer, id(5), id(21), false)
            .await
            .unwrap(),
    ))
    .unwrap();
    assert_eq!(before["visibility"]["mode"], "front_nine");
    assert!(before["complete"].is_null());
    assert!(before["confirmed"].is_null());
    numeric(&pool, admin, 12, 18, 3).await;
    let after = serde_json::to_value(read_projection(
        four_ball::get(&pool, viewer, id(5), id(21), false)
            .await
            .unwrap(),
    ))
    .unwrap();
    assert_eq!(before, after);
}
#[sqlx::test(migrations = "../migrations")]
async fn opening_rejects_missing_partner_and_split_flight(pool: PgPool) {
    draft(&pool, "four_ball_stroke_play").await;
    sqlx::query("DELETE FROM team_memberships WHERE round_id=$1 AND player_id=$2")
        .bind(id(5))
        .bind(id(12))
        .execute(&pool)
        .await
        .unwrap();
    assert!(round_lifecycle::open(&pool, id(5)).await.is_err());
    sqlx::query("INSERT INTO team_memberships(team_id,round_id,tournament_id,player_id) VALUES($1,$2,$3,$4)").bind(id(21)).bind(id(5)).bind(id(2)).bind(id(12)).execute(&pool).await.unwrap();
    sqlx::query("UPDATE flight_memberships SET flight_id=$1 WHERE round_id=$2 AND player_id=$3")
        .bind(id(32))
        .bind(id(5))
        .bind(id(12))
        .execute(&pool)
        .await
        .unwrap();
    assert!(round_lifecycle::open(&pool, id(5)).await.is_err());
    assert!(
        sqlx::query("UPDATE rounds SET number_of_holes=9 WHERE id=$1")
            .bind(id(5))
            .execute(&pool)
            .await
            .is_err()
    );
}
