#![cfg(feature = "database-tests")]
#[path = "support/legacy_sessions.rs"]
mod legacy_sessions;

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
const HOLE: Uuid = uuid!("19000000-0000-0000-0000-000000000007");
const TOKEN: &str = "completion-test-admin";

#[sqlx::test(migrations = "../migrations")]
async fn self_profile_handicap_change_preserves_actual_locked_scores_and_snapshots(pool: PgPool) {
    use golf_api::repositories::profile::{self, DetailsChange};
    let session = seed(&pool, true).await;
    sqlx::query("UPDATE users SET player_id=$2 WHERE id=$1")
        .bind(ADMIN)
        .bind(PLAYER)
        .execute(&pool)
        .await
        .unwrap();
    let p = profile::get(&pool, session).await.unwrap();
    let facts = "SELECT jsonb_build_object('players',(SELECT jsonb_agg(to_jsonb(t)) FROM tournament_players t),'snapshots',(SELECT jsonb_agg(to_jsonb(s)) FROM round_handicap_snapshots s),'scores',(SELECT jsonb_agg(to_jsonb(s)) FROM scores s),'confirmations',(SELECT jsonb_agg(to_jsonb(c)) FROM scorecard_confirmations c))";
    let before: Value = sqlx::query_scalar(facts).fetch_one(&pool).await.unwrap();
    assert_eq!(before["snapshots"].as_array().unwrap().len(), 1);
    assert_eq!(before["scores"].as_array().unwrap().len(), 1);
    profile::update_details(
        &pool,
        session,
        DetailsChange {
            version: p.version,
            player_updated_at: p.player_updated_at,
            display_name: "Updated name".into(),
            handicap: Some(18.5),
            reason: "Official update after the round".into(),
        },
    )
    .await
    .unwrap();
    let after: Value = sqlx::query_scalar(facts).fetch_one(&pool).await.unwrap();
    assert_eq!(after, before);
}

#[sqlx::test(migrations = "../migrations")]
async fn password_changed_session_cannot_complete_through_database_guard(pool: PgPool) {
    let session = seed(&pool, true).await;
    sqlx::query("UPDATE users SET password_hash='new-credential' WHERE id=$1")
        .bind(ADMIN)
        .execute(&pool)
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SELECT set_config('app.tournament_completion_id',$1::text,true),set_config('app.tournament_completion_session_id',$2::text,true)").bind(TRIP).bind(session).execute(&mut *tx).await.unwrap();
    let error = sqlx::query("UPDATE tournaments SET status='completed' WHERE id=$1")
        .bind(TRIP)
        .execute(&mut *tx)
        .await
        .unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().constraint(),
        Some("tournament_completion_admin_required")
    );
    tx.rollback().await.unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_completions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

async fn seed(pool: &PgPool, locked: bool) -> Uuid {
    seed_at_schema(pool, locked, false).await
}
async fn seed_at_schema(pool: &PgPool, locked: bool, legacy: bool) -> Uuid {
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
      ('19000000-0000-0000-0000-000000000006','19000000-0000-0000-0000-000000000005','Tee',113,4);
      INSERT INTO holes(id,tee_id,hole_number,par,stroke_index) VALUES
      ('19000000-0000-0000-0000-000000000007','19000000-0000-0000-0000-000000000006',1,4,1);
      INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_id,course_name,tee_id,tee_name,number_of_holes,scoring_format) VALUES
      ('19000000-0000-0000-0000-000000000002','19000000-0000-0000-0000-000000000001',1,'Round','2026-09-01','19000000-0000-0000-0000-000000000005','Course','19000000-0000-0000-0000-000000000006','Tee',1,'individual_stroke_play');
      INSERT INTO flights(id,round_id,tournament_id,name) VALUES
      ('19000000-0000-0000-0000-000000000008','19000000-0000-0000-0000-000000000002','19000000-0000-0000-0000-000000000001','Flight');
      INSERT INTO flight_memberships(flight_id,round_id,tournament_id,player_id) VALUES
      ('19000000-0000-0000-0000-000000000008','19000000-0000-0000-0000-000000000002','19000000-0000-0000-0000-000000000001','19000000-0000-0000-0000-000000000004');")
      .execute(pool).await.unwrap();
    if legacy {
        let session = legacy_sessions::create_and_start(pool, ADMIN, TRIP, TOKEN).await;
        if locked {
            lock_round(pool).await;
        }
        return session;
    }
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
    scorecards::save(
        pool,
        scorecards::SaveScore {
            round_id: ROUND,
            hole_id: HOLE,
            owner: ScoreOwner::Player { id: PLAYER },
            gross_strokes: 4,
            submitted_by: ADMIN,
        },
    )
    .await
    .unwrap();
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
    let mut builder = Request::post(format!("/api/tournaments/{id}/complete"))
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

#[sqlx::test(migrations = "../migrations")]
async fn completes_once_audits_actor_preserves_visibility_scores_and_member_reads(pool: PgPool) {
    seed(&pool, true).await;
    let expected = version(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let data = json!({"expected_tournament_updated_at": expected});
    let before: Value = sqlx::query_scalar("SELECT to_jsonb(s) FROM scores s LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    let response = app
        .clone()
        .oneshot(request(TRIP, Some(TOKEN), true, data.clone()))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "private, no-store");
    let completed = body(response).await;
    assert_eq!(completed["status"], "completed");
    assert_eq!(events.try_recv().unwrap().resource, "tournament");
    let response = app
        .oneshot(request(TRIP, Some(TOKEN), true, data))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await, completed);
    assert!(events.try_recv().is_err());
    let audit: (Uuid, i64) = sqlx::query_as("SELECT completed_by_user_id, count(*) OVER() FROM tournament_completions WHERE tournament_id=$1").bind(TRIP).fetch_one(&pool).await.unwrap();
    assert_eq!(audit, (ADMIN, 1));
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT final_round_back_nine_hidden FROM tournaments WHERE id=$1"
        )
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    let after: Value = sqlx::query_scalar("SELECT to_jsonb(s) FROM scores s LIMIT 1")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        tournaments::get_for_member(&pool, ADMIN, TRIP)
            .await
            .unwrap()
            .status,
        TournamentStatus::Completed
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn rejects_auth_csrf_shape_stale_and_not_ready_without_events(pool: PgPool) {
    seed(&pool, false).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let data = json!({"expected_tournament_updated_at": version(&pool).await});
    for (token, csrf, payload, expected) in [
        (None, false, data.clone(), StatusCode::UNAUTHORIZED),
        (Some(TOKEN), false, data.clone(), StatusCode::FORBIDDEN),
        (Some(TOKEN), true, json!({}), StatusCode::BAD_REQUEST),
        (
            Some(TOKEN),
            true,
            json!({"expected_tournament_updated_at": version(&pool).await, "extra": true}),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        assert_eq!(
            app.clone()
                .oneshot(request(TRIP, token, csrf, payload))
                .await
                .unwrap()
                .status(),
            expected
        );
    }
    let response = app
        .clone()
        .oneshot(request(TRIP, Some(TOKEN), true, data))
        .await
        .unwrap();
    assert_eq!(
        body(response).await["error"]["code"],
        "tournament_completion_not_ready"
    );
    let response = app
        .clone()
        .oneshot(request(
            TRIP,
            Some(TOKEN),
            true,
            json!({"expected_tournament_updated_at": Utc::now()-Duration::days(1)}),
        ))
        .await
        .unwrap();
    assert_eq!(
        body(response).await["error"]["code"],
        "tournament_completion_stale"
    );
    assert_eq!(
        app.clone()
            .oneshot(request(
                Uuid::new_v4(),
                Some(TOKEN),
                true,
                json!({"expected_tournament_updated_at": Utc::now()})
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::NOT_FOUND
    );
    sqlx::query("UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(
        app.oneshot(request(
            TRIP,
            Some(TOKEN),
            true,
            json!({"expected_tournament_updated_at": Utc::now()})
        ))
        .await
        .unwrap()
        .status(),
        StatusCode::FORBIDDEN
    );
    assert!(events.try_recv().is_err());
}

#[sqlx::test(migrations = "../migrations")]
async fn sql_requires_context_active_admin_and_locked_plan_and_blocks_reversal_archive(
    pool: PgPool,
) {
    let session = seed(&pool, false).await;
    let error = sqlx::query("UPDATE tournaments SET status='completed' WHERE id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        constraint(&error),
        Some("tournament_completion_context_required")
    );
    for (session_id, expected) in [
        (Uuid::new_v4(), "tournament_completion_admin_required"),
        (session, "tournament_completion_not_ready"),
    ] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT set_config('app.tournament_completion_id',$1::text,true), set_config('app.tournament_completion_session_id',$2::text,true)").bind(TRIP).bind(session_id).execute(&mut *tx).await.unwrap();
        let error = sqlx::query("UPDATE tournaments SET status='completed' WHERE id=$1")
            .bind(TRIP)
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_eq!(constraint(&error), Some(expected));
        tx.rollback().await.unwrap();
    }
    lock_round(&pool).await;
    tournaments::complete_authorized(&pool, session, TRIP, version(&pool).await)
        .await
        .unwrap();
    for status in ["active", "draft", "archived"] {
        assert!(
            sqlx::query("UPDATE tournaments SET status=$2::tournament_status WHERE id=$1")
                .bind(TRIP)
                .bind(status)
                .execute(&pool)
                .await
                .is_err()
        );
    }
    assert!(
        sqlx::query("DELETE FROM rounds WHERE id=$1")
            .bind(ROUND)
            .execute(&pool)
            .await
            .is_err()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn concurrent_completions_commit_once_and_waiting_demotion_is_rechecked(pool: PgPool) {
    let session = seed(&pool, true).await;
    let expected = version(&pool).await;
    let (a, b) = tokio::join!(
        tournaments::complete_authorized(&pool, session, TRIP, expected),
        tournaments::complete_authorized(&pool, session, TRIP, expected)
    );
    assert_ne!(a.unwrap().changed, b.unwrap().changed);
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE id=$1 FOR UPDATE")
        .bind(ROUND)
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    let cloned = pool.clone();
    let retry = tokio::spawn(async move {
        tournaments::complete_authorized(&cloned, session, TRIP, expected).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    sqlx::query("UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    blocker.commit().await.unwrap();
    assert!(matches!(
        retry.await.unwrap(),
        Err(tournaments::TournamentMutationError::Authorization(_))
    ));
}

#[sqlx::test(migrations = "../migrations")]
async fn completion_waits_for_join_parent_lock_and_closed_issue_is_rejected(pool: PgPool) {
    let session = seed(&pool, true).await;
    let expected = version(&pool).await;
    let mut joining = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO tournament_invitations(id,tournament_id,token_hash,created_by_user_id,expires_at,series_id) VALUES ($1,$2,$3,$4,clock_timestamp()+interval '1 day',$1)")
      .bind(Uuid::new_v4()).bind(TRIP).bind([7_u8;32].as_slice()).bind(ADMIN).execute(&mut *joining).await.unwrap();
    let cloned = pool.clone();
    let completion = tokio::spawn(async move {
        tournaments::complete_authorized(&cloned, session, TRIP, expected).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert!(!completion.is_finished());
    joining.commit().await.unwrap();
    completion.await.unwrap().unwrap();
    let mut req = request(
        TRIP,
        Some(TOKEN),
        true,
        json!({"expires_at":Utc::now()+Duration::days(1), "max_uses":null}),
    );
    *req.uri_mut() = format!("/api/tournaments/{TRIP}/invitations")
        .parse()
        .unwrap();
    let response = api::router(AppState::new(pool.clone()))
        .oneshot(req)
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(response).await["error"]["code"],
        "tournament_not_joinable"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn audit_forgery_edits_deletion_and_identity_moves_are_rejected(pool: PgPool) {
    seed(&pool, true).await;
    let error = sqlx::query("INSERT INTO tournament_completions VALUES ($1,$2,clock_timestamp())")
        .bind(TRIP)
        .bind(ADMIN)
        .execute(&pool)
        .await
        .unwrap_err();
    assert_eq!(
        constraint(&error),
        Some("tournament_completion_record_immutable")
    );
    let other = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO users(id,username,display_name,role) VALUES ($1,'other','Other','admin')",
    )
    .bind(other)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES ($1,$2,'admin')",
    )
    .bind(TRIP)
    .bind(other)
    .execute(&pool)
    .await
    .unwrap();
    let session = auth::create_session(
        &pool,
        other,
        &hash_session_token("audit-only-admin"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap()
    .session_id;
    tournaments::complete_authorized(&pool, session, TRIP, version(&pool).await)
        .await
        .unwrap();
    for statement in [
        "UPDATE tournament_completions SET completed_at=clock_timestamp()",
        "UPDATE tournament_completions SET completed_by_user_id=NULL",
        "DELETE FROM tournament_completions",
    ] {
        assert_eq!(
            constraint(&sqlx::query(statement).execute(&pool).await.unwrap_err()),
            Some("tournament_completion_record_immutable")
        );
    }
    assert_eq!(
        constraint(
            &sqlx::query("UPDATE tournament_memberships SET user_id=$1 WHERE tournament_id=$2")
                .bind(other)
                .bind(TRIP)
                .execute(&pool)
                .await
                .unwrap_err()
        ),
        Some("tournament_closed_to_joining")
    );
    assert_eq!(constraint(&sqlx::query("INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES ($1,$2,'viewer')").bind(TRIP).bind(other).execute(&pool).await.unwrap_err()), Some("tournament_closed_to_joining"));
    // The audit's explicit FK policy retains the record when an account is removed.
    sqlx::query("DELETE FROM users WHERE id=$1")
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, Option<Uuid>>(
            "SELECT completed_by_user_id FROM tournament_completions WHERE tournament_id=$1"
        )
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap()
        .is_none()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn database_rechecks_session_expiry_after_share_lock_wait(pool: PgPool) {
    let session = seed(&pool, true).await;
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
    let transition = tokio::spawn(async move {
        let mut tx = cloned.begin().await.unwrap();
        sqlx::query("SELECT set_config('app.tournament_completion_id',$1::text,true),set_config('app.tournament_completion_session_id',$2::text,true)").bind(TRIP).bind(session).execute(&mut *tx).await.unwrap();
        let result = sqlx::query("UPDATE tournaments SET status='completed' WHERE id=$1")
            .bind(TRIP)
            .execute(&mut *tx)
            .await;
        tx.rollback().await.unwrap();
        result
    });
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    assert!(!transition.is_finished());
    blocker.commit().await.unwrap();
    let error = transition.await.unwrap().unwrap_err();
    assert_eq!(
        constraint(&error),
        Some("tournament_completion_admin_required")
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn repository_rechecks_expired_session_after_parent_wait(pool: PgPool) {
    let session = seed(&pool, true).await;
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
    let transition = tokio::spawn(async move {
        tournaments::complete_authorized(&cloned, session, TRIP, expected).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    assert!(!transition.is_finished());
    blocker.commit().await.unwrap();
    assert!(matches!(
        transition.await.unwrap(),
        Err(tournaments::TournamentMutationError::Authorization(_))
    ));
}

#[sqlx::test(migrations = false)]
async fn schema18_upgrade_preserves_valid_closed_history_without_fabricating_actor(pool: PgPool) {
    for migration in golf_api::schema::MIGRATOR
        .iter()
        .filter(|migration| migration.version < 19)
    {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    seed_at_schema(&pool, true, true).await;
    sqlx::query("UPDATE tournaments SET status='completed' WHERE id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE tournaments SET status='archived' WHERE id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    let before: Value = sqlx::query_scalar("SELECT to_jsonb(t) FROM tournaments t WHERE id=$1")
        .bind(TRIP)
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::raw_sql(include_str!(
        "../../migrations/0019_tournament_completion.sql"
    ))
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
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM tournament_completions")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = false)]
async fn schema18_upgrade_rejects_incompatible_closed_history_without_rewriting_it(pool: PgPool) {
    for migration in golf_api::schema::MIGRATOR
        .iter()
        .filter(|migration| migration.version < 19)
    {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    seed_at_schema(&pool, false, true).await;
    sqlx::query("UPDATE tournaments SET status='completed' WHERE id=$1")
        .bind(TRIP)
        .execute(&pool)
        .await
        .unwrap();
    let mut connection = pool.acquire().await.unwrap();
    let error = sqlx::raw_sql(include_str!(
        "../../migrations/0019_tournament_completion.sql"
    ))
    .execute(&mut *connection)
    .await
    .unwrap_err();
    assert_eq!(
        constraint(&error),
        Some("tournament_completion_legacy_not_ready")
    );
    sqlx::query("ROLLBACK")
        .execute(&mut *connection)
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT status::text FROM tournaments WHERE id=$1")
            .bind(TRIP)
            .fetch_one(&pool)
            .await
            .unwrap(),
        "completed"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn closed_invites_reject_accept_register_rotate_but_allow_revoke_and_visibility(
    pool: PgPool,
) {
    let session = seed(&pool, true).await;
    let outsider = Uuid::new_v4();
    let invitation = Uuid::new_v4();
    let secret = golf_api::auth::generate_invitation_token().unwrap();
    sqlx::query("INSERT INTO users(id,username,display_name,role,player_id) VALUES ($1,'outsider','Outsider','admin',$2)").bind(outsider).bind(PLAYER).execute(&pool).await.unwrap();
    auth::create_session(
        &pool,
        outsider,
        &hash_session_token("outsider-session"),
        Utc::now() + Duration::hours(1),
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO tournament_invitations(id,tournament_id,token_hash,created_by_user_id,expires_at,series_id) VALUES ($1,$2,$3,$4,clock_timestamp()+interval '1 day',$1)")
      .bind(invitation).bind(TRIP).bind(golf_api::auth::hash_invitation_token(&secret).as_slice()).bind(ADMIN).execute(&pool).await.unwrap();
    tournaments::complete_authorized(&pool, session, TRIP, version(&pool).await)
        .await
        .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    for (path, token, data) in [
        (
            format!("/api/invitations/{invitation}/accept"),
            Some("outsider-session"),
            json!({"token":secret}),
        ),
        (
            format!("/api/invitations/{invitation}/register"),
            None,
            json!({"token":secret,"account":{"username":"closed_registration","password":"a secure test password"},"player":{"display_name":"New entrant","handicap_index":10}}),
        ),
        (
            format!("/api/tournaments/{TRIP}/invitations/{invitation}/rotate"),
            Some(TOKEN),
            json!({}),
        ),
    ] {
        let mut req = request(TRIP, token, true, data);
        *req.uri_mut() = path.parse().unwrap();
        let response = app.clone().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            body(response).await["error"]["code"],
            "tournament_not_joinable"
        );
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM tournament_memberships WHERE user_id=$1"
        )
        .bind(outsider)
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM users WHERE username='closed_registration'"
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        0
    );
    let mut revoke = request(TRIP, Some(TOKEN), true, json!({}));
    *revoke.uri_mut() = format!("/api/tournaments/{TRIP}/invitations/{invitation}")
        .parse()
        .unwrap();
    *revoke.method_mut() = axum::http::Method::DELETE;
    assert!(
        app.clone()
            .oneshot(revoke)
            .await
            .unwrap()
            .status()
            .is_success()
    );
    let current: DateTime<Utc> =
        sqlx::query_scalar("SELECT visibility_updated_at FROM tournaments WHERE id=$1")
            .bind(TRIP)
            .fetch_one(&pool)
            .await
            .unwrap();
    let mut visibility = request(
        TRIP,
        Some(TOKEN),
        true,
        json!({"back_nine_hidden":false,"expected_visibility_updated_at":current}),
    );
    *visibility.uri_mut() = format!("/api/tournaments/{TRIP}/final-round-visibility")
        .parse()
        .unwrap();
    *visibility.method_mut() = axum::http::Method::PATCH;
    assert_eq!(
        app.oneshot(visibility).await.unwrap().status(),
        StatusCode::OK
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn registration_waiting_on_closure_rolls_back_new_identity(pool: PgPool) {
    let session = seed(&pool, true).await;
    let invitation = Uuid::new_v4();
    let secret = golf_api::auth::generate_invitation_token().unwrap();
    sqlx::query("INSERT INTO tournament_invitations(id,tournament_id,token_hash,created_by_user_id,expires_at,series_id) VALUES ($1,$2,$3,$4,clock_timestamp()+interval '1 day',$1)")
      .bind(invitation).bind(TRIP).bind(golf_api::auth::hash_invitation_token(&secret).as_slice()).bind(ADMIN).execute(&pool).await.unwrap();
    let before:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM users),(SELECT count(*) FROM players),(SELECT count(*) FROM user_sessions)").fetch_one(&pool).await.unwrap();
    let mut closure = pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM rounds WHERE tournament_id=$1 ORDER BY id FOR UPDATE")
        .bind(TRIP)
        .fetch_all(&mut *closure)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM tournaments WHERE id=$1 FOR UPDATE")
        .bind(TRIP)
        .fetch_one(&mut *closure)
        .await
        .unwrap();
    let app = api::router(AppState::new(pool.clone()));
    let mut req = request(
        TRIP,
        None,
        false,
        json!({"token":secret,"account":{"username":"racing_registration","password":"a secure test password"},"player":{"display_name":"Racing entrant","handicap_index":10}}),
    );
    *req.uri_mut() = format!("/api/invitations/{invitation}/register")
        .parse()
        .unwrap();
    let registration = tokio::spawn(async move { app.oneshot(req).await.unwrap() });
    tokio::time::timeout(std::time::Duration::from_secs(5),async {
        loop {
            let waiting=sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND query LIKE '%INSERT INTO tournament_memberships%')").fetch_one(&pool).await.unwrap();
            if waiting { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    sqlx::query("SELECT set_config('app.tournament_completion_id',$1::text,true),set_config('app.tournament_completion_session_id',$2::text,true)").bind(TRIP).bind(session).execute(&mut *closure).await.unwrap();
    sqlx::query("UPDATE tournaments SET status='completed' WHERE id=$1")
        .bind(TRIP)
        .execute(&mut *closure)
        .await
        .unwrap();
    closure.commit().await.unwrap();
    let response = registration.await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert_eq!(
        body(response).await["error"]["code"],
        "tournament_not_joinable"
    );
    let after:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM users),(SELECT count(*) FROM players),(SELECT count(*) FROM user_sessions)").fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
}
