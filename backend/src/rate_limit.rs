use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};

use crate::proxy::ClientIdentity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitRoute {
    Login,
    Onboarding,
    InvitationPreview,
    InvitationRegister,
    InvitationAccept,
}

#[derive(Debug, Clone, Copy)]
struct Rule {
    window: Duration,
    per_key_limit: u32,
    per_client_limit: u32,
}

#[derive(Debug, Clone, Copy)]
struct Bucket {
    started_at: Instant,
    count: u32,
    window: Duration,
}

#[derive(Debug)]
struct State {
    buckets: HashMap<[u8; 32], Bucket>,
}

#[derive(Clone, Debug)]
pub struct RateLimiter {
    inner: Option<Arc<Inner>>,
}

#[derive(Debug)]
struct Inner {
    rules: HashMap<RateLimitRoute, Rule>,
    max_buckets: usize,
    state: Mutex<State>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimitExceeded {
    pub retry_after_seconds: u64,
}

impl RateLimiter {
    pub fn disabled() -> Self {
        Self { inner: None }
    }

    pub fn production() -> Self {
        Self::with_rules(
            [
                (RateLimitRoute::Login, Duration::from_secs(60), 10, 40),
                (
                    RateLimitRoute::Onboarding,
                    Duration::from_secs(60 * 60),
                    3,
                    6,
                ),
                (
                    RateLimitRoute::InvitationPreview,
                    Duration::from_secs(60),
                    30,
                    100,
                ),
                (
                    RateLimitRoute::InvitationRegister,
                    Duration::from_secs(10 * 60),
                    5,
                    20,
                ),
                (
                    RateLimitRoute::InvitationAccept,
                    Duration::from_secs(60),
                    10,
                    40,
                ),
            ],
            8_192,
        )
    }

    pub fn with_rules(
        rules: impl IntoIterator<Item = (RateLimitRoute, Duration, u32, u32)>,
        max_buckets: usize,
    ) -> Self {
        let rules = rules
            .into_iter()
            .map(|(route, window, per_key_limit, per_client_limit)| {
                (
                    route,
                    Rule {
                        window,
                        per_key_limit,
                        per_client_limit,
                    },
                )
            })
            .collect();
        Self {
            inner: Some(Arc::new(Inner {
                rules,
                max_buckets: max_buckets.max(2),
                state: Mutex::new(State {
                    buckets: HashMap::new(),
                }),
            })),
        }
    }

    pub fn check(
        &self,
        route: RateLimitRoute,
        client: ClientIdentity,
        logical_key: &[u8],
    ) -> Result<(), RateLimitExceeded> {
        self.check_at(route, client, logical_key, Instant::now())
    }

    fn check_at(
        &self,
        route: RateLimitRoute,
        client: ClientIdentity,
        logical_key: &[u8],
        now: Instant,
    ) -> Result<(), RateLimitExceeded> {
        let Some(inner) = &self.inner else {
            return Ok(());
        };
        let Some(rule) = inner.rules.get(&route).copied() else {
            return Ok(());
        };
        let narrow_key = hash_bucket(0, route, client, logical_key);
        let client_key = hash_bucket(1, route, client, &[]);
        let mut state = inner
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        ensure_buckets(
            &mut state,
            [narrow_key, client_key],
            rule.window,
            inner.max_buckets,
            now,
        );

        let narrow = state.buckets.get(&narrow_key).copied().unwrap_or(Bucket {
            started_at: now,
            count: 0,
            window: rule.window,
        });
        if narrow.count >= rule.per_key_limit {
            return Err(exceeded(narrow, now));
        }
        let client_bucket = state.buckets.get(&client_key).copied().unwrap_or(Bucket {
            started_at: now,
            count: 0,
            window: rule.window,
        });
        if client_bucket.count >= rule.per_client_limit {
            return Err(exceeded(client_bucket, now));
        }

        increment(&mut state, narrow_key);
        increment(&mut state, client_key);
        Ok(())
    }
}

impl From<RateLimitExceeded> for crate::error::ApiError {
    fn from(value: RateLimitExceeded) -> Self {
        Self::RateLimited {
            retry_after_seconds: value.retry_after_seconds,
        }
    }
}

fn ensure_buckets(
    state: &mut State,
    keys: [[u8; 32]; 2],
    window: Duration,
    max_buckets: usize,
    now: Instant,
) {
    state
        .buckets
        .retain(|_, bucket| now.saturating_duration_since(bucket.started_at) < bucket.window);
    let missing = keys
        .iter()
        .filter(|key| !state.buckets.contains_key(*key))
        .count();
    while state.buckets.len() + missing > max_buckets {
        let Some(oldest) = state
            .buckets
            .iter()
            .filter(|(key, _)| !keys.contains(key))
            .min_by_key(|(_, bucket)| bucket.started_at)
            .map(|(key, _)| *key)
        else {
            break;
        };
        state.buckets.remove(&oldest);
    }
    for key in keys {
        let bucket = state.buckets.entry(key).or_insert(Bucket {
            started_at: now,
            count: 0,
            window,
        });
        refresh(bucket, now);
    }
}

fn increment(state: &mut State, key: [u8; 32]) {
    if let Some(bucket) = state.buckets.get_mut(&key) {
        bucket.count = bucket.count.saturating_add(1);
    }
}

fn hash_bucket(
    kind: u8,
    route: RateLimitRoute,
    client: ClientIdentity,
    logical_key: &[u8],
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([kind, route as u8]);
    hasher.update(client.as_bytes());
    hasher.update(logical_key);
    hasher.finalize().into()
}

fn refresh(bucket: &mut Bucket, now: Instant) {
    if now.saturating_duration_since(bucket.started_at) >= bucket.window {
        *bucket = Bucket {
            started_at: now,
            count: 0,
            window: bucket.window,
        };
    }
}

fn exceeded(bucket: Bucket, now: Instant) -> RateLimitExceeded {
    let elapsed = now.saturating_duration_since(bucket.started_at);
    let remaining = bucket.window.saturating_sub(elapsed);
    RateLimitExceeded {
        retry_after_seconds: (remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0)).max(1),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::*;
    use crate::proxy::{PROXY_CLIENT_IP_HEADER, PROXY_SHARED_SECRET_HEADER, ProxyTrustConfig};

    const SECRET: &str = "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    fn client(ip: &str) -> ClientIdentity {
        let mut headers = HeaderMap::new();
        headers.insert(PROXY_CLIENT_IP_HEADER, HeaderValue::from_str(ip).unwrap());
        headers.insert(PROXY_SHARED_SECRET_HEADER, HeaderValue::from_static(SECRET));
        ProxyTrustConfig::trusted(SECRET.to_owned()).client_identity(&headers)
    }

    #[test]
    fn key_and_per_client_buckets_are_independent() {
        let start = Instant::now();
        let limiter =
            RateLimiter::with_rules([(RateLimitRoute::Login, Duration::from_secs(60), 2, 3)], 16);
        let client = client("198.51.100.1");
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, client, b"one", start),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, client, b"one", start),
            Ok(())
        );
        assert!(
            limiter
                .check_at(RateLimitRoute::Login, client, b"one", start)
                .is_err()
        );
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, client, b"two", start),
            Ok(())
        );
        assert!(
            limiter
                .check_at(RateLimitRoute::Login, client, b"three", start)
                .is_err()
        );
    }

    #[test]
    fn blocked_key_does_not_exhaust_another_key_or_client() {
        let start = Instant::now();
        let limiter =
            RateLimiter::with_rules([(RateLimitRoute::Login, Duration::from_secs(60), 1, 2)], 16);
        let first = client("198.51.100.1");
        let second = client("198.51.100.2");
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, first, b"blocked", start),
            Ok(())
        );
        for _ in 0..20 {
            assert!(
                limiter
                    .check_at(RateLimitRoute::Login, first, b"blocked", start)
                    .is_err()
            );
        }
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, first, b"other", start),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, second, b"blocked", start),
            Ok(())
        );
    }

    #[test]
    fn resets_after_the_window_and_reports_rounded_retry_seconds() {
        let start = Instant::now();
        let limiter = RateLimiter::with_rules(
            [(RateLimitRoute::Login, Duration::from_secs(60), 1, 10)],
            16,
        );
        let client = client("198.51.100.1");
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, client, b"one", start),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Login,
                client,
                b"one",
                start + Duration::from_millis(1_500)
            ),
            Err(RateLimitExceeded {
                retry_after_seconds: 59
            })
        );
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Login,
                client,
                b"one",
                start + Duration::from_secs(60)
            ),
            Ok(())
        );
    }

    #[test]
    fn all_bucket_storage_is_bounded_and_disabled_mode_is_open() {
        let start = Instant::now();
        let limiter = RateLimiter::with_rules(
            [(RateLimitRoute::Login, Duration::from_secs(60), 2, 100)],
            4,
        );
        for (ip, key) in [
            ("198.51.100.1", b"one".as_slice()),
            ("198.51.100.2", b"two"),
            ("198.51.100.3", b"three"),
        ] {
            assert_eq!(
                limiter.check_at(RateLimitRoute::Login, client(ip), key, start),
                Ok(())
            );
        }
        assert_eq!(
            limiter
                .inner
                .as_ref()
                .unwrap()
                .state
                .lock()
                .unwrap()
                .buckets
                .len(),
            4
        );
        assert_eq!(
            RateLimiter::disabled().check(RateLimitRoute::Login, client("198.51.100.1"), b"one"),
            Ok(())
        );
    }
}
