# Preserve active rate limits at capacity

Date: 2026-09-23. Parent commit: `d1b44de`. Scope: AUTH-1 only, as proposed in
`docs/PLANS.md` and authorized by the owner's “Move on”. The earlier
[assessment](../authentication-2026-09-23/README.md) remains the historical evidence
for the original defect. AUTH-2 is unchanged.

## Behavior

`backend/src/rate_limit.rs` now removes expired buckets and checks both existing
quotas before admitting buckets. If the required buckets will not fit, admission
fails without inserting either bucket or charging a quota. Unexpired counters
are never evicted to make room. Expiry, admission and increments remain atomic
under the existing mutex. Route limits, hashing, client identity, fixed windows,
8,192-bucket production capacity and disabled mode are unchanged.

At capacity, an already admitted key pair can use its remaining quota. New
client/resource buckets receive the existing 429 response, `rate_limited` JSON,
`Cache-Control: no-store` and rounded `Retry-After`. The capacity retry hint is the
earliest existing bucket expiry, not its creation time. One expiry may not free
all needed slots, so the hint does not guarantee admission on the next attempt.

This deliberately trades new admissions for preserving abuse protection: shared
saturation can temporarily deny new identities across routes, including login.
Expiry releases space without resetting unrelated active limits. It does not
provide per-route reserved capacity or multi-instance coordination.

## Regression evidence

Before changing production logic, eight focused tests ran against the old
implementation: four passed and four failed. The failures demonstrated active
counter eviction, admission at full capacity, admission by eviction when too few
slots were free and resource allocation by already limited clients. All passed after the repair.
The final focused coverage adds production saturation and boundary cases:

- Exhausted login key/client limits survive 8,192 distinct recovery-preview checks
  at both small and production capacity.
- Filling all 8,192 production buckets denies a new client while an existing key
  retains its remaining quota; the new client is admitted at expiry.
- Admission requiring two slots with one available rejects without partial
  allocation; another request needing only one slot still succeeds.
- Rejected checks neither charge an existing client nor extend its window.
- Mixed short/long windows preserve long-lived counters and calculate retry from
  expiry; exact expiry reopens capacity.
- Zero configured limits reject without allocation; disabled mode stays open.
- The real login router confirms the capacity error's status, JSON, no-store and
  retry headers, then permits the existing account's remaining quota and rejects
  it at its unchanged limit.

The four original failures were:

```text
capacity_preserves_counters_and_admits_existing_keys_until_their_limit
insufficient_capacity_does_not_partially_insert_or_charge_buckets
limited_client_cannot_allocate_fresh_resource_keys
rejected_cross_route_churn_preserves_login_limits_at_small_and_production_capacity
```

## Validation

Validation completed with offline dependencies and only a new task-owned
PostgreSQL 17.10 container from the cached image, synthetic accounts, loopback
binding and tmpfs data. No production systems, external targets or real credentials
were used. The container and its data were removed afterward; pre-existing local
services were left untouched.

| Check | Result |
| --- | --- |
| Focused limiter tests | 11 passed |
| Rust formatting | Passed |
| Backend workspace/all-targets | 215 passed, 0 failed |
| Workspace/all-targets with database tests | 601 passed, 0 failed, 3 ignored |
| Clippy, all targets/features with `-D warnings` | Passed |
| Migration command | Passed: schema current |
| Seed command | Passed: synthetic fixture seeded |
| Diff, local links and source-size check | Passed; limiter production logic 277 lines excluding blanks/comments/tests |

The database-enabled total includes the 215 backend tests plus 386 integration
tests; it is not 601 additional tests. The three ignored match-authorization
performance measurements (`hundred_matches`, `twelve_matches`, `twenty_four_matches`)
require an exclusive PostgreSQL instance configured with `pg_stat_statements`.
That extension was not configured for this unrelated repair; they were left
ignored, not counted as passes. The test harness's exact reason was
`exclusive disposable PostgreSQL with pg_stat_statements required`.
No platform cybersecurity safeguard or unresolved sandbox block occurred.
Sanitized command totals are retained in [results.txt](results.txt).

```sh
cargo test --offline -p golf-api --lib rate_limit::tests
cargo fmt --all -- --check
cargo test --offline --workspace --all-targets
cargo test --offline --workspace --all-targets --features database-tests
cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
cargo run --offline -p golf-api --bin migrate
cargo run --offline -p golf-api --bin seed
```

Independent read-only review found no correctness or security defects in the
implementation and regression diff. The reviewer specifically checked atomic
admission, quota preservation, mixed windows, zero limits and HTTP mapping.

No frontend, browser, migration or scoring behavior changed. Frontend/browser
ladders are therefore not applicable to this bounded repair. This does not close
the earlier native zoom, physical Android Chrome or public-host acceptance gaps.
Existing SQLx vendor deprecation/dead-code warnings are separate from this repair.

## Scope and remaining work

AUTH-1 is **READY** with the documented shared-capacity availability tradeoff.
This repair leaves session expiry during handicap-correction waits (AUTH-2), the
broader security assessment and existing deployment/device gates open. Deployment
sign-off remains **NOT READY**. Work and commits remain local under the owner's
existing local-only scope; no external publication is performed.
