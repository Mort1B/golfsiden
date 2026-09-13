//! Deployment operator only. No password or token is accepted in arguments.
use golf_api::{
    config::RecoveryOrigin, domain::password_recovery::RecoveryToken,
    repositories::password_recovery, schema,
};
use sqlx::postgres::PgPoolOptions;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
};
use uuid::Uuid;

struct Command {
    issue: bool,
    target: Uuid,
    reason: String,
    output: Option<PathBuf>,
}
fn parse(args: impl Iterator<Item = String>) -> Result<Command, &'static str> {
    let mut args = args;
    let action = args.next().ok_or("expected issue or revoke")?;
    let issue = match action.as_str() {
        "issue" => true,
        "revoke" => false,
        _ => return Err("expected issue or revoke"),
    };
    let target = args
        .next()
        .ok_or("exact account UUID is required")?
        .parse()
        .map_err(|_| "exact account UUID is required")?;
    let reason = args.next().ok_or("audit reason is required")?;
    if reason.trim().is_empty() || reason.chars().count() > 500 {
        return Err("audit reason must contain 1 to 500 characters");
    }
    let output = if issue {
        Some(PathBuf::from(
            args.next().ok_or("private output path is required")?,
        ))
    } else {
        None
    };
    if args.next().is_some() {
        return Err("unexpected argument");
    }
    Ok(Command {
        issue,
        target,
        reason,
        output,
    })
}
fn private_file(path: PathBuf) -> Result<File, &'static str> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
        options
            .open(path)
            .map_err(|_| "cannot create new private output file")
    }
    #[cfg(not(unix))]
    {
        let _ = (path, options);
        Err("private recovery output requires Unix file permissions")
    }
}
async fn run() -> Result<(), &'static str> {
    let command = parse(std::env::args().skip(1))?;
    let origin = if command.issue {
        let value = std::env::var("RESET_PASSWORD_ORIGIN")
            .map_err(|_| "RESET_PASSWORD_ORIGIN is required")?;
        let development = std::env::var("APP_ENV").as_deref() == Ok("development");
        Some(RecoveryOrigin::parse(&value, development)?)
    } else {
        None
    };
    let file = if let Some(path) = command.output {
        Some(
            tokio::task::spawn_blocking(move || private_file(path))
                .await
                .map_err(|_| "private file task failed")??,
        )
    } else {
        None
    };
    let database =
        std::env::var("DATABASE_URL").map_err(|_| "operator DATABASE_URL is required")?;
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database)
        .await
        .map_err(|_| "operator database connection failed")?;
    schema::check_compatibility(&pool)
        .await
        .map_err(|_| "database schema is incompatible")?;
    if let (Some(origin), Some(mut file)) = (origin, file) {
        let token = RecoveryToken::generate().map_err(|_| "token generation failed")?;
        let grant = password_recovery::operator_issue(
            &pool,
            command.target,
            &command.reason,
            &token.hash(),
        )
        .await
        .map_err(|_| "operator issue failed")?;
        let link = origin.link(grant.id, &token);
        let written = tokio::task::spawn_blocking(move || {
            file.write_all(format!("{link}\n").as_bytes())
                .and_then(|()| file.sync_all())
        })
        .await;
        if !matches!(written, Ok(Ok(()))) {
            // Exact grant only: never revoke a newer grant issued concurrently.
            password_recovery::operator_revoke_exact(
                &pool,
                command.target,
                grant.id,
                "Private output failed",
            )
            .await
            .map_err(|_| "output failed; operator must revoke recovery for this account")?;
            return Err("private output failed; new grant revoked");
        }
        eprintln!("Recovery link written to the private output file; expires in 30 minutes.");
    } else {
        password_recovery::operator_revoke(&pool, command.target, &command.reason)
            .await
            .map_err(|_| "operator revoke failed")?;
        eprintln!("Outstanding recovery links revoked.");
    }
    Ok(())
}
#[tokio::main]
async fn main() -> std::process::ExitCode {
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            std::process::ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parser_requires_exact_identity_reason_and_explicit_output() {
        let id = Uuid::new_v4().to_string();
        assert!(
            parse(
                ["issue", &id, "known contact verified", "/tmp/link"]
                    .map(str::to_owned)
                    .into_iter()
            )
            .is_ok()
        );
        assert!(parse(["issue", &id, "reason"].map(str::to_owned).into_iter()).is_err());
        assert!(
            parse(
                ["revoke", "someone", "reason"]
                    .map(str::to_owned)
                    .into_iter()
            )
            .is_err()
        );
        assert!(parse(["revoke", &id, " "].map(str::to_owned).into_iter()).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn private_output_never_overwrites_and_has_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!("golf-recovery-{}", Uuid::new_v4()));
        let file = private_file(path.clone()).unwrap();
        assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        assert!(private_file(path.clone()).is_err());
        std::fs::remove_file(path).unwrap();
    }
}
