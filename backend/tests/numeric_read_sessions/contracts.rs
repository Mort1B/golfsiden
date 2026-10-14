use super::support::*;
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use golf_api::{
    domain::{models::TournamentRole, scorecards::ScoreOwner},
    repositories::scorecards,
};
use serde_json::json;
use sqlx::PgPool;
use tokio::time::{Duration, timeout};
use tower::ServiceExt;
use uuid::Uuid;

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_reads_preserve_full_json_and_do_not_write_or_emit_sse(pool: PgPool) {
    use golf_api::domain::{score_visibility::unrestricted, scorecards::read_projection};
    let state = fixture(&pool).await;
    fill_team(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let summary = scorecards::get(&pool, ROUND, ScoreOwner::Team { id: TEAM })
        .await
        .unwrap();
    let expected_scoring = serde_json::to_value(&summary).unwrap();
    let expected_read = serde_json::to_value(read_projection(summary, unrestricted())).unwrap();
    assert_eq!(expected_scoring["confirmed"], true);
    assert_eq!(expected_read["holes"].as_array().unwrap().len(), 18);
    let r = get(&app, &paths()[0], &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    let expected_access = body(r).await;
    assert_eq!(
        expected_access["writable_owners"].as_array().unwrap().len(),
        4
    );
    let before = score_state(&pool).await;
    let rounds_before: serde_json::Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r")
            .fetch_one(&pool)
            .await
            .unwrap();
    let mut events = state.live_events.subscribe();
    let mut reads = Vec::new();
    for (index, path) in paths().into_iter().enumerate().cycle().take(12) {
        let expected = match index {
            0 => expected_access.clone(),
            1 => expected_read.clone(),
            _ => expected_scoring.clone(),
        };
        let router = app.clone();
        let t = token.clone();
        reads.push(tokio::spawn(async move {
            let r = get(&router, &path, &t).await;
            assert_eq!(r.status(), StatusCode::OK);
            assert_private(&r);
            assert_eq!(body(r).await, expected);
        }));
    }
    for r in reads {
        timeout(Duration::from_secs(10), r).await.unwrap().unwrap();
    }
    assert_eq!(score_state(&pool).await, before);
    let rounds_after: serde_json::Value =
        sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(r) ORDER BY id) FROM rounds r")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rounds_before, rounds_after);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn exact_roles_flight_permissions_resources_and_mutation_cache_are_preserved(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
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
        let privileged = matches!(role, TournamentRole::Admin | TournamentRole::Scorer);
        for (index, path) in paths().iter().enumerate() {
            let r = get(&app, path, &token).await;
            assert_private(&r);
            assert_eq!(
                r.status(),
                if index == 2 && !privileged {
                    StatusCode::FORBIDDEN
                } else {
                    StatusCode::OK
                }
            );
            let v = body(r).await;
            if index == 0 {
                assert_eq!(
                    v["writable_owners"].as_array().unwrap().len(),
                    if privileged { 4 } else { 0 }
                );
            }
        }
    }
    let player_user: Uuid = sqlx::query_scalar("SELECT id FROM users WHERE player_id=$1")
        .bind(FIRST_PLAYER)
        .fetch_one(&pool)
        .await
        .unwrap();
    let (_, player_token) = session_for(&pool, player_user).await;
    let r = get(&app, &paths()[0], &player_token).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(
        body(r).await["writable_owners"].as_array().unwrap().len(),
        2
    );
    let other = uuid::uuid!("00000000-0000-0000-0000-000000005003");
    let other_path = format!("/api/rounds/{ROUND}/scorecards/team/{other}");
    assert_eq!(
        get(&app, &paths()[2], &player_token).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        get(&app, &other_path, &player_token).await.status(),
        StatusCode::OK
    );
    assert_eq!(
        get(&app, &format!("{other_path}/scoring"), &player_token)
            .await
            .status(),
        StatusCode::FORBIDDEN
    );
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
        .bind(TRIP)
        .bind(USER)
        .execute(&pool)
        .await
        .unwrap();
    for path in paths() {
        let r = get(&app, &path, &token).await;
        assert_eq!(r.status(), StatusCode::FORBIDDEN);
        assert_private(&r);
    }
    for round in [Uuid::new_v4().to_string(), "invalid".to_owned()] {
        for suffix in [
            "score-access".to_owned(),
            format!("scorecards/team/{TEAM}"),
            format!("scorecards/team/{TEAM}/scoring"),
        ] {
            let r = get(&app, &format!("/api/rounds/{round}/{suffix}"), &token).await;
            assert_private(&r);
            assert_eq!(
                r.status(),
                if round == "invalid" {
                    StatusCode::BAD_REQUEST
                } else {
                    StatusCode::NOT_FOUND
                }
            );
        }
    }
    for path in [
        format!("/api/rounds/{ROUND}/scores"),
        format!("/api/rounds/{ROUND}/scorecards/team/{TEAM}/confirm"),
        format!("/api/rounds/{ROUND}/scores/conditional"),
    ] {
        let expected_cache = if path.ends_with("/conditional") {
            "private, no-store"
        } else {
            "no-store"
        };
        let request = if path.ends_with("/confirm") {
            Request::post(path)
        } else {
            Request::put(path)
        };
        let r = app
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(r.headers()["cache-control"], expected_cache);
        assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn format_owner_and_state_error_precedence_remains_unchanged(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    let missing = Uuid::new_v4();
    for (suffix, status, code) in [
        (
            format!("scorecards/team/{missing}"),
            StatusCode::NOT_FOUND,
            "not_found",
        ),
        (
            format!("scorecards/team/{missing}/scoring"),
            StatusCode::FORBIDDEN,
            "forbidden",
        ),
        (
            format!("scorecards/chicken/{TEAM}"),
            StatusCode::BAD_REQUEST,
            "validation_error",
        ),
    ] {
        let r = get(&app, &format!("/api/rounds/{ROUND}/{suffix}"), &token).await;
        assert_eq!(r.status(), status);
        assert_private(&r);
        assert_eq!(body(r).await["error"]["code"], code);
    }
    sqlx::query("UPDATE rounds SET scoring_format='singles_match_play' WHERE id=$1")
        .bind(DRAFT)
        .execute(&pool)
        .await
        .unwrap();
    let r = get(&app, &format!("/api/rounds/{DRAFT}/score-access"), &token).await;
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(body(r).await["writable_owners"], json!([]));
    for scoring in ["", "/scoring"] {
        let r = get(
            &app,
            &format!("/api/rounds/{DRAFT}/scorecards/player/{FIRST_PLAYER}{scoring}"),
            &token,
        )
        .await;
        assert_eq!(r.status(), StatusCode::CONFLICT);
        assert_private(&r);
        assert_eq!(
            body(r).await["error"]["code"],
            "score_owner_format_mismatch"
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn cancellation_releases_session_authority_on_all_three_reads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let before = score_state(&pool).await;
    let mut events = state.live_events.subscribe();
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
    assert_eq!(score_state(&pool).await, before);
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn late_confirmation_loading_keeps_one_score_and_confirmation_snapshot(pool: PgPool) {
    let state = fixture(&pool).await;
    fill_team(&pool).await;
    let app = app(&state);
    let (_, token) = session(&pool).await;
    for path in paths().into_iter().skip(1) {
        // Restore confirmation after the previous deliberate correction.
        scorecards::confirm(&pool, ROUND, ScoreOwner::Team { id: TEAM }, USER)
            .await
            .unwrap();
        let baseline = body(get(&app, &path, &token).await).await;
        assert_eq!(baseline["confirmed"], true);
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_loading(&mut gate, &path).await;
        let router = app.clone();
        let p = path.clone();
        let t = token.clone();
        let read = tokio::spawn(async move { get(&router, &p, &t).await });
        wait_blocked(&pool, pid).await;
        sqlx::query("SELECT set_config('app.score_mutation_round_id',$1::text,true)")
            .bind(ROUND)
            .execute(&mut *gate)
            .await
            .unwrap();
        sqlx::query("UPDATE scores SET gross_strokes=gross_strokes+1,submitted_by=$3 WHERE round_id=$1 AND team_id=$2").bind(ROUND).bind(TEAM).bind(USER).execute(&mut *gate).await.unwrap();
        gate.commit().await.unwrap();
        let changed = score_state(&pool).await;
        let mut events = state.live_events.subscribe();
        let r = timeout(Duration::from_secs(10), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(r.status(), StatusCode::OK);
        assert_eq!(body(r).await, baseline);
        let fresh = body(get(&app, &path, &token).await).await;
        assert_ne!(fresh, baseline);
        assert_eq!(fresh["confirmed"], false);
        assert_eq!(score_state(&pool).await, changed);
        assert!(events.try_recv().is_err());
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn nine_hole_foursomes_keep_preserved_handicaps_and_exact_payloads(pool: PgPool) {
    let state = fixture(&pool).await;
    let app = app(&state);
    let (session, token) = session(&pool).await;
    let course = Uuid::new_v4();
    let tee = Uuid::new_v4();
    sqlx::query("INSERT INTO courses(id,name) VALUES($1,'Synthetic nine')")
        .bind(course)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tees(id,course_id,name,category,number_of_holes,slope_rating,course_rating) VALUES($1,$2,'Test nine','male',9,113,36)").bind(tee).bind(course).execute(&pool).await.unwrap();
    for n in 1..=9_i16 {
        sqlx::query(
            "INSERT INTO holes(id,tee_id,hole_number,par,stroke_index) VALUES($1,$2,$3,4,$3)",
        )
        .bind(Uuid::new_v4())
        .bind(tee)
        .bind(n)
        .execute(&pool)
        .await
        .unwrap();
    }
    sqlx::query("UPDATE rounds SET course_id=$2,tee_id=$3,course_name='Synthetic nine',tee_name='Test nine',number_of_holes=9,scoring_format='two_player_foursomes',handicap_allowance_percent=50 WHERE id=$1").bind(DRAFT).bind(course).bind(tee).execute(&pool).await.unwrap();
    golf_api::repositories::round_lifecycle::open_authorized(&pool, session, DRAFT)
        .await
        .unwrap();
    let team: Uuid =
        sqlx::query_scalar("SELECT id FROM teams WHERE round_id=$1 ORDER BY id LIMIT 1")
            .bind(DRAFT)
            .fetch_one(&pool)
            .await
            .unwrap();
    let path = format!("/api/rounds/{DRAFT}/scorecards/team/{team}");
    let read = body(get(&app, &path, &token).await).await;
    let scoring = body(get(&app, &format!("{path}/scoring"), &token).await).await;
    assert_eq!(read["number_of_holes"], 9);
    assert_eq!(read["holes"].as_array().unwrap().len(), 9);
    assert_eq!(read["playing_handicap"], scoring["playing_handicap"]);
    let expected:i16=sqlx::query_scalar("SELECT playing_handicap FROM round_team_handicap_snapshots WHERE round_id=$1 AND team_id=$2").bind(DRAFT).bind(team).fetch_one(&pool).await.unwrap();
    assert_eq!(read["playing_handicap"], expected);
    sqlx::query("UPDATE players SET current_handicap_index=1 WHERE id=$1")
        .bind(FIRST_PLAYER)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(read, body(get(&app, &path, &token).await).await);
    assert_eq!(
        scoring,
        body(get(&app, &format!("{path}/scoring"), &token).await).await
    );
}
