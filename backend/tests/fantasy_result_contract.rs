#![cfg(feature = "database-tests")]
mod fantasy_results_support;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use fantasy_results_support::*;
use sha2::{Digest, Sha256};
#[sqlx::test(migrations = "../migrations")]
async fn malformed_paths_are_json_no_store_without_materialization(pool: PgPool) {
    let f = fixture(&pool).await;
    close(&pool, &f, f.rounds[0]).await;
    let app = api::router(AppState::new(pool.clone()));
    let t = f.base.tournament;
    for path in [
        "/api/tournaments/bad/fantasy/results".to_string(),
        format!("/api/tournaments/{t}/fantasy/rounds/bad/results"),
        format!("/api/tournaments/{t}/fantasy/results/golfers/bad"),
        format!("/api/tournaments/{t}/fantasy/results/managers/bad"),
    ] {
        let response = app
            .clone()
            .oneshot(request("GET", &path, Some(ADMIN_TOKEN), json!(null)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["content-type"], "application/json");
        let body: Value = serde_json::from_slice(
            &axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(body["error"]["code"], "validation_error");
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM fantasy_selections")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn all_round_mutation_events_use_tournament_then_round_identity(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    let t = f.base.tournament;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let input = lineup(&f);
    for (suffix, value) in [
        (
            "deadline",
            json!({"deadline":Utc::now()+Duration::hours(1)}),
        ),
        (
            "lineup",
            json!({"request_id":input.request_id,"expected_revision":input.expected_revision,"picks":input.picks,"captain":input.captain}),
        ),
    ] {
        let response = app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/api/tournaments/{t}/fantasy/rounds/{r}/{suffix}"),
                Some(ADMIN_TOKEN),
                value,
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let event = events.try_recv().unwrap();
        assert_eq!(event.tournament_id, t);
        assert_eq!(event.id, r);
        assert_eq!(event.resource, "round");
    }
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    let p = f.players[0];
    let source = fantasy::source(&pool, f.base.session, t, r, OwnerKind::Player, p)
        .await
        .unwrap();
    let response=app.oneshot(request("POST",&format!("/api/tournaments/{t}/fantasy/rounds/{r}/owners/player/{p}"),Some(ADMIN_TOKEN),json!({"expected_source_token":source.source_token,"disposed":true,"correction":false,"reason":"Stopped"}))).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let event = events.try_recv().unwrap();
    assert_eq!(event.tournament_id, t);
    assert_eq!(event.id, r);
    assert_eq!(event.resource, "round");
}
#[sqlx::test(migrations = "../migrations")]
async fn bulk_fingerprints_preserve_exact_f3_canonical_hashes(pool: PgPool) {
    let f = fixture(&pool).await;
    let r = f.rounds[0];
    close(&pool, &f, r).await;
    source_fixture(&pool, &f, r).await;
    for phase in 0..2 {
        if phase == 1 {
            numeric(
                &pool,
                &f,
                r,
                ScoreOwner::Player { id: f.players[0] },
                holes(&pool, r).await[0],
                3,
            )
            .await;
        }
        for p in &f.players {
            let old: Value =
                sqlx::query_scalar(include_str!("fantasy_support/f3_source_facts.sql"))
                    .bind(r)
                    .bind("player")
                    .bind(p)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
            let token = URL_SAFE_NO_PAD.encode(Sha256::digest(serde_json::to_vec(&old).unwrap()));
            assert_eq!(
                token,
                fantasy::source(
                    &pool,
                    f.base.session,
                    f.base.tournament,
                    r,
                    OwnerKind::Player,
                    *p
                )
                .await
                .unwrap()
                .source_token
            );
        }
    }
}
