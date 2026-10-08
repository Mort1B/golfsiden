#![cfg(feature = "database-tests")]
mod player_claims_support;
use golf_api::repositories::player_claims::{self, ClaimError};
use player_claims_support::*;

#[sqlx::test(migrations = "../migrations")]
async fn create_claim_is_atomic_preserves_identity_and_single_use(pool: PgPool) {
    let fixture = seed(&pool).await;
    let (grant, token) = prepared(&pool, &fixture).await;
    let before = history(&pool, grant.player_id).await;
    assert_eq!(before["global"].as_array().unwrap().len(), 1);
    assert_eq!(before["tournament"].as_array().unwrap().len(), 1);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users WHERE player_id=$1")
            .bind(grant.player_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let preview = player_claims::preview(&pool, grant.claim_id, &token)
        .await
        .unwrap();
    assert_eq!(preview.player.display_name, "Prepared Player");
    let (first, second) = tokio::join!(
        claim(&pool, grant.claim_id, &token, "first_claim"),
        claim(&pool, grant.claim_id, &token, "second_claim")
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert!(matches!(
        first.as_ref().err().or(second.as_ref().err()),
        Some(ClaimError::Unavailable)
    ));
    let user = first.or(second).unwrap();
    assert_eq!(user.player_id, grant.player_id);
    assert_eq!(history(&pool, grant.player_id).await, before);
    assert!(matches!(
        player_claims::preview(&pool, grant.claim_id, &token).await,
        Err(ClaimError::Unavailable)
    ));
    assert!(matches!(
        player_claims::reissue(
            &pool,
            fixture.session,
            fixture.tournament,
            grant.player_id,
            &[3; 32]
        )
        .await,
        Err(ClaimError::HasAccount)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tournament_memberships WHERE user_id=$1 AND role='player'"
        )
        .bind(user.session.user_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn revoke_reissue_expiry_and_conflict_leave_no_account(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, t) = prepared(&pool, &f).await;
    assert!(matches!(
        claim(&pool, g.claim_id, &t, "claims_admin").await,
        Err(ClaimError::Username)
    ));
    player_claims::preview(&pool, g.claim_id, &t).await.unwrap();
    player_claims::revoke(&pool, f.session, f.tournament, g.player_id)
        .await
        .unwrap();
    assert!(matches!(
        claim(&pool, g.claim_id, &t, "claim_one").await,
        Err(ClaimError::Unavailable)
    ));
    let token = generate_invitation_token().unwrap();
    let new = player_claims::reissue(
        &pool,
        f.session,
        f.tournament,
        g.player_id,
        &hash_invitation_token(&token),
    )
    .await
    .unwrap();
    assert!(matches!(
        player_claims::preview(&pool, g.claim_id, &t).await,
        Err(ClaimError::Unavailable)
    ));
    sqlx::query("UPDATE player_claim_grants SET expires_at=clock_timestamp()-interval '1 second' WHERE id=$1").bind(new.claim_id).execute(&pool).await.unwrap();
    assert!(matches!(
        claim(&pool, new.claim_id, &token, "claim_two").await,
        Err(ClaimError::Unavailable)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM users WHERE player_id=$1")
            .bind(g.player_id)
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn public_api_oracle_signed_in_validation_no_store_and_throttle(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, t) = prepared(&pool, &f).await;
    let app = api::router(AppState::new(pool.clone()));
    let path = format!("/api/player-claims/{}/register", g.claim_id);
    let wrong = app
        .clone()
        .oneshot(request(
            "POST",
            &path,
            None,
            json!({"token":generate_invitation_token().unwrap(),"unexpected":true}),
        ))
        .await
        .unwrap();
    assert_eq!(wrong.status(), StatusCode::NOT_FOUND);
    assert_eq!(wrong.headers()["cache-control"], "no-store");
    assert_eq!(body(wrong).await["error"]["code"], "claim_invalid");
    let payload =
        json!({"token":t,"account":{"username":"claim_user","password":"long password valid"}});
    let signed = app
        .clone()
        .oneshot(request("POST", &path, Some(ADMIN_TOKEN), payload.clone()))
        .await
        .unwrap();
    assert_eq!(signed.status(), StatusCode::CONFLICT);
    assert_eq!(body(signed).await["error"]["code"], "already_authenticated");
    let bad = app
        .clone()
        .oneshot(request(
            "POST",
            &path,
            None,
            json!({"token":t,"account":{"username":"bad.username","password":"short"}}),
        ))
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::BAD_REQUEST);
    let ok = app
        .oneshot(request("POST", &path, None, payload))
        .await
        .unwrap();
    assert_eq!(ok.status(), StatusCode::CREATED);
    assert!(ok.headers().contains_key("set-cookie"));
    assert_eq!(body(ok).await["player_id"], g.player_id.to_string());
    let state = AppState::with_runtime_services(
        pool.clone(),
        golf_api::auth::AuthConfig::local(),
        golf_api::course_provider::CourseProviderClient::disabled(),
        golf_api::rate_limit::RateLimiter::with_rules(
            [(
                golf_api::rate_limit::RateLimitRoute::PlayerClaimPreview,
                std::time::Duration::from_secs(60),
                1,
                5,
            )],
            10,
        ),
    );
    let app = api::router(state);
    let first = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/api/player-claims/{}/preview", g.claim_id),
            None,
            json!({"token":t}),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::GONE);
    let second = app
        .oneshot(request(
            "POST",
            &format!("/api/player-claims/{}/preview", g.claim_id.simple()),
            None,
            json!({"token":t}),
        ))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(second.headers()["cache-control"], "no-store");
    assert!(second.headers().contains_key("retry-after"));
}
#[sqlx::test(migrations = "../migrations")]
async fn admin_scope_withdrawal_invalidates_and_preserves_history(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, t) = prepared(&pool, &f).await;
    let other = auth::create_session(
        &pool,
        f.other,
        &hash_session_token("other"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    assert!(
        player_claims::create(&pool, other.session_id, f.tournament, "No", 1.0, &[1; 32])
            .await
            .is_err()
    );
    assert!(
        player_claims::list(&pool, other.session_id, f.tournament)
            .await
            .is_err()
    );
    assert!(
        player_claims::withdraw(&pool, other.session_id, f.tournament, g.player_id)
            .await
            .is_err()
    );
    let before = history(&pool, g.player_id).await;
    player_claims::withdraw(&pool, f.session, f.tournament, g.player_id)
        .await
        .unwrap();
    assert_eq!(history(&pool, g.player_id).await, before);
    assert!(matches!(
        claim(&pool, g.claim_id, &t, "withdrawn").await,
        Err(ClaimError::Unavailable)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT status::text FROM tournament_players WHERE player_id=$1"
        )
        .bind(g.player_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "withdrawn"
    );
    assert!(
        sqlx::query("DELETE FROM tournament_player_withdrawals WHERE player_id=$1")
            .bind(g.player_id)
            .execute(&pool)
            .await
            .is_err()
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn claimed_player_withdrawal_keeps_account_membership_and_blocks_admin(pool: PgPool) {
    let f = seed(&pool).await;
    let (g, t) = prepared(&pool, &f).await;
    let user = claim(&pool, g.claim_id, &t, "retained").await.unwrap();
    sqlx::query("UPDATE tournament_memberships SET role='admin' WHERE user_id=$1")
        .bind(user.session.user_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        player_claims::withdraw(&pool, f.session, f.tournament, g.player_id).await,
        Err(ClaimError::Admin)
    ));
    sqlx::query("UPDATE tournament_memberships SET role='scorer' WHERE user_id=$1")
        .bind(user.session.user_id)
        .execute(&pool)
        .await
        .unwrap();
    player_claims::withdraw(&pool, f.session, f.tournament, g.player_id)
        .await
        .unwrap();
    let mut read = pool.begin().await.unwrap();
    golf_api::repositories::tournament_authorization::require_tournament_member_read(
        &mut read,
        user.session.user_id,
        f.tournament,
    )
    .await
    .unwrap();
    read.commit().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>(
            "SELECT role::text FROM tournament_memberships WHERE user_id=$1"
        )
        .bind(user.session.user_id)
        .fetch_one(&pool)
        .await
        .unwrap(),
        "scorer"
    );
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM users u JOIN tournament_memberships m ON m.user_id=u.id WHERE u.player_id=$1").bind(g.player_id).fetch_one(&pool).await.unwrap(),1);
}

#[sqlx::test(migrations = "../migrations")]
async fn admin_http_requires_csrf_exact_authority_and_returns_contract(pool: PgPool) {
    let f = seed(&pool).await;
    let app = api::router(AppState::new(pool.clone()));
    let path = format!("/api/tournaments/{}/players", f.tournament);
    let value = json!({"display_name":"Prepared via API","handicap_index":5.5});
    let denied = app
        .clone()
        .oneshot(request("POST", &path, None, value.clone()))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(denied.headers()["cache-control"], "no-store");
    let mut no_csrf = request("POST", &path, Some(ADMIN_TOKEN), value.clone());
    no_csrf.headers_mut().remove("x-csrf-token");
    let denied = app.clone().oneshot(no_csrf).await.unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(denied.headers()["cache-control"], "no-store");
    let created = app
        .clone()
        .oneshot(request("POST", &path, Some(ADMIN_TOKEN), value))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let created = body(created).await;
    assert!(created["token"].as_str().is_some());
    assert!(created["claim_id"].as_str().is_some());
    let accounts = app
        .oneshot(request(
            "GET",
            &format!("/api/tournaments/{}/player-accounts", f.tournament),
            Some(ADMIN_TOKEN),
            json!(null),
        ))
        .await
        .unwrap();
    assert_eq!(accounts.status(), StatusCode::OK);
    let accounts = body(accounts).await;
    assert_eq!(accounts[0]["has_account"], false);
    assert_eq!(accounts[0]["player_id"], created["player_id"]);
    assert!(accounts[0].get("token").is_none());
}
