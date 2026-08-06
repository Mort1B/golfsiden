#![cfg(feature = "database-tests")]

use sqlx::PgPool;
use uuid::uuid;

const MIGRATION_1: &str = include_str!("../../migrations/0001_initial_schema.sql");
const MIGRATION_2: &str = include_str!("../../migrations/0002_round_opening.sql");
const MIGRATION_3: &str = include_str!("../../migrations/0003_scorecards.sql");
const MIGRATION_4: &str = include_str!("../../migrations/0004_round_completion.sql");

const BASE_V3_DATA: &str = r#"
INSERT INTO users (id, email, display_name, role)
VALUES ('60000000-0000-0000-0000-000000000001', 'upgrade@example.test', 'Admin', 'admin');
INSERT INTO players (id, display_name, current_handicap_index)
VALUES ('60000000-0000-0000-0000-000000000002', 'Player', 10.0);
INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
VALUES ('60000000-0000-0000-0000-000000000003', 'Upgrade', '2026-01-01', '2026-01-01', 1);
INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
VALUES ('60000000-0000-0000-0000-000000000003', '60000000-0000-0000-0000-000000000002', 10.0);
INSERT INTO courses (id, name)
VALUES ('60000000-0000-0000-0000-000000000004', 'Course');
INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
VALUES ('60000000-0000-0000-0000-000000000005', '60000000-0000-0000-0000-000000000004', 'Tee', 113, 4.0);
INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
VALUES ('60000000-0000-0000-0000-000000000006', '60000000-0000-0000-0000-000000000005', 1, 4, 1);
"#;

async fn apply_v3(pool: &PgPool) {
    for migration in [MIGRATION_1, MIGRATION_2, MIGRATION_3] {
        sqlx::raw_sql(migration).execute(pool).await.unwrap();
    }
    sqlx::raw_sql(BASE_V3_DATA).execute(pool).await.unwrap();
}

async fn create_valid_completed_round(pool: &PgPool) {
    let round_id = uuid!("60000000-0000-0000-0000-000000000007");
    sqlx::query("INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format) VALUES ($1, '60000000-0000-0000-0000-000000000003', 1, 'Valid', '2026-01-01', '60000000-0000-0000-0000-000000000004', 'Course', '60000000-0000-0000-0000-000000000005', 'Tee', 1, 'individual_stroke_play')")
        .bind(round_id).execute(pool).await.unwrap();

    let mut opening = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(round_id)
        .fetch_one(&mut *opening)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
        .bind(round_id)
        .execute(&mut *opening)
        .await
        .unwrap();
    sqlx::query("INSERT INTO round_handicap_snapshots (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap) VALUES ($1, '60000000-0000-0000-0000-000000000003', '60000000-0000-0000-0000-000000000002', 10.0, 10, 10)")
        .bind(round_id).execute(&mut *opening).await.unwrap();
    sqlx::query("UPDATE rounds SET status = 'open' WHERE id = $1")
        .bind(round_id)
        .execute(&mut *opening)
        .await
        .unwrap();
    opening.commit().await.unwrap();

    let mut scoring = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id = $1 FOR UPDATE")
        .bind(round_id)
        .fetch_one(&mut *scoring)
        .await
        .unwrap();
    sqlx::query("SELECT set_config('app.score_mutation_round_id', $1::text, true)")
        .bind(round_id)
        .execute(&mut *scoring)
        .await
        .unwrap();
    sqlx::query("INSERT INTO scores (id, round_id, tournament_id, hole_id, player_id, gross_strokes, submitted_by) VALUES ('60000000-0000-0000-0000-000000000008', $1, '60000000-0000-0000-0000-000000000003', '60000000-0000-0000-0000-000000000006', '60000000-0000-0000-0000-000000000002', 4, '60000000-0000-0000-0000-000000000001')")
        .bind(round_id).execute(&mut *scoring).await.unwrap();
    sqlx::query("INSERT INTO scorecard_confirmations (id, round_id, tournament_id, player_id, confirmed_by) VALUES ('60000000-0000-0000-0000-000000000009', $1, '60000000-0000-0000-0000-000000000003', '60000000-0000-0000-0000-000000000002', '60000000-0000-0000-0000-000000000001')")
        .bind(round_id).execute(&mut *scoring).await.unwrap();
    sqlx::query("UPDATE rounds SET status = 'completed' WHERE id = $1")
        .bind(round_id)
        .execute(&mut *scoring)
        .await
        .unwrap();
    scoring.commit().await.unwrap();
}

#[sqlx::test(migrations = false)]
async fn valid_completed_v3_round_upgrades_to_v4(pool: PgPool) {
    apply_v3(&pool).await;
    create_valid_completed_round(&pool).await;

    sqlx::raw_sql(MIGRATION_4).execute(&pool).await.unwrap();
    let ready = sqlx::query_scalar::<_, bool>(
        "SELECT round_scorecards_ready('60000000-0000-0000-0000-000000000007')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(ready);
}

#[sqlx::test(migrations = false)]
async fn invalid_completed_v3_round_fails_fast_with_round_id(pool: PgPool) {
    apply_v3(&pool).await;
    let round_id = uuid!("60000000-0000-0000-0000-000000000017");
    sqlx::query("INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_name, tee_name, number_of_holes, status, scoring_format) VALUES ($1, '60000000-0000-0000-0000-000000000003', 1, 'Invalid', '2026-01-01', 'Missing', 'Missing', 18, 'completed', 'individual_stroke_play')")
        .bind(round_id).execute(&pool).await.unwrap();

    let error = sqlx::raw_sql(MIGRATION_4).execute(&pool).await.unwrap_err();
    let message = error
        .as_database_error()
        .map(|database| database.message())
        .unwrap_or_default();
    assert!(message.contains("migration blocked"));
    assert!(message.contains("incomplete or unconfirmed scorecards"));
    assert!(message.contains(&round_id.to_string()));
}
