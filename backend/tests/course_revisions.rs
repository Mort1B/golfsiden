#![cfg(feature = "database-tests")]

use golf_api::{
    domain::course_revisions::{
        self, CourseRevisionCommand, CourseRevisionSource, HoleRevisionCommand, TeeCategory,
        TeeRevisionCommand,
    },
    repositories::course_revisions as repository,
};
use sqlx::PgPool;
use std::time::Duration;

const MIGRATIONS_1_TO_9: [&str; 9] = [
    include_str!("../../migrations/0001_initial_schema.sql"),
    include_str!("../../migrations/0002_round_opening.sql"),
    include_str!("../../migrations/0003_scorecards.sql"),
    include_str!("../../migrations/0004_round_completion.sql"),
    include_str!("../../migrations/0005_auth_sessions.sql"),
    include_str!("../../migrations/0006_tournament_memberships.sql"),
    include_str!("../../migrations/0007_tournament_invitations.sql"),
    include_str!("../../migrations/0008_reusable_invitations.sql"),
    include_str!("../../migrations/0009_username_accounts_fixed_handicaps.sql"),
];
const MIGRATION_10: &str = include_str!("../../migrations/0010_course_revisions.sql");
const CURRENT_MIGRATIONS_AFTER_10: [&str; 4] = [
    include_str!("../../migrations/0011_round_flights.sql"),
    include_str!("../../migrations/0012_remove_flight_scorekeepers.sql"),
    include_str!("../../migrations/0013_two_player_foursomes.sql"),
    include_str!("../../migrations/0014_tournament_counted_rounds.sql"),
];

fn command(source: CourseRevisionSource) -> CourseRevisionCommand {
    CourseRevisionCommand {
        source,
        provider_course_id: (source == CourseRevisionSource::GolfCourseApi)
            .then(|| "Provider-ID_is-opaque".to_owned()),
        course_name: "Revision Course".to_owned(),
        location: Some("Oslo, Norway".to_owned()),
        tee: TeeRevisionCommand {
            category: TeeCategory::Female,
            name: "Blue 54".to_owned(),
            course_rating: 73.2,
            slope_rating: 137,
            holes: vec![
                HoleRevisionCommand {
                    par: 4,
                    stroke_index: 2,
                    distance: Some(401),
                },
                HoleRevisionCommand {
                    par: 3,
                    stroke_index: 1,
                    distance: None,
                },
            ],
        },
    }
}

async fn insert_complete_legacy_revision(pool: &PgPool) {
    sqlx::raw_sql(
        "INSERT INTO courses (id, name) VALUES
           ('92000000-0000-0000-0000-000000000001', 'Concurrent legacy');
         INSERT INTO tees
           (id, course_id, name, category, number_of_holes, slope_rating, course_rating)
         VALUES ('92000000-0000-0000-0000-000000000002',
                 '92000000-0000-0000-0000-000000000001', 'Tee', 'male', 2, 120, 72.0);
         INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES
           ('92000000-0000-0000-0000-000000000003',
            '92000000-0000-0000-0000-000000000002', 1, 4, 1),
           ('92000000-0000-0000-0000-000000000004',
            '92000000-0000-0000-0000-000000000002', 2, 4, 2);",
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn transaction_persists_and_reads_exact_provider_facts(pool: PgPool) {
    let validated =
        course_revisions::validate(command(CourseRevisionSource::GolfCourseApi)).unwrap();
    let mut transaction = pool.begin().await.unwrap();
    let created = repository::insert_in_transaction(&mut transaction, &validated)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let loaded = repository::find_by_course_id(&pool, created.course_id)
        .await
        .unwrap();
    assert_eq!(loaded, created);
    assert_eq!(loaded.source, CourseRevisionSource::GolfCourseApi);
    assert_eq!(
        loaded.provider_course_id.as_deref(),
        Some("Provider-ID_is-opaque")
    );
    assert_eq!(loaded.tee.category, TeeCategory::Female);
    assert_eq!(loaded.tee.course_rating_tenths, 732);
    assert_eq!(loaded.tee.holes[0].number, 1);
    assert_eq!(loaded.tee.holes[0].distance, Some(401));
    assert_eq!(loaded.tee.holes[1].distance, None);
}

#[sqlx::test(migrations = "../migrations")]
async fn manual_revision_has_no_provider_id_and_allows_null_distances(pool: PgPool) {
    let mut input = command(CourseRevisionSource::Manual);
    input.tee.holes[0].distance = None;
    let validated = course_revisions::validate(input).unwrap();
    let mut transaction = pool.begin().await.unwrap();
    let created = repository::insert_in_transaction(&mut transaction, &validated)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    let loaded = repository::find_by_course_id(&pool, created.course_id)
        .await
        .unwrap();
    assert_eq!(loaded.source, CourseRevisionSource::Manual);
    assert_eq!(loaded.provider_course_id, None);
    assert!(loaded.tee.holes.iter().all(|hole| hole.distance.is_none()));
}

#[sqlx::test(migrations = "../migrations")]
async fn failed_child_write_leaves_no_partial_revision(pool: PgPool) {
    sqlx::raw_sql(
        "CREATE FUNCTION reject_revision_hole() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN RAISE EXCEPTION 'forced hole failure' USING ERRCODE = '23514'; END; $$;
         CREATE TRIGGER reject_revision_hole BEFORE INSERT ON holes
         FOR EACH ROW EXECUTE FUNCTION reject_revision_hole();",
    )
    .execute(&pool)
    .await
    .unwrap();
    let validated = course_revisions::validate(command(CourseRevisionSource::Manual)).unwrap();
    let mut transaction = pool.begin().await.unwrap();
    assert!(
        repository::insert_in_transaction(&mut transaction, &validated)
            .await
            .is_err()
    );
    transaction.rollback().await.unwrap();

    let course_count =
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM courses WHERE name = 'Revision Course'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(course_count, 0);
}

#[sqlx::test(migrations = "../migrations")]
async fn database_rejects_incomplete_finalization_and_invalid_provenance(pool: PgPool) {
    let incomplete = sqlx::query(
        "INSERT INTO courses (id, name, source, imported_at)
         VALUES (gen_random_uuid(), 'Incomplete', 'manual', now())",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        incomplete
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("course_revision_single_complete_tee")
    );

    let invalid_provider = sqlx::query(
        "INSERT INTO courses (id, name, source, provider_course_id, imported_at)
         VALUES (gen_random_uuid(), 'Invalid provider', 'golf_course_api', '  ', now())",
    )
    .execute(&pool)
    .await
    .unwrap_err();
    assert_eq!(
        invalid_provider
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("courses_revision_provenance_check")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn finalized_course_tee_and_holes_reject_direct_mutation(pool: PgPool) {
    let validated = course_revisions::validate(command(CourseRevisionSource::Manual)).unwrap();
    let mut transaction = pool.begin().await.unwrap();
    let created = repository::insert_in_transaction(&mut transaction, &validated)
        .await
        .unwrap();
    transaction.commit().await.unwrap();

    for statement in [
        format!(
            "UPDATE courses SET name = 'Changed' WHERE id = '{}'",
            created.course_id
        ),
        format!(
            "UPDATE tees SET name = 'Changed' WHERE id = '{}'",
            created.tee.tee_id
        ),
        format!(
            "UPDATE holes SET par = 5 WHERE id = '{}'",
            created.tee.holes[0].hole_id
        ),
        format!(
            "INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES \
             (gen_random_uuid(), '{}', 3, 4, 3)",
            created.tee.tee_id
        ),
        format!(
            "DELETE FROM holes WHERE id = '{}'",
            created.tee.holes[0].hole_id
        ),
        format!("DELETE FROM tees WHERE id = '{}'", created.tee.tee_id),
        format!("DELETE FROM courses WHERE id = '{}'", created.course_id),
    ] {
        let error = sqlx::query(&statement).execute(&pool).await.unwrap_err();
        assert_eq!(
            error
                .as_database_error()
                .and_then(|database| database.constraint()),
            Some("course_revision_immutable")
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn hole_mutation_serializes_before_finalization_completeness_check(pool: PgPool) {
    insert_complete_legacy_revision(&pool).await;
    let mut mutation = pool.begin().await.unwrap();
    sqlx::query("DELETE FROM holes WHERE id = '92000000-0000-0000-0000-000000000004'")
        .execute(&mut *mutation)
        .await
        .unwrap();

    let finalization_pool = pool.clone();
    let mut finalization = tokio::spawn(async move {
        let mut transaction = finalization_pool.begin().await.unwrap();
        sqlx::query(
            "UPDATE courses SET source = 'manual', imported_at = now()
             WHERE id = '92000000-0000-0000-0000-000000000001'",
        )
        .execute(&mut *transaction)
        .await
        .unwrap();
        sqlx::query("SET CONSTRAINTS courses_validate_finalized_revision IMMEDIATE")
            .execute(&mut *transaction)
            .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut finalization)
            .await
            .is_err()
    );

    mutation.commit().await.unwrap();
    let error = finalization.await.unwrap().unwrap_err();
    assert_eq!(
        error
            .as_database_error()
            .and_then(|database| database.constraint()),
        Some("course_revision_holes_incomplete")
    );
    let source = sqlx::query_scalar::<_, Option<String>>(
        "SELECT source::text FROM courses
         WHERE id = '92000000-0000-0000-0000-000000000001'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(source, None);
}

#[sqlx::test(migrations = "../migrations")]
async fn tee_mutation_waits_for_finalization_and_then_rejects(pool: PgPool) {
    insert_complete_legacy_revision(&pool).await;
    let mut finalization = pool.begin().await.unwrap();
    sqlx::query(
        "UPDATE courses SET source = 'manual', imported_at = now()
         WHERE id = '92000000-0000-0000-0000-000000000001'",
    )
    .execute(&mut *finalization)
    .await
    .unwrap();

    let mutation_pool = pool.clone();
    let mut mutation = tokio::spawn(async move {
        sqlx::query(
            "UPDATE tees SET name = 'Racing mutation'
             WHERE id = '92000000-0000-0000-0000-000000000002'",
        )
        .execute(&mutation_pool)
        .await
    });
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut mutation)
            .await
            .is_err()
    );

    sqlx::query("SET CONSTRAINTS courses_validate_finalized_revision IMMEDIATE")
        .execute(&mut *finalization)
        .await
        .unwrap();
    finalization.commit().await.unwrap();
    let error = mutation.await.unwrap().unwrap_err();
    assert_eq!(
        error
            .as_database_error()
            .and_then(|database| database.constraint()),
        Some("course_revision_immutable")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn seed_creates_one_complete_idempotent_manual_revision(pool: PgPool) {
    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../seed.sql"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let facts = sqlx::query_as::<_, (String, Option<String>, String, i16, i64)>(
        "SELECT c.source::text, c.provider_course_id, t.category::text,
                t.number_of_holes, count(h.id)
         FROM courses c
         JOIN tees t ON t.course_id = c.id
         JOIN holes h ON h.tee_id = t.id
         WHERE c.id = '00000000-0000-0000-0000-000000003001'
         GROUP BY c.source, c.provider_course_id, t.category, t.number_of_holes",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        facts,
        ("manual".to_owned(), None, "male".to_owned(), 18, 18)
    );
}

#[sqlx::test(migrations = false)]
async fn upgrade_preserves_existing_course_hierarchy_as_mutable_legacy(pool: PgPool) {
    for migration in MIGRATIONS_1_TO_9 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        "INSERT INTO courses (id, name) VALUES
           ('91000000-0000-0000-0000-000000000001', 'Legacy');
         INSERT INTO tees (id, course_id, name, slope_rating, course_rating) VALUES
           ('91000000-0000-0000-0000-000000000002',
            '91000000-0000-0000-0000-000000000001', 'Old tee', 120, 72.0);
         INSERT INTO holes (id, tee_id, hole_number, par, stroke_index) VALUES
           ('91000000-0000-0000-0000-000000000003',
            '91000000-0000-0000-0000-000000000002', 1, 4, 1);",
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::raw_sql(MIGRATION_10).execute(&pool).await.unwrap();
    let provenance =
        sqlx::query_as::<_, (Option<String>, Option<String>, Option<String>, Option<i16>)>(
            "SELECT c.source::text, c.provider_course_id, t.category::text, t.number_of_holes
         FROM courses c JOIN tees t ON t.course_id = c.id
         WHERE c.id = '91000000-0000-0000-0000-000000000001'",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(provenance, (None, None, None, None));

    sqlx::query(
        "UPDATE holes SET par = 5
         WHERE id = '91000000-0000-0000-0000-000000000003'",
    )
    .execute(&pool)
    .await
    .unwrap();
}

#[sqlx::test(migrations = false)]
async fn upgraded_deterministic_seed_is_backfilled_and_idempotently_finalized(pool: PgPool) {
    for migration in MIGRATIONS_1_TO_9 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql(
        "INSERT INTO courses (id, name, location)
         VALUES ('00000000-0000-0000-0000-000000003001',
                 'Fjord Golfklubb', 'Vestlandet');
         INSERT INTO tees (id, course_id, name, slope_rating, course_rating)
         VALUES ('00000000-0000-0000-0000-000000003101',
                 '00000000-0000-0000-0000-000000003001', 'Gul', 132, 71.8);
         INSERT INTO holes (id, tee_id, hole_number, par, stroke_index, yardage)
         SELECT ('00000000-0000-0000-0000-' ||
                 lpad((3200 + hole_number)::text, 12, '0'))::uuid,
                '00000000-0000-0000-0000-000000003101', hole_number,
                CASE WHEN hole_number IN (3, 7, 12, 16) THEN 3
                     WHEN hole_number IN (5, 9, 14, 18) THEN 5 ELSE 4 END,
                hole_number, 120 + hole_number * 15
         FROM generate_series(1, 18) AS hole_number;",
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(MIGRATION_10).execute(&pool).await.unwrap();
    for migration in CURRENT_MIGRATIONS_AFTER_10 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }

    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../seed.sql"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let facts = sqlx::query_as::<_, (String, String, i16)>(
        "SELECT c.source::text, t.category::text, t.number_of_holes
         FROM courses c JOIN tees t ON t.course_id = c.id
         WHERE c.id = '00000000-0000-0000-0000-000000003001'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(facts, ("manual".to_owned(), "male".to_owned(), 18));
}
