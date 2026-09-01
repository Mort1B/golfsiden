#![cfg(feature = "database-tests")]

use chrono::{DateTime, Utc};
use sqlx::PgPool;

const MIGRATIONS_THROUGH_14: [&str; 14] = [
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
    include_str!("../../migrations/0013_two_player_foursomes.sql"),
    include_str!("../../migrations/0014_tournament_counted_rounds.sql"),
];
const MIGRATION_15: &str = include_str!("../../migrations/0015_tournament_start.sql");

fn constraint(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .and_then(|database| database.constraint())
}

#[sqlx::test(migrations = false)]
async fn v15_promotes_legacy_started_tournaments_and_installs_guards(pool: PgPool) {
    for migration in MIGRATIONS_THROUGH_14 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        "INSERT INTO players (id, display_name, current_handicap_index)
         VALUES ('15100000-0000-0000-0000-000000000001', 'Legacy entrant', 10.0);
         INSERT INTO users (id, username, display_name, role)
         VALUES ('15100000-0000-0000-0000-000000000002', 'legacy_admin',
                 'Legacy admin', 'player');
         INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds, counted_rounds,
            updated_at)
         VALUES
           ('15100000-0000-0000-0000-000000000010', 'Legacy started',
            '2026-01-01', '2026-01-01', 1, 1, '2026-01-01T00:00:00Z'),
           ('15100000-0000-0000-0000-000000000020', 'Still draft',
            '2026-02-01', '2026-02-01', 1, 1, '2026-01-01T00:00:00Z');
         INSERT INTO tournament_memberships (tournament_id, user_id, role) VALUES
           ('15100000-0000-0000-0000-000000000010',
            '15100000-0000-0000-0000-000000000002', 'admin'),
           ('15100000-0000-0000-0000-000000000020',
            '15100000-0000-0000-0000-000000000002', 'admin');
         INSERT INTO tournament_players
           (tournament_id, player_id, tournament_handicap) VALUES
           ('15100000-0000-0000-0000-000000000010',
            '15100000-0000-0000-0000-000000000001', 10.0),
           ('15100000-0000-0000-0000-000000000020',
            '15100000-0000-0000-0000-000000000001', 10.0);
         INSERT INTO rounds
           (id, tournament_id, round_number, name, round_date, course_name,
            tee_name, status, scoring_format) VALUES
           ('15100000-0000-0000-0000-000000000011',
            '15100000-0000-0000-0000-000000000010', 1, 'Already open',
            '2026-01-01', '', '', 'open', 'individual_stroke_play'),
           ('15100000-0000-0000-0000-000000000021',
            '15100000-0000-0000-0000-000000000020', 1, 'Draft round',
            '2026-02-01', '', '', 'draft', 'individual_stroke_play');",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(MIGRATION_15).execute(&pool).await.unwrap();

    let tournaments = sqlx::query_as::<_, (String, DateTime<Utc>)>(
        "SELECT status::text, updated_at FROM tournaments ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(tournaments[0].0, "active");
    assert!(tournaments[0].1 > "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap());
    assert_eq!(tournaments[1].0, "draft");
    assert_eq!(
        tournaments[1].1,
        "2026-01-01T00:00:00Z".parse::<DateTime<Utc>>().unwrap()
    );
    let legacy_round_status: String = sqlx::query_scalar(
        "SELECT status::text FROM rounds
         WHERE id = '15100000-0000-0000-0000-000000000011'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(legacy_round_status, "open");

    let direct_start = sqlx::query(
        "UPDATE tournaments SET status = 'active'
         WHERE id = '15100000-0000-0000-0000-000000000020'",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&direct_start),
        Some("tournament_start_context_required")
    );

    let mut opening = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config(
           'app.round_opening_id',
           '15100000-0000-0000-0000-000000000021', true)",
    )
    .execute(&mut *opening)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO round_handicap_snapshots
           (round_id, tournament_id, player_id, handicap_index, course_handicap,
            playing_handicap)
         VALUES ('15100000-0000-0000-0000-000000000021',
                 '15100000-0000-0000-0000-000000000020',
                 '15100000-0000-0000-0000-000000000001', 10.0, 10, 10)",
    )
    .execute(&mut *opening)
    .await
    .unwrap();
    let draft_parent = sqlx::query(
        "UPDATE rounds SET status = 'open'
         WHERE id = '15100000-0000-0000-0000-000000000021'",
    )
    .execute(&mut *opening)
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&draft_parent),
        Some("round_opening_tournament_inactive")
    );
    opening.rollback().await.unwrap();
}
