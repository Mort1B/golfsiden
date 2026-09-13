use super::support::*;
use axum::http::StatusCode;
use golf_api::{
    AppState, api, domain::result_sharing::ResultShareToken, repositories::result_sharing,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
#[sqlx::test(migrations = "../migrations")]
async fn management_exact_authority_csrf_staleness_rotation_and_safe_metadata(pool: PgPool) {
    let session = fixture(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let path = format!("/api/tournaments/{TRIP}/result-share");
    assert_eq!(
        response(&app, request("GET", &path, Some(ADMIN_TOKEN), None, false))
            .await
            .1,
        json!({"tournament_id":TRIP,"grant":null})
    );
    for (token, csrf, status) in [
        (None, false, StatusCode::UNAUTHORIZED),
        (Some(USER_TOKEN), true, StatusCode::FORBIDDEN),
        (Some(ADMIN_TOKEN), false, StatusCode::FORBIDDEN),
    ] {
        assert_eq!(
            response(
                &app,
                request(
                    "POST",
                    &path,
                    token,
                    Some(json!({"expected_grant_id":null})),
                    csrf
                )
            )
            .await
            .0,
            status
        );
    }
    assert_eq!(
        response(
            &app,
            request("POST", &path, Some(ADMIN_TOKEN), Some(json!({})), true)
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    assert!(events.try_recv().is_err());
    let (status, created) = response(
        &app,
        request(
            "POST",
            &path,
            Some(ADMIN_TOKEN),
            Some(json!({"expected_grant_id":null})),
            true,
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(events.try_recv().unwrap().resource, "tournament");
    let id: Uuid = serde_json::from_value(created["grant"]["id"].clone()).unwrap();
    let token = ResultShareToken::parse(created["token"].as_str().unwrap().into()).unwrap();
    let stored: Vec<u8> =
        sqlx::query_scalar("SELECT token_hash FROM tournament_result_shares WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(stored.len(), 32);
    assert!(token.matches(&stored));
    let lifetime:i64=sqlx::query_scalar("SELECT extract(epoch FROM expires_at-created_at)::bigint FROM tournament_result_shares WHERE id=$1").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(lifetime, 30 * 86400);
    let metadata = response(&app, request("GET", &path, Some(ADMIN_TOKEN), None, false))
        .await
        .1;
    assert!(metadata.get("token").is_none());
    assert_eq!(metadata["grant"].as_object().unwrap().len(), 4);
    assert_eq!(
        response(
            &app,
            request(
                "POST",
                &path,
                Some(ADMIN_TOKEN),
                Some(json!({"expected_grant_id":null})),
                true
            )
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let replacement = response(
        &app,
        request(
            "POST",
            &path,
            Some(ADMIN_TOKEN),
            Some(json!({"expected_grant_id":id})),
            true,
        ),
    )
    .await
    .1;
    assert_eq!(events.try_recv().unwrap().resource, "tournament");
    assert_eq!(
        response(&app, public_request(id, &token, "gross", None))
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        response(
            &app,
            request(
                "DELETE",
                &format!("{path}/{id}"),
                Some(ADMIN_TOKEN),
                None,
                true
            )
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let new_id: Uuid = serde_json::from_value(replacement["grant"]["id"].clone()).unwrap();
    for _ in 0..2 {
        assert_eq!(
            response(
                &app,
                request(
                    "DELETE",
                    &format!("{path}/{new_id}"),
                    Some(ADMIN_TOKEN),
                    None,
                    true
                )
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );
    }
    assert_eq!(events.try_recv().unwrap().resource, "tournament");
    assert!(events.try_recv().is_err());
    assert!(
        result_sharing::status(&pool, session, TRIP)
            .await
            .unwrap()
            .unwrap()
            .revoked_at
            .is_some()
    );
    let audits: Vec<String> =
        sqlx::query_scalar("SELECT action FROM tournament_result_share_audits ORDER BY id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(audits, vec!["issued", "replaced", "issued", "revoked"]);
}
#[sqlx::test(migrations = "../migrations")]
async fn public_invalid_expired_revoked_unknown_share_uniform_unavailable_and_rate_limit(
    pool: PgPool,
) {
    let session = fixture(&pool).await;
    let (id, token) = expiring(&pool, session, -1000).await;
    let state = AppState::new(pool.clone());
    let app = api::router(state);
    let expired = response(&app, public_request(id, &token, "gross", None)).await;
    assert_eq!(
        expired,
        (
            StatusCode::NOT_FOUND,
            json!({"error":{"code":"result_share_unavailable","message":"result link is unavailable"}})
        )
    );
    assert_eq!(
        response(&app, public_request(Uuid::new_v4(), &token, "gross", None)).await,
        expired
    );
    result_sharing::revoke(&pool, session, TRIP, id)
        .await
        .unwrap();
    assert_eq!(
        response(&app, public_request(id, &token, "gross", None)).await,
        expired
    );
    let (active, secret) = issue(&pool, session, Some(id)).await;
    assert_eq!(
        response(&app, public_request(active.id, &token, "gross", None)).await,
        expired
    );
    assert_eq!(
        response(
            &app,
            request(
                "POST",
                "/api/public/results/not-a-uuid",
                None,
                Some(json!({"token":"bad","metric":"gross"})),
                false
            )
        )
        .await,
        expired
    );
    assert_eq!(
        response(
            &app,
            public_request(active.id, &secret, "bogus", Some(ADMIN_TOKEN))
        )
        .await
        .0,
        StatusCode::BAD_REQUEST
    );
    let mut state = AppState::new(pool.clone());
    std::sync::Arc::get_mut(&mut state).unwrap().rate_limiter =
        golf_api::rate_limit::RateLimiter::with_rules(
            [(
                golf_api::rate_limit::RateLimitRoute::PublicResults,
                std::time::Duration::from_secs(60),
                2,
                4,
            )],
            32,
        );
    let app = api::router(state);
    for _ in 0..2 {
        assert_eq!(
            response(&app, public_request(active.id, &secret, "gross", None))
                .await
                .0,
            StatusCode::OK
        );
    }
    assert_eq!(
        response(&app, public_request(active.id, &secret, "net", None))
            .await
            .0,
        StatusCode::TOO_MANY_REQUESTS
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn tournament_owns_grant_after_issuer_demotion_and_replacement_admin_can_revoke(
    pool: PgPool,
) {
    let session = fixture(&pool).await;
    let (grant, token) = issue(&pool, session, None).await;
    sqlx::query(
        "UPDATE tournament_memberships SET role='player' WHERE tournament_id=$1 AND user_id=$2",
    )
    .bind(TRIP)
    .bind(ADMIN)
    .execute(&pool)
    .await
    .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    assert_eq!(
        response(
            &app,
            public_request(grant.id, &token, "gross", Some(ADMIN_TOKEN))
        )
        .await
        .0,
        StatusCode::OK
    );
    assert!(
        result_sharing::revoke(&pool, session, TRIP, grant.id)
            .await
            .is_err()
    );
    sqlx::query(
        "UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2",
    )
    .bind(TRIP)
    .bind(USER)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(
        response(
            &app,
            request(
                "DELETE",
                &format!("/api/tournaments/{TRIP}/result-share/{}", grant.id),
                Some(USER_TOKEN),
                None,
                true
            )
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
}
