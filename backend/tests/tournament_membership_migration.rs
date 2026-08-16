#![cfg(feature = "database-tests")]

use sqlx::PgPool;

const MIGRATION_1: &str = include_str!("../../migrations/0001_initial_schema.sql");
const MIGRATION_2: &str = include_str!("../../migrations/0002_round_opening.sql");
const MIGRATION_3: &str = include_str!("../../migrations/0003_scorecards.sql");
const MIGRATION_4: &str = include_str!("../../migrations/0004_round_completion.sql");
const MIGRATION_5: &str = include_str!("../../migrations/0005_auth_sessions.sql");
const MIGRATION_6: &str = include_str!("../../migrations/0006_tournament_memberships.sql");

#[sqlx::test(migrations = false)]
async fn upgrade_backfills_authority_participation_and_tournament_handicaps(pool: PgPool) {
    for migration in [
        MIGRATION_1,
        MIGRATION_2,
        MIGRATION_3,
        MIGRATION_4,
        MIGRATION_5,
    ] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index) VALUES
        ('81000000-0000-0000-0000-000000000001', 'Linked', 22.0),
        ('81000000-0000-0000-0000-000000000002', 'Unlinked', 30.0);
        INSERT INTO users (id, email, display_name, role, player_id) VALUES
        ('81000000-0000-0000-0000-000000000011', 'admin@upgrade.test', 'Admin', 'admin', '81000000-0000-0000-0000-000000000002'),
        ('81000000-0000-0000-0000-000000000012', 'scorer@upgrade.test', 'Scorer', 'scorer', NULL),
        ('81000000-0000-0000-0000-000000000013', 'player@upgrade.test', 'Linked', 'player', '81000000-0000-0000-0000-000000000001');
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds) VALUES
        ('81000000-0000-0000-0000-000000000021', 'First', '2026-01-01', '2026-01-01', 1),
        ('81000000-0000-0000-0000-000000000022', 'Second', '2026-01-02', '2026-01-02', 1);
        INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap) VALUES
        ('81000000-0000-0000-0000-000000000021', '81000000-0000-0000-0000-000000000001', 7.5),
        ('81000000-0000-0000-0000-000000000021', '81000000-0000-0000-0000-000000000002', 18.4);
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(MIGRATION_6).execute(&pool).await.unwrap();

    let roles = sqlx::query_as::<_, (String, i64)>(
        "SELECT role::text, count(*) FROM tournament_memberships
         GROUP BY role ORDER BY role::text",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        roles,
        vec![
            ("admin".to_owned(), 2),
            ("player".to_owned(), 1),
            ("scorer".to_owned(), 2),
        ]
    );
    let player_tournaments = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT tournament_id FROM tournament_memberships
         WHERE user_id = '81000000-0000-0000-0000-000000000013'",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(player_tournaments.len(), 1);
    let admin_role = sqlx::query_scalar::<_, String>(
        "SELECT role::text FROM tournament_memberships
         WHERE tournament_id = '81000000-0000-0000-0000-000000000021'
           AND user_id = '81000000-0000-0000-0000-000000000011'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(admin_role, "admin");

    let history = sqlx::query_as::<_, (f64, i64)>(
        "SELECT sum(handicap_index)::float8, count(*) FROM tournament_handicap_history",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(history, (25.9, 2));

    let duplicate = sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ('81000000-0000-0000-0000-000000000021',
                 '81000000-0000-0000-0000-000000000011', 'viewer')",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(duplicate.as_database_error().unwrap().is_unique_violation());
}
