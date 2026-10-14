use super::support::*;
use axum::http::{StatusCode, header};
use golf_api::{domain::models::TournamentRole, repositories::tournaments};
use serde_json::{Value, json};
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use uuid::{Uuid, uuid};

const OTHER: Uuid = uuid!("88000000-0000-0000-0000-000000000001");

async fn add_trip(pool: &PgPool) {
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,'Later trip','2027-01-01','2027-01-01',1)").bind(OTHER).execute(pool).await.unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn new_membership_after_snapshot_is_absent_until_a_fresh_read(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    add_trip(&pool).await;
    for path in ["/api/tournaments", "/api/me/tournaments"] {
        let (_, token) = session(&pool).await;
        let before = body(get(&app, path, &token).await).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let router = app.clone();
        let pending_token = token.clone();
        let read = tokio::spawn(async move { get(&router, path, &pending_token).await });
        wait_blocked(&pool, pid).await;
        sqlx::query(
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'viewer')",
        )
        .bind(OTHER)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body(response).await, before);
        assert_eq!(
            body(get(&app, path, &token).await)
                .await
                .as_array()
                .unwrap()
                .len(),
            2
        );
        sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
            .bind(OTHER)
            .bind(USER)
            .execute(&pool)
            .await
            .unwrap();
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn collection_membership_share_locks_serialize_later_removal(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    for path in ["/api/tournaments", "/api/me/tournaments"] {
        let (_, token) = session(&pool).await;
        let before = body(get(&app, path, &token).await).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_tournaments(&mut gate).await;
        let router = app.clone();
        let pending_token = token.clone();
        let read = tokio::spawn(async move { get(&router, path, &pending_token).await });
        let reader_pid = wait_blocked(&pool, pid).await;
        let write_pool = pool.clone();
        let remove = tokio::spawn(async move {
            sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
                .bind(TRIP)
                .bind(USER)
                .execute(&write_pool)
                .await
                .unwrap();
        });
        wait_blocked(&pool, reader_pid).await;
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body(response).await, before);
        timeout(Duration::from_secs(10), remove)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(body(get(&app, path, &token).await).await, json!([]));
        sqlx::query(
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'admin')",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    }
}

async fn snapshot(pool: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object(
      'tournaments',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM tournaments t),
      'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY tournament_id,user_id) FROM tournament_memberships m),
      'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
      'sessions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM user_sessions s),
      'entrants',(SELECT jsonb_agg(to_jsonb(p) ORDER BY tournament_id,player_id) FROM tournament_players p))")
        .fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_valid_reads_match_payloads_without_writes_or_sse(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
    let expected =
        serde_json::to_value(tournaments::get(&pool, TRIP).await.unwrap().unwrap()).unwrap();
    let before = snapshot(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut reads = Vec::new();
    for path in paths().into_iter().cycle().take(12) {
        let router = app.clone();
        let token = token.clone();
        let expected = expected.clone();
        reads.push(tokio::spawn(async move {
            let response = get(&router, &path, &token).await;
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response.headers()[header::CACHE_CONTROL],
                "private, no-store"
            );
            let value = body(response).await;
            let trip = match path.as_str() {
                "/api/tournaments" => value[0].clone(),
                "/api/me/tournaments" => {
                    assert_eq!(value[0]["role"], "admin");
                    assert!(value[0]["player_id"].is_null());
                    value[0]["tournament"].clone()
                }
                _ => value,
            };
            assert_eq!(trip, expected);
        }));
    }
    for read in reads {
        timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
    }
    assert_eq!(
        serde_json::to_value(
            tournaments::get_for_session(&pool, session, TRIP)
                .await
                .unwrap()
        )
        .unwrap(),
        expected
    );
    assert_eq!(snapshot(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn cancellation_releases_session_authority_and_allows_subsequent_reads(pool: PgPool) {
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

#[sqlx::test(migrations = "../migrations")]
async fn response_order_roles_player_links_empty_and_error_contracts_are_preserved(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    // The two public list shapes deliberately sort names differently. Equal
    // case-folded names must still use the stable UUID tie breaker for "mine".
    let lower = OTHER;
    let upper = uuid!("88000000-0000-0000-0000-000000000002");
    for (id, name, role) in [
        (lower, "b-trip", TournamentRole::Scorer),
        (upper, "B-trip", TournamentRole::Viewer),
    ] {
        sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds) VALUES($1,$2,'2027-01-01','2027-01-01',1)").bind(id).bind(name).execute(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,$3)",
        )
        .bind(id)
        .bind(USER)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
    }
    let (_, token) = session(&pool).await;
    let list = body(get(&app, "/api/tournaments", &token).await).await;
    assert_eq!(
        list.as_array()
            .unwrap()
            .iter()
            .map(|t| t["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![upper.to_string(), lower.to_string(), TRIP.to_string()]
    );
    let mine = body(get(&app, "/api/me/tournaments", &token).await).await;
    assert_eq!(
        mine.as_array()
            .unwrap()
            .iter()
            .map(|t| t["tournament"]["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec![lower.to_string(), upper.to_string(), TRIP.to_string()]
    );
    assert_eq!(mine[0]["role"], "scorer");
    assert_eq!(mine[1]["role"], "viewer");
    let player = uuid!("88000000-0000-0000-0000-000000000003");
    sqlx::query("INSERT INTO players(id,display_name,current_handicap_index) VALUES($1,'Unclaimed entrant',10)").bind(player).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES($1,$2,10)").bind(TRIP).bind(player).execute(&pool).await.unwrap();
    sqlx::query("UPDATE users SET player_id=$1 WHERE id=$2")
        .bind(player)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    let mine = body(get(&app, "/api/me/tournaments", &token).await).await;
    assert_eq!(mine[2]["player_id"], player.to_string());
    assert!(mine[0]["player_id"].is_null());
    assert!(mine[1]["player_id"].is_null());
    sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    for path in paths() {
        let r = get(&app, &path, &token).await;
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
        if path.ends_with(&TRIP.to_string()) {
            assert_eq!(r.status(), StatusCode::FORBIDDEN);
            assert_eq!(body(r).await["error"]["code"], "forbidden");
        } else {
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(body(r).await, json!([]));
        }
        let r = get(&app, &path, "unknown-token").await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
    }
    let r = get(
        &app,
        &format!("/api/tournaments/{}", Uuid::new_v4()),
        &token,
    )
    .await;
    assert_eq!(r.status(), StatusCode::NOT_FOUND);
    assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
    assert_eq!(body(r).await["error"]["code"], "not_found");
    let r = get(&app, "/api/tournaments/invalid", &token).await;
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
}
