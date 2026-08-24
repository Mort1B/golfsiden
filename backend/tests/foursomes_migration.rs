#![cfg(feature = "database-tests")]

use sqlx::PgPool;

const MIGRATIONS_1_TO_12: [&str; 12] = [
    include_str!("../../migrations/0001_initial_schema.sql"),
    include_str!("../../migrations/0002_round_opening.sql"),
    include_str!("../../migrations/0003_scorecards.sql"),
    include_str!("../../migrations/0004_round_completion.sql"),
    include_str!("../../migrations/0005_auth_sessions.sql"),
    include_str!("../../migrations/0006_tournament_memberships.sql"),
    include_str!("../../migrations/0007_tournament_invitations.sql"),
    include_str!("../../migrations/0008_reusable_invitations.sql"),
    include_str!("../../migrations/0009_username_accounts_fixed_handicaps.sql"),
    include_str!("../../migrations/0010_course_revisions.sql"),
    include_str!("../../migrations/0011_round_flights.sql"),
    include_str!("../../migrations/0012_remove_flight_scorekeepers.sql"),
];
const MIGRATION_13: &str = include_str!("../../migrations/0013_two_player_foursomes.sql");

#[sqlx::test(migrations = false)]
async fn v12_upgrade_preserves_existing_formats_and_adds_foursomes_integrity(pool: PgPool) {
    for migration in MIGRATIONS_1_TO_12 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        "INSERT INTO users (id, username, display_name, role)
         VALUES ('13000000-0000-0000-0000-000000000010', 'upgrade_admin', 'Admin', 'admin');
         INSERT INTO players (id, display_name, current_handicap_index) VALUES
         ('13000000-0000-0000-0000-000000000011', 'Player one', 4.0),
         ('13000000-0000-0000-0000-000000000012', 'Player two', 12.0);
         INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds, status)
         VALUES ('13000000-0000-0000-0000-000000000001', 'Upgrade', '2026-01-01', '2026-01-03', 3, 'active');
         INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap) VALUES
         ('13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000011', 4.0),
         ('13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000012', 12.0);
         INSERT INTO courses (id, name) VALUES
         ('13000000-0000-0000-0000-000000000020', 'Upgrade course');
         INSERT INTO tees (id, course_id, name, slope_rating, course_rating) VALUES
         ('13000000-0000-0000-0000-000000000021', '13000000-0000-0000-0000-000000000020', 'Tee', 113, 4.0);
         INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES
         ('13000000-0000-0000-0000-000000000022', '13000000-0000-0000-0000-000000000021', 1, 4, 1);
         INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, handicap_allowance_percent, scoring_format)
         VALUES ('13000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', 1, 'Existing scramble', '2026-01-01', '13000000-0000-0000-0000-000000000020', 'Upgrade course', '13000000-0000-0000-0000-000000000021', 'Tee', 1, 95, 'team_scramble');
         INSERT INTO teams (id, round_id, tournament_id, name) VALUES
         ('13000000-0000-0000-0000-000000000030', '13000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', 'Existing pair');
         INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id) VALUES
         ('13000000-0000-0000-0000-000000000030', '13000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000011'),
         ('13000000-0000-0000-0000-000000000030', '13000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000012');",
    )
    .execute(&pool)
    .await
    .unwrap();

    let mut opening = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.round_opening_id', $1, true)")
        .bind("13000000-0000-0000-0000-000000000002")
        .execute(&mut *opening)
        .await
        .unwrap();
    sqlx::raw_sql(
        "INSERT INTO round_handicap_snapshots
           (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap) VALUES
         ('13000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000011', 4.0, 4, 4),
         ('13000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000012', 12.0, 12, 12);
         UPDATE rounds SET status = 'open' WHERE id = '13000000-0000-0000-0000-000000000002';",
    )
    .execute(&mut *opening)
    .await
    .unwrap();
    opening.commit().await.unwrap();

    let mut scoring = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1, true)")
        .bind("13000000-0000-0000-0000-000000000002")
        .execute(&mut *scoring)
        .await
        .unwrap();
    sqlx::raw_sql(
        "INSERT INTO scores (id, round_id, tournament_id, hole_id, team_id, gross_strokes, submitted_by)
         VALUES ('13000000-0000-0000-0000-000000000040', '13000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000022', '13000000-0000-0000-0000-000000000030', 4, '13000000-0000-0000-0000-000000000010');
         INSERT INTO scorecard_confirmations (id, round_id, tournament_id, team_id, confirmed_by)
         VALUES ('13000000-0000-0000-0000-000000000041', '13000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000030', '13000000-0000-0000-0000-000000000010');",
    )
    .execute(&mut *scoring)
    .await
    .unwrap();
    sqlx::query("SELECT set_config('app.round_completion_id', $1, true)")
        .bind("13000000-0000-0000-0000-000000000002")
        .execute(&mut *scoring)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET status = 'completed' WHERE id = $1::uuid")
        .bind("13000000-0000-0000-0000-000000000002")
        .execute(&mut *scoring)
        .await
        .unwrap();
    scoring.commit().await.unwrap();

    sqlx::raw_sql(MIGRATION_13).execute(&pool).await.unwrap();

    let existing = sqlx::query_as::<_, (String, i16)>(
        "SELECT scoring_format::text, handicap_allowance_percent FROM rounds WHERE id = '13000000-0000-0000-0000-000000000002'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(existing, ("team_scramble".to_owned(), 95));
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT round_scorecards_ready('13000000-0000-0000-0000-000000000002')",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );

    let mut post_upgrade = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1, true)")
        .bind("13000000-0000-0000-0000-000000000002")
        .execute(&mut *post_upgrade)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE scores SET gross_strokes = 5 WHERE id = '13000000-0000-0000-0000-000000000040'",
    )
    .execute(&mut *post_upgrade)
    .await
    .unwrap();
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT round_scorecards_ready('13000000-0000-0000-0000-000000000002')",
        )
        .fetch_one(&mut *post_upgrade)
        .await
        .unwrap()
    );
    sqlx::query(
        "INSERT INTO scorecard_confirmations (id, round_id, tournament_id, team_id, confirmed_by)
         VALUES ('13000000-0000-0000-0000-000000000042', '13000000-0000-0000-0000-000000000002', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000030', '13000000-0000-0000-0000-000000000010')",
    )
    .execute(&mut *post_upgrade)
    .await
    .unwrap();
    sqlx::query("SELECT set_config('app.round_lock_id', $1, true)")
        .bind("13000000-0000-0000-0000-000000000002")
        .execute(&mut *post_upgrade)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET status = 'locked' WHERE id = $1::uuid")
        .bind("13000000-0000-0000-0000-000000000002")
        .execute(&mut *post_upgrade)
        .await
        .unwrap();
    post_upgrade.commit().await.unwrap();
    assert!(
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT to_regclass('round_team_handicap_snapshots')::text",
        )
        .fetch_one(&pool)
        .await
        .unwrap()
        .is_some()
    );
    let invalid = sqlx::query(
        "INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_name, tee_name, handicap_allowance_percent, scoring_format)
         VALUES ('13000000-0000-0000-0000-000000000003', '13000000-0000-0000-0000-000000000001', 2, 'Invalid', '2026-01-02', '', '', 49, 'two_player_foursomes')",
    )
    .execute(&pool)
    .await;
    assert!(invalid.is_err());

    sqlx::raw_sql(
        "INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, handicap_allowance_percent, scoring_format)
         VALUES ('13000000-0000-0000-0000-000000000004', '13000000-0000-0000-0000-000000000001', 2, 'Foursomes without team snapshot', '2026-01-02', '13000000-0000-0000-0000-000000000020', 'Upgrade course', '13000000-0000-0000-0000-000000000021', 'Tee', 1, 50, 'two_player_foursomes');
         INSERT INTO teams (id, round_id, tournament_id, name) VALUES
         ('13000000-0000-0000-0000-000000000031', '13000000-0000-0000-0000-000000000004', '13000000-0000-0000-0000-000000000001', 'Missing snapshot pair');
         INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id) VALUES
         ('13000000-0000-0000-0000-000000000031', '13000000-0000-0000-0000-000000000004', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000011'),
         ('13000000-0000-0000-0000-000000000031', '13000000-0000-0000-0000-000000000004', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000012');",
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut missing_team_snapshot = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.round_opening_id', $1, true)")
        .bind("13000000-0000-0000-0000-000000000004")
        .execute(&mut *missing_team_snapshot)
        .await
        .unwrap();
    sqlx::raw_sql(
        "INSERT INTO round_handicap_snapshots
           (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap) VALUES
         ('13000000-0000-0000-0000-000000000004', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000011', 4.0, 4, 4),
         ('13000000-0000-0000-0000-000000000004', '13000000-0000-0000-0000-000000000001', '13000000-0000-0000-0000-000000000012', 12.0, 12, 12);",
    )
    .execute(&mut *missing_team_snapshot)
    .await
    .unwrap();
    let missing_snapshot_error = sqlx::query(
        "UPDATE rounds SET status = 'open' WHERE id = '13000000-0000-0000-0000-000000000004'",
    )
    .execute(&mut *missing_team_snapshot)
    .await
    .unwrap_err();
    assert_eq!(
        missing_snapshot_error
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("round_opening_team_snapshots_incomplete")
    );
    missing_team_snapshot.rollback().await.unwrap();
}
