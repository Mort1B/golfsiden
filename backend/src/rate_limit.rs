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
    ProfileCredentials,
    RecoveryAdmin,
    ResultShareAdmin,
    PublicResults,
    RecoveryPreview,
    RecoveryRedeem,
    Onboarding,
    TournamentCreation,
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
                    RateLimitRoute::ResultShareAdmin,
                    Duration::from_secs(60),
                    10,
                    30,
                ),
                (
                    RateLimitRoute::PublicResults,
                    Duration::from_secs(60),
                    60,
                    240,
                ),
                (
                    RateLimitRoute::RecoveryAdmin,
                    Duration::from_secs(60),
                    5,
                    20,
                ),
                (
                    RateLimitRoute::RecoveryPreview,
                    Duration::from_secs(60),
                    30,
                    60,
                ),
                (
                    RateLimitRoute::RecoveryRedeem,
                    Duration::from_secs(600),
                    5,
                    20,
                ),
                (
                    RateLimitRoute::ProfileCredentials,
                    Duration::from_secs(60),
                    5,
                    20,
                ),
                (
                    RateLimitRoute::TournamentCreation,
                    Duration::from_secs(3600),
                    20,
                    40,
                ),
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
        // Admission and accounting share one lock so concurrent requests cannot
        // overbook capacity or forget an active limit.
        state
            .buckets
            .retain(|_, bucket| now.saturating_duration_since(bucket.started_at) < bucket.window);

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

        ensure_buckets(
            &mut state,
            [narrow_key, client_key],
            rule.window,
            inner.max_buckets,
            now,
        )?;

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
) -> Result<(), RateLimitExceeded> {
    let missing = keys
        .iter()
        .filter(|key| !state.buckets.contains_key(*key))
        .count();
    if state.buckets.len() + missing > max_buckets {
        // Never evict live counters to admit a new identity/resource. Rejection
        // leaves both buckets untouched; the first expiry is only a retry hint.
        let retry_after_seconds = state
            .buckets
            .values()
            .map(|bucket| exceeded(*bucket, now).retry_after_seconds)
            .min()
            .unwrap_or(1);
        return Err(RateLimitExceeded {
            retry_after_seconds,
        });
    }
    for key in keys {
        state.buckets.entry(key).or_insert(Bucket {
            started_at: now,
            count: 0,
            window,
        });
    }
    Ok(())
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
    fn capacity_preserves_counters_and_admits_existing_keys_until_their_limit() {
        let start = Instant::now();
        let limiter =
            RateLimiter::with_rules([(RateLimitRoute::Login, Duration::from_secs(60), 2, 3)], 4);
        let first = client("198.51.100.1");
        let second = client("198.51.100.2");
        let third = client("198.51.100.3");
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, first, b"one", start),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, second, b"two", start),
            Ok(())
        );
        let later = start + Duration::from_millis(1500);
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, third, b"three", later),
            Err(RateLimitExceeded {
                retry_after_seconds: 59
            })
        );
        assert_eq!(bucket_count(&limiter), 4);
        // Refused admission must not reset a bucket or consume an admitted key's quota.
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, first, b"one", later),
            Ok(())
        );
        assert!(
            limiter
                .check_at(RateLimitRoute::Login, first, b"one", later)
                .is_err()
        );
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, second, b"two", later),
            Ok(())
        );
        assert!(
            limiter
                .check_at(RateLimitRoute::Login, second, b"two", later)
                .is_err()
        );
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Login,
                third,
                b"three",
                start + Duration::from_secs(60)
            ),
            Ok(())
        );
        assert_eq!(bucket_count(&limiter), 2);
        assert_eq!(
            RateLimiter::disabled().check(RateLimitRoute::Login, first, b"one"),
            Ok(())
        );
    }

    fn bucket_count(limiter: &RateLimiter) -> usize {
        limiter
            .inner
            .as_ref()
            .unwrap()
            .state
            .lock()
            .unwrap()
            .buckets
            .len()
    }

    #[test]
    fn rejected_cross_route_churn_preserves_login_limits_at_small_and_production_capacity() {
        for limiter in [
            RateLimiter::with_rules(
                [
                    (RateLimitRoute::Login, Duration::from_secs(60), 10, 40),
                    (
                        RateLimitRoute::RecoveryPreview,
                        Duration::from_secs(60),
                        30,
                        60,
                    ),
                ],
                8,
            ),
            RateLimiter::production(),
        ] {
            let start = Instant::now();
            let caller = client("198.51.100.1");
            for key in 0u32..4 {
                for _ in 0..10 {
                    assert_eq!(
                        limiter.check_at(RateLimitRoute::Login, caller, &key.to_be_bytes(), start),
                        Ok(())
                    );
                }
            }
            let later = start + Duration::from_millis(1);
            assert!(
                limiter
                    .check_at(RateLimitRoute::Login, caller, &0u32.to_be_bytes(), later)
                    .is_err()
            );
            assert!(
                limiter
                    .check_at(RateLimitRoute::Login, caller, b"new-account", later)
                    .is_err()
            );
            for key in 0u128..8192 {
                let _ = limiter.check_at(
                    RateLimitRoute::RecoveryPreview,
                    caller,
                    &key.to_be_bytes(),
                    later,
                );
            }
            assert!(
                limiter
                    .check_at(RateLimitRoute::Login, caller, &0u32.to_be_bytes(), later)
                    .is_err()
            );
            assert!(
                limiter
                    .check_at(RateLimitRoute::Login, caller, b"new-account", later)
                    .is_err()
            );
            assert!(bucket_count(&limiter) <= limiter.inner.as_ref().unwrap().max_buckets);
            assert_eq!(
                limiter.check_at(
                    RateLimitRoute::Login,
                    caller,
                    b"new-account",
                    start + Duration::from_secs(60)
                ),
                Ok(())
            );
        }
    }

    #[test]
    fn limited_client_cannot_allocate_fresh_resource_keys() {
        let start = Instant::now();
        let limiter =
            RateLimiter::with_rules([(RateLimitRoute::Login, Duration::from_secs(60), 1, 1)], 16);
        let caller = client("198.51.100.1");
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, caller, b"one", start),
            Ok(())
        );
        for key in 0u32..100 {
            assert!(
                limiter
                    .check_at(RateLimitRoute::Login, caller, &key.to_be_bytes(), start)
                    .is_err()
            );
        }
        assert_eq!(bucket_count(&limiter), 2);
    }

    #[test]
    fn insufficient_capacity_does_not_partially_insert_or_charge_buckets() {
        let start = Instant::now();
        let limiter =
            RateLimiter::with_rules([(RateLimitRoute::Login, Duration::from_secs(60), 1, 2)], 3);
        let first = client("198.51.100.1");
        let second = client("198.51.100.2");
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, first, b"one", start),
            Ok(())
        );
        assert!(
            limiter
                .check_at(RateLimitRoute::Login, second, b"one", start)
                .is_err()
        );
        assert_eq!(bucket_count(&limiter), 2);
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, first, b"two", start),
            Ok(())
        );
        assert_eq!(bucket_count(&limiter), 3);
    }

    #[test]
    fn short_window_expiry_does_not_discard_long_window_protection() {
        let start = Instant::now();
        let limiter = RateLimiter::with_rules(
            [
                (RateLimitRoute::Login, Duration::from_secs(60), 1, 1),
                (RateLimitRoute::Onboarding, Duration::from_secs(3600), 1, 1),
            ],
            4,
        );
        let first = client("198.51.100.1");
        let second = client("198.51.100.2");
        assert_eq!(
            limiter.check_at(RateLimitRoute::Onboarding, first, b"long", start),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(RateLimitRoute::Login, first, b"short", start),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Login,
                second,
                b"new",
                start + Duration::from_secs(60)
            ),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Onboarding,
                first,
                b"long",
                start + Duration::from_secs(60)
            ),
            Err(RateLimitExceeded {
                retry_after_seconds: 3540
            })
        );
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Onboarding,
                first,
                b"long",
                start + Duration::from_secs(3600)
            ),
            Ok(())
        );
    }
    #[test]
    fn production_capacity_stays_bounded_and_recovers_for_new_clients() {
        let start = Instant::now();
        let limiter = RateLimiter::production();
        for id in 0..4096 {
            let caller = client(&format!("2001:db8::{id:x}"));
            assert_eq!(
                limiter.check_at(RateLimitRoute::Login, caller, b"account", start),
                Ok(())
            );
        }
        assert_eq!(bucket_count(&limiter), 8192);
        let new_client = client("198.51.100.2");
        assert!(
            limiter
                .check_at(RateLimitRoute::Login, new_client, b"new", start)
                .is_err()
        );
        let existing_client = client("2001:db8::0");
        for _ in 1..10 {
            assert_eq!(
                limiter.check_at(RateLimitRoute::Login, existing_client, b"account", start),
                Ok(())
            );
        }
        assert!(
            limiter
                .check_at(RateLimitRoute::Login, existing_client, b"account", start)
                .is_err()
        );
        assert_eq!(bucket_count(&limiter), 8192);
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Login,
                new_client,
                b"new",
                start + Duration::from_secs(60)
            ),
            Ok(())
        );
        assert_eq!(bucket_count(&limiter), 2);
    }

    #[test]
    fn capacity_retry_uses_earliest_expiry_not_oldest_bucket() {
        let start = Instant::now();
        let limiter = RateLimiter::with_rules(
            [
                (RateLimitRoute::Onboarding, Duration::from_secs(3600), 2, 4),
                (RateLimitRoute::Login, Duration::from_secs(60), 2, 2),
            ],
            4,
        );
        let caller = client("198.51.100.1");
        assert_eq!(
            limiter.check_at(RateLimitRoute::Onboarding, caller, b"long", start),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Login,
                caller,
                b"short",
                start + Duration::from_secs(1)
            ),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Login,
                caller,
                b"new",
                start + Duration::from_millis(2500)
            ),
            Err(RateLimitExceeded {
                retry_after_seconds: 59
            })
        );
        // Capacity rejection did not charge the existing login client counter.
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Login,
                caller,
                b"short",
                start + Duration::from_secs(3)
            ),
            Ok(())
        );
        assert_eq!(
            limiter.check_at(
                RateLimitRoute::Login,
                caller,
                b"new",
                start + Duration::from_secs(61)
            ),
            Ok(())
        );
    }

    #[test]
    fn zero_limits_reject_without_allocating() {
        for (per_key, per_client) in [(0, 1), (1, 0)] {
            let limiter = RateLimiter::with_rules(
                [(
                    RateLimitRoute::Login,
                    Duration::from_secs(60),
                    per_key,
                    per_client,
                )],
                2,
            );
            assert!(
                limiter
                    .check(RateLimitRoute::Login, client("198.51.100.1"), b"one")
                    .is_err()
            );
            assert_eq!(bucket_count(&limiter), 0);
        }
    }
}
