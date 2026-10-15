use std::{
    future::{Future, poll_fn},
    sync::{Condvar, Mutex},
    task::Poll,
    time::Duration,
};

use super::*;

// These are the only unit tests that call the process-wide entry points.
// Local limiters keep all deterministic scheduling tests independent.
pub(super) static PASSWORD_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

struct Gate(Arc<(Mutex<bool>, Condvar)>);

impl Gate {
    fn new() -> Self {
        Self(Arc::new((Mutex::new(false), Condvar::new())))
    }

    fn operation(&self) -> impl FnOnce() + Send + 'static {
        let state = self.0.clone();
        move || {
            let (lock, condition) = &*state;
            let mut open = lock.lock().unwrap();
            while !*open {
                open = condition.wait(open).unwrap();
            }
        }
    }
}

impl Drop for Gate {
    fn drop(&mut self) {
        let (lock, condition) = &*self.0;
        *lock.lock().unwrap() = true;
        condition.notify_all();
    }
}

async fn blocked_worker(
    limiter: Arc<PasswordWorkLimiter>,
    gate: &Gate,
) -> tokio::task::JoinHandle<Result<(), AsyncPasswordHashError>> {
    let (started, receiver) = tokio::sync::oneshot::channel();
    let operation = gate.operation();
    let task = tokio::spawn(async move {
        run_bounded_blocking(&limiter, move || {
            let _ = started.send(());
            operation();
        })
        .await
    });
    tokio::time::timeout(Duration::from_secs(5), receiver)
        .await
        .unwrap()
        .unwrap();
    task
}

async fn pending_once<T>(future: std::pin::Pin<&mut impl Future<Output = T>>) {
    let mut future = future;
    assert!(
        poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx)))
            .await
            .is_pending()
    );
}

async fn released(limiter: &PasswordWorkLimiter, active: usize, admitted: usize) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while limiter.active.available_permits() != active
            || limiter.admitted.available_permits() != admitted
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn bounded_hashes_and_verification_preserve_argon2_behavior() {
    let _guard = PASSWORD_TEST_LOCK.lock().await;
    let hash = hash_password_bounded("a sufficiently long password".to_owned())
        .await
        .unwrap();
    assert!(hash.starts_with("$argon2id$v=19$m=19456,t=2,p=1$"));
    assert!(
        verify_password_bounded("a sufficiently long password".to_owned(), hash.clone())
            .await
            .unwrap()
    );
    assert!(
        !verify_password_bounded("incorrect password".to_owned(), hash)
            .await
            .unwrap()
    );
    assert!(
        !verify_password_bounded(
            "a sufficiently long password".to_owned(),
            "invalid encoding".to_owned()
        )
        .await
        .unwrap()
    );
    released(&PASSWORD_WORK_LIMIT, 4, 8).await;
}

#[tokio::test]
async fn hashing_and_verification_share_prompt_admission_rejection() {
    let _guard = PASSWORD_TEST_LOCK.lock().await;
    let held = PASSWORD_WORK_LIMIT
        .admitted
        .clone()
        .acquire_many_owned(8)
        .await
        .unwrap();
    let mut hashing = Box::pin(hash_password_bounded("synthetic password".to_owned()));
    let mut verification = Box::pin(verify_password_bounded(
        "synthetic password".to_owned(),
        "unused hash".to_owned(),
    ));
    for result in [
        poll_fn(|cx| Poll::Ready(hashing.as_mut().poll(cx))).await,
        poll_fn(|cx| Poll::Ready(verification.as_mut().poll(cx)))
            .await
            .map(|v| v.map(|_| String::new())),
    ] {
        assert!(matches!(
            result,
            Poll::Ready(Err(AsyncPasswordHashError::CapacityUnavailable))
        ));
    }
    drop(held);
    assert!(
        hash_password_bounded("synthetic password".to_owned())
            .await
            .is_ok()
    );
    released(&PASSWORD_WORK_LIMIT, 4, 8).await;
}

#[tokio::test]
async fn bounded_runner_rejects_ninth_operation_without_waiting() {
    let limiter = Arc::new(PasswordWorkLimiter::new(4, 8));
    let gate = Gate::new();
    let mut active = Vec::new();
    for _ in 0..4 {
        active.push(blocked_worker(limiter.clone(), &gate).await);
    }
    let mut waiting = Vec::new();
    for _ in 0..4 {
        let mut future = Box::pin(run_bounded_blocking(&limiter, || ()));
        pending_once(future.as_mut()).await;
        waiting.push(future);
    }
    assert_eq!(limiter.active.available_permits(), 0);
    assert_eq!(limiter.admitted.available_permits(), 0);
    // One poll must reject: it cannot register another waiting operation.
    let mut excess = Box::pin(run_bounded_blocking(&limiter, || panic!("excess work ran")));
    let rejection = poll_fn(|cx| Poll::Ready(excess.as_mut().poll(cx))).await;
    assert!(matches!(
        rejection,
        Poll::Ready(Err(AsyncPasswordHashError::CapacityUnavailable))
    ));
    assert_eq!(limiter.active.available_permits(), 0);
    assert_eq!(limiter.admitted.available_permits(), 0);
    drop(gate);
    for task in active {
        task.await.unwrap().unwrap();
    }
    for future in waiting {
        future.await.unwrap();
    }
    released(&limiter, 4, 8).await;
    assert_eq!(run_bounded_blocking(&limiter, || 42).await.unwrap(), 42);
}

#[tokio::test]
async fn waiting_cancellation_releases_admission_without_running_operation() {
    let limiter = Arc::new(PasswordWorkLimiter::new(1, 2));
    let gate = Gate::new();
    let active = blocked_worker(limiter.clone(), &gate).await;
    let mut waiting = Box::pin(run_bounded_blocking(&limiter, || {
        panic!("cancelled work ran")
    }));
    pending_once(waiting.as_mut()).await;
    assert_eq!(limiter.admitted.available_permits(), 0);
    drop(waiting);
    assert_eq!(limiter.admitted.available_permits(), 1);
    assert_eq!(limiter.active.available_permits(), 0);
    let mut replacement = Box::pin(run_bounded_blocking(&limiter, || 42));
    pending_once(replacement.as_mut()).await;
    drop(gate);
    active.await.unwrap().unwrap();
    assert_eq!(replacement.await.unwrap(), 42);
    released(&limiter, 1, 2).await;
}

#[tokio::test]
async fn active_cancellation_keeps_both_permits_until_work_finishes() {
    let limiter = Arc::new(PasswordWorkLimiter::new(1, 1));
    let gate = Gate::new();
    let active = blocked_worker(limiter.clone(), &gate).await;
    active.abort();
    assert!(active.await.unwrap_err().is_cancelled());
    assert_eq!(limiter.active.available_permits(), 0);
    assert_eq!(limiter.admitted.available_permits(), 0);
    assert!(matches!(
        run_bounded_blocking(&limiter, || ()).await,
        Err(AsyncPasswordHashError::CapacityUnavailable)
    ));
    drop(gate);
    released(&limiter, 1, 1).await;
    assert_eq!(run_bounded_blocking(&limiter, || 42).await.unwrap(), 42);
}

#[tokio::test]
async fn operation_error_panic_and_closed_limiters_release_admission() {
    let limiter = PasswordWorkLimiter::new(1, 2);
    assert_eq!(
        run_bounded_blocking(&limiter, || Err::<(), _>(()))
            .await
            .unwrap(),
        Err(())
    );
    released(&limiter, 1, 2).await;
    assert!(matches!(
        run_bounded_blocking(&limiter, || panic!("synthetic worker panic")).await,
        Err(AsyncPasswordHashError::TaskFailed)
    ));
    released(&limiter, 1, 2).await;
    assert_eq!(run_bounded_blocking(&limiter, || 42).await.unwrap(), 42);
    limiter.active.close();
    assert!(matches!(
        run_bounded_blocking(&limiter, || ()).await,
        Err(AsyncPasswordHashError::CapacityUnavailable)
    ));
    assert_eq!(limiter.admitted.available_permits(), 2);
    limiter.admitted.close();
    assert!(matches!(
        run_bounded_blocking(&limiter, || ()).await,
        Err(AsyncPasswordHashError::CapacityUnavailable)
    ));
}

#[test]
fn cancelled_work_queued_in_tokio_pool_keeps_both_permits() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .max_blocking_threads(1)
        .build()
        .unwrap();
    runtime.block_on(async {
        let occupied = Gate::new();
        let (started, receiver) = tokio::sync::oneshot::channel();
        let operation = occupied.operation();
        let occupying = tokio::task::spawn_blocking(move || {
            let _ = started.send(());
            operation();
        });
        receiver.await.unwrap();
        let limiter = PasswordWorkLimiter::new(1, 1);
        let (ran, receiver) = tokio::sync::oneshot::channel();
        let mut queued = Box::pin(run_bounded_blocking(&limiter, move || {
            let _ = ran.send(());
        }));
        pending_once(queued.as_mut()).await;
        drop(queued);
        assert_eq!(limiter.active.available_permits(), 0);
        assert_eq!(limiter.admitted.available_permits(), 0);
        assert!(matches!(
            run_bounded_blocking(&limiter, || ()).await,
            Err(AsyncPasswordHashError::CapacityUnavailable)
        ));
        drop(occupied);
        occupying.await.unwrap();
        tokio::time::timeout(Duration::from_secs(5), receiver)
            .await
            .unwrap()
            .unwrap();
        released(&limiter, 1, 1).await;
    });
}
