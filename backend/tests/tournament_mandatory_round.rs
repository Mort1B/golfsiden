#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use chrono::{Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    repositories::{auth, tournaments},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::sync::broadcast::error::TryRecvError;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TOURNAMENT: Uuid = uuid!("16000000-0000-0000-0000-000000000001");
const ADMIN: Uuid = uuid!("16000000-0000-0000-0000-000000000002");
const ROUND_ONE: Uuid = uuid!("16000000-0000-0000-0000-000000000011");
const ROUND_TWO: Uuid = uuid!("16000000-0000-0000-0000-000000000012");
const TOKEN: &str = "mandatory-round-admin-token";

const MIGRATIONS_1_TO_15: [&str; 15] = [
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
];
const MIGRATION_16: &str = include_str!("../../migrations/0016_tournament_mandatory_round.sql");

async fn seed(pool: &PgPool) {
    sqlx::raw_sql(
        "INSERT INTO users (id, username, display_name, role)
         VALUES ('16000000-0000-0000-0000-000000000002', 'mandatory_admin', 'Admin', 'player');
         INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds, counted_rounds)
         VALUES ('16000000-0000-0000-0000-000000000001', 'Mandatory',
                 '2026-09-01', '2026-09-02', 2, 1);
         INSERT INTO tournament_memberships (tournament_id, user_id, role)
         VALUES ('16000000-0000-0000-0000-000000000001',
                 '16000000-0000-0000-0000-000000000002', 'admin');
         INSERT INTO rounds
           (id, tournament_id, round_number, name, round_date, course_name, tee_name,
            scoring_format)
         VALUES
           ('16000000-0000-0000-0000-000000000011',
            '16000000-0000-0000-0000-000000000001', 1, 'One', '2026-09-01', '', '',
            'individual_stroke_play'),
           ('16000000-0000-0000-0000-000000000012',
            '16000000-0000-0000-0000-000000000001', 2, 'Two', '2026-09-02', '', '',
            'individual_stroke_play');",
    )
    .execute(pool)
    .await
    .unwrap();
    auth::create_session(
        pool,
        ADMIN,
        &hash_session_token(TOKEN),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
}

async fn updated_at(pool: &PgPool) -> chrono::DateTime<Utc> {
    sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn patch(body: Value) -> Request<Body> {
    Request::patch(format!("/api/tournaments/{TOURNAMENT}/counted-rounds"))
        .header(header::COOKIE, format!("golf_session={TOKEN}"))
        .header("x-csrf-token", derive_csrf_token(TOKEN))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn response_json(response: axum::response::Response) -> Value {
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

async fn race_configuration_with_delete(
    pool: &PgPool,
    session_id: Uuid,
    target_round_id: Uuid,
) -> (
    Result<tournaments::UpdateCountedRoundsResult, tournaments::TournamentMutationError>,
    bool,
) {
    let expected = updated_at(pool).await;
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(3));
    let update_pool = pool.clone();
    let update_barrier = barrier.clone();
    let update = tokio::spawn(async move {
        update_barrier.wait().await;
        tournaments::update_counted_rounds_authorized(
            &update_pool,
            session_id,
            TOURNAMENT,
            1,
            Some(target_round_id),
            expected,
        )
        .await
    });
    let delete_pool = pool.clone();
    let delete_barrier = barrier.clone();
    let delete = tokio::spawn(async move {
        delete_barrier.wait().await;
        let mut transaction = delete_pool.begin().await.unwrap();
        if sqlx::query("DELETE FROM rounds WHERE id = $1")
            .bind(target_round_id)
            .execute(&mut *transaction)
            .await
            .is_err()
        {
            return false;
        }
        transaction.commit().await.is_ok()
    });
    barrier.wait().await;
    let (update, deleted) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(update, delete)
    })
    .await
    .expect("mandatory-round configuration and deletion deadlocked");
    (update.unwrap(), deleted.unwrap())
}

#[sqlx::test(migrations = false)]
async fn upgrade_enforces_same_tournament_delete_and_parent_cascade(pool: PgPool) {
    for migration in MIGRATIONS_1_TO_15 {
        sqlx::raw_sql(migration).execute(&pool).await.unwrap();
    }
    seed(&pool).await;
    let other_tournament = Uuid::new_v4();
    let other_round = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO tournaments
           (id, name, start_date, end_date, number_of_rounds, counted_rounds)
         VALUES ($1, 'Other', '2026-09-01', '2026-09-01', 1, 1)",
    )
    .bind(other_tournament)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO rounds
           (id, tournament_id, round_number, name, round_date, course_name, tee_name,
            scoring_format)
         VALUES ($1, $2, 1, 'Other', '2026-09-01', '', '', 'individual_stroke_play')",
    )
    .bind(other_round)
    .bind(other_tournament)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::raw_sql(MIGRATION_16).execute(&pool).await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT mandatory_round_id FROM tournaments WHERE id = $1",
        )
        .bind(TOURNAMENT)
        .fetch_one(&pool)
        .await
        .unwrap(),
        None
    );

    let mut cross = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('app.tournament_configuration_tournament_id', $1::text, true),
                set_config('app.tournament_configuration_user_id', $2::text, true)",
    )
    .bind(TOURNAMENT)
    .bind(ADMIN)
    .execute(&mut *cross)
    .await
    .unwrap();
    sqlx::query("UPDATE tournaments SET mandatory_round_id = $2 WHERE id = $1")
        .bind(TOURNAMENT)
        .bind(other_round)
        .execute(&mut *cross)
        .await
        .unwrap();
    assert!(cross.commit().await.is_err());

    let mut same = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('app.tournament_configuration_tournament_id', $1::text, true),
                set_config('app.tournament_configuration_user_id', $2::text, true)",
    )
    .bind(TOURNAMENT)
    .bind(ADMIN)
    .execute(&mut *same)
    .await
    .unwrap();
    sqlx::query("UPDATE tournaments SET mandatory_round_id = $2 WHERE id = $1")
        .bind(TOURNAMENT)
        .bind(ROUND_TWO)
        .execute(&mut *same)
        .await
        .unwrap();
    same.commit().await.unwrap();

    assert!(
        sqlx::query("DELETE FROM rounds WHERE id = $1")
            .bind(ROUND_TWO)
            .execute(&pool)
            .await
            .is_err()
    );
    sqlx::query("DELETE FROM tournaments WHERE id = $1")
        .bind(TOURNAMENT)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM rounds WHERE tournament_id = $1")
            .bind(TOURNAMENT)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn patch_requires_explicit_nullable_target_and_sets_replaces_and_clears(pool: PgPool) {
    seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let expected = updated_at(&pool).await;

    let omitted = app
        .clone()
        .oneshot(patch(json!({
            "counted_rounds": 1,
            "expected_tournament_updated_at": expected
        })))
        .await
        .unwrap();
    assert_eq!(omitted.status(), StatusCode::BAD_REQUEST);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    let cross = app
        .clone()
        .oneshot(patch(json!({
            "counted_rounds": 1,
            "mandatory_round_id": Uuid::new_v4(),
            "expected_tournament_updated_at": expected
        })))
        .await
        .unwrap();
    assert_eq!(cross.status(), StatusCode::BAD_REQUEST);
    assert!(matches!(events.try_recv(), Err(TryRecvError::Empty)));

    let set = app
        .clone()
        .oneshot(patch(json!({
            "counted_rounds": 1,
            "mandatory_round_id": ROUND_ONE,
            "expected_tournament_updated_at": expected
        })))
        .await
        .unwrap();
    assert_eq!(set.status(), StatusCode::OK);
    assert_eq!(
        response_json(set).await["mandatory_round_id"],
        ROUND_ONE.to_string()
    );
    assert_eq!(events.try_recv().unwrap().resource, "tournament");

    let replace_expected = updated_at(&pool).await;
    let replace = app
        .clone()
        .oneshot(patch(json!({
            "counted_rounds": 1,
            "mandatory_round_id": ROUND_TWO,
            "expected_tournament_updated_at": replace_expected
        })))
        .await
        .unwrap();
    assert_eq!(replace.status(), StatusCode::OK);
    assert_eq!(events.try_recv().unwrap().id, TOURNAMENT);

    let clear = app
        .clone()
        .oneshot(patch(json!({
            "counted_rounds": 1,
            "mandatory_round_id": null,
            "expected_tournament_updated_at": updated_at(&pool).await
        })))
        .await
        .unwrap();
    assert_eq!(clear.status(), StatusCode::OK);
    assert_eq!(
        response_json(clear).await["mandatory_round_id"],
        Value::Null
    );
    assert_eq!(events.try_recv().unwrap().id, TOURNAMENT);

    let no_op_expected = updated_at(&pool).await;
    let no_op = app
        .oneshot(patch(json!({
            "counted_rounds": 1,
            "mandatory_round_id": null,
            "expected_tournament_updated_at": no_op_expected
        })))
        .await
        .unwrap();
    assert_eq!(no_op.status(), StatusCode::OK);
    assert_eq!(updated_at(&pool).await, no_op_expected);
    assert!(matches!(
        events.try_recv(),
        Err(TryRecvError::Empty | TryRecvError::Closed)
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn selection_and_replacement_serialize_with_target_round_deletion(pool: PgPool) {
    seed(&pool).await;
    let session_id =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM user_sessions WHERE token_hash = $1")
            .bind(hash_session_token(TOKEN).as_slice())
            .fetch_one(&pool)
            .await
            .unwrap();

    tournaments::update_counted_rounds_authorized(
        &pool,
        session_id,
        TOURNAMENT,
        1,
        Some(ROUND_ONE),
        updated_at(&pool).await,
    )
    .await
    .unwrap();
    let (replacement, deleted) = race_configuration_with_delete(&pool, session_id, ROUND_TWO).await;
    match (replacement, deleted) {
        (Ok(updated), false) => assert_eq!(updated.tournament.mandatory_round_id, Some(ROUND_TWO)),
        (Err(tournaments::TournamentMutationError::MandatoryRoundInvalid), true) => {
            assert_eq!(
                sqlx::query_scalar::<_, Option<Uuid>>(
                    "SELECT mandatory_round_id FROM tournaments WHERE id = $1",
                )
                .bind(TOURNAMENT)
                .fetch_one(&pool)
                .await
                .unwrap(),
                Some(ROUND_ONE)
            );
        }
        _ => panic!("replacement/delete race did not produce one valid winner"),
    }
    tournaments::update_counted_rounds_authorized(
        &pool,
        session_id,
        TOURNAMENT,
        1,
        None,
        updated_at(&pool).await,
    )
    .await
    .unwrap();

    let (selection, deleted) = race_configuration_with_delete(&pool, session_id, ROUND_ONE).await;
    match (selection, deleted) {
        (Ok(updated), false) => assert_eq!(updated.tournament.mandatory_round_id, Some(ROUND_ONE)),
        (Err(tournaments::TournamentMutationError::MandatoryRoundInvalid), true) => {
            assert_eq!(
                sqlx::query_scalar::<_, Option<Uuid>>(
                    "SELECT mandatory_round_id FROM tournaments WHERE id = $1",
                )
                .bind(TOURNAMENT)
                .fetch_one(&pool)
                .await
                .unwrap(),
                None
            );
        }
        _ => panic!("selection/delete race did not produce one valid winner"),
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn direct_mandatory_update_is_rejected_after_snapshot_freeze(pool: PgPool) {
    seed(&pool).await;
    let player_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO players (id, display_name, current_handicap_index)
         VALUES ($1, 'Snapshot player', 8.0)",
    )
    .bind(player_id)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournament_players (tournament_id, player_id, tournament_handicap)
         VALUES ($1, $2, 8.0)",
    )
    .bind(TOURNAMENT)
    .bind(player_id)
    .execute(&pool)
    .await
    .unwrap();
    let mut snapshot = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.round_opening_id', $1::text, true)")
        .bind(ROUND_ONE)
        .execute(&mut *snapshot)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO round_handicap_snapshots
           (round_id, tournament_id, player_id, handicap_index, course_handicap,
            playing_handicap)
         VALUES ($1, $2, $3, 8.0, 8, 8)",
    )
    .bind(ROUND_ONE)
    .bind(TOURNAMENT)
    .bind(player_id)
    .execute(&mut *snapshot)
    .await
    .unwrap();
    snapshot.commit().await.unwrap();

    let mut direct = pool.begin().await.unwrap();
    sqlx::query(
        "SELECT set_config('app.tournament_configuration_tournament_id', $1::text, true),
                set_config('app.tournament_configuration_user_id', $2::text, true)",
    )
    .bind(TOURNAMENT)
    .bind(ADMIN)
    .execute(&mut *direct)
    .await
    .unwrap();
    let denied = sqlx::query("UPDATE tournaments SET mandatory_round_id = $2 WHERE id = $1")
        .bind(TOURNAMENT)
        .bind(ROUND_TWO)
        .execute(&mut *direct)
        .await
        .unwrap_err();
    assert_eq!(
        denied
            .as_database_error()
            .and_then(|error| error.constraint()),
        Some("tournament_configuration_locked")
    );
}
