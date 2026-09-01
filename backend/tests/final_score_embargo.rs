#![cfg(feature = "database-tests")]

use std::time::Duration;

use chrono::{DateTime, Utc};
use golf_api::{
    domain::scorecards::ScoreOwner,
    repositories::{round_completion, scorecards},
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

const MIGRATIONS_1_TO_16: [&str; 16] = [
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
    include_str!("../../migrations/0015_tournament_start.sql"),
    include_str!("../../migrations/0016_tournament_mandatory_round.sql"),
];
const MIGRATION_17: &str = include_str!("../../migrations/0017_final_score_embargo.sql");

#[derive(Clone, Copy)]
struct Fixture {
    tournament: Uuid,
    user: Uuid,
    players: [Uuid; 2],
    round: Uuid,
    non_final_round: Option<Uuid>,
    hole: Uuid,
    team: Option<Uuid>,
}

impl Fixture {
    fn owner(self, player_index: usize) -> ScoreOwner {
        self.team.map_or(
            ScoreOwner::Player {
                id: self.players[player_index],
            },
            |id| ScoreOwner::Team { id },
        )
    }
}

async fn seed_open_format(pool: &PgPool, format: &str, include_non_final: bool) -> Fixture {
    seed_format(pool, format, include_non_final, true).await
}

async fn seed_format(
    pool: &PgPool,
    format: &str,
    include_non_final: bool,
    start_and_open: bool,
) -> Fixture {
    let fixture = Fixture {
        tournament: Uuid::new_v4(),
        user: Uuid::new_v4(),
        players: [Uuid::new_v4(), Uuid::new_v4()],
        round: Uuid::new_v4(),
        non_final_round: include_non_final.then(Uuid::new_v4),
        hole: Uuid::new_v4(),
        team: (format != "individual_stroke_play").then(Uuid::new_v4),
    };
    let course = Uuid::new_v4();
    let tee = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO users (id, username, display_name, role) VALUES ($1, $2, 'Admin', 'player')",
    )
    .bind(fixture.user)
    .bind(format!(
        "embargo_{}",
        fixture
            .user
            .simple()
            .to_string()
            .chars()
            .take(20)
            .collect::<String>()
    ))
    .execute(pool)
    .await
    .unwrap();
    for (index, player) in fixture.players.into_iter().enumerate() {
        sqlx::query(
            "INSERT INTO players (id, display_name, current_handicap_index) VALUES ($1, $2, 8.0)",
        )
        .bind(player)
        .bind(format!("Player {index}"))
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query(
        "INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
         VALUES ($1, 'Embargo', '2026-09-01', '2026-09-02', $2)",
    )
    .bind(fixture.tournament)
    .bind(if include_non_final { 2_i16 } else { 1_i16 })
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ($1, $2, 'admin')",
    )
    .bind(fixture.tournament)
    .bind(fixture.user)
    .execute(pool)
    .await
    .unwrap();
    for player in fixture.players {
        sqlx::query(
            "INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
             VALUES ($1, $2, 8.0)",
        )
        .bind(fixture.tournament)
        .bind(player)
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query("INSERT INTO courses (id, name) VALUES ($1, 'Embargo course')")
        .bind(course)
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
         VALUES ($1, $2, 'Tee', 113, 4.0)",
    )
    .bind(tee)
    .bind(course)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
         VALUES ($1, $2, 1, 4, 1)",
    )
    .bind(fixture.hole)
    .bind(tee)
    .execute(pool)
    .await
    .unwrap();

    if let Some(non_final) = fixture.non_final_round {
        insert_round(
            pool,
            fixture,
            non_final,
            1,
            course,
            tee,
            "individual_stroke_play",
        )
        .await;
    }
    insert_round(
        pool,
        fixture,
        fixture.round,
        if include_non_final { 2 } else { 1 },
        course,
        tee,
        format,
    )
    .await;

    if let Some(team) = fixture.team {
        sqlx::query(
            "INSERT INTO teams (id, round_id, tournament_id, name) VALUES ($1, $2, $3, 'Pair')",
        )
        .bind(team)
        .bind(fixture.round)
        .bind(fixture.tournament)
        .execute(pool)
        .await
        .unwrap();
        for (index, player) in fixture.players.into_iter().enumerate() {
            sqlx::query(
                "INSERT INTO team_memberships
                   (team_id, round_id, tournament_id, player_id, display_order)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(team)
            .bind(fixture.round)
            .bind(fixture.tournament)
            .bind(player)
            .bind(index as i16 + 1)
            .execute(pool)
            .await
            .unwrap();
        }
    }

    if !start_and_open {
        return fixture;
    }

    let mut start = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('app.tournament_start_tournament_id', $1::text, true),
                set_config('app.tournament_start_user_id', $2::text, true)",
    )
    .bind(fixture.tournament)
    .bind(fixture.user)
    .execute(&mut *start)
    .await
    .unwrap();
    sqlx::query("UPDATE tournaments SET status = 'active' WHERE id = $1")
        .bind(fixture.tournament)
        .execute(&mut *start)
        .await
        .unwrap();
    start.commit().await.unwrap();

    if let Some(non_final) = fixture.non_final_round {
        open_round(pool, fixture, non_final, false).await;
    }
    open_round(
        pool,
        fixture,
        fixture.round,
        format == "two_player_foursomes",
    )
    .await;
    fixture
}

async fn insert_round(
    pool: &PgPool,
    fixture: Fixture,
    round: Uuid,
    round_number: i16,
    course: Uuid,
    tee: Uuid,
    format: &str,
) {
    sqlx::query(
        "INSERT INTO rounds
           (id, tournament_id, round_number, name, round_date, course_id, course_name,
            tee_id, tee_name, number_of_holes, handicap_allowance_percent, scoring_format)
         VALUES ($1, $2, $3, 'Round', '2026-09-01', $4, 'Embargo course',
                 $5, 'Tee', 1, $6, $7::scoring_format)",
    )
    .bind(round)
    .bind(fixture.tournament)
    .bind(round_number)
    .bind(course)
    .bind(tee)
    .bind(if format == "two_player_foursomes" {
        50_i16
    } else {
        100_i16
    })
    .bind(format)
    .execute(pool)
    .await
    .unwrap();
}

async fn open_round(pool: &PgPool, fixture: Fixture, round: Uuid, foursomes: bool) {
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
        .bind(round)
        .execute(&mut *transaction)
        .await
        .unwrap();
    for player in fixture.players {
        sqlx::query(
            "INSERT INTO round_handicap_snapshots
               (round_id, tournament_id, player_id, handicap_index, course_handicap,
                playing_handicap)
             VALUES ($1, $2, $3, 8.0, 8, 8)",
        )
        .bind(round)
        .bind(fixture.tournament)
        .bind(player)
        .execute(&mut *transaction)
        .await
        .unwrap();
    }
    if foursomes {
        sqlx::query(
            "INSERT INTO round_team_handicap_snapshots
               (round_id, tournament_id, team_id, playing_handicap)
             VALUES ($1, $2, $3, 8)",
        )
        .bind(round)
        .bind(fixture.tournament)
        .bind(fixture.team.unwrap())
        .execute(&mut *transaction)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE rounds SET status = 'open' WHERE id = $1")
        .bind(round)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
}

async fn start_direct(pool: &PgPool, fixture: Fixture) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM rounds WHERE tournament_id = $1 ORDER BY id FOR UPDATE",
    )
    .bind(fixture.tournament)
    .fetch_all(&mut *transaction)
    .await?;
    sqlx::query("SELECT id FROM tournaments WHERE id = $1 FOR UPDATE")
        .bind(fixture.tournament)
        .fetch_one(&mut *transaction)
        .await?;
    sqlx::query(
        "SELECT set_config('app.tournament_start_tournament_id', $1::text, true),
                set_config('app.tournament_start_user_id', $2::text, true)",
    )
    .bind(fixture.tournament)
    .bind(fixture.user)
    .execute(&mut *transaction)
    .await?;
    sqlx::query("UPDATE tournaments SET status = 'active' WHERE id = $1")
        .bind(fixture.tournament)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await
}

async fn score(pool: &PgPool, fixture: Fixture, round: Uuid, owner: ScoreOwner, gross: i16) {
    scorecards::save(
        pool,
        scorecards::SaveScore {
            round_id: round,
            hole_id: fixture.hole,
            owner,
            gross_strokes: gross,
            submitted_by: fixture.user,
        },
    )
    .await
    .unwrap();
}

async fn deadline(pool: &PgPool, round: Uuid) -> Option<DateTime<Utc>> {
    sqlx::query_scalar("SELECT final_scores_hidden_until FROM rounds WHERE id = $1")
        .bind(round)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn round_updated_at(pool: &PgPool, round: Uuid) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT updated_at FROM rounds WHERE id = $1")
        .bind(round)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn insert_confirmation_at(
    transaction: &mut Transaction<'_, Postgres>,
    fixture: Fixture,
    round: Uuid,
    owner: ScoreOwner,
    confirmed_at: &str,
) {
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(round)
        .execute(&mut **transaction)
        .await
        .unwrap();
    match owner {
        ScoreOwner::Player { id } => {
            sqlx::query(
                "INSERT INTO scorecard_confirmations
                   (id, round_id, tournament_id, player_id, confirmed_by, confirmed_at)
                 VALUES ($1, $2, $3, $4, $5, $6::timestamptz)",
            )
            .bind(Uuid::new_v4())
            .bind(round)
            .bind(fixture.tournament)
            .bind(id)
            .bind(fixture.user)
            .bind(confirmed_at)
            .execute(&mut **transaction)
            .await
            .unwrap();
        }
        ScoreOwner::Team { id } => {
            sqlx::query(
                "INSERT INTO scorecard_confirmations
                   (id, round_id, tournament_id, team_id, confirmed_by, confirmed_at)
                 VALUES ($1, $2, $3, $4, $5, $6::timestamptz)",
            )
            .bind(Uuid::new_v4())
            .bind(round)
            .bind(fixture.tournament)
            .bind(id)
            .bind(fixture.user)
            .bind(confirmed_at)
            .execute(&mut **transaction)
            .await
            .unwrap();
        }
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn final_confirmation_starts_once_and_pre_expiry_correction_restarts(pool: PgPool) {
    let fixture = seed_open_format(&pool, "individual_stroke_play", true).await;
    let non_final = fixture.non_final_round.unwrap();
    let reclassify = sqlx::query("UPDATE tournaments SET number_of_rounds = 3 WHERE id = $1")
        .bind(fixture.tournament)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        reclassify
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("tournament_round_count_started_frozen")
    );
    assert_eq!(
        sqlx::query_scalar::<_, i16>("SELECT number_of_rounds FROM tournaments WHERE id = $1")
            .bind(fixture.tournament)
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    for index in 0..2 {
        score(&pool, fixture, non_final, fixture.owner(index), 4).await;
        scorecards::confirm(&pool, non_final, fixture.owner(index), fixture.user)
            .await
            .unwrap();
    }
    assert_eq!(deadline(&pool, non_final).await, None);

    for index in 0..2 {
        score(&pool, fixture, fixture.round, fixture.owner(index), 4).await;
    }
    scorecards::confirm(&pool, fixture.round, fixture.owner(0), fixture.user)
        .await
        .unwrap();
    assert_eq!(deadline(&pool, fixture.round).await, None);
    let before: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&pool)
        .await
        .unwrap();
    scorecards::confirm(&pool, fixture.round, fixture.owner(1), fixture.user)
        .await
        .unwrap();
    let after: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let first = deadline(&pool, fixture.round).await.unwrap();
    assert!(first >= before + chrono::Duration::hours(24));
    assert!(first <= after + chrono::Duration::hours(24));

    let updated_at = round_updated_at(&pool, fixture.round).await;
    let duplicate = scorecards::confirm(&pool, fixture.round, fixture.owner(1), fixture.user)
        .await
        .unwrap();
    assert!(!duplicate.changed);
    assert_eq!(deadline(&pool, fixture.round).await, Some(first));
    assert_eq!(round_updated_at(&pool, fixture.round).await, updated_at);

    score(&pool, fixture, fixture.round, fixture.owner(0), 5).await;
    assert_eq!(deadline(&pool, fixture.round).await, None);
    scorecards::confirm(&pool, fixture.round, fixture.owner(0), fixture.user)
        .await
        .unwrap();
    let restarted = deadline(&pool, fixture.round).await.unwrap();
    assert!(restarted > first);

    round_completion::complete(&pool, fixture.round)
        .await
        .unwrap();
    assert_eq!(deadline(&pool, fixture.round).await, Some(restarted));
    round_completion::lock(&pool, fixture.round).await.unwrap();
    assert_eq!(deadline(&pool, fixture.round).await, Some(restarted));

    let direct = sqlx::query(
        "UPDATE rounds SET final_scores_hidden_until = clock_timestamp() + interval '1 hour'
         WHERE id = $1",
    )
    .bind(fixture.round)
    .execute(&pool)
    .await
    .unwrap_err();
    let direct = direct.as_database_error().unwrap();
    assert_eq!(direct.code().as_deref(), Some("23514"));
    assert!(
        direct
            .message()
            .contains("final-score embargo must be changed by the confirmation workflow")
    );

    sqlx::query("DELETE FROM tournaments WHERE id = $1")
        .bind(fixture.tournament)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM rounds WHERE tournament_id = $1")
            .bind(fixture.tournament)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn team_formats_start_the_same_database_owned_deadline(pool: PgPool) {
    for format in ["team_scramble", "two_player_foursomes"] {
        let fixture = seed_open_format(&pool, format, false).await;
        score(&pool, fixture, fixture.round, fixture.owner(0), 4).await;
        let before: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
            .fetch_one(&pool)
            .await
            .unwrap();
        scorecards::confirm(&pool, fixture.round, fixture.owner(0), fixture.user)
            .await
            .unwrap();
        let stored = deadline(&pool, fixture.round).await.unwrap();
        assert!(stored >= before + chrono::Duration::hours(24));
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_confirmations_and_correction_lifecycle_follow_round_lock_order(pool: PgPool) {
    let fixture = seed_open_format(&pool, "individual_stroke_play", false).await;
    for index in 0..2 {
        score(&pool, fixture, fixture.round, fixture.owner(index), 4).await;
    }
    let first_pool = pool.clone();
    let second_pool = pool.clone();
    let first = tokio::spawn(async move {
        scorecards::confirm(&first_pool, fixture.round, fixture.owner(0), fixture.user).await
    });
    let second = tokio::spawn(async move {
        scorecards::confirm(&second_pool, fixture.round, fixture.owner(1), fixture.user).await
    });
    let (first, second) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(first, second)
    })
    .await
    .expect("concurrent final confirmations deadlocked");
    first.unwrap().unwrap();
    second.unwrap().unwrap();
    let original = deadline(&pool, fixture.round).await.unwrap();

    let mut correction = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(fixture.round)
        .fetch_one(&mut *correction)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(fixture.round)
        .execute(&mut *correction)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE scores SET gross_strokes = 5, submitted_by = $2
         WHERE round_id = $1 AND player_id = $3",
    )
    .bind(fixture.round)
    .bind(fixture.user)
    .bind(fixture.players[0])
    .execute(&mut *correction)
    .await
    .unwrap();
    let completion_pool = pool.clone();
    let mut completion =
        tokio::spawn(
            async move { round_completion::complete(&completion_pool, fixture.round).await },
        );
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut completion)
            .await
            .is_err()
    );
    correction.commit().await.unwrap();
    assert!(completion.await.unwrap().is_err());
    assert_eq!(deadline(&pool, fixture.round).await, None);
    assert!(original > Utc::now());

    scorecards::confirm(&pool, fixture.round, fixture.owner(0), fixture.user)
        .await
        .unwrap();
    round_completion::complete(&pool, fixture.round)
        .await
        .unwrap();
    let mut locked_correction = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(fixture.round)
        .fetch_one(&mut *locked_correction)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(fixture.round)
        .execute(&mut *locked_correction)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE scores SET gross_strokes = 6, submitted_by = $2
         WHERE round_id = $1 AND player_id = $3",
    )
    .bind(fixture.round)
    .bind(fixture.user)
    .bind(fixture.players[0])
    .execute(&mut *locked_correction)
    .await
    .unwrap();
    let lock_pool = pool.clone();
    let mut locking =
        tokio::spawn(async move { round_completion::lock(&lock_pool, fixture.round).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut locking)
            .await
            .is_err()
    );
    locked_correction.commit().await.unwrap();
    assert!(locking.await.unwrap().is_err());
    assert_eq!(deadline(&pool, fixture.round).await, None);
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status::text FROM rounds WHERE id = $1")
            .bind(fixture.round)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "completed"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn start_and_final_round_renumber_serialize_without_identity_drift(pool: PgPool) {
    let fixture = seed_format(&pool, "individual_stroke_play", true, false).await;

    sqlx::query("UPDATE rounds SET round_number = 3 WHERE id = $1")
        .bind(fixture.round)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET round_number = 2 WHERE id = $1")
        .bind(fixture.round)
        .execute(&pool)
        .await
        .unwrap();

    let mut atomic = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('app.tournament_start_tournament_id', $1::text, true),
                set_config('app.tournament_start_user_id', $2::text, true)",
    )
    .bind(fixture.tournament)
    .bind(fixture.user)
    .execute(&mut *atomic)
    .await
    .unwrap();
    let atomic_error =
        sqlx::query("UPDATE tournaments SET status = 'active', number_of_rounds = 3 WHERE id = $1")
            .bind(fixture.tournament)
            .execute(&mut *atomic)
            .await
            .unwrap_err();
    assert_eq!(
        atomic_error
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("tournament_round_count_started_frozen")
    );
    atomic.rollback().await.unwrap();

    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let start_pool = pool.clone();
    let start_barrier = barrier.clone();
    let starting = tokio::spawn(async move {
        start_barrier.wait().await;
        start_direct(&start_pool, fixture).await
    });
    let renumber_pool = pool.clone();
    let renumber_barrier = barrier.clone();
    let renumbering = tokio::spawn(async move {
        renumber_barrier.wait().await;
        sqlx::query("UPDATE rounds SET round_number = 3 WHERE id = $1")
            .bind(fixture.round)
            .execute(&renumber_pool)
            .await
    });
    barrier.wait().await;
    let (started, renumbered) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(starting, renumbering)
    })
    .await
    .expect("tournament start and final-round renumber deadlocked");
    let started = started.unwrap();
    let renumbered = renumbered.unwrap();
    assert_ne!(started.is_ok(), renumbered.is_ok());

    if started.is_ok() {
        let renumber_error = renumbered.unwrap_err();
        assert_eq!(
            renumber_error
                .as_database_error()
                .and_then(|error| error.constraint()),
            Some("round_number_started_frozen")
        );
    } else {
        renumbered.unwrap();
        assert_eq!(
            sqlx::query_scalar::<_, String>("SELECT status::text FROM tournaments WHERE id = $1",)
                .bind(fixture.tournament)
                .fetch_one(&pool)
                .await
                .unwrap(),
            "draft"
        );
        sqlx::query("UPDATE rounds SET round_number = 2 WHERE id = $1")
            .bind(fixture.round)
            .execute(&pool)
            .await
            .unwrap();
        start_direct(&pool, fixture).await.unwrap();
    }

    assert_eq!(
        sqlx::query_scalar::<_, i16>("SELECT round_number FROM rounds WHERE id = $1")
            .bind(fixture.round)
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    let post_start = sqlx::query("UPDATE rounds SET round_number = 3 WHERE id = $1")
        .bind(fixture.round)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        post_start
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("round_number_started_frozen")
    );
    open_round(&pool, fixture, fixture.round, false).await;
    for index in 0..2 {
        score(&pool, fixture, fixture.round, fixture.owner(index), 4).await;
        scorecards::confirm(&pool, fixture.round, fixture.owner(index), fixture.user)
            .await
            .unwrap();
    }
    assert!(deadline(&pool, fixture.round).await.is_some());
}

#[sqlx::test(migrations = false)]
async fn schema_16_upgrade_backfills_historical_clock_and_preserves_expired_reveal(pool: PgPool) {
    for migration in MIGRATIONS_1_TO_16 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    let fixture = seed_open_format(&pool, "individual_stroke_play", true).await;
    let unready = seed_open_format(&pool, "individual_stroke_play", false).await;
    let non_final = fixture.non_final_round.unwrap();
    for round in [non_final, fixture.round] {
        for index in 0..2 {
            score(&pool, fixture, round, fixture.owner(index), 4).await;
        }
    }
    let mut confirmations = pool.begin().await.unwrap();
    insert_confirmation_at(
        &mut confirmations,
        fixture,
        non_final,
        fixture.owner(0),
        "2026-08-01 10:00:00+00",
    )
    .await;
    insert_confirmation_at(
        &mut confirmations,
        fixture,
        non_final,
        fixture.owner(1),
        "2026-08-01 11:00:00+00",
    )
    .await;
    insert_confirmation_at(
        &mut confirmations,
        fixture,
        fixture.round,
        fixture.owner(0),
        "2026-08-02 10:00:00+00",
    )
    .await;
    insert_confirmation_at(
        &mut confirmations,
        fixture,
        fixture.round,
        fixture.owner(1),
        "2026-08-02 12:00:00+00",
    )
    .await;
    confirmations.commit().await.unwrap();
    score(&pool, unready, unready.round, unready.owner(0), 4).await;
    let mut unready_confirmation = pool.begin().await.unwrap();
    insert_confirmation_at(
        &mut unready_confirmation,
        unready,
        unready.round,
        unready.owner(0),
        "2026-08-02 13:00:00+00",
    )
    .await;
    unready_confirmation.commit().await.unwrap();

    sqlx::raw_sql(MIGRATION_17).execute(&pool).await.unwrap();
    assert_eq!(deadline(&pool, non_final).await, None);
    assert_eq!(deadline(&pool, unready.round).await, None);
    assert_eq!(
        deadline(&pool, fixture.round).await.unwrap().to_rfc3339(),
        "2026-08-03T12:00:00+00:00"
    );
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT final_score_embargo_is_unexpired($1::timestamptz, $1::timestamptz)",
        )
        .bind("2026-08-03 12:00:00+00")
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    assert!(
        !sqlx::query_scalar::<_, bool>(
            "SELECT final_score_embargo_is_unexpired($1::timestamptz, $2::timestamptz)",
        )
        .bind("2026-08-03 12:00:00+00")
        .bind("2026-08-03 12:00:01+00")
        .fetch_one(&pool)
        .await
        .unwrap()
    );

    score(&pool, fixture, fixture.round, fixture.owner(0), 5).await;
    assert_eq!(
        deadline(&pool, fixture.round).await.unwrap().to_rfc3339(),
        "2026-08-03T12:00:00+00:00"
    );
    scorecards::confirm(&pool, fixture.round, fixture.owner(0), fixture.user)
        .await
        .unwrap();
    assert_eq!(
        deadline(&pool, fixture.round).await.unwrap().to_rfc3339(),
        "2026-08-03T12:00:00+00:00"
    );
}
