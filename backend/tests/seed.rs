#![cfg(feature = "database-tests")]

use golf_api::repositories::round_lifecycle;
use sqlx::PgPool;
use uuid::{Uuid, uuid};

const SEEDED_ROUNDS: [Uuid; 5] = [
    uuid!("00000000-0000-0000-0000-000000004001"),
    uuid!("00000000-0000-0000-0000-000000004002"),
    uuid!("00000000-0000-0000-0000-000000004003"),
    uuid!("00000000-0000-0000-0000-000000004004"),
    uuid!("00000000-0000-0000-0000-000000004005"),
];

async fn run_seed(pool: &PgPool) {
    sqlx::raw_sql(include_str!("../seed.sql"))
        .execute(pool)
        .await
        .unwrap();
}

async fn assert_representative_pairings(pool: &PgPool) {
    let counts = sqlx::query_as::<_, (i64, i64, i64, i64)>(
        "SELECT
            (SELECT count(*) FROM teams),
            (SELECT count(*) FROM team_memberships),
            (SELECT count(*) FROM flights),
            (SELECT count(*) FROM flight_memberships)",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(counts, (12, 24, 10, 40));

    let flights = sqlx::query_scalar::<_, String>(
        "SELECT concat(
             r.round_number, ':', right(f.id::text, 4), ':', f.starting_hole, ':',
             string_agg(right(fm.player_id::text, 4), ',' ORDER BY fm.display_order)
         )
         FROM flights f
         JOIN rounds r ON r.id = f.round_id
         JOIN flight_memberships fm ON fm.flight_id = f.id
         GROUP BY r.round_number, f.id, f.starting_hole
         ORDER BY r.round_number, f.starting_hole",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(
        flights,
        vec![
            "1:6001:1:1001,1008,1002,1007",
            "1:6002:10:1003,1006,1004,1005",
            "2:6003:1:1001,1006,1003,1008",
            "2:6004:10:1002,1005,1004,1007",
            "3:6005:1:1001,1003,1005,1007",
            "3:6006:10:1002,1004,1006,1008",
            "4:6007:1:1001,1005,1002,1006",
            "4:6008:10:1003,1007,1004,1008",
            "5:6009:1:1001,1004,1006,1007",
            "5:6010:10:1002,1003,1005,1008",
        ]
    );

    let teams = sqlx::query_scalar::<_, String>(
        "SELECT concat(
             r.round_number, ':', right(t.id::text, 4), ':',
             coalesce(t.starting_hole::text, '-'), ':',
             string_agg(right(tm.player_id::text, 4), ',' ORDER BY tm.display_order)
         )
         FROM teams t
         JOIN rounds r ON r.id = t.round_id
         JOIN team_memberships tm ON tm.team_id = t.id
         GROUP BY r.round_number, t.id, t.starting_hole
         ORDER BY r.round_number, t.id",
    )
    .fetch_all(pool)
    .await
    .unwrap();
    assert_eq!(
        teams,
        vec![
            "1:5001:-:1001,1008",
            "1:5002:-:1002,1007",
            "1:5003:-:1003,1006",
            "1:5004:-:1004,1005",
            "2:5005:-:1001,1006",
            "2:5006:-:1002,1005",
            "2:5007:-:1003,1008",
            "2:5008:-:1004,1007",
            "4:5009:-:1001,1005",
            "4:5010:-:1002,1006",
            "4:5011:-:1003,1007",
            "4:5012:-:1004,1008",
        ]
    );

    let individual_team_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM teams t JOIN rounds r ON r.id = t.round_id
         WHERE r.scoring_format = 'individual_stroke_play'",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(individual_team_count, 0);

    let invalid_scramble_teams = sqlx::query_scalar::<_, i64>(
        "SELECT count(*)
         FROM teams t
         JOIN rounds r ON r.id = t.round_id
         WHERE r.scoring_format = 'team_scramble'
           AND (
             (SELECT count(*) FROM team_memberships tm WHERE tm.team_id = t.id) <> 2
             OR (SELECT count(DISTINCT fm.flight_id)
                 FROM team_memberships tm
                 LEFT JOIN flight_memberships fm
                   ON fm.round_id = tm.round_id AND fm.player_id = tm.player_id
                 WHERE tm.team_id = t.id) <> 1
           )",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(invalid_scramble_teams, 0);
}

#[sqlx::test(migrations = "../migrations")]
async fn development_seed_has_ready_idempotent_representative_pairings(pool: PgPool) {
    run_seed(&pool).await;

    // Recreate the exact pre-flight round-two seed state to exercise the guarded
    // draft upgrade rather than only the already-current idempotent path.
    sqlx::query(
        "DELETE FROM flights
         WHERE id IN (
           '00000000-0000-0000-0000-000000006003',
           '00000000-0000-0000-0000-000000006004'
         )",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE teams
         SET starting_hole = CASE
           WHEN id IN (
             '00000000-0000-0000-0000-000000005005',
             '00000000-0000-0000-0000-000000005006'
           ) THEN 1 ELSE 10 END
         WHERE id BETWEEN
           '00000000-0000-0000-0000-000000005005'
           AND '00000000-0000-0000-0000-000000005008'",
    )
    .execute(&pool)
    .await
    .unwrap();

    run_seed(&pool).await;
    assert_representative_pairings(&pool).await;

    let usernames = sqlx::query_scalar::<_, String>("SELECT username FROM users ORDER BY username")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        usernames,
        vec![
            "admin",
            "anders",
            "bjarne",
            "christian",
            "daniel",
            "eirik",
            "fredrik",
            "geir",
            "henrik",
        ]
    );

    for round_id in SEEDED_ROUNDS {
        let validation = round_lifecycle::pairing_validation(&pool, round_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            validation.ready,
            "round {round_id}: {:?}",
            validation.issues
        );
    }

    // A later seed execution must not attempt any pairing write against a
    // frozen round; its preserved assignments remain unchanged.
    round_lifecycle::open(&pool, SEEDED_ROUNDS[0])
        .await
        .unwrap();
    run_seed(&pool).await;
    assert_representative_pairings(&pool).await;
}
