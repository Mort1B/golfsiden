#![cfg(feature = "database-tests")]

use chrono::{Duration, Utc};
use golf_api::{
    auth::hash_session_token,
    repositories::{auth, round_lifecycle, tournaments},
};
use sqlx::PgPool;
use uuid::{Uuid, uuid};

const TOURNAMENT_ID: Uuid = uuid!("98000000-0000-0000-0000-000000000001");
const ROUND_ID: Uuid = uuid!("98000000-0000-0000-0000-000000000002");
const PLAYER_ID: Uuid = uuid!("98000000-0000-0000-0000-000000000003");
const ADMIN_ID: Uuid = uuid!("98000000-0000-0000-0000-000000000004");

async fn seed(pool: &PgPool) -> Uuid {
    sqlx::raw_sql(
        r#"
        INSERT INTO users (id, username, display_name, role)
        VALUES ('98000000-0000-0000-0000-000000000004', 'correction_admin', 'Admin', 'viewer');
        INSERT INTO players (id, display_name, current_handicap_index)
        VALUES ('98000000-0000-0000-0000-000000000003', 'Player', 18.0);
        INSERT INTO tournaments
          (id, name, start_date, end_date, number_of_rounds, status)
        VALUES ('98000000-0000-0000-0000-000000000001', 'Correction trip',
                '2026-09-01', '2026-09-01', 1, 'draft');
        INSERT INTO tournament_memberships (tournament_id, user_id, role)
        VALUES ('98000000-0000-0000-0000-000000000001',
                '98000000-0000-0000-0000-000000000004', 'admin');
        INSERT INTO tournament_players
          (tournament_id, player_id, tournament_handicap)
        VALUES ('98000000-0000-0000-0000-000000000001',
                '98000000-0000-0000-0000-000000000003', 18.0);
        INSERT INTO tournament_handicap_history
          (id, tournament_id, player_id, handicap_index, changed_by, reason)
        VALUES ('98000000-0000-0000-0000-000000000005',
                '98000000-0000-0000-0000-000000000001',
                '98000000-0000-0000-0000-000000000003', 18.0,
                '98000000-0000-0000-0000-000000000004',
                'initial tournament handicap');
        INSERT INTO courses (id, name)
        VALUES ('98000000-0000-0000-0000-000000000006', 'Course');
        INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
        VALUES ('98000000-0000-0000-0000-000000000007',
                '98000000-0000-0000-0000-000000000006', 'Tee', 137, 4.7);
        INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
        VALUES ('98000000-0000-0000-0000-000000000008',
                '98000000-0000-0000-0000-000000000007', 1, 4, 1);
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_id,
           course_name, tee_id, tee_name, number_of_holes, scoring_format)
        VALUES ('98000000-0000-0000-0000-000000000002',
                '98000000-0000-0000-0000-000000000001', 1, 'Round',
                '2026-09-01', '98000000-0000-0000-0000-000000000006',
                'Course', '98000000-0000-0000-0000-000000000007', 'Tee', 1,
                'individual_stroke_play');
        INSERT INTO teams (id, round_id, tournament_id, name)
        VALUES ('98000000-0000-0000-0000-000000000009',
                '98000000-0000-0000-0000-000000000002',
                '98000000-0000-0000-0000-000000000001', 'Group');
        INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id)
        VALUES ('98000000-0000-0000-0000-000000000009',
                '98000000-0000-0000-0000-000000000002',
                '98000000-0000-0000-0000-000000000001',
                '98000000-0000-0000-0000-000000000003');
        "#,
    )
    .execute(pool)
    .await
    .unwrap();
    auth::create_session(
        pool,
        ADMIN_ID,
        &hash_session_token("correction-session"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id
}

fn constraint(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .and_then(|database| database.constraint())
}

#[sqlx::test(migrations = "../migrations")]
async fn direct_sql_is_guarded_and_correction_audit_is_exactly_once_and_immutable(pool: PgPool) {
    let session_id = seed(&pool).await;
    let direct = sqlx::query(
        "UPDATE tournament_players SET tournament_handicap = 17.0
         WHERE tournament_id = $1 AND player_id = $2",
    )
    .bind(TOURNAMENT_ID)
    .bind(PLAYER_ID)
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&direct),
        Some("tournament_handicap_correction_context_required")
    );

    sqlx::raw_sql(
        r#"
        INSERT INTO users (id, username, display_name)
        VALUES ('98000000-0000-0000-0000-000000000020', 'correction_viewer', 'Viewer');
        INSERT INTO tournament_memberships (tournament_id, user_id, role)
        VALUES ('98000000-0000-0000-0000-000000000001',
                '98000000-0000-0000-0000-000000000020', 'viewer');
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    let mut unauthorized = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT
           set_config('app.tournament_handicap_correction_tournament_id', $1::text, true),
           set_config('app.tournament_handicap_correction_player_id', $2::text, true),
           set_config('app.tournament_handicap_correction_user_id', $3::text, true),
           set_config('app.tournament_handicap_correction_audit_id', $4::text, true),
           set_config('app.tournament_handicap_correction_reason', 'spoofed', true)",
    )
    .bind(TOURNAMENT_ID)
    .bind(PLAYER_ID)
    .bind(uuid!("98000000-0000-0000-0000-000000000020"))
    .bind(Uuid::new_v4())
    .execute(&mut *unauthorized)
    .await
    .unwrap();
    let unauthorized_error = sqlx::query(
        "UPDATE tournament_players SET tournament_handicap = 17.5
         WHERE tournament_id = $1 AND player_id = $2",
    )
    .bind(TOURNAMENT_ID)
    .bind(PLAYER_ID)
    .execute(&mut *unauthorized)
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&unauthorized_error),
        Some("tournament_handicap_admin_required")
    );
    unauthorized.rollback().await.unwrap();

    let correction = tournaments::change_player_handicap_authorized(
        &pool,
        session_id,
        TOURNAMENT_ID,
        PLAYER_ID,
        17.0,
        " verified review ",
    )
    .await
    .unwrap();
    assert_eq!(correction.player.tournament_handicap, 17.0);
    assert_eq!(correction.audit.reason.as_deref(), Some("verified review"));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tournament_handicap_history
             WHERE tournament_id = $1 AND player_id = $2",
        )
        .bind(TOURNAMENT_ID)
        .bind(PLAYER_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        2
    );

    let unchanged = tournaments::change_player_handicap_authorized(
        &pool,
        session_id,
        TOURNAMENT_ID,
        PLAYER_ID,
        17.0,
        "same value",
    )
    .await
    .unwrap_err();
    assert!(matches!(
        unchanged,
        tournaments::TournamentMutationError::HandicapUnchanged
    ));

    for statement in [
        "UPDATE tournament_handicap_history SET reason = 'rewritten' WHERE id = $1",
        "DELETE FROM tournament_handicap_history WHERE id = $1",
    ] {
        let error = sqlx::query(statement)
            .bind(correction.audit.id)
            .execute(&pool)
            .await
            .unwrap_err();
        assert_eq!(
            constraint(&error),
            Some("tournament_handicap_history_immutable")
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn correction_and_open_race_finishes_with_one_authoritative_snapshot(pool: PgPool) {
    let session_id = seed(&pool).await;
    let correction_pool = pool.clone();
    let correction = tokio::spawn(async move {
        tournaments::change_player_handicap_authorized(
            &correction_pool,
            session_id,
            TOURNAMENT_ID,
            PLAYER_ID,
            16.0,
            "race review",
        )
        .await
    });
    let open_pool = pool.clone();
    let opening = tokio::spawn(async move {
        round_lifecycle::open_authorized(&open_pool, session_id, ROUND_ID).await
    });
    let (correction, opening) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(correction, opening)
    })
    .await
    .expect("correction and opening deadlocked");
    let correction = correction.unwrap();
    opening.unwrap().unwrap();

    let registered = sqlx::query_scalar::<_, f64>(
        "SELECT tournament_handicap::float8 FROM tournament_players
         WHERE tournament_id = $1 AND player_id = $2",
    )
    .bind(TOURNAMENT_ID)
    .bind(PLAYER_ID)
    .fetch_one(&pool)
    .await
    .unwrap();
    let snapshotted = sqlx::query_scalar::<_, f64>(
        "SELECT handicap_index::float8 FROM round_handicap_snapshots
         WHERE round_id = $1 AND player_id = $2",
    )
    .bind(ROUND_ID)
    .bind(PLAYER_ID)
    .fetch_one(&pool)
    .await
    .unwrap();
    match correction {
        Ok(_) => {
            assert_eq!(registered, 16.0);
            assert_eq!(snapshotted, 16.0);
        }
        Err(tournaments::TournamentMutationError::HandicapLocked) => {
            assert_eq!(registered, 18.0);
            assert_eq!(snapshotted, 18.0);
        }
        Err(other) => panic!("unexpected correction result: {other:?}"),
    }

    sqlx::query("DELETE FROM rounds WHERE id = $1")
        .bind(ROUND_ID)
        .execute(&pool)
        .await
        .unwrap();
    let after_round_deletion = tournaments::change_player_handicap_authorized(
        &pool,
        session_id,
        TOURNAMENT_ID,
        PLAYER_ID,
        15.0,
        "must remain locked",
    )
    .await
    .unwrap_err();
    assert!(matches!(
        after_round_deletion,
        tournaments::TournamentMutationError::HandicapLocked
    ));
    let lock_delete = sqlx::query("DELETE FROM tournament_handicap_locks WHERE tournament_id = $1")
        .bind(TOURNAMENT_ID)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        constraint(&lock_delete),
        Some("tournament_handicap_lock_immutable")
    );
    sqlx::query("DELETE FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT_ID)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tournament_handicap_locks
             WHERE tournament_id = $1",
        )
        .bind(TOURNAMENT_ID)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn scramble_snapshots_store_the_capped_effective_index_before_tee_conversion(pool: PgPool) {
    sqlx::raw_sql(
        r#"
        INSERT INTO players (id, display_name, current_handicap_index) VALUES
        ('98100000-0000-0000-0000-000000000001', 'A', 35.9),
        ('98100000-0000-0000-0000-000000000002', 'B', 36.0),
        ('98100000-0000-0000-0000-000000000003', 'C', 36.1),
        ('98100000-0000-0000-0000-000000000004', 'D', 54.0);
        INSERT INTO tournaments
          (id, name, start_date, end_date, number_of_rounds, status)
        VALUES ('98100000-0000-0000-0000-000000000010', 'Cap trip',
                '2026-09-01', '2026-09-01', 1, 'draft');
        INSERT INTO tournament_players
          (tournament_id, player_id, tournament_handicap) VALUES
        ('98100000-0000-0000-0000-000000000010', '98100000-0000-0000-0000-000000000001', 35.9),
        ('98100000-0000-0000-0000-000000000010', '98100000-0000-0000-0000-000000000002', 36.0),
        ('98100000-0000-0000-0000-000000000010', '98100000-0000-0000-0000-000000000003', 36.1),
        ('98100000-0000-0000-0000-000000000010', '98100000-0000-0000-0000-000000000004', 54.0);
        INSERT INTO courses (id, name)
        VALUES ('98100000-0000-0000-0000-000000000011', 'Cap course');
        INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
        VALUES ('98100000-0000-0000-0000-000000000012',
                '98100000-0000-0000-0000-000000000011', 'Non-neutral', 155, 72.2);
        INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
        SELECT gen_random_uuid(), '98100000-0000-0000-0000-000000000012', hole, 4, hole
        FROM generate_series(1, 18) AS hole;
        INSERT INTO rounds
          (id, tournament_id, round_number, name, round_date, course_id,
           course_name, tee_id, tee_name, number_of_holes, scoring_format)
        VALUES ('98100000-0000-0000-0000-000000000013',
                '98100000-0000-0000-0000-000000000010', 1, 'Scramble',
                '2026-09-01', '98100000-0000-0000-0000-000000000011',
                'Cap course', '98100000-0000-0000-0000-000000000012',
                'Non-neutral', 18, 'team_scramble');
        INSERT INTO teams (id, round_id, tournament_id, name) VALUES
        ('98100000-0000-0000-0000-000000000014',
         '98100000-0000-0000-0000-000000000013',
         '98100000-0000-0000-0000-000000000010', 'Team A'),
        ('98100000-0000-0000-0000-000000000015',
         '98100000-0000-0000-0000-000000000013',
         '98100000-0000-0000-0000-000000000010', 'Team B');
        INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id) VALUES
        ('98100000-0000-0000-0000-000000000014', '98100000-0000-0000-0000-000000000013',
         '98100000-0000-0000-0000-000000000010', '98100000-0000-0000-0000-000000000001'),
        ('98100000-0000-0000-0000-000000000014', '98100000-0000-0000-0000-000000000013',
         '98100000-0000-0000-0000-000000000010', '98100000-0000-0000-0000-000000000002'),
        ('98100000-0000-0000-0000-000000000015', '98100000-0000-0000-0000-000000000013',
         '98100000-0000-0000-0000-000000000010', '98100000-0000-0000-0000-000000000003'),
        ('98100000-0000-0000-0000-000000000015', '98100000-0000-0000-0000-000000000013',
         '98100000-0000-0000-0000-000000000010', '98100000-0000-0000-0000-000000000004');
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    round_lifecycle::open(&pool, uuid!("98100000-0000-0000-0000-000000000013"))
        .await
        .unwrap();
    let rows = sqlx::query_as::<_, (f64, f64, i16)>(
        "SELECT tp.tournament_handicap::float8, rhs.handicap_index::float8,
                rhs.course_handicap
         FROM tournament_players tp
         JOIN round_handicap_snapshots rhs
           ON rhs.tournament_id = tp.tournament_id AND rhs.player_id = tp.player_id
         WHERE tp.tournament_id = '98100000-0000-0000-0000-000000000010'
         ORDER BY tp.tournament_handicap",
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows,
        vec![
            (35.9, 35.9, 49),
            (36.0, 36.0, 50),
            (36.1, 36.0, 50),
            (54.0, 36.0, 50),
        ]
    );
}
