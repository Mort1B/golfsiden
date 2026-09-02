use argon2::{
    Argon2, PasswordHash, PasswordHasher, PasswordVerifier,
    password_hash::{SaltString, rand_core::OsRng},
};
use std::{sync::Arc, sync::LazyLock};
use thiserror::Error;
use tokio::sync::Semaphore;

pub const MAX_CONCURRENT_PASSWORD_HASHES: usize = 4;
static PASSWORD_HASH_LIMIT: LazyLock<Arc<Semaphore>> =
    LazyLock::new(|| Arc::new(Semaphore::new(MAX_CONCURRENT_PASSWORD_HASHES)));

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
    run_bounded_blocking(PASSWORD_HASH_LIMIT.clone(), move || {
        hash_password(password.as_bytes())
    })
    .await?
    .map_err(|_| AsyncPasswordHashError::HashFailed)
}

async fn run_bounded_blocking<T, F>(
    limiter: Arc<Semaphore>,
    operation: F,
) -> Result<T, AsyncPasswordHashError>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    let permit = limiter
        .acquire_owned()
        .await
        .map_err(|_| AsyncPasswordHashError::CapacityUnavailable)?;
    tokio::task::spawn_blocking(move || {
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
    run_bounded_blocking(PASSWORD_HASH_LIMIT.clone(), move || {
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
mod tests {
    use std::sync::{
        Condvar, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    use super::*;

    #[tokio::test]
    async fn bounded_hashes_are_argon2_and_release_capacity() {
        let hash = hash_password_bounded("a sufficiently long password".to_owned())
            .await
            .unwrap();
        assert!(hash.starts_with("$argon2"));
        assert_eq!(MAX_CONCURRENT_PASSWORD_HASHES, 4);
    }

    #[tokio::test]
    async fn bounded_verification_uses_the_shared_capacity_and_releases_it() {
        let hash = hash_password(b"a sufficiently long password").unwrap();
        assert!(
            verify_password_bounded("a sufficiently long password".to_owned(), hash)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn cancellation_keeps_permit_until_blocking_work_finishes() {
        let limiter = Arc::new(Semaphore::new(1));
        let (started_sender, started_receiver) = tokio::sync::oneshot::channel();
        let (release_sender, release_receiver) = tokio::sync::oneshot::channel();
        let worker_limiter = limiter.clone();
        let task = tokio::spawn(async move {
            run_bounded_blocking(worker_limiter, move || {
                let _ = started_sender.send(());
                let _ = release_receiver.blocking_recv();
            })
            .await
        });

        started_receiver.await.unwrap();
        assert_eq!(limiter.available_permits(), 0);
        task.abort();
        let _ = task.await;
        assert_eq!(limiter.available_permits(), 0);

        let _ = release_sender.send(());
        tokio::time::timeout(std::time::Duration::from_secs(1), async {
            while limiter.available_permits() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(limiter.available_permits(), 1);
    }

    #[tokio::test]
    async fn bounded_runner_never_enters_more_than_four_operations() {
        let limiter = Arc::new(Semaphore::new(MAX_CONCURRENT_PASSWORD_HASHES));
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let (started_sender, mut started_receiver) = tokio::sync::mpsc::unbounded_channel();
        let mut tasks = Vec::new();

        for _ in 0..=MAX_CONCURRENT_PASSWORD_HASHES {
            let task_limiter = limiter.clone();
            let task_active = active.clone();
            let task_maximum = maximum.clone();
            let task_gate = gate.clone();
            let task_started = started_sender.clone();
            tasks.push(tokio::spawn(async move {
                run_bounded_blocking(task_limiter, move || {
                    let now_active = task_active.fetch_add(1, Ordering::SeqCst) + 1;
                    task_maximum.fetch_max(now_active, Ordering::SeqCst);
                    let _ = task_started.send(());
                    let (lock, condition) = &*task_gate;
                    let mut open = lock.lock().unwrap();
                    while !*open {
                        open = condition.wait(open).unwrap();
                    }
                    task_active.fetch_sub(1, Ordering::SeqCst);
                })
                .await
            }));
        }

        for _ in 0..MAX_CONCURRENT_PASSWORD_HASHES {
            started_receiver.recv().await.unwrap();
        }
        assert!(
            tokio::time::timeout(
                std::time::Duration::from_millis(50),
                started_receiver.recv()
            )
            .await
            .is_err()
        );
        assert_eq!(
            maximum.load(Ordering::SeqCst),
            MAX_CONCURRENT_PASSWORD_HASHES
        );

        let (lock, condition) = &*gate;
        *lock.lock().unwrap() = true;
        condition.notify_all();
        for task in tasks {
            task.await.unwrap().unwrap();
        }
        assert_eq!(
            maximum.load(Ordering::SeqCst),
            MAX_CONCURRENT_PASSWORD_HASHES
        );
    }
}
