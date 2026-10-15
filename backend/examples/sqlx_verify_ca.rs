//! Opt-in A1w handshake probe. Only synthetic, disposable local fixtures are supported.
use sqlx::{
    Connection, PgConnection,
    postgres::{PgConnectOptions, PgSslMode},
};
use std::{env, path::PathBuf, time::Duration};

#[tokio::main]
async fn main() {
    let host = env::var("GOLF_TLS_HOST").unwrap();
    assert!(host == "localhost" || host == "127.0.0.1");
    let mode = match env::var("GOLF_TLS_MODE").unwrap().as_str() {
        "verify-ca" => PgSslMode::VerifyCa,
        "verify-full" => PgSslMode::VerifyFull,
        "require" => PgSslMode::Require,
        "prefer" => PgSslMode::Prefer,
        other => panic!("unsupported probe mode: {other}"),
    };
    let options = PgConnectOptions::new()
        .host(&host)
        .port(env::var("GOLF_TLS_PORT").unwrap().parse().unwrap())
        .username("golf")
        .password(&env::var("GOLF_TLS_PASSWORD").unwrap())
        .database("golf")
        .ssl_mode(mode)
        .ssl_root_cert(PathBuf::from(env::var_os("GOLF_TLS_CA").unwrap()));
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        PgConnection::connect_with(&options),
    )
    .await
    .expect("local handshake timed out");
    let result = match result {
        Ok(mut connection) => {
            let (encrypted, protocol): (bool, Option<String>) =
                sqlx::query_as("SELECT ssl,version FROM pg_stat_ssl WHERE pid=pg_backend_pid()")
                    .fetch_one(&mut connection)
                    .await
                    .unwrap();
            assert!(encrypted, "successful control must execute encrypted SQL");
            assert!(matches!(protocol.as_deref(), Some("TLSv1.2" | "TLSv1.3")));
            assert_eq!(
                sqlx::query_scalar::<_, i32>("SELECT 42")
                    .fetch_one(&mut connection)
                    .await
                    .unwrap(),
                42
            );
            connection.close().await.unwrap();
            serde_json::json!({"observed":"ok", "encrypted":true, "query":42, "protocol":protocol})
        }
        Err(error) => {
            assert!(matches!(error, sqlx::Error::Io(_) | sqlx::Error::Tls(_)));
            let message = error.to_string();
            let debug = format!("{error:?}");
            // Emit only a whitelisted certificate/negotiation category. An unrelated
            // connection failure cannot count as a negative certificate control.
            let reason = if message.contains("server does not support TLS") {
                "no_tls"
            } else if message.contains("invalid peer certificate") {
                if debug.contains("NotValidForNameContext") {
                    "hostname_context"
                } else if debug.contains("UnknownIssuer") {
                    "unknown_issuer"
                } else if debug.contains("Expired") {
                    "expired"
                } else if debug.contains("NotValidYet") {
                    "not_yet_valid"
                } else if debug.contains("BadSignature") {
                    "bad_signature"
                } else if debug.contains("InvalidPurpose") {
                    "invalid_purpose"
                } else {
                    panic!("unclassified certificate failure in synthetic probe");
                }
            } else {
                panic!("unexpected connection failure in synthetic probe");
            };
            serde_json::json!({"observed":reason, "rejected":true})
        }
    };
    println!("{result}");
}
