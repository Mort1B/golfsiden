#![cfg(feature = "database-tests")]

use sqlx::PgPool;

const MIGRATIONS_1_TO_8: [&str; 8] = [
    include_str!("../../migrations/0001_initial_schema.sql"),
    include_str!("../../migrations/0002_round_opening.sql"),
    include_str!("../../migrations/0003_scorecards.sql"),
    include_str!("../../migrations/0004_round_completion.sql"),
    include_str!("../../migrations/0005_auth_sessions.sql"),
    include_str!("../../migrations/0006_tournament_memberships.sql"),
    include_str!("../../migrations/0007_tournament_invitations.sql"),
    include_str!("../../migrations/0008_reusable_invitations.sql"),
];
const MIGRATION_9: &str =
    include_str!("../../migrations/0009_username_accounts_fixed_handicaps.sql");

#[sqlx::test(migrations = false)]
async fn v8_upgrade_backfills_collision_safe_usernames_and_preserves_identity(pool: PgPool) {
    for migration in MIGRATIONS_1_TO_8 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index)
        VALUES ('99000000-0000-0000-0000-000000000001', 'Legacy Player', 12.0);
        INSERT INTO users
          (id, email, display_name, role, password_hash, player_id, created_at) VALUES
        ('99000000-0000-0000-0000-000000000011', 'Foo.Bar@example.test', 'First', 'player',
         'hash-one', '99000000-0000-0000-0000-000000000001', '2026-01-01T00:00:00Z'),
        ('99000000-0000-0000-0000-000000000012', 'foo_bar@example.test', 'Second', 'viewer',
         'hash-two', NULL, '2026-01-02T00:00:00Z'),
        ('99000000-0000-0000-0000-000000000013', 'foo_bar_2@example.test', 'Third', 'viewer',
         'hash-three', NULL, '2026-01-03T00:00:00Z'),
        ('99000000-0000-0000-0000-000000000014', '+@example.test', 'Short', 'viewer',
         'hash-four', NULL, '2026-01-04T00:00:00Z');
        INSERT INTO user_sessions (id, user_id, token_hash, expires_at)
        VALUES ('99000000-0000-0000-0000-000000000021',
                '99000000-0000-0000-0000-000000000011',
                decode(repeat('01', 32), 'hex'), now() + interval '1 day');
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
        VALUES ('99000000-0000-0000-0000-000000000030', 'Legacy trip',
                '2026-01-01', '2026-01-01', 1);
        INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
        VALUES ('99000000-0000-0000-0000-000000000030',
                '99000000-0000-0000-0000-000000000001', 12.0);
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_name, tee_name,
           number_of_holes, scoring_format)
        VALUES ('99000000-0000-0000-0000-000000000031',
                '99000000-0000-0000-0000-000000000030', 1, 'Legacy round',
                '2026-01-01', '', '', 1, 'individual_stroke_play');
        SELECT set_config(
            'app.round_opening_id',
            '99000000-0000-0000-0000-000000000031',
            false
        );
        INSERT INTO round_handicap_snapshots
          (round_id, tournament_id, player_id, handicap_index, course_handicap,
           playing_handicap)
        VALUES ('99000000-0000-0000-0000-000000000031',
                '99000000-0000-0000-0000-000000000030',
                '99000000-0000-0000-0000-000000000001', 12.0, 12, 12);
        UPDATE rounds SET status = 'open'
        WHERE id = '99000000-0000-0000-0000-000000000031';
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(MIGRATION_9).execute(&pool).await.unwrap();

    let users = sqlx::query_as::<_, (String, String, String, Option<uuid::Uuid>)>(
        "SELECT username, display_name, password_hash, player_id
         FROM users ORDER BY id",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(users[0].0, "foo_bar");
    assert_eq!(users[1].0, "foo_bar_2");
    assert_eq!(users[2].0, "foo_bar_2_2");
    assert_eq!(users[3].0, "user_");
    assert_eq!(users[0].1, "First");
    assert_eq!(users[0].2, "hash-one");
    assert_eq!(
        users[0].3,
        Some(uuid::uuid!("99000000-0000-0000-0000-000000000001"))
    );
    let session_user = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT user_id FROM user_sessions
         WHERE id = '99000000-0000-0000-0000-000000000021'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        session_user,
        uuid::uuid!("99000000-0000-0000-0000-000000000011")
    );
    let email_column_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
           SELECT 1 FROM information_schema.columns
           WHERE table_schema = 'public' AND table_name = 'users'
             AND column_name = 'email'
         )",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!email_column_exists);
    let lock_reason = sqlx::query_scalar::<_, String>(
        "SELECT reason::text FROM tournament_handicap_locks
         WHERE tournament_id = '99000000-0000-0000-0000-000000000030'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lock_reason, "snapshot_captured");
}

#[sqlx::test(migrations = "../migrations")]
async fn clean_schema_requires_valid_normalized_username_and_has_no_account_email(pool: PgPool) {
    let invalid = sqlx::query(
        "INSERT INTO users (id, username, display_name)
         VALUES (gen_random_uuid(), 'Bad.Name', 'Invalid')",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        invalid
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("users_username_syntax_check")
    );
    let email_column_exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(
           SELECT 1 FROM information_schema.columns
           WHERE table_schema = 'public' AND table_name = 'users'
             AND column_name = 'email'
         )",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(!email_column_exists);
}
