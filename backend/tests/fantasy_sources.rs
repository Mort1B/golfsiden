#![cfg(feature = "database-tests")]
mod fantasy_support;
use fantasy_support::*;
use golf_api::{domain::scorecards::ScoreOwner, repositories::scorecards};
async fn token(pool: &PgPool, f: &Fantasy, r: Uuid, p: Uuid) -> fantasy::Source {
    fantasy::source(
        pool,
        f.base.session,
        f.base.tournament,
        r,
        OwnerKind::Player,
        p,
    )
    .await
    .unwrap()
}
async fn score(pool: &PgPool, f: &Fantasy, r: Uuid, p: Uuid, h: Uuid, strokes: i16) {
    scorecards::save_authenticated(
        pool,
        scorecards::AuthenticatedSaveScore {
            round_id: r,
            hole_id: h,
            owner: ScoreOwner::Player { id: p },
            gross_strokes: strokes,
            session_id: f.base.session,
        },
    )
    .await
    .unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn empty_disposition_score_correction_confirmation_and_admin_privacy(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    source_fixture(&pool, &f, r).await;
    let p = f.players[0];
    let initial = token(&pool, &f, r, p).await;
    assert!(
        fantasy::source(
            &pool,
            f.member_session,
            f.base.tournament,
            r,
            OwnerKind::Player,
            p
        )
        .await
        .is_err()
    );
    let mut input = Dispose {
        expected_source_token: initial.source_token.clone(),
        disposed: true,
        correction: false,
        reason: "Stopped playing".into(),
    };
    let result = fantasy::dispose(
        &pool,
        f.base.session,
        f.base.tournament,
        r,
        OwnerKind::Player,
        p,
        &input,
    )
    .await
    .unwrap();
    assert!(result.disposition_current);
    let holes:Vec<Uuid>=sqlx::query_scalar("SELECT h.id FROM holes h JOIN rounds r ON r.tee_id=h.tee_id WHERE r.id=$1 ORDER BY hole_number").bind(r).fetch_all(&pool).await.unwrap();
    score(&pool, &f, r, p, holes[0], 5).await;
    let after = token(&pool, &f, r, p).await;
    assert_ne!(initial.source_token, after.source_token);
    assert!(!after.disposition_current);
    assert!(matches!(
        fantasy::dispose(
            &pool,
            f.base.session,
            f.base.tournament,
            r,
            OwnerKind::Player,
            p,
            &input
        )
        .await,
        Err(Error::Conflict)
    ));
    input.expected_source_token = after.source_token;
    input.correction = true;
    assert!(
        fantasy::dispose(
            &pool,
            f.base.session,
            f.base.tournament,
            r,
            OwnerKind::Player,
            p,
            &input
        )
        .await
        .unwrap()
        .disposition_current
    );
    for h in &holes {
        score(&pool, &f, r, p, *h, 4).await;
    }
    let before_confirm = token(&pool, &f, r, p).await;
    scorecards::confirm_authenticated(&pool, r, ScoreOwner::Player { id: p }, f.base.session)
        .await
        .unwrap();
    let after_confirm = token(&pool, &f, r, p).await;
    assert_ne!(before_confirm.source_token, after_confirm.source_token);
    input.expected_source_token = after_confirm.source_token;
    assert!(
        fantasy::dispose(
            &pool,
            f.base.session,
            f.base.tournament,
            r,
            OwnerKind::Player,
            p,
            &input
        )
        .await
        .is_err(),
        "cannot override accepted completed result"
    );
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM fantasy_dispositions WHERE round_id=$1")
            .bind(r)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 2);
    assert!(
        sqlx::query("UPDATE fantasy_owner_generations SET generation=1 WHERE round_id=$1")
            .bind(r)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM fantasy_dispositions WHERE round_id=$1")
            .bind(r)
            .execute(&pool)
            .await
            .is_err()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn source_generation_retains_deletion_and_locked_disposition_requires_correction(
    pool: PgPool,
) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    source_fixture(&pool, &f, r).await;
    let p = f.players[0];
    let initial = token(&pool, &f, r, p).await.source_token;
    let h:Uuid=sqlx::query_scalar("SELECT h.id FROM holes h JOIN rounds r ON r.tee_id=h.tee_id WHERE r.id=$1 ORDER BY hole_number LIMIT 1").bind(r).fetch_one(&pool).await.unwrap();
    score(&pool, &f, r, p, h, 4).await;
    // Owner-only deletion fixture: ordinary sporting deletion stays prohibited.
    // Leave the new generation trigger enabled to exercise retained deletion evidence.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("ALTER TABLE scores DISABLE TRIGGER scores_validate_mutation")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM scores WHERE round_id=$1")
        .bind(r)
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE scores ENABLE TRIGGER scores_validate_mutation")
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let after = token(&pool, &f, r, p).await.source_token;
    assert_ne!(initial, after);
    historical_status(&pool, r, p, "locked").await;
    let mut input = Dispose {
        expected_source_token: token(&pool, &f, r, p).await.source_token,
        disposed: true,
        correction: false,
        reason: "Historical non-finish".into(),
    };
    assert!(matches!(
        fantasy::dispose(
            &pool,
            f.base.session,
            f.base.tournament,
            r,
            OwnerKind::Player,
            p,
            &input
        )
        .await,
        Err(Error::Conflict)
    ));
    input.correction = true;
    fantasy::dispose(
        &pool,
        f.base.session,
        f.base.tournament,
        r,
        OwnerKind::Player,
        p,
        &input,
    )
    .await
    .unwrap();
}
