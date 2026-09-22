use super::*;
use axum::{
    extract::Request as HttpRequest,
    middleware::{self, Next},
};
use golf_api::{AppState, api};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream},
    sync::Notify,
};
#[path = "../transaction_cancellation/proxy.rs"]
mod proxy;
struct Dropped(Arc<Notify>);
impl Drop for Dropped {
    fn drop(&mut self) {
        self.0.notify_one();
    }
}

#[sqlx::test(migrations = "../migrations")]
async fn http_disconnect_during_begin_reuses_clean_authorized_connection(source: PgPool) {
    ready(&source).await;
    fixture::session(&source, 205, None, "viewer", "cancel-viewer").await;
    let (proxy, pool) = proxy::Proxy::pool(&source).await;
    let before: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await
        .unwrap();
    let dropped = Arc::new(Notify::new());
    let observer = dropped.clone();
    let app = api::router(AppState::new(pool.clone())).layer(middleware::from_fn(
        move |request: HttpRequest, next: Next| {
            let guard = Dropped(observer.clone());
            async move {
                let _guard = guard;
                next.run(request).await
            }
        },
    ));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let path = format!(
        "/api/rounds/{}/match-play/matches?player_id={}",
        id(5),
        id(11)
    );
    let mut socket = TcpStream::connect(address).await.unwrap();
    proxy.gate.arm();
    socket
        .write_all(
            format!(
                "GET {path} HTTP/1.1\r\nHost: {address}\r\nCookie: golf_session={TOKEN}\r\n\r\n"
            )
            .as_bytes(),
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), proxy.gate.reached.notified())
        .await
        .unwrap();
    drop(socket);
    // Observe the actual Axum request future being dropped, not just the client abort.
    tokio::time::timeout(Duration::from_secs(5), dropped.notified())
        .await
        .unwrap();
    proxy.gate.release.notify_one();
    let after: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    assert_eq!(
        proxy
            .gate
            .commands()
            .iter()
            .filter(|c| *c == "ROLLBACK")
            .count(),
        1
    );
    let client = reqwest::Client::new();
    let url = format!("http://{address}{path}");
    for token in [TOKEN, "cancel-viewer"] {
        let response = client
            .get(&url)
            .header("cookie", format!("golf_session={token}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "private, no-store");
        let value: serde_json::Value = response.json().await.unwrap();
        assert_eq!(value["matches"].as_array().unwrap().len(), 1);
    }
    sqlx::query("DELETE FROM tournament_memberships WHERE user_id=$1")
        .bind(id(205))
        .execute(&source)
        .await
        .unwrap();
    let denied = client
        .get(&url)
        .header("cookie", "golf_session=cancel-viewer")
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), reqwest::StatusCode::FORBIDDEN);
    let allowed = client
        .get(&url)
        .header("cookie", format!("golf_session={TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(allowed.status(), reqwest::StatusCode::OK);
    sqlx::query(
        "UPDATE user_sessions SET created_at=clock_timestamp()-interval '2 seconds', expires_at=clock_timestamp()-interval '1 second' WHERE user_id=$1",
    )
    .bind(id(1))
    .execute(&source)
    .await
    .unwrap();
    let expired = client
        .get(&url)
        .header("cookie", format!("golf_session={TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(expired.status(), reqwest::StatusCode::UNAUTHORIZED);
    server.abort();
    pool.close().await;
}
