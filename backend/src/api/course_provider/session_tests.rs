mod support;
use super::{CourseProviderApiError, authorized_course};
use crate::{course_provider::CourseProviderError, repositories::auth};
use axum::http::StatusCode;
use sqlx::PgPool;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use support::*;
use tokio::{
    sync::oneshot,
    time::{Duration, timeout},
};

#[sqlx::test(migrations = "../migrations")]
async fn pending_remote_success_and_errors_release_all_authority_and_recheck_it(pool: PgPool) {
    fixture(&pool).await;
    for error in [false, true] {
        for loss in ["none", "session", "credential", "membership"] {
            sqlx::query("UPDATE tournament_memberships SET role='admin' WHERE tournament_id=$1 AND user_id=$2").bind(TRIP).bind(USER).execute(&pool).await.unwrap();
            let sid = session(&pool).await;
            let mock = provider(true, error).await;
            let client = mock.client.clone();
            let read_pool = pool.clone();
            let read = tokio::spawn(async move {
                output(
                    authorized_course(&read_pool, sid, TRIP, || async move {
                        client
                            .course(COURSE)
                            .await
                            .map_err(CourseProviderApiError::Provider)
                    })
                    .await,
                )
            });
            timeout(Duration::from_secs(3), mock.requested.notified())
                .await
                .unwrap();
            // These writers must finish while real loopback HTTP is still held.
            timeout(Duration::from_secs(2),async {
                match loss {
                    "session"=>auth::revoke_session(&pool,sid).await.unwrap(),
                    "credential"=>{sqlx::query("UPDATE users SET password_hash=$1 WHERE id=$2").bind(format!("synthetic-new-{sid}")).bind(USER).execute(&pool).await.unwrap();},
                    "membership"=>{sqlx::query("UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1 AND user_id=$2").bind(TRIP).bind(USER).execute(&pool).await.unwrap();},
                    _=>{},
                }
            }).await.expect("provider I/O must hold no session/user or membership locks");
            mock.release.notify_one();
            let r = timeout(Duration::from_secs(3), read)
                .await
                .unwrap()
                .unwrap();
            let expected = match loss {
                "none" if error => StatusCode::SERVICE_UNAVAILABLE,
                "none" => StatusCode::OK,
                "membership" => StatusCode::FORBIDDEN,
                _ => StatusCode::UNAUTHORIZED,
            };
            assert_eq!(r.status(), expected);
            let value = body(r).await;
            if loss == "none" && !error {
                assert_eq!(value["provider_course_id"], COURSE);
            } else {
                assert_eq!(
                    value["error"]["code"],
                    match loss {
                        "none" => "course_provider_unavailable",
                        "membership" => "forbidden",
                        _ => "unauthenticated",
                    }
                );
                assert!(value.get("tees").is_none());
            }
            assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
        }
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn postflight_snapshot_retry_never_replays_fetch_or_cache_work(pool: PgPool) {
    fixture(&pool).await;
    for exhausted in [false, true] {
        let sid = session(&pool).await;
        let mock = provider(true, false).await;
        let client = mock.client.clone();
        let read_pool = pool.clone();
        let fetches = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&fetches);
        let read = tokio::spawn(async move {
            output(
                authorized_course(&read_pool, sid, TRIP, || async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    client
                        .course(COURSE)
                        .await
                        .map_err(CourseProviderApiError::Provider)
                })
                .await,
            )
        });
        timeout(Duration::from_secs(3), mock.requested.notified())
            .await
            .unwrap();
        sqlx::raw_sql("CREATE SEQUENCE course_postflight_attempts; CREATE TABLE course_postflight_failure(always_fail boolean NOT NULL); CREATE FUNCTION fail_course_postflight() RETURNS boolean LANGUAGE plpgsql AS $$ DECLARE attempt bigint; fail boolean; BEGIN attempt:=nextval('course_postflight_attempts'); SELECT always_fail INTO fail FROM course_postflight_failure; IF fail OR attempt=1 THEN RAISE EXCEPTION 'synthetic postflight conflict' USING ERRCODE='40001'; END IF; RETURN true; END $$; ALTER TABLE tournaments RENAME TO stored_tournaments; CREATE VIEW tournaments AS SELECT * FROM stored_tournaments WHERE fail_course_postflight();").execute(&pool).await.unwrap();
        sqlx::query("INSERT INTO course_postflight_failure VALUES($1)")
            .bind(exhausted)
            .execute(&pool)
            .await
            .unwrap();
        mock.release.notify_one();
        let r = timeout(Duration::from_secs(3), read)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            r.status(),
            if exhausted {
                StatusCode::SERVICE_UNAVAILABLE
            } else {
                StatusCode::OK
            }
        );
        if exhausted {
            assert_eq!(body(r).await["error"]["code"], "service_unavailable");
        }
        let attempts: i64 = sqlx::query_scalar("SELECT last_value FROM course_postflight_attempts")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(attempts, 2);
        assert_eq!(fetches.load(Ordering::SeqCst), 1);
        assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            mock.client.course(COURSE).await.unwrap().provider_course_id,
            COURSE
        );
        assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
        sqlx::raw_sql("DROP VIEW tournaments; ALTER TABLE stored_tournaments RENAME TO tournaments; DROP FUNCTION fail_course_postflight(); DROP TABLE course_postflight_failure; DROP SEQUENCE course_postflight_attempts;").execute(&pool).await.unwrap();
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn cached_facts_keep_payload_and_still_require_postflight_authority(pool: PgPool) {
    fixture(&pool).await;
    let sid = session(&pool).await;
    let mock = provider(false, false).await;
    let r = output(
        authorized_course(&pool, sid, TRIP, || async {
            mock.client
                .course(COURSE)
                .await
                .map_err(CourseProviderApiError::Provider)
        })
        .await,
    );
    assert_eq!(r.status(), StatusCode::OK);
    let expected = mock.client.course(COURSE).await.unwrap();
    assert_eq!(body(r).await, serde_json::to_value(expected).unwrap());
    let r = timeout(
        Duration::from_secs(3),
        authorized_course(&pool, sid, TRIP, || async {
            let cached = mock
                .client
                .course(COURSE)
                .await
                .map_err(CourseProviderApiError::Provider);
            auth::revoke_session(&pool, sid).await.unwrap();
            cached
        }),
    )
    .await
    .unwrap();
    let r = output(r);
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body(r).await["error"]["code"], "unauthenticated");
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
}

#[sqlx::test(migrations = "../migrations")]
async fn natural_expiry_during_provider_future_is_rechecked(pool: PgPool) {
    fixture(&pool).await;
    let sid = session(&pool).await;
    let mock = provider(false, false).await;
    let detail = mock.client.course(COURSE).await.unwrap();
    // A held normalized-result future avoids coupling this check to HTTP's 5s timeout.
    sqlx::query(
        "UPDATE user_sessions SET expires_at=clock_timestamp()+interval '8 seconds' WHERE id=$1",
    )
    .bind(sid)
    .execute(&pool)
    .await
    .unwrap();
    let (started, start) = oneshot::channel();
    let (release, wait) = oneshot::channel();
    let read_pool = pool.clone();
    let read = tokio::spawn(async move {
        output(
            authorized_course(&read_pool, sid, TRIP, || async {
                started.send(()).unwrap();
                wait.await.unwrap();
                Ok(detail)
            })
            .await,
        )
    });
    timeout(Duration::from_secs(3), start)
        .await
        .unwrap()
        .unwrap();
    let active: bool =
        sqlx::query_scalar("SELECT expires_at>clock_timestamp() FROM user_sessions WHERE id=$1")
            .bind(sid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(active);
    timeout(Duration::from_secs(12), async {
        loop {
            let active: bool = sqlx::query_scalar(
                "SELECT expires_at>clock_timestamp() FROM user_sessions WHERE id=$1",
            )
            .bind(sid)
            .fetch_one(&pool)
            .await
            .unwrap();
            if !active {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    release.send(()).unwrap();
    let r = timeout(Duration::from_secs(3), read)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(body(r).await["error"]["code"], "unauthenticated");
}

#[sqlx::test(migrations = "../migrations")]
async fn cancelling_remote_wait_leaves_authority_available_and_no_response_replay(pool: PgPool) {
    fixture(&pool).await;
    let sid = session(&pool).await;
    let mock = provider(true, false).await;
    let read_pool = pool.clone();
    let client = mock.client.clone();
    let read = tokio::spawn(async move {
        authorized_course(&read_pool, sid, TRIP, || async move {
            client
                .course(COURSE)
                .await
                .map_err(CourseProviderApiError::Provider)
        })
        .await
    });
    timeout(Duration::from_secs(3), mock.requested.notified())
        .await
        .unwrap();
    read.abort();
    assert!(read.await.is_err());
    timeout(Duration::from_secs(2), auth::revoke_session(&pool, sid))
        .await
        .unwrap()
        .unwrap();
    timeout(
        Duration::from_secs(2),
        sqlx::query(
            "UPDATE tournament_memberships SET role='viewer' WHERE tournament_id=$1 AND user_id=$2",
        )
        .bind(TRIP)
        .bind(USER)
        .execute(&pool),
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(mock.calls.load(Ordering::SeqCst), 1);
    let r = output(
        authorized_course(&pool, sid, TRIP, || async {
            Err(CourseProviderApiError::Provider(
                CourseProviderError::Upstream,
            ))
        })
        .await,
    );
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}
