#![cfg(feature = "database-tests")]

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use chrono::{DateTime, Duration, Utc};
use golf_api::{
    AppState, api,
    auth::{derive_csrf_token, hash_session_token},
    domain::{models::TournamentStatus, scorecards::ScoreOwner},
    repositories::{auth, round_completion, round_lifecycle, scorecards, tournaments},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::{Uuid, uuid};

const TRIP: Uuid = uuid!("19000000-0000-0000-0000-000000000001");
const ROUND: Uuid = uuid!("19000000-0000-0000-0000-000000000002");
const ADMIN: Uuid = uuid!("19000000-0000-0000-0000-000000000003");
const PLAYER: Uuid = uuid!("19000000-0000-0000-0000-000000000004");
const TOKEN: &str = "archive-test-admin";

async fn seed(pool: &PgPool, locked: bool) -> Uuid {
    sqlx::raw_sql("INSERT INTO users(id,username,display_name,role) VALUES
      ('19000000-0000-0000-0000-000000000003','completion_admin','Admin','player');
      INSERT INTO players(id,display_name,current_handicap_index) VALUES
      ('19000000-0000-0000-0000-000000000004','Player',0);
      INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES
      ('19000000-0000-0000-0000-000000000001','Completion','2026-09-01','2026-09-01',1,1);
      INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES
      ('19000000-0000-0000-0000-000000000001','19000000-0000-0000-0000-000000000003','admin');
      INSERT INTO tournament_players(tournament_id,player_id,tournament_handicap) VALUES
      ('19000000-0000-0000-0000-000000000001','19000000-0000-0000-0000-000000000004',0);
      INSERT INTO courses(id,name) VALUES ('19000000-0000-0000-0000-000000000005','Course');
      INSERT INTO tees(id,course_id,name,slope_rating,course_rating) VALUES
      ('19000000-0000-0000-0000-000000000006','19000000-0000-0000-0000-000000000005','Tee',113,72);
      INSERT INTO holes(id,tee_id,hole_number,par,stroke_index) VALUES
      ('19000000-0000-0000-0000-000000000007','19000000-0000-0000-0000-000000000006',1,4,1);
      INSERT INTO holes(id,tee_id,hole_number,par,stroke_index)
      SELECT gen_random_uuid(),'19000000-0000-0000-0000-000000000006',n,4,n FROM generate_series(2,18) n;
      INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_id,course_name,tee_id,tee_name,number_of_holes,scoring_format) VALUES
      ('19000000-0000-0000-0000-000000000002','19000000-0000-0000-0000-000000000001',1,'Round','2026-09-01','19000000-0000-0000-0000-000000000005','Course','19000000-0000-0000-0000-000000000006','Tee',18,'individual_stroke_play');
      INSERT INTO flights(id,round_id,tournament_id,name) VALUES
      ('19000000-0000-0000-0000-000000000008','19000000-0000-0000-0000-000000000002','19000000-0000-0000-0000-000000000001','Flight');
      INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id) VALUES
      ('19000000-0000-0000-0000-000000000008','19000000-0000-0000-0000-000000000002','19000000-0000-0000-0000-000000000001','19000000-0000-0000-0000-000000000004');")
      .execute(pool).await.unwrap();
    let session = auth::create_session(
        pool,
        ADMIN,
        &hash_session_token(TOKEN),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    tournaments::start_authorized(pool, session.session_id, TRIP, version(pool).await)
        .await
        .unwrap();
    if locked {
        lock_round(pool).await;
    }
    session.session_id
}

async fn lock_round(pool: &PgPool) {
    round_lifecycle::open(pool, ROUND).await.unwrap();
    let holes = sqlx::query_scalar::<_, Uuid>("SELECT id FROM holes ORDER BY hole_number")
        .fetch_all(pool)
        .await
        .unwrap();
    for hole_id in holes {
        scorecards::save(
            pool,
            scorecards::SaveScore {
                round_id: ROUND,
                hole_id,
                owner: ScoreOwner::Player { id: PLAYER },
                gross_strokes: 4,
                submitted_by: ADMIN,
            },
        )
        .await
        .unwrap();
    }
    scorecards::confirm(pool, ROUND, ScoreOwner::Player { id: PLAYER }, ADMIN)
        .await
        .unwrap();
    round_completion::complete(pool, ROUND).await.unwrap();
    round_completion::lock(pool, ROUND).await.unwrap();
}
async fn version(pool: &PgPool) -> DateTime<Utc> {
    sqlx::query_scalar("SELECT updated_at FROM tournaments WHERE id=$1")
        .bind(TRIP)
        .fetch_one(pool)
        .await
        .unwrap()
}
fn request(id: Uuid, token: Option<&str>, csrf: bool, data: Value) -> Request<Body> {
    let mut builder = Request::post(format!("/api/tournaments/{id}/archive"))
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("cookie", format!("golf_session={token}"));
        if csrf {
            builder = builder.header("x-csrf-token", derive_csrf_token(token));
        }
    }
    builder.body(Body::from(data.to_string())).unwrap()
}
async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
fn constraint(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .and_then(|error| error.constraint())
}

async fn completed(pool: &PgPool) -> Uuid {
    let session = seed(pool, true).await;
    tournaments::complete_authorized(pool, session, TRIP, version(pool).await)
        .await
        .unwrap();
    session
}

async fn wait_for_lock(pool: &PgPool, query_fragment: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let waiting=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND position($1 IN query)>0)")
                .bind(query_fragment).fetch_one(pool).await.unwrap();
            if waiting { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn sql_guard_rejects_real_non_admin_revoked_expired_and_foreign_sessions(pool: PgPool) {
    let session = completed(&pool).await;
    for case in ["viewer", "revoked", "expired", "foreign"] {
        let mut tx = pool.begin().await.unwrap();
        match case {
            "viewer" => {
                sqlx::query(
                    "UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1",
                )
                .bind(TRIP)
                .execute(&mut *tx)
                .await
                .unwrap();
            }
            "foreign" => {
                sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1")
                    .bind(TRIP)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            }
            "revoked" => {
                sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
                    .bind(session)
                    .execute(&mut *tx)
                    .await
                    .unwrap();
            }
            _ => {
                sqlx::query("UPDATE user_sessions SET created_at=clock_timestamp()-interval '1 day', expires_at=clock_timestamp()-interval '1 second' WHERE id=$1").bind(session).execute(&mut *tx).await.unwrap();
            }
        }
        sqlx::query("SELECT set_config('app.tournament_archive_id',$1::text,true),set_config('app.tournament_archive_session_id',$2::text,true)").bind(TRIP).bind(session).execute(&mut *tx).await.unwrap();
        let error = sqlx::query("UPDATE tournaments SET status='archived' WHERE id=$1")
            .bind(TRIP)
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_eq!(
            constraint(&error),
            Some("tournament_archive_admin_required")
        );
        tx.rollback().await.unwrap();
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn membership_demotion_while_authorization_waits_is_rechecked(pool: PgPool) {
    let session = completed(&pool).await;
    let expected = version(&pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1")
        .bind(TRIP)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let cloned = pool.clone();
    let archive = tokio::spawn(async move {
        tournaments::archive_authorized(&cloned, session, TRIP, expected).await
    });
    wait_for_lock(&pool, "SELECT role FROM tournament_memberships").await;
    blocker.commit().await.unwrap();
    assert!(matches!(
        archive.await.unwrap(),
        Err(tournaments::TournamentMutationError::Authorization(
            golf_api::repositories::tournament_authorization::AuthorizationError::Forbidden
        ))
    ));
    assert_eq!(
        tournaments::get(&pool, TRIP).await.unwrap().unwrap().status,
        TournamentStatus::Completed
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn revocation_while_session_lock_waits_is_rechecked(pool: PgPool) {
    let session = completed(&pool).await;
    let expected = version(&pool).await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("UPDATE user_sessions SET revoked_at=clock_timestamp() WHERE id=$1")
        .bind(session)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let cloned = pool.clone();
    let archive = tokio::spawn(async move {
        tournaments::archive_authorized(&cloned, session, TRIP, expected).await
    });
    wait_for_lock(&pool, "SELECT s.id AS session_id").await;
    blocker.commit().await.unwrap();
    assert!(matches!(
        archive.await.unwrap(),
        Err(tournaments::TournamentMutationError::Authorization(
            golf_api::repositories::tournament_authorization::AuthorizationError::Unauthenticated
        ))
    ));
}

async fn expiry_after_parent_wait(pool: PgPool, retry: bool) {
    let session = completed(&pool).await;
    if retry {
        tournaments::archive_authorized(&pool, session, TRIP, version(&pool).await)
            .await
            .unwrap();
    }
    let expected = version(&pool).await;
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '1 second' WHERE id=$1",
    )
    .bind(session)
    .execute(&pool)
    .await
    .unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR SHARE")
        .bind(TRIP)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let cloned = pool.clone();
    let archive = tokio::spawn(async move {
        tournaments::archive_authorized(&cloned, session, TRIP, expected).await
    });
    wait_for_lock(&pool, "FROM tournaments WHERE id = $1 FOR UPDATE").await;
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    blocker.commit().await.unwrap();
    assert!(matches!(
        archive.await.unwrap(),
        Err(tournaments::TournamentMutationError::Authorization(
            golf_api::repositories::tournament_authorization::AuthorizationError::Unauthenticated
        ))
    ));
    assert_eq!(version(&pool).await, expected);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_archives")
            .fetch_one(&pool)
            .await
            .unwrap(),
        i64::from(retry)
    );
}
#[sqlx::test(migrations = "../migrations")]
async fn archive_rechecks_wall_clock_expiry_after_parent_wait(pool: PgPool) {
    expiry_after_parent_wait(pool, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn archived_retry_rechecks_wall_clock_expiry_after_parent_wait(pool: PgPool) {
    expiry_after_parent_wait(pool, true).await;
}

#[sqlx::test(migrations = "../migrations")]
async fn sql_guard_rechecks_expiry_after_membership_wait(pool: PgPool) {
    let session = completed(&pool).await;
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '1 second' WHERE id=$1",
    )
    .bind(session)
    .execute(&pool)
    .await
    .unwrap();
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 FOR UPDATE")
        .bind(TRIP)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let cloned = pool.clone();
    let archive = tokio::spawn(async move {
        let mut tx = cloned.begin().await.unwrap();
        sqlx::query("SELECT set_config('app.tournament_archive_id',$1::text,true),set_config('app.tournament_archive_session_id',$2::text,true)").bind(TRIP).bind(session).execute(&mut *tx).await.unwrap();
        let result = sqlx::query("UPDATE tournaments SET status='archived' WHERE id=$1")
            .bind(TRIP)
            .execute(&mut *tx)
            .await;
        tx.rollback().await.unwrap();
        result
    });
    wait_for_lock(&pool, "UPDATE tournaments SET status='archived'").await;
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    blocker.commit().await.unwrap();
    assert_eq!(
        constraint(&archive.await.unwrap().unwrap_err()),
        Some("tournament_archive_admin_required")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn archive_audit_survives_deletion_of_otherwise_deletable_actor(pool: PgPool) {
    let session = seed(&pool, true).await;
    let actor = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES ($1,'archive_only','Archive only','player')").bind(actor).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES ($1,$2,'admin')",
    )
    .bind(TRIP)
    .bind(actor)
    .execute(&pool)
    .await
    .unwrap();
    let archive_session = auth::create_session(
        &pool,
        actor,
        &hash_session_token("archive-only"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id;
    tournaments::complete_authorized(&pool, session, TRIP, version(&pool).await)
        .await
        .unwrap();
    tournaments::archive_authorized(&pool, archive_session, TRIP, version(&pool).await)
        .await
        .unwrap();
    let before: DateTime<Utc> =
        sqlx::query_scalar("SELECT archived_at FROM tournament_archives WHERE tournament_id=$1")
            .bind(TRIP)
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(actor)
        .execute(&pool)
        .await
        .unwrap();
    let after: (Option<Uuid>, DateTime<Utc>) = sqlx::query_as(
        "SELECT archived_by_user_id,archived_at FROM tournament_archives WHERE tournament_id=$1",
    )
    .bind(TRIP)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(after, (None, before));
}

async fn legacy_upgrade(pool: PgPool, archived: bool) {
    for migration in golf_api::schema::MIGRATOR
        .iter()
        .filter(|migration| migration.version < 19)
    {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let session = seed(&pool, true).await;
    sqlx::query("UPDATE tournaments SET status='completed' WHERE id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    if archived {
        sqlx::query("UPDATE tournaments SET status='archived' WHERE id=$1")
            .bind(TRIP)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::raw_sql(include_str!(
        "../../migrations/0019_tournament_completion.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    let before: Value = sqlx::query_scalar("SELECT to_jsonb(t) FROM tournaments t WHERE id=$1")
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!("../../migrations/0020_tournament_archive.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let after: Value = sqlx::query_scalar("SELECT to_jsonb(t) FROM tournaments t WHERE id=$1")
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_archives")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_completions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    let result = tournaments::archive_authorized(&pool, session, TRIP, version(&pool).await)
        .await
        .unwrap();
    assert_eq!(result.changed, !archived);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_archives")
            .fetch_one(&pool)
            .await
            .unwrap(),
        i64::from(!archived)
    );
}
#[sqlx::test(migrations = false)]
async fn schema19_upgrade_preserves_legacy_archived_without_inventing_audit(pool: PgPool) {
    legacy_upgrade(pool, true).await;
}
#[sqlx::test(migrations = false)]
async fn schema19_upgrade_accepts_legacy_completed_without_completion_audit(pool: PgPool) {
    legacy_upgrade(pool, false).await;
}

#[sqlx::test(migrations = false)]
async fn schema19_upgrade_preserves_workflow_completion_evidence(pool: PgPool) {
    for migration in golf_api::schema::MIGRATOR
        .iter()
        .filter(|migration| migration.version < 20)
    {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let session = completed(&pool).await;
    let before = history(&pool).await;
    sqlx::raw_sql(include_str!("../../migrations/0020_tournament_archive.sql"))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(before, history(&pool).await);
    tournaments::archive_authorized(&pool, session, TRIP, version(&pool).await)
        .await
        .unwrap();
    assert_eq!(before, history(&pool).await);
}

async fn history(pool: &PgPool) -> Value {
    // All preserved domain rows in this fixture, excluding the intended parent status/timestamp.
    sqlx::query_scalar("SELECT jsonb_build_object(
      'trip', (SELECT to_jsonb(t)-'status'-'updated_at' FROM tournaments t WHERE id=$1),
      'rounds', (SELECT jsonb_agg(to_jsonb(r) ORDER BY r.id) FROM rounds r),
      'scores', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.id) FROM scores s),
      'memberships', (SELECT jsonb_agg(to_jsonb(m) ORDER BY m.user_id) FROM tournament_memberships m),
      'entrants', (SELECT jsonb_agg(to_jsonb(p) ORDER BY p.player_id) FROM tournament_players p),
      'teams', (SELECT jsonb_agg(to_jsonb(t) ORDER BY t.id) FROM teams t),
      'team_memberships', (SELECT jsonb_agg(to_jsonb(m) ORDER BY m.team_id,m.player_id) FROM team_memberships m),
      'snapshots', (SELECT jsonb_agg(to_jsonb(s) ORDER BY s.round_id,s.player_id) FROM round_handicap_snapshots s),
      'confirmations', (SELECT jsonb_agg(to_jsonb(c) ORDER BY c.id) FROM scorecard_confirmations c),
      'completion', (SELECT jsonb_agg(to_jsonb(c) ORDER BY c.tournament_id) FROM tournament_completions c)
    )").bind(TRIP).fetch_one(pool).await.unwrap()
}

async fn private_get(app: &axum::Router, path: &str, token: &str) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::get(path)
                .header("cookie", format!("golf_session={token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK, "{path}");
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    body(response).await
}

#[sqlx::test(migrations = "../migrations")]
async fn archived_member_results_stay_private_and_visibility_and_revocation_remain_independent(
    pool: PgPool,
) {
    let session = seed(&pool, true).await;
    let viewer = Uuid::new_v4();
    sqlx::query("INSERT INTO users(id,username,display_name,role) VALUES ($1,'archive_viewer','Viewer','player')").bind(viewer).execute(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES ($1,$2,'viewer')",
    )
    .bind(TRIP)
    .bind(viewer)
    .execute(&pool)
    .await
    .unwrap();
    auth::create_session(
        &pool,
        viewer,
        &hash_session_token("archive-viewer"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    let invitation_path = format!("/api/tournaments/{TRIP}/invitations");
    let invitation_payload = json!({"expires_at":Utc::now()+Duration::days(1),"max_uses":null});
    let mut issue = request(TRIP, Some(TOKEN), true, invitation_payload.clone());
    *issue.uri_mut() = invitation_path.parse().unwrap();
    let response = app.clone().oneshot(issue).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let invitation = body(response).await;
    let invitation_id = invitation["id"].as_str().unwrap();
    tournaments::complete_authorized(&pool, session, TRIP, version(&pool).await)
        .await
        .unwrap();
    let paths = [
        format!("/api/tournaments/{TRIP}/leaderboards/gross"),
        format!("/api/tournaments/{TRIP}/leaderboards/net"),
        format!("/api/rounds/{ROUND}/completion-validation"),
        format!("/api/rounds/{ROUND}/scorecards/player/{PLAYER}"),
    ];
    let mut before = Vec::new();
    for path in &paths {
        before.push(private_get(&app, path, "archive-viewer").await);
    }
    tournaments::archive_authorized(&pool, session, TRIP, version(&pool).await)
        .await
        .unwrap();
    for (path, expected) in paths.iter().zip(before) {
        assert_eq!(private_get(&app, path, "archive-viewer").await, expected);
    }
    let visibility_path = format!("/api/tournaments/{TRIP}/final-round-visibility");
    let card_path = format!("/api/rounds/{ROUND}/scorecards/player/{PLAYER}");
    let card = private_get(&app, &card_path, "archive-viewer").await;
    assert_eq!(card["holes"].as_array().unwrap().len(), 9);
    for hidden in [false, true] {
        let current = private_get(&app, &visibility_path, TOKEN).await;
        let mut patch = request(
            TRIP,
            Some(TOKEN),
            true,
            json!({"back_nine_hidden":hidden,"expected_visibility_updated_at":current["visibility_updated_at"]}),
        );
        *patch.uri_mut() = visibility_path.parse().unwrap();
        *patch.method_mut() = axum::http::Method::PATCH;
        assert_eq!(
            app.clone().oneshot(patch).await.unwrap().status(),
            StatusCode::OK
        );
        let card = private_get(&app, &card_path, "archive-viewer").await;
        assert_eq!(
            card["holes"].as_array().unwrap().len(),
            if hidden { 9 } else { 18 }
        );
    }
    for (path, payload) in [
        (invitation_path.clone(), invitation_payload),
        (
            format!("{invitation_path}/{invitation_id}/rotate"),
            json!({}),
        ),
        (
            format!("/api/invitations/{invitation_id}/register"),
            json!({"token":invitation["token"],"account":{"username":"blocked_archive_join","password":"long-password-for-test"},"player":{"display_name":"Blocked","handicap_index":0}}),
        ),
    ] {
        let mut req = request(TRIP, Some(TOKEN), true, payload);
        *req.uri_mut() = path.parse().unwrap();
        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT, "{path}");
        assert_eq!(
            body(response).await["error"]["code"],
            "tournament_not_joinable"
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM users WHERE username='blocked_archive_join'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let mut revoke = request(TRIP, Some(TOKEN), true, json!({}));
    *revoke.uri_mut() = format!("{invitation_path}/{invitation_id}")
        .parse()
        .unwrap();
    *revoke.method_mut() = axum::http::Method::DELETE;
    assert_eq!(
        app.clone().oneshot(revoke).await.unwrap().status(),
        StatusCode::NO_CONTENT
    );
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1 AND user_id=$2")
        .bind(TRIP)
        .bind(viewer)
        .execute(&pool)
        .await
        .unwrap();
    for path in &paths {
        let response = app
            .clone()
            .oneshot(
                Request::get(path)
                    .header("cookie", "golf_session=archive-viewer")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn archives_once_with_audit_event_and_unchanged_private_history(pool: PgPool) {
    completed(&pool).await;
    let expected = version(&pool).await;
    let before = history(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let payload = json!({"expected_tournament_updated_at": expected});
    let response = app
        .clone()
        .oneshot(request(TRIP, Some(TOKEN), true, payload.clone()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let archived = body(response).await;
    assert_eq!(archived["id"], TRIP.to_string());
    assert_eq!(archived["status"], "archived");
    assert_ne!(version(&pool).await, expected);
    let event = events.try_recv().unwrap();
    assert_eq!(event.resource, "tournament");
    let audit: (Uuid, DateTime<Utc>) = sqlx::query_as(
        "SELECT archived_by_user_id,archived_at FROM tournament_archives WHERE tournament_id=$1",
    )
    .bind(TRIP)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit.0, ADMIN);
    assert!(audit.1 >= expected);
    let response = app
        .clone()
        .oneshot(request(TRIP, Some(TOKEN), true, payload))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, archived);
    assert!(events.try_recv().is_err());
    assert_eq!(before, history(&pool).await);
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_archives")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        tournaments::get_for_member(&pool, ADMIN, TRIP)
            .await
            .unwrap()
            .status,
        TournamentStatus::Archived
    );
    assert_eq!(
        tournaments::list_for_member(&pool, ADMIN)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        tournaments::list_for_user(&pool, ADMIN).await.unwrap()[0]
            .tournament
            .status,
        TournamentStatus::Archived
    );
    // No global-role bypass: this same account loses both archive and private read authority.
    sqlx::query("UPDATE users SET role='admin' WHERE id=$1")
        .bind(ADMIN)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DELETE FROM tournament_memberships WHERE tournament_id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    let response = app
        .oneshot(request(
            TRIP,
            Some(TOKEN),
            true,
            json!({"expected_tournament_updated_at": expected}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(
        tournaments::get_for_member(&pool, ADMIN, TRIP)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn rejects_auth_csrf_shape_stale_missing_and_non_admin_without_side_effects(pool: PgPool) {
    completed(&pool).await;
    let before = history(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let payload = json!({"expected_tournament_updated_at": version(&pool).await});
    for (token, csrf, data, expected) in [
        (None, false, payload.clone(), StatusCode::UNAUTHORIZED),
        (Some(TOKEN), false, payload.clone(), StatusCode::FORBIDDEN),
        (Some(TOKEN), true, json!({}), StatusCode::BAD_REQUEST),
        (
            Some(TOKEN),
            true,
            json!({"expected_tournament_updated_at":null}),
            StatusCode::BAD_REQUEST,
        ),
        (
            Some(TOKEN),
            true,
            json!({"expected_tournament_updated_at":"invalid"}),
            StatusCode::BAD_REQUEST,
        ),
        (
            Some(TOKEN),
            true,
            json!({"expected_tournament_updated_at":version(&pool).await,"extra":true}),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(request(TRIP, token, csrf, data))
            .await
            .unwrap();
        assert_eq!(response.status(), expected);
        assert!(body(response).await["error"]["code"].is_string());
    }
    let response = app
        .clone()
        .oneshot(request(
            TRIP,
            Some(TOKEN),
            true,
            json!({"expected_tournament_updated_at":Utc::now()-Duration::days(1)}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(response).await["error"]["code"],
        "tournament_archive_stale"
    );
    let response = app
        .clone()
        .oneshot(request(Uuid::new_v4(), Some(TOKEN), true, payload.clone()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(before, history(&pool).await);
    sqlx::query("UPDATE users SET role='admin' WHERE id=$1")
        .bind(ADMIN)
        .execute(&pool)
        .await
        .unwrap();
    for role in ["viewer", "scorer", "player"] {
        sqlx::query(
            "UPDATE tournament_memberships SET role=$2::tournament_role WHERE tournament_id=$1",
        )
        .bind(TRIP)
        .bind(role)
        .execute(&pool)
        .await
        .unwrap();
        assert_eq!(
            app.clone()
                .oneshot(request(TRIP, Some(TOKEN), true, payload.clone()))
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
    assert!(events.try_recv().is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_archives")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn draft_and_active_cannot_skip_completion(pool: PgPool) {
    seed(&pool, false).await;
    let draft: Uuid = sqlx::query_scalar("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES ($1,'Draft','2026-09-01','2026-09-01',1,1) RETURNING id").bind(Uuid::new_v4()).fetch_one(&pool).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES ($1,$2,'admin')",
    )
    .bind(draft)
    .bind(ADMIN)
    .execute(&pool)
    .await
    .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    for id in [draft, TRIP] {
        let response = app
            .clone()
            .oneshot(request(
                id,
                Some(TOKEN),
                true,
                json!({"expected_tournament_updated_at":Utc::now()}),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            body(response).await["error"]["code"],
            "tournament_archive_invalid_state"
        );
        assert!(
            sqlx::query("UPDATE tournaments SET status='archived' WHERE id=$1")
                .bind(id)
                .execute(&pool)
                .await
                .is_err()
        );
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn sql_guards_context_actor_audit_and_reverse_transitions(pool: PgPool) {
    let session = completed(&pool).await;
    let error = sqlx::query("UPDATE tournaments SET status='archived' WHERE id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        constraint(&error),
        Some("tournament_archive_context_required")
    );
    for (id, session_text, expected) in [
        (
            Uuid::new_v4(),
            session.to_string(),
            "tournament_archive_context_required",
        ),
        (
            TRIP,
            Uuid::new_v4().to_string(),
            "tournament_archive_admin_required",
        ),
        (
            TRIP,
            "not-a-session".to_owned(),
            "tournament_archive_admin_required",
        ),
    ] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('app.tournament_archive_id',$1::text,true),set_config('app.tournament_archive_session_id',$2,true)").bind(id).bind(session_text).execute(&mut *tx).await.unwrap();
        let error = sqlx::query("UPDATE tournaments SET status='archived' WHERE id=$1")
            .bind(TRIP)
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_eq!(constraint(&error), Some(expected));
        tx.rollback().await.unwrap();
    }
    // Context alone cannot manufacture evidence, even for the correct actor.
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.tournament_archive_id',$1::text,true)")
        .bind(TRIP)
        .execute(&mut *tx)
        .await
        .unwrap();
    let error = sqlx::query("INSERT INTO tournament_archives VALUES ($1,$2,clock_timestamp())")
        .bind(TRIP)
        .bind(ADMIN)
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(
        constraint(&error),
        Some("tournament_archive_record_immutable")
    );
    tx.rollback().await.unwrap();
    tournaments::archive_authorized(&pool, session, TRIP, version(&pool).await)
        .await
        .unwrap();
    for statement in [
        "UPDATE tournament_archives SET archived_at=clock_timestamp()",
        "UPDATE tournament_archives SET archived_by_user_id=NULL",
        "UPDATE tournament_archives SET tournament_id=gen_random_uuid()",
        "DELETE FROM tournament_archives",
    ] {
        assert_eq!(
            constraint(&sqlx::query(statement).execute(&pool).await.unwrap_err()),
            Some("tournament_archive_record_immutable")
        );
    }
    for status in ["draft", "active", "completed"] {
        assert_eq!(
            constraint(
                &sqlx::query("UPDATE tournaments SET status=$2::tournament_status WHERE id=$1")
                    .bind(TRIP)
                    .bind(status)
                    .execute(&pool)
                    .await
                    .unwrap_err()
            ),
            Some("tournament_status_transition_invalid")
        );
    }
    assert!(
        sqlx::query("DELETE FROM tournaments WHERE id=$1")
            .bind(TRIP)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("DELETE FROM rounds WHERE id=$1")
            .bind(ROUND)
            .execute(&pool)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_api_archives_emit_one_event_and_one_audit(pool: PgPool) {
    completed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let payload = json!({"expected_tournament_updated_at":version(&pool).await});
    let (a, b) = tokio::join!(
        app.clone()
            .oneshot(request(TRIP, Some(TOKEN), true, payload.clone())),
        app.oneshot(request(TRIP, Some(TOKEN), true, payload))
    );
    let a = a.unwrap();
    let b = b.unwrap();
    assert_eq!(a.status(), StatusCode::OK);
    assert_eq!(b.status(), StatusCode::OK);
    assert_eq!(body(a).await, body(b).await);
    assert_eq!(events.try_recv().unwrap().resource, "tournament");
    assert!(events.try_recv().is_err());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_archives")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
}
