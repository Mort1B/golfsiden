use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use std::{sync::Arc, sync::LazyLock};
use thiserror::Error;
use tokio::sync::Semaphore;

pub const MAX_CONCURRENT_PASSWORD_HASHES: usize = 4;
const MAX_ADMITTED_PASSWORD_OPERATIONS: usize = 8;
static PASSWORD_WORK_LIMIT: LazyLock<PasswordWorkLimiter> = LazyLock::new(|| {
    PasswordWorkLimiter::new(
        MAX_CONCURRENT_PASSWORD_HASHES,
        MAX_ADMITTED_PASSWORD_OPERATIONS,
    )
});

struct PasswordWorkLimiter {
    active: Arc<Semaphore>,
    admitted: Arc<Semaphore>,
}

impl PasswordWorkLimiter {
    fn new(active: usize, admitted: usize) -> Self {
        Self {
            active: Arc::new(Semaphore::new(active)),
            admitted: Arc::new(Semaphore::new(admitted)),
        }
    }
}

#[derive(Debug, Error)]
pub enum AsyncPasswordHashError {
    #[error("password hash capacity is unavailable")]
    CapacityUnavailable,
    #[error("password hash task failed")]
    TaskFailed,
    #[error("password hashing failed")]
    HashFailed,
}

pub fn hash_password(password: &[u8]) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    Ok(Argon2::default()
        .hash_password(password, &salt)?
        .to_string())
}

pub async fn hash_password_bounded(password: String) -> Result<String, AsyncPasswordHashError> {
    run_bounded_blocking(&PASSWORD_WORK_LIMIT, move || {
        hash_password(password.as_bytes())
    })
    .await?
    .map_err(|_| AsyncPasswordHashError::HashFailed)
}

async fn run_bounded_blocking<T, F>(
    limiter: &PasswordWorkLimiter,
    operation: F,
) -> Result<T, AsyncPasswordHashError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    // Admission never waits: only this finite set may retain inputs while
    // awaiting a worker slot. Cancellation here drops admission automatically.
    let admission = limiter
        .admitted
        .clone()
        .try_acquire_owned()
        .map_err(|_| AsyncPasswordHashError::CapacityUnavailable)?;
    let permit = limiter
        .active
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| AsyncPasswordHashError::CapacityUnavailable)?;
    tokio::task::spawn_blocking(move || {
        // Tokio cannot cancel started blocking work. Both permits belong to
        // the closure, including any time queued in Tokio's blocking pool.
        let _admission = admission;
        let _permit = permit;
        operation()
    })
    .await
    .map_err(|_| AsyncPasswordHashError::TaskFailed)
}

pub async fn verify_password_bounded(
    password: String,
    encoded_hash: String,
) -> Result<bool, AsyncPasswordHashError> {
    run_bounded_blocking(&PASSWORD_WORK_LIMIT, move || {
        let Ok(hash) = PasswordHash::new(&encoded_hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
    })
    .await
}

pub async fn verify_password(password: String, encoded_hash: String) -> bool {
    verify_password_bounded(password, encoded_hash)
        .await
        .unwrap_or(false)
}

#[cfg(test)]
mod tests;

#[cfg(all(test, feature = "database-tests"))]
mod api_tests;
