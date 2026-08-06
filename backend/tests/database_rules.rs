#![cfg(feature = "database-tests")]

use sqlx::{PgPool, Row};

const BASE_DATA: &str = r#"
INSERT INTO users (id, email, display_name, role)
VALUES ('10000000-0000-0000-0000-000000000001', 'test@example.test', 'Test Admin', 'admin');
INSERT INTO players (id, display_name, current_handicap_index)
VALUES ('10000000-0000-0000-0000-000000000002', 'Test Player', 12.0);
INSERT INTO tournaments (id, name, start_date, end_date, number_of_rounds)
VALUES ('10000000-0000-0000-0000-000000000003', 'Test Tournament', '2026-01-01', '2026-01-01', 1);
INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
VALUES ('10000000-0000-0000-0000-000000000003', '10000000-0000-0000-0000-000000000002', 12.0);
INSERT INTO courses (id, name) VALUES ('10000000-0000-0000-0000-000000000004', 'Test Course');
INSERT INTO tees (id, course_id, name) VALUES ('10000000-0000-0000-0000-000000000005', '10000000-0000-0000-0000-000000000004', 'Test Tee');
INSERT INTO holes (id, tee_id, hole_number, par, stroke_index)
VALUES ('10000000-0000-0000-0000-000000000006', '10000000-0000-0000-0000-000000000005', 1, 4, 1);
INSERT INTO rounds (id, tournament_id, round_number, name, round_date, course_id, course_name, tee_id, tee_name, number_of_holes, scoring_format)
VALUES ('10000000-0000-0000-0000-000000000007', '10000000-0000-0000-0000-000000000003', 1, 'Round 1', '2026-01-01', '10000000-0000-0000-0000-000000000004', 'Test Course', '10000000-0000-0000-0000-000000000005', 'Test Tee', 1, 'team_scramble');
INSERT INTO teams (id, round_id, tournament_id, name) VALUES
('10000000-0000-0000-0000-000000000008', '10000000-0000-0000-0000-000000000007', '10000000-0000-0000-0000-000000000003', 'Team 1'),
('10000000-0000-0000-0000-000000000009', '10000000-0000-0000-0000-000000000007', '10000000-0000-0000-0000-000000000003', 'Team 2');
"#;

async fn seed_base(pool: &PgPool) {
    sqlx::raw_sql(BASE_DATA).execute(pool).await.unwrap();
}

async fn open_fixture_round(pool: &PgPool) {
    let round_id = uuid::uuid!("10000000-0000-0000-0000-000000000007");
    let mut transaction = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
        .bind(round_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("INSERT INTO round_handicap_snapshots (round_id, tournament_id, player_id, handicap_index, course_handicap, playing_handicap) VALUES ($1, $2, $3, 12.0, 12, 12)")
        .bind(round_id)
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000003"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000002"))
        .execute(&mut *transaction)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET status = 'open' WHERE id = $1")
        .bind(round_id)
        .execute(&mut *transaction)
        .await
        .unwrap();
    transaction.commit().await.unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn player_cannot_join_two_teams_in_one_round(pool: PgPool) {
    seed_base(&pool).await;
    sqlx::query("INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id) VALUES ($1, $2, $3, $4)")
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000008"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000007"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000003"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000002"))
        .execute(&pool).await.unwrap();
    let error = sqlx::query("INSERT INTO team_memberships (team_id, round_id, tournament_id, player_id) VALUES ($1, $2, $3, $4)")
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000009"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000007"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000003"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000002"))
        .execute(&pool).await.unwrap_err();
    assert!(
        error
            .as_database_error()
            .is_some_and(|error| error.is_unique_violation())
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn score_cannot_have_both_player_and_team_owners(pool: PgPool) {
    seed_base(&pool).await;
    let error = sqlx::query("INSERT INTO scores (id, round_id, tournament_id, hole_id, player_id, team_id, gross_strokes, submitted_by) VALUES ($1, $2, $3, $4, $5, $6, 4, $7)")
        .bind(uuid::Uuid::new_v4())
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000007"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000003"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000006"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000002"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000008"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000001"))
        .execute(&pool).await.unwrap_err();
    assert!(
        error
            .as_database_error()
            .is_some_and(|error| error.is_check_violation())
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn locked_round_rejects_normal_score_changes(pool: PgPool) {
    seed_base(&pool).await;
    let score_id = uuid::Uuid::new_v4();
    sqlx::query("INSERT INTO scores (id, round_id, tournament_id, hole_id, team_id, gross_strokes, submitted_by) VALUES ($1, $2, $3, $4, $5, 4, $6)")
        .bind(score_id)
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000007"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000003"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000006"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000008"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000001"))
        .execute(&pool).await.unwrap();
    open_fixture_round(&pool).await;
    sqlx::query("UPDATE rounds SET status = 'completed' WHERE id = $1")
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000007"))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE rounds SET status = 'locked' WHERE id = $1")
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000007"))
        .execute(&pool)
        .await
        .unwrap();
    let error = sqlx::query("UPDATE scores SET gross_strokes = 5 WHERE id = $1")
        .bind(score_id)
        .execute(&pool)
        .await
        .unwrap_err();
    assert!(
        error
            .as_database_error()
            .is_some_and(|error| error.is_check_violation())
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn handicap_change_does_not_modify_round_snapshot(pool: PgPool) {
    seed_base(&pool).await;
    open_fixture_round(&pool).await;
    sqlx::query("UPDATE players SET current_handicap_index = 8.0 WHERE id = $1")
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000002"))
        .execute(&pool)
        .await
        .unwrap();
    let row = sqlx::query("SELECT handicap_index::float8 AS handicap FROM round_handicap_snapshots WHERE round_id = $1 AND player_id = $2")
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000007"))
        .bind(uuid::uuid!("10000000-0000-0000-0000-000000000002"))
        .fetch_one(&pool).await.unwrap();
    assert_eq!(row.get::<f64, _>("handicap"), 12.0);
}
