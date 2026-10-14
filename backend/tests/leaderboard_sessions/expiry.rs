use super::support::*;
use axum::http::{StatusCode, header};
use sqlx::PgPool;
use tokio::time::{Duration, sleep, timeout};

async fn expires_soon(pool: &PgPool, session: uuid::Uuid) {
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '2 seconds' WHERE id=$1",
    )
    .bind(session)
    .execute(pool)
    .await
    .unwrap();
}
async fn wait_expired(pool: &PgPool, session: uuid::Uuid) {
    timeout(Duration::from_secs(5), async {
        loop {
            let expired: bool = sqlx::query_scalar(
                "SELECT expires_at<=clock_timestamp() FROM user_sessions WHERE id=$1",
            )
            .bind(session)
            .fetch_one(pool)
            .await
            .unwrap();
            if expired {
                break;
            }
            sleep(Duration::from_millis(15)).await;
        }
    })
    .await
    .unwrap();
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_membership_wait_denies_all_boards(pool: PgPool) {
    let f = fixture(&pool).await;
    for path in paths() {
        let (session, token) = session(&pool).await;
        expires_soon(&pool, session).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        lock_membership(&mut gate).await;
        let app = f.app.clone();
        let pending_path = path.clone();
        let task = tokio::spawn(async move { get(&app, &pending_path, &token).await });
        wait_blocked(&pool, pid).await;
        wait_expired(&pool, session).await;
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(body(response).await["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn expiry_during_score_loading_denies_all_boards(pool: PgPool) {
    let f = fixture(&pool).await;
    for path in paths() {
        let (session, token) = session(&pool).await;
        expires_soon(&pool, session).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        sqlx::query("LOCK TABLE scores IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *gate)
            .await
            .unwrap();
        let app = f.app.clone();
        let pending_path = path.clone();
        let task = tokio::spawn(async move { get(&app, &pending_path, &token).await });
        wait_blocked(&pool, pid).await;
        wait_expired(&pool, session).await;
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{path}");
        assert_eq!(
            response.headers()[header::CACHE_CONTROL],
            "private, no-store"
        );
        assert_eq!(body(response).await["error"]["code"], "unauthenticated");
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn match_only_envelope_rechecks_expiry_after_wait(pool: PgPool) {
    let f = fixture(&pool).await;
    let match_trip = uuid::Uuid::new_v4();
    let match_round = uuid::Uuid::new_v4();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("INSERT INTO tournaments(id,name,start_date,end_date,number_of_rounds,counted_rounds) VALUES($1,'Match-only','2026-10-01','2026-10-01',1,NULL)")
        .bind(match_trip).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO rounds(id,tournament_id,round_number,name,round_date,course_name,tee_name,scoring_format) VALUES($1,$2,1,'Match','2026-10-01','','','singles_match_play')")
        .bind(match_round).bind(match_trip).execute(&mut *tx).await.unwrap();
    sqlx::query(
        "INSERT INTO tournament_memberships(tournament_id,user_id,role) VALUES($1,$2,'viewer')",
    )
    .bind(match_trip)
    .bind(USER)
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    for metric in ["gross", "net"] {
        let (session, token) = session(&pool).await;
        let path = format!("/api/tournaments/{match_trip}/leaderboards/{metric}");
        let baseline = get(&f.app, &path, &token).await;
        assert_eq!(baseline.status(), StatusCode::OK);
        assert_eq!(body(baseline).await["type"], "not_applicable");
        expires_soon(&pool, session).await;
        let mut gate = pool.begin().await.unwrap();
        let pid = blocker(&mut gate).await;
        sqlx::query("LOCK TABLE tournaments IN ACCESS EXCLUSIVE MODE")
            .execute(&mut *gate)
            .await
            .unwrap();
        let app = f.app.clone();
        let task = tokio::spawn(async move { get(&app, &path, &token).await });
        wait_blocked(&pool, pid).await;
        wait_expired(&pool, session).await;
        gate.commit().await.unwrap();
        let response = timeout(Duration::from_secs(5), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(body(response).await["error"]["code"], "unauthenticated");
    }
}
