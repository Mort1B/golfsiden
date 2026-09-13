use super::support::*;
use golf_api::{
    domain::password_recovery::RecoveryToken,
    repositories::{
        auth,
        password_recovery::{self, RecoveryError},
    },
};
use sqlx::PgPool;
use uuid::Uuid;

#[sqlx::test(migrations = false)]
async fn schema23_upgrade_preserves_accounts_sessions_and_seed_is_idempotent(pool: PgPool) {
    for migration in golf_api::schema::MIGRATOR.iter().filter(|m| m.version < 24) {
        sqlx::raw_sql(&migration.sql).execute(&pool).await.unwrap();
    }
    let r = fixture(&pool).await;
    let before:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM users),(SELECT count(*) FROM tournament_players),(SELECT count(*) FROM round_handicap_snapshots)").fetch_one(&pool).await.unwrap();
    sqlx::raw_sql(include_str!(
        "../../../migrations/0024_password_recovery.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        auth::find_active_session(&pool, &golf_api::auth::hash_session_token(TOKEN))
            .await
            .unwrap()
            .is_some()
    );
    for _ in 0..2 {
        sqlx::raw_sql(include_str!("../../seed.sql"))
            .execute(&pool)
            .await
            .unwrap();
    }
    let after:(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM users),(SELECT count(*) FROM tournament_players),(SELECT count(*) FROM round_handicap_snapshots)").fetch_one(&pool).await.unwrap();
    assert_eq!(before, after);
    let (grant, token) = issue(&pool, &r).await;
    password_recovery::preview(&pool, grant.id, &token)
        .await
        .unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn schema_protects_provenance_identity_history_and_rejects_expired_tokens(pool: PgPool) {
    let r = fixture(&pool).await;
    let (grant, _token) = issue(&pool, &r).await;
    for sql in [
        "UPDATE password_recovery_grants SET token_hash=decode(repeat('00',32),'hex') WHERE id=$1",
        "UPDATE password_recovery_grants SET issuer_kind='operator',issuer_user_id=NULL,tournament_id=NULL WHERE id=$1",
        "DELETE FROM password_recovery_grants WHERE id=$1",
        "DELETE FROM password_recovery_audits WHERE grant_id=$1",
    ] {
        assert!(
            sqlx::query(sql)
                .bind(grant.id)
                .execute(&pool)
                .await
                .is_err()
        );
    }
    assert!(
        sqlx::query("UPDATE password_recovery_authority_changes SET version=version+1000")
            .execute(&pool)
            .await
            .is_err()
    );
    password_recovery::admin_revoke(&pool, &r).await.unwrap();
    assert!(
        sqlx::query("UPDATE password_recovery_grants SET outcome=NULL,ended_at=NULL WHERE id=$1")
            .bind(grant.id)
            .execute(&pool)
            .await
            .is_err()
    );
    let expired = Uuid::new_v4();
    let expired_token = RecoveryToken::generate().unwrap();
    sqlx::query("INSERT INTO password_recovery_grants(id,token_hash,target_user_id,credential_generation,authority_version,issuer_kind,reason,issued_at,expires_at) SELECT $1,$2,id,credential_generation,0,'operator','Expiry fixture',clock_timestamp()-interval '1 hour',clock_timestamp()-interval '40 minutes' FROM users WHERE id=$3")
 .bind(expired).bind(expired_token.hash().as_bytes()).bind(USER).execute(&pool).await.unwrap();
    assert!(matches!(
        password_recovery::preview(&pool, expired, &expired_token).await,
        Err(RecoveryError::Invalid)
    ));
}
#[sqlx::test(migrations = "../migrations")]
async fn broad_runtime_dml_cannot_forge_operator_provenance_or_audits(pool: PgPool) {
    let r = fixture(&pool).await;
    let (grant, _) = issue(&pool, &r).await;
    let mut tx = pool.begin().await.unwrap();
    // Role creation is transactional: a fixed test name cannot leak to later runs.
    sqlx::query("CREATE ROLE recovery_runtime_test NOLOGIN NOSUPERUSER NOCREATEDB NOCREATEROLE")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("GRANT USAGE ON SCHEMA public TO recovery_runtime_test")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query(
        "GRANT SELECT,INSERT,UPDATE,DELETE ON ALL TABLES IN SCHEMA public TO recovery_runtime_test",
    )
    .execute(&mut *tx)
    .await
    .unwrap();
    sqlx::query("GRANT USAGE,SELECT ON ALL SEQUENCES IN SCHEMA public TO recovery_runtime_test")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO recovery_runtime_test")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SET LOCAL ROLE recovery_runtime_test")
        .execute(&mut *tx)
        .await
        .unwrap();
    for (sql, expected) in [
        (
            "INSERT INTO password_recovery_grants(id,token_hash,target_user_id,credential_generation,authority_version,issuer_kind,reason,issued_at,expires_at) SELECT gen_random_uuid(),decode(repeat('ff',32),'hex'),id,credential_generation,0,'operator','Forged provenance',now(),now()+interval '30 minutes' FROM users WHERE id=$1",
            "42501",
        ),
        (
            "INSERT INTO password_recovery_audits(grant_id,outcome,database_actor,reason) VALUES($1,'redeemed','forged','forged')",
            "23514",
        ),
    ] {
        sqlx::query("SAVEPOINT attempted")
            .execute(&mut *tx)
            .await
            .unwrap();
        let id = if expected == "42501" { OTHER } else { grant.id };
        let error = sqlx::query(sql)
            .bind(id)
            .execute(&mut *tx)
            .await
            .unwrap_err();
        assert_eq!(
            error.as_database_error().unwrap().code().as_deref(),
            Some(expected)
        );
        sqlx::query("ROLLBACK TO SAVEPOINT attempted")
            .execute(&mut *tx)
            .await
            .unwrap();
    }
    // Ordinary grant terminal operations still work with real restricted privileges.
    sqlx::query("UPDATE password_recovery_grants SET outcome='revoked',ended_at=clock_timestamp() WHERE id=$1").bind(grant.id).execute(&mut *tx).await.unwrap();
    tx.rollback().await.unwrap();
}
#[sqlx::test(migrations = "../migrations")]
async fn operator_cli_issues_to_private_file_without_stdout_and_revokes(pool: PgPool) {
    fixture(&pool).await;
    let mut database = reqwest::Url::parse(&std::env::var("DATABASE_URL").unwrap()).unwrap();
    database.set_path(pool.connect_options().get_database().unwrap());
    let database = database.to_string();
    let path = std::env::temp_dir().join(format!("golf-recovery-cli-{}", Uuid::new_v4()));
    let command_path = path.clone();
    let issue_database = database.clone();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(env!("CARGO_BIN_EXE_password-recovery"))
            .args([
                "issue",
                &ADMIN.to_string(),
                "Known organizer identity verified",
            ])
            .arg(command_path)
            .env("DATABASE_URL", issue_database)
            .env("RESET_PASSWORD_ORIGIN", "https://golf.example")
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let link = std::fs::read_to_string(&path).unwrap();
    let (prefix, secret) = link.trim().split_once("#token=").unwrap();
    let id = prefix.rsplit('/').next().unwrap().parse().unwrap();
    let token = RecoveryToken::parse(secret.to_owned()).unwrap();
    assert!(!String::from_utf8_lossy(&output.stderr).contains(secret));
    password_recovery::preview(&pool, id, &token).await.unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new(env!("CARGO_BIN_EXE_password-recovery"))
            .args(["revoke", &ADMIN.to_string(), "Organizer revoked link"])
            .env("DATABASE_URL", database)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert!(matches!(
        password_recovery::preview(&pool, id, &token).await,
        Err(RecoveryError::Invalid)
    ));
    std::fs::remove_file(path).unwrap();
}
