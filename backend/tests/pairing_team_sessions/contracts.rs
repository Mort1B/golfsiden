use super::support::*;
use axum::{
    body::Body,
    http::{Request, StatusCode, header},
};
use golf_api::{
    domain::models::{ParticipantStatus, TournamentRole},
    repositories::teams,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use uuid::{Uuid, uuid};

async fn snapshot(pool: &PgPool) -> String {
    // Digest failures never print session/user records.
    sqlx::query_scalar("SELECT md5(jsonb_build_object(
      'rounds',(SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r),
      'users',(SELECT jsonb_agg(to_jsonb(u) ORDER BY id) FROM users u),
      'sessions',(SELECT jsonb_agg(to_jsonb(s) ORDER BY id) FROM user_sessions s),
      'memberships',(SELECT jsonb_agg(to_jsonb(m) ORDER BY tournament_id,user_id) FROM tournament_memberships m),
      'players',(SELECT jsonb_agg(to_jsonb(p) ORDER BY id) FROM players p),
      'entrants',(SELECT jsonb_agg(to_jsonb(p) ORDER BY tournament_id,player_id) FROM tournament_players p),
      'teams',(SELECT jsonb_agg(to_jsonb(t) ORDER BY id) FROM teams t),
      'team_members',(SELECT jsonb_agg(to_jsonb(m) ORDER BY team_id,player_id) FROM team_memberships m),
      'flights',(SELECT jsonb_agg(to_jsonb(f) ORDER BY id) FROM flights f),
      'flight_members',(SELECT jsonb_agg(to_jsonb(m) ORDER BY flight_id,player_id) FROM flight_memberships m)
      )::text)")
        .fetch_one(pool).await.unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_valid_reads_preserve_payloads_and_do_not_write_or_emit_sse(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let pairings = body(get(&app, &paths()[0], &token).await).await;
    assert_eq!(pairings["round_id"], ROUND.to_string());
    assert_eq!(pairings["tournament_id"], TRIP.to_string());
    assert_eq!(pairings["status"], "draft");
    assert_eq!(pairings["scoring_format"], "team_scramble");
    assert_eq!(pairings["active_entrants"].as_array().unwrap().len(), 8);
    assert_eq!(pairings["teams"].as_array().unwrap().len(), 4);
    assert_eq!(pairings["flights"].as_array().unwrap().len(), 2);
    assert_ne!(pairings["teams"][0]["id"], pairings["flights"][0]["id"]);
    let teams = serde_json::to_value(teams::list(&pool, ROUND).await.unwrap()).unwrap();
    assert_eq!(teams[0]["id"], TEAM.to_string());
    assert_eq!(
        teams[0]["members"][0]["player_id"],
        FIRST_PLAYER.to_string()
    );
    let before = snapshot(&pool).await;
    let mut events = state.live_events.subscribe();
    let mut reads = Vec::new();
    for path in paths().into_iter().cycle().take(12) {
        let router = app.clone();
        let token = token.clone();
        let expected = if path.ends_with("/pairings") {
            pairings.clone()
        } else {
            teams.clone()
        };
        reads.push(tokio::spawn(async move {
            let r = get(&router, &path, &token).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
            assert_eq!(body(r).await, expected);
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
async fn roles_cross_tournament_missing_invalid_and_put_contracts_are_preserved(pool: PgPool) {
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
    let _other = empty_round(&pool).await;
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
        let r = get(&app, &path, "unknown").await;
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
    }
    for target in [Uuid::new_v4().to_string(), "invalid".into()] {
        for suffix in ["pairings", "teams"] {
            let r = get(&app, &format!("/api/rounds/{target}/{suffix}"), &token).await;
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
    let r = app
        .clone()
        .oneshot(
            Request::put(format!("/api/rounds/{ROUND}/pairings"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(r.headers()[header::CACHE_CONTROL], "private, no-store");
}

#[sqlx::test(migrations = "../migrations")]
async fn empty_round_reads_preserve_typed_pairing_and_empty_team_envelopes(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let (trip, round) = empty_round(&pool).await;
    let r = get(&app, &format!("/api/rounds/{round}/teams"), &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(body(r).await, json!([]));
    let r = get(&app, &format!("/api/rounds/{round}/pairings"), &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    let v = body(r).await;
    assert_eq!(v["round_id"], round.to_string());
    assert_eq!(v["tournament_id"], trip.to_string());
    assert_eq!(v["status"], "draft");
    assert_eq!(v["scoring_format"], "team_scramble");
    for key in [
        "active_entrants",
        "inactive_entrants",
        "teams",
        "flights",
        "legacy_individual_groups",
    ] {
        assert_eq!(v[key], json!([]));
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn legacy_individual_groups_keep_identity_schedule_and_distinct_sorting(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let (trip, round) = empty_round(&pool).await;
    sqlx::query("UPDATE rounds SET scoring_format='individual_stroke_play' WHERE id=$1")
        .bind(round)
        .execute(&pool)
        .await
        .unwrap();
    let lo = uuid!("d1000000-0000-0000-0000-000000000001");
    let hi = uuid!("d1000000-0000-0000-0000-000000000002");
    let p1 = uuid!("d1000000-0000-0000-0000-000000000011");
    let p2 = uuid!("d1000000-0000-0000-0000-000000000012");
    let inactive = uuid!("d1000000-0000-0000-0000-000000000013");
    for (id, name) in [(p1, "b-player"), (p2, "B-player"), (inactive, "Withdrawn")] {
        sqlx::query("INSERT INTO players(id,display_name,current_handicap_index,active) VALUES($1,$2,12,$3)").bind(id).bind(name).bind(id!=inactive).execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap,status) VALUES($1,$2,12,$3)").bind(trip).bind(id).bind(if id==inactive {ParticipantStatus::Withdrawn} else {ParticipantStatus::Active}).execute(&pool).await.unwrap();
    }
    for (id, name) in [(lo, "b-group"), (hi, "B-group")] {
        sqlx::query("INSERT INTO teams(id,round_id,tournament_id,name,starting_hole,tee_time) VALUES($1,$2,$3,$4,3,'08:30')").bind(id).bind(round).bind(trip).bind(name).execute(&pool).await.unwrap();
    }
    for player in [p1, p2] {
        sqlx::query("INSERT INTO team_memberships(team_id,round_id,tournament_id,player_id,display_order) VALUES($1,$2,$3,$4,NULL)").bind(lo).bind(round).bind(trip).bind(player).execute(&pool).await.unwrap();
    }
    let teams = body(get(&app, &format!("/api/rounds/{round}/teams"), &token).await).await;
    let pairing = body(get(&app, &format!("/api/rounds/{round}/pairings"), &token).await).await;
    assert_eq!(teams[0]["id"], hi.to_string());
    assert_eq!(teams[1]["id"], lo.to_string());
    assert_eq!(teams[1]["members"][0]["player_id"], p2.to_string());
    assert_eq!(pairing["teams"], json!([]));
    assert_eq!(pairing["flights"], json!([]));
    let groups = &pairing["legacy_individual_groups"];
    assert_eq!(groups[0]["id"], lo.to_string());
    assert_eq!(groups[1]["id"], hi.to_string());
    assert_eq!(groups[0]["members"][0]["player_id"], p1.to_string());
    assert_eq!(groups[0]["starting_hole"], 3);
    assert_eq!(groups[0]["tee_time"], "08:30:00");
    assert_eq!(groups[0]["created_at"], teams[1]["created_at"]);
    assert_eq!(groups[0]["updated_at"], teams[1]["updated_at"]);
    assert_eq!(
        pairing["inactive_entrants"][0]["player_id"],
        inactive.to_string()
    );
    assert_eq!(pairing["inactive_entrants"][0]["status"], "withdrawn");
}

#[sqlx::test(migrations = "../migrations")]
async fn member_loading_uses_one_snapshot_for_group_and_member_facts(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    for path in paths() {
        let (_, token) = session(&pool).await;
        let before = body(get(&app, &path, &token).await).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        sqlx::query("LOCK TABLE team_memberships IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *gate)
            .await
            .unwrap();
        let router = app.clone();
        let pending = path.clone();
        let pending_token = token.clone();
        let read = tokio::spawn(async move { get(&router, &pending, &pending_token).await });
        wait_blocked(&pool, pid).await;
        sqlx::query("UPDATE teams SET name=name || ' changed' WHERE id=$1")
            .bind(TEAM)
            .execute(&mut *gate)
            .await
            .unwrap();
        sqlx::query("UPDATE team_memberships SET display_order=display_order+2 WHERE team_id=$1 AND player_id=$2").bind(TEAM).bind(FIRST_PLAYER).execute(&mut *gate).await.unwrap();
        gate.commit().await.unwrap();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(body(r).await, before);
        assert_ne!(body(get(&app, &path, &token).await).await, before);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn cancellation_releases_session_authority_for_both_reads(pool: PgPool) {
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
