use super::*;
#[sqlx::test(migrations = "../migrations")]
async fn draft_pairing_readiness_same_flight_settings_and_snapshot_freeze(pool: PgPool) {
    let session = fixture::draft(&pool, "singles_match_play").await;
    let validation = round_lifecycle::pairing_validation(&pool, id(5))
        .await
        .unwrap()
        .unwrap();
    assert!(!validation.ready);
    assert!(
        validation.issues.iter().any(
            |i| i.code == golf_api::domain::models::ReadinessIssueCode::InvalidMatchAssignments
        )
    );
    let updated = sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id=$1")
        .bind(id(5))
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        match_play::setup::replace(
            &pool,
            session,
            id(5),
            Assignments {
                expected_round_updated_at: updated,
                matches: vec![Pair {
                    first_player_id: id(11),
                    second_player_id: id(13)
                }]
            }
        )
        .await
        .is_err()
    );
    let r = match_play::setup::settings(
        &pool,
        session,
        id(5),
        match_play::setup::Settings {
            expected_round_updated_at: updated,
            handicap_enabled: true,
        },
    )
    .await
    .unwrap();
    assert_eq!(r.updated_at, updated);
    match_play::setup::replace(
        &pool,
        session,
        id(5),
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
    sqlx::query("UPDATE flights SET starting_hole=10 WHERE id=$1")
        .bind(id(31))
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        !round_lifecycle::pairing_validation(&pool, id(5))
            .await
            .unwrap()
            .unwrap()
            .ready
    );
    sqlx::query("UPDATE flights SET starting_hole=1 WHERE id=$1")
        .bind(id(31))
        .execute(&pool)
        .await
        .unwrap();
    let opened = round_lifecycle::open(&pool, id(5)).await.unwrap();
    assert_eq!(
        opened
            .handicap_snapshots
            .iter()
            .find(|s| s.player_id == id(12))
            .unwrap()
            .playing_handicap,
        40
    );
    assert!(
        sqlx::query("UPDATE rounds SET handicap_enabled=FALSE WHERE id=$1")
            .bind(id(5))
            .execute(&pool)
            .await
            .is_err()
    );
}
