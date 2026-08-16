#![cfg(feature = "database-tests")]

use sqlx::PgPool;

const MIGRATIONS: [&str; 8] = [
    include_str!("../../migrations/0001_initial_schema.sql"),
    include_str!("../../migrations/0002_round_opening.sql"),
    include_str!("../../migrations/0003_scorecards.sql"),
    include_str!("../../migrations/0004_round_completion.sql"),
    include_str!("../../migrations/0005_auth_sessions.sql"),
    include_str!("../../migrations/0006_tournament_memberships.sql"),
    include_str!("../../migrations/0007_tournament_invitations.sql"),
    include_str!("../../migrations/0008_reusable_invitations.sql"),
];

#[sqlx::test(migrations = false)]
async fn upgrade_adds_series_and_append_only_exact_redemptions(pool: PgPool) {
    for migration in &MIGRATIONS[..7] {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index)
        VALUES ('92000000-0000-0000-0000-000000000001', 'Player', 11.2);
        INSERT INTO users (id, email, display_name, role, player_id)
        VALUES ('92000000-0000-0000-0000-000000000002', 'player@upgrade.test',
                'Player', 'player', '92000000-0000-0000-0000-000000000001');
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
        VALUES ('92000000-0000-0000-0000-000000000003', 'Upgrade',
                '2026-09-01', '2026-09-02', 1);
        INSERT INTO tournament_memberships (tournament_id, user_id, role)
        VALUES ('92000000-0000-0000-0000-000000000003',
                '92000000-0000-0000-0000-000000000002', 'admin');
        INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
        VALUES ('92000000-0000-0000-0000-000000000003',
                '92000000-0000-0000-0000-000000000001', 11.2);
        INSERT INTO tournament_invitations
          (id, tournament_id, token_hash, created_by_user_id, expires_at, max_uses)
        VALUES ('92000000-0000-0000-0000-000000000004',
                '92000000-0000-0000-0000-000000000003',
                decode(repeat('ab', 32), 'hex'),
                '92000000-0000-0000-0000-000000000002', now() + interval '2 days', 2);
        INSERT INTO tournament_invitations
          (id, tournament_id, token_hash, created_by_user_id, created_at,
           expires_at, revoked_at)
        VALUES ('92000000-0000-0000-0000-000000000007',
                '92000000-0000-0000-0000-000000000003',
                decode(repeat('12', 32), 'hex'),
                '92000000-0000-0000-0000-000000000002', now() - interval '1 day',
                now() + interval '2 days', now());
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(MIGRATIONS[7]).execute(&pool).await.unwrap();
    let root = sqlx::query_scalar::<_, bool>(
        "SELECT series_id = id FROM tournament_invitations
         WHERE id = '92000000-0000-0000-0000-000000000004'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(root);
    let legacy = sqlx::query_as::<_, (Option<uuid::Uuid>, bool)>(
        "SELECT revoked_by_user_id, revocation_actor_known
         FROM tournament_invitations
         WHERE id = '92000000-0000-0000-0000-000000000007'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(legacy, (None, false));

    sqlx::raw_sql(
        r#"
        UPDATE tournament_invitations
        SET revoked_at = now(), revoked_by_user_id = '92000000-0000-0000-0000-000000000002'
        WHERE id = '92000000-0000-0000-0000-000000000004';
        INSERT INTO tournament_invitations
          (id, tournament_id, token_hash, created_by_user_id, expires_at, max_uses,
           series_id, predecessor_id)
        SELECT '92000000-0000-0000-0000-000000000005', tournament_id,
               decode(repeat('cd', 32), 'hex'), created_by_user_id, expires_at, max_uses,
               series_id, id
        FROM tournament_invitations
        WHERE id = '92000000-0000-0000-0000-000000000004';
        INSERT INTO invitation_redemptions
          (id, invitation_id, series_id, tournament_id, user_id, player_id, mode)
        VALUES ('92000000-0000-0000-0000-000000000006',
                '92000000-0000-0000-0000-000000000005',
                '92000000-0000-0000-0000-000000000004',
                '92000000-0000-0000-0000-000000000003',
                '92000000-0000-0000-0000-000000000002',
                '92000000-0000-0000-0000-000000000001', 'acceptance');
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    for statement in [
        "UPDATE invitation_redemptions SET redeemed_at = now() WHERE id = '92000000-0000-0000-0000-000000000006'",
        "DELETE FROM invitation_redemptions WHERE id = '92000000-0000-0000-0000-000000000006'",
        "DELETE FROM tournament_memberships WHERE tournament_id = '92000000-0000-0000-0000-000000000003' AND user_id = '92000000-0000-0000-0000-000000000002'",
        "DELETE FROM tournament_players WHERE tournament_id = '92000000-0000-0000-0000-000000000003' AND player_id = '92000000-0000-0000-0000-000000000001'",
    ] {
        assert!(sqlx::query(statement).execute(&pool).await.is_err());
    }

    let duplicate_successor = sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at, max_uses,
            series_id, predecessor_id)
         SELECT gen_random_uuid(), tournament_id, decode(repeat('ef', 32), 'hex'),
                created_by_user_id, expires_at, max_uses, series_id, id
         FROM tournament_invitations
         WHERE id = '92000000-0000-0000-0000-000000000004'",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(
        duplicate_successor
            .as_database_error()
            .unwrap()
            .is_unique_violation()
    );

    let policy_mismatch = sqlx::query(
        "INSERT INTO tournament_invitations
           (id, tournament_id, token_hash, created_by_user_id, expires_at, max_uses,
            series_id, predecessor_id)
         SELECT gen_random_uuid(), tournament_id, decode(repeat('11', 32), 'hex'),
                created_by_user_id, expires_at + interval '1 hour', max_uses, series_id, id
         FROM tournament_invitations
         WHERE id = '92000000-0000-0000-0000-000000000005'",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert!(
        policy_mismatch
            .as_database_error()
            .unwrap()
            .is_check_violation()
    );

    sqlx::query("DELETE FROM tournaments WHERE id = '92000000-0000-0000-0000-000000000003'")
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM invitation_redemptions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
