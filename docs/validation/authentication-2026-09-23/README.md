# Local authentication and tournament authorization assessment

Date: 2026-09-23. Application source assessed:
`d46ba303446919f5a336031ea7e1e7a9df766c41`.

The owner authorized defensive source inspection, existing tests and disposable
local validation with synthetic accounts. No production systems, external targets
or real credentials were accessed. Application source, migrations and dependencies
were not changed. Two independent read-only reviewers covered authentication and
tournament access; the latter also verified the session-expiry reproducer.

The bounded assessment is complete. At assessment time two confirmed P2 findings
were unfixed. AUTH-1 is resolved by the subsequent
[rate-limit repair](../rate-limit-capacity-2026-09-23/README.md); AUTH-2 remains
unfixed. Deployment remains **NOT READY**, including the wider security and
deployment gates. P2 denotes medium priority here, not a calculated CVSS score.

## AUTH-1 — P2: capacity eviction removes active login throttles (resolved)

**Confirmed at assessed commit `d46ba30`.** The following source descriptions,
line references and diagnostic results describe that revision, before repair.
[rate_limit.rs](../../../backend/src/rate_limit.rs), lines 199–205,
allocates buckets before checking either limit. Lines 238–270 evict the oldest
other buckets at capacity even when their windows have not expired. Routes share
the production store of 8,192 buckets. Thus even rejected requests with fresh
resource keys can discard another route's active counters.

A public caller controls the UUID key in
[password_recovery/public.rs](../../../backend/src/api/password_recovery/public.rs),
lines 40–48: throttling occurs before recovery-token validation. No valid recovery
grant is needed to reach that allocation. Login uses this same limiter in
[api/auth.rs](../../../backend/src/api/auth.rs), lines 115–120.

The retained [diagnostic](probes.rs) invokes the actual `RateLimiter::production()`:

1. Consume 40 login attempts across four account keys for one synthetic client.
2. Confirm both a previously used account and a new account are denied, proving
   the narrow and broader client limits have been reached.
3. Check 8,192 distinct recovery-preview keys for that client. Only 60 checks
   succeed; the other 8,132 are rejected but still alter bucket storage.
4. Check the original login key again. It succeeds after 615 ms, before the
   60-second window has expired.

**Impact:** an unauthenticated caller can weaken the intended login attempt and
resource-abuse limits through cross-route key churn. This does not authenticate a
caller without a password. The algorithm is reproduced directly in memory; public
route reachability is source-supported. No HTTP flood was performed, and achievable
request throughput or practical credential-guessing success was not measured.

**Recommended repair:** reject an already limited client before admitting new
resource buckets, and preserve unexpired consumed limits when storage is full.
Fail closed for new admissions when necessary rather than silently forgetting
active protection. Add regressions for rejected cross-route churn, both limit
levels, capacity, expiry recovery and bounded memory. Existing capacity tests
check storage bounds but do not establish preservation of active limits.

## AUTH-2 — P2: handicap correction commits after session expiry while waiting

**Confirmed.**
[tournaments/handicaps.rs](../../../backend/src/repositories/tournaments/handicaps.rs),
lines 100–109, validates the active session and exact tournament-admin membership,
then waits for a tournament row lock. Lines 126–151 update the handicap, obtain
the audit record and commit without a final wall-clock session-expiry check.
[tournament_authorization.rs](../../../backend/src/repositories/tournament_authorization.rs),
lines 133–148, checks the session before the membership lock as well. The audited
handicap database guard checks the actor's membership, not session expiry.
The actual route is wired in
[api/tournaments.rs](../../../backend/src/api/tournaments.rs), lines 139–168.

The diagnostic creates an isolated draft tournament, player and exact tournament
admin, with a fresh session and matching CSRF token. It holds the tournament row
lock, dispatches the real API router request through Tower, and observes the
specific PostgreSQL blocking PID and query. It then waits until the database clock
confirms expiry before releasing the lock. Results:

| Case | Correction | Handicap | Audit delta | Invalidation | Next session read |
| --- | --- | --- | --- | --- | --- |
| Valid session after the wait | 201 | 12 → 5 | +1 | Emitted | 200 |
| Session expired before releasing the wait | 201 | 12 → 5 | +1 | Emitted | 401 |

The short three-second fixture lifetime makes the boundary deterministic; it is
not a claim that production permits a three-second session configuration. A
normally configured session nearing expiry has the same boundary.

**Impact:** an initially authorized administrator correction can persist after
session expiry during a database wait. This contradicts the final active-session
checks used by other protected writes. It does not demonstrate accepting a new
request with an already expired session, bypassing CSRF, obtaining another
tournament's privileges, or bypassing logout/revocation or membership demotion.
Those changes involve locks; natural passage of time does not.

**Recommended repair:** revalidate current session expiry after the relevant waits
and immediately before commit, preserving existing lock order and audit semantics.
An expired session should return 401, roll back the handicap and audit changes,
and emit no invalidation. Retain this lock-wait scenario as a permanent regression
and cover the membership wait if that shared authorization helper is changed.

## Reviewed controls and remaining concerns

Source and passing tests support the following controls within this scope:

- OS-generated 256-bit opaque session tokens, hashed server-side storage,
  session-bound CSRF derivation and constant-time CSRF comparison
  (`backend/src/auth/session.rs`).
- Argon2 verification with bounded worker concurrency, revocable server sessions,
  credential-generation checks and serialized credential changes
  (`backend/src/auth/password.rs`, `backend/src/repositories/auth.rs`,
  `backend/src/repositories/profile.rs`).
- HttpOnly/SameSite cookies, configured Secure cookies, non-cacheable auth
  responses, CSRF extraction and explicit configured CORS origins
  (`backend/src/api/auth.rs`, `backend/src/api/mod.rs`).
- Exact tournament membership rather than implicit global-role access, protected
  private reads and concurrent membership-removal tests
  (`backend/src/repositories/tournament_authorization.rs`, the private-read and
  tournament test suites below).
- Account-rooted frontend query ownership and cache clearing on identity changes,
  with stale-session-result protection (`frontend/src/features/auth/` and
  `frontend/src/api/privateWorkspace.test.ts`).

No additional CSRF or cross-tournament authorization bypass was confirmed. This
is a bounded assessment, not proof that none exists.

**Source-supported concern, not a confirmed finding:** some metadata read paths
use the user identity extracted at request entry rather than holding a session
lock through the read transaction. An in-flight read may therefore finish after
concurrent logout. The intended linearization contract for these reads and a
controlled reproducer remain unverified; do not conflate this with accepting a
new unauthenticated request or the confirmed mutation-expiry finding.

**Not covered completely:** recovery capability lifecycle, every public result
projection, offline persistence, operational privileges/proxy configuration and
dependency advisories. Recovery preview was inspected specifically for limiter
reachability. No external advisory queries or vulnerability scans were run.
The disposable tests used an owner database role; they are not fresh proof of
production least privilege. No Chrome or cross-origin browser validation ran in
this step. Earlier browser/recovery reports retain their own narrower evidence.

## Validation and reproducibility

All commands ran against the source commit above. PostgreSQL was **17.10**, from
an already cached `postgres:17-alpine` image (`--pull=never`), in a task-owned
Podman container with tmpfs data, bound only to `127.0.0.1:55443`. Accounts, tokens
and database credentials were synthetic. No API listener was started; the
mutation probe used the real router in-process and local PostgreSQL.
The task-owned container and its tmpfs data were removed after validation;
pre-existing local containers were left untouched.

| Checks | Result |
| --- | --- |
| Rust library tests | 206 passed |
| PostgreSQL integration tests, nine suites | 35 passed |
| Frontend authentication/cache tests, six files | 18 passed |
| Actual-code probes | Both findings reproduced; expiry control passed |

Commands executed:

```sh
cargo test --offline -p golf-api --lib

cargo test --offline -p golf-api --features database-tests \
  --test auth --test profile --test profile_concurrency \
  --test tournament_authorization --test private_workspace_reads \
  --test private_scorecard_live_reads --test tournament_scope_isolation \
  --test round_creation_authorization --test tournament_handicap_corrections \
  -- --test-threads=4

# From frontend/
npm run test -- src/features/auth/AuthProvider.test.ts \
  src/features/auth/navigation.test.ts src/api/auth.test.ts \
  src/api/privateWorkspace.test.ts src/features/auth/passwordForms.test.tsx \
  src/api/profile.test.ts

# Separate temporary diagnostic crate, current checkout as path dependency
cargo run --offline --manifest-path /tmp/golf-auth-review-d46ba30/probe/Cargo.toml
```

The local wrapper supplied only disposable database settings and disabled external
course-provider credentials. PostgreSQL suites passed: auth 5; profile 7;
profile concurrency 3; tournament authorization 4; private workspace reads 4;
private scorecard reads 4; tournament isolation 1; round creation 4;
handicap corrections 3.

[probes.rs](probes.rs) retains the diagnostic source for the assessed revision.
It asserts the original vulnerable behavior and is expected to fail against the
AUTH-1 repair; use the repair regressions to validate current behavior.
[results.txt](results.txt)
retains its sanitized output and test totals. To recreate it, use a separate
temporary Cargo crate with this file as `src/main.rs`, edition 2024, an empty
`[workspace]`, and a path dependency on this checkout's `backend`. Dependencies
are axum 0.8 (macros), chrono 0.4 (serde), serde_json 1, sqlx 0.8
(runtime-tokio-rustls, postgres, uuid, chrono, migrate), tokio 1 (macros, net,
rt-multi-thread, signal, sync, time), tower 0.5 (util), and uuid 1 (serde, v4).
Patch `sqlx-postgres` to this checkout's vendor directory and copy the root
`Cargo.lock` into that temporary crate before invoking Cargo offline. Supply only
the designated disposable `golf_review` database URL on loopback port 55443;
the probe rejects other endpoints. It migrates that empty database and inserts
synthetic fixtures. Destroy the disposable database after use.

### Initial failures, safeguards and limits

The first Rust library run passed 198 tests and failed eight course-provider mock
tests because the default sandbox denied a local listener bind at
`backend/src/course_provider/tests.rs:24:59`. Exact notice:

```text
Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }
```

The same command passed all 206 tests after permission for local sockets was
granted. The first expiry probe had an overlong synthetic username; shortening
the fixture to the documented username limit resolved it. Neither was an
application fix. Existing SQLx vendor deprecation/dead-code warnings remain.

**No platform cybersecurity safeguard blocked an action in this run.** The local
socket restriction above was a sandbox limitation and was resolved. No security
filter notice was returned. Earlier interrupted assessments do not substitute
for the fresh results recorded here.

The complete all-target backend, Clippy, full frontend build/test and browser
ladders were not rerun: no application implementation changed. Focused source,
unit, PostgreSQL and frontend checks establish the stated evidence, not full
release acceptance. Remediation and its full affected ladder are separate work.

## Next bounded work

AUTH-1 was repaired separately with explicit capacity/expiry regressions. Repair
AUTH-2 in a separate authorized step. Continue the remaining broader assessment and existing
deployment/device gates afterward; do not infer sign-off from passing tests.
This report and its diagnostics are local assessment artifacts; external
publication is excluded from this local-only run.
