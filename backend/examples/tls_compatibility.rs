//! Opt-in local TLS probe; run with the retained A4 Python harness.
//! Uses synthetic credentials and temporary certificates only.
use golf_api::course_provider::{CourseProviderClient, CourseProviderError};
use sqlx::{
    Connection, PgConnection,
    postgres::{PgConnectOptions, PgSslMode},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

fn database_options(host: &str, port: u16, mode: PgSslMode, ca: Option<&Path>) -> PgConnectOptions {
    let options = PgConnectOptions::new()
        .host(host)
        .port(port)
        .username("golf")
        .password("golf")
        .database("golf")
        .ssl_mode(mode);
    match ca {
        Some(ca) => options.ssl_root_cert(ca),
        None => options,
    }
}

async fn database_success(options: PgConnectOptions, name: &str) {
    let mut connection = tokio::time::timeout(
        Duration::from_secs(10),
        PgConnection::connect_with(&options),
    )
    .await
    .unwrap()
    .unwrap();
    let (encrypted, protocol): (bool, Option<String>) =
        sqlx::query_as("SELECT ssl,version FROM pg_stat_ssl WHERE pid=pg_backend_pid()")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert!(encrypted);
    assert!(matches!(protocol.as_deref(), Some("TLSv1.2" | "TLSv1.3")));
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT 42")
            .fetch_one(&mut connection)
            .await
            .unwrap(),
        42
    );
    connection.close().await.unwrap();
    println!("PASS {name}: encrypted SQL query");
}

enum TlsRejection {
    WrongHost,
    UnknownIssuer,
    NoTls,
}

async fn database_rejected(options: PgConnectOptions, name: &str, rejection: TlsRejection) {
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        PgConnection::connect_with(&options),
    )
    .await
    .unwrap();
    let error = result.expect_err("connection must fail");
    // Rustls handshake failures surface as SQLx Io errors; negotiation refusal
    // uses Tls. Require the certificate/mode reason, never an arbitrary IO error.
    let expected = match rejection {
        TlsRejection::WrongHost => "not valid for name",
        TlsRejection::UnknownIssuer => "UnknownIssuer",
        TlsRejection::NoTls => "server does not support TLS",
    };
    assert!(
        matches!(error, sqlx::Error::Tls(_) | sqlx::Error::Io(_))
            && error.to_string().contains(expected),
        "expected specific TLS rejection: {error}"
    );
    println!("PASS {name}: TLS rejection");
}

async fn database(certs: &Path) {
    let ca = certs.join("ca.crt");
    let wrong_ca = certs.join("wrong-ca.crt");
    database_success(
        database_options("localhost", 55450, PgSslMode::VerifyFull, Some(&ca)),
        "database verify-full trusted hostname",
    )
    .await;
    database_rejected(
        database_options("127.0.0.1", 55450, PgSslMode::VerifyFull, Some(&ca)),
        "database verify-full wrong hostname",
        TlsRejection::WrongHost,
    )
    .await;
    database_rejected(
        database_options("localhost", 55450, PgSslMode::VerifyFull, Some(&wrong_ca)),
        "database verify-full unknown issuer",
        TlsRejection::UnknownIssuer,
    )
    .await;
    // Both the original and updated lockfiles return NotValidForNameContext;
    // SQLx 0.8.6 only suppresses the older NotValidForName variant. Preserve
    // this stricter existing behavior; changing the driver is a separate step.
    database_rejected(
        database_options("127.0.0.1", 55450, PgSslMode::VerifyCa, Some(&ca)),
        "database verify-ca preserves existing hostname rejection",
        TlsRejection::WrongHost,
    )
    .await;
    database_success(
        database_options("localhost", 55450, PgSslMode::VerifyCa, Some(&ca)),
        "database verify-ca trusted matching hostname",
    )
    .await;
    database_rejected(
        database_options("localhost", 55450, PgSslMode::VerifyCa, Some(&wrong_ca)),
        "database verify-ca unknown issuer",
        TlsRejection::UnknownIssuer,
    )
    .await;
    database_success(
        database_options("localhost", 55450, PgSslMode::Require, None),
        "database require keeps encryption-only semantics",
    )
    .await;
    database_success(
        database_options("localhost", 55450, PgSslMode::Prefer, None),
        "database prefer negotiates available TLS",
    )
    .await;
    database_rejected(
        database_options("localhost", 55449, PgSslMode::Require, None),
        "database require rejects plaintext-only server",
        TlsRejection::NoTls,
    )
    .await;
    database_rejected(
        database_options("localhost", 55449, PgSslMode::VerifyFull, Some(&ca)),
        "database verify-full rejects plaintext-only server",
        TlsRejection::NoTls,
    )
    .await;
}

async fn provider(certs: &Path) {
    // This is the unchanged production provider constructor/trust policy.
    let provider = CourseProviderClient::configured_with_base_url(
        "synthetic-invalid-key".into(),
        "https://localhost:55451/",
    )
    .unwrap();
    assert!(matches!(
        provider.course("7k2m9qb4").await,
        Err(CourseProviderError::Upstream)
    ));
    println!("PASS actual provider rejects local untrusted certificate");
    // Positive local transport uses an explicit test root; production roots
    // and the provider constructor are never modified by the probe.
    let root =
        reqwest::Certificate::from_pem(&std::fs::read(certs.join("ca.crt")).unwrap()).unwrap();
    let client = reqwest::Client::builder()
        .tls_built_in_root_certs(false)
        .add_root_certificate(root)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(2))
        .timeout(Duration::from_secs(5))
        .no_proxy()
        .build()
        .unwrap();
    let response = client
        .get("https://localhost:55451/v1/courses/7k2m9qb4")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let value: serde_json::Value = response.json().await.unwrap();
    assert_eq!(value["course"]["id"], "7k2m9qb4");
    println!("PASS Reqwest/Rustls explicit test root accepts local HTTPS and JSON");
    let error = client
        .get("https://127.0.0.1:55451/v1/courses/7k2m9qb4")
        .send()
        .await
        .unwrap_err();
    assert!(error.is_connect());
    println!("PASS Reqwest/Rustls trusted issuer rejects wrong hostname");
}

async fn public_provider() {
    let provider = CourseProviderClient::configured("synthetic-invalid-key".into()).unwrap();
    // This result is only produced after an HTTP401/403 at the fixed official
    // endpoint. No real API key or paid data request is used.
    let result = provider.course("7k2m9qb4").await;
    if matches!(result, Err(CourseProviderError::Unavailable)) {
        println!("PASS actual provider official HTTPS reaches authentication rejection");
    } else {
        println!("BLOCKED official provider HTTPS did not reach authentication rejection");
        std::process::exit(2);
    }
}

#[tokio::main]
async fn main() {
    let mode = std::env::args().nth(1).unwrap();
    if mode == "public-provider" {
        public_provider().await;
        return;
    }
    let certs = PathBuf::from(std::env::var_os("GOLF_TLS_FIXTURES").unwrap());
    match mode.as_str() {
        "database" => database(&certs).await,
        "provider" => provider(&certs).await,
        _ => panic!("unknown probe mode"),
    }
}
