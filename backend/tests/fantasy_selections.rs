#![cfg(feature = "database-tests")]
mod fantasy_support;
use fantasy_support::*;
#[sqlx::test(migrations = "../migrations")]
async fn manual_receipts_privacy_and_atomic_constraints(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let input = lineup(&f);
    let receipt = fantasy::save(&pool, f.member_session, f.base.tournament, r, &input)
        .await
        .unwrap();
    assert_eq!(receipt.revision, 1);
    assert!(
        read(&pool, &f, r).await.selections.is_empty(),
        "administrator cannot inspect another manager before lock"
    );
    assert_eq!(
        fantasy::save(&pool, f.member_session, f.base.tournament, r, &input)
            .await
            .unwrap()
            .id,
        receipt.id
    );
    let mut bad = input.clone();
    bad.captain = f.players[0];
    assert!(matches!(
        fantasy::save(&pool, f.member_session, f.base.tournament, r, &bad).await,
        Err(Error::Conflict)
    ));
    bad.request_id = Uuid::new_v4();
    bad.picks[0] = bad.picks[1];
    assert!(matches!(
        fantasy::save(&pool, f.member_session, f.base.tournament, r, &bad).await,
        Err(Error::Invalid)
    ));
    close(&pool, &f, r).await;
    let view = read(&pool, &f, r).await;
    assert_eq!(
        view.selections
            .iter()
            .find(|s| s.user_id == f.base.other)
            .unwrap()
            .receipt
            .as_ref()
            .unwrap()
            .id,
        receipt.id
    );
    assert_eq!(
        fantasy::save(&pool, f.member_session, f.base.tournament, r, &input)
            .await
            .unwrap()
            .id,
        receipt.id
    );
    let mut late = input.clone();
    late.request_id = Uuid::new_v4();
    late.expected_revision = 1;
    assert!(matches!(
        fantasy::save(&pool, f.member_session, f.base.tournament, r, &late).await,
        Err(Error::Closed)
    ));
    assert!(
        sqlx::query("UPDATE fantasy_lineups SET captain=first_player WHERE id=$1")
            .bind(receipt.id)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM fantasy_selections WHERE round_id=$1")
            .bind(r)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        fantasy::set_deadline(&pool, f.base.session, f.base.tournament, r, None)
            .await
            .is_err()
    );
    assert!(
        fantasy::configure(&pool, f.base.session, f.base.tournament, false)
            .await
            .is_err()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn lazy_carry_uses_actual_deadline_before_withdrawal_and_preserves_captain(pool: PgPool) {
    let f = fixture(&pool).await;
    let input = lineup(&f);
    fantasy::save(
        &pool,
        f.base.session,
        f.base.tournament,
        f.rounds[0],
        &input,
    )
    .await
    .unwrap();
    close(&pool, &f, f.rounds[0]).await;
    close(&pool, &f, f.rounds[1]).await;
    player_claims::withdraw(&pool, f.base.session, f.base.tournament, f.players[1])
        .await
        .unwrap();
    let view = read(&pool, &f, f.rounds[1]).await;
    let s = view
        .selections
        .iter()
        .find(|s| s.user_id == f.base.admin)
        .unwrap();
    let receipt = s.receipt.as_ref().unwrap();
    assert_eq!(receipt.origin, "carried_forward");
    assert_eq!(receipt.source_round, Some(f.rounds[0]));
    assert_eq!(receipt.captain, input.captain);
    close(&pool, &f, f.rounds[2]).await;
    let view = read(&pool, &f, f.rounds[2]).await;
    assert_eq!(
        view.selections
            .iter()
            .find(|s| s.user_id == f.base.admin)
            .unwrap()
            .state,
        "invalid"
    );
    assert_eq!(
        read(&pool, &f, f.rounds[1])
            .await
            .selections
            .iter()
            .find(|s| s.user_id == f.base.admin)
            .unwrap()
            .receipt
            .as_ref()
            .unwrap()
            .id,
        receipt.id
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn invalid_current_falls_back_and_current_valid_wins(pool: PgPool) {
    let f = fixture(&pool).await;
    let first = lineup(&f);
    fantasy::save(
        &pool,
        f.base.session,
        f.base.tournament,
        f.rounds[0],
        &first,
    )
    .await
    .unwrap();
    close(&pool, &f, f.rounds[0]).await;
    let mut next = lineup(&f);
    next.picks[3] = f.players[4];
    next.captain = f.players[4];
    fantasy::save(&pool, f.base.session, f.base.tournament, f.rounds[1], &next)
        .await
        .unwrap();
    sqlx::query("UPDATE players SET active=false WHERE id=$1")
        .bind(f.players[4])
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        read(&pool, &f, f.rounds[1]).await.selections[0].state,
        "invalid_draft"
    );
    close(&pool, &f, f.rounds[1]).await;
    let receipt = read(&pool, &f, f.rounds[1])
        .await
        .selections
        .into_iter()
        .find(|s| s.user_id == f.base.admin)
        .unwrap()
        .receipt
        .unwrap();
    assert_eq!(receipt.picks, first.picks);
    assert_eq!(receipt.origin, "carried_forward");
    let mut third = lineup(&f);
    third.captain = f.players[2];
    fantasy::save(
        &pool,
        f.base.session,
        f.base.tournament,
        f.rounds[2],
        &third,
    )
    .await
    .unwrap();
    close(&pool, &f, f.rounds[2]).await;
    let receipt = read(&pool, &f, f.rounds[2])
        .await
        .selections
        .into_iter()
        .find(|s| s.user_id == f.base.admin)
        .unwrap()
        .receipt
        .unwrap();
    assert_eq!(receipt.captain, third.captain);
    assert_eq!(receipt.origin, "submitted");
}
#[sqlx::test(migrations = "../migrations")]
async fn membership_expiry_late_entries_and_nonchronological_round_closure(pool: PgPool) {
    let f = fixture(&pool).await;
    let input = lineup(&f);
    fantasy::save(
        &pool,
        f.member_session,
        f.base.tournament,
        f.rounds[0],
        &input,
    )
    .await
    .unwrap();
    close(&pool, &f, f.rounds[1]).await;
    close(&pool, &f, f.rounds[0]).await;
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
        .bind(f.base.tournament)
        .bind(f.base.other)
        .execute(&pool)
        .await
        .unwrap();
    let v = read(&pool, &f, f.rounds[1]).await;
    assert_eq!(
        v.selections
            .iter()
            .find(|s| s.user_id == f.base.other)
            .unwrap()
            .state,
        "missed"
    );
    let v = read(&pool, &f, f.rounds[0]).await;
    assert_eq!(
        v.selections
            .iter()
            .find(|s| s.user_id == f.base.other)
            .unwrap()
            .state,
        "locked"
    );
    assert!(
        fantasy::read_round(&pool, f.member_session, f.base.tournament, f.rounds[0])
            .await
            .is_err()
    );
    close(&pool, &f, f.rounds[2]).await;
    let v = read(&pool, &f, f.rounds[2]).await;
    assert!(!v.selections.iter().any(|s| s.user_id == f.base.other));
}
#[sqlx::test(migrations = "../migrations")]
async fn api_admin_member_csrf_validation_and_no_store(pool: PgPool) {
    let f = fixture(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    let url = format!("/api/tournaments/{}/fantasy", f.base.tournament);
    let response = app
        .clone()
        .oneshot(request(
            "PUT",
            &url,
            Some("fantasy-member"),
            json!({"enabled":false}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let response = app
        .clone()
        .oneshot(request("GET", &url, None, json!(null)))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let mut req = request("PUT", &url, Some(ADMIN_TOKEN), json!({"enabled":false}));
    req.headers_mut().remove("x-csrf-token");
    assert_eq!(
        app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::FORBIDDEN
    );
    let url = format!(
        "/api/tournaments/{}/fantasy/rounds/{}/lineup",
        f.base.tournament, f.rounds[0]
    );
    let input = lineup(&f);
    let response=app.oneshot(request("PUT",&url,Some(ADMIN_TOKEN),json!({"request_id":input.request_id,"expected_revision":0,"picks":input.picks,"captain":input.captain}))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await["revision"], 1);
}
#[sqlx::test(migrations = "../migrations")]
async fn late_entry_has_no_backfill_and_claim_keeps_selected_identity(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let input = lineup(&f);
    fantasy::save(&pool, f.base.session, f.base.tournament, r, &input)
        .await
        .unwrap();
    let token = generate_invitation_token().unwrap();
    let grant = player_claims::reissue(
        &pool,
        f.base.session,
        f.base.tournament,
        f.players[0],
        &hash_invitation_token(&token),
    )
    .await
    .unwrap();
    close(&pool, &f, r).await;
    let claimed = claim(&pool, grant.claim_id, &token, "fantasy_claimed")
        .await
        .unwrap();
    fantasy::enter(&pool, claimed.session.session_id, f.base.tournament)
        .await
        .unwrap();
    sqlx::query("UPDATE players SET active=false WHERE id=$1")
        .bind(f.players[1])
        .execute(&pool)
        .await
        .unwrap();
    let view = read(&pool, &f, r).await;
    assert!(
        !view
            .selections
            .iter()
            .any(|s| s.user_id == claimed.session.user_id)
    );
    let receipt = view
        .selections
        .iter()
        .find(|s| s.user_id == f.base.admin)
        .unwrap()
        .receipt
        .as_ref()
        .unwrap();
    assert_eq!(receipt.picks, input.picks);
    assert_eq!(claimed.player_id, f.players[0]);
    close(&pool, &f, f.rounds[1]).await;
    let view = read(&pool, &f, f.rounds[1]).await;
    assert_eq!(
        view.selections
            .iter()
            .find(|s| s.user_id == claimed.session.user_id)
            .unwrap()
            .state,
        "missed"
    );
}
