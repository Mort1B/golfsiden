# Recheck handicap-correction sessions immediately before commit

Date: 2026-09-23. Parent revision: `3c9fa44`. Scope: the approved AUTH-2 repair.
The [assessment](../authentication-2026-09-23/README.md) records the original
finding; the AUTH-1 limiter repair is unchanged. Work and validation are local,
with disposable PostgreSQL and synthetic accounts only. No production, external
targets or real credentials are used.

## Change and boundaries

[handicaps.rs](../../../backend/src/repositories/tournaments/handicaps.rs) reuses
`auth::lock_active_session` immediately before commit, after the handicap update
and audit read. This existing query checks database wall-clock expiry, revocation
and credential generation. The transaction already holds the session/user share
locks; checking them again adds no new lock-order dependency. Shared authorization
helpers, migration guards and HTTP mappings are unchanged.

An expired session returns the established `401` / `unauthenticated` error. The
transaction is dropped and rolled back, preserving both the previous handicap and
its audit history. The API publishes only after a successful repository result,
so rejected corrections emit no invalidation. Valid requests still produce one
audited correction and one matching event. Existing unchanged-handicap errors,
exact tournament-admin scope, round-opening locks and historical snapshots remain
unchanged.

The guarantee is a wall-clock session check immediately before commit after
transactional waits. Time can still pass during COMMIT itself; this repair does
not claim expiry is checked atomically at the instant PostgreSQL makes the commit
durable. It also does not change logout/revocation serialization.

## Reproduction and regressions

The new [expiry tests](../../../backend/tests/tournament_handicap_corrections/expiry.rs)
use the real API router, CSRF/session extraction, repositories and PostgreSQL.
Each synthetic fixture has an exact tournament administrator whose global role
is only viewer. Tests hold one of three database locks:

| Observed request wait | Expiry expected after repair | Valid-session control |
| --- | --- | --- |
| Tournament parent lock | 401; unchanged handicap/history; no event | 201; one audit and matching event |
| Membership share lock | 401; unchanged handicap/history; no event | 201; one audit and matching event |
| Roster UPDATE lock | 401; unchanged handicap/history; no event | 201; one audit and matching event |

For each case, `pg_stat_activity` and `pg_blocking_pids` confirm the specific query
waiting on the held backend PID. The session must still be active when that wait
is observed. The expiry cases then wait for PostgreSQL's clock to report expiry
before releasing the lock. The three-second fixture lifetime exercises a normal
session nearing expiry; it does not change production TTL configuration.

Before the repair, all three expiry regressions failed with **201 instead of
401**, while all three valid controls passed. After the repair, all nine tests in
`tournament_handicap_corrections` passed: these six cases plus the existing SQL
integrity/audit, unchanged-value, opening-race and snapshot scenarios.

The initial fixture attempt dropped the router's final event sender, causing
valid controls to observe a closed channel instead of an empty one. Retaining an
app clone corrected the fixture; the pre-repair run was repeated successfully
before changing production code. The recorded three-failure baseline is from
that corrected fixture. No application behavior was changed to accommodate it.

## Validation

Dependencies ran offline. Local mock listeners and a task-owned PostgreSQL 17.10
container used synthetic inputs; the database was bound to `127.0.0.1:55443`, with
tmpfs data and the cached image (`--pull=never`). No API server was exposed: HTTP
tests dispatched through the real router in-process. The container, its data and
temporary synthetic credential configuration were removed afterward. Pre-existing
local containers were untouched.

| Check | Result |
| --- | --- |
| Corrected pre-repair expiry fixture | 3 controls passed; 3 expiry cases failed 201 vs 401 |
| Focused handicap correction suite after repair | 9 passed |
| Rust formatting | Passed |
| Backend workspace/all-targets | 215 passed, 0 failed |
| Workspace/all-targets with database tests | 607 passed, 0 failed, 3 ignored |
| Clippy, all targets/features with `-D warnings` | Passed |
| Migration command | Passed: schema current |
| Seed command | Passed: synthetic fixture seeded |
| Diff and local documentation links | Passed |

The database-enabled total includes the 215 backend tests and 392 integration
tests. Three existing match-authorization measurements (`hundred_matches`,
`twelve_matches`, `twenty_four_matches`) remained ignored with the exact reason
`exclusive disposable PostgreSQL with pg_stat_statements required`. That extension
was not configured for this unrelated repair, and these are not counted as passes.
Existing SQLx vendor deprecation/dead-code warnings remain; Clippy passed for the
workspace. No platform cybersecurity safeguard or unresolved sandbox block occurred.
[results.txt](results.txt) retains sanitized command totals.

```sh
cargo test --offline -p golf-api --features database-tests \
  --test tournament_handicap_corrections expiry::
cargo test --offline -p golf-api --features database-tests \
  --test tournament_handicap_corrections
cargo fmt --all -- --check
cargo test --offline --workspace --all-targets
cargo test --offline --workspace --all-targets --features database-tests
cargo clippy --offline --workspace --all-targets --all-features -- -D warnings
cargo run --offline -p golf-api --bin migrate
cargo run --offline -p golf-api --bin seed
```

Independent read-only review found no correctness or security defects in the
implementation or regressions. It checked lock order, final-check placement,
rollback/error conversion, precise wait evidence and event assertions. No
frontend or browser behavior changed; those ladders are not applicable to this
bounded transaction repair. Existing Chrome/device/public-host acceptance gaps
remain as previously recorded. No migration definition was changed.

## Remaining work

This AUTH-2 repair is **READY** within its stated final-check guarantee.
Both confirmed authentication findings now have separate repairs. The wider
security assessment and public-host, native zoom and physical Android Chrome
gates still prevent deployment sign-off: **NOT READY**. This does not imply a
complete security review or proof of absence of other defects.
Work and commits remain local under the owner's existing scope; no push or
external publication is included.
