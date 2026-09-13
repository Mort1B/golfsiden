use super::support::*;
use golf_api::{
    domain::{scorecards::ExpectedScore, stableford::card::Input},
    repositories::{
        round_completion, round_lifecycle,
        stableford::{
            self,
            settings::{self, SettingsError},
        },
    },
};
use sqlx::PgPool;
#[sqlx::test(migrations = "../migrations")]
async fn draft_settings_are_admin_scoped_versioned_bounded_and_frozen_at_opening(pool: PgPool) {
    let admin = draft(&pool, "individual_stableford").await;
    let timestamp = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
        .bind(id(5))
        .fetch_one(&pool)
        .await
        .unwrap();
    let viewer = session(&pool, 41, None, "viewer", "settings-viewer").await;
    assert!(matches!(
        settings::update(&pool, viewer, id(5), timestamp, true, 50).await,
        Err(SettingsError::Authorization(_))
    ));
    for allowance in [-1, 101] {
        assert!(matches!(
            settings::update(&pool, admin, id(5), timestamp, true, allowance).await,
            Err(SettingsError::InvalidAllowance)
        ));
    }
    let round = settings::update(&pool, admin, id(5), timestamp, true, 50)
        .await
        .unwrap();
    assert_eq!(round.handicap_allowance_percent, 50);
    assert!(matches!(
        settings::update(&pool, admin, id(5), timestamp, true, 75).await,
        Err(SettingsError::Stale)
    ));
    let opened = round_lifecycle::open(&pool, id(5)).await.unwrap();
    assert_eq!(
        opened
            .handicap_snapshots
            .iter()
            .find(|s| s.player_id == id(12))
            .unwrap()
            .playing_handicap,
        20
    );
    assert!(matches!(
        settings::update(&pool, admin, id(5), round.updated_at, false, 0).await,
        Err(SettingsError::NotDraft)
    ));
    assert!(
        sqlx::query("UPDATE rounds SET handicap_allowance_percent=75 WHERE id=$1")
            .bind(id(5))
            .execute(&pool)
            .await
            .is_err()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn pickups_resolve_zero_points_and_corrections_clear_confirmation_even_same_points(
    pool: PgPool,
) {
    let admin = ready(&pool).await;
    assert!(
        stableford::confirm(&pool, admin, id(5), id(11))
            .await
            .is_err()
    );
    for player in [11, 12, 13, 14] {
        for hole in 1..=18 {
            stableford::save_conditional(
                &pool,
                operation(
                    admin,
                    player,
                    hole,
                    Input::NoScore {},
                    ExpectedScore::Absent {},
                ),
            )
            .await
            .unwrap();
        }
        stableford::confirm(&pool, admin, id(5), id(player))
            .await
            .unwrap();
    }
    let card = stableford::get(&pool, admin, id(5), id(11), true)
        .await
        .unwrap();
    let values = card.values.unwrap();
    assert_eq!(
        (
            values.gross_points,
            values.net_points,
            values.gross_equivalent,
            values.actual_gross_total
        ),
        (0, 0, 36, None)
    );
    assert_eq!(
        (card.holes_scored, card.complete, card.confirmed),
        (18, Some(true), Some(true))
    );
    assert!(
        round_completion::validation(&pool, id(5))
            .await
            .unwrap()
            .unwrap()
            .ready_to_complete
    );
    round_completion::complete(&pool, id(5)).await.unwrap();
    let entry = card.holes[0].score.as_ref().unwrap();
    let expected = ExpectedScore::Present {
        score_id: entry.id,
        revision: entry.revision,
    };
    let noop =
        stableford::save_conditional(&pool, operation(admin, 11, 1, Input::NoScore {}, expected))
            .await
            .unwrap();
    assert!(!noop.changed);
    assert_eq!(
        stableford::get(&pool, admin, id(5), id(11), true)
            .await
            .unwrap()
            .confirmed,
        Some(true)
    );
    let first = stableford::save_conditional(
        &pool,
        operation(admin, 11, 1, Input::Numeric { gross_strokes: 9 }, expected),
    )
    .await
    .unwrap();
    assert!(first.changed);
    assert_eq!(
        stableford::get(&pool, admin, id(5), id(11), true)
            .await
            .unwrap()
            .confirmed,
        Some(false)
    );
    stableford::confirm(&pool, admin, id(5), id(11))
        .await
        .unwrap();
    let expected = ExpectedScore::Present {
        score_id: first.value.applied_score.score_id,
        revision: first.value.applied_score.revision,
    };
    let second = stableford::save_conditional(
        &pool,
        operation(admin, 11, 1, Input::Numeric { gross_strokes: 10 }, expected),
    )
    .await
    .unwrap();
    assert!(second.changed);
    assert_eq!(
        stableford::get(&pool, admin, id(5), id(11), true)
            .await
            .unwrap()
            .confirmed,
        Some(false)
    );
    assert!(round_completion::lock(&pool, id(5)).await.is_err());
    stableford::confirm(&pool, admin, id(5), id(11))
        .await
        .unwrap();
    let request = second.value.request_id;
    let mut replay = operation(admin, 11, 1, Input::Numeric { gross_strokes: 10 }, expected);
    replay.request_id = request;
    assert!(
        !stableford::save_conditional(&pool, replay)
            .await
            .unwrap()
            .changed
    );
    assert_eq!(
        stableford::get(&pool, admin, id(5), id(11), true)
            .await
            .unwrap()
            .confirmed,
        Some(true)
    );
    round_completion::lock(&pool, id(5)).await.unwrap();
    let mut replay = operation(admin, 11, 1, Input::Numeric { gross_strokes: 10 }, expected);
    replay.request_id = request;
    assert!(stableford::save_conditional(&pool, replay).await.is_err());
}
#[sqlx::test(migrations = "../migrations")]
async fn disabled_snapshots_and_non_18_hole_configuration_fail_closed(pool: PgPool) {
    let admin = draft(&pool, "individual_stableford").await;
    assert!(
        sqlx::query("UPDATE rounds SET number_of_holes=9 WHERE id=$1")
            .bind(id(5))
            .execute(&pool)
            .await
            .is_err()
    );
    let timestamp = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
        .bind(id(5))
        .fetch_one(&pool)
        .await
        .unwrap();
    settings::update(&pool, admin, id(5), timestamp, false, 100)
        .await
        .unwrap();
    let opened = round_lifecycle::open(&pool, id(5)).await.unwrap();
    assert!(
        opened
            .handicap_snapshots
            .iter()
            .all(|s| s.playing_handicap == 0)
    );
}
