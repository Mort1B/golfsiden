use std::time::Duration as WaitDuration;

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use golf_api::{AppState, api, auth::derive_csrf_token};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

use super::*;

#[derive(Clone, Copy, Debug)]
enum WaitPoint {
    Tournament,
    Membership,
    Roster,
}

impl WaitPoint {
    fn lock_sql(self) -> &'static str {
        match self {
            Self::Tournament => "SELECT id FROM tournaments WHERE id=$1 FOR UPDATE",
            Self::Membership => {
                "SELECT user_id FROM tournament_memberships WHERE tournament_id=$1 FOR UPDATE"
            }
            Self::Roster => {
                "SELECT player_id FROM tournament_players WHERE tournament_id=$1 FOR UPDATE"
            }
        }
    }
    fn waiting_query(self) -> &'static str {
        match self {
            Self::Tournament => "SELECT id FROM tournaments%",
            Self::Membership => "SELECT role FROM tournament_memberships%",
            Self::Roster => "UPDATE tournament_players AS tp%",
        }
    }
}

async fn correction_after_wait(pool: PgPool, point: WaitPoint, expire: bool) {
    let session_id = seed(&pool).await;
    let state = AppState::new(pool.clone());
    let mut events = state.live_events.subscribe();
    let app = api::router(state);
    let before: (f64, i64) = stored_state(&pool).await;
    let mut lock = pool.begin().await.unwrap();
    let blocker: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *lock)
        .await
        .unwrap();
    sqlx::query(point.lock_sql())
        .bind(TOURNAMENT_ID)
        .execute(&mut *lock)
        .await
        .unwrap();
    if expire {
        sqlx::query("UPDATE user_sessions SET expires_at=clock_timestamp()+interval '3 seconds' WHERE id=$1")
            .bind(session_id).execute(&pool).await.unwrap();
    }
    let request = Request::post(format!(
        "/api/tournaments/{TOURNAMENT_ID}/players/{PLAYER_ID}/handicap-corrections"
    ))
    .header("cookie", "golf_session=correction-session")
    .header("x-csrf-token", derive_csrf_token("correction-session"))
    .header("content-type", "application/json")
    .body(Body::from(
        json!({"handicap_index":16,"reason":"Expiry regression"}).to_string(),
    ))
    .unwrap();
    let task = tokio::spawn(app.clone().oneshot(request));
    tokio::time::timeout(WaitDuration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database()
                 AND $1=ANY(pg_blocking_pids(pid)) AND query LIKE $2)",
            )
            .bind(blocker)
            .bind(point.waiting_query())
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::time::sleep(WaitDuration::from_millis(10)).await;
        }
    })
    .await
    .expect("correction must reach the specific database wait before expiry");
    assert!(
        !session_expired(&pool, session_id).await,
        "fixture expired before observed {point:?} wait"
    );
    if expire {
        tokio::time::timeout(WaitDuration::from_secs(5), async {
            while !session_expired(&pool, session_id).await {
                tokio::time::sleep(WaitDuration::from_millis(20)).await;
            }
        })
        .await
        .unwrap();
    }
    lock.commit().await.unwrap();
    let response = tokio::time::timeout(WaitDuration::from_secs(5), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let status = response.status();
    let body: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    if expire {
        assert_eq!(
            status,
            StatusCode::UNAUTHORIZED,
            "expired correction at {point:?}"
        );
        assert_eq!(body["error"]["code"], "unauthenticated");
        assert_eq!(stored_state(&pool).await, before);
        assert!(matches!(
            events.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ));
    } else {
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(stored_state(&pool).await, (16.0, before.1 + 1));
        let event = events.try_recv().unwrap();
        assert_eq!(event.resource, "tournament");
        assert_eq!(event.tournament_id, TOURNAMENT_ID);
        assert_eq!(event.id, TOURNAMENT_ID);
        assert!(matches!(
            events.try_recv(),
            Err(tokio::sync::broadcast::error::TryRecvError::Empty)
        ));
    }
}

async fn session_expired(pool: &PgPool, session_id: Uuid) -> bool {
    sqlx::query_scalar("SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1")
        .bind(session_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn stored_state(pool: &PgPool) -> (f64, i64) {
    sqlx::query_as(
        "SELECT tournament_handicap::float8,
         (SELECT count(*) FROM tournament_handicap_history WHERE tournament_id=$1 AND player_id=$2)
         FROM tournament_players WHERE tournament_id=$1 AND player_id=$2",
    )
    .bind(TOURNAMENT_ID)
    .bind(PLAYER_ID)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_parent_wait_rolls_back_without_event(pool: PgPool) {
    correction_after_wait(pool, WaitPoint::Tournament, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn parent_wait_with_valid_session_commits_once(pool: PgPool) {
    correction_after_wait(pool, WaitPoint::Tournament, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_membership_wait_rolls_back_without_event(pool: PgPool) {
    correction_after_wait(pool, WaitPoint::Membership, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn membership_wait_with_valid_session_commits_once(pool: PgPool) {
    correction_after_wait(pool, WaitPoint::Membership, false).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_roster_update_wait_rolls_back_without_event(pool: PgPool) {
    correction_after_wait(pool, WaitPoint::Roster, true).await;
}
#[sqlx::test(migrations = "../migrations")]
async fn roster_update_wait_with_valid_session_commits_once(pool: PgPool) {
    correction_after_wait(pool, WaitPoint::Roster, false).await;
}
