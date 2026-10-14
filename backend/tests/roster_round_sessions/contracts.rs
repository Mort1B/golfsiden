use super::support::*;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use golf_api::{
    domain::models::TournamentRole,
    repositories::{round_lifecycle, rounds, tournaments},
};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use uuid::Uuid;

async fn snapshot(pool: &PgPool) -> String {
    // Report a digest on failure, not session/user rows.
    sqlx::query_scalar("SELECT md5(jsonb_build_object(
      'tournaments',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tournaments t),
      'rounds',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r),
      'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY tournament_id,user_id) FROM tournament_memberships m),
      'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
      'sessions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM user_sessions s),
      'players',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM players p),
      'entrants',(SELECT jsonb_agg(to_jsonb(p) ORDER BY tournament_id,player_id) FROM tournament_players p),
      'locks',(SELECT jsonb_agg(to_jsonb(l) ORDER BY tournament_id) FROM tournament_handicap_locks l)
      )::text)")
        .fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_reads_preserve_payloads_order_and_make_no_writes_or_sse(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let roster =
        serde_json::to_value(tournaments::list_players(&pool, TRIP).await.unwrap()).unwrap();
    let list = serde_json::to_value(rounds::list(&pool, TRIP).await.unwrap()).unwrap();
    let round = serde_json::to_value(rounds::get(&pool, ROUND).await.unwrap().unwrap()).unwrap();
    assert_eq!(roster["players"].as_array().unwrap().len(), 8);
    assert_eq!(
        roster["players"][0]["player_id"],
        "00000000-0000-0000-0000-000000001001"
    );
    assert_eq!(roster["players"][0]["tournament_handicap"], 8.2);
    assert_eq!(
        list.as_array()
            .unwrap()
            .iter()
            .map(|r| r["round_number"].as_i64().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5]
    );
    let before = snapshot(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut reads = Vec::new();
    for path in paths().into_iter().cycle().take(12) {
        let router = app.clone();
        let token = token.clone();
        let expected = if path.ends_with("/players") {
            roster.clone()
        } else if path.ends_with("/rounds") {
            list.clone()
        } else {
            round.clone()
        };
        reads.push(tokio::spawn(async move {
            let response = get(&router, &path, &token).await;
            assert_eq!(response.status(), StatusCode::OK, "{path}");
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "private, no-store"
            );
            assert_eq!(body(response).await, expected);
        }));
    }
    for read in reads {
        timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(snapshot(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn every_role_missing_cross_tournament_and_get_only_cache_contracts(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    sqlx::query("UPDATE users SET role='viewer' WHERE id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    for role in [
        TournamentRole::Admin,
        TournamentRole::Scorer,
        TournamentRole::Player,
        TournamentRole::Viewer,
    ] {
        sqlx::query(
            "UPDATE tournament_memberships SET role=$1 WHERE tournament_id=$2 AND user_id=$3",
        )
        .bind(role)
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        for path in paths() {
            let r = get(&app, &path, &token).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        }
    }
    // A global admin with membership only in a different trip has no bypass.
    let other = Uuid::new_v4();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Other','2026-10-01','2026-10-01',1)").bind(other).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
    )
    .bind(other)
    .bind(USER)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query("UPDATE users SET role='admin' WHERE id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1 AND tournament_id=$2")
        .bind(USER)
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    for path in paths() {
        let r = get(&app, &path, &token).await;
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        assert_eq!(body(r).await["error"]["code"], "forbidden");
        let r = get(&app, &path, "unknown-session").await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
    }
    for target in [Uuid::new_v4().to_string(), "invalid".into()] {
        for path in [
            format!("/api/tournaments/{target}/players"),
            format!("/api/tournaments/{target}/rounds"),
            format!("/api/rounds/{target}"),
        ] {
            let r = get(&app, &path, &token).await;
            assert_eq!(
                r.status(),
                if target == "invalid" {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::NOT_FOUND
                }
            );
            assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        }
    }
    let post = app
        .clone()
        .oneshot(
            Request::post(format!("/api/tournaments/{TRIP}/rounds"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(post.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(post.headers()[header::CACHE_CONTROL], "no-store");
}

#[sqlx::test(migrations = "../migrations")]
async fn roster_preserves_handicap_lock_state_and_round_fields_after_opening(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
    let path = format!("/api/tournaments/{TRIP}/players");
    let before = body(get(&app, &path, &token).await).await;
    assert_eq!(before["handicap_correction"], json!({"state":"editable"}));
    let updated = sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap();
    tournaments::start_authorized(&pool, session, TRIP, updated)
        .await
        .unwrap();
    round_lifecycle::open_authorized(&pool, session, ROUND)
        .await
        .unwrap();
    let roster = body(get(&app, &path, &token).await).await;
    assert_eq!(roster["players"], before["players"]);
    assert_eq!(
        roster["handicap_correction"],
        json!({"state":"locked","reason":"snapshot_captured"})
    );
    let round = body(get(&app, &format!("/api/rounds/{ROUND}"), &token).await).await;
    assert_eq!(round["status"], "open");
    assert_eq!(round["scoring_format"], "team_scramble");
    assert_eq!(
        round,
        serde_json::to_value(rounds::get(&pool, ROUND).await.unwrap().unwrap()).unwrap()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn empty_roster_and_round_list_preserve_their_envelopes(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let trip = empty_trip(&pool).await;
    let r = get(&app, &format!("/api/tournaments/{trip}/rounds"), &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(body(r).await, json!([]));
    let r = get(&app, &format!("/api/tournaments/{trip}/players"), &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        body(r).await,
        json!({"players":[],"handicap_correction":{"state":"editable"}})
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn cancelled_membership_wait_releases_authority_for_all_three_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    for path in paths() {
        let (session, token) = session(&pool).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let router = app.clone();
        let pending = path.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &token).await });
        wait_blocked(&pool, pid).await;
        read.abort();
        assert!(read.await.unwrap_err().is_cancelled());
        gate.commit().await.unwrap();
        timeout(
            Duration::from_secs(10),
            golf_api::repositories::auth::revoke_session(&pool, session),
        )
        .await
        .unwrap()
        .unwrap();
        let (_, fresh) = super::support::session(&pool).await;
        assert_eq!(get(&app, &path, &fresh).await.status(), StatusCode::OK);
    }
}
