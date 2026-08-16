#![cfg(feature = "database-tests")]

use sqlx::PgPool;

const MIGRATION_1: &str = include_str!("../../migrations/0001_initial_schema.sql");
const MIGRATION_2: &str = include_str!("../../migrations/0002_round_opening.sql");
const MIGRATION_3: &str = include_str!("../../migrations/0003_scorecards.sql");
const MIGRATION_4: &str = include_str!("../../migrations/0004_round_completion.sql");
const MIGRATION_5: &str = include_str!("../../migrations/0005_auth_sessions.sql");
const MIGRATION_6: &str = include_str!("../../migrations/0006_tournament_memberships.sql");
const MIGRATION_7: &str = include_str!("../../migrations/0007_tournament_invitations.sql");

#[sqlx::test(migrations = false)]
async fn upgrade_preserves_rows_and_adds_constrained_invitations(pool: PgPool) {
    for migration in [
        MIGRATION_1,
        MIGRATION_2,
        MIGRATION_3,
        MIGRATION_4,
        MIGRATION_5,
        MIGRATION_6,
    ] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index)
        VALUES ('91000000-0000-0000-0000-000000000001', 'Creator', 12.0);
        INSERT INTO users (id, email, display_name, role, player_id)
        VALUES ('91000000-0000-0000-0000-000000000002', 'creator@upgrade.test', 'Creator', 'player',
                '91000000-0000-0000-0000-000000000001');
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
        VALUES
        ('91000000-0000-0000-0000-000000000003', 'Preserved', '2026-09-01', '2026-09-01', 1),
        ('91000000-0000-0000-0000-000000000005', 'Other', '2026-09-02', '2026-09-02', 1);
        INSERT INTO tournament_memberships (tournament_id, user_id, role)
        VALUES ('91000000-0000-0000-0000-000000000003',
                '91000000-0000-0000-0000-000000000002', 'admin');
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(MIGRATION_7).execute(&pool).await.unwrap();
    let preserved = sqlx::query_scalar::<_, String>(
        "SELECT name FROM tournaments WHERE id = '91000000-0000-0000-0000-000000000003'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(preserved, "Preserved");

    sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at)
         VALUES ('91000000-0000-0000-0000-000000000004',
                 '91000000-0000-0000-0000-000000000003', decode(repeat('ab', 32), 'hex'),
                 '91000000-0000-0000-0000-000000000002', now() + interval '1 day')",
    )
    .execute(&pool)
    .await
    .unwrap();

    for statement in [
        "INSERT INTO tournament_invitations (id, tournament_id, token_hash, created_by_user_id, expires_at) VALUES (gen_random_uuid(), '91000000-0000-0000-0000-000000000003', decode(repeat('ab', 32), 'hex'), '91000000-0000-0000-0000-000000000002', now() + interval '1 day')",
        "INSERT INTO tournament_invitations (id, tournament_id, token_hash, created_by_user_id, expires_at) VALUES (gen_random_uuid(), '91000000-0000-0000-0000-000000000003', decode('ab', 'hex'), '91000000-0000-0000-0000-000000000002', now() + interval '1 day')",
        "INSERT INTO tournament_invitations (id, tournament_id, token_hash, created_by_user_id, expires_at) VALUES (gen_random_uuid(), '91000000-0000-0000-0000-000000000003', decode(repeat('cd', 32), 'hex'), '91000000-0000-0000-0000-000000000002', now() - interval '1 day')",
        "INSERT INTO tournament_invitations (id, tournament_id, token_hash, created_by_user_id, expires_at, max_uses) VALUES (gen_random_uuid(), '91000000-0000-0000-0000-000000000003', decode(repeat('ef', 32), 'hex'), '91000000-0000-0000-0000-000000000002', now() + interval '1 day', 0)",
    ] {
        let error = sqlx::query(statement).execute(&pool).await.unwrap_err();
        let database = error.as_database_error().unwrap();
        assert!(database.is_check_violation() || database.is_unique_violation());
    }

    for statement in [
        "INSERT INTO tournament_invitations (id, tournament_id, token_hash, created_by_user_id, expires_at) VALUES (gen_random_uuid(), '91000000-0000-0000-0000-000000000005', decode(repeat('11', 32), 'hex'), '91000000-0000-0000-0000-000000000002', now() + interval '1 day')",
        "INSERT INTO tournament_invitations (id, tournament_id, token_hash, created_by_user_id, expires_at) VALUES (gen_random_uuid(), '91000000-0000-0000-0000-000000000003', decode(repeat('22', 32), 'hex'), '91000000-0000-0000-0000-000000000099', now() + interval '1 day')",
        "INSERT INTO tournament_invitations (id, tournament_id, token_hash, created_by_user_id, expires_at) VALUES (gen_random_uuid(), '91000000-0000-0000-0000-000000000099', decode(repeat('33', 32), 'hex'), '91000000-0000-0000-0000-000000000002', now() + interval '1 day')",
    ] {
        let error = sqlx::query(statement).execute(&pool).await.unwrap_err();
        assert!(
            error
                .as_database_error()
                .unwrap()
                .is_foreign_key_violation()
        );
    }

    let revoked_error = sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, created_at, expires_at, revoked_at)
         VALUES (gen_random_uuid(), '91000000-0000-0000-0000-000000000003',
                 decode(repeat('44', 32), 'hex'),
                 '91000000-0000-0000-0000-000000000002',
                 now(), now() + interval '1 day', now() - interval '1 day')",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(
        revoked_error
            .as_database_error()
            .unwrap()
            .is_check_violation()
    );

    let restricted =
        sqlx::query("DELETE FROM users WHERE id = '91000000-0000-0000-0000-000000000002'")
            .execute(&pool)
            .await
            .unwrap_err();
    assert!(
        restricted
            .as_database_error()
            .unwrap()
            .is_foreign_key_violation()
    );

    sqlx::query("DELETE FROM tournaments WHERE id = '91000000-0000-0000-0000-000000000003'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_invitations")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tournament_memberships
             WHERE tournament_id = '91000000-0000-0000-0000-000000000003'",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}
