#![cfg(feature = "database-tests")]

use sqlx::{PgPool, postgres::PgDatabaseError};
use std::time::Duration;

const MIGRATIONS_1_TO_10: [&str; 10] = [
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
];
const MIGRATION_11: &str = include_str!("../../migrations/0011_round_flights.sql");
const MIGRATION_12: &str = include_str!("../../migrations/0012_remove_flight_scorekeepers.sql");

const TOURNAMENT_1: &str = "a1000000-0000-0000-0000-000000000001";
const TOURNAMENT_2: &str = "a1000000-0000-0000-0000-000000000002";
const ROUND_1: &str = "a1000000-0000-0000-0000-000000000011";
const ROUND_2: &str = "a1000000-0000-0000-0000-000000000012";
const PLAYER_1: &str = "a1000000-0000-0000-0000-000000000021";
const PLAYER_2: &str = "a1000000-0000-0000-0000-000000000022";
const PLAYER_3: &str = "a1000000-0000-0000-0000-000000000023";
const PLAYER_4: &str = "a1000000-0000-0000-0000-000000000024";
const PLAYER_5: &str = "a1000000-0000-0000-0000-000000000025";
const PLAYER_6: &str = "a1000000-0000-0000-0000-000000000026";
const FLIGHT_1: &str = "a1000000-0000-0000-0000-000000000031";
const FLIGHT_2: &str = "a1000000-0000-0000-0000-000000000032";

fn constraint(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .and_then(|error| error.try_downcast_ref::<PgDatabaseError>())
        .and_then(PgDatabaseError::constraint)
}

async fn insert_clean_fixture(pool: &PgPool) {
    sqlx::raw_sql(&format!(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index) VALUES
          ('{PLAYER_1}', 'Linked one', 10.0),
          ('{PLAYER_2}', 'Unlinked', 11.0),
          ('{PLAYER_3}', 'Linked three', 12.0),
          ('{PLAYER_4}', 'Unassigned', 13.0),
          ('{PLAYER_5}', 'Other tournament', 14.0),
          ('{PLAYER_6}', 'No tournament', 15.0);
        INSERT INTO users (id, username, display_name, role, player_id) VALUES
          ('a1000000-0000-0000-0000-000000000041', 'linked_one', 'Linked one', 'player', '{PLAYER_1}'),
          ('a1000000-0000-0000-0000-000000000043', 'linked_three', 'Linked three', 'player', '{PLAYER_3}');
        INSERT INTO users (id, username, display_name, role)
        VALUES ('a1000000-0000-0000-0000-000000000049', 'flight_admin', 'Flight admin', 'player');
        INSERT INTO tournaments
          (id, name, start_date, end_date, number_of_rounds) VALUES
          ('{TOURNAMENT_1}', 'First trip', '2026-01-01', '2026-01-02', 2),
          ('{TOURNAMENT_2}', 'Other trip', '2026-02-01', '2026-02-01', 1);
        INSERT INTO tournament_memberships (tournament_id, user_id, role)
        VALUES ('{TOURNAMENT_1}', 'a1000000-0000-0000-0000-000000000049', 'admin');
        INSERT INTO tournament_players
          (tournament_id, player_id, tournament_handicap) VALUES
          ('{TOURNAMENT_1}', '{PLAYER_1}', 10.0),
          ('{TOURNAMENT_1}', '{PLAYER_2}', 11.0),
          ('{TOURNAMENT_1}', '{PLAYER_3}', 12.0),
          ('{TOURNAMENT_1}', '{PLAYER_4}', 13.0),
          ('{TOURNAMENT_2}', '{PLAYER_5}', 14.0);
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_name,
           tee_name, scoring_format) VALUES
          ('{ROUND_1}', '{TOURNAMENT_1}', 1, 'First round', '2026-01-01', '', '',
           'individual_stroke_play'),
          ('{ROUND_2}', '{TOURNAMENT_1}', 2, 'Second round', '2026-01-02', '', '',
           'individual_stroke_play');
        INSERT INTO flights
          (id, round_id, tournament_id, name, starting_hole, tee_time) VALUES
          ('{FLIGHT_1}', '{ROUND_1}', '{TOURNAMENT_1}', 'Flight 1', 1, '08:30'),
          ('{FLIGHT_2}', '{ROUND_1}', '{TOURNAMENT_1}', 'Flight 2', NULL, NULL);
        INSERT INTO flight_memberships
          (flight_id, round_id, tournament_id, player_id, display_order) VALUES
          ('{FLIGHT_1}', '{ROUND_1}', '{TOURNAMENT_1}', '{PLAYER_1}', 1),
          ('{FLIGHT_1}', '{ROUND_1}', '{TOURNAMENT_1}', '{PLAYER_2}', 2),
          ('{FLIGHT_2}', '{ROUND_1}', '{TOURNAMENT_1}', '{PLAYER_3}', 1);
        "#
    ))
    .execute(pool)
    .await
    .unwrap();
    let guarded_start = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
           SELECT 1 FROM pg_trigger WHERE tgname = 'tournaments_validate_status_transition'
         )",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let mut lifecycle = pool.begin().await.unwrap();
    if guarded_start {
        sqlx::query(
            "SELECT
               set_config('app.tournament_start_tournament_id', $1, true),
               set_config('app.tournament_start_user_id', $2, true)",
        )
        .bind(TOURNAMENT_1)
        .bind("a1000000-0000-0000-0000-000000000049")
        .execute(&mut *lifecycle)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE tournaments SET status = 'active' WHERE id = $1::uuid")
        .bind(TOURNAMENT_1)
        .execute(&mut *lifecycle)
        .await
        .unwrap();
    lifecycle.commit().await.unwrap();
}

async fn stage_first_round_open(transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>) {
    sqlx::query("SELECT set_config('app.round_opening_id', $1, true)")
        .bind(ROUND_1)
        .execute(&mut **transaction)
        .await
        .unwrap();
    sqlx::query(&format!(
        "INSERT INTO round_handicap_snapshots
           (round_id, tournament_id, player_id, handicap_index, course_handicap,
            playing_handicap)
         SELECT '{ROUND_1}', tournament_id, player_id, tournament_handicap,
                tournament_handicap::smallint, tournament_handicap::smallint
         FROM tournament_players WHERE tournament_id = '{TOURNAMENT_1}'"
    ))
    .execute(&mut **transaction)
    .await
    .unwrap();
    sqlx::query("UPDATE rounds SET status = 'open' WHERE id = $1::uuid")
        .bind(ROUND_1)
        .execute(&mut **transaction)
        .await
        .unwrap();
}

async fn open_first_round(pool: &PgPool) {
    let mut transaction = pool.begin().await.unwrap();
    stage_first_round_open(&mut transaction).await;
    transaction.commit().await.unwrap();
}

#[sqlx::test(migrations = false)]
async fn v11_upgrade_discards_only_designations_and_preserves_flights_exactly(pool: PgPool) {
    for migration in MIGRATIONS_1_TO_10 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(MIGRATION_11).execute(&pool).await.unwrap();
    insert_clean_fixture(&pool).await;
    sqlx::query(&format!(
        "INSERT INTO flight_scorekeepers
           (flight_id, round_id, tournament_id, player_id, created_at)
         VALUES ('{FLIGHT_1}', '{ROUND_1}', '{TOURNAMENT_1}', '{PLAYER_1}',
                 '2026-01-01T09:00:00.123456Z')"
    ))
    .execute(&pool)
    .await
    .unwrap();

    let before = sqlx::query_scalar::<_, String>(
        "SELECT jsonb_build_object(
           'flights', (SELECT jsonb_agg(to_jsonb(f) ORDER BY f.id) FROM flights f),
           'memberships', (
             SELECT jsonb_agg(to_jsonb(fm) ORDER BY fm.flight_id, fm.player_id)
             FROM flight_memberships fm
           )
         )::text",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flight_scorekeepers")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    sqlx::raw_sql(MIGRATION_12).execute(&pool).await.unwrap();

    let after = sqlx::query_scalar::<_, String>(
        "SELECT jsonb_build_object(
           'flights', (SELECT jsonb_agg(to_jsonb(f) ORDER BY f.id) FROM flights f),
           'memberships', (
             SELECT jsonb_agg(to_jsonb(fm) ORDER BY fm.flight_id, fm.player_id)
             FROM flight_memberships fm
           )
         )::text",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, before);
    assert_eq!(
        sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass('flight_scorekeepers')::text")
            .fetch_one(&pool)
            .await
            .unwrap(),
        None
    );
}

#[sqlx::test(migrations = false)]
async fn v10_upgrade_preserves_legacy_teams_without_inferring_flights(pool: PgPool) {
    for migration in MIGRATIONS_1_TO_10 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index)
        VALUES ('a2000000-0000-0000-0000-000000000001', 'Legacy player', 9.5);
        INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
        VALUES ('a2000000-0000-0000-0000-000000000002', 'Legacy trip',
                '2026-03-01', '2026-03-01', 1);
        INSERT INTO tournament_players
          (tournament_id, player_id, tournament_handicap)
        VALUES ('a2000000-0000-0000-0000-000000000002',
                'a2000000-0000-0000-0000-000000000001', 9.5);
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_name, tee_name,
           scoring_format)
        VALUES ('a2000000-0000-0000-0000-000000000003',
                'a2000000-0000-0000-0000-000000000002', 1, 'Legacy round',
                '2026-03-01', 'Legacy course', 'Legacy tee', 'team_scramble');
        INSERT INTO teams
          (id, round_id, tournament_id, name, starting_hole, tee_time, created_at,
           updated_at)
        VALUES ('a2000000-0000-0000-0000-000000000004',
                'a2000000-0000-0000-0000-000000000003',
                'a2000000-0000-0000-0000-000000000002', 'Legacy team', 7,
                '09:17:23', '2026-03-01T07:00:00.123456Z',
                '2026-03-01T07:01:02.654321Z');
        INSERT INTO team_memberships
          (team_id, round_id, tournament_id, player_id, display_order, created_at)
        VALUES ('a2000000-0000-0000-0000-000000000004',
                'a2000000-0000-0000-0000-000000000003',
                'a2000000-0000-0000-0000-000000000002',
                'a2000000-0000-0000-0000-000000000001', 4,
                '2026-03-01T07:02:03.456789Z');
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    let before = sqlx::query_scalar::<_, String>(
        "SELECT jsonb_build_object(
           'team', (SELECT to_jsonb(t) FROM teams t),
           'membership', (SELECT to_jsonb(tm) FROM team_memberships tm)
         )::text",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(MIGRATION_11).execute(&pool).await.unwrap();
    sqlx::raw_sql(MIGRATION_12).execute(&pool).await.unwrap();

    let after = sqlx::query_scalar::<_, String>(
        "SELECT jsonb_build_object(
           'team', (SELECT to_jsonb(t) FROM teams t),
           'membership', (SELECT to_jsonb(tm) FROM team_memberships tm)
         )::text",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, before);
    assert_eq!(
        sqlx::query_as::<_, (i64, i64)>(
            "SELECT (SELECT count(*) FROM flights),
                    (SELECT count(*) FROM flight_memberships)",
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        (0, 0)
    );
    assert_eq!(
        sqlx::query_scalar::<_, Option<String>>("SELECT to_regclass('flight_scorekeepers')::text")
            .fetch_one(&pool)
            .await
            .unwrap(),
        None
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn clean_schema_has_only_integrity_constrained_flights_and_memberships(pool: PgPool) {
    insert_clean_fixture(&pool).await;
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT table_name FROM information_schema.tables
             WHERE table_schema = 'public' AND table_name LIKE 'flight%'
             ORDER BY table_name",
        )
        .fetch_all(&pool)
        .await
        .unwrap(),
        vec!["flight_memberships".to_owned(), "flights".to_owned()]
    );

    for (statement, expected_constraint) in [
        (
            format!(
                "INSERT INTO flights (id, round_id, tournament_id, name)
                 VALUES (gen_random_uuid(), '{ROUND_1}', '{TOURNAMENT_2}', 'Wrong trip')"
            ),
            "flights_round_tournament_fkey",
        ),
        (
            format!(
                "INSERT INTO flight_memberships
                   (flight_id, round_id, tournament_id, player_id)
                 VALUES ('{FLIGHT_1}', '{ROUND_2}', '{TOURNAMENT_1}', '{PLAYER_4}')"
            ),
            "flight_memberships_flight_identity_fkey",
        ),
        (
            format!(
                "INSERT INTO flight_memberships
                   (flight_id, round_id, tournament_id, player_id)
                 VALUES ('{FLIGHT_2}', '{ROUND_1}', '{TOURNAMENT_1}', '{PLAYER_1}')"
            ),
            "flight_memberships_round_player_unique",
        ),
        (
            format!(
                "INSERT INTO flight_memberships
                   (flight_id, round_id, tournament_id, player_id)
                 VALUES ('{FLIGHT_2}', '{ROUND_1}', '{TOURNAMENT_1}', '{PLAYER_5}')"
            ),
            "flight_memberships_tournament_entrant_fkey",
        ),
        (
            format!(
                "INSERT INTO flight_memberships
                   (flight_id, round_id, tournament_id, player_id)
                 VALUES ('{FLIGHT_2}', '{ROUND_1}', '{TOURNAMENT_1}', '{PLAYER_6}')"
            ),
            "flight_memberships_tournament_entrant_fkey",
        ),
        (
            format!(
                "INSERT INTO flights (id, round_id, tournament_id, name)
                 VALUES (gen_random_uuid(), '{ROUND_1}', '{TOURNAMENT_1}', 'Flight 1')"
            ),
            "flights_round_name_unique",
        ),
    ] {
        let error = sqlx::query(&statement).execute(&pool).await.unwrap_err();
        assert_eq!(constraint(&error), Some(expected_constraint));
    }

    let untrimmed = sqlx::query(&format!(
        "INSERT INTO flights (id, round_id, tournament_id, name)
         VALUES (gen_random_uuid(), '{ROUND_2}', '{TOURNAMENT_1}', ' Untrimmed ')"
    ))
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&untrimmed),
        Some("flights_name_trimmed_nonempty_check")
    );
    let invalid_start = sqlx::query(&format!(
        "INSERT INTO flights (id, round_id, tournament_id, name, starting_hole)
         VALUES (gen_random_uuid(), '{ROUND_2}', '{TOURNAMENT_1}', 'Bad start', 37)"
    ))
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&invalid_start),
        Some("flights_starting_hole_check")
    );

    sqlx::query(&format!(
        "UPDATE flights SET updated_at = '2000-01-01', name = 'Flight one renamed'
         WHERE id = '{FLIGHT_1}'"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let timestamp_advanced = sqlx::query_scalar::<_, bool>(&format!(
        "SELECT updated_at > '2000-01-01' FROM flights WHERE id = '{FLIGHT_1}'"
    ))
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(timestamp_advanced);
}

#[sqlx::test(migrations = "../migrations")]
async fn flight_round_and_tournament_deletes_cascade_memberships(pool: PgPool) {
    insert_clean_fixture(&pool).await;
    sqlx::query(&format!(
        "DELETE FROM flight_memberships
         WHERE flight_id = '{FLIGHT_1}' AND player_id = '{PLAYER_1}'"
    ))
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(&format!(
            "SELECT count(*) FROM flight_memberships
             WHERE flight_id = '{FLIGHT_1}' AND player_id = '{PLAYER_1}'"
        ))
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );

    sqlx::query(&format!("DELETE FROM flights WHERE id = '{FLIGHT_2}'"))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(&format!(
            "SELECT count(*) FROM flight_memberships WHERE flight_id = '{FLIGHT_2}'"
        ))
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );

    sqlx::query(&format!(
        "INSERT INTO flight_memberships
           (flight_id, round_id, tournament_id, player_id)
         VALUES ('{FLIGHT_1}', '{ROUND_1}', '{TOURNAMENT_1}', '{PLAYER_1}')"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(&format!("DELETE FROM rounds WHERE id = '{ROUND_1}'"))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flights")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM flight_memberships")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );

    sqlx::raw_sql(&format!(
        "INSERT INTO flights (id, round_id, tournament_id, name)
         VALUES ('{FLIGHT_1}', '{ROUND_2}', '{TOURNAMENT_1}', 'Tournament cascade');
         INSERT INTO flight_memberships
           (flight_id, round_id, tournament_id, player_id)
         VALUES ('{FLIGHT_1}', '{ROUND_2}', '{TOURNAMENT_1}', '{PLAYER_1}');"
    ))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "DELETE FROM tournaments WHERE id = '{TOURNAMENT_1}'"
    ))
    .execute(&pool)
    .await
    .unwrap();
    for table in ["flights", "flight_memberships"] {
        let remaining = sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 0, "{table} did not cascade with its tournament");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn flight_mutation_first_makes_opening_wait_for_the_committed_pairing(pool: PgPool) {
    insert_clean_fixture(&pool).await;
    let mut mutation = pool.begin().await.unwrap();
    sqlx::query(&format!(
        "UPDATE flights SET starting_hole = 2 WHERE id = '{FLIGHT_1}'"
    ))
    .execute(&mut *mutation)
    .await
    .unwrap();

    let opening_pool = pool.clone();
    let mut opening = tokio::spawn(async move {
        let mut transaction = opening_pool.begin().await.unwrap();
        stage_first_round_open(&mut transaction).await;
        let starting_hole = sqlx::query_scalar::<_, Option<i16>>(&format!(
            "SELECT starting_hole FROM flights WHERE id = '{FLIGHT_1}'"
        ))
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
        transaction.commit().await.unwrap();
        starting_hole
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut opening)
            .await
            .is_err()
    );

    mutation.commit().await.unwrap();
    assert_eq!(opening.await.unwrap(), Some(2));
    assert_eq!(
        sqlx::query_scalar::<_, String>(&format!(
            "SELECT status::text FROM rounds WHERE id = '{ROUND_1}'"
        ))
        .fetch_one(&pool)
        .await
        .unwrap(),
        "open"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn opening_first_makes_waiting_flight_mutation_recheck_frozen_state(pool: PgPool) {
    insert_clean_fixture(&pool).await;
    let mut opening = pool.begin().await.unwrap();
    stage_first_round_open(&mut opening).await;

    let mutation_pool = pool.clone();
    let mut mutation = tokio::spawn(async move {
        sqlx::query(&format!(
            "UPDATE flights SET starting_hole = 2 WHERE id = '{FLIGHT_1}'"
        ))
        .execute(&mutation_pool)
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut mutation)
            .await
            .is_err()
    );

    opening.commit().await.unwrap();
    let error = mutation.await.unwrap().unwrap_err();
    assert_eq!(constraint(&error), Some("round_pairing_frozen"));
    assert_eq!(
        sqlx::query_scalar::<_, Option<i16>>(&format!(
            "SELECT starting_hole FROM flights WHERE id = '{FLIGHT_1}'"
        ))
        .fetch_one(&pool)
        .await
        .unwrap(),
        Some(1)
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn every_direct_flight_mutation_is_rejected_after_round_opens(pool: PgPool) {
    insert_clean_fixture(&pool).await;
    open_first_round(&pool).await;

    for statement in [
        format!(
            "INSERT INTO flights (id, round_id, tournament_id, name)
             VALUES (gen_random_uuid(), '{ROUND_1}', '{TOURNAMENT_1}', 'Late flight')"
        ),
        format!("UPDATE flights SET name = 'Late rename' WHERE id = '{FLIGHT_1}'"),
        format!("DELETE FROM flights WHERE id = '{FLIGHT_1}'"),
        format!(
            "INSERT INTO flight_memberships
               (flight_id, round_id, tournament_id, player_id)
             VALUES ('{FLIGHT_2}', '{ROUND_1}', '{TOURNAMENT_1}', '{PLAYER_4}')"
        ),
        format!(
            "UPDATE flight_memberships SET display_order = 7
             WHERE flight_id = '{FLIGHT_1}' AND player_id = '{PLAYER_2}'"
        ),
        format!(
            "DELETE FROM flight_memberships
             WHERE flight_id = '{FLIGHT_1}' AND player_id = '{PLAYER_2}'"
        ),
    ] {
        let error = sqlx::query(&statement).execute(&pool).await.unwrap_err();
        assert_eq!(constraint(&error), Some("round_pairing_frozen"));
    }

    sqlx::query(&format!("DELETE FROM rounds WHERE id = '{ROUND_1}'"))
        .execute(&pool)
        .await
        .unwrap();
    for table in ["flights", "flight_memberships"] {
        let remaining = sqlx::query_scalar::<_, i64>(&format!("SELECT count(*) FROM {table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 0, "{table} did not cascade with its open round");
    }
}
