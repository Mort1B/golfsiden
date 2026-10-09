#![cfg(feature = "database-tests")]
mod fantasy_support;
use fantasy_support::*;

#[sqlx::test(migrations = "../migrations")]
async fn competing_sessions_distinguish_unaccepted_revision_from_accepted_replay(pool: PgPool) {
    let f = fixture(&pool).await;
    let second = auth::create_session(
        &pool,
        f.base.other,
        &hash_session_token("fantasy-second"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    let original = lineup(&f);
    let competing = lineup(&f);
    let accepted = fantasy::save(
        &pool,
        second.session_id,
        f.base.tournament,
        f.rounds[0],
        &competing,
    )
    .await
    .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    let url = format!(
        "/api/tournaments/{}/fantasy/rounds/{}/lineup",
        f.base.tournament, f.rounds[0]
    );
    for _ in 0..2 {
        let response = app.clone().oneshot(request("PUT", &url, Some("fantasy-member"), json!({"request_id":original.request_id,"expected_revision":original.expected_revision,"picks":original.picks,"captain":original.captain}))).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(
            body(response).await["error"]["code"],
            "fantasy_revision_conflict"
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_lineups WHERE request_id=$1")
            .bind(original.request_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let mut newer = lineup(&f);
    newer.expected_revision = 1;
    let latest = fantasy::save(
        &pool,
        f.member_session,
        f.base.tournament,
        f.rounds[0],
        &newer,
    )
    .await
    .unwrap();
    assert_eq!(latest.revision, 2);
    let replay = fantasy::save(
        &pool,
        f.member_session,
        f.base.tournament,
        f.rounds[0],
        &competing,
    )
    .await
    .unwrap();
    assert_eq!(replay.id, accepted.id);
    let current = fantasy::read_round(&pool, f.member_session, f.base.tournament, f.rounds[0])
        .await
        .unwrap();
    assert_eq!(
        current.selections[0].receipt.as_ref().unwrap().id,
        latest.id
    );
    let mut collision = competing.clone();
    collision.captain = f.players[0];
    assert!(matches!(
        fantasy::save(
            &pool,
            f.member_session,
            f.base.tournament,
            f.rounds[0],
            &collision
        )
        .await,
        Err(Error::Conflict)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_lineups")
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn transient_database_serialization_is_not_definitive_revision_rejection(pool: PgPool) {
    let error = sqlx::query("DO $$ BEGIN RAISE EXCEPTION 'simulated serialization failure' USING ERRCODE='40001'; END $$").execute(&pool).await.unwrap_err();
    assert!(matches!(Error::from(error), Error::Conflict));
}

#[sqlx::test(migrations = "../migrations")]
async fn future_revision_conflict_is_not_a_permanent_original_request_outcome(pool: PgPool) {
    let f = fixture(&pool).await;
    let mut future = lineup(&f);
    future.expected_revision = 1;
    assert!(matches!(
        fantasy::save(
            &pool,
            f.member_session,
            f.base.tournament,
            f.rounds[0],
            &future
        )
        .await,
        Err(Error::Conflict)
    ));
    fantasy::save(
        &pool,
        f.member_session,
        f.base.tournament,
        f.rounds[0],
        &lineup(&f),
    )
    .await
    .unwrap();
    let accepted = fantasy::save(
        &pool,
        f.member_session,
        f.base.tournament,
        f.rounds[0],
        &future,
    )
    .await
    .unwrap();
    assert_eq!(accepted.request_id, Some(future.request_id));
    assert_eq!(accepted.revision, 2);
}

#[sqlx::test(migrations = "../migrations")]
async fn superseded_unaccepted_request_is_definitive_even_when_original_pick_is_withdrawn(
    pool: PgPool,
) {
    let f = fixture(&pool).await;
    let original = lineup(&f);
    player_claims::withdraw(&pool, f.base.session, f.base.tournament, f.players[0])
        .await
        .unwrap();
    let mut competing = lineup(&f);
    competing.picks[0] = f.players[4];
    fantasy::save(
        &pool,
        f.member_session,
        f.base.tournament,
        f.rounds[0],
        &competing,
    )
    .await
    .unwrap();
    assert!(matches!(
        fantasy::save(
            &pool,
            f.member_session,
            f.base.tournament,
            f.rounds[0],
            &original
        )
        .await,
        Err(Error::RevisionConflict)
    ));
}
